# Brand audit

## Found and replaced
- `crates/evo-desktop/ui/app/home.html`: sprout glyph in top bar → `evo-mark.svg` (28px).
- `site/assets/favicon.svg`: sprout favicon → `evo-mark.svg`.
- `site/assets/og.svg` (old) → `og-image.png` generated from lockup.
- `scripts/build-app.sh`: placeholder Python PNG icon → rendered `evo-icon.svg` iconset + `.icns`.

## Remaining default framework icons
- None found after grep for `sprout` / default Tauri / Rust icons.

## Not checked
- Notification and Spotlight icons require confirmation on the user's Mac.
- DMG background not produced by this build (no DMG step in build-app.sh).
