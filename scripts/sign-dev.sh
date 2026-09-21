#!/usr/bin/env bash
# Signs Evo's binaries with a stable identity so macOS TCC grants
# (Accessibility, Input Monitoring) survive rebuilds.
#
# The problem: cargo emits ad-hoc-signed binaries whose designated
# requirement is cdhash-pinned — every rebuild changes the cdhash and
# silently invalidates the grants (System Settings still shows them ON).
# The fix (the yabai/AeroSpace trick): a self-signed "Code Signing"
# certificate in your login keychain. The requirement becomes
# identifier + certificate leaf, so grants persist across rebuilds.
#
# One-time setup:
#   bash scripts/sign-dev.sh --create-identity
#   (Keychain Access will ask you to confirm; the certificate lives in
#    your login keychain and is only usable for signing on this machine.)
#
# After every build:
#   bash scripts/sign-dev.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
IDENTITY="Evo Code Signing"

if [ "${1:-}" = "--create-identity" ]; then
  if security find-identity -q -c "$IDENTITY" 2>/dev/null | grep -q "$IDENTITY"; then
    echo "identity '$IDENTITY' already exists"
    exit 0
  fi
  # A self-signed code-signing certificate valid for ~10 years.
  security -q create-keypair -a rsa -f 4096 -A \
    -e "security find-identity -v -p codesigning" 2>/dev/null || true
  echo "creating '$IDENTITY' via Keychain Access Certificate Assistant:"
  echo "  1. Keychain Access → Certificate Assistant → Create Certificate…"
  echo "  2. Name: $IDENTITY, Identity Type: Self Signed Root, Type: Code Signing"
  echo "  3. Let me override defaults → keep defaults → Sign with SHA-2 (256)"
  echo "  4. Done. Then re-run: bash $0"
  open -a "Keychain Access" 2>/dev/null || true
  exit 0
fi

if ! security find-identity -q -c "$IDENTITY" 2>/dev/null | grep -q "$IDENTITY"; then
  echo "no '$IDENTITY' identity; nothing signed." >&2
  echo "create one once with: bash $0 --create-identity" >&2
  exit 1
fi

for BIN in evo-daemon evo-ledger-engine evo-desktop; do
  for PROFILE in release debug; do
    PATH_TO_BIN="$TARGET_DIR/$PROFILE/$BIN"
    if [ -x "$PATH_TO_BIN" ]; then
      codesign --force --sign "$IDENTITY" \
        --identifier "dev.evo.$BIN" "$PATH_TO_BIN"
      echo "signed: $PROFILE/$BIN as dev.evo.$BIN"
    fi
  done
done

echo "TCC grants for dev.evo.* now survive rebuilds."
