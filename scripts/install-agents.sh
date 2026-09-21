#!/usr/bin/env bash
# Installs (or removes) the LaunchAgents that keep Evo's capture running
# always: launchd owns the daemon and the engine; the desktop shell is
# their UI. Capture then survives reboots and app quits — the "always
# remembers" promise.
#
# Usage:
#   bash scripts/install-agents.sh            # install + start (dev paths)
#   BUNDLE=/Applications/Evo.app bash scripts/install-agents.sh   # release app
#   bash scripts/install-agents.sh --remove   # stop + uninstall
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STORAGE_ROOT="$HOME/Library/Application Support/evo/storage"
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

if [ -n "${BUNDLE:-}" ]; then
  # Release layout: the agents run the binaries inside the .app bundle.
  DAEMON_BIN="$BUNDLE/Contents/MacOS/evo-daemon"
  ENGINE_BIN="$BUNDLE/Contents/MacOS/evo-ledger-engine"
else
  # Dev layout: the agents run the cargo output directly.
  DAEMON_BIN="$REPO_ROOT/target/release/evo-daemon"
  ENGINE_BIN="$REPO_ROOT/target/release/evo-ledger-engine"
fi

for BIN in "$DAEMON_BIN" "$ENGINE_BIN"; do
  if [ ! -x "$BIN" ]; then
    echo "missing binary: $BIN (build first: cargo build --release -p evo-daemon -p evo-ledger)" >&2
    exit 1
  fi
done

mkdir -p "$AGENTS_DIR"

# The daemon: capture (AX focus, FSEvents saves, input counters, URL polls).
# KeepAlive only on crash — a clean exit (single-instance lock loss, SIGTERM)
# stays down; RunAtLoad starts capture at login.
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
launchctl bootout "gui/$UID_/dev.evo.capture" 2>/dev/null || true
launchctl bootout "gui/$UID_/dev.evo.ledger" 2>/dev/null || true
launchctl bootstrap "gui/$UID_" "$AGENTS_DIR/dev.evo.capture.plist"
launchctl bootstrap "gui/$UID_" "$AGENTS_DIR/dev.evo.ledger.plist"
launchctl kickstart "gui/$UID_/dev.evo.capture"
launchctl kickstart "gui/$UID_/dev.evo.ledger"

echo "installed: dev.evo.capture, dev.evo.ledger (RunAtLoad + KeepAlive-on-crash)"
echo "capture now survives reboots and app quits; remove with: $0 --remove"
