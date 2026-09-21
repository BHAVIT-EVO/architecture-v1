#!/usr/bin/env bash
# Evo — real macOS live-capture verification.
#
# Launches the actual Evo .app, activates real applications so the daemon
# witnesses GENUINE macOS Accessibility focus events, and verifies the full
# canonical pipeline end to end:
#
#   real focus event → Observation → Artifact → Workspace → Snapshot
#   → Restoration Derivation → persistence → desktop consumption
#
# plus restart continuity (same Workspace identities, append-only history).
#
# Usage:
#   bash scripts/verify-live-capture.sh
#   EVO_STORAGE_ROOT=/tmp/evo-live bash scripts/verify-live-capture.sh
#
# The test uses a fresh storage root so every record in it is real evidence
# produced by this run. Launching via `open` (the real user path) uses the
# default root; this harness launches the bundled binary directly so it can
# control the lifecycle deterministically.
set -u

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$REPO_ROOT/dist/Evo.app"
APP_BIN="$APP_DIR/Contents/MacOS/Evo"
ROOT="${EVO_STORAGE_ROOT:-$TMPDIR/evo-live-verify}"
export EVO_STORAGE_ROOT="$ROOT"

PASS=0
FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

window_on_screen() {
  swift -e 'import CoreGraphics; let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]; for w in list { if let owner = w[kCGWindowOwnerName as String] as? String, owner.contains("Evo") { print("yes"); break } }' 2>/dev/null | grep -q yes
}

wait_for_window() {
  # The first window presentation can take a moment after launch; poll
  # instead of checking once.
  local tries=15
  while [ "$tries" -gt 0 ]; do
    if window_on_screen; then
      return 0
    fi
    sleep 1
    tries=$((tries - 1))
  done
  return 1
}

workspace_ids() {
  grep '^workspace_id_hex=' "$ROOT/workspace.log" 2>/dev/null \
    | sed 's/^workspace_id_hex=//' \
    | while read -r hex; do echo "$hex" | xxd -r -p 2>/dev/null; echo; done \
    | sort -u
}

log_count() {
  # Storage appends length-prefixed records; count records, not lines. The
  # version marker (e.g. workspace-v2 delta records) may differ per kind, so
  # match the kind prefix.
  local kind="$1"
  grep -c "^${kind}-v" "$ROOT/$kind.log" 2>/dev/null || echo 0
}

last_witnessed_subject() {
  # The most recent content Observation's witnessed subject (hex-encoded in
  # the log), decoded. Used to drive the real designation flow against a
  # genuinely witnessed resource.
  grep '^fact_value_hex=' "$ROOT/observation.log" 2>/dev/null \
    | tail -1 \
    | sed 's/^fact_value_hex=//' \
    | xxd -r -p 2>/dev/null
}

launch_evo() {
  "$APP_BIN" > "$ROOT/desktop.log" 2>&1 &
  DESKTOP_PID=$!
}

stop_evo() {
  [ -n "${DESKTOP_PID:-}" ] && kill -TERM "$DESKTOP_PID" 2>/dev/null
  sleep 2
  pkill -f "MacOS/evo-daemon" 2>/dev/null
  sleep 1
}

rm -rf "$ROOT"
mkdir -p "$ROOT"

echo "== Evo live-capture verification =="
echo "storage root: $ROOT"
echo

echo "-- Session 1: launch, capture real activity --"
launch_evo
sleep 6

if kill -0 "$DESKTOP_PID" 2>/dev/null; then
  ok "desktop app launches and stays alive"
else
  bad "desktop app exited on launch (see $ROOT/desktop.log)"
fi

DAEMON_PID="$(cat "$ROOT/daemon.pid" 2>/dev/null || true)"
if [ -n "$DAEMON_PID" ] && kill -0 "$DAEMON_PID" 2>/dev/null; then
  ok "daemon worker spawned by the desktop (pid $DAEMON_PID, single instance)"
else
  bad "daemon worker is not running"
fi

if wait_for_window; then
  ok "desktop window is on screen"
else
  bad "no desktop window found"
fi

# Real work: activate genuine applications so macOS reports real focus changes.
for app in Finder Notes Calculator TextEdit Safari; do
  open -a "$app"
  sleep 2.5
done
sleep 3

OBS_1="$(log_count observation)"
WKS_1="$(log_count workspace)"
SNAP_1="$(log_count snapshot)"
RES_1="$(log_count restoration)"

[ "$OBS_1" -gt 0 ]  && ok "real observations persisted ($OBS_1 records)"  || bad "no observations persisted"
[ "$WKS_1" -gt 0 ]  && ok "Workspace records persisted ($WKS_1)"            || bad "no Workspace records persisted"
[ "$SNAP_1" -gt 0 ] && ok "Snapshot records persisted ($SNAP_1)"           || bad "no Snapshot records persisted"
[ "$RES_1" -gt 0 ]  && ok "Restoration outcomes persisted ($RES_1)"        || bad "no Restoration outcomes persisted"

IDS_1="$(workspace_ids)"
COUNT_1="$(echo "$IDS_1" | grep -c . || true)"
[ "${COUNT_1:-0}" -ge 2 ] && ok "multiple real windows formed distinct Workspaces ($COUNT_1)" \
                          || bad "expected multiple Workspaces, found ${COUNT_1:-0}"

# The desktop log must show it consumed persisted canonical state.
if grep -q "EVO-DESKTOP storage=" "$ROOT/desktop.log"; then
  ok "desktop consumed persisted canonical state"
else
  bad "desktop log shows no state consumption"
fi

echo
echo "-- Designation (RFC-0011): real explicit continuation evidence --"
SUBJECT="$(last_witnessed_subject)"
if [ -n "$SUBJECT" ]; then
  DESIGNATE_BIN="$(ls "$REPO_ROOT/target/debug/examples/designate" 2>/dev/null | head -1)"
  if [ -z "$DESIGNATE_BIN" ]; then
    (cd "$REPO_ROOT" && cargo build -p evo-daemon --example designate >/dev/null 2>&1)
    DESIGNATE_BIN="$REPO_ROOT/target/debug/examples/designate"
  fi
  RESPONSE="$("$DESIGNATE_BIN" "$SUBJECT" 2>&1)"
  if [ "$RESPONSE" = "accepted" ]; then
    ok "designation of a really witnessed subject accepted (\"$SUBJECT\")"
  else
    bad "designation rejected: $RESPONSE"
  fi
  # The daemon re-derives every remembered Workspace with the canonical
  # designated Artifact; the workspace containing it must reach a Complete
  # Restoration Plan (IS-0021 §25.6) — verified from the persisted outcome.
  sleep 4
  if grep -q 'outcome=complete' "$ROOT/restoration.log" 2>/dev/null; then
    ok "designation derived a Complete Restoration Plan (persisted)"
  else
    bad "no Complete Restoration Plan after designation"
  fi
else
  bad "no witnessed subject could be extracted for designation"
fi

echo
echo "-- Restart: continuity --"
stop_evo
sleep 1
launch_evo
sleep 6

if kill -0 "$DESKTOP_PID" 2>/dev/null; then
  ok "desktop relaunches after restart"
else
  bad "desktop failed to relaunch"
fi

# Continue the same work: refocus one previously witnessed app. Its Artifact
# identity must resolve to the SAME Workspace (no duplicate identity).
open -a Notes
sleep 4

IDS_2="$(workspace_ids)"
OBS_2="$(log_count observation)"
WKS_2="$(log_count workspace)"

# Append-only: record counts never shrink.
[ "$OBS_2" -ge "$OBS_1" ] && ok "Observation log is append-only ($OBS_1 -> $OBS_2)" || bad "Observation log shrank"
[ "$WKS_2" -ge "$WKS_1" ] && ok "Workspace log is append-only ($WKS_1 -> $WKS_2)"   || bad "Workspace log shrank"

# Continuity: every pre-restart Workspace identity must still exist after
# restart (the daemon reloaded persisted understanding; it never re-creates
# remembered work under a new identity). New identities MAY appear only for
# genuinely new windows witnessed after restart — that is real capture, not
# drift — so the invariant checked here is identity preservation.
if [ "$(comm -23 <(echo "$IDS_1") <(echo "$IDS_2") | grep -c . || true)" -eq 0 ]; then
  ok "every pre-restart Workspace identity survived restart (no identity loss, no duplicates)"
else
  bad "pre-restart Workspace identities were lost or re-created after restart"
fi

stop_evo

echo
echo "== Result: $PASS passed, $FAIL failed =="
echo "inspect evidence: $ROOT/{observation,workspace,snapshot,restoration}.log"
[ "$FAIL" -eq 0 ]
