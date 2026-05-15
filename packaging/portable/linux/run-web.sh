#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

export LEPTOS_OUTPUT_NAME=logmancer-web
export LEPTOS_SITE_ROOT="$SCRIPT_DIR/site"
mkdir -p "$SCRIPT_DIR/logs"
export LOGMANCER_LOG_FILE="$SCRIPT_DIR/logs/logmancer-web.log"

exec "$SCRIPT_DIR/logmancer-web" "$@"
