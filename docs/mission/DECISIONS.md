# Mission decisions

Every threshold, stop rule and the final reply format come from
[`MISSION.md`](MISSION.md). They are never re-derived here.

## D-000 — the mission text is the source of truth
Saved verbatim to `docs/mission/MISSION.md` as the first action, because the
build agent does not otherwise have the numeric contract (Dusk/Day tokens,
luminance targets, contrast ranges, timeouts). `scripts/mission-verify.sh`
asserts the same numbers as executable checks.

## D-001 — baseline commit is narrow, not a blanket `git add -A`
The tree held 223 changed paths (93 modified, 45 deleted, 85 untracked)
including `graphify-out/` (a 200 MB+ generated graph), `.kiro/`, `exports/`,
`site/` and a root screenshot. None were models, secrets or files over 5 MB
(the inventory in `evidence/0-inventory.txt` records this), but committing a
generated graph is wrong for its own reasons.

Committed: `crates/`, `scripts/`, `docs/`, `tests/`, `ui/`, `design/`,
`assets/`, `.github/`, root markdown, `Cargo.lock`, `deny.toml`, and every
tracked modification or deletion.
Left uncommitted and logged here: `graphify-out/`, `.kiro/`, `exports/`,
`site/`, `screenshot.png`. The full diff is saved outside the repo at
`~/Evo-backups/baseline-tree.patch`.

## D-002 — backups outside iCloud
`~/Desktop` is synced (Desktop & Documents). The data backup holds the user's
work history, so both backups live in `~/Evo-backups/`, which is not synced:
`Evo-data-backup-20261116` (441 MB copy) and `Evo-app-backup.zip` (232 MB).
No `~/Desktop/Evo-backup.app` existed, so nothing was removed.

## D-003 — ONNX Runtime choice is version-gated
`Cargo.lock` pins `ort`/`ort-sys` `2.0.0-rc.13`; the accepted runtime is
`1.29.0`. Both local candidates are 1.29.0. The build script verifies the
candidate's Mach-O version and `OrtGetApiBase` before using it, and fails on a
mismatch instead of falling back.
