package main

import (
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"testing"
	"time"
)

// fakeHost implements the three endpoints the agent uses. It keeps uploads in
// memory and can report a finished upload either immediately or after one poll.
type fakeHost struct {
	mu           sync.Mutex
	token        string
	files        map[string]int64
	uploads      []string
	dirs         []string
	jobs         map[string][]uploadStatus
	jobSeq       int
	polls        int
	pending      int
	stageError   string
	hideFromList bool
	encryptModes []string
}

func newFakeHost(token string) *fakeHost {
	return &fakeHost{token: token, files: map[string]int64{}, jobs: map[string][]uploadStatus{}}
}

func (f *fakeHost) ServeHTTP(response http.ResponseWriter, request *http.Request) {
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.token != "" && request.URL.Query().Get("token") != f.token {
		writeTestJSON(response, http.StatusForbidden, map[string]any{"ok": false, "error": "missing or invalid token"})
		return
	}
	switch request.URL.Path {
	case "/api/status":
		writeTestJSON(response, http.StatusOK, map[string]any{"ok": true, "status": map[string]any{}})
	case "/api/mkdir":
		var body struct {
			Path string `json:"path"`
		}
		_ = json.NewDecoder(request.Body).Decode(&body)
		f.dirs = append(f.dirs, body.Path)
		writeTestJSON(response, http.StatusOK, map[string]any{"ok": true})
	case "/api/upload":
		reader, err := request.MultipartReader()
		if err != nil {
			writeTestJSON(response, http.StatusBadRequest, map[string]any{"ok": false, "error": err.Error()})
			return
		}
		dir := request.URL.Query().Get("path")
		f.encryptModes = append(f.encryptModes, request.URL.Query().Get("encrypt"))
		var name string
		var size int64
		for {
			part, err := reader.NextPart()
			if err == io.EOF {
				break
			}
			if err != nil {
				writeTestJSON(response, http.StatusBadRequest, map[string]any{"ok": false, "error": err.Error()})
				return
			}
			name = part.FileName()
			body, _ := io.ReadAll(part)
			size = int64(len(body))
		}
		remote := path.Join(dir, name)
		f.files[remote] = size
		f.uploads = append(f.uploads, remote)
		f.jobSeq++
		id := "job-" + strconv.Itoa(f.jobSeq)
		f.jobs[id] = []uploadStatus{{Name: name, Size: size, Stage: "telegram"}}
		writeTestJSON(response, http.StatusOK, map[string]any{"ok": true, "job": map[string]any{
			"id": id,
			"files": []map[string]any{{
				"name": name, "size": size, "stage": "telegram",
			}},
		}})
	case "/api/upload/status":
		f.polls++
		stage := "telegram"
		if f.stageError != "" {
			stage = "error"
		} else if f.polls > f.pending {
			stage = "done"
		}
		job := f.jobs[request.URL.Query().Get("id")]
		files := make([]map[string]any, 0, len(job))
		for _, file := range job {
			files = append(files, map[string]any{
				"name": file.Name, "size": file.Size, "stage": stage, "error": f.stageError,
			})
		}
		writeTestJSON(response, http.StatusOK, map[string]any{"ok": true, "job": map[string]any{"id": "job-1", "files": files}})
	case "/api/list":
		dir := request.URL.Query().Get("path")
		entries := []map[string]any{}
		if !f.hideFromList {
			for remote, size := range f.files {
				if path.Dir(remote) != dir {
					continue
				}
				entries = append(entries, map[string]any{"name": path.Base(remote), "type": "file", "size": size})
			}
		}
		writeTestJSON(response, http.StatusOK, map[string]any{"ok": true, "entries": entries})
	default:
		writeTestJSON(response, http.StatusNotFound, map[string]any{"ok": false, "error": "not found"})
	}
}

func writeTestJSON(response http.ResponseWriter, status int, body any) {
	response.Header().Set("Content-Type", "application/json")
	response.WriteHeader(status)
	_ = json.NewEncoder(response).Encode(body)
}

func testConfig(t *testing.T, host string, folder string) Config {
	t.Helper()
	dir := t.TempDir()
	return Config{
		Host:            host,
		Token:           "secret",
		Folder:          folder,
		Remote:          "/Backup",
		IntervalSeconds: 5,
		SettleSeconds:   0,
		Path:            filepath.Join(dir, "config.json"),
		LockPath:        filepath.Join(dir, "agent.lock"),
		LogPath:         filepath.Join(dir, "backup.log"),
	}
}

func testAgent(t *testing.T, cfg Config, host *fakeHost) *agent {
	t.Helper()
	client, err := newAPIClient(cfg.Host, cfg.Token)
	if err != nil {
		t.Fatalf("client: %v", err)
	}
	journalPath := filepath.Join(filepath.Dir(cfg.Path), "journal.json")
	store, err := loadJournal(journalPath)
	if err != nil {
		t.Fatalf("journal: %v", err)
	}
	return newAgent(cfg, client, store, func(string, ...any) {})
}

func writeFile(t *testing.T, path string, body string, mod time.Time) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(body), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Chtimes(path, mod, mod); err != nil {
		t.Fatal(err)
	}
}

func TestPassArchivesOldestFileFirstAndRemovesIt(t *testing.T) {
	folder := t.TempDir()
	now := time.Now()
	writeFile(t, filepath.Join(folder, "newer.txt"), "bbbb", now)
	writeFile(t, filepath.Join(folder, "older.txt"), "aaaa", now.Add(-time.Hour))

	host := newFakeHost("secret")
	server := httptest.NewServer(host)
	defer server.Close()
	cfg := testConfig(t, server.URL, folder)
	agent := testAgent(t, cfg, host)

	// The first pass only records stamps: nothing is old enough to be trusted.
	if processed, err := agent.pass(context.Background()); processed || err != nil {
		t.Fatalf("first pass = %v, %v; want no work", processed, err)
	}
	if len(host.uploads) != 0 {
		t.Fatalf("first pass uploaded %v", host.uploads)
	}

	processed, err := agent.pass(context.Background())
	if err != nil || !processed {
		t.Fatalf("second pass = %v, %v; want one upload", processed, err)
	}
	if len(host.uploads) != 1 || host.uploads[0] != "/Backup/older.txt" {
		t.Fatalf("uploads = %v, want the oldest file first", host.uploads)
	}
	if _, err := os.Stat(filepath.Join(folder, "older.txt")); !os.IsNotExist(err) {
		t.Fatalf("older.txt still exists after a verified upload: %v", err)
	}
	if _, err := os.Stat(filepath.Join(folder, "newer.txt")); err != nil {
		t.Fatalf("newer.txt was touched before its turn: %v", err)
	}
	if _, done := agent.journal.done("older.txt", 4, now.Add(-time.Hour).UnixNano()); !done {
		t.Fatal("journal did not record the archived file")
	}

	if processed, err := agent.pass(context.Background()); err != nil || !processed {
		t.Fatalf("third pass = %v, %v; want the next file", processed, err)
	}
	if len(host.uploads) != 2 || host.uploads[1] != "/Backup/newer.txt" {
		t.Fatalf("uploads = %v", host.uploads)
	}
	if _, err := os.Stat(filepath.Join(folder, "newer.txt")); !os.IsNotExist(err) {
		t.Fatalf("newer.txt still exists: %v", err)
	}
	// Plaintext is what makes the byte-count verification meaningful: an
	// encrypted upload stores ciphertext whose size differs from the file.
	if len(host.encryptModes) != 2 {
		t.Fatalf("encrypt modes = %v", host.encryptModes)
	}
	for _, mode := range host.encryptModes {
		if mode != "0" {
			t.Fatalf("upload used encrypt=%q, want 0", mode)
		}
	}
}

func TestPreexistingRemoteFileFinishesWithoutUpload(t *testing.T) {
	folder := t.TempDir()
	mod := time.Now().Add(-time.Hour)
	writeFile(t, filepath.Join(folder, "a.txt"), "aaaa", mod)

	host := newFakeHost("secret")
	host.files["/Backup/a.txt"] = 4
	server := httptest.NewServer(host)
	defer server.Close()
	cfg := testConfig(t, server.URL, folder)
	agent := testAgent(t, cfg, host)

	agent.pass(context.Background())
	if processed, err := agent.pass(context.Background()); err != nil || !processed {
		t.Fatalf("pass = %v, %v; want the file finished", processed, err)
	}
	if len(host.uploads) != 0 {
		t.Fatalf("re-uploaded a file the host already had: %v", host.uploads)
	}
	if _, err := os.Stat(filepath.Join(folder, "a.txt")); !os.IsNotExist(err) {
		t.Fatalf("already-archived file was left behind: %v", err)
	}
	if _, done := agent.journal.done("a.txt", 4, mod.UnixNano()); !done {
		t.Fatal("journal did not record the pre-existing copy")
	}
}

func TestKeepOriginalLeavesFileInPlace(t *testing.T) {
	folder := t.TempDir()
	file := filepath.Join(folder, "keep.txt")
	writeFile(t, file, "keep", time.Now().Add(-time.Hour))

	host := newFakeHost("secret")
	server := httptest.NewServer(host)
	defer server.Close()
	cfg := testConfig(t, server.URL, folder)
	cfg.KeepOriginal = true
	agent := testAgent(t, cfg, host)

	agent.pass(context.Background())
	if _, err := agent.pass(context.Background()); err != nil {
		t.Fatalf("pass: %v", err)
	}
	if _, err := os.Stat(file); err != nil {
		t.Fatalf("keep_original removed the file: %v", err)
	}
	if len(host.uploads) != 1 {
		t.Fatalf("uploads = %v", host.uploads)
	}
}

func TestUnverifiedUploadKeepsFileAndSkipsToNext(t *testing.T) {
	folder := t.TempDir()
	writeFile(t, filepath.Join(folder, "a.txt"), "a", time.Now().Add(-2*time.Hour))

	host := newFakeHost("secret")
	host.hideFromList = true
	server := httptest.NewServer(host)
	defer server.Close()
	cfg := testConfig(t, server.URL, folder)
	agent := testAgent(t, cfg, host)

	agent.pass(context.Background())
	processed, err := agent.pass(context.Background())
	if err != nil || processed {
		t.Fatalf("pass = %v, %v; want the failed file set aside", processed, err)
	}
	if _, err := os.Stat(filepath.Join(folder, "a.txt")); err != nil {
		t.Fatalf("unverified file was removed: %v", err)
	}
	if len(host.uploads) != 1 {
		t.Fatalf("uploads = %v", host.uploads)
	}
	if len(agent.journal.data.Uploaded) != 0 {
		t.Fatalf("journal recorded an unverified file: %v", agent.journal.data.Uploaded)
	}

	// The next pass must not retry the same file inside the retry window, and a
	// newer file has to be picked up instead of waiting behind it forever.
	host.hideFromList = false
	writeFile(t, filepath.Join(folder, "b.txt"), "b", time.Now().Add(-time.Hour))
	if processed, err := agent.pass(context.Background()); err != nil || processed {
		t.Fatalf("stamp pass = %v, %v", processed, err)
	}
	if processed, err := agent.pass(context.Background()); err != nil || !processed {
		t.Fatalf("next file pass = %v, %v; want b.txt uploaded", processed, err)
	}
	if len(host.uploads) != 2 || host.uploads[1] != "/Backup/b.txt" {
		t.Fatalf("uploads = %v, want a.txt then b.txt", host.uploads)
	}
}

func TestUploadErrorKeepsFile(t *testing.T) {
	folder := t.TempDir()
	writeFile(t, filepath.Join(folder, "a.txt"), "a", time.Now().Add(-time.Hour))

	host := newFakeHost("secret")
	host.stageError = "telegram rejected the file"
	server := httptest.NewServer(host)
	defer server.Close()
	cfg := testConfig(t, server.URL, folder)
	agent := testAgent(t, cfg, host)

	agent.pass(context.Background())
	if processed, err := agent.pass(context.Background()); err != nil || processed {
		t.Fatalf("pass = %v, %v; want the failure set aside", processed, err)
	}
	if _, err := os.Stat(filepath.Join(folder, "a.txt")); err != nil {
		t.Fatalf("file removed after a failed upload: %v", err)
	}
	if len(host.uploads) != 1 {
		t.Fatalf("uploads = %v; the retry window must hold the file back", host.uploads)
	}
}

func TestJournalSkipsArchivedFilesAcrossRestart(t *testing.T) {
	folder := t.TempDir()
	writeFile(t, filepath.Join(folder, "a.txt"), "a", time.Now().Add(-time.Hour))

	host := newFakeHost("secret")
	server := httptest.NewServer(host)
	defer server.Close()
	cfg := testConfig(t, server.URL, folder)
	agent := testAgent(t, cfg, host)
	agent.pass(context.Background())
	if _, err := agent.pass(context.Background()); err != nil {
		t.Fatalf("pass: %v", err)
	}

	// Same journal, fresh agent: the archived file must not be uploaded again.
	restarted := testAgent(t, cfg, host)
	restarted.pass(context.Background())
	if processed, err := restarted.pass(context.Background()); processed || err != nil {
		t.Fatalf("restarted pass = %v, %v; want nothing to do", processed, err)
	}
	if len(host.uploads) != 1 {
		t.Fatalf("uploads = %v, want one", host.uploads)
	}
}

func TestFileChangedBetweenScansIsDeferred(t *testing.T) {
	folder := t.TempDir()
	file := filepath.Join(folder, "growing.txt")
	writeFile(t, file, "part", time.Now().Add(-time.Hour))

	host := newFakeHost("secret")
	server := httptest.NewServer(host)
	defer server.Close()
	cfg := testConfig(t, server.URL, folder)
	cfg.SettleSeconds = 60
	agent := testAgent(t, cfg, host)

	agent.pass(context.Background())
	writeFile(t, file, "part two", time.Now().Add(-time.Minute))
	if processed, err := agent.pass(context.Background()); processed || err != nil {
		t.Fatalf("changed file pass = %v, %v; want deferred", processed, err)
	}
	if len(host.uploads) != 0 {
		t.Fatalf("uploads = %v", host.uploads)
	}
}

func TestRemotePathMirrorsLocalFolders(t *testing.T) {
	cfg := Config{Remote: "/Backup"}
	agent := &agent{cfg: cfg}
	dir, name := agent.remotePath("photos/2024/a.jpg")
	if dir != "/Backup/photos/2024" || name != "a.jpg" {
		t.Fatalf("remotePath = %q %q", dir, name)
	}
	dir, name = agent.remotePath("a.jpg")
	if dir != "/Backup" || name != "a.jpg" {
		t.Fatalf("remotePath = %q %q", dir, name)
	}
}

func TestAPIClientDefaultsHostAndPort(t *testing.T) {
	client, err := newAPIClient("100.64.0.7", "")
	if err != nil {
		t.Fatalf("newAPIClient: %v", err)
	}
	if got := client.base.String(); got != "http://100.64.0.7:8080" {
		t.Fatalf("base = %q", got)
	}
	if _, err := newAPIClient("https://tdrive.example:8443/", "t"); err != nil {
		t.Fatalf("full URL rejected: %v", err)
	}
	if _, err := newAPIClient("100.64.0.7/path", "t"); err == nil {
		t.Fatal("host with a path accepted")
	}
}

func TestWrongTokenStopsThePass(t *testing.T) {
	folder := t.TempDir()
	writeFile(t, filepath.Join(folder, "a.txt"), "a", time.Now().Add(-time.Hour))

	host := newFakeHost("secret")
	server := httptest.NewServer(host)
	defer server.Close()
	cfg := testConfig(t, server.URL, folder)
	cfg.Token = "wrong"
	agent := testAgent(t, cfg, host)

	agent.pass(context.Background())
	if _, err := agent.pass(context.Background()); err == nil || !strings.Contains(err.Error(), "token") {
		t.Fatalf("error = %v, want a token message", err)
	}
	if _, err := os.Stat(filepath.Join(folder, "a.txt")); err != nil {
		t.Fatalf("file removed despite the rejected token: %v", err)
	}
}

func TestSummarizeUpload(t *testing.T) {
	if done, failure := summarizeUpload(uploadJob{Files: []uploadStatus{{Stage: "done"}}}); !done || failure != "" {
		t.Fatalf("done job = %v %q", done, failure)
	}
	if done, _ := summarizeUpload(uploadJob{Files: []uploadStatus{{Stage: "telegram"}}}); done {
		t.Fatal("pending job reported done")
	}
	if _, failure := summarizeUpload(uploadJob{Files: []uploadStatus{{Stage: "error", Error: "boom"}}}); failure != "boom" {
		t.Fatalf("failure = %q", failure)
	}
	if _, failure := summarizeUpload(uploadJob{}); failure == "" {
		t.Fatal("empty job reported no failure")
	}
}
