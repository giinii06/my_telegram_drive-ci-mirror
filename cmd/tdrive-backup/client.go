package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

// apiClient talks to the TDrive web server over the tailnet. Uploads use the
// same endpoints the browser uses, so the agent needs no daemon socket and no
// new server API: POST /api/upload streams the file to the host, and the host
// moves it into Telegram in the background under the job it returns.
type apiClient struct {
	base  *url.URL
	token string
	http  *http.Client
	poll  time.Duration
}

type apiError struct {
	Status  int
	Message string
}

func (e *apiError) Error() string {
	if e.Status > 0 {
		return fmt.Sprintf("%s (HTTP %d)", e.Message, e.Status)
	}
	return e.Message
}

// newAPIClient accepts "100.64.0.1:8080", "host", or a full http(s) URL.
// Tailscale names have no dots-and-port requirement, so a missing scheme and a
// missing port both get the desktop-install defaults.
func newAPIClient(host, token string) (*apiClient, error) {
	raw := strings.TrimSpace(host)
	if raw == "" {
		return nil, fmt.Errorf("host is required")
	}
	if !strings.Contains(raw, "://") {
		raw = "http://" + raw
	}
	parsed, err := url.Parse(raw)
	if err != nil {
		return nil, fmt.Errorf("invalid host %q: %w", host, err)
	}
	if parsed.Scheme != "http" && parsed.Scheme != "https" {
		return nil, fmt.Errorf("invalid host %q: use http or https", host)
	}
	if parsed.Hostname() == "" {
		return nil, fmt.Errorf("invalid host %q", host)
	}
	if parsed.Path != "" && parsed.Path != "/" {
		return nil, fmt.Errorf("host %q must not carry a path; set the remote folder instead", host)
	}
	if parsed.Port() == "" {
		parsed.Host = parsed.Hostname() + ":8080"
	}
	parsed.Path, parsed.RawQuery = "", ""
	return &apiClient{
		base:  parsed,
		token: token,
		http:  &http.Client{Transport: http.DefaultTransport.(*http.Transport).Clone()},
		poll:  time.Second,
	}, nil
}

func (c *apiClient) url(endpoint string, query url.Values) string {
	target := *c.base
	target.Path = endpoint
	if c.token != "" {
		query.Set("token", c.token)
	}
	target.RawQuery = query.Encode()
	return target.String()
}

type uploadStatus struct {
	Name  string `json:"name"`
	Size  int64  `json:"size"`
	Stage string `json:"stage"`
	Error string `json:"error"`
}

type uploadJob struct {
	ID    string         `json:"id"`
	Files []uploadStatus `json:"files"`
}

type apiEnvelope struct {
	OK    bool       `json:"ok"`
	Error string     `json:"error"`
	Job   *uploadJob `json:"job"`
}

func (c *apiClient) doJSON(request *http.Request, out any) error {
	response, err := c.http.Do(request)
	if err != nil {
		return err
	}
	defer response.Body.Close()
	if response.StatusCode == http.StatusForbidden {
		io.Copy(io.Discard, response.Body)
		return &apiError{Status: response.StatusCode, Message: "invalid or missing access token"}
	}
	body, readErr := io.ReadAll(io.LimitReader(response.Body, 1<<20))
	if readErr != nil {
		return readErr
	}
	var envelope apiEnvelope
	if err := json.Unmarshal(body, &envelope); err != nil {
		if response.StatusCode >= 300 {
			return &apiError{Status: response.StatusCode, Message: strings.TrimSpace(string(body))}
		}
		return fmt.Errorf("unexpected response: %s", strings.TrimSpace(string(body)))
	}
	if !envelope.OK {
		message := envelope.Error
		if message == "" {
			message = "server rejected the request"
		}
		return &apiError{Status: response.StatusCode, Message: message}
	}
	if out != nil {
		if err := json.Unmarshal(body, out); err != nil {
			return fmt.Errorf("decode response: %w", err)
		}
	}
	return nil
}

// Ping proves the host is reachable and the token is right before a pass starts
// moving files.
func (c *apiClient) Ping(ctx context.Context) error {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.url("/api/status", url.Values{}), nil)
	if err != nil {
		return err
	}
	return c.doJSON(request, nil)
}

// EnsureDir creates the remote backup folder. The server's mkdir is recursive,
// so nested folder names work.
func (c *apiClient) EnsureDir(ctx context.Context, dir string) error {
	payload, err := json.Marshal(map[string]string{"path": dir})
	if err != nil {
		return err
	}
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, c.url("/api/mkdir", url.Values{}), strings.NewReader(string(payload)))
	if err != nil {
		return err
	}
	request.Header.Set("Content-Type", "application/json")
	return c.doJSON(request, nil)
}

// UploadFile streams one local file into remoteDir and waits until the host
// reports the Telegram upload finished.
func (c *apiClient) UploadFile(ctx context.Context, localPath, remoteDir string, timeout time.Duration) error {
	file, err := os.Open(localPath)
	if err != nil {
		return fmt.Errorf("open %s: %w", localPath, err)
	}
	defer file.Close()
	info, err := file.Stat()
	if err != nil {
		return err
	}

	reader, writer := io.Pipe()
	form := multipart.NewWriter(writer)
	go func() {
		part, createErr := form.CreateFormFile("files", filepath.Base(localPath))
		if createErr != nil {
			writer.CloseWithError(createErr)
			return
		}
		if _, copyErr := io.Copy(part, file); copyErr != nil {
			writer.CloseWithError(copyErr)
			return
		}
		if closeErr := form.Close(); closeErr != nil {
			writer.CloseWithError(closeErr)
			return
		}
		writer.Close()
	}()

	// Store the archive as-is. An encrypted upload keeps ciphertext on the
	// host, whose byte count differs from the local file, so the size check
	// that frees the original could never pass; it would also need the vault
	// unlocked after every reboot of an unattended host.
	query := url.Values{}
	query.Set("path", remoteDir)
	query.Set("encrypt", "0")
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, c.url("/api/upload", query), reader)
	if err != nil {
		return err
	}
	request.Header.Set("Content-Type", form.FormDataContentType())
	request.ContentLength = -1

	var accepted struct {
		Job *uploadJob `json:"job"`
	}
	if err := c.doJSON(request, &accepted); err != nil {
		return fmt.Errorf("upload %s: %w", filepath.Base(localPath), err)
	}
	if accepted.Job == nil || accepted.Job.ID == "" {
		return fmt.Errorf("upload %s: server returned no job id", filepath.Base(localPath))
	}

	deadline := time.NewTimer(timeout)
	defer deadline.Stop()
	for {
		state, err := c.uploadState(ctx, accepted.Job.ID)
		if err != nil {
			return fmt.Errorf("upload %s: %w", filepath.Base(localPath), err)
		}
		done, failure := summarizeUpload(state)
		if failure != "" {
			return fmt.Errorf("upload %s: %s", filepath.Base(localPath), failure)
		}
		if done {
			if len(state.Files) != 1 || state.Files[0].Size != info.Size() {
				return fmt.Errorf("upload %s: server stored %d bytes, want %d", filepath.Base(localPath), storedSize(state), info.Size())
			}
			return nil
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-deadline.C:
			return fmt.Errorf("upload %s: timed out after %s", filepath.Base(localPath), timeout)
		case <-time.After(c.poll):
		}
	}
}

func storedSize(job uploadJob) int64 {
	var total int64
	for _, file := range job.Files {
		total += file.Size
	}
	return total
}

// summarizeUpload folds per-file stages into one verdict. Any error fails the
// job; every file must report "done" before the caller deletes the original.
func summarizeUpload(job uploadJob) (done bool, failure string) {
	if len(job.Files) == 0 {
		return false, "upload job reported no files"
	}
	done = true
	for _, file := range job.Files {
		switch file.Stage {
		case "done":
		case "error":
			if file.Error != "" {
				return false, file.Error
			}
			return false, "server reported an upload error"
		default:
			done = false
		}
	}
	return done, ""
}

func (c *apiClient) uploadState(ctx context.Context, id string) (uploadJob, error) {
	query := url.Values{}
	query.Set("id", id)
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.url("/api/upload/status", query), nil)
	if err != nil {
		return uploadJob{}, err
	}
	var out struct {
		Job uploadJob `json:"job"`
	}
	if err := c.doJSON(request, &out); err != nil {
		return uploadJob{}, err
	}
	return out.Job, nil
}

type remoteEntry struct {
	Name string `json:"name"`
	Type string `json:"type"`
	Size int64  `json:"size"`
}

// RemoteHas is the post-upload check. The deletion only happens when the host
// lists the file with the same byte count, so a dropped connection cannot take
// the only copy with it.
func (c *apiClient) RemoteHas(ctx context.Context, dir, name string, size int64) (bool, error) {
	query := url.Values{}
	query.Set("path", dir)
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.url("/api/list", query), nil)
	if err != nil {
		return false, err
	}
	var out struct {
		Entries []remoteEntry `json:"entries"`
	}
	if err := c.doJSON(request, &out); err != nil {
		return false, err
	}
	for _, entry := range out.Entries {
		if entry.Name != name {
			continue
		}
		return entry.Type == "file" && entry.Size == size, nil
	}
	return false, nil
}

func formatBytes(size int64) string {
	const unit = 1024
	if size < unit {
		return strconv.FormatInt(size, 10) + " B"
	}
	div, exp := int64(unit), 0
	for n := size / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(size)/float64(div), "KMGT"[exp])
}
