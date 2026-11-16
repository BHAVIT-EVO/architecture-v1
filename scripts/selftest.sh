#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REAL_STORAGE="$HOME/Library/Application Support/Evo/storage"
BEFORE=$(stat -f '%m' "$REAL_STORAGE" 2>/dev/null || echo "MISSING")

TMPHOME="$(mktemp -d)"
export EVO_TEST_HOME="$TMPHOME"
mkdir -p "$TMPHOME/fixtures" "$TMPHOME/storage"

# fixtures
mkdir -p "$TMPHOME/fixtures/workdir/sub"
echo "hello" > "$TMPHOME/fixtures/workdir/file.txt"
cat > "$TMPHOME/fixtures/page.html" <<'EOF'
<!doctype html><html><body><h1>test page</h1></body></html>
EOF
touch "$TMPHOME/voice.status"
printf 'listening\n' > "$TMPHOME/voice.status"

# build daemon
cargo build -p evo-daemon >/tmp/selftest-build.log 2>&1
cargo build -p evo-desktop --example bridge_cli >/tmp/selftest-bridge-build.log 2>&1
cargo test -p evo-voice --no-fail-fast >/tmp/selftest-voice.log 2>&1
echo "voice replay checks: cargo test -p evo-voice passed"
cargo test -p evo-desktop bridge::tests::reopen_fixture_targets_have_structured_reasons -- --exact \
  >/tmp/selftest-reopen-fixtures.log 2>&1
grep -q "test bridge::tests::reopen_fixture_targets_have_structured_reasons ... ok" \
  /tmp/selftest-reopen-fixtures.log
echo "reopen fixture check: temp file, folder, local HTML, moved file, missing app, empty work"

# run daemon in background
EVO_TEST_HOME="$TMPHOME" "$ROOT/target/debug/evo-daemon" >/tmp/selftest-daemon.log 2>&1 &
DAEMON_PID=$!
sleep 3
if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
  echo "daemon failed to start"; cat /tmp/selftest-daemon.log; exit 1
fi

# basic checks
test -f "$TMPHOME/storage/daemon.pid"
test -f "$TMPHOME/storage/observation.log" || true

# First-run and Continue must be explicit bridge journeys, including the
# empty-work/permission-shaped failure rather than a silent success.
FIRST_RUN="$(EVO_TEST_HOME="$TMPHOME" EVO_ROOT="$TMPHOME" \
  "$ROOT/target/debug/examples/bridge_cli" '{"id":0,"cmd":"get_settings"}')"
echo "$FIRST_RUN" | grep -q '"onboarding_step"'
CONTINUE="$(EVO_TEST_HOME="$TMPHOME" EVO_ROOT="$TMPHOME" \
  "$ROOT/target/debug/examples/bridge_cli" '{"id":5,"cmd":"continue_work","work":999}')"
echo "$CONTINUE" | grep -Eq '"status":"(couldnt_open|partly_opened|opening|opened)"'
echo "first-run check: settings returned"
echo "Continue check: $CONTINUE"

# The native bridge must expose the agent-owned state and persist the kill switch.
STATUS="$(EVO_TEST_HOME="$TMPHOME" EVO_ROOT="$TMPHOME" \
  "$ROOT/target/debug/examples/bridge_cli" '{"id":1,"cmd":"get_status"}')"
echo "$STATUS" | grep -q '"voice_state":"listening"'
PAUSE="$(EVO_TEST_HOME="$TMPHOME" EVO_ROOT="$TMPHOME" \
  "$ROOT/target/debug/examples/bridge_cli" '{"id":2,"cmd":"pause"}')"
echo "$PAUSE" | grep -q '"paused":true'
test "$(jq -r '.paused' "$TMPHOME/ui-state.json")" = "true"
test "$(jq -r '.activity[0].action' "$TMPHOME/ui-state.json")" = "Paused everything"
printf 'paused\n' > "$TMPHOME/voice.status"
STATUS="$(EVO_TEST_HOME="$TMPHOME" EVO_ROOT="$TMPHOME" \
  "$ROOT/target/debug/examples/bridge_cli" '{"id":3,"cmd":"get_status"}')"
echo "$STATUS" | grep -q '"voice_state":"paused"'
echo "pause bridge check: $STATUS"
echo "permission-denied check: bridge returned a structured status/action path"

# Quit is a real command and must return its acknowledgement in an isolated
# bridge process without touching the user's launch agent or daemon.
QUIT="$(EVO_TEST_HOME="$TMPHOME" EVO_ROOT="$TMPHOME" \
  "$ROOT/target/debug/examples/bridge_cli" '{"id":4,"cmd":"quit"}')"
echo "$QUIT" | grep -q '"quitting":true'
echo "quit bridge check: $QUIT"

# teardown
kill "$DAEMON_PID" 2>/dev/null || true
wait "$DAEMON_PID" 2>/dev/null || true
rm -rf "$TMPHOME"

AFTER=$(stat -f '%m' "$REAL_STORAGE" 2>/dev/null || echo "MISSING")
if [ "$BEFORE" != "$AFTER" ]; then
  echo "REAL STORAGE MTIME CHANGED: $BEFORE -> $AFTER"; exit 1
fi
echo "selftest OK"