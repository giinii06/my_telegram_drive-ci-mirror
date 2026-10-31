; Inno Setup 6 script for the TDrive server (Windows 11).
; Build it through build-exe.ps1, which passes the version, commit, numeric
; version and the directory holding dist\tdrive.exe.
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef MyAppNumericVersion
  #define MyAppNumericVersion "0.0.0.0"
#endif
#ifndef MyAppCommit
  #define MyAppCommit "unknown"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\dist"
#endif

#define MyAppName "TDrive"

[Setup]
; A fixed AppId keeps upgrades and uninstalls attached to the same entry.
AppId={{B2F5B4C1-9B3E-4E77-9C1B-3D2A6C0F7A11}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppName}
VersionInfoVersion={#MyAppNumericVersion}
VersionInfoDescription=Installs the TDrive server for the current user
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppNumericVersion}
DefaultDirName={localappdata}\TDrive
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir={#SourceDir}
OutputBaseFilename=TDrive-Setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName={#MyAppName}
AllowNoIcons=yes

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Windows 시작 시 TDrive를 백그라운드로 자동 실행"; GroupDescription: "시작 옵션"; Flags: checkedonce
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\tdrive.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-web.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-hidden.vbs"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-service.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-open.cmd"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\tdrive-open.cmd"; WorkingDir: "{app}"; Comment: "TDrive 웹 UI를 엽니다"
Name: "{autoprograms}\{#MyAppName} 서버 시작"; Filename: "{app}\tdrive-service.cmd"; Parameters: "start"; WorkingDir: "{app}"
Name: "{autoprograms}\{#MyAppName} 서버 중지"; Filename: "{app}\tdrive-service.cmd"; Parameters: "stop"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\tdrive-open.cmd"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\tdrive-service.cmd"; Parameters: "install"; Tasks: autostart; Flags: runhidden waituntilterminated; StatusMsg: "TDrive를 백그라운드로 시작하는 중..."
Filename: "{app}\tdrive-open.cmd"; Description: "TDrive 열기"; Flags: postinstall nowait skipifsilent shellexec

[UninstallRun]
Filename: "{app}\tdrive-service.cmd"; Parameters: "remove"; Flags: runhidden waituntilterminated; RunOnceId: "TDriveService"

[Code]
const
  EnvironmentKey = 'Environment';

var
  PortPage: TInputQueryWizardPage;
  AccessPage: TInputOptionWizardPage;
  Port: String;
  FinishedNoteAdded: Boolean;

procedure InitializeWizard;
begin
  PortPage := CreateInputQueryPage(wpSelectDir,
    'TDrive 웹 포트', '웹 UI가 사용할 포트를 정합니다.',
    '기본값 8080을 그대로 두어도 됩니다. 나중에 이 폴더의 port.txt를 고치면 바뀝니다.');
  PortPage.Add('포트', False);
  PortPage.Values[0] := '8080';

  AccessPage := CreateInputOptionPage(PortPage.ID,
    '접속 범위', '어디에서 이 TDrive에 접속할지 정합니다.',
    '다른 PC에서 자동 백업을 보내려면 이 PC가 Tailscale 주소로 접속을 받아야 합니다.' + #13#10 +
    '두 번째 항목은 모든 인터페이스에 열리며, 접속할 때는 항상 토큰이 필요합니다.',
    True, False);
  AccessPage.Add('이 PC에서만 접속 (127.0.0.1)');
  AccessPage.Add('다른 기기에서도 접속 허용 (0.0.0.0, Tailscale·LAN)');
  AccessPage.Values[0] := False;
  AccessPage.Values[1] := True;

  WizardForm.WelcomeLabel2.Caption :=
    WizardForm.WelcomeLabel2.Caption + #13#10#13#10 +
    'TDrive 서버가 이 Windows 계정에 설치됩니다. 설치 후에는 Telegram 로그인만' + #13#10 +
    '브라우저에서 하면 됩니다. API ID·hash, 전화번호, 2FA는 지금 넣거나' + #13#10 +
    '나중에 웹 UI에서 다시 넣을 수 있습니다.' + #13#10#13#10 +
    '빌드: {#MyAppCommit}';
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var
  Value: Integer;
begin
  Result := True;
  if CurPageID <> PortPage.ID then
    exit;
  Value := StrToIntDef(PortPage.Values[0], 0);
  if (Value < 1) or (Value > 65535) then
  begin
    MsgBox('포트는 1에서 65535 사이의 숫자여야 합니다.', mbError, MB_OK);
    Result := False;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    Port := PortPage.Values[0];
    SaveStringToFile(ExpandConstant('{app}\port.txt'), Port, False);
    if AccessPage.Values[1] then
      SaveStringToFile(ExpandConstant('{app}\listen.txt'), '0.0.0.0', False)
    else
      SaveStringToFile(ExpandConstant('{app}\listen.txt'), '127.0.0.1', False);
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if (CurPageID = wpFinished) and not FinishedNoteAdded then
  begin
    FinishedNoteAdded := True;
    WizardForm.FinishedLabel.Caption := WizardForm.FinishedLabel.Caption + #13#10#13#10 +
      '다른 PC에서 자동 백업을 보낼 때:' + #13#10 +
      '  서버 주소: tailscale ip -4 로 확인한 이 PC의 Tailscale 주소 (포트 ' + PortPage.Values[0] + ')' + #13#10 +
      '  접속 토큰: %USERPROFILE%\.config\TDrive\web.token 파일의 값' + #13#10 +
      '두 값은 TDrive Backup 설치 프로그램에 넣거나, 설치 후 backup.json에서 바꿀 수 있습니다.';
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    DelTree(ExpandConstant('{app}'), True, False, True);
end;
