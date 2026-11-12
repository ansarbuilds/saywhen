#!/usr/bin/env bash
# Renders the README visuals from scripts/visuals/compose.html.
# Start the dev server first (npm run dev), then run this from the repository root.
set -euo pipefail
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
OUT="${1:-docs/visuals}"
shot() { # name width height
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
    --window-size="$2,$3" --virtual-time-budget=6000 \
    --screenshot="$OUT/$1.png" "http://localhost:1420/scripts/visuals/compose.html?frame=$1" >/dev/null 2>&1
  sips -s format jpeg -s formatOptions 88 "$OUT/$1.png" --out "$OUT/$1.jpg" >/dev/null && rm "$OUT/$1.png"
  echo "$OUT/$1.jpg"
}
shot hero 1600 860
shot steps 1600 520
shot app 1600 1000
shot social 1280 640
