#!/usr/bin/env bash
# The mission's source of truth: one PASS or FAIL line per check.
# Every phase appends its own section. Thresholds come from
# docs/mission/MISSION.md; none are invented in this script.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
FAILED=0

pass() { printf 'PASS %s\n' "$1"; }
fail() { printf 'FAIL %s: %s\n' "$1" "$2"; FAILED=1; }

check() { # id, description, command
  local id="$1" description="$2"
  shift 2
  local output
  if output="$("$@" 2>&1)"; then
    pass "$id"
  else
    fail "$id" "$description"
    printf '%s\n' "$output" | sed 's/^/    /' | tail -20
  fi
}

# ── 0: baseline artefacts ───────────────────────────────────────────────
check 0.0-mission-saved "docs/mission/MISSION.md missing or empty" \
  test -s docs/mission/MISSION.md
check 0.1-tag "git tag pre-mission missing" \
  git rev-parse -q --verify refs/tags/pre-mission
check 0.4-state "docs/mission/STATE.md missing" test -s docs/mission/STATE.md

# ── 1: the build works by itself ────────────────────────────────────────
check 1.2-ort-pin "scripts/ort.sha256 missing" test -s scripts/ort.sha256
check 1.2-no-static-branch "build-app.sh still selects static ORT linking for voice builds" \
  bash -c '! grep -q "static ONNX Runtime linking was selected" scripts/build-app.sh || grep -q "ORT_PREFER_DYNAMIC_LINK=1" scripts/build-app.sh'

# ── 2: the voice helper ─────────────────────────────────────────────────
check 2.3-voice-state-fields "get_voice_state is missing one of its six fields" \
  bash -c 'for f in agent_loaded pid build_id mic_auth paused last_heartbeat_ms; do grep -q "$f" crates/evo-desktop/src/bridge.rs || exit 1; done'

# ── 3: Continue reopens work ────────────────────────────────────────────
check 3.2-no-voice-socket "continue_work still routes through the voice socket" \
  bash -c '! sed -n "/fn continue_work/,/^    }$/p" crates/evo-desktop/src/bridge.rs | grep -q VoiceSocket'
check 3.3-fixtures "reopen fixture test missing" \
  grep -q "reopen_fixture_targets_have_structured_reasons" crates/evo-desktop/src/bridge.rs

# ── 4: Hey Evo ──────────────────────────────────────────────────────────
check 4.1-model-dir "EVO_MODEL_DIR is not read by the voice binaries" \
  bash -c 'grep -rq "EVO_MODEL_DIR" crates/evo-voice/src crates/evo-desktop/src'

# ── 5: Day and Dusk ─────────────────────────────────────────────────────
check 5.1-scene-themes "scene has no dusk/day palette pair" \
  bash -c 'grep -q "SCENES" crates/evo-desktop/ui/scene/stage.js'
check 5.4-theme-proof "docs/mission/THEME-MEASURE.md missing" \
  test -s docs/mission/THEME-MEASURE.md

# ── 6: settings, language, titles, claims ───────────────────────────────
check 6.3-strings "docs/mission/STRINGS.md missing" test -s docs/mission/STRINGS.md

# ── 7: final gate artefacts ─────────────────────────────────────────────
check 7.3-human "docs/mission/HUMAN.md missing" test -s docs/mission/HUMAN.md
check 7.4-report "docs/mission/FINAL_REPORT.md missing" test -s docs/mission/FINAL_REPORT.md

if [ "$FAILED" -ne 0 ]; then
  echo "MISSION VERIFY: FAIL"
  exit 1
fi
echo "MISSION VERIFY: PASS"
