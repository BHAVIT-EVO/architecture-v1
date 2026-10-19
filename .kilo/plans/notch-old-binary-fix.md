# Fix for Notch Still Showing Old Appearance

## Root Cause
The user is running an **old installed binary**. The `evo-agent-install` installs the agent to `~/Library/Application Support/Evo/agent/` and registers a LaunchAgent. Rebuilding without reinstalling leaves the old binary running.

**Evidence:** Red/orangish mic ring, centered notch, blurred "listening" = OLD code (before my changes).

## Solution
Rebuild with features AND reinstall the agent:

```bash
# 1. Build the agent with required features (release mode)
cd /Users/bhavitsaini/Desktop/Evo-Evolution.nosync
cargo build --release -p evo-desktop --features "voice-wake,evo-notch" --bin evo-agent

# 2. Build the daemon (required by installer)
cargo build --release -p evo-daemon --bin evo-daemon

# 3. Reinstall the agent (copies new binaries, updates LaunchAgent)
cargo run --release --bin evo-agent-install
# OR run the built installer directly:
# ./target/release/evo-agent-install

# 4. Restart the agent (LaunchAgent will auto-restart, or manually):
launchctl bootout gui/$(id -u) com.evo.agent 2>/dev/null; launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.evo.agent.plist
```

## Verification
After reinstall:
- Mic ring should be **caramel** (not red)
- Notch expands from **hardware notch position** (top center of screen, not screen center)
- Fonts should be **DESIGN.md sizes** (not blurred/hardcoded)
- "LISTENING" text in **teal voice** (#5B8A8F) at 10.5pt semibold

## Debugging if Still Broken
Check the agent log:
```bash
tail -f ~/Library/Logs/evo-agent.log
```

Look for:
- `EVO notch: hardware notch WxHpt acquired` — confirms probe works
- `EVO-AGENT: notch decision: Own` — confirms notch surface started

If probe fails (returns None), check:
- MacBook Air M2 has a notch (yes)
- `auxiliaryTopLeftArea`/`RightArea` return correct values
- Screen is the built-in display (not external)

## Code Changes Already Applied
All notch redesign changes are in `crates/evo-desktop/src/notch_window.rs`:
- ✅ Hardware notch anchoring via `geometry.canvas_notch_x()`
- ✅ DESIGN.md typography (7 font roles)
- ✅ DESIGN.md color tokens (INK_PRIMARY, TEAL_VOICE, CARAMEL, etc.)
- ✅ Spring animations (response 0.42, damping 0.8)
- ✅ Chip hover states with NSTrackingArea
- ✅ CoreText glyph rasterization
- ✅ All 199 tests pass