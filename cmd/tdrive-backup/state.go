package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"time"

	"TDrive/backend/processlock"
)

// uploadedFile records one finished upload. The stamp is kept so a file that
// changed after its backup is uploaded again, while an unchanged one is left
// alone across restarts.
type uploadedFile struct {
	Size    int64     `json:"size"`
	ModTime int64     `json:"mod_time"`
	Remote  string    `json:"remote"`
	At      time.Time `json:"at"`
}

type journal struct {
	Uploaded  map[string]uploadedFile `json:"uploaded"`
	UpdatedAt time.Time               `json:"updated_at"`
}

type journalStore struct {
	path string
	data journal
}

func loadJournal(path string) (*journalStore, error) {
	store := &journalStore{path: path, data: journal{Uploaded: map[string]uploadedFile{}}}
	body, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		return store, nil
	}
	if err != nil {
		return nil, fmt.Errorf("read journal: %w", err)
	}
	if err := json.Unmarshal(body, &store.data); err != nil {
		// A damaged journal is quarantined rather than silently replaced: it is
		// the only record of which local files were already archived.
		broken := path + ".broken-" + strconv.FormatInt(time.Now().Unix(), 10)
		_ = os.Rename(path, broken)
		return nil, fmt.Errorf("journal %s was unreadable and moved to %s: %w", path, broken, err)
	}
	if store.data.Uploaded == nil {
		store.data.Uploaded = map[string]uploadedFile{}
	}
	return store, nil
}

func (j *journalStore) done(rel string, size int64, mod int64) (uploadedFile, bool) {
	entry, ok := j.data.Uploaded[rel]
	if !ok || entry.Size != size || entry.ModTime != mod {
		return uploadedFile{}, false
	}
	return entry, true
}

func (j *journalStore) record(rel string, entry uploadedFile) error {
	j.data.Uploaded[rel] = entry
	j.data.UpdatedAt = time.Now()
	return j.save()
}

func (j *journalStore) save() error {
	if err := os.MkdirAll(filepath.Dir(j.path), 0o700); err != nil {
		return fmt.Errorf("create journal dir: %w", err)
	}
	body, err := json.MarshalIndent(j.data, "", "  ")
	if err != nil {
		return err
	}
	temp := j.path + ".tmp"
	if err := os.WriteFile(temp, body, 0o600); err != nil {
		return fmt.Errorf("write journal: %w", err)
	}
	if err := os.Rename(temp, j.path); err != nil {
		return fmt.Errorf("replace journal: %w", err)
	}
	return nil
}

// agentLock keeps two agents off the same folder. The logon task starts one,
// and a manual run must not double-upload while it is alive.
type agentLock struct {
	path string
}

func acquireLock(path string) (*agentLock, error) {
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return nil, err
	}
	file, err := os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0o600)
	if err != nil {
		if !errors.Is(err, os.ErrExist) {
			return nil, fmt.Errorf("backup lock: %w", err)
		}
		pid := readLockPID(path)
		if pid > 0 && processlock.ProcessRunning(pid) {
			return nil, fmt.Errorf("another tdrive-backup is running (pid %d)", pid)
		}
		_ = os.Remove(path)
		file, err = os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0o600)
		if err != nil {
			return nil, fmt.Errorf("backup lock: %w", err)
		}
	}
	if _, err := fmt.Fprintf(file, "%d\n", os.Getpid()); err != nil {
		_ = file.Close()
		_ = os.Remove(path)
		return nil, err
	}
	if err := file.Close(); err != nil {
		return nil, err
	}
	return &agentLock{path: path}, nil
}

func (l *agentLock) release() {
	if l != nil {
		_ = os.Remove(l.path)
	}
}

func readLockPID(path string) int {
	body, err := os.ReadFile(path)
	if err != nil {
		return 0
	}
	pid, err := strconv.Atoi(string(trimSpace(body)))
	if err != nil {
		return 0
	}
	return pid
}

func trimSpace(body []byte) []byte {
	start, end := 0, len(body)
	for start < end && (body[start] == ' ' || body[start] == '\n' || body[start] == '\r' || body[start] == '\t') {
		start++
	}
	for end > start && (body[end-1] == ' ' || body[end-1] == '\n' || body[end-1] == '\r' || body[end-1] == '\t') {
		end--
	}
	return body[start:end]
}
