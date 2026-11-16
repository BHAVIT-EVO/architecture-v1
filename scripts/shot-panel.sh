#!/usr/bin/env bash
# Screenshot one panel of the running app, for evidence shots.
#
#   bash scripts/shot-panel.sh activity docs/shots/s9-3-activity.png
#
# Starts the web shell with EVO_START_PANEL, waits for the works to load, then
# captures the window and stops the app it started (by pid, never by name).
set -euo pipefail

panel="${1:?panel name}"
out="${2:?output png}"
bin="${EVO_BIN:-./target/debug/evo-desktop}"
wait_secs="${SHOT_WAIT:-95}"

EVO_UI=web EVO_START_PANEL="$panel" "$bin" >"/tmp/evo-shot-$panel.log" 2>&1 &
pid=$!
cleanup() { kill "$pid" 2>/dev/null || true; }
trap cleanup EXIT

sleep "$wait_secs"
wid=$(/tmp/wingeo | awk '$3 == "1200x880" { print $1 }' | head -1)
if [ -z "${wid:-}" ]; then
  echo "shot-panel: no 1200x880 window for $panel" >&2
  tail -5 "/tmp/evo-shot-$panel.log" >&2
  exit 1
fi
screencapture -x -o -l"$wid" "$out"
sips -Z 1100 "$out" >/dev/null
echo "shot-panel: $panel -> $out"
