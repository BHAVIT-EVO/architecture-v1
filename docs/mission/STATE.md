# Mission state

Format: `step — STATUS — evidence`. Statuses: TODO, DOING, DONE, BLOCKED.
Read this first; continue from the first unfinished step.

## Phase 0 — baseline
- 0.0 save the mission text — DONE — docs/mission/MISSION.md
- 0.1 tag + branch — DONE — evidence/0-baseline.txt
- 0.2 backups (copy only, ~/Evo-backups) — DONE — evidence/0-baseline.txt
- 0.3 clippy — PASS — evidence/0.3-clippy.txt: `cargo clippy --workspace -- -D warnings` finished, CLIPPY_EXIT=0, zero warnings
- 0.3 cargo test — **BLOCKED** — `evidence/0.3-cargo-test.txt` holds a completed run,
  `TEST_EXIT=101`: 25 suites `ok`, 1 FAILED. Exact error:
  `crates/evo-ledger/tests/regression_gate.rs:41` — `regression gate failures:\n a mega-work holds 1420 of 1584 resources`
  (`test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 151.36s`).
  Cause, measured (`evidence/0.3-gate-mega-work.txt`): `crates/evo-ledger/src/hypotheses.rs:467`
  `union.union_reason(a, b, "title similarity")` applies 907 pairwise title links with
  unconditional union-find closure — `[group-debug] pages=3770 groups=2384 title_links=907
  largest=[1066, 23, 16, 13, 11, 9]`. One group holds 1066 pages, the next holds 23.
  Tried: (1) removed the quadratic rescan in `specific_tokens` (corpus token frequency built once) —
  the suite went from >26 min unfinished to 136–151 s, so the assertion now runs; (2) re-ran with the
  per-page memo deleted — identical failure, so the optimisation is not implicated;
  (3) instrumented the union phase to count groups and links — numbers above.
  Not done: changing the closure itself. That is a grouping-model change (any cap or ordering
  rule is a number the mission forbids inventing) and it rewrites how this person's real works are
  formed, so it is listed in HUMAN.md for a decision instead of guessed.
  25 suites `ok` (1 ignored in two suites, 252 tests in the largest), 1 suite FAILED —
  `evo-ledger --test regression_gate::regression_gate_on_the_real_log`,
  `test result: FAILED. 0 passed; 1 failed; … finished in 151.36s`.
  The assertion is the gate's own no-mega-work tripwire: `a mega-work holds 1420 of 1584 resources`
  (90 %, rule trips above 60 %). The run before this one never finished: the same suite was still
  executing after 26 minutes because `specific_tokens` rescanned every page title per token;
  `sample` named `content_adjacent_to_dominant → specific_tokens`. Cache the corpus token
  frequency once (identical arithmetic: same predicate, same divisor) and the suite completes in
  136–151 s. The mega-work failure is not caused by that change: with the per-page memo removed and
  only the frequency map in place, the gate still fails identically (`evidence/0.3-gate-nomemo.txt`).
  Fixing it means changing how attachment groups pages — a grouping-model decision, not a repair —
  so it is left open rather than guessed.
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
