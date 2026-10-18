#!/usr/bin/env bash
# Installs (or removes) the LaunchAgents that keep Evo's capture running
# always. The voice-capable `com.evo.agent` owns and supervises the daemon
# when it is installed; this script must not launch a second daemon against
# the same storage root. On machines without the voice agent, the legacy
# `dev.evo.capture` fallback remains available. The ledger engine is always
# installed here.
#
# Usage:
#   bash scripts/install-agents.sh            # install + start from /Applications/Evo.app
#   BUNDLE=/Applications/Evo.app bash scripts/install-agents.sh   # explicit canonical path
#   EVO_USE_DEV_BINARIES=1 bash scripts/install-agents.sh         # explicit dev binaries
#   bash scripts/install-agents.sh --remove   # stop + uninstall
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CANONICAL_BUNDLE="/Applications/Evo.app"
STORAGE_ROOT="$HOME/Library/Application Support/Evo/storage"
AGENTS_DIR="$HOME/Library/LaunchAgents"
ACTION="${1:-install}"

remove() {
  for LABEL in dev.evo.capture dev.evo.ledger; do
    launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
    rm -f "$AGENTS_DIR/$LABEL.plist"
    echo "removed: $LABEL"
  done
}

if [ "$ACTION" = "--remove" ]; then
  remove
  exit 0
fi

if [ -n "${BUNDLE:-}" ] && [ "$BUNDLE" != "$CANONICAL_BUNDLE" ]; then
  echo "refusing non-canonical Evo bundle: $BUNDLE" >&2
  echo "use $CANONICAL_BUNDLE instead" >&2
  exit 1
fi
BUNDLE="${BUNDLE:-$CANONICAL_BUNDLE}"

if [ -n "${BUNDLE:-}" ]; then
  # Release layout: the agents run the binaries inside the .app bundle.
  DAEMON_BIN="$BUNDLE/Contents/MacOS/evo-daemon"
  ENGINE_BIN="$BUNDLE/Contents/MacOS/evo-ledger-engine"
elif [ "${EVO_USE_DEV_BINARIES:-}" = "1" ]; then
  # Development is opt-in because its helper has a different TCC identity
  # than the bundle a user enables in System Settings.
  DAEMON_BIN="$REPO_ROOT/target/release/evo-daemon"
  ENGINE_BIN="$REPO_ROOT/target/release/evo-ledger-engine"
else
  echo "missing bundled Evo.app; build it first with: bash scripts/build-app.sh" >&2
  echo "set EVO_USE_DEV_BINARIES=1 only when intentionally testing cargo output." >&2
  exit 1
fi

for BIN in "$DAEMON_BIN" "$ENGINE_BIN"; do
  if [ ! -x "$BIN" ]; then
    echo "missing binary: $BIN (build first: cargo build --release -p evo-daemon -p evo-ledger)" >&2
    exit 1
  fi
done

mkdir -p "$AGENTS_DIR"

# The engine: derives works from the observation log every 10 s.
cat > "$AGENTS_DIR/dev.evo.ledger.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>dev.evo.ledger</string>
  <key>ProgramArguments</key>
  <array>
    <string>$ENGINE_BIN</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>EVO_STORAGE_ROOT</key>
    <string>$STORAGE_ROOT</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>Crashed</key>
    <true/>
    <key>SuccessfulExit</key>
    <false/>
  </dict>
  <key>ProcessType</key>
  <string>Background</string>
</dict>
</plist>
PLIST

UID_="$(id -u)"

bootstrap_agent() {
  local domain="$1"
  local plist="$2"
  local attempt=1
  while ! launchctl bootstrap "$domain" "$plist"; do
    if [ "$attempt" -ge 5 ]; then
      return 1
    fi
    attempt=$((attempt + 1))
    sleep 0.25
  done
}

# There must be exactly one daemon owner for the shared storage root. The
# headless voice agent owns that daemon when present; remove the old capture
# registration before installing the ledger so it cannot race the agent.
VOICE_AGENT_SERVICE="gui/$UID_/com.evo.agent"
if launchctl print "$VOICE_AGENT_SERVICE" >/dev/null 2>&1; then
  launchctl bootout "gui/$UID_/dev.evo.capture" 2>/dev/null || true
  rm -f "$AGENTS_DIR/dev.evo.capture.plist"
  echo "voice agent owns daemon: retired dev.evo.capture"
else
  cat > "$AGENTS_DIR/dev.evo.capture.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>dev.evo.capture</string>
  <key>ProgramArguments</key>
  <array>
    <string>$DAEMON_BIN</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>EVO_STORAGE_ROOT</key>
    <string>$STORAGE_ROOT</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>Crashed</key>
    <true/>
    <key>SuccessfulExit</key>
    <false/>
  </dict>
  <key>ProcessType</key>
  <string>Background</string>
</dict>
</plist>
PLIST
fi

launchctl bootout "gui/$UID_/dev.evo.capture" 2>/dev/null || true
launchctl bootout "gui/$UID_/dev.evo.ledger" 2>/dev/null || true
if [ -f "$AGENTS_DIR/dev.evo.capture.plist" ]; then
  bootstrap_agent "gui/$UID_" "$AGENTS_DIR/dev.evo.capture.plist"
fi
bootstrap_agent "gui/$UID_" "$AGENTS_DIR/dev.evo.ledger.plist"
if [ -f "$AGENTS_DIR/dev.evo.capture.plist" ]; then
  launchctl kickstart "gui/$UID_/dev.evo.capture"
fi
launchctl kickstart "gui/$UID_/dev.evo.ledger"

if [ -f "$AGENTS_DIR/dev.evo.capture.plist" ]; then
  echo "installed: dev.evo.capture, dev.evo.ledger (RunAtLoad + KeepAlive-on-crash)"
else
  echo "installed: dev.evo.ledger; daemon remains owned by com.evo.agent"
fi
echo "capture now survives reboots and app quits; remove with: $0 --remove"
