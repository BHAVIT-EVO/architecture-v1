# Attack Log — Redesign + RFC-0014 WorkNamed vertical

**Date:** 2026-10-08 · **Branch:** `pods` · **Method:** three adversarial personas,
each attacking the new surfaces with a different failure theory; every finding is
either fixed at cause below or recorded as an accepted, honest limitation.

**Scope attacked:** the user-naming channel (`evo-daemon/src/naming.rs`, socket
wire, capture normalization, acceptance, engagement declarations, derive
overlay, `NameIndex` read-back), the Home rack additions (rename panel, search),
and the verification runs themselves.

---

## Persona A — the hostile local process

*Theory: the new socket channel is an unvalidated writer into canonical state.*

| Attack | Result |
|---|---|
| Probe-connect then drop (no bytes) | Handler reads EOF, rejects harmlessly; channel receives nothing. Covered by the shared `wait_for_listener` probe contract. |
| Oversized field (>4096 B) | `read_field` refuses: "empty or too long". Never forwarded. |
| Non-UTF-8 bytes | Refused: "not valid text". |
| Blank name (`"   "`) | Refused at the wire boundary, honestly reported as `Rejected`. |
| Name identical to the subject | **Found & fixed this round.** The channel forwarded it; only `Declarations::name` refused it downstream, so the shell reported `Accepted` for a declaration that carries nothing. `handle_naming_connection` now refuses it at the boundary with a canonical reason. |
| Malformed length line | Parse error → refused, connection answered, no panic. |
| Shell writes canonical state | None: the desktop only speaks through the socket; the daemon validates, persists, and re-derives. |

**Found & fixed (A1 — wire deadlock).** `read_request` created a fresh
`BufReader` per field; the first field's reader could buffer (and swallow) the
second field's bytes, deadlocking both sides. Both socket tests then hung
>60 s. Fixed at cause: one buffered reader spans both fields, matching the
grouping/designation wire contract. `naming::tests` now pass (3/3).

**Found & fixed (A2 — acceptance gate).** `ObservationSchema::is_canonical()`
did not list `("OBS-WORK-NAMED", 1)`, so the acceptance pipeline rejected every
WorkNamed observation as `UnknownSchema` even though the schema, language, and
validation layer accepted it. Added to the frozen set; the full vertical
(forward → normalize → accept → persist → interpret → derive) now closes.

## Persona B — the confused person

*Theory: the new controls say things Evo cannot back up, or strand the user.*

| Attack | Result |
|---|---|
| Rename with a blank field | Honest note: "A blank name says nothing; nothing was changed." Session closes, nothing sent. |
| Rename a work that faded mid-session (rebuild/decay) | The panel closes itself rather than naming a work the surface no longer shows. |
| Escape / Cancel | Session closes without sending. |
| Search with no matches | Honest caption — the rack explains that only real work is kept; no fake "did you mean". |
| Rename accepted → when does the card update? | Stated in the button flow: "Name sent to capture; it appears on the next refresh." No client-side title patching: the shell only reflects what the daemon re-derives (RFC-0011 shell rule). |
| Search for a work by *derived* vocabulary only (e.g. the URL slug) | Works: the person's name AND the derived title/narrative/trail all match — `work_matches` scores name (user name first, RFC-0014), narrative, and trail entries. Pinned by test. |
| Rename subject choice | **Found & fixed this round.** The panel picked `member_subjects.first()`, which can be a `Downloaded:` or `app:` identity — a product or container, not the work. Now the first non-download, non-app member subject; fallbacks preserved. |

## Persona C — the skeptical auditor

*Theory: the read-back chain lies somewhere — a name that appears on one
surface but not another, or derivation that overrules the person.*

| Claim traced | Verdict |
|---|---|
| Person's name outranks derived titles (Law IX) | Holds on **both** read paths. Workspace path: `understanding::derive` resolves the declared subject → Artifact (RFC-0011 §4 rule) → the projected Workspace holding it, and overrides that title. Thread path: `NameIndex` carries `OBS-WORK-NAMED` as a top-priority layer, verbatim, never suffix-stripped. Pinned by tests on both sides. |
| A name for a subject the evidence never witnessed | Inert by construction: kept in the declarations, resolves to no Artifact, never mints a body of work, never retitles. Pinned by test (`Ghost work` leaves titles unchanged). |
| Card identity drift | None: names decorate presentation only; thread ids and Workspace identity are untouched. `member_subjects` still engine-anchored. |
| The daemon's live path and replay disagree | Live: `cache::absorb` routes names through the same `evo_engagement::interpret` the batch path uses. The overlay in `derive` is a pure function of evidence. |
| Honest copy | The phase labels, standing, and "Evo has not yet linked this work…" states were re-read; no fabricated capability was added with this feature. The naming copy promises only transport + next-refresh appearance. |

## Verification record

- `evo-daemon` lib: **170 passed** (incl. the 3 socket tests, the derive overlay
  tests, the NameIndex priority + round-trip tests). 1 ignored (pre-existing).
- `evo-desktop`: **148 passed** (146 + 2 new rack search tests).
- `evo-observation`, `evo-engagement`, `evo-capture`, `evo-threads`,
  `evo-workspace`, `evo-retrieval`, `evo-restoration`, `evo-artifact`,
  `evo-execution`, `evo-storage`, `evo-voice`: all suites green in the full
  workspace run.
- `qa_layout`: **17/17 shots, exit 0**, zero clipped content except the
  intentional 420 px minimum-width stress shot. Run against a bounded fixture
  (`EVO_STORAGE_ROOT`, 2,500 of 681k records) — see limitations.

## Accepted limitations (stated, not hidden)

1. **Ledger regression gate on the live corpus fails, and honestly so.**
   `evo-ledger` `regression_gate_on_the_real_log` reports `a mega-work holds
   369 of 552 resources` (67% > the 60% tripwire). This session touched no
   ledger source; the working tree carries *earlier* uncommitted ledger changes
   (interleave dwell cap et al.), and the live observation log grew from 666k
   to 681k lines during the session (the machine's own capture daemon is
   running). The gate is doing its job on live data; it was **not** weakened.
   The mega-work question belongs to the legacy ledger path — the threads
   engine now serves Home and is gated by its own suites.
2. **Corpus before/after metrics are infeasible on this machine.** Full replay
   of the 681k-record log was already OOM-killed at 8 GB in a prior session;
   a debug-mode full-corpus `qa_layout` began the same climb (RSS >800 MB
   within a minute) and was aborted in favor of the bounded fixture above.
   The D2 finding is therefore settled by code identity, not corpus numbers:
   the mutual-strongest + average-linkage founding and the UbiquitousOnlyBridge
   gate already exist in the live Rust path (`evo-engagement`), with their own
   suites green (engagement 111 ✓). The stale `DIAGNOSIS.md` this plan
   inherited tracked a deleted Python prototype.
3. **Socket channels are local-trust surfaces.** They bind to a hashed path in
   the temp directory with no peer credential check — the same trust model as
   the pre-existing designation/grouping/continuation channels. A hostile
   *local user* can always write their own declarations; that is the product's
   threat model (the person's word is canonical by definition), not a defect.
