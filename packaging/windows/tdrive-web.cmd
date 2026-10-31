@echo off
setlocal
rem Native install: skip FUSE/WinFsp mounts and let the daemon serve its private
rem WebDAV endpoint, which is the path the web UI already streams through.
set "TDRIVE_NO_OS_MOUNT=1"
if not defined TDRIVE_PORT set "TDRIVE_PORT=8080"

echo Starting the TDrive web server. Closing this window stops the server.
echo Address: http://127.0.0.1:%TDRIVE_PORT%/
echo.
"%~dp0tdrive.exe" web --listen 127.0.0.1 --port %TDRIVE_PORT% --open

echo.
echo The TDrive web server stopped. Run TDrive again to restart it.
pause >nul
