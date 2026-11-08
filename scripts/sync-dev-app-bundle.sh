#!/usr/bin/env bash
# Wrap the debug Mach-O in saywhen-dev.app so Dock/System Settings get the real icon and
# Accessibility grants bind to bundle id dev.saywhen.app instead of a raw linker hash.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${1:?binary path required}"
if [[ "$BIN" != /* ]]; then
  BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
fi

TAURI_DIR="$ROOT/src-tauri"
APP="$TAURI_DIR/target/debug/saywhen-dev.app"
MACOS_DIR="$APP/Contents/MacOS"
RES_DIR="$APP/Contents/Resources"

mkdir -p "$MACOS_DIR" "$RES_DIR"
cp "$BIN" "$MACOS_DIR/saywhen"
chmod +x "$MACOS_DIR/saywhen"
cp "$TAURI_DIR/icons/icon.icns" "$RES_DIR/icon.icns"
cp "$ROOT/scripts/saywhen-dev-Info.plist" "$APP/Contents/Info.plist"

SIGN_ID="${SAYWHEN_DEV_SIGN_ID:-}"
if [[ -z "$SIGN_ID" ]]; then
  SIGN_ID="$(security find-identity -v -p codesigning 2>/dev/null | awk -F'"' '/Apple Development/ { print $2; exit }')"
fi

if [[ -z "$SIGN_ID" ]]; then
  echo "sync-dev-app-bundle: no Apple Development identity; set SAYWHEN_DEV_SIGN_ID" >&2
  exit 0
fi

codesign --force --deep --sign "$SIGN_ID" --identifier dev.saywhen.app --timestamp=none "$APP" 2>&1
echo "sync-dev-app-bundle: $APP"
