@echo off
setlocal
rem Runs the backup agent in the foreground. The logon entry point is
rem tdrive-backup-hidden.vbs instead; this script is for a visible manual run.
set "DIR=%~dp0"
set "CONFIG=%DIR%backup.json"

if not exist "%CONFIG%" (
  echo TDrive backup: no config at %CONFIG%
  echo TDrive backup: reinstall, or run tdrive-backup.exe setup --host ... --folder ...
  exit /b 1
)

if /I "%~1"=="once" (
  "%DIR%tdrive-backup.exe" once --config "%CONFIG%"
  exit /b %errorlevel%
)
if /I "%~1"=="status" (
  "%DIR%tdrive-backup.exe" status --config "%CONFIG%"
  exit /b %errorlevel%
)

"%DIR%tdrive-backup.exe" run --config "%CONFIG%"
exit /b %errorlevel%
