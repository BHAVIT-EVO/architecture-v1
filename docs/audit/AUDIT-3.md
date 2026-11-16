# AUDIT-3

## Context block
Same as AUDIT-1.

## Verdict
NOT READY for reopen/voice verification. Continue/reopen paths and voice privacy claims could not be exercised at runtime; code-level evidence shows an always-on launchd agent and bundled local models (Whisper, ONNX), consistent with "Stays on this Mac" but not proven by capture.

## Top problems
1. No runtime test of Continue across work kinds (not exercised).
2. Voice privacy not proven by packet capture (lsof/nettop not run).
3. Wake latency and notch rect not measured.
4. Tray/menu behavior with main window closed not exercised.

## Findings
F-REOPEN-01 [COULD NOT TEST] Continue/reopen all work kinds
Reason: native app interaction not automated; no test harness.
Confidence: N/A.

F-NOTCH-01 [COULD NOT TEST] Notch rect/states/latency
Reason: requires visual/runtime measurement; hardware notch present (Mac14,2).
Confidence: N/A.

F-VOICE-01 [SUSPECTED] "Stays on this Mac" plausible
Evidence: bundled models are local (whisper-base.en.bin, *.onnx); no remote URLs in bundle UI; connect-src 'none' in CSP.
Confidence: SUSPECTED (packet capture not performed).

F-VOICE-02 [CONFIRMED] evo-agent launchd running
Evidence: launchctl print gui/501/com.evo.agent state=running.
Confidence: CONFIRMED.

F-TRAY-01 [COULD NOT TEST] Tray/menu/Dock behavior
Reason: not exercised.
Confidence: N/A.

## Could not test
- All reopen scenarios and failure cases.
- Notch states, hotkey, mic denied/device change, false wake, noisy room, external display, notch-less Mac.
- Pause/kill switch proof.
- Tray menu with main window closed, launch at login, quit behavior.