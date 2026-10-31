@echo off
rem Shows the agent's current configuration and queue. Kept open so the numbers
rem stay readable after the command returns.
setlocal
cd /d "%~dp0"
call tdrive-backup.cmd status
echo.
pause
