@echo off
set LEPTOS_OUTPUT_NAME=logmancer-web
set LEPTOS_SITE_ROOT=%~dp0site
if not exist "%~dp0logs" mkdir "%~dp0logs"
set LOGMANCER_LOG_FILE=%~dp0logs\logmancer-desktop.log
start "" "%~dp0logmancer-desktop.exe" %*
