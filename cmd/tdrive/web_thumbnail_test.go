package main

import (
	"bytes"
	"encoding/base64"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"TDrive/backend/daemon"
)

var (
	testFileThumbnailOut      daemon.FileThumbnailResponse
	testFileThumbnailErr      error
	testFileThumbnailStoreOut daemon.FileThumbnailStoreResponse
	testFileThumbnailStoreErr error
	storedThumbnailPath       string
	storedThumbnailData       string
)

func (f *fakeWebClient) FileThumbnailInDrive(_ int64, _ string) (daemon.FileThumbnailResponse, error) {
	return testFileThumbnailOut, testFileThumbnailErr
}

func (f *fakeWebClient) StoreFileThumbnailInDrive(_ int64, remotePath string, dataBase64 string) (daemon.FileThumbnailStoreResponse, error) {
	storedThumbnailPath, storedThumbnailData = remotePath, dataBase64
	return testFileThumbnailStoreOut, testFileThumbnailStoreErr
}

func TestWebThumbnailServesStoredPoster(t *testing.T) {
	raw := []byte("\xff\xd8\xff\xd9")
	testFileThumbnailOut = daemon.FileThumbnailResponse{ContentType: "image/jpeg", DataBase64: base64.StdEncoding.EncodeToString(raw)}
	t.Cleanup(func() { testFileThumbnailOut = daemon.FileThumbnailResponse{} })

	handler := newWebTestServer(&fakeWebClient{}, nil).routes()
	request := httptest.NewRequest(http.MethodGet, webauthed("/api/thumb?path=/clip.mp4"), nil)
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusOK {
		t.Fatalf("thumbnail status = %d, body %s", recorder.Code, recorder.Body.String())
	}
	if recorder.Body.String() != string(raw) {
		t.Fatalf("thumbnail body = %q, want stored bytes", recorder.Body.String())
	}
	if got := recorder.Header().Get("Content-Type"); got != "image/jpeg" {
		t.Fatalf("Content-Type = %q, want image/jpeg", got)
	}
	if got := recorder.Header().Get("Cache-Control"); !strings.Contains(got, "private") {
		t.Fatalf("Cache-Control = %q, want private", got)
	}
}

func TestWebThumbnailMissingAnswers404(t *testing.T) {
	testFileThumbnailErr = errors.New("missing_rendition")
	t.Cleanup(func() { testFileThumbnailErr = nil })

	handler := newWebTestServer(&fakeWebClient{}, nil).routes()
	request := httptest.NewRequest(http.MethodGet, webauthed("/api/thumb?path=/clip.mp4"), nil)
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusNotFound {
		t.Fatalf("missing thumbnail status = %d, want 404", recorder.Code)
	}
	if got := recorder.Header().Get("Cache-Control"); got != "no-store" {
		t.Fatalf("missing thumbnail cache = %q, want no-store", got)
	}
}

func TestWebThumbnailStoreForwardsPoster(t *testing.T) {
	storedThumbnailPath, storedThumbnailData = "", ""
	body, err := json.Marshal(map[string]string{"path": "/clip.mp4", "data_base64": "QUJD"})
	if err != nil {
		t.Fatal(err)
	}
	handler := newWebTestServer(&fakeWebClient{}, nil).routes()
	request := httptest.NewRequest(http.MethodPost, webauthed("/api/thumb?path=/clip.mp4"), bytes.NewReader(body))
	request.Header.Set("Content-Type", "application/json")
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusOK {
		t.Fatalf("store status = %d, body %s", recorder.Code, recorder.Body.String())
	}
	if storedThumbnailPath != "/clip.mp4" || storedThumbnailData != "QUJD" {
		t.Fatalf("forwarded thumbnail = %q %q", storedThumbnailPath, storedThumbnailData)
	}
}

func TestWebThumbnailStoreRejectsEmptyPayload(t *testing.T) {
	storedThumbnailPath = ""
	body, _ := json.Marshal(map[string]string{"path": "/clip.mp4"})
	handler := newWebTestServer(&fakeWebClient{}, nil).routes()
	request := httptest.NewRequest(http.MethodPost, webauthed("/api/thumb"), bytes.NewReader(body))
	request.Header.Set("Content-Type", "application/json")
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusBadRequest {
		t.Fatalf("empty thumbnail status = %d, want 400", recorder.Code)
	}
	if storedThumbnailPath != "" {
		t.Fatal("empty thumbnail reached the daemon")
	}
}
