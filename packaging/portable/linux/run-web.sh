#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

export LEPTOS_OUTPUT_NAME=logmancer-web
export LEPTOS_SITE_ROOT="$SCRIPT_DIR/site"
# Restricts server-side file browsing to the user's home directory; prefer a narrower directory when possible.
# export LOGMANCER_SERVER_FILE_ROOT=~/
# Exposes the standalone web server on all network interfaces at port 3000.
# export LOGMANCER_BIND_ADDR=0.0.0.0:3000
mkdir -p "$SCRIPT_DIR/logs"
export LOGMANCER_LOG_FILE="$SCRIPT_DIR/logs/logmancer-web.log"

exec "$SCRIPT_DIR/logmancer-web" "$@"
