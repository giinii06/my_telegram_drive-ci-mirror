package main

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strings"
	"time"

	"TDrive/backend/daemon"
)

// webAuthFlow tracks the in-flight Telegram login so the browser can poll for
// the code or 2FA prompt while the daemon's streaming Login call is blocked.
type webAuthFlow struct {
	running   bool
	stage     string
	err       string
	startedAt time.Time
	cancel    context.CancelFunc
	timer     *time.Timer
}

const defaultWebLoginTimeout = 10 * time.Minute

func (s *webServer) webLoginTimeout() time.Duration {
	if s.loginTimeout > 0 {
		return s.loginTimeout
	}
	return defaultWebLoginTimeout
}

func (s *webServer) expireWebLogin() {
	s.authMu.Lock()
	flow := s.authFlow
	expired := flow != nil && flow.running && time.Since(flow.startedAt) > s.webLoginTimeout()
	s.authMu.Unlock()
	if expired {
		s.cancelWebLoginFlow(flow, "로그인 시간이 초과되었습니다. 다시 시도하세요.")
	}
}

func (s *webServer) cancelWebLogin(message string) bool {
	s.authMu.Lock()
	flow := s.authFlow
	s.authMu.Unlock()
	if flow == nil {
		return false
	}
	return s.cancelWebLoginFlow(flow, message)
}

func (s *webServer) cancelWebLoginFlow(target *webAuthFlow, message string) bool {
	s.authMu.Lock()
	flow := s.authFlow
	if flow == nil || flow != target || !flow.running {
		s.authMu.Unlock()
		return false
	}
	cancel := flow.cancel
	flow.cancel = nil
	if flow.timer != nil {
		flow.timer.Stop()
		flow.timer = nil
	}
	flow.running = false
	flow.stage = "login"
	flow.err = message
	s.authMu.Unlock()
	if cancel != nil {
		cancel()
	}
	return true
}

func (s *webServer) onboardingState() (map[string]any, error) {
	s.expireWebLogin()
	auth, err := s.client.AuthStatus()
	if err != nil {
		return nil, err
	}
	status, err := s.client.Status()
	if err != nil {
		return nil, err
	}

	configured := auth.Status.SystemStatus != "NEEDS_SETUP"
	stage := "api"
	if configured {
		stage = "login"
	}
	if auth.Status.LoggedIn {
		stage = "drive"
		if status.ActiveChannelID != 0 {
			stage = "ready"
		}
	}

	s.authMu.Lock()
	flow := s.authFlow
	if flow != nil {
		if flow.running && flow.stage != "" {
			stage = flow.stage
		}
	}
	errText := ""
	if flow != nil && !auth.Status.LoggedIn {
		errText = flow.err
	}
	s.authMu.Unlock()

	return map[string]any{
		"configured":      configured,
		"logged_in":       auth.Status.LoggedIn,
		"stage":           stage,
		"error":           errText,
		"active_drive_id": status.ActiveChannelID,
	}, nil
}

func (s *webServer) handleOnboardingStatus(response http.ResponseWriter, request *http.Request) {
	if request.Method != http.MethodGet {
		writeWebError(response, http.StatusMethodNotAllowed, "GET only")
		return
	}
	state, err := s.onboardingState()
	if err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	writeWebJSON(response, http.StatusOK, map[string]any{"ok": true, "onboarding": state})
}

func decodeOnboardingJSON(request *http.Request, value any) error {
	if !strings.HasPrefix(strings.ToLower(request.Header.Get("Content-Type")), "application/json") {
		return fmt.Errorf("Content-Type must be application/json")
	}
	defer request.Body.Close()
	if err := json.NewDecoder(http.MaxBytesReader(nil, request.Body, 1<<20)).Decode(value); err != nil {
		return fmt.Errorf("invalid JSON body")
	}
	return nil
}

func (s *webServer) handleOnboardingSetup(response http.ResponseWriter, request *http.Request) {
	if request.Method != http.MethodPost {
		writeWebError(response, http.StatusMethodNotAllowed, "POST only")
		return
	}
	var body struct {
		APIID   int    `json:"api_id"`
		APIHash string `json:"api_hash"`
	}
	if err := decodeOnboardingJSON(request, &body); err != nil {
		writeWebError(response, http.StatusBadRequest, err.Error())
		return
	}
	apiHash := strings.TrimSpace(body.APIHash)
	if body.APIID <= 0 || apiHash == "" || len(apiHash) > 200 {
		writeWebError(response, http.StatusBadRequest, "API ID와 API hash를 확인하세요.")
		return
	}
	auth, err := s.client.AuthStatus()
	if err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	if auth.Status.LoggedIn {
		writeWebError(response, http.StatusConflict, "이미 Telegram에 로그인되어 있습니다.")
		return
	}
	if _, err := s.client.AuthSetup(body.APIID, apiHash); err != nil {
		writeWebError(response, http.StatusBadGateway, "API 자격 증명을 저장하지 못했습니다.")
		return
	}
	writeWebJSON(response, http.StatusOK, map[string]any{"ok": true})
}

func (s *webServer) handleOnboardingLogin(response http.ResponseWriter, request *http.Request) {
	if request.Method != http.MethodPost {
		writeWebError(response, http.StatusMethodNotAllowed, "POST only")
		return
	}
	var body struct {
		Phone string `json:"phone"`
	}
	if err := decodeOnboardingJSON(request, &body); err != nil {
		writeWebError(response, http.StatusBadRequest, err.Error())
		return
	}
	phone := strings.TrimSpace(body.Phone)
	if phone == "" || len(phone) > 64 {
		writeWebError(response, http.StatusBadRequest, "전화번호를 확인하세요.")
		return
	}
	auth, err := s.client.AuthStatus()
	if err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	if auth.Status.SystemStatus == "NEEDS_SETUP" {
		writeWebError(response, http.StatusBadRequest, "먼저 API ID와 hash를 설정하세요.")
		return
	}
	if auth.Status.LoggedIn {
		writeWebJSON(response, http.StatusOK, map[string]any{"ok": true})
		return
	}

	s.expireWebLogin()
	s.authMu.Lock()
	if s.authFlow != nil && s.authFlow.running {
		s.authMu.Unlock()
		writeWebError(response, http.StatusConflict, "이미 로그인이 진행 중입니다.")
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	flow := &webAuthFlow{
		running:   true,
		stage:     "waiting",
		startedAt: time.Now(),
		cancel:    cancel,
	}
	flow.timer = time.AfterFunc(s.webLoginTimeout(), func() {
		s.cancelWebLoginFlow(flow, "로그인 시간이 초과되었습니다. 다시 시도하세요.")
	})
	s.authFlow = flow
	s.authMu.Unlock()

	go s.runWebLogin(ctx, flow, phone)
	writeWebJSON(response, http.StatusAccepted, map[string]any{"ok": true})
}

func (s *webServer) runWebLogin(ctx context.Context, flow *webAuthFlow, phone string) {
	_, err := s.client.LoginContext(ctx, phone, func(event daemon.Event) {
		s.authMu.Lock()
		defer s.authMu.Unlock()
		if !flow.running {
			return
		}
		switch event.Name {
		case "login-code-required":
			flow.stage = "code"
			flow.err = ""
		case "login-code-invalid":
			flow.stage = "code"
			flow.err = "로그인 코드가 올바르지 않습니다."
		case "login-password-required":
			flow.stage = "password"
			flow.err = ""
		}
	})

	s.authMu.Lock()
	defer s.authMu.Unlock()
	if !flow.running {
		return
	}
	flow.running = false
	if flow.timer != nil {
		flow.timer.Stop()
		flow.timer = nil
	}
	if flow.cancel != nil {
		flow.cancel()
		flow.cancel = nil
	}
	if err != nil {
		flow.err = err.Error()
		if flow.stage != "code" && flow.stage != "password" {
			flow.stage = "login"
		}
		return
	}
	flow.err = ""
}

func (s *webServer) handleOnboardingCode(response http.ResponseWriter, request *http.Request) {
	if request.Method != http.MethodPost {
		writeWebError(response, http.StatusMethodNotAllowed, "POST only")
		return
	}
	var body struct {
		Code string `json:"code"`
	}
	if err := decodeOnboardingJSON(request, &body); err != nil {
		writeWebError(response, http.StatusBadRequest, err.Error())
		return
	}
	code := strings.TrimSpace(body.Code)
	if code == "" || len(code) > 64 {
		writeWebError(response, http.StatusBadRequest, "로그인 코드를 확인하세요.")
		return
	}

	s.authMu.Lock()
	flow := s.authFlow
	if flow == nil || !flow.running || (flow.stage != "code" && flow.stage != "waiting") {
		s.authMu.Unlock()
		writeWebError(response, http.StatusConflict, "지금은 코드를 입력할 단계가 아닙니다.")
		return
	}
	s.authMu.Unlock()

	if err := s.client.SubmitLoginCode(code); err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	s.authMu.Lock()
	flow.stage = "waiting"
	flow.err = ""
	s.authMu.Unlock()
	writeWebJSON(response, http.StatusOK, map[string]any{"ok": true})
}

func (s *webServer) handleOnboardingPassword(response http.ResponseWriter, request *http.Request) {
	if request.Method != http.MethodPost {
		writeWebError(response, http.StatusMethodNotAllowed, "POST only")
		return
	}
	var body struct {
		Password string `json:"password"`
	}
	if err := decodeOnboardingJSON(request, &body); err != nil {
		writeWebError(response, http.StatusBadRequest, err.Error())
		return
	}
	password := body.Password
	if password == "" || len(password) > 256 {
		writeWebError(response, http.StatusBadRequest, "2FA 비밀번호를 확인하세요.")
		return
	}

	s.authMu.Lock()
	flow := s.authFlow
	if flow == nil || !flow.running || flow.stage != "password" {
		s.authMu.Unlock()
		writeWebError(response, http.StatusConflict, "지금은 2FA 비밀번호를 입력할 단계가 아닙니다.")
		return
	}
	s.authMu.Unlock()

	if err := s.client.SubmitLoginPassword(password); err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	s.authMu.Lock()
	flow.stage = "waiting"
	flow.err = ""
	s.authMu.Unlock()
	writeWebJSON(response, http.StatusOK, map[string]any{"ok": true})
}

func (s *webServer) handleOnboardingCancel(response http.ResponseWriter, request *http.Request) {
	if request.Method != http.MethodPost {
		writeWebError(response, http.StatusMethodNotAllowed, "POST only")
		return
	}
	var body struct{}
	if err := decodeOnboardingJSON(request, &body); err != nil {
		writeWebError(response, http.StatusBadRequest, err.Error())
		return
	}
	canceled := s.cancelWebLogin("로그인이 취소되었습니다. 다시 시도하세요.")
	writeWebJSON(response, http.StatusOK, map[string]any{"ok": true, "canceled": canceled})
}

func (s *webServer) handleOnboardingDrives(response http.ResponseWriter, request *http.Request) {
	if request.Method != http.MethodGet {
		writeWebError(response, http.StatusMethodNotAllowed, "GET only")
		return
	}
	auth, err := s.client.AuthStatus()
	if err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	if !auth.Status.LoggedIn {
		writeWebError(response, http.StatusConflict, "Telegram 로그인이 필요합니다.")
		return
	}
	setup, err := s.client.PreparePersonalDrive()
	if err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	candidates := setup.Candidates
	if candidates == nil {
		candidates = []daemon.PersonalDriveCandidate{}
	}
	writeWebJSON(response, http.StatusOK, map[string]any{
		"ok":                true,
		"status":            setup.Status,
		"active_channel_id": setup.ActiveChannelID,
		"candidates":        candidates,
	})
}

func (s *webServer) handleOnboardingDrive(response http.ResponseWriter, request *http.Request) {
	if request.Method != http.MethodPost {
		writeWebError(response, http.StatusMethodNotAllowed, "POST only")
		return
	}
	var body struct {
		ChannelID string `json:"channel_id"`
		Create    bool   `json:"create"`
	}
	if err := decodeOnboardingJSON(request, &body); err != nil {
		writeWebError(response, http.StatusBadRequest, err.Error())
		return
	}
	channelID := strings.TrimSpace(body.ChannelID)
	if body.Create && channelID != "" {
		writeWebError(response, http.StatusBadRequest, "channel_id와 create는 함께 쓸 수 없습니다.")
		return
	}
	if !body.Create && (channelID == "" || len(channelID) > 64) {
		writeWebError(response, http.StatusBadRequest, "드라이브를 선택하세요.")
		return
	}

	auth, err := s.client.AuthStatus()
	if err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	if !auth.Status.LoggedIn {
		writeWebError(response, http.StatusConflict, "Telegram 로그인이 필요합니다.")
		return
	}

	var setup daemon.PersonalDriveSetup
	if body.Create {
		setup, err = s.client.CreatePersonalDrive()
	} else {
		setup, err = s.client.SelectPersonalDrive(channelID)
	}
	if err != nil {
		writeWebError(response, http.StatusBadGateway, err.Error())
		return
	}
	if err := ensureWebMount(s.client); err != nil {
		fmt.Fprintf(os.Stderr, "web: mount after drive selection: %v\n", err)
	}
	writeWebJSON(response, http.StatusOK, map[string]any{
		"ok":                true,
		"status":            setup.Status,
		"active_channel_id": setup.ActiveChannelID,
	})
}
