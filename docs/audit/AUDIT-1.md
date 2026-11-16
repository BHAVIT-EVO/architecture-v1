# AUDIT-1

## 1. Context block
- Git commit: 49fae2a25c27876829327b46bcc7615d5f14f3f0
- macOS: 26.5.2 (25F84)
- Mac model: Mac14,2 (notch present)
- Display: built-in Liquid Retina, 2560x1664
- Build used: /Applications/Evo.app, signed 15 Nov 2026 11:41, version 0.1.0 build 1, EVO_VOICE=on, EVO_ORT_PROVISIONING=external
- App launched: binary directly and via open; data dir ~/Library/Application Support/Evo/storage (EVO_STORAGE_ROOT throwaway dir remained empty in test)
- Auditor: read-only, no product code changes

## 2. Verdict
NOT READY. Build does not pass clippy with -D warnings (10 errors), cargo audit reports 1 vulnerability, and Gatekeeper rejects the ad-hoc signed bundle. Core flows appear to run, but distribution and code-quality gates are red.

## 3. Top 10 problems
1. cargo clippy -- -D warnings fails with 10 errors (evo-pods 2, evo-ledger 6, plus 2).
2. cargo audit reports 1 vulnerability (zvariant unmaintained) + 5 allowed warnings.
3. Gatekeeper rejects /Applications/Evo.app (ad-hoc signature, no TeamIdentifier).
4. No cargo-deny installed/configured.
5. Throwaway EVO_STORAGE_ROOT run did not populate a data directory (mechanism unverified for first run).
6. evo-daemon and evo-agent remain running after the desktop app exits (orphan/always-on not documented in audit).
7. Bundle ships mock fallback code in bridge.js (browser preview path) though native path is used.
8. 11 compiler warnings in cargo build.
9. 535 unwrap/expect calls and 297 unsafe blocks in evo-desktop (manual review needed).
10. No integration tests for first run, Hey Evo, Continue, pause, permission denied.

## 4. Findings

### Build health
F-BUILD-01 [P1] Clippy fails under -D warnings
Symptom: cargo clippy --workspace -- -D warnings exits 101 with 10 errors.
Evidence: /tmp/clippy.log; errors: evo-pods 2, evo-ledger 6 (redundant closure x4, map keys, complex type x2, manual clamp).
Confidence: CONFIRMED.

F-BUILD-02 [P2] cargo build emits 11 warnings
Symptom: build succeeds but warns.
Evidence: cargo build --workspace tail: "evo-desktop (lib) generated 11 warnings".
Confidence: CONFIRMED.

F-BUILD-03 [P1] cargo audit reports 1 vulnerability
Symptom: dependency audit fails.
Evidence: "error: 1 vulnerability found!" (zvariant/zvariant_derive unmaintained, RUSTSEC-2026-0370/0372).
Confidence: CONFIRMED.

F-BUILD-04 [P2] cargo-deny not installed
Symptom: cannot run cargo deny.
Evidence: command -v cargo-deny returned not installed.
Confidence: CONFIRMED.

### Bundle
F-BUNDLE-01 [P1] Gatekeeper rejects bundle
Symptom: spctl --assess rejected.
Evidence: "/Applications/Evo.app: rejected", origin=Evo Code Signing, TeamIdentifier=not set.
Confidence: CONFIRMED.

F-BUNDLE-02 [P2] Mock fallback code shipped in bundle
Symptom: bridge.js contains mock() and mockRows() fallback.
Evidence: grep in /Applications/Evo.app/Contents/Resources/ui/app/bridge.js.
Confidence: CONFIRMED (native path used when webview present).

F-BUNDLE-03 [PASS] No sample/mock data files bundled
Evidence: find for *mock*/*sample* in ui returned only CSS comment and bridge.js; no data/ dir.
Confidence: CONFIRMED.

F-BUNDLE-04 [PASS] No remote URLs in bundle UI
Evidence: grep https?:// returned only w3.org, c2pa.org, github.com (fonts), scripts.sil.org.
Confidence: CONFIRMED.

### Security
F-SEC-01 [PASS] CSP present in home.html
Evidence: meta Content-Security-Policy default-src 'self'; connect-src 'none'.
Confidence: CONFIRMED.

F-SEC-02 [PASS] Inspector disabled in release
Evidence: webshell.rs cfg!(debug_assertions) sets developerExtrasEnabled 0.
Confidence: CONFIRMED.

F-SEC-03 [PASS] Custom scheme handler blocks traversal
Evidence: webshell.rs resolve() rejects non-Normal/CurDir components, canonicalizes, checks starts_with root and is_file.
Confidence: CONFIRMED (code review; no runtime traversal test performed).

F-SEC-04 [PASS] No innerHTML writes in product UI
Evidence: grep innerHTML only in safe.test.js (which throws on write).
Confidence: CONFIRMED.

F-SEC-05 [P2] 297 unsafe blocks and 535 unwrap/expect in evo-desktop
Symptom: manual audit required; potential panics on user paths.
Evidence: grep counts.
Confidence: SUSPECTED (not enumerated).

F-SEC-06 [P2] Logs and data dir permissions
Symptom: data dir 755, logs 644.
Evidence: ls -ld storage; ls -l logs.
Confidence: CONFIRMED.

F-SEC-07 [PASS] No work titles/transcripts in stderr log
Evidence: grep title/transcript/.docx/window in stderr.log returned only home/title.js serve lines.
Confidence: CONFIRMED.

### Install/upgrade
F-INSTALL-01 [P1] EVO_STORAGE_ROOT throwaway run did not populate
Symptom: /tmp/evo-audit2 and /tmp/evo-audit-data remained empty after launch.
Evidence: find /tmp/evo-audit-data empty.
Confidence: SUSPECTED (may require interaction or longer runtime; mechanism exists in code).

F-INSTALL-02 [P2] Orphan agent/daemon after app exit
Symptom: evo-agent and evo-daemon remain running after desktop app killed.
Evidence: ps showed PIDs 29540/29542 after killing 31103.
Confidence: CONFIRMED (may be by design for always-on capture).

### Performance
F-PERF-01 [COULD NOT TEST] Cold start, CPU, memory, redraw loop not measured
Reason: no automated harness; GUI process exits when detached; manual measurement not performed.
Confidence: N/A.

## 5. PASSED WITH EVIDENCE
- cargo build --workspace succeeds (11 warnings).
- cargo test -p evo-desktop: 311 passed, 0 failed, 0 ignored.
- CSP present; inspector disabled in release; no remote URLs.
- Bridge parses with MAX_MESSAGE_BYTES=8192 and typed validation.
- resolve() traversal protection (code review).
- No innerHTML writes.
- Bundle contains no sample data files.
- App icon .icns present; menu-bar PNGs present.

## 6. COULD NOT TEST
- First-run/migration with throwaway data (env mechanism unverified at runtime).
- Critical journeys (Hey Evo, Continue, pause, permission denied) have no integration tests.
- Performance numbers (cold start, CPU, memory, redraw loop, 1/100/5000 works).
- Notification/Spotlight icons need human confirmation.
- Auto-update: no auto-update mechanism found in repo (not implemented).