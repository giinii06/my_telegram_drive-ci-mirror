package main

import (
	"encoding/base64"
	"encoding/json"
	"net/http"
	"strconv"
)

// webThumbnailBodyLimit bounds the JSON body a browser posts with a decoded
// poster. The service applies its own image and size validation afterwards.
const webThumbnailBodyLimit = 3 << 20

func (s *webServer) handleThumbnail(response http.ResponseWriter, request *http.Request) {
	switch request.Method {
	case http.MethodGet, http.MethodHead:
		s.serveThumbnail(response, request)
	case http.MethodPost:
		s.storeThumbnail(response, request)
	default:
		writeWebError(response, http.StatusMethodNotAllowed, "GET or POST only")
	}
}

// serveThumbnail returns the stored grid thumbnail for one video. A miss is a
// 404 rather than an error page: the browser answers it by decoding a frame
// and posting it back.
func (s *webServer) serveThumbnail(response http.ResponseWriter, request *http.Request) {
	// A miss is how the browser learns a poster still needs decoding, so it
	// must never be served from a cache.
	response.Header().Set("Cache-Control", "no-store")
	remotePath, err := cleanWebPath(request.URL.Query().Get("path"))
	if err != nil {
		writeWebError(response, http.StatusBadRequest, err.Error())
		return
	}
	out, err := s.client.FileThumbnailInDrive(s.driveID, remotePath)
	if err != nil {
		writeWebError(response, http.StatusNotFound, "thumbnail unavailable")
		return
	}
	data, err := base64.StdEncoding.DecodeString(out.DataBase64)
	if err != nil || len(data) == 0 {
		writeWebError(response, http.StatusBadGateway, "thumbnail decode failed")
		return
	}
	contentType := out.ContentType
	if contentType == "" {
		contentType = http.DetectContentType(data)
	}
	response.Header().Set("Content-Type", contentType)
	response.Header().Set("Content-Length", strconv.Itoa(len(data)))
	response.Header().Set("Cache-Control", "private, max-age=86400")
	response.Header().Set("X-Content-Type-Options", "nosniff")
	response.WriteHeader(http.StatusOK)
	if request.Method != http.MethodHead {
		_, _ = response.Write(data)
	}
}

type webThumbnailStoreRequest struct {
	Path       string `json:"path"`
	DataBase64 string `json:"data_base64"`
}

func (s *webServer) storeThumbnail(response http.ResponseWriter, request *http.Request) {
	request.Body = http.MaxBytesReader(response, request.Body, webThumbnailBodyLimit)
	var body webThumbnailStoreRequest
	if err := json.NewDecoder(request.Body).Decode(&body); err != nil {
		writeWebError(response, http.StatusBadRequest, "invalid JSON body")
		return
	}
	remotePath, err := cleanWebPath(body.Path)
	if err != nil {
		writeWebError(response, http.StatusBadRequest, err.Error())
		return
	}
	if body.DataBase64 == "" {
		writeWebError(response, http.StatusBadRequest, "thumbnail data required")
		return
	}
	out, err := s.client.StoreFileThumbnailInDrive(s.driveID, remotePath, body.DataBase64)
	if err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	writeWebJSON(response, http.StatusOK, map[string]any{"ok": true, "drive": out.Drive})
}
