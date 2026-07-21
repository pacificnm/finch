#!/usr/bin/env bash
# Start Finch desktop in development mode.
# Runs the Tauri dev command from the correct UI package directory.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Load local environment variables (Schwab API credentials, etc.) if present.
if [[ -f .env ]]; then
  set -a
  # shellcheck source=/dev/null
  source .env
  set +a
fi

cd "$SCRIPT_DIR/desktop/ui"
npm run tauri:dev
