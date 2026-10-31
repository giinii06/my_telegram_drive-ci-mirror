[CmdletBinding()]
param(
  [string]$Version = "",
  [string]$Commit = ""
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$dist = Join-Path $repo "dist"

function Invoke-Git {
  param([string[]]$GitArgs)
  try {
    $output = & git -C $repo @GitArgs 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $output) { return $null }
    return ([string]($output | Select-Object -First 1)).Trim()
  } catch {
    return $null
  }
}

if (-not $Version) {
  $Version = Invoke-Git @("describe", "--tags", "--always", "--dirty")
  if (-not $Version) { throw "버전을 확인하지 못했습니다. -Version 0.1.0 처럼 지정하세요." }
  if ($Version.StartsWith("v")) { $Version = $Version.Substring(1) }
}
if (-not $Commit) {
  $Commit = Invoke-Git @("rev-parse", "--short", "HEAD")
  if (-not $Commit) { $Commit = "unknown" }
}

$numericVersion = "0.0.0.0"
if ($Version -match '^(\d+(?:\.\d+){0,3})') {
  $parts = @($Matches[1].Split('.')) + @('0', '0', '0')
  $numericVersion = ($parts[0..3] -join '.')
}

New-Item -ItemType Directory -Force -Path $dist | Out-Null

# The agent runs on the machine that holds the files, so it is its own
# executable; keeping the server binary out of this installer keeps the two
# installs independent.
$env:CGO_ENABLED = "0"
$env:GOOS = "windows"
$env:GOARCH = "amd64"
$ldflags = "-s -w -X main.buildVersion=$Version -X main.buildCommit=$Commit"
$backupExe = Join-Path $dist "tdrive-backup.exe"

Push-Location $repo
try {
  & go build -trimpath -ldflags $ldflags -o $backupExe ./cmd/tdrive-backup
  if ($LASTEXITCODE -ne 0) { throw "go build 실패 (exit $LASTEXITCODE)" }
} finally {
  Pop-Location
}

$iscc = $null
$found = Get-Command iscc.exe -ErrorAction SilentlyContinue
if ($found) { $iscc = $found.Source }
if (-not $iscc) {
  foreach ($candidate in @(
      "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
      "C:\Program Files\Inno Setup 6\ISCC.exe")) {
    if (Test-Path -LiteralPath $candidate) { $iscc = $candidate; break }
  }
}
if (-not $iscc) {
  throw "Inno Setup 6(ISCC.exe)을 찾을 수 없습니다. 'winget install JRSoftware.InnoSetup'로 설치한 뒤 다시 실행하세요."
}

& $iscc "/DMyAppVersion=$Version" "/DMyAppNumericVersion=$numericVersion" "/DMyAppCommit=$Commit" "/DBackupExe=$backupExe" "/DOutputDir=$dist" (Join-Path $PSScriptRoot "tdrive-backup.iss")
if ($LASTEXITCODE -ne 0) { throw "Inno Setup 컴파일 실패 (exit $LASTEXITCODE)" }

$installer = Join-Path $dist "TDrive-Backup-Setup-$Version.exe"
if (-not (Test-Path -LiteralPath $installer)) {
  throw "설치 파일이 생성되지 않았습니다: $installer"
}
Write-Host ""
Write-Host "완료: $installer"
Write-Host "버전: $Version ($Commit)"
