package main

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"os"
	"path"
	"path/filepath"
	"time"
)

// retryDelay keeps one bad file (locked, already present on the host, or a
// transient failure) from stalling the whole folder: it is set aside for a
// while and the next file goes ahead.
const retryDelay = 10 * time.Minute

// failedFile remembers the version of a file whose backup failed, so the retry
// waits for retryDelay and a changed file is retried immediately.
type failedFile struct {
	stamp fileStamp
	until time.Time
}

// agent runs the backup pipeline: one file at a time, oldest first. A file is
// uploaded, confirmed against the host's listing, and only then removed from
// the local folder. Nothing ever deletes the Telegram copy, so removing the
// original later does not touch the archive.
type agent struct {
	cfg           Config
	client        *apiClient
	journal       *journalStore
	seen          map[string]fileStamp
	failed        map[string]failedFile
	logf          func(format string, args ...any)
	uploadTimeout time.Duration
	retryDelay    time.Duration
}

func newAgent(cfg Config, client *apiClient, store *journalStore, logf func(string, ...any)) *agent {
	return &agent{
		cfg:           cfg,
		client:        client,
		journal:       store,
		seen:          map[string]fileStamp{},
		failed:        map[string]failedFile{},
		logf:          logf,
		uploadTimeout: cfg.uploadTimeout(),
		retryDelay:    retryDelay,
	}
}

// remotePath is where a file lands: the backup root plus the folder structure it
// had locally, so two files with the same name in different folders never
// collide.
func (a *agent) remotePath(rel string) (dir, name string) {
	rel = filepath.ToSlash(rel)
	dir = path.Join(a.cfg.Remote, path.Dir(rel))
	if dir == "." {
		dir = a.cfg.Remote
	}
	return dir, path.Base(rel)
}

// pass handles at most one successful file, so the loop re-scans after each
// upload and new arrivals are picked up in order without building a queue in
// memory. A file that fails is set aside for retryDelay and the pass moves on,
// except for an authentication failure: that affects every file, so it is
// returned and the loop keeps waiting instead of failing each file in turn.
func (a *agent) pass(ctx context.Context) (bool, error) {
	files, err := scanFolder(a.cfg.Folder, a.skipPaths())
	if err != nil {
		return false, err
	}
	defer rememberStamps(a.seen, files)
	if a.failed == nil {
		a.failed = map[string]failedFile{}
	}

	now := time.Now()
	for _, file := range files {
		if _, archived := a.journal.done(file.rel, file.size, file.stamp.mod); archived {
			continue
		}
		if retry, ok := a.failed[file.rel]; ok && retry.stamp == file.stamp && now.Before(retry.until) {
			continue
		}
		if !readyForUpload(file, a.seen, a.cfg.settle(), now) {
			continue
		}
		if err := a.archive(ctx, file); err != nil {
			if isAuthFailure(err) {
				return false, err
			}
			a.failed[file.rel] = failedFile{stamp: file.stamp, until: now.Add(a.retryDelay)}
			a.logf("error: %v (keeping %s, retrying in %s)", err, file.rel, a.retryDelay)
			continue
		}
		delete(a.failed, file.rel)
		return true, nil
	}
	return false, nil
}

func isAuthFailure(err error) bool {
	var apiErr *apiError
	return errors.As(err, &apiErr) && apiErr.Status == http.StatusForbidden
}

func (a *agent) archive(ctx context.Context, file candidate) error {
	remoteDir, name := a.remotePath(file.rel)
	if err := a.client.EnsureDir(ctx, remoteDir); err != nil {
		return fmt.Errorf("remote folder %s: %w", remoteDir, err)
	}
	// A crash between the upload and the journal write leaves the file on the
	// host with no local record. Finding it first finishes that job instead of
	// storing a second Telegram copy.
	listed, err := a.client.RemoteHas(ctx, remoteDir, name, file.size)
	if err != nil {
		return fmt.Errorf("check %s: %w", file.rel, err)
	}
	if listed {
		a.logf("%s is already on the host", file.rel)
	} else {
		a.logf("uploading %s (%s)", file.rel, formatBytes(file.size))
		if err := a.client.UploadFile(ctx, file.path, remoteDir, a.uploadTimeout); err != nil {
			return err
		}
		listed, err = a.client.RemoteHas(ctx, remoteDir, name, file.size)
		if err != nil {
			return fmt.Errorf("verify %s: %w", file.rel, err)
		}
		if !listed {
			return fmt.Errorf("host did not list %s with %d bytes; keeping the local file", path.Join(remoteDir, name), file.size)
		}
	}
	entry := uploadedFile{
		Size:    file.size,
		ModTime: file.stamp.mod,
		Remote:  path.Join(remoteDir, name),
		At:      time.Now(),
	}
	if err := a.journal.record(file.rel, entry); err != nil {
		return err
	}
	if a.cfg.KeepOriginal {
		a.logf("archived %s -> %s (original kept)", file.rel, entry.Remote)
		return nil
	}
	if err := removeLocal(file.path); err != nil {
		return fmt.Errorf("archived %s but could not remove the original: %w", file.rel, err)
	}
	a.logf("archived %s -> %s (original removed)", file.rel, entry.Remote)
	return nil
}

func removeLocal(path string) error {
	if err := os.Remove(path); err != nil {
		return err
	}
	// Empty folders are left alone: the user picked the folder layout, and a
	// missing folder would make the next pass scan fail.
	return nil
}

// skipPaths keeps the agent from archiving its own journal, lock and log when a
// user points the backup folder at the configuration directory.
func (a *agent) skipPaths() map[string]bool {
	skip := map[string]bool{}
	for _, name := range []string{a.journal.path, a.journal.path + ".tmp", a.cfg.LockPath, a.cfg.LogPath, a.cfg.Path} {
		if name == "" {
			continue
		}
		if absolute, err := filepath.Abs(name); err == nil {
			skip[absolute] = true
		}
	}
	return skip
}

func (a *agent) run(ctx context.Context) error {
	a.logf("watching %s -> %s on %s", a.cfg.Folder, a.cfg.Remote, a.cfg.Host)
	for {
		processed, err := a.pass(ctx)
		if err != nil {
			a.logf("error: %v", err)
		}
		if ctx.Err() != nil {
			return nil
		}
		wait := a.cfg.interval()
		if processed {
			wait = time.Second
		}
		select {
		case <-ctx.Done():
			return nil
		case <-time.After(wait):
		}
	}
}
