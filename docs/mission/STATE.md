# Mission state

Format: `step — STATUS — evidence`. Statuses: TODO, DOING, DONE, BLOCKED.
Read this first; continue from the first unfinished step.

## Phase 0 — baseline
- 0.0 save the mission text — DONE — docs/mission/MISSION.md
- 0.1 tag + branch — DONE — evidence/0-baseline.txt
- 0.2 backups (copy only, ~/Evo-backups) — DONE — evidence/0-baseline.txt
- 0.3 clippy — PASS — evidence/0.3-clippy.txt: `cargo clippy --workspace -- -D warnings` finished, CLIPPY_EXIT=0, zero warnings
- 0.3 cargo test — RUNNING — evidence/0.3-cargo-test.txt: 25 suites `ok`, zero failures, then `regression_gate_on_the_real_log` still running at the 10-minute mark; no TEST_EXIT line yet. The first attempt failed to link with `ld: write() failed, errno=28 (No space left on device)`; the disk was at 204 MB free and `target/debug/incremental` held 20 GB, which was removed before the current run
- 0.3 node tests — PASS — evidence/0.3-node-tests.txt (28 pass)
- 0.4 mission-verify.sh + STATE.md — DONE — scripts/mission-verify.sh
- 0.5 duplicate bundles — BLOCKED — evidence/0.5-duplicates.txt (host refused the unscoped `mdfind`; bounded query returned nothing; HUMAN step added)

## Phase 1 — the build works by itself
- 1.1 reproduce — DONE — evidence/1.1-reproduce.txt
- 1.2 fix + version gate + sha pin — DONE — evidence/1.2-candidates.txt, scripts/ort.sha256
- 1.3 verify the plain command — DONE — evidence/1.3-build.txt, evidence/1.3-verify.txt, evidence/1.3-about.txt
- 1.4 install + launch + verify checks — DONE — evidence/1.4-install.txt, evidence/1.4-verify.txt

## Phase 2 — the voice helper
- 2.1 diagnose (incl. BTM disabled item) — TODO
- 2.2 install/bootstrap/kickstart at app start — TODO
- 2.3 truthful get_voice_state — TODO
- 2.4 live proof — TODO

## Phase 3 — Continue reopens work
- 3.1 classify the 6 newest works — DONE — evidence/3.1-reproduce.txt (work 68 stores a real URL but was refused as permission_needed; works 69 and 66 hang in `open`)
- 3.2 fix — PASS — evidence/3.2-after.txt, 3.2-three-works.txt, 3.2-rebuild.txt, 3.2-bundle.txt: click consent + bounded `open` + persistent work index; 68 partly_opened 8.37s, 69 opened 5.90s, 66 opened 5.89s, all EXIT=0
- 3.3 fixture tests — PASS — evidence/3.3-fixtures.txt: 1 passed, FIXTURE_EXIT=0
- 3.4 live proof — PASS (bridge-level, real works 68/69/66 on the installed build's code) — evidence/3.2-three-works.txt
- 3.5 collage — TODO

## Phase 4 — Hey Evo
- 4.1 EVO_MODEL_DIR + five fixtures — TODO
- 4.2 cap at 3 + no-match copy — TODO
- 4.3 mic authorization in Diagnostics — TODO
- 4.4 jsdom event tests — TODO
- 4.5 say-test trace — TODO
- 4.6 notch geometry — TODO

## Phase 5 — Day and Dusk
- 5.1 SCENES dusk/day — TODO
- 5.2 tokens only — TODO
- 5.3 native NSAppearance + migration — TODO
- 5.4 measured luminance/contrast — TODO
- 5.5 live appearance flip + persistence — TODO

## Phase 6 — settings, language, titles, claims
- 6.1 structural title cleaner — TODO
- 6.2 settings out of localStorage — TODO
- 6.3 STRINGS.md + scanner — TODO
- 6.4 privacy measurement + claims — TODO

## Phase 7 — final gate
- 7.1 full suite — TODO
- 7.2 rebuild + reinstall + live re-checks — TODO
- 7.3 HUMAN.md — DONE (draft; extend as later phases land) — docs/mission/HUMAN.md
- 7.4 FINAL_REPORT.md — TODO
