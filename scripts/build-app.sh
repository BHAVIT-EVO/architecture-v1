#!/usr/bin/env bash
# Builds a real macOS .app bundle for Evo — all three processes.
#
# Usage:
#   bash scripts/build-app.sh                # release profile (default)
#   PROFILE=dev bash scripts/build-app.sh    # debug profile
#   EVO_ORT_PROVISIONING=external ORT_LIB_PATH=/path/to/ort \
#     ORT_PREFER_DYNAMIC_LINK=1 bash scripts/build-app.sh
#   EVO_ORT_PROVISIONING=bundled EVO_ORT_DYLIB=/exact/local/runtime.dylib \
#     EVO_ORT_LICENSE=/exact/local/LICENSE \
#     EVO_ORT_THIRD_PARTY_NOTICES=/exact/local/ThirdPartyNotices.txt \
#     EVO_APP_OUTPUT_DIR=/fresh/local/output bash scripts/build-app.sh
#   EVO_APP_OUTPUT_DIR=/private/var/tmp/evo-dist bash scripts/build-app.sh
#
# Bundled mode is a narrow arm64 ONNX Runtime 1.29.0 local packaging contract.
# Supplied notices are copied, not interpreted as redistribution/license approval.
#
# Produces: /Applications/Evo.app by default (override with
# EVO_APP_OUTPUT_DIR for an explicitly isolated build) with:
#   Contents/MacOS/Evo               — the desktop shell (renamed evo-desktop)
#   Contents/MacOS/evo-daemon        — the capture daemon
#   Contents/MacOS/evo-ledger-engine — the works engine
#   Contents/MacOS/evo-agent         — the headless voice agent
#   Contents/MacOS/evo-agent-install — its LaunchAgent installer
#   Contents/Resources/AppIcon.icns  — placeholder icon
#   Contents/Info.plist
#
# Capture survives reboots via the LaunchAgents that scripts/install-agents.sh
# installs (launchd owns the daemon and the engine; the shell is their UI).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="Evo"
PROFILE="${PROFILE:-release}"
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
if [ "$PROFILE" = "dev" ]; then
  OUT_DIR="debug"
elif [ "$PROFILE" = "release" ]; then
  OUT_DIR="release"
else
  OUT_DIR="$PROFILE"
fi
APP_OUTPUT_DIR="${EVO_APP_OUTPUT_DIR:-/Applications}"
APP_DIR="$APP_OUTPUT_DIR/$APP_NAME.app"
IDENTITY="${EVO_CODESIGN_IDENTITY:-Evo Code Signing}"
ORT_PROVISIONING="${EVO_ORT_PROVISIONING-none}"
ORT_LIB_DIR="${ORT_LIB_PATH-${ORT_LIB_LOCATION-}}"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "error: scripts/build-app.sh only produces macOS bundles (Darwin required)." >&2
  exit 1
fi

find_ort_library() {
  local directory="$1"
  shift
  local suffix candidate
  for suffix in "$@"; do
    for candidate in "$directory"/libonnxruntime*"$suffix" "$directory"/onnxruntime*"$suffix"; do
      if [ -f "$candidate" ] && [ -s "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done
  done
  return 1
}

validate_ort_provisioning() {
  if [ -n "$ORT_LIB_DIR" ] && [ "${ORT_LIB_DIR#/}" = "$ORT_LIB_DIR" ]; then
    ORT_LIB_DIR="$REPO_ROOT/$ORT_LIB_DIR"
  fi
  case "$ORT_PROVISIONING" in
    none)
      # The default passive bundle does not compile embeddings or voice, so
      # it has no ONNX Runtime dependency and should not require a local model
      # runtime merely to build focused-window capture.
      return 0
      ;;
    external) ;;
    bundled)
      # Refuse to overwrite/sign an existing bundle in this opt-in path.
      if [ -e "$APP_DIR" ] || [ -L "$APP_DIR" ]; then
        echo "error: bundled mode requires a fresh output app path; refusing to replace $APP_DIR." >&2
        return 1
      fi
      python3 "$REPO_ROOT/scripts/evo-doctor.py" --workspace "$REPO_ROOT" --check-ort-inputs
      local input_path
      for input_path in EVO_ORT_DYLIB EVO_ORT_LICENSE EVO_ORT_THIRD_PARTY_NOTICES; do
        if [ "${!input_path#/}" = "${!input_path}" ]; then
          printf -v "$input_path" '%s' "$REPO_ROOT/${!input_path}"
        fi
      done
      # Snapshot exactly the supplied files before Cargo. Link and copy from
      # this same local snapshot; never glob/discover a substitute wheel.
      mkdir -p "$TARGET_DIR"
      ORT_BUNDLE_STAGE="$(mktemp -d "$TARGET_DIR/ort-bundled-link.XXXXXX")"
      cp -X "$EVO_ORT_DYLIB" "$ORT_BUNDLE_STAGE/libonnxruntime.1.dylib"
      cp -X "$EVO_ORT_LICENSE" "$ORT_BUNDLE_STAGE/LICENSE"
      cp -X "$EVO_ORT_THIRD_PARTY_NOTICES" "$ORT_BUNDLE_STAGE/ThirdPartyNotices.txt"
      ln -s libonnxruntime.1.dylib "$ORT_BUNDLE_STAGE/libonnxruntime.dylib"
      EVO_ORT_DYLIB="$ORT_BUNDLE_STAGE/libonnxruntime.1.dylib" \
        EVO_ORT_LICENSE="$ORT_BUNDLE_STAGE/LICENSE" \
        EVO_ORT_THIRD_PARTY_NOTICES="$ORT_BUNDLE_STAGE/ThirdPartyNotices.txt" \
        python3 "$REPO_ROOT/scripts/evo-doctor.py" --workspace "$REPO_ROOT" --check-ort-inputs
      ORT_LIB_DIR="$ORT_BUNDLE_STAGE"
      export ORT_LIB_PATH="$ORT_LIB_DIR" ORT_PREFER_DYNAMIC_LINK=1 ORT_SKIP_DOWNLOAD=1
      echo "ONNX Runtime: explicit local bundled snapshot at $ORT_BUNDLE_STAGE; supplied notices only."
      return 0
      ;;
    *)
      echo "error: ONNX Runtime provisioning is not explicit; set EVO_ORT_PROVISIONING." >&2
      echo "       external = link an existing reviewed runtime (the app is not portable)." >&2
      echo "       bundled = narrow reviewed local dylib plus exact supplied license/notices paths." >&2
      return 1
      ;;
  esac

  if [ -z "$ORT_LIB_DIR" ] || [ ! -d "$ORT_LIB_DIR" ]; then
    echo "error: ORT_LIB_PATH (or ORT_LIB_LOCATION) must name an existing ONNX Runtime directory." >&2
    return 1
  fi
  export ORT_LIB_PATH="$ORT_LIB_DIR"

  case "${ORT_PREFER_DYNAMIC_LINK:-}" in
    1|[Tt][Rr][Uu][Ee]) ORT_DYNAMIC=1 ;;
    *) ORT_DYNAMIC=0 ;;
  esac
  if [ "$ORT_DYNAMIC" = "1" ]; then
    local ort_library
    ort_library="$(find_ort_library "$ORT_LIB_DIR" .dylib || true)"
    if [ -z "$ort_library" ]; then
      echo "error: dynamic ONNX Runtime linking was requested but no nonempty .dylib was found in $ORT_LIB_DIR." >&2
      return 1
    fi
    # ort-sys asks the linker for -lonnxruntime and the runtime loader for
    # libonnxruntime.1.dylib. Python wheels commonly ship only a fully
    # versioned filename. Keep the reviewed source untouched and stage local
    # aliases under target/ so the build is reproducible and does not mutate
    # a user's external runtime directory.
    if [ "$(uname -s)" = "Darwin" ]; then
      local ort_link_dir="$TARGET_DIR/ort-link"
      mkdir -p "$ort_link_dir"
      ln -sfn "$ort_library" "$ort_link_dir/libonnxruntime.dylib"
      ln -sfn "$ort_library" "$ort_link_dir/libonnxruntime.1.dylib"
      ORT_LIB_DIR="$ort_link_dir"
      export ORT_LIB_PATH="$ORT_LIB_DIR"
    fi
  elif ! find_ort_library "$ORT_LIB_DIR" .a >/dev/null; then
    echo "error: static ONNX Runtime linking was selected but no nonempty .a was found in $ORT_LIB_DIR." >&2
    echo "       Set ORT_PREFER_DYNAMIC_LINK=1 for a reviewed dylib, or provision a static archive." >&2
    return 1
  fi

  echo "ONNX Runtime: explicit development link from $ORT_LIB_DIR; portable release remains unverified."
}

validate_ort_provisioning

can_sign_with_identity() {
  if [ "$IDENTITY" = "-" ]; then
    return 1
  fi
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

# Accessibility is granted to a code requirement, not merely to the app's
# display name. An ad-hoc signature makes that requirement the binary's
# cdhash, which changes on every build while System Settings can still show
# the old Evo entry as enabled. Refuse to produce a bundle in that state.
if ! can_sign_with_identity; then
  echo "error: a stable code-signing identity is required for Evo's macOS permissions." >&2
  echo "       Create the local development identity once with:" >&2
  echo "       bash scripts/sign-dev.sh --create-identity" >&2
  echo "       Then rerun this build. Override the identity with EVO_CODESIGN_IDENTITY." >&2
  exit 1
fi

# ── Voice, and its microphone, are opt-in — and off by default ──────────────
#
# Evo's default experience is passive observation: it reads focused-window
# titles, file events and URLs. It never sees the screen, and it never listens.
# The microphone exists for exactly one feature — "Hey Evo" — so it is compiled
# in and *declared to macOS* only when that feature is explicitly requested:
#
#   EVO_VOICE=on bash scripts/build-app.sh
#
# The default keeps the smallest honest permission surface: Accessibility, and
# nothing else. That is the promise the app makes on its permission screen, and
# a bundle that asks for a microphone it does not need breaks it.
EVO_VOICE="${EVO_VOICE:-off}"
EVO_NOTCH="${EVO_NOTCH:-on}"
case "$EVO_VOICE" in
  on)
    # The agent's wake/transcription path needs the audio + notch features,
    # and the daemon must also compile semantic embeddings or every accepted
    # utterance fails with "voice model unavailable" after transcription.
    EVO_CARGO_FEATURES="voice-wake,evo-notch,embeddings"
    ;;
  off)
    EVO_CARGO_FEATURES=""
    ;;
  *)
    echo "error: EVO_VOICE must be 'on' or 'off' (got '$EVO_VOICE')" >&2
    exit 1
    ;;
  esac

if [ "$EVO_VOICE" = "off" ]; then
  case "$EVO_NOTCH" in
    on) EVO_CARGO_FEATURES="evo-notch" ;;
    off) EVO_CARGO_FEATURES="" ;;
    *) echo "error: EVO_NOTCH must be 'on' or 'off' (got '$EVO_NOTCH')." >&2; exit 1 ;;
  esac
fi

echo "Building Evo ($PROFILE, voice=$EVO_VOICE, notch=$EVO_NOTCH, features=${EVO_CARGO_FEATURES:-none})..."
if [ -n "$EVO_CARGO_FEATURES" ]; then
  (cd "$REPO_ROOT" && CARGO_NET_OFFLINE=true cargo build --offline --locked \
    -p evo-desktop --features "$EVO_CARGO_FEATURES" --bins \
    -p evo-ledger --profile "$PROFILE")
  if [ "$EVO_VOICE" = "on" ]; then
    # `evo-desktop`'s `embeddings` feature covers its daemon dependency, but
    # the separately selected daemon executable needs the feature explicitly.
    (cd "$REPO_ROOT" && CARGO_NET_OFFLINE=true cargo build --offline --locked \
      -p evo-daemon --features embeddings --profile "$PROFILE")
  fi
else
  (cd "$REPO_ROOT" && CARGO_NET_OFFLINE=true cargo build --offline --locked \
    -p evo-desktop --bins -p evo-daemon -p evo-ledger --profile "$PROFILE")
fi

if [ "$ORT_PROVISIONING" = "bundled" ]; then
  mkdir -p "$APP_OUTPUT_DIR"
  # Reserve atomically after Cargo too: never merge into an app that appeared
  # while compilation was running, even if the initial preflight was fresh.
  if ! mkdir "$APP_DIR"; then
    echo "error: could not reserve fresh bundled app path; refusing to overwrite/sign $APP_DIR." >&2
    exit 1
  fi
fi
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp -X "$TARGET_DIR/$OUT_DIR/evo-desktop"       "$APP_DIR/Contents/MacOS/$APP_NAME"
cp -X "$TARGET_DIR/$OUT_DIR/evo-daemon"        "$APP_DIR/Contents/MacOS/evo-daemon"
cp -X "$TARGET_DIR/$OUT_DIR/evo-ledger-engine" "$APP_DIR/Contents/MacOS/evo-ledger-engine"
# The voice agent and installer are only shipped when voice does: the agent's
# Cargo bin declares required-features = ["voice-wake", "evo-notch"], while
# the installer is kept alongside it only to avoid advertising an unused
# microphone-capable runtime in an EVO_VOICE=off bundle.
if [ "$EVO_VOICE" = "on" ]; then
  cp -X "$TARGET_DIR/$OUT_DIR/evo-agent"         "$APP_DIR/Contents/MacOS/evo-agent"
  cp -X "$TARGET_DIR/$OUT_DIR/evo-agent-install" "$APP_DIR/Contents/MacOS/evo-agent-install"
fi
chmod +x "$APP_DIR/Contents/MacOS/"*

cat > "$APP_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>Evo</string>
  <key>CFBundleDisplayName</key>
  <string>Evo</string>
  <key>CFBundleIdentifier</key>
  <string>dev.evo.desktop</string>
  <key>CFBundleExecutable</key>
  <string>Evo</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.productivity</string>
</dict>
</plist>
PLIST

# The microphone declaration is added only when the microphone is compiled in.
# The string above is deleted from the base bundle on purpose: a plist key is a
# statement to macOS and to the person reading the app's permissions, so it may
# not claim a capability the bundle does not use.
if [ "$EVO_VOICE" = "on" ]; then
  plutil -insert NSMicrophoneUsageDescription -string \
    'Evo listens locally for the “Hey Evo” wake word and spoken commands. Audio is processed on this Mac and never leaves it.' \
    "$APP_DIR/Contents/Info.plist"
fi

# Voice assets are runtime inputs, not Cargo artifacts, and they are only
# needed when the voice feature is on. Fail the bundle build rather than
# shipping a feature-enabled app that exits its voice thread.
if [ "$EVO_VOICE" = "on" ]; then
  VOICE_MODELS=(whisper-base.en.bin melspectrogram.onnx embedding_model.onnx hey_evoh.onnx)
  for model in "${VOICE_MODELS[@]}"; do
    source="$TARGET_DIR/$OUT_DIR/$model"
    if [ ! -s "$source" ]; then
      echo "error: missing voice model $source" >&2
      echo "       provision the local model assets before building Evo.app." >&2
      exit 1
    fi
  done
  mkdir -p "$APP_DIR/Contents/Resources"
  for model in "${VOICE_MODELS[@]}"; do
    install -m 644 "$TARGET_DIR/$OUT_DIR/$model" "$APP_DIR/Contents/Resources/$model"
  done
fi

# The headless voice agent also needs the local semantic model when it is
# launched by launchd (whose working directory is not the Cargo target
# directory). Passive builds do not compile that agent and must not require or
# ship model assets from a previous voice build.
if [ "$EVO_VOICE" = "on" ]; then
  for model in model.onnx tokenizer.json config.json special_tokens_map.json tokenizer_config.json; do
    source="$TARGET_DIR/model-metadata/$model"
    if [ ! -s "$source" ]; then
      echo "error: missing embedding asset $source" >&2
      exit 1
    fi
    install -m 644 "$source" "$APP_DIR/Contents/Resources/$model"
  done
fi

# Placeholder icon (solid-color, pure-Python PNG writer) until a real one is
# designed: some macOS surfaces expect an icon in the bundle.
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
for SIZE in 16 32 128 256; do
  python3 - "$SIZE" "$ICONSET" <<'PYEOF'
import struct, zlib, sys
size = int(sys.argv[1]); out = sys.argv[2]
def chunk(typ, data):
    return struct.pack('>I', len(data)) + typ + data + struct.pack('>I', zlib.crc32(typ + data))
raw = b''.join(b'\x00' + b'\x1a\x2a\x3c\xff' * size for _ in range(size))
png = (b'\x89PNG\r\n\x1a\n'
       + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0))
       + chunk(b'IDAT', zlib.compress(raw))
       + chunk(b'IEND', b''))
open(f'{out}/icon_{size}x{size}.png', 'wb').write(png)
open(f'{out}/icon_{size * 2}x{size * 2}.png', 'wb').write(png)  # HiDPI pair
PYEOF
done
iconutil -c icns "$ICONSET" -o "$APP_DIR/Contents/Resources/AppIcon.icns" >/dev/null 2>&1 \
  || echo "note: iconutil unavailable; bundle ships without an icon"
plutil -replace CFBundleIconFile -string "AppIcon" "$APP_DIR/Contents/Info.plist" >/dev/null 2>&1 || true

if [ "$ORT_PROVISIONING" = "bundled" ] || [ "$ORT_PROVISIONING" = "external" ]; then
  mkdir -p "$APP_DIR/Contents/Frameworks" "$APP_DIR/Contents/Resources/onnxruntime"
  if [ "$ORT_PROVISIONING" = "bundled" ]; then
    runtime_source="$ORT_BUNDLE_STAGE/libonnxruntime.1.dylib"
    cp -X "$ORT_BUNDLE_STAGE/LICENSE" "$APP_DIR/Contents/Resources/onnxruntime/LICENSE"
    cp -X "$ORT_BUNDLE_STAGE/ThirdPartyNotices.txt" "$APP_DIR/Contents/Resources/onnxruntime/ThirdPartyNotices.txt"
    cmp "$ORT_BUNDLE_STAGE/LICENSE" "$APP_DIR/Contents/Resources/onnxruntime/LICENSE"
    cmp "$ORT_BUNDLE_STAGE/ThirdPartyNotices.txt" "$APP_DIR/Contents/Resources/onnxruntime/ThirdPartyNotices.txt"
    # The reviewed wheel's Mach-O minimum is macOS 14, not external mode's 13.
    plutil -replace LSMinimumSystemVersion -string "14.0" "$APP_DIR/Contents/Info.plist"
  else
    # External development linking still needs a runnable app bundle. Copy the
    # explicitly selected dylib into this bundle and rewrite the binaries to
    # load it from Contents/Frameworks instead of depending on a shell-only
    # DYLD_LIBRARY_PATH.
    runtime_source="$ORT_LIB_DIR/libonnxruntime.1.dylib"
  fi
  cp -X "$runtime_source" "$APP_DIR/Contents/Frameworks/libonnxruntime.1.dylib"
  for binary in Evo evo-agent evo-daemon evo-ledger-engine; do
    binary_path="$APP_DIR/Contents/MacOS/$binary"
    # evo-agent is absent in an EVO_VOICE=off bundle; there is nothing to
    # rewrite and it is not an error.
    [ -f "$binary_path" ] || continue
    # Strip development/Cargo rpaths so the exact bundled runtime wins without
    # DYLD_LIBRARY_PATH. All edits happen on copies, before any bundle signing.
    old_rpaths="$(otool -l "$binary_path" | awk '
      /cmd LC_RPATH/ { found=1; next }
      found && /path .* \(offset / { sub(/^[[:space:]]*path /, ""); sub(/ \(offset .*$/, ""); print; found=0 }
    ')"
    while IFS= read -r old_rpath; do
      if [ -n "$old_rpath" ]; then
        install_name_tool -delete_rpath "$old_rpath" "$binary_path"
      fi
    done <<< "$old_rpaths"
    install_name_tool -add_rpath '@executable_path/../Frameworks' "$binary_path"
  done
  if [ "$ORT_PROVISIONING" = "bundled" ]; then
    python3 "$REPO_ROOT/scripts/evo-doctor.py" --check-ort-bundle "$APP_DIR"
  fi
fi

# Finder metadata on a copied build artifact invalidates the sealed bundle.
# It is not application data, so clear it before the signature is created.
xattr -cr "$APP_DIR"

# Sign each executable explicitly before signing the outer bundle. This keeps
# their requirements stable and gives the desktop and capture helper the
# identifiers users see in System Settings.
echo "Signing with $IDENTITY (stable TCC grants)..."
if [ "$ORT_PROVISIONING" = "bundled" ] || [ "$ORT_PROVISIONING" = "external" ]; then
  codesign --force --sign "$IDENTITY" --identifier dev.evo.onnxruntime "$APP_DIR/Contents/Frameworks/libonnxruntime.1.dylib"
  codesign --verify --strict "$APP_DIR/Contents/Frameworks/libonnxruntime.1.dylib"
fi
codesign --force --sign "$IDENTITY" --identifier dev.evo.daemon "$APP_DIR/Contents/MacOS/evo-daemon"
codesign --force --sign "$IDENTITY" --identifier dev.evo.ledger "$APP_DIR/Contents/MacOS/evo-ledger-engine"
for optional_binary in evo-agent evo-agent-install; do
  optional_path="$APP_DIR/Contents/MacOS/$optional_binary"
  if [ -f "$optional_path" ]; then
    if [ "$optional_binary" = "evo-agent" ]; then
      optional_identifier="dev.evo.agent"
    else
      optional_identifier="dev.evo.agent-install"
    fi
    codesign --force --sign "$IDENTITY" --identifier "$optional_identifier" "$optional_path"
  fi
done
# Signing nested code can make Finder recreate metadata on the enclosing
# directory. Remove it again before sealing the desktop executable and bundle.
xattr -cr "$APP_DIR"
codesign --force --sign "$IDENTITY" --identifier dev.evo.desktop "$APP_DIR/Contents/MacOS/Evo"
xattr -cr "$APP_DIR"
# Finder can restore its metadata immediately after a failed first seal attempt
# on a Desktop-backed directory. A clean retry produces the same signature.
if ! codesign --force --sign "$IDENTITY" --identifier dev.evo.desktop "$APP_DIR"; then
  xattr -cr "$APP_DIR"
  codesign --force --sign "$IDENTITY" --identifier dev.evo.desktop "$APP_DIR"
fi
# Finder can also re-apply metadata AFTER a successful seal. The code seal
# does not cover extended attributes, so clearing detritus now keeps the
# signature valid — a sealed bundle left on a Desktop-backed directory fails
# verification (and launchd's signature check) without this pass.
for _ in 1 2 3; do
  if codesign --verify --deep --strict "$APP_DIR" 2>/dev/null; then
    break
  fi
  xattr -cr "$APP_DIR"
done
codesign --verify --deep --strict "$APP_DIR"
if [ "$ORT_PROVISIONING" = "bundled" ]; then
  python3 "$REPO_ROOT/scripts/evo-doctor.py" --check-ort-bundle "$APP_DIR"
fi

echo "Built: $APP_DIR"
if [ "$ORT_PROVISIONING" = "bundled" ]; then
  echo "Runtime note: local bundled closure/signature checks passed; license approval, isolated loading/API negotiation and release readiness remain separate."
elif [ "$ORT_PROVISIONING" = "none" ]; then
  echo "Runtime note: passive bundle has no ONNX Runtime dependency; voice and semantic embeddings are disabled."
else
  echo "Runtime note: explicitly provisioned development link; portable release is not verified."
fi
echo "Launch with: open \"$APP_DIR\""
echo "Install always-on capture with: bash scripts/install-agents.sh"
