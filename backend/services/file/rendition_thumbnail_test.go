package file

import (
	"bytes"
	"context"
	"encoding/base64"
	"errors"
	"image"
	"image/color"
	"image/jpeg"
	"testing"

	"TDrive/backend/projection"
)

func testPosterJPEG(t *testing.T) []byte {
	t.Helper()
	img := image.NewRGBA(image.Rect(0, 0, 160, 90))
	for y := 0; y < 90; y++ {
		for x := 0; x < 160; x++ {
			img.Set(x, y, color.RGBA{R: uint8(x), G: uint8(y), B: 90, A: 255})
		}
	}
	var encoded bytes.Buffer
	if err := jpeg.Encode(&encoded, img, &jpeg.Options{Quality: 85}); err != nil {
		t.Fatal(err)
	}
	return encoded.Bytes()
}

func seedVideoSource(t *testing.T, s *Service, raw []byte, encrypted bool) projection.File {
	t.Helper()
	source := preparationOriginal(t, s, raw, encrypted)
	if _, err := s.DB.Exec(`UPDATE files SET name='clip.mp4' WHERE channel_id=? AND msg_id=?`, personalChannelID, source.MsgID); err != nil {
		t.Fatal(err)
	}
	updated, found, err := projection.FileByID(s.DB, personalChannelID, source.MsgID)
	if err != nil || !found {
		t.Fatalf("video source %v", err)
	}
	return updated
}

// A browser-decoded poster becomes a normal rendition: encrypted with the
// source key, published to Telegram, and served back through Thumbnail.
func TestStoreThumbnailPublishesEncryptedVideoRendition(t *testing.T) {
	s, _, _, _ := newTestService(t)
	key := bytes.Repeat([]byte{3}, 32)
	s.RequireEncryptionKey = func(bool) ([]byte, error) { return append([]byte(nil), key...), nil }

	source := seedVideoSource(t, s, testPosterJPEG(t), true)
	poster := testPosterJPEG(t)
	if err := s.StoreThumbnail(context.Background(), personalChannelID, source.MsgID, poster); err != nil {
		t.Fatalf("StoreThumbnail: %v", err)
	}
	ref, err := projection.CurrentFileRendition(context.Background(), s.DB, personalChannelID, source.MsgID, projection.RenditionThumbnail)
	if err != nil {
		t.Fatalf("rendition ref: %v", err)
	}
	if !ref.Encrypted {
		t.Fatal("encrypted video stored a plaintext thumbnail rendition")
	}
	payload, err := s.Thumbnail(context.Background(), personalChannelID, int(source.MsgID))
	if err != nil {
		t.Fatalf("Thumbnail: %v", err)
	}
	got, err := base64.StdEncoding.DecodeString(payload.DataBase64)
	if err != nil || !bytes.Equal(got, poster) {
		t.Fatalf("served poster mismatch: %v", err)
	}

	// A second poster never replaces the published one.
	if err := s.StoreThumbnail(context.Background(), personalChannelID, source.MsgID, testPosterJPEG(t)); err != nil {
		t.Fatalf("second StoreThumbnail: %v", err)
	}
	again, err := projection.CurrentFileRendition(context.Background(), s.DB, personalChannelID, source.MsgID, projection.RenditionThumbnail)
	if err != nil || again.MsgID != ref.MsgID {
		t.Fatalf("published thumbnail replaced: %+v %v", again, err)
	}
}

func TestStoreThumbnailRejectsNonVideoAndInvalidImage(t *testing.T) {
	s, _, _, _ := newTestService(t)
	s.RequireEncryptionKey = func(bool) ([]byte, error) { return bytes.Repeat([]byte{1}, 32), nil }

	photo := preparationOriginal(t, s, testPosterJPEG(t), false)
	if err := s.StoreThumbnail(context.Background(), personalChannelID, photo.MsgID, testPosterJPEG(t)); !errors.Is(err, errPreviewNotSupported) {
		t.Fatalf("photo source error = %v, want unsupported", err)
	}

	video := seedVideoSource(t, s, testPosterJPEG(t), false)
	if err := s.StoreThumbnail(context.Background(), personalChannelID, video.MsgID, []byte("not a jpeg")); !errors.Is(err, errPreviewNotSupported) {
		t.Fatalf("invalid poster error = %v, want unsupported", err)
	}
}
