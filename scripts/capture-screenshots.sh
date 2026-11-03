#!/usr/bin/env bash
set -euo pipefail

CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
OUT_DIR="docs/screenshots"
mkdir -p "$OUT_DIR"

echo "Capturing Stage 2, Stage 3, and Stage 4 screenshots from http://localhost:1420 ..."

"$CHROME" --headless=new --disable-gpu --hide-scrollbars \
  --window-size=1440,940 \
  --virtual-time-budget=2000 \
  --screenshot="$OUT_DIR/stage2-audio-core.png" \
  "http://localhost:1420/?demo=stage2"

"$CHROME" --headless=new --disable-gpu --hide-scrollbars \
  --window-size=1440,940 \
  --virtual-time-budget=2000 \
  --screenshot="$OUT_DIR/stage3-multimodal-listening.png" \
  "http://localhost:1420/?demo=stage3"

"$CHROME" --headless=new --disable-gpu --hide-scrollbars \
  --window-size=1440,1040 \
  --virtual-time-budget=2000 \
  --screenshot="$OUT_DIR/stage4-cleanup-preview.png" \
  "http://localhost:1420/?demo=stage4"

"$CHROME" --headless=new --disable-gpu --hide-scrollbars \
  --window-size=1440,940 \
  --virtual-time-budget=2000 \
  --screenshot="$OUT_DIR/stage5-onboarding-polish.png" \
  "http://localhost:1420/?demo=stage5"

echo "Saved screenshots to $OUT_DIR:"
ls -lh "$OUT_DIR"
