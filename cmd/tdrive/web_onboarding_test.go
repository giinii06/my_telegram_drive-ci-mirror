package main

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"TDrive/backend/daemon"
)

func (f *fakeWebClient) AuthStatus() (daemon.AuthStatusResponse, error) {
	return f.authStatusOut, f.authStatusErr
}

func (f *fakeWebClient) AuthSetup(apiID int, apiHash string) (daemon.AuthStatusResponse, error) {
	f.authSetupID = apiID
	f.authSetupHash = apiHash
	return f.authStatusOut, f.authSetupErr
}

func (f *fakeWebClient) Login(phone string, _ daemon.EventHandler) (daemon.AuthLoginResponse, error) {
	if f.loginStarted != nil {
		f.loginStarted <- phone
	}
	return f.loginOut, f.loginErr
}

func (f *fakeWebClient) LoginContext(ctx context.Context, phone string, onEvent daemon.EventHandler) (daemon.AuthLoginResponse, error) {
	if f.loginCtxStarted != nil {
		f.loginCtxStarted <- struct{}{}
	}
	if f.loginCtxBlock {
		<-ctx.Done()
		return daemon.AuthLoginResponse{}, ctx.Err()
	}
	return f.Login(phone, onEvent)
}

func (f *fakeWebClient) SubmitLoginCode(code string) error {
	f.submitCode = code
	return f.submitCodeErr
}

func (f *fakeWebClient) SubmitLoginPassword(password string) error {
	f.submitPass = password
	return f.submitPassErr
}

func (f *fakeWebClient) PreparePersonalDrive() (daemon.PersonalDriveSetup, error) {
	return f.prepareOut, f.prepareErr
}

func (f *fakeWebClient) SelectPersonalDrive(channelID string) (daemon.PersonalDriveSetup, error) {
	f.selectedID = channelID
	return f.selectOut, f.selectErr
}

func (f *fakeWebClient) CreatePersonalDrive() (daemon.PersonalDriveSetup, error) {
	f.createCalled = true
	return f.createOut, f.createErr
}

func onboardingJSONRequest(t *testing.T, handler http.Handler, target string, body string) *httptest.ResponseRecorder {
	t.Helper()
	request := httptest.NewRequest(http.MethodPost, webauthed(target), strings.NewReader(body))
	request.Header.Set("Content-Type", "application/json")
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	return recorder
}

func TestWebOnboardingStatusStages(t *testing.T) {
	cases := []struct {
		name   string
		auth   daemon.AuthStatusResponse
		status daemon.Status
		want   string
	}{
		{
			name: "needs setup",
			auth: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "NEEDS_SETUP"}},
			want: "api",
		},
		{
			name: "ready for login",
			auth: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN"}},
			want: "login",
		},
		{
			name: "logged in without drive",
			auth: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN", LoggedIn: true}},
			want: "drive",
		},
		{
			name:   "logged in with drive",
			auth:   daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN", LoggedIn: true}},
			status: daemon.Status{ActiveChannelID: 7},
			want:   "ready",
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			client := &fakeWebClient{authStatusOut: tc.auth, statusOut: tc.status}
			handler := newWebTestServer(client, nil).routes()
			recorder := httptest.NewRecorder()
			handler.ServeHTTP(recorder, httptest.NewRequest(http.MethodGet, webauthed("/api/onboarding/status"), nil))
			if recorder.Code != http.StatusOK {
				t.Fatalf("status = %d, body %s", recorder.Code, recorder.Body.String())
			}
			var body struct {
				Onboarding struct {
					Stage string `json:"stage"`
				} `json:"onboarding"`
			}
			if err := json.Unmarshal(recorder.Body.Bytes(), &body); err != nil {
				t.Fatalf("decode status: %v", err)
			}
			if body.Onboarding.Stage != tc.want {
				t.Fatalf("stage = %q, want %q", body.Onboarding.Stage, tc.want)
			}
		})
	}
}

func TestWebOnboardingSetupSavesCredentials(t *testing.T) {
	client := &fakeWebClient{authStatusOut: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "NEEDS_SETUP"}}}
	handler := newWebTestServer(client, nil).routes()
	recorder := onboardingJSONRequest(t, handler, "/api/onboarding/setup", `{"api_id":12345,"api_hash":"secret-hash"}`)
	if recorder.Code != http.StatusOK {
		t.Fatalf("setup = %d, body %s", recorder.Code, recorder.Body.String())
	}
	if client.authSetupID != 12345 || client.authSetupHash != "secret-hash" {
		t.Fatalf("saved credentials = %d %q", client.authSetupID, client.authSetupHash)
	}
}

func TestWebOnboardingSetupRejectsLoggedInUser(t *testing.T) {
	client := &fakeWebClient{authStatusOut: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN", LoggedIn: true}}}
	handler := newWebTestServer(client, nil).routes()
	recorder := onboardingJSONRequest(t, handler, "/api/onboarding/setup", `{"api_id":12345,"api_hash":"secret-hash"}`)
	if recorder.Code != http.StatusConflict {
		t.Fatalf("setup while logged in = %d, want 409", recorder.Code)
	}
	if client.authSetupID != 0 {
		t.Fatal("setup must not save credentials while logged in")
	}
}

func TestWebOnboardingRequiresJSONContentType(t *testing.T) {
	handler := newWebTestServer(&fakeWebClient{}, nil).routes()
	request := httptest.NewRequest(http.MethodPost, webauthed("/api/onboarding/setup"), strings.NewReader(`{}`))
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusBadRequest {
		t.Fatalf("setup without content type = %d, want 400", recorder.Code)
	}
}

func TestWebOnboardingLoginStartsFlow(t *testing.T) {
	started := make(chan string, 1)
	client := &fakeWebClient{
		authStatusOut: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN"}},
		loginStarted:  started,
		loginOut:      daemon.AuthLoginResponse{LoggedIn: true},
	}
	handler := newWebTestServer(client, nil).routes()
	recorder := onboardingJSONRequest(t, handler, "/api/onboarding/login", `{"phone":"+821012345678"}`)
	if recorder.Code != http.StatusAccepted {
		t.Fatalf("login = %d, body %s", recorder.Code, recorder.Body.String())
	}
	select {
	case phone := <-started:
		if phone != "+821012345678" {
			t.Fatalf("login phone = %q", phone)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("login flow did not start")
	}
}

func TestWebOnboardingLoginNeedsSetup(t *testing.T) {
	client := &fakeWebClient{authStatusOut: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "NEEDS_SETUP"}}}
	handler := newWebTestServer(client, nil).routes()
	recorder := onboardingJSONRequest(t, handler, "/api/onboarding/login", `{"phone":"+821012345678"}`)
	if recorder.Code != http.StatusBadRequest {
		t.Fatalf("login without setup = %d, want 400", recorder.Code)
	}
}

func TestWebOnboardingCodeAndPasswordRequireFlow(t *testing.T) {
	handler := newWebTestServer(&fakeWebClient{}, nil).routes()
	for _, target := range []string{"/api/onboarding/code", "/api/onboarding/password"} {
		recorder := onboardingJSONRequest(t, handler, target, `{"code":"12345","password":"secret"}`)
		if recorder.Code != http.StatusConflict {
			t.Fatalf("%s without flow = %d, want 409", target, recorder.Code)
		}
	}
}

func TestWebOnboardingDriveSelection(t *testing.T) {
	client := &fakeWebClient{
		authStatusOut: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN", LoggedIn: true}},
		prepareOut: daemon.PersonalDriveSetup{
			Status:     "selection_required",
			Candidates: []daemon.PersonalDriveCandidate{{ID: "123", Title: "My Drive", Recommended: true}},
		},
		selectOut: daemon.PersonalDriveSetup{Status: "ready", ActiveChannelID: "123"},
		mountOut:  daemon.MountResponse{Mounted: true},
	}
	handler := newWebTestServer(client, nil).routes()

	listRecorder := httptest.NewRecorder()
	handler.ServeHTTP(listRecorder, httptest.NewRequest(http.MethodGet, webauthed("/api/onboarding/drives"), nil))
	if listRecorder.Code != http.StatusOK {
		t.Fatalf("drives = %d, body %s", listRecorder.Code, listRecorder.Body.String())
	}
	var listBody struct {
		Candidates []daemon.PersonalDriveCandidate `json:"candidates"`
	}
	if err := json.Unmarshal(listRecorder.Body.Bytes(), &listBody); err != nil {
		t.Fatalf("decode drives: %v", err)
	}
	if len(listBody.Candidates) != 1 || listBody.Candidates[0].ID != "123" {
		t.Fatalf("candidates = %+v", listBody.Candidates)
	}

	recorder := onboardingJSONRequest(t, handler, "/api/onboarding/drive", `{"channel_id":"123"}`)
	if recorder.Code != http.StatusOK {
		t.Fatalf("drive selection = %d, body %s", recorder.Code, recorder.Body.String())
	}
	if client.selectedID != "123" {
		t.Fatalf("selected channel = %q", client.selectedID)
	}
}

func TestWebOnboardingCreateDrive(t *testing.T) {
	client := &fakeWebClient{
		authStatusOut: daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN", LoggedIn: true}},
		createOut:     daemon.PersonalDriveSetup{Status: "ready", ActiveChannelID: "456"},
		mountOut:      daemon.MountResponse{Mounted: true},
	}
	handler := newWebTestServer(client, nil).routes()
	recorder := onboardingJSONRequest(t, handler, "/api/onboarding/drive", `{"create":true}`)
	if recorder.Code != http.StatusOK {
		t.Fatalf("create drive = %d, body %s", recorder.Code, recorder.Body.String())
	}
	if !client.createCalled {
		t.Fatal("create drive was not requested")
	}
}

func onboardingStageOf(t *testing.T, handler http.Handler) (string, string) {
	t.Helper()
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, httptest.NewRequest(http.MethodGet, webauthed("/api/onboarding/status"), nil))
	var body struct {
		Onboarding struct {
			Stage string `json:"stage"`
			Error string `json:"error"`
		} `json:"onboarding"`
	}
	if err := json.Unmarshal(recorder.Body.Bytes(), &body); err != nil {
		t.Fatalf("decode status: %v", err)
	}
	return body.Onboarding.Stage, body.Onboarding.Error
}

func TestWebOnboardingCancelAllowsRetry(t *testing.T) {
	client := &fakeWebClient{
		authStatusOut:   daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN"}},
		loginCtxBlock:   true,
		loginCtxStarted: make(chan struct{}, 2),
	}
	handler := newWebTestServer(client, nil).routes()

	if rec := onboardingJSONRequest(t, handler, "/api/onboarding/login", `{"phone":"+8210"}`); rec.Code != http.StatusAccepted {
		t.Fatalf("first login = %d, body %s", rec.Code, rec.Body.String())
	}
	select {
	case <-client.loginCtxStarted:
	case <-time.After(2 * time.Second):
		t.Fatal("first login did not start")
	}

	if rec := onboardingJSONRequest(t, handler, "/api/onboarding/cancel", `{}`); rec.Code != http.StatusOK {
		t.Fatalf("cancel = %d, body %s", rec.Code, rec.Body.String())
	}
	deadline := time.Now().Add(2 * time.Second)
	for {
		stage, _ := onboardingStageOf(t, handler)
		if stage == "login" {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("stage after cancel = %q, want login", stage)
		}
		time.Sleep(10 * time.Millisecond)
	}

	if rec := onboardingJSONRequest(t, handler, "/api/onboarding/login", `{"phone":"+8211"}`); rec.Code != http.StatusAccepted {
		t.Fatalf("retry login = %d, body %s", rec.Code, rec.Body.String())
	}
	select {
	case <-client.loginCtxStarted:
	case <-time.After(2 * time.Second):
		t.Fatal("retry login did not start")
	}
	onboardingJSONRequest(t, handler, "/api/onboarding/cancel", `{}`)
}

func TestWebOnboardingLoginTimeoutAllowsRetry(t *testing.T) {
	client := &fakeWebClient{
		authStatusOut:   daemon.AuthStatusResponse{Status: daemon.AuthStatus{SystemStatus: "READY_FOR_LOGIN"}},
		loginCtxBlock:   true,
		loginCtxStarted: make(chan struct{}, 2),
	}
	server := newWebTestServer(client, nil)
	server.loginTimeout = 20 * time.Millisecond
	handler := server.routes()

	if rec := onboardingJSONRequest(t, handler, "/api/onboarding/login", `{"phone":"+8210"}`); rec.Code != http.StatusAccepted {
		t.Fatalf("first login = %d, body %s", rec.Code, rec.Body.String())
	}
	select {
	case <-client.loginCtxStarted:
	case <-time.After(2 * time.Second):
		t.Fatal("first login did not start")
	}

	time.Sleep(50 * time.Millisecond)
	stage, errText := onboardingStageOf(t, handler)
	if stage != "login" || !strings.Contains(errText, "시간이 초과") {
		t.Fatalf("after timeout stage = %q, error = %q", stage, errText)
	}

	if rec := onboardingJSONRequest(t, handler, "/api/onboarding/login", `{"phone":"+8211"}`); rec.Code != http.StatusAccepted {
		t.Fatalf("retry login = %d, body %s", rec.Code, rec.Body.String())
	}
	select {
	case <-client.loginCtxStarted:
	case <-time.After(2 * time.Second):
		t.Fatal("retry login did not start")
	}
	onboardingJSONRequest(t, handler, "/api/onboarding/cancel", `{}`)
}
