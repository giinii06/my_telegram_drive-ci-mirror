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
ChangesEnvironment=yes
UninstallDisplayName={#MyAppName}
AllowNoIcons=yes

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startup"; Description: "Windows를 시작할 때 TDrive 실행"; GroupDescription: "시작 옵션"; Flags: unchecked

[Files]
Source: "{#SourceDir}\tdrive.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\tdrive-web.cmd"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\tdrive-web.cmd"; WorkingDir: "{app}"; Comment: "TDrive 웹 서버를 시작하고 브라우저를 엽니다"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\tdrive-web.cmd"; WorkingDir: "{app}"; Tasks: desktopicon
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\tdrive-web.cmd"; WorkingDir: "{app}"; Tasks: startup; Comment: "로그인할 때 TDrive 웹 서버를 시작합니다"

[Run]
Filename: "{app}\tdrive-web.cmd"; Description: "TDrive 열기"; Flags: postinstall nowait skipifsilent shellexec

[Code]
const
  EnvironmentKey = 'Environment';

function PathContains(Haystack, Needle: string): Boolean;
var
  PaddedHaystack, PaddedNeedle: string;
begin
  PaddedHaystack := ';' + Uppercase(Haystack) + ';';
  PaddedNeedle := ';' + Uppercase(Needle) + ';';
  Result := Pos(PaddedNeedle, PaddedHaystack) > 0;
end;

procedure EnvAddPath(Path: string);
var
  Paths: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    Paths := '';
  if PathContains(Paths, Path) then
    exit;
  if (Paths <> '') and (Paths[Length(Paths)] <> ';') then
    Paths := Paths + ';';
  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths + Path);
end;

procedure EnvRemovePath(Path: string);
var
  Paths, PaddedPaths: string;
  Position: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    exit;
  // Pad with ';' so a match cannot land inside another entry, and search the
  // uppercased copy while deleting from the original (they keep the same
  // offsets, so the user's casing survives).
  Paths := ';' + Paths + ';';
  PaddedPaths := Uppercase(Paths);
  Position := Pos(';' + Uppercase(Path) + ';', PaddedPaths);
  if Position = 0 then
    exit;
  Delete(Paths, Position, Length(Path) + 2);
  if (Paths <> '') and (Paths[1] = ';') then
    Delete(Paths, 1, 1);
  if (Paths <> '') and (Paths[Length(Paths)] = ';') then
    Delete(Paths, Length(Paths), 1);
  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    EnvAddPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    EnvRemovePath(ExpandConstant('{app}'));
end;

procedure InitializeWizard;
begin
  WizardForm.WelcomeLabel2.Caption :=
    WizardForm.WelcomeLabel2.Caption + #13#10#13#10 +
    'TDrive 서버가 이 Windows 계정에 설치됩니다. 설치 후 시작 메뉴의 TDrive를 실행하면' +
    ' 웹 서버가 시작되고 기본 브라우저가 열립니다.' + #13#10 +
    '빌드: {#MyAppCommit}';
end;
