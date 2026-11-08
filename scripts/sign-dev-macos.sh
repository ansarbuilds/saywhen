#!/usr/bin/env bash
# Re-sign the debug Mach-O after each link so TCC Accessibility survives cargo rebuilds.
# Ad-hoc linker signing uses a cdhash; each rebuild invalidates an existing grant.
set -euo pipefail

BIN="${1:-}"
if [[ -z "$BIN" || ! -f "$BIN" ]]; then
  echo "sign-dev-macos: missing binary path" >&2
  exit 1
fi

SIGN_ID="${SAYWHEN_DEV_SIGN_ID:-}"
if [[ -z "$SIGN_ID" ]]; then
  SIGN_ID="$(security find-identity -v -p codesigning 2>/dev/null | awk -F'"' '/Apple Development/ { print $2; exit }')"
fi

if [[ -z "$SIGN_ID" ]]; then
  echo "sign-dev-macos: no Apple Development identity; set SAYWHEN_DEV_SIGN_ID" >&2
  exit 0
fi

codesign --force --sign "$SIGN_ID" --identifier dev.saywhen.app --timestamp=none "$BIN" 2>&1
echo "sign-dev-macos: signed with \"$SIGN_ID\""
