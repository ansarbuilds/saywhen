#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$1"
shift
if [[ "$BIN" != /* ]]; then
  BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
fi
# Only wrap the main app binary. `cargo test` and `cargo run --bin replay` also use the
# target runner; overwriting `saywhen-dev.app` while the app is open breaks macOS TCC.
if [[ "$(basename "$BIN")" != "saywhen" || "$BIN" == */deps/* ]]; then
  exec "$BIN" "$@"
fi
"$ROOT/scripts/sync-dev-app-bundle.sh" "$BIN"
APP="$ROOT/src-tauri/target/debug/saywhen-dev.app"
exec "$APP/Contents/MacOS/saywhen" "$@"
