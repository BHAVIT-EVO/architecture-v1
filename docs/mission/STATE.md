# Mission state

Format: `step — STATUS — evidence`. Statuses: TODO, DOING, DONE, BLOCKED.
Read this first; continue from the first unfinished step.

## Phase 0 — baseline
- 0.0 save the mission text — DONE — docs/mission/MISSION.md
- 0.1 tag + branch — DONE — evidence/0-baseline.txt
- 0.2 backups (copy only, ~/Evo-backups) — DONE — evidence/0-baseline.txt
- 0.3 baseline suites — TODO — evidence/0.3-node-tests.txt (28 pass); the cargo test file is empty because the runner restarted mid-run, so it must be run again
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
- 3.1 classify the 6 newest works — TODO
- 3.2 fix (openable target, UI pulse, reasons) — DOING — Continue now pulses instead of drawing listening bars (home.js, scene/index.js); openable-target work TODO
- 3.3 fixture tests — TODO
- 3.4 live proof — TODO
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
