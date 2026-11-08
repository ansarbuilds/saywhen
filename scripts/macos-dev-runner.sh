#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$1"
shift
if [[ "$BIN" != /* ]]; then
  BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
fi
"$ROOT/scripts/sign-dev-macos.sh" "$BIN"
exec "$BIN" "$@"
