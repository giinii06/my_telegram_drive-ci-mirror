@echo off
setlocal
rem The logon task is the autostart mechanism: schtasks runs at sign-in, the
rem VBS wrapper hides the console, and the task stays per-user so the daemon
rem keeps the same config directory as the tdrive CLI.
set "TASK=TDrive"
set "DIR=%~dp0"

if /I "%~1"=="install" goto install
if /I "%~1"=="remove" goto remove
if /I "%~1"=="start" goto start
if /I "%~1"=="stop" goto stop
echo usage: tdrive-service.cmd install^|remove^|start^|stop >&2
exit /b 2

:install
schtasks /Create /F /SC ONLOGON /TN "%TASK%" /TR "\"%SystemRoot%\System32\wscript.exe\" \"%DIR%tdrive-hidden.vbs\"" /RL LIMITED >nul 2>&1
if errorlevel 1 (
  echo TDrive: could not register the logon task >&2
  exit /b 1
)
schtasks /Run /TN "%TASK%" >nul 2>&1
exit /b 0

:remove
schtasks /End /TN "%TASK%" >nul 2>&1
schtasks /Delete /F /TN "%TASK%" >nul 2>&1
"%DIR%tdrive.exe" daemon stop >nul 2>&1
exit /b 0

:start
schtasks /Run /TN "%TASK%" >nul 2>&1
exit /b 0

:stop
schtasks /End /TN "%TASK%" >nul 2>&1
"%DIR%tdrive.exe" daemon stop >nul 2>&1
exit /b 0
