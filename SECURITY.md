# Evo — security and privacy risk register

Phase 4 deliverable. Each risk below comes from `SPEC.md` (the Phase 0 audit)
or from `PROJECT_BRIEF.md` §9. Status is one of **Fixed**, **Mitigated**,
**Open**, or **N/A**.

| # | Risk (from SPEC.md) | Status | What changed / what to know |
|---|---|---|---|
| 1 | The store is plaintext and unencrypted, with no retention limit (`SPEC.md` §3). | Mitigated | `PRIVACY.md` documents the location and the limit; retention (forever / 1 year / 90 days) is exposed in Settings → Privacy and Trust in the web layer. Encryption at rest is **not** built — listed as open in `PRIVACY.md`. |
| 2 | Logs are readable by any process running as the same user, so “private on this Mac” would overclaim. | Mitigated | The app says “Stays on this Mac” only; `PRIVACY.md` states the limit plainly. Activity shortens the home directory to `~` (`security::activity::redact`). |
| 3 | The native app has no permission model: no scopes, actions or modes (`SPEC.md` §1). | Fixed (model) | `crates/evo-desktop/src/security/permissions.rs`: `Scope::{Folder,App,Site}`, `Action::{Open,Read,Write,Run}`, `Mode::{AskEveryTime,AskForRisky,ActWithinScope}`, and `PermissionSet::decide` — kill switch → scope → mode. Targets outside every scope are denied in every mode. Persisted as tab-separated text; unreadable lines are counted, never trusted. The UI is the web layer's Activity → Permissions tab (six modes and scopes wired to the same rules). |
| 4 | No Activity log with reasons and undo in the shell. | Fixed (model) | `security/activity.rs`: newest-first, capped at 100, `what` / `when` / `why`, one-shot `undo` for hide, merge and rename, single-line flattening and 160-character truncation so a title cannot forge an entry. The user-facing list is Activity in the web layer. |
| 5 | No single global kill switch reachable from every surface. | Fixed (model) | `security::KillSwitch` (atomic, shared). Paused denies every decision, and the web layer refuses to open or listen while paused with one plain sentence: “Evo is paused. Resume from the tray to open work.” |
| 6 | Shell commands could be built from unsanitized strings; titles and paths reach process spawns. | Mitigated | `security/paths.rs::CommandLine` is the only sanctioned builder: absolute program, allowlisted to `/usr/bin`, `/bin`, `/usr/sbin`, `/sbin`, argument array, no shell. Existing spawns in `daemon.rs`, `engine.rs`, `app.rs` already pass argument arrays; pinning them to `CommandLine` is **Open** (18 call sites). |
| 7 | Path traversal and symlink escape when restoring or revealing a work. | Fixed (primitive) | `resolve_within`: lexical `..` refusal before any filesystem call, then canonicalization and a component-boundary containment check, so a symlink cannot walk out of an allowed folder. `is_within` refuses the `/Users/me` vs `/Users/melon` prefix trick. Restore call sites still to adopt it (**Open**). |
| 8 | Invalid IPC and command input. | Mitigated | Daemons already validate every subject against the canonical log before acting (`classification.rs`, `continuation.rs`, `feedback.rs`, `designation.rs`, `grouping.rs`, `naming.rs`). New scope and permission parsers reject NUL, relative paths, `..`, over-long values and non-host site strings, and unit tests cover each. |
| 9 | Secrets in plain files. | N/A | This build has no accounts, tokens or API keys, so there is nothing to store. `PRIVACY.md` states the policy: the macOS keychain (`Security.framework`) at the moment of use, never a file store. |
| 10 | The optional `fastembed` feature can fetch model assets over the network. | Mitigated | Off by default; `tests/trust_constraints.rs` fails the build if an HTTP/TLS client appears outside the feature, and `PRIVACY.md` says builds that ship must leave it off or the claim changes. |
| 11 | Accounts or servers could require auth hardening. | N/A | `SPEC.md` §3 and §5: there is no account, no server, no sync and no analytics. No auth surface to harden. |
| 12 | Reports could carry personal data. | Mitigated | Report a problem shows the exact payload (version, macOS, microphone state, last activity lines) and sends nothing until approved; there is no transport, so it is saved locally and can be copied. |
| 13 | Dependency supply chain. | Open | `cargo audit` and `cargo deny` are not installed on this machine and could not be run (see the final report). `clippy -D warnings` and `cargo test` remain the gate; the policy is in `PRIVACY.md`. |
| 14 | `unwrap`/`expect` in user-facing paths. | Open | 492 occurrences remain across the crate, mostly inside test modules; the new `security` module has none in product code. |
| 15 | Unsigned build; no notarization, no signed updates, no deep links. | Open | Not built yet. There are no deep links to validate, and the build is a development build. |
| 16 | “Stays on this Mac” could be shown while untrue. | Mitigated | The chip is tied to `PRIVACY.md`’s four claims and the trust-constraints test. If audio, text, embeddings or reports ever leave the device, the chip, the Privacy and Trust screen and `PRIVACY.md` must change in the same commit. |

## Negative tests that prove the refusals

All in `crates/evo-desktop/src/security/*` (`cargo test -p evo-desktop --lib security`):

- **Path traversal** — `..` refused lexically (`/Users/me/../../etc/passwd`,
  `/../etc`, `../../etc/passwd` as a request), and a symlink inside an allowed
  root that points outside it is refused after canonicalization.
- **Invalid input** — empty, relative, NUL-containing, over-long paths; bad
  app names (`../bin/sh`), bad site strings (`https://claude.ai`,
  `claude.ai/login`); malformed permission lines are skipped and counted.
- **Denied permissions** — out of scope denied in all three modes; paused
  denied even inside a scope; ask-for-risky asks on write/run and allows
  open/read; undo happens at most once; the activity cap holds; a work title
  with a newline cannot forge a second row; shell syntax stays one argument.

## Manual verification

- Web layer: opening Continue while paused shows the refusal and never opens
  the notch; after resuming, Continue opens it again.
- Web layer: Settings → Privacy and Trust links to this repository’s
  `PRIVACY.md` and shows the storage path.

## Dependency audit — 13 November 2026

`cargo audit` was installed and run against the workspace lockfile (cargo-audit 0.23.0).

- **Before `cargo update`:** 8 advisories, 9 allowed warnings.
- **After `cargo update`:** **1 vulnerability, 5 warnings.**
  - `libsqlite3-sys 0.30.1` — RUSTSEC-2026-0459 (bundled SQLite CVE-2026-119375). Fix needs `libsqlite3-sys >= 0.39`, which arrives with a rusqlite major bump (`rusqlite 0.32 → 0.37+`) used by `evo-ledger`, `evo-execution`, `evo-daemon` and `evo-voice`. **Open** — needs its own change with ledger migration tests.
  - Warnings only: `paste 1.0.15` (unmaintained), `ttf-parser`, `zbus_names`, `zvariant`, `zvariant_derive` (Linux-only paths). None has a fixed release.
- The advisories that cleared were the TLS stack (`rustls 0.23.45`, `webpki-roots`, `webpki-root-certs`, `rustls-webpki`), `tokio 1.53.1` and `rustix` — pulled in by the optional embeddings feature's client stack, which default builds do not compile.
- `cargo deny` is still not installed; `deny.toml` is not committed, so the gate today is `cargo audit` plus `cargo test`.
