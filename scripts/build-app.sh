#!/usr/bin/env bash
# Builds a real macOS .app bundle for Evo — all three processes.
#
# Usage:
#   bash scripts/build-app.sh                # release profile (default)
#   PROFILE=dev bash scripts/build-app.sh    # debug profile
#
# Produces: dist/Evo.app with:
#   Contents/MacOS/Evo               — the desktop shell (renamed evo-desktop)
#   Contents/MacOS/evo-daemon        — the capture daemon
#   Contents/MacOS/evo-ledger-engine — the works engine
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
APP_DIR="$REPO_ROOT/dist/$APP_NAME.app"

echo "Building all three processes ($PROFILE)..."
(cd "$REPO_ROOT" && cargo build -p evo-desktop -p evo-daemon -p evo-ledger --profile "$PROFILE")

mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp "$TARGET_DIR/$OUT_DIR/evo-desktop"       "$APP_DIR/Contents/MacOS/$APP_NAME"
cp "$TARGET_DIR/$OUT_DIR/evo-daemon"        "$APP_DIR/Contents/MacOS/evo-daemon"
cp "$TARGET_DIR/$OUT_DIR/evo-ledger-engine" "$APP_DIR/Contents/MacOS/evo-ledger-engine"
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

# Signing: with a self-signed "Evo Code Signing" identity (see
# scripts/sign-dev.sh) the app's TCC grants survive rebuilds; ad-hoc
# signatures are cdhash-pinned and silently invalidate grants every rebuild.
if security find-identity -q -c "Evo Code Signing" 2>/dev/null | grep -q "Evo Code Signing"; then
  IDENTITY="Evo Code Signing"
  echo "Signing with $IDENTITY (stable TCC grants)..."
  codesign --force --sign "$IDENTITY" --identifier dev.evo.daemon "$APP_DIR/Contents/MacOS/evo-daemon"
  codesign --force --sign "$IDENTITY" --identifier dev.evo.ledger "$APP_DIR/Contents/MacOS/evo-ledger-engine"
  codesign --force --sign "$IDENTITY" --identifier dev.evo.desktop "$APP_DIR"
else
  echo "note: no 'Evo Code Signing' identity; using ad-hoc (TCC grants reset per rebuild)."
  echo "      create one once with: bash scripts/sign-dev.sh --create-identity"
  codesign --force --deep --sign - "$APP_DIR" >/dev/null 2>&1 || true
fi

echo "Built: $APP_DIR"
echo "Launch with: open \"$APP_DIR\""
echo "Install always-on capture with: bash scripts/install-agents.sh"
