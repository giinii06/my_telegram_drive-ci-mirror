package daemon

import (
	"context"
	"encoding/base64"
	"fmt"

	"TDrive/backend/core"
)

// maxThumbnailPayloadBytes bounds one poster a browser may hand back. The
// service validates the image again before anything reaches Telegram.
const maxThumbnailPayloadBytes = 2 << 20

// fileThumbnail returns the grid thumbnail for the file at remotePath. Only
// Telegram-side derivatives are served: an encrypted video answers with its
// encrypted rendition, and a missing one answers with an error so the browser
// can decode a frame itself and store it.
func (s *Server) fileThumbnail(ctx context.Context, remotePath string, driveID int64) (FileThumbnailResponse, error) {
	scope, resolved, err := s.resolveFileEntry(driveID, remotePath)
	if err != nil {
		return FileThumbnailResponse{}, err
	}
	payload, err := s.engine.FileService().Thumbnail(ctx, scope.drive.ID, int(resolved.MsgID))
	if err != nil {
		return FileThumbnailResponse{}, err
	}
	return FileThumbnailResponse{
		Drive:       scope.drive,
		ContentType: payload.MimeType,
		DataBase64:  payload.DataBase64,
	}, nil
}

// storeFileThumbnail publishes a browser-decoded poster as the file's
// thumbnail rendition. The file service encrypts it with the same key as the
// body when the source is encrypted, so no plaintext frame leaves the device.
func (s *Server) storeFileThumbnail(ctx context.Context, remotePath string, driveID int64, dataBase64 string) (FileThumbnailStoreResponse, error) {
	scope, resolved, err := s.resolveFileEntry(driveID, remotePath)
	if err != nil {
		return FileThumbnailStoreResponse{}, err
	}
	raw, err := base64.StdEncoding.DecodeString(dataBase64)
	if err != nil || len(raw) == 0 || len(raw) > maxThumbnailPayloadBytes {
		return FileThumbnailStoreResponse{}, fmt.Errorf("%w: invalid thumbnail payload", ErrInvalidRequest)
	}
	if err := s.engine.FileService().StoreThumbnail(ctx, scope.drive.ID, resolved.MsgID, raw); err != nil {
		return FileThumbnailStoreResponse{}, err
	}
	return FileThumbnailStoreResponse{Drive: scope.drive}, nil
}

func (s *Server) resolveFileEntry(driveID int64, remotePath string) (driveScope, core.ResolvedEntry, error) {
	scope, err := s.scopeForRequest(driveID, remotePath)
	if err != nil {
		return driveScope{}, core.ResolvedEntry{}, err
	}
	resolved, err := s.engine.ResolveEntryPath(scope.drive.ID, scope.cwd, remotePath)
	if err != nil {
		return driveScope{}, core.ResolvedEntry{}, err
	}
	if resolved.Type != "file" {
		return driveScope{}, core.ResolvedEntry{}, fmt.Errorf("%w: %s is not a file", ErrInvalidRequest, resolved.Path)
	}
	return scope, resolved, nil
}
