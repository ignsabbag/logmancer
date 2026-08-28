@echo off
set LEPTOS_OUTPUT_NAME=logmancer-web
set LEPTOS_SITE_ROOT=%~dp0site
rem Restricts server-side file browsing to the user's home directory; prefer a narrower directory when possible.
rem set "LOGMANCER_SERVER_FILE_ROOT=%USERPROFILE%"
rem Exposes the standalone web server on all network interfaces at port 3000.
rem set "LOGMANCER_BIND_ADDR=0.0.0.0:3000"
if not exist "%~dp0logs" mkdir "%~dp0logs"
set LOGMANCER_LOG_FILE=%~dp0logs\logmancer-web.log
"%~dp0logmancer-web.exe" %*
