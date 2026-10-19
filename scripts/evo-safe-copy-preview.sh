#!/usr/bin/env bash
# Evo Safe Copy Coping helper for the consolidation pass.
# DO NOT run this file directly as a build/install step. It is a guarded
# preview tool: it prints the exact rsync/cp commands that WOULD copy the
# latest build artifacts into a fresh target app bundle, without executing
# any copy, delete, move, or codesign yet.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
OUT_DIR="${PROFILE:-release}"
PROFILE="${PROFILE:-release}"
if [ "$PROFILE" != "release" ] && [ "$PROFILE" != "dev" ]; then
  echo "error: PROFILE must be 'release' or 'dev' (got '$PROFILE')." >&2
  exit 1
fi

MULTIMEDIA_BINS=(
  "$TARGET_DIR/$OUT_DIR/evo-desktop"
  "$TARGET_DIR/$OUT_DIR/evo-daemon"
  "$TARGET_DIR/$OUT_DIR/evo-ledger-engine"
)

MACOS_DIR="${1:-/Applications/Evo.app/Contents/MacOS}"
RESOURCES_DIR="${2:-/Applications/Evo.app/Contents/Resources}"
VERBOSE="${VERBOSE:-0}"

if [ "$VERBOSE" = "1" ]; then
  set -x
fi

missing=0
for bin in "${MULTIMEDIA_BINS[@]}"; do
  if [ ! -x "$bin" ]; then
    echo "MISSING: $bin" >&2
    missing=$((missing + 1))
  fi
done

if [ "$missing" -gt 0 ]; then
  echo "error: $missing expected build artifact(s) are missing." >&2
  echo "build first: cargo build --release -p evo-desktop -p evo-daemon -p evo-ledger" >&2
  exit 1
fi

echo "# WOULD CREATE MACOS DIR:"
echo "mkdir -p '$MACOS_DIR'"
echo "# WOULD COPY LATEST BINARIES:"
for bin in "${MULTIMEDIA_BINS[@]}"; do
  echo "cp '$bin' '$MACOS_DIR/$(basename "$bin")'"
done
echo "# WOULD chmod:"
echo "chmod +x '$MACOS_DIR'/*"
echo
echo "# WOULD ENSURE RESOURCES DIR EXISTS:"
echo "mkdir -p '$RESOURCES_DIR'"
echo "# WOULD COPY VOICE MODELS FROM target/release (only if present):"
for model in whisper-base.en.bin melspectrogram.onnx hey_evoh.onnx embedding_model.onnx; do
  src="$TARGET_DIR/$OUT_DIR/$model"
  if [ -f "$src" ]; then
    echo "cp '$src' '$RESOURCES_DIR/$model'"
  else
    echo "# MISSING MODEL (would skip): $src"
  fi
done
echo
echo "# WOULD SET MODELS READ-ONLY:"
echo "chmod 444 '$RESOURCES_DIR'/whisper-base.en.bin '$RESOURCES_DIR'/melspectrogram.onnx '$RESOURCES_DIR'/hey_evoh.onnx '$RESOURCES_DIR'/embedding_model.onnx"
echo
echo "# WOULD LIST RESULT (preview only):"
echo "ls -la '$MACOS_DIR'"
echo "ls -la '$RESOURCES_DIR'"
echo
echo "# NOTE: This script does NOT execute the above. It is a safe preview."
echo "# To actually perform the copy, use the real build/install scripts or an"
echo "# explicit admin-reviewed command sequence."
