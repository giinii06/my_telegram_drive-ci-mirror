; Inno Setup 6 script for the Windows 11 automatic backup agent.
; Build it through build-backup-exe.ps1, which passes the version, commit and
; the path to dist\tdrive-backup.exe.
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef MyAppNumericVersion
  #define MyAppNumericVersion "0.0.0.0"
#endif
#ifndef MyAppCommit
  #define MyAppCommit "unknown"
#endif
#ifndef BackupExe
  #define BackupExe "..\..\dist\tdrive-backup.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif

#define MyAppName "TDrive Backup"

[Setup]
AppId={{7A6C1E4D-51B8-4C2A-8F3D-6A9E0B7C2D55}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppName}
VersionInfoVersion={#MyAppNumericVersion}
VersionInfoDescription=Archives a local folder into a TDrive host over Tailscale
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppNumericVersion}
DefaultDirName={localappdata}\TDriveBackup
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=yes
PrivilegesRequired=lowest
OutputDir={#OutputDir}
OutputBaseFilename=TDrive-Backup-Setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName={#MyAppName}
AllowNoIcons=yes

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Windows에 로그인할 때 백업을 백그라운드로 자동 실행"; GroupDescription: "시작 옵션"; Flags: checkedonce

[Files]
Source: "{#BackupExe}"; DestDir: "{app}"; DestName: "tdrive-backup.exe"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-backup.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-backup-hidden.vbs"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-backup-service.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-backup-status.cmd"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "TDriveBackup"; ValueData: """{sys}\wscript.exe"" ""{app}\tdrive-backup-hidden.vbs"""; Flags: uninsdeletevalue; Tasks: autostart

[Icons]
Name: "{autoprograms}\{#MyAppName} 상태"; Filename: "{app}\tdrive-backup-status.cmd"; WorkingDir: "{app}"
Name: "{autoprograms}\{#MyAppName} 시작"; Filename: "{sys}\wscript.exe"; Parameters: """{app}\tdrive-backup-hidden.vbs"""; WorkingDir: "{app}"
Name: "{autoprograms}\{#MyAppName} 중지"; Filename: "{app}\tdrive-backup-service.cmd"; Parameters: "stop"; WorkingDir: "{app}"
Name: "{autoprograms}\{#MyAppName} 설정"; Filename: "{sys}\notepad.exe"; Parameters: """{app}\backup.json"""; WorkingDir: "{app}"
Name: "{autoprograms}\{#MyAppName} 로그"; Filename: "{sys}\notepad.exe"; Parameters: """{app}\backup.log"""; WorkingDir: "{app}"

[Run]
Filename: "{sys}\wscript.exe"; Parameters: """{app}\tdrive-backup-hidden.vbs"""; Description: "지금 백업 시작"; Flags: postinstall nowait skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM tdrive-backup.exe"; Flags: runhidden waituntilterminated; RunOnceId: "TDriveBackupStop"

[Code]
var
  HostPage: TInputQueryWizardPage;
  FolderPage: TInputDirWizardPage;

function JsonEsc(const Value: String): String;
var
  Index: Integer;
  Current: Char;
begin
  Result := '';
  for Index := 1 to Length(Value) do
  begin
    Current := Value[Index];
    if (Current = '\') or (Current = '"') then
      Result := Result + '\' + Current
    else if Current = #9 then
      Result := Result + '\t'
    else
      Result := Result + Current;
  end;
end;

procedure InitializeWizard;
begin
  HostPage := CreateInputQueryPage(wpWelcome,
    'TDrive 서버 (Tailscale)',
    '파일을 보낼 TDrive 서버를 지정합니다.',
    'Tailscale에 연결된 서버의 이름이나 주소를 넣습니다. 포트를 생략하면 8080을 씁니다.' + #13#10 +
    '예: my-server, my-server.tailnet-xxxx.ts.net, 100.101.102.103, 100.101.102.103:8080');
  HostPage.Add('서버 주소', False);
  HostPage.Add('접속 토큰 (서버의 web.token 값, 비워 둘 수 있음)', False);

  FolderPage := CreateInputDirPage(HostPage.ID,
    '자동 백업 폴더',
    '이 폴더의 파일을 오래된 것부터 하나씩 서버로 옮깁니다.',
    '업로드와 Telegram 저장이 확인된 파일만 원본이 삭제됩니다. Telegram 사본은 지워지지 않습니다.' + #13#10 +
    '찾아보기로 기존 폴더를 고르거나, 없으면 새 폴더를 만듭니다.',
    False, '백업 폴더');
  FolderPage.Add('백업 폴더');
  FolderPage.Values[0] := ExpandConstant('{%USERPROFILE}\TDriveBackup');

  WizardForm.WelcomeLabel2.Caption :=
    WizardForm.WelcomeLabel2.Caption + #13#10#13#10 +
    '이 설치본은 이 PC의 폴더를 TDrive 서버로 자동 백업합니다.' + #13#10 +
    '서버 주소(보통 Tailscale 이름)와 백업 폴더만 정하면 되고, 두 값은 설치 후' + #13#10 +
    '설치 폴더의 backup.json에서 언제든 바꿀 수 있습니다.' + #13#10#13#10 +
    '파일 하나를 올리고 → 서버가 Telegram에 저장하고 → 서버의 임시 파일이 지워진 뒤' + #13#10 +
    '→ 원본을 지우고 다음 파일로 넘어갑니다. Telegram에 저장된 사본은 지우지 않습니다.' + #13#10#13#10 +
    '빌드: {#MyAppCommit}';
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  // A silent install has nobody to answer the wizard, so it keeps the
  // defaults and writes them to backup.json instead of blocking on a box.
  if WizardSilent then
    exit;
  if CurPageID = HostPage.ID then
  begin
    if Trim(HostPage.Values[0]) = '' then
    begin
      MsgBox('서버 주소를 입력하세요. 예: my-server 또는 100.101.102.103', mbError, MB_OK);
      Result := False;
    end
    else if Pos(' ', HostPage.Values[0]) > 0 then
    begin
      MsgBox('서버 주소에는 공백을 넣을 수 없습니다.', mbError, MB_OK);
      Result := False;
    end
    else if Trim(HostPage.Values[1]) = '' then
    begin
      Result := MsgBox('접속 토큰이 비어 있습니다. 서버가 토큰을 요구하면 백업이 거부됩니다.' + #13#10 +
        '서버의 %USERPROFILE%\.config\TDrive\web.token 값을 넣는 것이 좋습니다.' + #13#10#13#10 +
        '토큰 없이 계속할까요?', mbConfirmation, MB_YESNO) = IDYES;
    end;
    exit;
  end;
  if CurPageID = FolderPage.ID then
  begin
    if Trim(FolderPage.Values[0]) = '' then
    begin
      MsgBox('백업 폴더를 지정하세요.', mbError, MB_OK);
      Result := False;
    end
    else if not ForceDirectories(FolderPage.Values[0]) then
    begin
      MsgBox('백업 폴더를 만들 수 없습니다: ' + FolderPage.Values[0], mbError, MB_OK);
      Result := False;
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Config: String;
begin
  if CurStep <> ssPostInstall then
    exit;
  Config :=
    '{' + #13#10 +
    '  "host": "' + JsonEsc(Trim(HostPage.Values[0])) + '",' + #13#10 +
    '  "token": "' + JsonEsc(Trim(HostPage.Values[1])) + '",' + #13#10 +
    '  "folder": "' + JsonEsc(FolderPage.Values[0]) + '",' + #13#10 +
    '  "remote": "/Backup",' + #13#10 +
    '  "keep_original": false,' + #13#10 +
    '  "interval_seconds": 30,' + #13#10 +
    '  "settle_seconds": 10,' + #13#10 +
    '  "upload_timeout_seconds": 21600' + #13#10 +
    '}' + #13#10;
  if not SaveStringToFile(ExpandConstant('{app}\backup.json'), Config, False) then
    MsgBox('backup.json을 쓰지 못했습니다: ' + ExpandConstant('{app}\backup.json') + #13#10 +
      '설치 후 직접 만들면 됩니다.', mbError, MB_OK);
end;
