# AUDIT-4

## Context block
Same as AUDIT-1.

## Verdict
NOT READY for data/privacy claims. Settings are largely localStorage-backed (not Rust-enforced), and privacy/scope enforcement could not be proven. First-run and state scenarios not exercised.

## Top problems
1. Settings values mostly persisted in localStorage, not the Rust store.
2. Privacy scopes/actions/modes enforcement not verified.
3. Delete everything not tested (only throwaway allowed; mechanism unknown).
4. No automated state-trigger tests.

## Findings
F-ONBOARD-01 [COULD NOT TEST] First-run flow
Reason: not exercised.

F-SETTINGS-01 [P1] Settings persisted in localStorage
Symptom: readSettings/saveSettings use localStorage; Rust-backed only for some tabs.
Evidence: settings.js lines ~24-30.
Confidence: CONFIRMED.

F-PRIVACY-01 [COULD NOT TEST] Scope enforcement
Reason: not exercised.

F-DATA-01 [P2] Data dir location
Evidence: ~/Library/Application Support/Evo/storage, mode 755; logs 644.
Confidence: CONFIRMED.

F-STATES-01 [COULD NOT TEST] Empty/loading/error states
Reason: not triggered.

F-HYGIENE-01 [P2] TODO items
Evidence: crates/evo-desktop/TODO.md has 4 open items.
Confidence: CONFIRMED.

## Could not test
- First run, onboarding, permissions, replay.
- All settings controls persistence/enforcement.
- Privacy scopes, Activity, Undo, Pause, kill switch, Export, Delete.
- Data integrity (migration, corruption, disk full, dual daemon, crash recovery).
- States and dead code/CI.