@echo off
setlocal
rem Manual control for the background backup agent. Autostart is a per-user
rem Run entry written by the installer, so nothing here changes startup.
set "DIR=%~dp0"
set "EXE=tdrive-backup.exe"

if /I "%~1"=="start" goto start
if /I "%~1"=="stop" goto stop
if /I "%~1"=="restart" goto restart
echo usage: tdrive-backup-service.cmd start^|stop^|restart >&2
exit /b 2

:start
tasklist /FI "IMAGENAME eq %EXE%" 2>nul | find /I "%EXE%" >nul && (
  echo TDrive backup: already running
  exit /b 0
)
start "" "%SystemRoot%\System32\wscript.exe" "%DIR%tdrive-backup-hidden.vbs"
exit /b 0

:stop
taskkill /F /IM "%EXE%" >nul 2>&1
if errorlevel 1 (
  echo TDrive backup: not running
  exit /b 0
)
echo TDrive backup: stopped
exit /b 0

:restart
taskkill /F /IM "%EXE%" >nul 2>&1
ping -n 2 127.0.0.1 >nul
start "" "%SystemRoot%\System32\wscript.exe" "%DIR%tdrive-backup-hidden.vbs"
exit /b 0
