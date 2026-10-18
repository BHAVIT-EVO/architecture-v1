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
# Explicitly re-sign one already staged bundled app (never implicit dist/):
#   EVO_ORT_PROVISIONING=bundled EVO_ORT_DYLIB=/exact/runtime.dylib \
#     EVO_ORT_LICENSE=/exact/LICENSE EVO_ORT_THIRD_PARTY_NOTICES=/exact/notices \
#     bash scripts/sign-dev.sh --app /exact/local/Evo.app
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
IDENTITY="${EVO_CODESIGN_IDENTITY:-Evo Code Signing}"

usage() {
  echo "usage: EVO_CODESIGN_IDENTITY='name' bash $0 [--create-identity | --app /exact/Evo.app]" >&2
}

if [ "$IDENTITY" = "-" ]; then
  echo "error: ad-hoc signing is not a stable Evo identity; nothing signed." >&2
  exit 1
fi

can_sign_with_identity() {
  local probe
  probe="$(mktemp "${TMPDIR:-/tmp}/evo-codesign-probe.XXXXXX")"
  printf '#!/bin/sh\nexit 0\n' > "$probe"
  chmod +x "$probe"
  if codesign --force --sign "$IDENTITY" --identifier dev.evo.signing-probe "$probe" >/dev/null 2>&1; then
    rm -f "$probe"
    return 0
  fi
  rm -f "$probe"
  return 1
}

if [ "${1:-}" = "--create-identity" ]; then
  if [ "$#" -gt 1 ]; then
    usage
    exit 2
  fi
  if can_sign_with_identity; then
    echo "identity '$IDENTITY' already exists"
    exit 0
  fi
  echo "creating '$IDENTITY' via Keychain Access Certificate Assistant:"
  echo "  1. Keychain Access → Certificate Assistant → Create Certificate…"
  echo "  2. Name: $IDENTITY, Identity Type: Self Signed Root, Type: Code Signing"
  echo "  3. Let me override defaults → keep defaults → Sign with SHA-2 (256)"
  echo "  4. Done. Then re-run: bash $0"
  open -a "Keychain Access" 2>/dev/null || true
  exit 0
fi

if [ "${1:-}" = "--app" ]; then
  if [ "$#" -ne 2 ]; then
    usage
    exit 2
  fi
  if [ "$(uname -s)" != "Darwin" ]; then
    echo "error: app signing requires macOS; nothing signed." >&2
    exit 1
  fi
  # The opt-in app path is deliberately bundled-only. Default invocation still
  # signs only target/{release,debug}, and never locates/signs dist implicitly.
  python3 "$REPO_ROOT/scripts/evo-doctor.py" --workspace "$REPO_ROOT" --check-ort-inputs
  APP_DIR="$2"
  python3 "$REPO_ROOT/scripts/evo-doctor.py" --check-ort-bundle "$APP_DIR"
  for input_name in EVO_ORT_LICENSE EVO_ORT_THIRD_PARTY_NOTICES; do
    input_path="${!input_name}"
    if [ "${input_path#/}" = "$input_path" ]; then
      input_path="$REPO_ROOT/$input_path"
    fi
    case "$input_name" in
      EVO_ORT_LICENSE) notice_name=LICENSE ;;
      EVO_ORT_THIRD_PARTY_NOTICES) notice_name=ThirdPartyNotices.txt ;;
    esac
    cmp "$input_path" "$APP_DIR/Contents/Resources/onnxruntime/$notice_name"
  done
  if ! can_sign_with_identity; then
    echo "no '$IDENTITY' identity; app not signed." >&2
    exit 1
  fi
  xattr -cr "$APP_DIR"
  codesign --force --sign "$IDENTITY" --identifier dev.evo.onnxruntime "$APP_DIR/Contents/Frameworks/libonnxruntime.1.dylib"
  codesign --verify --strict "$APP_DIR/Contents/Frameworks/libonnxruntime.1.dylib"
  for binary in evo-daemon evo-ledger-engine Evo; do
    case "$binary" in
      evo-daemon) signing_id=dev.evo.daemon ;;
      evo-ledger-engine) signing_id=dev.evo.ledger ;;
      Evo) signing_id=dev.evo.desktop ;;
    esac
    xattr -cr "$APP_DIR"
    codesign --force --sign "$IDENTITY" --identifier "$signing_id" "$APP_DIR/Contents/MacOS/$binary"
    codesign --verify --strict "$APP_DIR/Contents/MacOS/$binary"
  done
  xattr -cr "$APP_DIR"
  codesign --force --sign "$IDENTITY" --identifier dev.evo.desktop "$APP_DIR"
  codesign --verify --deep --strict "$APP_DIR"
  python3 "$REPO_ROOT/scripts/evo-doctor.py" --check-ort-bundle "$APP_DIR"
  echo "signed explicit bundled app: $APP_DIR (local signature, not notarization or license approval)"
  exit 0
fi

if [ "$#" -gt 0 ]; then
  usage
  exit 2
fi

if ! can_sign_with_identity; then
  echo "no '$IDENTITY' identity; nothing signed." >&2
  echo "create one once with: bash $0 --create-identity" >&2
  exit 1
fi

for BIN in evo-daemon evo-ledger-engine evo-desktop; do
  case "$BIN" in
    evo-desktop) SIGNING_ID="dev.evo.desktop" ;;
    evo-daemon) SIGNING_ID="dev.evo.daemon" ;;
    evo-ledger-engine) SIGNING_ID="dev.evo.ledger" ;;
    *) echo "unsupported binary: $BIN" >&2; exit 1 ;;
  esac
  for PROFILE in release debug; do
    PATH_TO_BIN="$TARGET_DIR/$PROFILE/$BIN"
    if [ -x "$PATH_TO_BIN" ]; then
      codesign --force --sign "$IDENTITY" \
        --identifier "$SIGNING_ID" "$PATH_TO_BIN"
      codesign --verify --strict "$PATH_TO_BIN"
      echo "signed: $PROFILE/$BIN as $SIGNING_ID"
    fi
  done
done

echo "TCC grants for dev.evo.* now survive rebuilds."
