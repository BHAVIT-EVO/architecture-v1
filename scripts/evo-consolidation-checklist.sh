#!/usr/bin/env bash
# Evo Consolidation Checklist / Planner
# This script prints the consolidation plan with checkboxes and the commands
# that WOULD be used at each step. It does NOT perform any destructive
# operation. Default mode is --dry-run.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${PROFILE:-release}"
if [ "$PROFILE" != "release" ] && [ "$PROFILE" != "dev" ]; then
  echo "error: PROFILE must be 'release' or 'dev' (got '$PROFILE')." >&2
  exit 1
fi

TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
OUT_DIR="$TARGET_DIR/$PROFILE"

INSTALLED_APP="/Applications/Evo.app"
BACKUP_ROOT="$REPO_ROOT/backups"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
BACKUP_APP="$BACKUP_ROOT/installed-Evo.app.$TIMESTAMP"

AGENT_BIN="$HOME/Library/Application Support/Evo/agent/evo-agent"
AGENT_PLIST="$HOME/Library/LaunchAgents/com.evo.agent.plist"
AGENT_LOG="$HOME/Library/Logs/evo-agent.log"
AGENT_DATA="$HOME/Library/Application Support/Evo"

usage() {
  cat <<EOF
Usage: bash $0 [--help]

Prints the Evo consolidation plan with checkboxes and the commands that
WOULD be used at each step. No destructive operation is executed.

Environment:
  PROFILE=release|dev   which cargo profile to use (default: release)
EOF
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

cat <<EOF
=== Evo consolidation plan (dry-run) ===
Workspace: $REPO_ROOT
Profile : $PROFILE
Out dir : $OUT_DIR
Timestamp: $TIMESTAMP

--- Step 1: Backup installed app + agent state (WOULD do) ---
[ ] Backup installed app to: $BACKUP_APP
Would run (review before uncommenting):
  mkdir -p '$BACKUP_ROOT'
  if [ -d '$INSTALLED_APP' ]; then
    cp -R '$INSTALLED_APP' '$BACKUP_APP'
    echo "backed up installed app -> $BACKUP_APP"
  else
    echo "no installed app at $INSTALLED_APP to back up"
  fi

[ ] Save agent state snapshot (WOULD do)
Would run:
  mkdir -p '$BACKUP_ROOT/agent-state.$TIMESTAMP'
  [ -f '$AGENT_PLIST' ] && cp '$AGENT_PLIST' '$BACKUP_ROOT/agent-state.$TIMESTAMP/com.evo.agent.plist'
  [ -f '$AGENT_LOG' ] && cp '$AGENT_LOG' '$BACKUP_ROOT/agent-state.$TIMESTAMP/evo-agent.log'
  [ -d '$AGENT_DATA' ] && tar czf '$BACKUP_ROOT/agent-state.$TIMESTAMP/application-support-evo.tar.gz' -C '$HOME/Library/Application Support' Evo
  echo "saved agent state snapshot -> $BACKUP_ROOT/agent-state.$TIMESTAMP"

--- Step 2: Verify source + build + voice models (WOULD verify) ---
[ ] Confirm latest binaries exist
Would check:
  - $OUT_DIR/evo-desktop
  - $OUT_DIR/evo-daemon
  - $OUT_DIR/evo-ledger-engine
And voice models:
  - $OUT_DIR/whisper-base.en.bin
  - $OUT_DIR/melspectrogram.onnx
  - $OUT_DIR/hey_evoh.onnx
  - $OUT_DIR/embedding_model.onnx
If any is missing, build first:
  cargo build --profile $PROFILE -p evo-desktop -p evo-daemon -p evo-ledger

[ ] Confirm consolidated bundle would have the right features
Would verify installed/consolidated evo-desktop has:
  - notch symbols (notch_window::spawn, NotchEngine::voice_event, show_live,
    surface_layouts, occupancy_probe, EvoNotchHost render path)
  - voice-audio / voice-wake symbols (AcousticWake / CustomWakeSpotter,
    Transcriber, Mic, spawn_voice_thread, VoiceSocketClient)
And that voice models are shipped inside the bundle's Resources.

--- Step 3: Assemble consolidated Evo.app (WOULD do) ---
[ ] Create fresh bundle at: $INSTALLED_APP
Would run (after reviewing):
  rm -rf '$INSTALLED_APP'   # only after confirming nothing important is there
  mkdir -p '$INSTALLED_APP/Contents/MacOS'
  mkdir -p '$INSTALLED_APP/Contents/Resources'
  cp '$OUT_DIR/evo-desktop' '$INSTALLED_APP/Contents/MacOS/Evo'
  cp '$OUT_DIR/evo-daemon' '$INSTALLED_APP/Contents/MacOS/evo-daemon'
  cp '$OUT_DIR/evo-ledger-engine' '$INSTALLED_APP/Contents/MacOS/evo-ledger-engine'
  chmod +x '$INSTALLED_APP/Contents/MacOS'/*
  cp '$OUT_DIR/whisper-base.en.bin' '$INSTALLED_APP/Contents/Resources/'
  cp '$OUT_DIR/melspectrogram.onnx' '$INSTALLED_APP/Contents/Resources/'
  cp '$OUT_DIR/hey_evoh.onnx' '$INSTALLED_APP/Contents/Resources/'
  cp '$OUT_DIR/embedding_model.onnx' '$INSTALLED_APP/Contents/Resources/'
  chmod 444 '$INSTALLED_APP/Contents/Resources/'*.bin '$INSTALLED_APP/Contents/Resources/'*.onnx

[ ] Install Info.plist + icon (WOULD do)
Would install the same Info.plist the build script emits, with:
  CFBundleIdentifier = dev.evo.desktop
  CFBundleExecutable = Evo
  CFBundleShortVersionString = 0.1.0
  CFBundleVersion = 1
  LSMinimumSystemVersion = 13.0

[ ] Sign binaries (WOULD do, only if stable identity exists)
Would run:
  bash scripts/sign-dev.sh
or for an explicit bundle:
  bash scripts/sign-dev.sh --app '$INSTALLED_APP'
Note: stable signing identity required for TCC grants to survive rebuilds.

--- Step 4: Stop/remove stale processes and stale bundles (WOULD do) ---
[ ] Stop stale processes still holding deleted-bundle paths
Would inspect and stop:
  - PID 61734 /Users/bhavitsaini/Applications/'Evo Development.app'/Contents/MacOS/evo-ledger-engine
  - PID 63123 /Users/bhavitsaini/Applications/'Evo Development.app'/Contents/MacOS/evo-daemon
  - PID 59588 /private/var/tmp/evo-bundled-dist-ipijKy/Evo.app/Contents/MacOS/evo-ledger-engine
Approach:
  launchctl bootout gui/$(id -u)/dev.evo.ledger   2>/dev/null || true
  launchctl bootout gui/$(id -u)/dev.evo.capture  2>/dev/null || true
  launchctl bootout gui/$(id -u)/com.evo.agent    2>/dev/null || true
  # Then kill the stale PIDs if still alive, and remove the tmp bundle only after
  # confirming it is a duplicate/temporary copy.

[ ] Remove stale/deleted bundle artifacts (WOULD do, after review)
Would consider removing:
  - /private/var/tmp/evo-bundled-dist-ipijKy/Evo.app   (temporary copy)
  - any leftover traces of /Users/.../Evo Development.app if truly obsolete
Only after confirming:
  - installed consolidated app exists and works
  - agent/daemon/ledger converge on it
  - the temp bundle is not the source of truth

--- Step 5: Converge agent + LaunchAgents on the one app (WOULD do) ---
[ ] Keep/reinstall the agent LaunchAgent
Would ensure:
  - ~/Library/LaunchAgents/com.evo.agent.plist exists
  - ProgramArguments -> agent binary in Application Support, not a stray deleted-bundle path
  - RunAtLoad = true
  - Agent binary has voice + notch-forwarder features

[ ] Reinstall capture + ledger LaunchAgents from the consolidated app
Would run (after building + signing the bundle):
  BUNDLE=/Applications/Evo.app bash scripts/install-agents.sh

--- Step 6: Verify wiring + permissions + Hey Evo when not frontmost (WOULD verify) ---
[ ] Verify permissions granted and effective
Would check:
  - Microphone permission for the agent/binary that listens
  - Accessibility / input monitoring where required
  - Any screen/window/audio capture permissions needed by the capture daemon
Use System Settings + live behavior, not just "requested".

[ ] Verify Hey Evo when Evo app is not frontmost
Would:
  - Ensure agent is running
  - Ensure consolidated app's evo-desktop can talk to the voice socket / notch surface
  - Confirm agent hears "hey evo" and can drive the notch/notifications even when the
    Evo window is not active.

--- Step 7: Tests + hostile smoke test + iterate (WOULD do) ---
[ ] Run the project's tests
Would run (as applicable):
  cargo test

[ ] Hostile smoke test
Would:
  - Kill/duplicate processes, relaunch, confirm no leftover stale paths
  - Watch logs: $AGENT_LOG
  - Confirm no duplicate Evo stacks remain
  - Confirm one installed app is the single source of truth

--- End of plan ---
EOF
