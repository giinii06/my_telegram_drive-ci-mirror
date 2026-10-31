package main

import (
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"time"
)

// fileStamp identifies one version of a file. A backup pass only touches a file
// whose stamp has not changed between two scans, so a half-copied download is
// never moved and then deleted.
type fileStamp struct {
	size int64
	mod  int64
}

type candidate struct {
	path  string
	rel   string
	size  int64
	mod   time.Time
	stamp fileStamp
}

// scanFolder lists regular files under root, oldest first. Ordering is what
// makes the pipeline predictable: the agent handles one file at a time in the
// order they were last written.
func scanFolder(root string, skip map[string]bool) ([]candidate, error) {
	var files []candidate
	err := filepath.WalkDir(root, func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if entry.IsDir() {
			return nil
		}
		info, err := entry.Info()
		if err != nil {
			return nil
		}
		if !info.Mode().IsRegular() {
			return nil
		}
		absolute, err := filepath.Abs(path)
		if err != nil {
			absolute = path
		}
		if skip[absolute] {
			return nil
		}
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return nil
		}
		files = append(files, candidate{
			path:  absolute,
			rel:   filepath.ToSlash(rel),
			size:  info.Size(),
			mod:   info.ModTime(),
			stamp: fileStamp{size: info.Size(), mod: info.ModTime().UnixNano()},
		})
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("scan %s: %w", root, err)
	}
	sort.Slice(files, func(i, j int) bool {
		if !files[i].mod.Equal(files[j].mod) {
			return files[i].mod.Before(files[j].mod)
		}
		return files[i].rel < files[j].rel
	})
	return files, nil
}

// readyForUpload reports whether the file is old enough and unchanged since the
// previous scan. The first sighting of a file never qualifies.
func readyForUpload(candidate candidate, seen map[string]fileStamp, settle time.Duration, now time.Time) bool {
	previous, ok := seen[candidate.rel]
	if !ok || previous != candidate.stamp {
		return false
	}
	return now.Sub(candidate.mod) >= settle
}

func rememberStamps(seen map[string]fileStamp, files []candidate) {
	present := make(map[string]bool, len(files))
	for _, file := range files {
		seen[file.rel] = file.stamp
		present[file.rel] = true
	}
	for rel := range seen {
		if !present[rel] {
			delete(seen, rel)
		}
	}
}

func folderExists(path string) error {
	info, err := os.Stat(path)
	if err != nil {
		return err
	}
	if !info.IsDir() {
		return fmt.Errorf("%s is not a folder", path)
	}
	return nil
}
