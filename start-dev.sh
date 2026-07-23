#!/usr/bin/env bash
# Start Finch desktop in development mode.
#
# Invokes the locally-installed tauri CLI binary directly, cd'd straight
# into desktop/src-tauri/ (where tauri.conf.json lives) — deliberately not
# via `npm run`/`npm exec --prefix`. That indirection (desktop/ui/package.json's
# old "tauri:dev": "cd .. && npm exec --prefix ui tauri dev", and the
# shared nest-build lib's `npm run tauri:dev --prefix ui`) has cwd
# resolution behavior for the tauri CLI's own process that isn't fully
# pinned down — different invocations of the same script chain have been
# observed both working (devUrl reachable, frontendDist irrelevant) and
# failing ("asset not found: index.html", meaning the CLI resolved
# tauri.conf.json's relative frontendDist path, or beforeDevCommand's own
# relative path, against the wrong base directory). Removing every layer
# of npm --prefix/exec indirection and just `cd`-ing to the exact right
# directory before invoking the binary removes that ambiguity entirely.
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

 export DATABASE_URL="${DATABASE_URL}"
 cd "$SCRIPT_DIR/desktop/src-tauri"
exec "$SCRIPT_DIR/desktop/ui/node_modules/.bin/tauri" dev "$@"
