package mountdav

import (
	"context"
	"io"
	"net/http"
	"net/http/httptest"
	"sync"
	"testing"
	"time"

	"TDrive/backend/mountfs"
)

type tailRecordingContent struct {
	data []byte

	mu      sync.Mutex
	maxRead int64
	reads   int
}

func (c *tailRecordingContent) ReadAt(ctx context.Context, buffer []byte, offset int64) (int, error) {
	if err := ctx.Err(); err != nil {
		return 0, err
	}
	c.mu.Lock()
	if offset > c.maxRead {
		c.maxRead = offset
	}
	c.reads++
	c.mu.Unlock()
	if offset >= int64(len(c.data)) {
		return 0, io.EOF
	}
	n := copy(buffer, c.data[offset:])
	if n < len(buffer) {
		return n, io.EOF
	}
	return n, nil
}

func (c *tailRecordingContent) Close() error { return nil }

func (c *tailRecordingContent) snapshot() (int64, int) {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.maxRead, c.reads
}

type tailRecordingOpener struct {
	content *tailRecordingContent
}

func (o *tailRecordingOpener) OpenContent(ctx context.Context, channelID int64, entry mountfs.SourceEntry) (mountfs.RandomAccessContent, error) {
	return o.content, nil
}

func testTailWarmFS(t *testing.T, name string, size int) (*readApplication, *tailRecordingContent) {
	t.Helper()
	data := make([]byte, size)
	for i := range data {
		data[i] = byte(i)
	}
	content := &tailRecordingContent{data: data, maxRead: -1}
	modTime := time.Unix(1700000000, 0)
	mfs, err := mountfs.New(42, memorySource{
		mountfs.RootID: {
			{ID: "f:movie", ParentID: mountfs.RootID, Name: name, Kind: mountfs.KindFile, Size: int64(size), ModTime: modTime, ContentRef: "42"},
		},
	}, &tailRecordingOpener{content: content})
	if err != nil {
		t.Fatalf("mountfs.New: %v", err)
	}
	return &readApplication{fs: NewFileSystem(mfs)}, content
}

func serveTestFile(t *testing.T, application *readApplication, target string) *httptest.ResponseRecorder {
	t.Helper()
	request := httptest.NewRequest(http.MethodGet, target, nil)
	request.Header.Set("Range", "bytes=0-100")
	recorder := httptest.NewRecorder()
	application.serveFile(recorder, request)
	return recorder
}

// TestServeFileWarmsMediaTail proves the streaming optimization: the first
// GET of a media file kicks off a background fetch of its tail (where players
// look for the moov atom next), so the follow-up tail read hits warm blocks.
func TestServeFileWarmsMediaTail(t *testing.T) {
	// Fixture must stay above the tail-warm threshold, or the "tail read"
	// assertion below would pass without any background fetch happening.
	const size = 20 * 1024 * 1024
	if size <= mediaTailWarmBytes {
		t.Fatalf("fixture size %d must exceed tail-warm threshold %d", size, mediaTailWarmBytes)
	}
	application, content := testTailWarmFS(t, "movie.mp4", size)
	recorder := serveTestFile(t, application, "/movie.mp4")
	if recorder.Code != http.StatusPartialContent {
		t.Fatalf("GET status = %d, want 206", recorder.Code)
	}
	deadline := time.Now().Add(5 * time.Second)
	for {
		maxRead, _ := content.snapshot()
		if maxRead >= int64(size)-mediaTailWarmBytes {
			return
		}
		if time.Now().After(deadline) {
			t.Fatalf("no tail read within timeout, max offset read = %d, want >= %d", maxRead, int64(size)-mediaTailWarmBytes)
		}
		time.Sleep(20 * time.Millisecond)
	}
}

// A conditional request writes no body, so nothing should be fetched for it:
// the tail warm must not run on a 304.
func TestServeFileConditionalRequestSkipsMediaTailWarm(t *testing.T) {
	const size = 20 * 1024 * 1024
	application, content := testTailWarmFS(t, "movie.mp4", size)
	stat, err := application.fs.Stat(context.Background(), "/movie.mp4")
	if err != nil {
		t.Fatalf("stat: %v", err)
	}
	info, ok := stat.(fileInfo)
	if !ok {
		t.Fatalf("stat type = %T, want fileInfo", stat)
	}
	etag, err := info.ETag(context.Background())
	if err != nil {
		t.Fatalf("etag: %v", err)
	}
	request := httptest.NewRequest(http.MethodGet, "/movie.mp4", nil)
	request.Header.Set("If-None-Match", etag)
	recorder := httptest.NewRecorder()
	application.serveFile(recorder, request)
	if recorder.Code != http.StatusNotModified {
		t.Fatalf("conditional GET status = %d, want 304", recorder.Code)
	}
	time.Sleep(200 * time.Millisecond)
	if maxRead, _ := content.snapshot(); maxRead >= int64(size)-mediaTailWarmBytes {
		t.Fatalf("304 response triggered a tail read (max offset %d)", maxRead)
	}
}

// The tail fetch shares the mount's background getFile budget with the head
// read that produces the first frame, so it must start only once the body is
// being written, and only once.
func TestFirstWriteWarmerDefersWarmUntilBody(t *testing.T) {
	warmed := make(chan struct{}, 2)
	writer := &firstWriteWarmer{
		ResponseWriter: httptest.NewRecorder(),
		warm:           func() { warmed <- struct{}{} },
	}
	writer.WriteHeader(http.StatusOK)
	select {
	case <-warmed:
		t.Fatal("warm ran before the response body was written")
	case <-time.After(50 * time.Millisecond):
	}
	if _, err := writer.Write([]byte("first")); err != nil {
		t.Fatalf("write: %v", err)
	}
	select {
	case <-warmed:
	case <-time.After(time.Second):
		t.Fatal("warm did not run after the first body write")
	}
	if _, err := writer.Write([]byte("second")); err != nil {
		t.Fatalf("second write: %v", err)
	}
	select {
	case <-warmed:
		t.Fatal("warm ran more than once")
	default:
	}
}

// TestServeFileSkipsTailWarmForDocuments ensures the extra background fetch
// only runs for media containers, not for every large download.
func TestServeFileSkipsTailWarmForDocuments(t *testing.T) {
	// Same oversized fixture as the media case: with a file below the
	// threshold, "no tail read" would hold because warming never runs, which
	// would make this test vacuously true.
	const size = 20 * 1024 * 1024
	if size <= mediaTailWarmBytes {
		t.Fatalf("fixture size %d must exceed tail-warm threshold %d", size, mediaTailWarmBytes)
	}
	application, content := testTailWarmFS(t, "notes.txt", size)
	recorder := serveTestFile(t, application, "/notes.txt")
	if recorder.Code != http.StatusPartialContent {
		t.Fatalf("GET status = %d, want 206", recorder.Code)
	}
	time.Sleep(500 * time.Millisecond)
	if maxRead, _ := content.snapshot(); maxRead >= int64(size)-mediaTailWarmBytes {
		t.Fatalf("non-media file triggered a tail read (max offset %d)", maxRead)
	}
}
