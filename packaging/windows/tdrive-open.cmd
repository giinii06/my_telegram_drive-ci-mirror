@echo off
setlocal
set "DIR=%~dp0"
set "PORT=8080"
if exist "%DIR%port.txt" set /p PORT=<"%DIR%port.txt"
set "TOKEN_FILE=%USERPROFILE%\.config\TDrive\web.token"

if not exist "%TOKEN_FILE%" call "%DIR%tdrive-service.cmd" start

set /a tries=0
:wait
if exist "%TOKEN_FILE%" goto open
set /a tries+=1
if %tries% geq 20 goto missing
ping -n 2 127.0.0.1 >nul
goto wait

:open
set /p TOKEN=<"%TOKEN_FILE%"
start "" "http://127.0.0.1:%PORT%/?token=%TOKEN%"
exit /b 0

:missing
echo TDrive: the web server did not start. Log: %LOCALAPPDATA%\TDrive\tdrive.log >&2
echo TDrive: run "%DIR%tdrive-service.cmd" start and try again. >&2
exit /b 1
