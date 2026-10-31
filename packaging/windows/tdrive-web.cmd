@echo off
setlocal
rem Native install: skip FUSE/WinFsp mounts and let the daemon serve its private
rem WebDAV endpoint, which is the path the web UI already streams through.
set "TDRIVE_NO_OS_MOUNT=1"

rem The installer writes port.txt and listen.txt next to this script. Manual
rem edits take effect on the next start.
set "TDRIVE_PORT=8080"
if exist "%~dp0port.txt" set /p TDRIVE_PORT=<"%~dp0port.txt"
set "TDRIVE_LISTEN=127.0.0.1"
if exist "%~dp0listen.txt" set /p TDRIVE_LISTEN=<"%~dp0listen.txt"

rem The logon task starts this script with no argument, so the browser only
rem opens when the user launches TDrive from the Start menu (--open).
"%~dp0tdrive.exe" web --listen %TDRIVE_LISTEN% --port %TDRIVE_PORT% %1
