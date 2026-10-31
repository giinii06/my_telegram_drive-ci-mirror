@echo off
setlocal
rem Manual control for the background server. Autostart is the per-user Run
rem entry written by the installer, so nothing here changes startup.
set "DIR=%~dp0"
set "EXE=tdrive.exe"

if /I "%~1"=="start" goto start
if /I "%~1"=="stop" goto stop
if /I "%~1"=="restart" goto restart
echo usage: tdrive-service.cmd start^|stop^|restart >&2
exit /b 2

:start
tasklist /FI "IMAGENAME eq %EXE%" 2>nul | find /I "%EXE%" >nul && (
  echo TDrive: already running
  exit /b 0
)
start "" "%SystemRoot%\System32\wscript.exe" "%DIR%tdrive-hidden.vbs"
exit /b 0

:stop
"%DIR%tdrive.exe" daemon stop >nul 2>&1
taskkill /F /IM "%EXE%" >nul 2>&1
if errorlevel 1 (
  echo TDrive: not running
  exit /b 0
)
echo TDrive: stopped
exit /b 0

:restart
call "%~f0" stop >nul
ping -n 2 127.0.0.1 >nul
call "%~f0" start >nul
exit /b 0
