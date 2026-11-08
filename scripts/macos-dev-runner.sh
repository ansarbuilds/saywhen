#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$1"
shift
if [[ "$BIN" != /* ]]; then
  BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
fi
"$ROOT/scripts/sync-dev-app-bundle.sh" "$BIN"
APP="$ROOT/src-tauri/target/debug/saywhen-dev.app"
exec "$APP/Contents/MacOS/saywhen" "$@"
