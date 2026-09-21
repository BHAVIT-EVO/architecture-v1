# BE-AUDIT-0001 — Backend Behavioral Audit

**Status:** Report. Not authority. Nothing in this document amends a frozen contract.
**Scope:** Phases 0–11 of the audit brief.
**Source changes:** **None.** No file under `crates/` was created, modified, or deleted; every write this audit made went to this one document. `git status` is **not** a valid check of that claim — the working tree was already dirty before the audit began. §0.5 gives the check that does work.
**Verification environment:** no Rust toolchain. See §0.3.

---

## 0. Method, scope, and honest verification status

### 0.1 What was read at source

Authority documents read in full this pass, with their real paths under `docs/`: `constitution/CONSTITUTION.md`, `foundation/ARCHITECTURAL_LAWS.md`, `foundation/COGNITIVE_MODEL.md`, `product/PRODUCT.md`, **`architecture/ARCHITECTURE.md`**, `rfcs/RFC-0003`, `RFC-0004`, `RFC-0006`, `RFC-0010`, `RFC-0011`, `RFC-0012`, `RFC-0013`, `implementation/IS-0011`, `IS-0012`, `IS-0013`, `IS-0014`, `IS-0019`, `IS-0021`.

*(Path correction, verification pass: an earlier draft of this line placed `CONSTITUTION.md` under `foundation/` and gave `PRODUCT.md` no directory at all. `CONSTITUTION.md` is at `docs/constitution/CONSTITUTION.md` and `PRODUCT.md` is at `docs/product/PRODUCT.md`. Only this index line was wrong; every Article quotation in this report was re-verified at the real path during the verification pass and all of them are verbatim — Article III at `:81-93`, Article IV at `:99-114`, Article VII at `:162-179`, Article VIII at `:185-200`.)*

Code read at source, by `file:line`: `evo-daemon` (`runtime.rs`, `cache.rs`, `persistence.rs`, `grouping.rs`, `continuation.rs`, `designation.rs`, `workspace_replay.rs`, `ui.rs`, `daemon_status.rs`), `evo-workspace` (`formation.rs`, `deterministic.rs`, `co_membership.rs`, `workspace_decision.rs`, `attachment.rs`, `confidence.rs`, `workspace.rs`, `scenarios.rs`), `evo-restoration` (`derivation.rs`, `execution.rs`, `resume_point.rs`, `context_chain.rs`), `evo-execution` (`engine.rs`, `macos.rs`, `selection.rs`, `preflight.rs`), `evo-capture` (`macos_fsevents.rs`, `macos_event_source.rs`, `adapters/macos.rs`, `raw_event.rs`), `evo-storage/storage.rs`, `evo-api` (all five files), `evo-desktop` (`app.rs`, `state.rs`, `home.rs`, `detail.rs`, `shell.rs`, `theme.rs`), all 28 `examples/`, the single file under `tests/`.

**Not read:** `docs/TRACE-0001.md`, `IS-0002`, `IS-0005`, `IS-0020`, RFC-0001/0002/0005/0007/0008/0009. No claim below depends on them. RFC-0009 (`learning-contract`) is named in §11 only to record that it has no implementing code — I have not read its clauses and make no claim about what it requires.

**How to check a `file.rs:NN` citation.** **Fifteen** basenames under `crates/` occur in more than one crate, and **nine** of those are cited in this report, so a basename plus a line number does not always identify a file. Every such citation was resolved individually — §0.5 records the method, and records what the first version of this note got wrong. Where the line number alone is ambiguous, the crate is: `engine.rs:37` and `engine.rs:44-59` are **evo-execution** (evo-capture's `engine.rs` is 94 lines, so every higher number is unambiguous); `macos.rs:137-142`, `:158-171`, `:298-325`, `:316`, `:677` are **evo-execution** (769 lines), while evo-capture's `adapters/macos.rs` is 313 lines and is cited only with its `adapters/` path; `ui.rs:16` is **evo-daemon**, as are the report's only two other `ui.rs` citations, which already carry their crate (`evo-daemon/src/ui.rs:231-245`, `daemon/ui.rs:382`) — evo-desktop's `ui.rs` is never cited by line number in this report. `macos_event_source.rs` is evo-capture's **`src/`** copy at 918 lines; the same-named integration test is 63 lines, and every citation here (`:356-357`, `:452`, `:462-478`, `:648-659`) is far above it. `main.rs`, `persistence.rs`, `daemon.rs`, `replay.rs` and `validation.rs` citations are all above the shorter file's length and so resolve uniquely.

### 0.2 Classification legend

Used literally throughout. **PASS** — behavior verified correct against a cited clause. **FAIL** — behavior verified to contradict a cited clause. **ABSENT** — no code implements the thing; distinguished from FAIL because absence is sometimes contract-compliant. **NOT VERIFIED — ENVIRONMENT** — could not be executed here. **ARCHITECTURAL CONFLICT** — two authoritative documents disagree; I have stopped rather than resolved. **PRODUCT DECISION REQUIRED** — the contracts are silent and the answer is a judgment you own.

### 0.3 NOT VERIFIED — ENVIRONMENT

There is no `cargo`, `rustc`, or `rustup` in this environment and one cannot be installed (rustup download returns HTTP 403 from the proxy; `apt-get download rustc` cannot locate the package).

Consequently: **no test in this repository was executed. No binary was compiled. No end-to-end scenario was run.** Every behavioral claim below is derived from reading source, and is labeled as such. Where I say a test "passes" I mean *its assertion, read literally, is satisfied by the code path I traced* — not that I ran it. The 708 in-crate tests, the 1 integration test, the 28 examples, and the two vertical harnesses are all **NOT VERIFIED — ENVIRONMENT**. (`#[test]` counted mechanically: 708 under `crates/*/src`, 1 under `crates/*/tests`, 0 under `crates/*/examples` — the examples assert via `main`, which is §13.4's finding.)

This limitation cuts both ways and I want it stated plainly: it means I cannot prove the defects below reproduce at runtime, only that the code as written must produce them.

### 0.4 One thing the brief assumed that does not exist

The brief's API phase names eleven REST endpoints and two domain concepts (`moment`, `task`). **None of them exist** — not as routes, handlers, types, or strings. There is no HTTP server, no router, no JSON, no async runtime anywhere in the repository. I did not invent endpoints to audit. See §10.

### 0.5 What the verification pass checked, and what it found

Run after the report was complete, against the finished text rather than against my memory of it. **Six mechanical checks, all now clean:** every distinct code citation (181) resolves to a real file under `crates/` at a line that exists; every distinct document citation (66) likewise; all 17 tables have consistent column counts; every internal cross-reference of the form section-N-point-M resolves to a real heading (0 dangling, from 3 before the pass); every quoted phrase of five or more words is located verbatim in a real source file or accounted for by hand (110 located, 41 accounted for); and every citation whose basename occurs in more than one crate resolves to exactly one file. The fifth check exists because the first four cannot catch a misattributed quotation, and it found five. The sixth exists because none of the first five can catch a citation that names a file which exists twice, and it found the disambiguation note itself to be wrong in three ways. Both are recorded below.

*(Two notes on the citation counts, because the numbers moved. An earlier version of this paragraph said 157 code citations and 55 document citations. The population did not shrink and nothing became unresolvable — my first checker's pattern silently excluded two-segment paths such as `adapters/macos.rs` and every `Cargo.toml` citation, and it ran before §1.4 and §1.5 were added. The rebuilt checker sees 181 and 66, and all of them resolve. The first version of that checker also, on its very first run, matched **zero** `.rs` files while reporting "code citations in range: 0" — which reads exactly like a pass. Every checker used here was therefore validated by injecting known-bad inputs and confirming each was caught; a checker that cannot fail is not evidence.)*

It found six defects in my own text, all now corrected in place and recorded where they occurred: the `CONSTITUTION.md` and `PRODUCT.md` paths in §0.1; "Evo always explains its actions honestly" attributed to `ARCHITECTURE.md:248` when it is `PRODUCT.md:248` (`ARCHITECTURE.md:248` is a real but different line, the Decision glossary entry, which is how the slip happened); CONFLICT A's "multiple Artifact histories" attributed to `IS-0012:197`, where the word "multiple" does not occur at all — the language is `IS-0011:72`, which makes the conflict *stronger*; two cross-references pointing into subsections of section 11 that were never written (§11 has no subsections; both now point at §9.3 and §11); a §4.2 with no §4.1 above it; and a `|` inside a grep pattern breaking one table row. Five further inexact quotations were caught by the quotation check and are tabulated below. It also found two substantive omissions — Laws X and XIV (§1.4) and COGNITIVE_MODEL Principle V (§1.5) — which is the reason a mechanical pass was worth running on a report that was already written.

**Nothing the verification pass found reversed a verdict.** Two verdicts gained a second, independent clause (Scenario F, §6.2); one gained a stronger authority than the one I had cited (CONFLICT A); one gained a criterion it had been missing (§6.3). The direction of every correction was toward the finding being better grounded, not weaker, which is worth stating plainly because the opposite outcome was equally possible and would have needed reporting just as prominently.

**The verification pass introduced an error of its own, and range-checking could not see it.** §1.5 and §6.3 first cited Principle V's decisive sentence — "A person has resumed when continuing becomes easier than reconstructing." — as `COGNITIVE_MODEL.md:96`. It is at **`:94`**; `:96` is the next sentence, "The objective of resumption is capability." Both are inside Principle V and both are real lines, so the quotation attached to the wrong one of two adjacent true statements. The principle's range was likewise given as `:88-99` when its last body line is `:98`. Both are now corrected. This is the second time a correction has nearly or actually introduced a fresh error — the first was writing Article IV's range as `:99-113` when it ends at `:114`, caught before it shipped — and the pattern is worth naming: the riskiest line in a report is the one being rewritten, because it gets a confident tone from the act of correcting without necessarily getting a second look at the source.

It also marks the boundary of what the mechanical checker can do. All four checks passed on the wrong citation, because `:96` resolves to a line that exists in a file that exists. **A range check proves a citation is not dangling; it cannot prove it is not misattributed.**

So a second checker was written to close that gap: it lifts every quoted phrase of five or more words out of this report, normalizes smart quotes, dashes, markdown emphasis and the leading markup that prefixes source lines but never appears inside a quotation (`//!`, `///`, `>`, list bullets, table pipes), and searches for it verbatim across all of `docs/` and all of `crates/` — per line and, because `COGNITIVE_MODEL.md` and `ARCHITECTURAL_LAWS.md` put one sentence per line, against each file's whole joined text as well. It reports the true `file:line` of every phrase it finds and flags every phrase it cannot find anywhere. Of 142 distinct quoted phrases, 107 are now located verbatim in a source file. **It found five inexact quotations that the range checker had passed**, all now corrected:

| Where | I had written | The source actually says |
|---|---|---|
| §4.2.1 | "Under-attachment is preferred to over-attachment" | "Under-attachment is **always** preferred to over-attachment" (`ARCHITECTURE.md:151`, `:265`) — dropping *always* weakens a normative sentence |
| §8.6 Scenario D | "The purpose of restoration is not perfect recall… **it is** to restore the user's capability…" | Law XIV `:216`/`:218` repeat the subject: "The purpose of restoration is not perfect recall. **The purpose of restoration is** to restore the user's capability to continue meaningful activity." My elision rewrote the second clause instead of eliding it |
| §11 | "State is never stored… derived by replaying…" | `ARCHITECTURE.md:39` reads "State is never stored. State is derived, at any point in time, by replaying…" — the elision spliced two separated words into one phrase |
| §6.3 | introducing "no new inference responsibility or new primitives" | `ARCHITECTURE.md:160` reads "**They do not introduce** new inference responsibility or new primitives" — a grammatical adaptation inside quotation marks |
| §3.6 CONFLICT B | "previous Workspace state / historical Attachments / historical Snapshots" | `IS-0013:66-73` is a six-item bulleted list; three bullets were slash-joined into something that looks verbatim and is not |

None of the five changes a verdict, and that is the point worth being uncomfortable about: each one was a small convenience taken while quoting, none felt like an error while writing, and a report whose entire method is "check it at the source" cannot afford any of them. The remaining unmatched phrases — 41 on the current run, eight of them the superseded wordings quoted in the corrections table just above, in this paragraph, and in the basename correction below — were each examined by hand and are legitimate: quotations of your brief (which is not a file in this repository), my own paraphrase in quotation marks, quotes of my own superseded drafts labelled as such, and a handful whose text is verbatim but unmatchable mechanically because it nests a quotation (`CONSTITUTION.md:193-197`'s "Why?") or is split across a Rust format placeholder. Both checkers were validated by injecting known-bad inputs and confirming each was caught; the quotation checker also asserts on every run that a sentence known to be at `COGNITIVE_MODEL.md:94` is located there and that a fabricated sentence is not found. Its first version had a bug of exactly the kind it exists to catch — it joined blank lines into the search text, producing a double space between sentences, which made every quotation spanning two source lines fail and reported 69 phrases as missing when the true number was 40.

**The one check the header line originally got wrong.** That line used to read "`git status` should show only this document and `docs/audit/`." It is not a valid verification and I am publishing it rather than quietly swapping it out. The working tree was already dirty when the audit began: `git status --porcelain -- crates/` reports **68** entries — 45 modified, 23 untracked, six of the latter being whole untracked directories (`crates/evo-api/`, `crates/evo-capture/`, `crates/evo-daemon/`, `crates/evo-desktop/`, `crates/evo-execution/`, `crates/evo-workspace/examples/`) — and `docs/audit/` is itself untracked, so it appears as one directory entry rather than as this file. A reader following that instruction would have seen 68 dirty paths under `crates/` and reasonably concluded the audit had edited source. The check that does work is modification time: the newest mtime anywhere under `crates/` is `crates/evo-desktop/src/ui.rs` at **2026-08-17 16:01:51 +0530**, which is the earlier UI-implementation work of the same day, and every subsequent write on this machine went to this document, whose own mtime is strictly later than every file under `crates/`. Nothing under `crates/` has been touched since the audit began.

**A third check was needed, because a basename is not a file.** The citation checker indexed everything under `crates/` **by basename** and validated each reference against `max(linecount)` across all candidates sharing that name. Fifteen basenames under `crates/` are not unique, and nine of them are cited here, so for those the check was quietly weaker than "181/181 code citations resolve" implies: a citation counted as in-range if it fitted the **larger** file, even where the report meant the smaller one. This is the same species of error as the `:96`/`:94` misattribution — a check that passes for a reason unrelated to the thing being checked — and I did not notice it until a shell command of my own picked the wrong file: `find crates -name 'macos_event_source.rs' | head -1` returned the 63-line integration test rather than the 918-line source, and printed nothing for every line I asked it for.

A third pass resolved all **40** citations that used an ambiguous basename in the report as audited. **Thirty** resolve uniquely, either because the cited line exceeds the shorter file's length or because a quotation on the same report line matches only one candidate. **Ten** were genuinely ambiguous by range — `engine.rs:37`, `engine.rs:44-59`, `macos.rs:137-142`, `macos.rs:158-171`, `ui.rs:16` — and each was opened at source. (Re-running the checker now reports 45 and 15 rather than 40 and 10, because §0.1's corrected note and this paragraph each mention those same five citations again. The distinct set is unchanged at five targets; the count rose only because the disambiguation text has to name them.) All ten are correct as §0.1 assigns them: `engine.rs:37` is evo-execution's `use evo_restoration::execution::ExecutionRequest;`, which is exactly the narrow claim §8.1(b) makes about it; `engine.rs:44-59` is `TargetStatus` with its five variants and `#[derive(Debug, Clone, PartialEq, Eq)]` and no serialization derive, which is what both §8.6 Scenario E and §9.4 rest on; `macos.rs:137-142` and `:158-171` are evo-execution's window-title and file paths; `ui.rs:16` is evo-daemon's `workspaces.iter().max_by(compare_workspaces)`, quoted verbatim in §6.1. No verdict moves.

**The note itself, however, was wrong in three ways, and it was the note whose whole job was to prevent this.** It said "Eight basenames occur in two crates each" — fifteen do, and nine are cited here. It named only eight and **omitted `macos_event_source.rs`**, the one file whose ambiguity actually bit me; that omission is a completeness gap rather than a bad citation, since all four of its citations sit at `:356` or above and the rival file stops at 63. And its parenthetical "(evo-desktop's `ui.rs` is cited only above line 419)" is simply false: **419 is the length of evo-daemon's `ui.rs`**, and the sentence garbled the identical "shorter file's length" construction it had used correctly one clause earlier for `engine.rs`. The true statement is stronger than the false one — evo-desktop's `ui.rs` is never cited by line number anywhere in this report.

So there are three orthogonal properties a citation needs, and I had been treating them as one: the line must **exist** (range check), the quoted words must be **at** it (quotation check), and the name must identify **one** file (uniqueness check). Each of the three found defects the other two passed.

---

## 1. Corrections to my own prior claims

Published rather than silently fixed, per the working agreement.

**1.1 I invented a clause-numbering scheme for RFC-0006 and used it as authority.** My `docs/design/UI-VALIDATION-0001.md` cites "RFC-0006 R-1…R-7" roughly twenty times, including a quoted "R-2: Restoration MUST NOT exist solely to expose architectural state." **RFC-0006 has no `R-n` scheme.** `grep -n 'R-[0-9]'` on the file returns nothing. Its clauses are `## Requirement 1` … `## Requirement 7`, plus unnumbered `# Guarantees` (:161) and `# Forbidden Behaviour` (:173); Requirement 2 is *Assistance* and Requirement 7 is *Explainability*.

This is worse than a typo, because `R-2` and `R-7` are real identifiers *elsewhere* — IS-0001:50 (R-2 Validation), IS-0001:85 (R-7 Persistence), IS-0003:60, IS-0004:169, IS-0005:497. A reader checking "RFC-0006 R-2" would land on a real clause in a different document about a different subject. UI-VALIDATION-0001 line 513 claims its quotations were "confirmed word-for-word"; that claim is false as to the identifiers. **UI-VALIDATION-0001 needs a correction pass. Its conclusions may survive; its citations do not.**

Root cause, generalized: clause-ID schemes differ per document family and I assumed uniformity. RFC series uses `Requirement N`; **RFC-0012 has no numbered clauses at all** and must be cited by named section; IS-0011 uses `W-n`, IS-0012 `WF-n`, IS-0013 `WR-n`, IS-0014 `CS-n`, IS-0019 `RSP-n/CS-n/RP-n/RM-n/BL-n`, IS-0021 `§25.x`; ARCHITECTURAL_LAWS uses Roman numerals; COGNITIVE_MODEL uses `Principle I…VIII`. No `F-n` or `CM-n` scheme exists anywhere in the corpus.

**1.2 I had concluded that Home showing one row per observed file is contract-compliant. That conclusion was wrong, and `ARCHITECTURE.md` is why.**

My reasoning was: RFC-0012's Competing-World Test rejects every signal that would group unrelated files; the code implements exactly the two surviving signals; therefore 100 files → 100 Workspaces is required behavior, and the defect is only that nothing sits between Formation and the screen.

The first half stands. The second half was too generous. `ARCHITECTURE.md` — authority level 3, above every RFC, and explicitly self-asserting supremacy at line 7 ("If an RFC contradicts this document, the RFC is wrong") — **names four mechanisms that would sit between Formation and the screen, and all four are unimplemented**. So the missing layer is not an unspecified gap the architecture is silent about. It is specified, at the highest architectural level, and was never built. See §14.

**1.3 A violation I suspected and disproved.** `ARCHITECTURE.md:151` requires "Attachment decisions consider only a small, recent, local candidate set — never the full history." I expected a full scan. `evo-daemon/src/cache.rs:385-410` `formation_candidates_for` is **index-narrowed**: it looks up `candidate_positions` by `ArtifactId`, then adds only the Workspaces of declared/repository co-member subjects. `formation.rs:44-53` documents this explicitly as avoiding "an O(remembered-Workspaces) scan per Observation." **PASS on "small" and "local."** The word *recent* is not honored — the index has no time bound — but no clause defines a recency window, so that is not a FAIL either. Recording this because an audit that only confirms its own hypotheses is worthless.

**1.4 Two laws that bear directly on this audit, and that I had not cited.** Caught in the verification pass by enumerating all seventeen laws rather than the ones I had already reached for.

**Law X — Minimize Reconstruction, Never Thinking (`:160-168`):** *"The purpose of Evo is not to eliminate thinking. The purpose of Evo is to eliminate unnecessary re-thinking. Novel reasoning belongs to the user. Repeated reconstruction belongs to Evo."* This is the law form of the product promise in your brief, and it sharpens the §6 finding: a Home that lists 116 items and asserts nothing about which are resumable does not eliminate reconstruction, it relocates it. The user now reconstructs *which rows are work* instead of reconstructing the work. That is re-thinking, performed every time Home is opened.

**Law XIV — Capability Is The Objective (`:214-220`):** *"The purpose of restoration is not perfect recall. The purpose of restoration is to restore the user's capability to continue meaningful activity. Every architectural optimization should ultimately reduce the effort required to produce the next meaningful action."* This changes two verdicts' *reasoning* without changing the verdicts. Scenario D (a file reopens, but into the default app rather than the editor that held it) is a PASS **because** of Law XIV, not in spite of it — capability is restored even though recall is imperfect, and that is the trade the law explicitly asks for. Scenario F (all 12 at once) is a FAIL against Law XIV independently of `:211`'s layers, because twelve simultaneous windows increase the effort to produce the next action rather than reducing it. And §8.7's TTP finding is upgraded: see the paragraph added there.

Recording this as a correction rather than folding it in silently, because the failure mode it represents — reaching for the laws I already remembered instead of reading all seventeen — is the same failure mode that produced §1.1.

**1.5 The same omission in the cognitive model, and it cost the report its sharpest sentence.** Having found §1.4 by enumeration, I enumerated `COGNITIVE_MODEL.md`'s eight principles too. I had cited I, II and III. **Principle V (`:88-98`) contains the corpus's only testable definition of successful resumption**, and I had not used it:

> "A person has resumed when continuing becomes easier than reconstructing." (`:94`)

This is now threaded into §6.3, where it supplies the *criterion* for the resume-candidacy predicate — turning what I had described as a shape without a rule into a shape with a rule. **It does not remove any of the four decisions in §14.7**, and I want to be exact about that rather than overstate the find: decision 1 asks whether Home may *omit* a Workspace, which is a question about what Evo shows, while Principle V answers what Evo may *claim*. What it does is shrink decision 1 — with the criterion supplied, the narrow option (show everything, assert resumability only for the subset that passes) is fully specified and implementable without any further judgment from you. I am flagging the near-miss because a report that recommends a predicate while leaving its criterion to the reader is doing the easier half of the job.

---

## 2. Phase 1 — The intended behavioral model, reconstructed

### 2.1 The model in the documents' own words

`ARCHITECTURE.md:39` is the load-bearing sentence:

> **State is never stored. State is derived, at any point in time, by replaying a confidence-scored interpretation function over an immutable observation log, scoped to a window of resources called a Workspace.**

And `:41`: "The only fact that is permanently true is: *at time T, Evo observed X.*" Everything else is the output of a function, expected to change many times, never migrated — improved and replayed (`:189` "Improvement is replay. It is never migration.").

Five primitives, `ARCHITECTURE.md:47-64`: **Observation**, **Artifact**, **Workspace**, **Knowledge**, **Decision**. `:66-67` names the non-primitives explicitly: "Moments, Tasks, intent, narrative summaries … none of these are persisted as first-class, migratable entities."

### 2.2 The brief's pipeline versus the architecture's pipeline

The brief asked me to audit a fifteen-stage pipeline. `ARCHITECTURE.md:71-131` defines **eight** stages, and it excludes three of the brief's stages *by name*. This matters more than a naming mismatch, because auditing Evo against a pipeline it deliberately does not have would manufacture false defects.

| Brief stage | Architecture stage | Status |
|---|---|---|
| computer observation → captured event | [1] Capture | exists |
| normalization | (inside [1]/[2]) | exists, thin — `canonicalization.rs` is identity today |
| **noise filtering** | **no stage** | **ABSENT by omission, not by prohibition — see §12** |
| artifact/resource identity | [2] Artifact Resolution | exists |
| **temporal/session segmentation** | **explicitly excluded** | `ARCHITECTURE.md:131`; "session" appears in the corpus once, as a *prohibited* grouping basis (RFC-0012 Competing-World Test) |
| **moment** | **explicitly excluded as a stage** | `:131` "There is no separate 'Understanding' stage producing Moments or intent as an independent pipeline step". Moments are a *derived view* (`:67`, `:250`), not a pipeline object |
| work identity | [3] Workspace Assignment | exists |
| membership evidence | [3] → Attachment | exists |
| continuation evidence | RFC-0011 / RFC-0013 layer | exists (added after ARCHITECTURE was frozen) |
| body-of-work state | [4] Workspace State Update | exists |
| **resume candidate** | **no stage; the Decision primitive + Experience layer** | **ABSENT — the central finding** |
| restoration plan | [7] Restoration Planning | exists, degenerate |
| restoration execution | [8] Restoration Execution | exists |

Two conclusions. First, **the brief's "session segmentation" and "moment" stages must not be built** — `ARCHITECTURE.md:131` says they "were explored during design and deliberately excluded." Second, **"resume candidate" has no pipeline stage but does have an architectural home**: the Decision primitive (`:63-64`) plus the Experience layer (`:159-160`). That is where the missing work belongs.

### 2.3 Transition table

For each transition the architecture actually defines. `F` = fact, `I` = inference.

| Transition | Evidence required | Insufficient evidence | Ambiguity / conflict | Must never | Persisted | Reversible | Visible | F/I |
|---|---|---|---|---|---|---|---|---|
| OS event → Observation | a witnessed signal with a non-empty subject and a timestamp | empty/whitespace subject → rejected (`evo-observation/src/validation.rs:133-194`) | n/a — capture does not interpret | decide importance or membership (`ARCHITECTURE.md:142`); be slowed or made fallible by inference (`:142`) | yes, append-only, fsync per record | **never** (`Law VIII`, `ARCHITECTURE.md:141`) | not directly | **F** |
| Observation → Artifact identity | that two Observations most likely refer to the same external entity | remain distinct Artifacts | new identity hypothesis via replay, never a rewrite (`:146`) | determine Workspace membership, intent, meaning, or importance (`:147`) | yes (`artifact.log`) | by replay | not directly | **I**, "provisional" (`:146`) |
| Artifact → Workspace Attachment | exact same-Artifact re-witnessing (1.0), witnessed repository co-membership (0.8), explicit user declaration (1.0) | **"attaches nowhere"** (`:150`) — an authorized third outcome | under-attachment always preferred to over-attachment (`:151`, `:265`) | call a model; keep competing scoring subsystems; hard-merge (`:152`, `:264`) | yes, with confidence, additive + superseded (`:151`) | supersession only, never destructive edit | as membership | **I** |
| Workspace ↔ Workspace | *"linked by a relationship with a strength score"* (`:151`) | no link | — | hard merge, anywhere (`:264`) | **required by `:151`, ABSENT in code** | — | — | **I** |
| Workspace → Snapshot | idle time, significant change, or explicit request (`:105-106`) | no snapshot | — | hold UI state or plan semantics (IS-0011 W-28/W-29); be rewritten by replay (`:187`) | yes, immutable | **no** — historical fact | via detail screen | **F** about belief at time T |
| Workspace → Resume candidate | **undefined in the architecture** | — | — | — | disposable cache (`:170`) | — | Home | **I** |
| Snapshot → Restoration Plan | latest Snapshot, read cheaply | refuse honestly | — | perform new inference at restore time (`:157`, `:219`) | derivation status only | — | detail screen | **I** |
| Plan → Execution | a resolvable locator per target | report unavailable per target | — | skip a failure without logging (`:157`); block on every artifact before showing progress (`:157`) | **execution status is not persisted — see §9** | — | per-attempt | **F** per attempt |
| Any Evo action → Decision | the action itself | — | — | — | **`decision.log` declared, never written** | — | explainability (`:64`) | **F** |

The three rows in bold-adjacent positions — Workspace↔Workspace links, resume candidacy, and the Decision log — are the audit's substance.

---

## 3. ARCHITECTURAL CONFLICTS — document versus document

Per the brief I have stopped at each of these rather than resolving them. Twelve are recorded; A–I were established in the D2 extraction pass and are summarized; J–L are new from `ARCHITECTURE.md` and are the consequential ones.

### 3.1 The precedence rule itself is contested

`ARCHITECTURE.md:7`: "**If an RFC contradicts this document, the RFC is wrong.**" This agrees with the brief's precedence (3 Architecture > 5 RFCs) and disagrees with the practice visible in the repository, where RFC-0012 and RFC-0013 have been written *after* ARCHITECTURE and clearly refine it. Several conflicts below are resolvable only once you say which of these two is true. I have not assumed.

### 3.2 CONFLICT J — "container" (decisive for the whole product)

- `ARCHITECTURE.md:57-58`: "**Workspace** — The single durable, confidence-bearing derived **container** representing an ongoing body of work… Workspace is the primary object a user interacts with." Glossary `:242`: "a persistent, restorable **container** for an ongoing body of work."
- `RFC-0003:125-129`: "A Workspace **MUST NEVER** be treated as a container of Artifacts."
- `IS-0011:227-238`: canonical components closed at exactly four; **no name, title, description, label, last-activity timestamp, or artifact list exists**, and identity must be "semantically meaningless" (`IS-0011:246-253`, W-3).

This is a word-level contradiction on the system's primary object, and it propagates. `ARCHITECTURE.md:58` calls the Workspace "the primary object a user interacts with"; IS-0011 gives that object no human-readable attribute of any kind. That is precisely why `evo-desktop/src/state.rs:570-579` has to synthesize a title out of a raw witnessed subject string and fall back to the literal `"Untitled work"` — the UI is compensating for a primitive the IS layer forbids from having a name.

**PRODUCT DECISION REQUIRED.** Either the Workspace gets a derived (non-canonical, disposable, replayable) display identity, or Home permanently shows filesystem paths as work titles. I have not chosen.

**Refinement added while writing §14, and it narrows this conflict usefully.** I initially read `ARCHITECTURE.md:66-67` as forbidding narrative labels outright. Re-reading it at source, it does not: *"Moments, Tasks, intent, narrative summaries beyond what is stored verbatim in a Snapshot — none of these are **persisted as first-class, migratable entities**. They are **computed from the five primitives above, cached where performance requires it, and treated as fully disposable**."* And `:171` repeats it as a storage class: *"Replayed, never independently stored as a mutable object: … any narrative text beyond what is stored verbatim in a Snapshot. Computed on demand, cached only as an optimization, never migrated."*

So a derived display label **is architecturally authorized**, and by exactly the same mechanism as the ranking cache in §3.4 — an Experience-layer derived view that must survive being dropped. IS-0011 §4 forbids putting a title *on the Workspace primitive*; `:66-67` and `:171` authorize computing one *outside* it. The two documents are not in conflict about the mechanism at all; I was reading a prohibition into `:66-67` that its own sentence structure refutes.

What remains genuinely undecided is only the content question: **what should the label say**, given that the only material available is witnessed subjects. That is still yours, and it is a much smaller question than "may Evo name a body of work." The word "container" remains a real contradiction with `RFC-0003:125-129` and I am not resolving that; but it no longer blocks Home from having honest titles.

### 3.3 CONFLICT K — the Workspace↔Workspace relationship link

- `ARCHITECTURE.md:151`: "Workspaces are never hard-merged; only **linked by a relationship with a strength score**." `:264`, constraint 5: "No hard merges, anywhere. Supersession and relationship links only."
- No such object exists. `grep -i 'relationship|link_strength|related_workspace|workspace_link'` across all crates returns 17 hits in 7 files; **I read every one, and all 17 are prose inside doc comments** describing the Artifact↔Workspace attachment. There is no type, no struct, no field, no storage record, no log kind.
- No RFC or IS document defines it either. IS-0011 §4 closes the canonical component list at four, which forbids putting it *on* the Workspace — but says nothing about a separate relationship record.

**ARCHITECTURAL CONFLICT + the single largest specification gap.** Level-3 authority mandates a mechanism; levels 4–5 neither define nor forbid it; the code lacks it. This is the mechanism by which 100 Workspaces become 4 bodies of work without a hard merge, and it was never specified below level 3.

### 3.4 CONFLICT L — ranking is simultaneously forbidden and required

- `IS-0011:592-611` forbids the Workspace Model layer from "perform ranking", "perform search", "determine notification policy". No permitted input to that layer carries relevance or importance. `IS-0014` + W-7: Confidence is evidential strength only and SHALL NOT represent importance, priority, or ranking.
- `ARCHITECTURE.md:170` names, as a legitimate category of stored data: "Cached, explicitly disposable and cheaply rebuilt: embeddings, **ranking scores, any home-screen ordering**." `:199` names "**ranking formulas**" among the things "free to change constantly, without architectural consequence." `:64` gives the Decision primitive the example "**a search result ranked**."

**This one resolves, and the resolution is the audit's key insight.** The two are compatible because they are scoped to different layers. IS-0011 forbids *the Workspace Model* from ranking. ARCHITECTURE authorizes ranking as a **disposable Experience-layer cache** — `:159-160` "Search, Voice, Notifications, and Prediction are Experience-layer features — consumers of Workspace and Decision data through read interfaces… They do not introduce new inference responsibility or new primitives."

So: **surfacing is not prohibited. It is authorized, named, layered, and bounded — and it is unimplemented.** The test `ARCHITECTURE.md:170` sets for it is exact: "can it be silently dropped and regenerated without the user noticing anything beyond a brief delay?"

I earlier reported that surfacing was prohibited. §1.2 corrects that.

### 3.5 CONFLICT M — layered restoration

`ARCHITECTURE.md:156` makes it an *invariant*: "Restoration is layered and progressive." `:211` specifies the layers concretely — Layer 1 context shown instantly; Layer 2 the single primary artifact opened first; **Layer 3 supporting artifacts, capped to a small number; Layer 4 reference artifacts, opened last or on demand; Layer 5 historical context, never opened automatically**.

`IS-0021 §25.3`/`§25.4` hard-empty the Context Chain and Blockers, and `evo-restoration/src/derivation.rs:528-536` implements that literally. The result is that **Layer 2 is the only layer that exists**; 3, 4 and 5 are structurally unreachable, and Layer 1 has no object to carry it. See §8.

### 3.6 CONFLICTS A–I (from the RFC/IS layer, summarized)

**A.** The definition of a Workspace requires *multiple* Artifact histories, while RFC-0012 explicitly preserves single-artifact Workspaces as valid. `IS-0011:72` — the frozen Workspace Model's own definition — reads: "A Workspace is Evo's current best explanatory hypothesis that **multiple** Artifact histories collectively describe one coherent body of work." `RFC-0003:24` and `RFC-0012:68`/`:622` repeat it in the same words. Against that, `RFC-0012:497`: "Existing per-Artifact Workspaces remain valid historical understanding." Any implementation enforcing "multiple" as a validity gate violates one of them; the shipped implementation enforces nothing and so satisfies the second at the cost of the first. **No minimum Workspace size exists anywhere in the corpus** — I searched for one specifically, because "some workspaces contain only one resource" was on your defect list. W-19 (`IS-0011:504`) requires a Workspace to *cease to exist* if all supporting evidence is removed, which is the opposite of a floor.

*(Citation correction, verification pass: an earlier draft attributed the "multiple Artifact histories" language to `IS-0012:197`. That is wrong — the word "multiple" does not occur anywhere in IS-0012, and `:197` is "Otherwise a new Workspace SHALL be recognized." The language is `IS-0011:72`. The conflict is unchanged in substance and stronger in authority: it is not a procedural line in the Formation spec, it is the **definition of the primitive** in the frozen Workspace Model, and it says that a one-file Workspace is not a Workspace. That makes "some workspaces contain only one resource" — your third symptom — a contradiction of IS-0011's own definition rather than a mere silence in the corpus. It is still not a code FAIL, because no clause makes the definition a gate and RFC-0012:497 expressly declines to.)*

**B.** `IS-0013:66-73` says "Workspace Replay SHALL NOT consume:" and then lists, among six bullets, "previous Workspace state" (`:69`), "historical Attachments" (`:70`) and "historical Snapshots" (`:71`); `IS-0012:97-101` requires Formation — which Replay must reuse unchanged — to consume "existing canonical Workspace understanding."

**C.** `IS-0014:160-162` forbids Confidence from determining membership; RFC-0012's attachment rule makes the outcome a pure function of confidence scores, and `formation.rs:122-149` implements it that way (`score = same_artifact.max(co_membership)`, then `workspace_decision.rs:25-62` thresholds it).

**D.** RFC-0012 describes its amendments to IS-0014 and RFC-0003 as "proposed; applied only upon acceptance" — **yet both target files already carry them verbatim** (`IS-0014:110`, `RFC-0003:117`), with no changelog, no amendment note, and no record of the accepting authority. RFC-0003's `Depends On` list stops at RFC-0002 while its Requirement 2 now normatively cites RFC-0012. Process defect in the frozen corpus itself.

**E.** IS documents delegate the attachment algorithm as "an implementation detail" while RFC-0012 freezes the concrete constants 1.0 / 0.8 / 0.0.

**F.** Whether one Artifact may belong to multiple Workspaces is **neither permitted nor forbidden anywhere**. W-5 constrains only the other direction. This is a genuine, load-bearing hole — it is the difference between "this file is used by two projects" being representable or not.

**G.** W-19's "cease to exist" has no lifecycle state to express it: `IS-0011:271-274` closes the enumeration at `Active` / `Superseded`.

**H.** No idle threshold, session boundary, gap timeout, context-switch rule, or day boundary is defined anywhere in the corpus. COGNITIVE_MODEL Principle III: "Time alone does not determine whether an engagement has ended."

**I.** RFC-0012 forbids co-membership from feeding the Resume Point, Next Step, Context Chain, Blockers, or any execution selection ("**Relatedness is not a reason to restore**"), while RFC-0013's Continuation Surface is already used for exactly that in `evo-desktop/src/state.rs:762-777` with no contract authorizing it — the previously-recorded Phase-2 block, still unresolved.
---

## 4. Phase 2 — Traceability: contract → code → storage → read path → test → behavior

The brief asked for a classification `A–J`. I do not reproduce those letters because I cannot restate their definitions faithfully, and inventing a legend to look compliant is exactly the failure mode this audit exists to catch. The §0.2 legend is used instead.

One row per contract obligation that `ARCHITECTURE.md` states as a responsibility or invariant. "Test" means a test whose *assertion* covers the obligation, not a test that merely touches the code.

### 4.1 The traceability matrix

| # | Obligation (clause) | Implementation | Persisted | Read path | Assertion covers it | Verdict |
|---|---|---|---|---|---|---|
| 1 | Capture appends only; no interpretation inline (`:141-142`) | `evo-capture/macos_fsevents.rs`, `evo-observation/validation.rs:133-194` | `observation.log` | `cache.rs` incremental offsets | yes | **PASS** (read, not run) |
| 2 | Artifact identity is lookup + dedup, never inference (`:55`, `:146-147`) | `evo-artifact` | `artifact.log` | `cache.rs` | yes | **PASS** |
| 3 | Assignment decides attach / form-new / **attach nowhere** (`:150`) | `formation.rs:96-114`, `workspace_decision.rs:8-12` | — | — | no test names the third outcome | **FAIL — see §4.2.1** |
| 4 | Under-attachment preferred to over-attachment (`:151`, `:265`) | `workspace_decision.rs:34-61` | — | — | yes (`:78-133`) | **PASS in the tie rule, FAIL in effect — §5.4** |
| 5 | Attachments additive **and superseded**, never destructively edited (`:151`) | `attachment.rs:42-49` | `attachment.log` | — | none | **ABSENT — §4.2.2** |
| 6 | Single confidence value per Attachment; no competing scoring subsystems (`:152`) | `formation.rs:132-146` | yes | — | yes | **PASS** |
| 7 | Candidate set small and local, never full history (`:151`) | `cache.rs:385-410` | — | — | yes (`bench_formation.rs`) | **PASS** — see §1.3 |
| 8 | Models never decide identity or attachment (`:152`, `:266`) | no model anywhere in the tree | — | — | n/a | **PASS** |
| 9 | **Workspaces linked by a relationship with a strength score** (`:151`, `:264`) | none — 17 grep hits, all prose | none | none | none | **ABSENT — CONFLICT K** |
| 10 | Snapshot triggered by idle time, significant change, or explicit request (`:105-106`) | `formation.rs:177`, `:194` — one Snapshot per formation | `snapshot.log` | `cache.rs` | no test asserts a trigger | **FAIL — §4.2.3** |
| 11 | Snapshots immutable, never rewritten by replay (`:187`, Law VIII) | `snapshot.rs`, `workspace_replay.rs` | yes | yes | yes | **PASS** |
| 12 | Restoration is layered, Layers 1–5 (`:156`, `:211`) | `derivation.rs:520-542` | `restoration.log` | `cache.rs:533` | tests assert the *empty* chain | **FAIL — CONFLICT M, §8** |
| 13 | Every attempted artifact reports success or failure explicitly (`:156`) | `evo-execution/engine.rs:44-59`, `:171-215` | **no** | in-memory only | yes | **PASS in-memory, FAIL on durability — §9.3** |
| 14 | Restoration performs no new inference (`:157`, `:219`) | `execute_selection` reads a derived plan | — | — | yes (`selection.rs:537`) | **PASS** |
| 15 | Partial failure is a first-class outcome, never an exception (`:215`) | `TargetStatus` five variants; `all_opened()` | — | — | yes (`engine.rs:485-501`) | **PASS** |
| 16 | **Decision** — every action Evo takes is logged for explainability (`:63-64`, `:128`, `:168`, `:248`; Law XI; Article VIII) | `StorageObjectKind::Decision` declared at `storage.rs:366` | **never written** | none | none | **FAIL — §9.3, §11** |
| 17 | **Knowledge** — corroborated durable facts (`:60-61`) | `StorageObjectKind::Knowledge` declared at `storage.rs:365` | never written | none | none | **ABSENT, contract-consistent — §9.3, §11** |
| 18 | Time To Productive is *the* product metric (`:217`) | nothing; `grep -ril time_to_productive` and `grep -riow ttp` over `crates/` both → **0 hits** | — | — | — | **ABSENT — §8.7, §14.3** |
| 19 | Local-first; no network call to produce, interpret, or restore (`:227`, Law XIII) | no HTTP client, no socket beyond the local UDS | — | — | partially | **PASS — §12.4** |
| 20 | The Observation log is the most sensitive data and must never be transmitted (`:228`) | never transmitted; **stored in `$TMPDIR`** | `storage.rs:354-356` | — | none | **FAIL on durability, PASS on transmission — §9.1** |

Eleven PASS, six FAIL, two ABSENT, one split. The PASS column is not decoration: the parts of Evo that were built were built carefully, and the specific things the brief suspected of being sloppy — full-history scans, models deciding attachment, silent restoration failures, hidden network calls — are all clean. The failures cluster in one place, and §14 names it.

### 4.2 Three findings only the matrix reveals

**4.2.1 "Attaches nowhere" is not merely unimplemented — it is unrepresentable.**

`ARCHITECTURE.md:150` gives the Workspace Engine three outcomes: "decide whether it attaches to an existing Workspace, forms a new one, **or attaches nowhere**."

`formation.rs:96` declares `fn form_workspace_from(...) -> Workspace`. Not `Option<Workspace>`. The return type cannot express "nowhere." `WorkspaceDecision` (`workspace_decision.rs:8-12`) has exactly two variants, `AttachExisting` and `RecognizeNew`, and `formation.rs:105-110` maps them onto exactly two constructors. Every Observation that reaches formation therefore produces or extends a Workspace. There is no path on which an Observation is witnessed, recorded, and left unattached.

This is the mechanical root of the symptom you reported first. "Under-attachment is always preferred to over-attachment" (`:151`, `:265`) is honored *within* the comparison — a tie with no same-artifact support declines to attach (`workspace_decision.rs:56-61`) — but declining to attach is implemented as **forming a new Workspace**, which is not under-attachment. It is proliferation. The architecture's third outcome is precisely the escape hatch that would have made under-attachment cheap, and it is the one outcome the type system forbids.

I want to be exact about what this is and is not. It is **FAIL** against `:150`. It is **not** a violation of `:151`/`:265`, because those clauses constrain the attach-versus-not decision and the code decides it conservatively. And it is not fixable by changing the return type alone: an Observation that attaches nowhere still needs a place to be found later, which is what the missing relationship link (`:151`) and the missing Experience layer (`:170`) were for.

**4.2.2 Attachment supersession does not exist.**

`ARCHITECTURE.md:151`: "Attachments are additive **and superseded**, never destructively edited — confidence changes produce a new record, with the old one marked superseded, so history remains explainable **without a separate provenance subsystem**."

`grep -rn 'supersed' crates/evo-workspace/src/` returns four hits: `deterministic.rs:179`, `snapshot.rs:140`, `lifecycle.rs:66`, `lifecycle.rs:72`. All four concern `WorkspaceLifecycle::Superseded`, which supersedes a *Workspace*. **No Attachment is ever marked superseded.** `Attachment` (`attachment.rs:42-49`) holds exactly two fields, `artifact_id` and `confidence`, and `build_attached_workspace` (`formation.rs:168-174`) mutates the set by `push`, never by supersession.

The clause's final clause is the load-bearing one. `:151` justifies having no provenance subsystem *on the grounds that supersession supplies the history*. Neither exists. `Attachment` carries no `ObservationId`, no timestamp, and no superseded marker, so given an Attachment at confidence 0.8 there is no stored path back to the Observation that produced it. Law XI (`:172-181`) — "the system must always be capable of explaining why an interpretation exists… Explanation is a first-class architectural requirement. Not a debugging feature" — is therefore unsatisfiable for the single most important interpretation in the system.

In practice the chain is recoverable *by re-derivation*: the Observation log is intact, so replaying formation reconstructs which Observation produced which Attachment. That is a real mitigation and I record it rather than overstating the finding. But re-derivation is not explanation: it answers "what would the current function conclude" rather than "why does this record exist," and `:151` explicitly chose supersession over recomputation as the mechanism.

**4.2.3 Snapshot triggers do not exist; a Snapshot is a side effect of every formation.**

`ARCHITECTURE.md:105-106` triggers a Snapshot on "idle time, significant change, or explicit request." `build_new_workspace` (`formation.rs:194`) and `build_attached_workspace` (`:177`) each append a Snapshot unconditionally. `grep -rn 'idle' crates/evo-daemon/src/ crates/evo-workspace/src/` returns one hit, `cache.rs:698`, inside a test name. There is no idle detector, no significance test, and no explicit-snapshot request path.

Consequence: Snapshot count equals formation count. A Workspace that received forty file saves holds forty Snapshots, each differing from the last only in `captured_at`. `IS-0011` W-10 ordering is preserved (`deterministic.rs:50-59` clamps monotonically), so nothing is *incorrect*; the object simply does not mean what `:105-106` says it means. Since restoration reads only `snapshots().last()`, the other thirty-nine cost storage and reload time and carry no information. This also makes `verify_scenarios.rs:66`'s printed expectation — "snapshots preserved: 3 (expected 3 for interrupted work)" — an artifact of there having been three formations, not evidence of three meaningful checkpoints.

---

## 5. Phase 3 — Work identity, membership, and the 100 FILES test

### 5.1 The complete formation algorithm, stated exactly

Read out of `formation.rs:122-153`, `deterministic.rs:22-31`, `co_membership.rs:166-219`, and `workspace_decision.rs:25-62`. This is the whole of Evo's work-identity logic; there is nothing else.

For an incoming Artifact `A` with witnessed subject `s(A)`, and each candidate Workspace `W`:

```
same_artifact(A, W) = max{ conf(att) : att ∈ W.attachments, att.artifact = A }   (0 if none)
co_member(A, W)     = max{ strength(s(A), s(B)) : B ∈ W.attachments, B ≠ A }
score(A, W)         = max(same_artifact, co_member)

strength(x, y) = 1.0  if (x,y) is an explicit OBS-WORK-GROUPED pair
               = 0.8  if OBS-REPOSITORY-MEMBERSHIP places x and y in the same repository
               = 0.0  otherwise

decide: best = argmax score
        best ≤ 0.0                          → RecognizeNew          (a new Workspace)
        unique best                          → AttachExisting(best)
        tie, exactly one has same_artifact    → AttachExisting(that one)
        tie, zero or ≥2 have same_artifact    → RecognizeNew
```

Three facts follow directly, and all three are certain from the code rather than inferred.

**`strength` has exactly two non-zero sources.** An explicit user grouping, and git repository membership. There is no third. `grep -rni 'threshold' crates/ --include=*.rs` returns **0 hits** across the entire tree, so `best ≤ 0.0` at `workspace_decision.rs:34` is the only gate — a zero test, not a tunable threshold. Nothing in the system can be adjusted to group more or less aggressively.

**Repository membership is genuinely witnessed and genuinely wired.** `macos_fsevents.rs:297-315` resolves `repository_identity_for_path` at capture time on every witnessed file save and emits `RepositoryMembership` **before** the `FileSaved` signal; `:285-290` documents why the order matters. `repo_identity.rs:17-22` handles worktrees and submodules correctly. `co_membership.rs:178-181` requires *both* subjects to carry a membership fact for the same repository. This is real, working machinery, and it is the one place Evo already does what `PRODUCT.md:118` promises.

**Nothing else can ever co-member.** `repository_identity_for_path` is called only for file paths and reflog paths. A URL is never in `member_repository`; a window title is never in `member_repository`. Therefore `strength(url, anything) = 0.0` and `strength(window_title, anything) = 0.0`, always, unless the user manually declares a pair.

### 5.2 The 100 FILES test, executed on paper

The brief's scenario: 100 files, 15 browser tabs, 3 terminals, 2 editors, four real bodies of work. Traced through §5.1. Counts are mine; the brief's figure of 118 differs from my arithmetic and I have not adjusted mine to match.

| Input | Witnessed as | Co-membership available | Workspaces formed |
|---|---|---|---|
| 100 files, all inside 4 git repositories | `OBS-FILE-SAVED` + `OBS-REPOSITORY-MEMBERSHIP` | 0.8 within each repository | **4** |
| 100 files, not in any repository (`~/Documents`, `~/Desktop`, exports, downloads) | `OBS-FILE-SAVED` only | none | **100** |
| 15 browser tabs | `OBS-URL-NAVIGATED` | **structurally none** | **15** |
| 3 terminals | `OBS-WINDOW-FOCUS-GAINED` | **structurally none** | **3** |
| 2 editors | `OBS-WINDOW-FOCUS-GAINED` | **structurally none** | **2** |

Best case, all files in git: **24 Workspaces** for 4 bodies of work. Worst case, no git: **120**. Neither is 4.

**This is the answer to the test, and it is not the answer I expected.** Evo does not fail the 100 FILES test uniformly. It passes it for git-tracked files — 100 files in 4 repositories genuinely become 4 Workspaces, and `scenarios.rs:564-577` asserts exactly this shape (one Workspace, four distinct member Artifacts, including a commit). It then fails it for every other resource class, and the failure is total rather than partial: a browser tab and a focused window can *never* join anything, at any confidence, under any amount of evidence, because the only function that could relate them returns 0.0 for their subject types.

So the residue after the mechanism that works has done its work is: every URL ever navigated, every window title ever focused, and every file saved outside a repository — each one its own "body of work" on Home. That is what you are seeing, and it explains all three of your first symptoms at once, including "some workspaces contain only one resource," which is not an anomaly but the guaranteed steady state for two of the three resource classes Evo captures.

**And it cannot be fixed by a frontend filter**, for a reason worth stating precisely: a filter would have to decide which of the 120 are real, which is the co-membership judgment itself, relocated to a layer that `IS-0011:592-611` forbids from ranking and that has no access to the evidence. The brief's constraint here is correct on the architecture's own terms.

### 5.3 Why the test's own premise is contested inside the corpus

`scenarios.rs:227-245` is not an oversight. Read the doc comment:

> "…no canonical fact witnesses the repository membership of a file, so the Workspaces remain separate. This is the honest outcome under IS-0012 (formation consumes only canonical primitives) — grouping these files would require a new canonical evidence class."

and the assertion at `:242`: `assert_eq!(workspaces.len(), 3, "no canonical evidence links the three files")`. Three files in `/Users/alice/Evo/src/`, three Workspaces, **asserted as correct**. The test is right on its own terms — its fixture supplies no membership evidence, and inventing a link from shared path prefix is exactly what `RFC-0012`'s Competing-World Test rejects. `scenarios.rs:404-430` asserts the same for filename resemblance ("title/file-name resemblance is not evidence").

One correction to a claim in my own earlier notes: the comment at `:418-419` — "no Workspace ever contains more than one distinct Artifact under the current canonical evidence" — is **scoped to `run_scenario`**, which supplies no co-membership. `run_scenario_with_co_membership` at `:570-577` produces one Workspace holding four distinct Artifacts. Stating that invariant globally, as I previously did, overstates it. The correct statement is: *absent repository membership or an explicit user declaration, no Workspace ever holds more than one distinct Artifact* — which is a narrower claim and the one the code supports.

So the corpus contains a deliberate, defended position that one-Artifact Workspaces are honest. `RFC-0012:497` states it directly ("Existing per-Artifact Workspaces remain valid historical understanding"), and no minimum Workspace size exists anywhere in the corpus — I searched for one specifically. `IS-0011` W-19 requires a Workspace to *cease to exist* when its evidence is removed, which is the opposite of a floor.

**The gap is therefore not "formation is broken."** It is that exactly one automatic co-membership class was ever specified, `ARCHITECTURE.md:151` mandates a Workspace↔Workspace relationship link that would absorb the rest, and that link was never defined below authority level 3. That is CONFLICT K, and §5.2 is what its absence costs.

### 5.4 A latent proliferation mechanism, and an honest limit on what I can prove

`workspace_decision.rs:56-61`: when several candidates tie at the maximal score and none has same-artifact support, the outcome is `RecognizeNew`. For repository co-membership every tie is at exactly 0.8, and same-artifact support is 0 for a file being witnessed for the first time. So **if a repository's members are ever split across two Workspaces, the next new file in that repository forms a third rather than joining either** — and each subsequent new file faces a wider tie and forms another.

The trigger is a repository whose members occupy two Workspaces. `macos_fsevents.rs:285-290` names one route to that state in the code's own words: "a resource whose membership arrives only after its own save would be assigned to a solo Workspace before the co-membership exists." The emission order at `:302-314` is designed to prevent it within a single event, and `co_membership.rs:209-211` skips any attached Artifact whose subject is unknown, which is a second route.

**NOT VERIFIED — ENVIRONMENT.** I can prove the branch exists and what it does when reached. I cannot prove ties occur on your machine, because no test in this repository exercises a co-membership tie and I cannot run anything. I am recording it as a mechanism to check first if Home shows *repository* files individually, since that would not be explained by §5.2.

### 5.5 One hypothesis I expected to confirm and disproved

I expected Workspace identity to be content-derived and therefore unstable — `build_new_workspace` computes `workspace_identity(&lifecycle, &attachments, &snapshots)` (`formation.rs:197`), an FNV hash over the Workspace's own contents (`deterministic.rs:33-40`). If growth recomputed it, every attachment would mint a new ID, `upsert_workspace` (`persistence.rs:695-706`) would never match, and Home would gain a row per save. That would have explained your symptom on its own.

It does not happen. `build_attached_workspace` reuses `candidate.id().clone()` (`formation.rs:180`). Identity is assigned once, at recognition, and survives growth. **PASS**, and I am recording the disproof because an audit that only lands the findings it went looking for is not an audit.

While confirming this I did find a stale invariant comment: `formation.rs:166-167` claims "Workspace Decision only attaches an Artifact that is already part of the candidate Workspace (candidate_score > 0), so the set is stable." That is true of the same-artifact path only. The co-membership path attaches Artifacts that are *not* in the candidate, `:169-174` pushes them, and `scenarios.rs:572-577` asserts a four-member set. The comment describes a superseded version of the algorithm and should not be trusted by the next reader.

---

## 6. Phase 4 — Resume candidacy and what Home actually claims

### 6.1 There is no resume-candidacy step. Home is the Workspace list.

`app.rs:255-270` is a `map` over `self.workspaces`, with no `filter`, no `sort`, no `take`, and no predicate of any kind:

```rust
self.home_cards = self.workspaces.iter().map(|workspace| { … }).collect();
```

So `home_cards.len() == workspaces.len()`, unconditionally. `home.rs:157-166` then draws one `work_row` per card in a plain `for` loop with no bound. The only narrowing on the path is `home.rs:322-352`, three user-driven predicates — a substring query, a kind chip, a period chip — each inert until the user acts.

Ordering is the order records were appended. `persistence.rs:695-706` `upsert_workspace` replaces in place when the id already exists and otherwise pushes to the tail, so Home is ascending by first-formation time with the newest work at the **bottom**. `home.rs:171-175` states this to the user without spin: *"In the order Evo recorded them. Nothing here is ranked, scored, or promoted for being recent."* That sentence is accurate.

**Correction to my own earlier claim.** I previously reported `evo-daemon/src/ui.rs:231-245` (`compare_workspaces` / `display_rank`) as dead code whose output the desktop discards. That is wrong twice. It is **live** — `evo-desktop/src/state.rs:16` imports `select_display_workspace`, which is `workspaces.iter().max_by(compare_workspaces)` at `ui.rs:16`, and it is called at `state.rs:310` and as the final fallback of `select_workspace_with_designation` at `state.rs:345`. What is true is narrower and more interesting: the tree's only ranking comparator is used exclusively to pick **one** Workspace, never to order the list. Nothing orders Home.

The only type in the repository named `ResumeCandidate` lives in the orphaned `evo-api` crate, and its producer returns `Err(ApiError::CapabilityUnavailable)` unconditionally (`api.rs:35-37`). See §10.

### 6.2 What Home therefore claims, and the clause it fails

Every row on Home is an assertion. `PRODUCT.md:226` — "It is not another dashboard" — and `:242` — "Evo remains invisible until needed" — set what a row means: not *Evo witnessed this*, but *this is work you may want to continue*. With §5.2's counts, Home makes that assertion 24 times in the best case and 120 times in the worst, for four bodies of work.

The clauses this fails, in precedence order:

**`CONSTITUTION.md` Article III (`:81-93`)** — "A system that is occasionally silent is preferable to one that is confidently wrong… Whenever uncertainty exists, Evo shall prefer restraint over speculation. **Trust shall always take precedence over completeness.**" Home is the completeness end of that trade, absolutely. It shows everything and asserts nothing about which items are continuable, which in a list whose semantics are "work you may want to continue" is not restraint — it is 116 confident wrong claims.

**`ARCHITECTURAL_LAWS.md` Law VI (`:104-112`)** — "Attaching too little context is recoverable. Inventing incorrect context damages trust… **Silence is preferable to fabrication.**"

**`PRODUCT.md:246`** — "Evo never pretends certainty."

**Verdict: FAIL.** Your brief's standard — "a false positive on Home is a product failure" — is not a stylistic preference. It is Article III of the Constitution, and it is the one place where Evo's current behavior contradicts its own founding document rather than merely lagging its architecture.

**Law X (`:160-168`) is the second clause it fails, and it is the more diagnostic of the two:** "The purpose of Evo is to eliminate unnecessary re-thinking… Repeated reconstruction belongs to Evo." A Home that lists everything and distinguishes nothing has not eliminated reconstruction; it has moved it. The user reconstructs *which rows are work* on every visit — reliably, repeatedly, and with no way to record the conclusion, because §11 establishes that Evo captures no signal for "this is not work I will return to." That is precisely the repeated reconstruction the law assigns to Evo, performed by the user instead.

### 6.3 The missing concept has a name in the corpus, and the corpus also says what shape it must take

`COGNITIVE_MODEL.md` Principle II (`:44-58`) already defines exactly what Home is missing:

> "Most engagements naturally conclude. These are closed engagements. Some engagements remain meaningfully continuable after attention leaves them. **These are open engagements. Only open engagements create continuity across interruption.** Openness is a property of an engagement. **It is not a separate object.**"

Nothing in the code computes openness. `grep` finds no open/closed distinction, no continuability predicate, and no candidacy rule; `WorkspaceLifecycle` (`IS-0011:271-274`) closes at `Active` / `Superseded`, which is a supersession state, not an openness state. Every Workspace Evo has ever formed is equally and permanently presented as resumable.

Two independent clauses then agree on the implementation shape, which is unusual enough to be worth relying on:

- Principle II's own last line: openness "is not a separate object."
- **Law XVI, Identity Law (`:236-246`)**: "Concepts that merely describe, rank, evaluate, or relate computational objects MUST NOT themselves become computational objects. Those concepts SHALL instead be represented as **relationships, derived values, or transient computational state**."

And `ARCHITECTURE.md:170` supplies the storage class it belongs in: "Cached, explicitly disposable and cheaply rebuilt: embeddings, **ranking scores, any home-screen ordering**," with a stated acceptance test — "can it be silently dropped and regenerated without the user noticing anything beyond a brief delay?" `:159-160` puts it in the Experience layer, as a consumer of Workspace and Decision data "through read interfaces": "They do not introduce new inference responsibility or new primitives."

So candidacy needs no amendment, no new primitive, no canonical field, and no new write path. It is a derived, disposable, replayable predicate over data Evo already has. That is the single most actionable finding in this report.

**And the corpus supplies the predicate's criterion, not just its shape.** `COGNITIVE_MODEL.md` Principle V (`:88-98`) — "Resumption restores capability, not memory" — contains one sentence that is operationally decisive and that I had not cited anywhere in an earlier draft:

> **"A person has resumed when continuing becomes easier than reconstructing."** (`:94`)

That is a testable definition of success, and it resolves the candidacy question without a product decision. A Workspace belongs on Home as a resume candidate when continuing it would be easier than reconstructing it. For the four real bodies of work in the 100 FILES test, obviously true. For a single `.xcuserstate` write, a `node_modules` file, or one URL navigated once and never returned to, obviously false — there is nothing to continue, so continuing cannot be easier than reconstructing. The predicate does not need to be clever, and it must not become a ranking model (Law XVI); it needs only to distinguish "there is an engagement here" from "there is a witnessed event here." Principle IV (`:76-85`) supplies the same boundary from the other side: "open engagements persist beyond interruption. Memory supports continuity. It does not create it." Evo currently ships the memory and not the continuity.

### 6.4 What is authorized, and the one part that is your decision

The distinction matters because it is the difference between implementing and amending.

**Ordering Home is explicitly authorized** — `ARCHITECTURE.md:170` names "any home-screen ordering" as a legitimate disposable cache, and `:199` puts "ranking formulas" among the things "free to change constantly, without architectural consequence." `IS-0011:592-611` forbids *the Workspace Model layer* from ranking; it says nothing about the Experience layer, and CONFLICT L (§3.4) resolves that scoping.

**Omitting a Workspace from Home is authorized nowhere.** I searched for a clause covering suppression, hiding, or non-display and found none. Article III favors silence; Law IX (`:146-157`) — "Architectural decisions must preserve the user's authority over meaning, priorities, goals, and decisions" — argues the other way, since hiding witnessed work removes the user's ability to judge it.

**PRODUCT DECISION REQUIRED.** The narrow reading I would defend, and am not adopting on your behalf: Home shows everything, and Evo asserts resumability for only the subset that passes the candidacy predicate — a distinction in what is *claimed*, not in what is *shown*. That satisfies Article III without suppressing evidence and without touching visual design. The alternative — suppress non-candidates behind a disclosure — reads better and needs a clause that does not exist yet.

### 6.5 Two places where Home states something Evo has not checked

**`detail.rs:305-311`** prints, in the `else` branch:

> "Because Evo's record holds exactly one resource for this work. Nothing was ranked or guessed."

Nothing counts resources. The branch is reached whenever the Resume Point's Artifact is not the marked one — including when a Workspace holds four Artifacts, as `scenarios.rs:572-577` shows it can. The sentence is a fabricated explanation, and it fails **Article VIII (`:183-200`)** ("When Evo acts, it should always be possible to answer one question honestly: 'Why?'"), **`PRODUCT.md:248`** ("Evo always explains its actions honestly"), and **Law XI (`:172-181`)**. It is a two-line fix and I have not made it. **FAIL.**

**`state.rs:570-579`** falls back to the literal `"Untitled work"` when no witnessed subject resolves. This is not a defect in the UI; it is CONFLICT J (§3.2) surfacing. `ARCHITECTURE.md:58` calls the Workspace "the primary object a user interacts with" while `IS-0011:227-238` closes its canonical components at four and gives it no name, title, label, or description, so the desktop must synthesize a display identity from a raw witnessed subject — which is why Home shows filesystem paths as work titles. **PRODUCT DECISION REQUIRED**, unchanged from §3.2.

---

## 7. Phase 5 — The continuation model

### 7.1 All three continuation signals are user declarations. None is observed.

The corpus defines exactly ten canonical Observation kinds (`grep -o 'OBS-[A-Z-]*'`): `OBS-WINDOW-FOCUS-GAINED`, `OBS-FILE-SAVED`, `OBS-URL-NAVIGATED`, `OBS-COMMIT-MADE`, `OBS-REPOSITORY-MEMBERSHIP`, `OBS-WORK-DESIGNATED`, `OBS-WORK-GROUPED`, `OBS-CONTINUATION-SURFACE`, plus `OBS-UNKNOWN` and `OBS-FUTURE-COLLECTOR` as reserved sentinels.

Four are witnessed activity. One (`OBS-REPOSITORY-MEMBERSHIP`) is witnessed structure. **The three that carry continuation meaning are all explicit user acts**, and `evo-capture/src/adapters/macos.rs` says so in its own doc comments: `:36-38` "The declaration is a witnessed user act, not a capture signal" for `WorkDesignated`; `:52-54` "the user explicitly declared two already-witnessed subjects to be related work" for `WorkGrouped`; `:60-64` "the user explicitly declared a set of already-witnessed subjects" for `ContinuationSurface`.

Consequently: **Evo cannot form a multi-resource continuation from observation alone.** Repository membership groups files that share a git root; nothing else groups anything unless the user declares it, pair by pair or set by set.

### 7.2 CONFLICT N — "quietly learns" versus the Competing-World rejection

This is new, and by the brief's precedence rule it is the most consequential conflict in the report.

`PRODUCT.md` (authority level 2) at `:118-136`:

> "It **quietly learns** which resources belong together. Files. Repositories. Browser tabs. Notes. Documents. Conversations. Meetings. Research. Then, whenever you decide to continue something, Evo reconstructs the complete environment that thought belongs in."

and at `:228-230`:

> "**Evo does not ask people to organize their work.** It accepts work exactly as it happens."

`RFC-0013` (authority level 5) `:180-257` runs a Competing-World Analysis over six candidate evidence classes and rejects five: Candidate C (derive the surface from Workspace membership), **Candidate D (derive it from recency, frequency, focus, or temporal grouping)**, Candidate E (reuse `WorkGrouped`), Candidate F (LLM / semantic similarity / embeddings), and Candidate B (reinterpret `WorkDesignated` as multi-subject). The sole survivor is Candidate A: an explicit multi-subject user declaration.

`RFC-0012` performs the same exercise for membership and admits only repository co-membership and explicit grouping.

Both RFCs are individually well-reasoned, and I am not arguing they chose wrong — each rejection is defensible under Law V, Law VI, and `ARCHITECTURE.md:29` ("If a claim about the user's work cannot be justified by the artifact evidence itself, Evo does not make that claim"). The conflict is at the level of what the product then *is*: if the only route to a multi-resource continuation is a user declaration, then the user is organizing their work, which `PRODUCT.md:228-230` forbids in as many words, and Evo is not "quietly learning" which browser tabs belong with which document.

**ARCHITECTURAL CONFLICT.** Under the brief's precedence (2 Product > 5 RFCs) and `ARCHITECTURE.md:7` ("if an RFC contradicts this document, the RFC is wrong"), PRODUCT wins and RFC-0013's exhaustive rejection of automatic continuation evidence cannot stand as written. Under the repository's *practice* — where RFC-0012 and RFC-0013 were written after ARCHITECTURE was frozen and visibly refine it — the RFCs win and `PRODUCT.md:118` is aspirational copy. I have not chosen, and this is the contradiction the brief told me to stop at rather than silently resolve. It is also the reason §5.2's residue exists: not an implementation gap, a resolved-in-the-wrong-direction product question.

Worth noting where the resolution most likely lives: `ARCHITECTURE.md:151`'s Workspace↔Workspace relationship link (CONFLICT K) is a mechanism for relating bodies of work *without asserting membership*, which is exactly the epistemic register a rejected candidate like temporal adjacency could legitimately occupy — a weak, scored, non-membership link. RFC-0012 and RFC-0013 each evaluated their candidates against a binary membership/continuation decision, because the middle register they would have needed was never specified below level 3.

### 7.3 The surface already decides execution, and three clauses forbid it

Verified verbatim at source this pass, all three sides:

- **`IS-0021:683`** — "It SHALL NOT change execution ordering or selection; **it never authorizes opening, focusing, or launching any resource.**"
- **`IS-0019` CS-4 (`:119-121`)** — "The Continuation Surface SHALL NOT change execution ordering or selection; it never authorizes opening, focusing, or launching any resource (RFC-0013 Non-Goals)."
- **`RFC-0013:414-416`, Gating Invariants** — "The surface does **not** change execution ordering or selection. Execution is downstream and remains out of scope."

And the code, `evo-desktop/src/state.rs:762-777`, inside `run_execution`:

```rust
if !outcome.continuation_surface().is_empty() {
    return match selection {
        Some(selection) => execute_selection(selection, locators, executor, executor),
        …
```

Its own doc comment, `:757-761`, states the intent in as many words: *"Once the user has declared a continuation surface (RFC-0013), Continue acts on exactly its restore-worthy members — the derived selection — and never on anything else."* That matters for how this FAIL should be read. It is not an accident, a leak, or a misunderstanding of the clause; it is a deliberate, documented design decision that the specification forbids. The author knew what the branch does and thought it was right — and on the product merits I think they were right, which is exactly why the remedy is an amendment and not a revert.

A non-empty surface takes strict precedence and becomes the execution set. That is precisely what all three clauses forbid. **FAIL**, and unambiguously so.

The trap, unchanged from my earlier report and now confirmed against the clause text: **do not fix the code.** `RFC-0013` Open Question 3 defers the surface→executable-target mapping, and `IS-0021 §25.3` hard-empties the Context Chain, so removing this branch leaves Continue with exactly one target on every Workspace — a strictly worse product that is also, by `IS-0019` RM-7 (`:349-353`, "Restoration SHALL minimize cognitive reload. This is the primary optimization objective of the Restoration layer"), further from the contract's own objective. A minimal amendment RFC authorizing surface-scoped execution selection is the prerequisite. Phase 2 remains blocked, on stronger evidence than before.

### 7.4 Why a Workspace so often has exactly one "focused window" continuation

Your fourth symptom, traced end to end. For a Workspace whose only Artifact was established by `OBS-WINDOW-FOCUS-GAINED`:

1. `IS-0019` RSP-1 permits exactly one Resume Point, and `IS-0021 §25.2` derives it from the Snapshot's candidate set — here, a single Artifact.
2. `IS-0021 §25.3` hard-empties the Context Chain; `derivation.rs:531-532` implements that literally, with the comment "No canonical supporting relationship exists in the current model."
3. `IS-0021 §25.4` hard-empties Blockers; `derivation.rs:536` implements it.
4. The plan therefore holds exactly one target.
5. `classify_locator` (`locator.rs:69-76`) maps that Artifact's schema to `LocatorKind::WindowTitle(subject)` — the subject *is* a window title, because that is all the Observation witnessed.
6. `engine.rs:185` maps `WindowTitle` to `executor.focus_window(title)`. There is no other action for a window title in the entire tree.

So "Restore" on such a Workspace focuses a window, and §8 covers what happens when the window is closed. Note step 2: the reason the chain is empty is that `ARCHITECTURE.md:151`'s relationship link — the *canonical supporting relationship* the comment says does not exist — was never specified. CONFLICT K again, arriving from the opposite end of the pipeline.

---

## 8. Phase 6 — Restoration, layer by layer, and the six reality scenarios

### 8.1 Two corrections to my own earlier findings, before the analysis

**(a) `MacOSExecutor` is real and shipped.** I earlier searched with `grep "impl .*PlatformExecutor for"`, which misses `impl<S: WindowSource> PlatformExecutor for MacOSExecutor<S>` at `evo-execution/src/macos.rs:153`. There is a genuine production executor: URLs via `NSWorkspace openURL:`, files via an existence check then `openFile:`, windows via a live Accessibility window enumeration and exact unique-title match (`:106-176`, `:298-325`, `:328`). It is wired into the shipped desktop at `detail.rs:390` and `app.rs:333`. Restoration is not a stub.

**(b) `evo_restoration::execution::execute` is a superseded dead stub, not the live path.** It does return `ExecutionResult::Refused` unconditionally (`:203-215`), and its stated reason — "no frozen Restoration Execution specification defines how a Restoration Plan is performed (IS-0019 §11; IS-0021 §20)" — was true when written and is now stale, because `evo-execution` was subsequently built. `grep` confirms nothing calls it outside its own tests; only `ExecutionRequest` is still imported from that module (`engine.rs:37`, `state.rs:20`, `detail.rs:406`). Reporting it as "restoration refuses to act" would have been a false FAIL. It is dead code carrying a misleading justification — worth deleting, not worth alarm.

### 8.2 The five layers against the code

`ARCHITECTURE.md:211` specifies the order precisely: "Layer 1, context — the Snapshot's summary, shown instantly. Layer 2, the single primary artifact, opened first. Layer 3, supporting artifacts, capped to a small number. Layer 4, reference artifacts, opened last or on demand. Layer 5, historical context, never opened automatically."

| Layer | Specified | In code | Verdict |
|---|---|---|---|
| 1 — context, Snapshot summary shown instantly | :211 | Snapshot has no summary. `IS-0011` §4 closes its components; W-28/W-29 forbid it holding UI state or plan semantics. Nothing is shown before execution. | **ABSENT** |
| 2 — single primary artifact opened first | :211, `IS-0019` RSP-1 | `ordered_targets` (`engine.rs:145`) puts the Resume Point first — but only on the plan path. See §8.3. | **SPLIT** |
| 3 — supporting artifacts, capped to a small number | :211 | Context Chain is hard-empty by `IS-0021 §25.3`; `derivation.rs:531-532` builds `ContextChain::new(Vec::new())`. No cap exists because there is nothing to cap. | **ABSENT** |
| 4 — reference artifacts, opened last or on demand | :211 | No such partition, no deferral, no on-demand path anywhere. | **ABSENT** |
| 5 — historical context, never opened automatically | :211 | `RestorationSelection.historical: Vec<ArtifactId>` (`selection.rs:142`), documented "durable membership, never restored" (`:179-181`, `:211-212`) and never passed to preflight (`preflight.rs:202-205`). | **PASS** |

So the layering exists at its two endpoints and is missing in the middle. Layer 3's absence is CONFLICT K arriving a third time: the Context Chain is empty because the *canonical supporting relationship* `ARCHITECTURE.md:151` mandates was never specified, and Layer 3 is the thing that relationship was for.

### 8.3 The two execution paths disagree about order, and the wrong one wins

There are two ordering rules in `evo-execution`, and they are not the same rule.

**Plan path** — `ordered_targets` (`engine.rs:133-152`): "The Resume Point is always first (IS-0019 RSP-1: the cognitive entry point), followed by the Context Chain in its canonical order." Correct, and faithful to Layer 2.

**Selection path** — `preflight_selection` (`preflight.rs:209-239`) builds restore-worthy ∪ unavailable and then, at `:236-238`:

```rust
// One canonical deterministic order for the whole report: ascending
// ArtifactId across the surface. Execution attempts follow this order.
outcomes.sort_by(|a, b| a.artifact_id().as_str().cmp(b.artifact_id().as_str()));
```

`ArtifactId` is an opaque identity string, not a rank. Sorting by it is deterministic — which satisfies `IS-0019` RP-4 — and simultaneously **arbitrary with respect to cognition**. Worse, `select_restoration` (`selection.rs:229`) iterates *only* the Continuation Surface, so the Resume Point is included only if it happens to be a declared surface member; otherwise it is not restored at all on this path.

And `state.rs:762-777` gives the selection path strict precedence whenever the surface is non-empty (§7.3). Therefore: **on the path that actually runs for any Workspace with a declared surface, the primary artifact is neither guaranteed first nor guaranteed present.**

This fails `ARCHITECTURE.md:211` Layer 2 and undercuts `IS-0019` RSP-1's stated purpose. It is a distinct defect from §7.3 — §7.3 is about *whether* the surface may select; this is about the surface path having silently replaced a cognitive ordering with a hash ordering. **FAIL.** Determinism was preserved and the reason for the ordering was lost.

### 8.4 There is no cap, no staging, and no on-demand tier

`grep -rn "\.take(\|MAX_\|cap\b\|limit" crates/evo-execution/src crates/evo-restoration/src` returns **zero hits**. Every READY target is attempted in one synchronous pass in `execute_selection` (`engine.rs:178-209`). Nothing is deferred, nothing is capped, nothing waits for the primary artifact to reach the foreground.

### 8.5 Scenarios A–F, answered from the code

| # | Scenario | What Evo actually does | Verdict |
|---|---|---|---|
| **A** | User closes everything, later asks Evo to resume | Split by artifact kind. Window-title artifacts → all `Unavailable` ("no window titled “{title}” is currently open; Evo does not guess which application owned it", `macos.rs:137-142` — `{title}` is the format placeholder, quoted here as it appears in source). File-path artifacts → `Ready` if the path exists (`preflight.rs:151-161`), reopened via `openFile:`. URL artifacts → always `Ready` (`:166`), reopened. Commits → `Unsupported`. So a Workspace built from file saves and URLs survives; one built from window focus is **wholly unrestorable**. | **FAIL (partial)** — Evo is honest about it, but the honest answer for the most common artifact kind is "nothing" |
| **B** | Some applications open, some closed | Handled correctly and per-target. Preflight runs over every surface member before anything is attempted (`engine.rs:177`); each gets its own status and reason; a failure never aborts the rest. This is exactly `ARCHITECTURE.md:213`'s "Partial restoration: a normal, expected outcome, never an error state." | **PASS** |
| **C** | A browser tab is closed. Can Evo restore it? | **Yes.** `OBS-URL-NAVIGATED` is captured live by `MacOSURLPoller` (`runtime.rs:192`), the URL is the canonical subject, preflight is unconditionally `Ready`, and `openURL:` opens it in the default browser. Evo does not restore the *tab* (no scroll position, no window grouping) — it opens the URL. | **PASS** |
| **D** | A file was open in an editor but the editor is closed. Can Evo reopen it? | **Yes, but not into that editor.** `open_file` checks existence then calls `NSWorkspace openFile:`, which resolves to the *system default application for the file type* (`macos.rs:158-171`, `:316`). Whether the editor was open is irrelevant — Evo never witnessed which application held the file, so it cannot and does not try to. The file comes back; the tool does not. **Law XIV (`:214-220`) makes this the right trade explicitly:** "The purpose of restoration is not perfect recall. The purpose of restoration is to restore the user's capability to continue meaningful activity." | **PASS with a caveat the UI does not state** |
| **E** | A resource is unavailable. Does Evo honestly say so? | **Yes, and this is the strongest part of the system.** Five distinct statuses (`engine.rs:44-59`), each carrying a written reason naming what was missing and stating that nothing was done; all five rendered with those verbatim reasons in the desktop (`detail.rs:587-598`). No substitution, no fuzzy path recovery, no guessing an owning app. Fully satisfies Article VIII, Law XI, Law VI, and `PRODUCT.md:248`. | **PASS** |
| **F** | 12 resources belong to the work. Does Evo reopen all 12 at once? *It must not.* | **It does.** All 12 are preflighted, then every READY one is attempted in a single pass in ascending-`ArtifactId` order, with no cap and no deferral (§8.4). The intended sequence from the architecture is unambiguous — `:211` Layer 2 one primary first, Layer 3 "capped to a small number", Layer 4 "opened last or on demand", Layer 5 never — and none of Layers 1/3/4 exists to sequence with. **Law XIV independently forbids it:** twelve simultaneous windows increase "the effort required to produce the next meaningful action," which the law requires every optimization to reduce. | **FAIL** |

Scenario F is the one place in this audit where your brief asserted the required behavior outright ("It must not"), and the architecture agrees with you verbatim at `:211`. This is a real defect, it is in the backend, and no frontend change can fix it.

### 8.6 The window-title trap, and the witnessed evidence Evo throws away

Your symptom — *"Restore/Continue often means merely focusing an already-open window; if that window isn't open the resource becomes UNAVAILABLE"* — is now traced to its root, and the root is in capture, not restoration.

`macos_event_source.rs` binds to the frontmost application by **pid** (`:356-357`, `frontmost_application_pid()` at `:648-659`), stores that pid in `FocusAttachment` (`:452`), and reads the focused window's title through that application's AX element. At the instant of witnessing, Evo knows the owning application. Then `emit_current_subject` (`:462-478`) emits:

```rust
MacOSSignal::WindowFocusGained { subject, observed_at: SystemTime::now() }
```

The application identity is **witnessed and discarded**. `adapters/macos.rs:17-20` carries only `subject` and `observed_at`; `classify_locator` therefore has nothing but a title to work with (`locator.rs:71`); and `MacOSExecutor::focus_window` can only search the live window list for that exact title. When the window is gone, the target is gone, permanently and irrecoverably — for information Evo had in hand.

The clause: **Law II (`:52`)** — "Every higher-level structure must remain derivable from preserved evidence." Reopening the application that held a window is a higher-level structure Evo needs for Layer 2, it was observable, and it is not derivable from preserved evidence because it was never preserved. I state this precisely rather than sweepingly: Law II's "never discarded" clauses (`:46-50`) concern recorded evidence and are not violated here; `:52` is the clause that bites, and it bites on a capture-completeness gap.

The licensed remedy is narrow and does not break anything: `ObservationSchema` is versioned (`observation_schema.rs:98-108`, `version: 1`), and `ARCHITECTURE.md:184-189` makes replay the mechanism for improvement — "Improvement is replay. It is never migration." An `OBS-WINDOW-FOCUS-GAINED` v2 carrying the owning application's bundle identifier alongside the title is additive, replayable, and leaves every v1 Observation valid and interpretable. **PRODUCT DECISION REQUIRED** only on whether to spend a schema version on it; the architecture already permits it. I have not written it.

Note also that focus is *deduplicated by subject* (`:465-468`: identical consecutive titles suppressed), so a window whose title changes — every editor with a dirty-file indicator, every browser — produces a *new* Artifact on each change. That is a second contributor to §5.2's Workspace inflation and to the "many things appear resumable" symptom, and it is a capture-layer cause, not a formation-layer one.

### 8.7 Time To Productive is not measured anywhere

`ARCHITECTURE.md:217` states: "**The one product metric:** Time To Productive — wall-clock time from the Resume click to the primary artifact being open and in the foreground. Every architectural decision in this document exists, ultimately, to make that number smaller."

`grep -rin "time_to_productive\|\bttp\b"` across `crates/` returns **zero hits**. No timer starts on the Continue press, no timestamp is recorded when the first target reports `Opened`, and `TargetStatus::Opened { detail }` carries a human string with no duration. Execution results are not persisted at all (see §9), so no TTP measurement could be reconstructed after the fact even in principle.

`ARCHITECTURE.md:215` is careful to distinguish what can be known mechanically ("whether each planned artifact opened successfully" — which Evo does know, honestly, per Scenario E) from what it declines to claim ("whether restoration truly helped the user resume productive work"). TTP is squarely in the first category, and it is the document's own single metric. **ABSENT.** This is why no one can currently say whether Evo is getting better or worse at the only thing it claims to optimize — including this audit, which is why §8.5's verdicts are stated in terms of behavior rather than performance.

**Law XIV (`:214-220`) says the same thing at constitutional level and raises the stakes:** *"The purpose of restoration is not perfect recall. The purpose of restoration is to restore the user's capability to continue meaningful activity. Every architectural optimization should ultimately reduce the effort required to produce the next meaningful action."* A law that requires every optimization to reduce an effort, in a system that measures no effort, cannot be complied with or violated — it can only be asserted. That is the precise sense in which TTP's absence is not a missing feature but a missing feedback loop: it is what allowed all four mechanisms in §14 to go unbuilt without anything registering a cost.

---

## 9. Phase 7 — Storage: there is no database, and where the data actually lives

### 9.1 The schema, stated plainly

Your brief asked for a database audit. There is no database — no SQL, no embedded key-value store, no ORM, and no `serde`. Persistence is eight append-only plaintext logs under one root directory (`storage.rs:358-369`):

`observation.log`, `artifact.log`, `workspace.log`, `attachment.log`, `snapshot.log`, `knowledge.log`, `decision.log`, `restoration.log`

Each record is length-framed as `len\n<bytes>\n`, appended with `OpenOptions::append(true)` followed by `sync_data()` per record (`storage.rs:93-101`), with fields encoded as hex `key=value` pairs. There are no migrations because there is no schema to migrate: `ObservationSchema` carries a name and a `version` (`observation_schema.rs:86-108`) and new versions coexist with old ones rather than replacing them.

This is a faithful and, in my assessment, well-executed realization of `ARCHITECTURE.md:164-176` and of "Improvement is replay. It is never migration." (`:189`). The torn-tail parser at `storage.rs:130-180` discriminates carefully between an interrupted final write and genuine mid-log corruption, and reports the latter as an error rather than silently swallowing it. **PASS**, and it deserves saying, because the next four findings are all serious.

### 9.2 The canonical Observation log lives in the operating system's temporary directory

`storage.rs:354-356`:

```rust
fn default_root() -> PathBuf {
    std::env::temp_dir().join("evo-storage")
}
```

and the shipped desktop resolves the same path at `state.rs:34-38`:

```rust
pub fn canonical_storage_root() -> PathBuf {
    std::env::var_os("EVO_STORAGE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("evo-storage"))
}
```

**Correction to my own first draft of this subsection, published rather than quietly fixed.** I originally wrote that "`EVO_STORAGE_ROOT` is read by ten `examples/` and by nothing in the shipped application." **That is false.** All three production entry points read it: `evo-desktop/src/state.rs:35`, `evo-daemon/src/main.rs:60`, and `evo-daemon/src/runtime.rs:705`. Six examples also read it. I built that claim from a partial grep and it would have made the defect look like an oversight, which it is not.

The corrected finding is narrower and, I think, more damning. The env var **is** honored, and all three call sites carry a doc comment saying what it is for: "`EVO_STORAGE_ROOT` overrides the location for developer/testing purposes **only**" (`state.rs:32`), "honor `EVO_STORAGE_ROOT` for developer/testing, default to the shared temp root" (`main.rs:59`), and the same again at `runtime.rs:703`. So the temp path is not an accident and it is not a test artifact leaking into production — it is the **documented default for real users**, chosen knowingly, with the real location left as a to-do. `daemon.rs:124` propagates whatever the desktop resolved into the spawned daemon's environment, so both processes always agree; the agreement is correct and the value is wrong.

There are **four** independent copies of the same default literal — `state.rs:37`, `main.rs:62`, `runtime.rs:707`, `storage.rs:355` — which is the second half of the problem: the canonical location of the canonical log is not defined in one place, so it cannot be corrected in one place.

On macOS `std::env::temp_dir()` returns `$TMPDIR`, a per-user, per-session directory under `/var/folders/<hash>/T/`. I want to be precise about what I am and am not claiming here. **NOT VERIFIED — ENVIRONMENT:** I cannot run macOS here, so I have not observed data loss and I will not assert a specific purge schedule. What I can assert from the code alone is the contract violation: this is the location the operating system designates for files that may be discarded, and Evo is putting there the record it promises is permanent. Every Observation, Artifact, Workspace, Attachment, Snapshot, and derived Restoration outcome Evo has ever recorded is stored in the directory whose entire purpose is to hold things that do not need to survive.

The clauses:

- **`ARCHITECTURE.md:13`** — the entire promise is "I can return to work **later** without having to reconstruct what I was doing."
- **`CONSTITUTION.md` Article IV — Continuity Is The Product.** A product whose canonical record is disposable by construction cannot deliver continuity.
- **`ARCHITECTURE.md:142`** — "Every write is append-only; nothing is ever mutated or deleted **except explicit user-requested purge**." Periodic OS cleanup is neither append-only nor user-requested.
- **Law II (`:44`)** — "Observable evidence is the permanent foundation of the system." It is currently not permanent.

**FAIL, and this is the most serious single defect in this audit.** Everything else in this report concerns Evo saying something wrong; this concerns Evo losing everything it knows.

The destination is not in doubt — `~/Library/Application Support/Evo/` is the platform-correct location, and because the log is a directory of plain files the move itself is a rename. What is in doubt is the transition: moving the root orphans every existing log, and Law VIII (History Must Never Change) plus `:142` mean the old logs cannot simply be abandoned or silently rewritten. **PRODUCT DECISION REQUIRED** on the one-time migration — copy-forward, replay-forward, or accept the loss and say so to the user — not on the destination. I have not changed it.

### 9.3 Two of the eight canonical kinds are never written

Counting every reference outside `storage.rs` across all of `crates/*/src`:

| Kind | References outside `storage.rs` |
|---|---|
| Observation | 3 |
| Artifact | 1 |
| Workspace | 9 |
| Attachment | 2 |
| Snapshot | 2 |
| Restoration | 3 |
| **Knowledge** | **0** |
| **Decision** | **0** |

`knowledge.log` and `decision.log` are never written, never read, and never created on disk. `ARCHITECTURE.md:61-64` lists Knowledge and Decision among the five primitives; `:128` gives Decision pipeline stage 7; `:168` gives both durable storage. Nothing produces either.

The Decision log's absence is the load-bearing one, and it is why several other findings in this report have no remedy available to them. `PRODUCT.md:248`'s "Evo always explains its actions honestly," **Article VIII** ("When Evo acts, it should always be possible to answer one question honestly: 'Why?'"), and **Law XI** ("Explanation is a first-class architectural requirement. Not a debugging feature.") all require a record of what Evo decided and on what evidence. Evo currently explains only what it can recompute in the present moment — which is why `detail.rs:305-311` (§6.5) reaches for a fabricated sentence instead of a recorded reason. **ABSENT.**

### 9.4 Nothing about an execution is ever persisted

`grep -rn "ExecutionReport"` intersected with append/persist/encode returns **zero hits**. `StorageObjectKind::Restoration` (written at `persistence.rs:574`) holds the derived *plan outcome* — what Evo would do — not what happened when the user pressed Continue. `TargetStatus::Opened { detail }` carries a human-readable string and no duration, and the enum derives no serialization (`engine.rs:44-59`).

Four consequences follow directly, and each of them is a finding elsewhere in this report losing its evidence base:

1. **TTP cannot be measured** (§8.7) — not merely unmeasured, but unmeasurable retroactively.
2. **Layer 2 success rate is unknowable.** Nobody can say how often Continue actually reopens the primary artifact.
3. **RFC-0009's learning loop has nothing to learn from.** Suppression, feedback, and ranking improvement all require a history of outcomes.
4. **The execution report exists only in UI transient state.** This is the structural cause of the earlier B-01/B-02 symptom — a canonical reload erasing a successful restoration's own report. That was mitigated in the frontend by keying transient state to navigation; the underlying fact remains that the only record of what Evo did to the user's machine is a value in a struct field.

**ABSENT.** Note the epistemics carefully: `ARCHITECTURE.md:215` explicitly declines to claim Evo can know "whether restoration truly helped the user resume productive work," and I am not asking it to. It does claim, in the same sentence, that "Evo can know, deterministically, whether each planned artifact opened successfully" — and it does know, at the instant of execution, and then forgets.

### 9.5 A read path truncates the canonical log, and only one of the two read paths was fixed

`storage.rs:182-192`, inside `read_all`:

```rust
if last_good < bytes.len() {
    let file = OpenOptions::new().write(true).open(&path)?;
    file.set_len(last_good as u64)?;
    file.sync_data()?;
}
```

I want to be precise about how bad this is, because the design is more careful than it first looks:

- **In its favour:** it removes only an incomplete trailing record that was never a complete Observation, and `:106-112` argues correctly that recovering a torn write is crash recovery rather than history rewriting. A single-writer lock exists — `daemon_lock.rs` holds a `daemon.pid` file and refuses a second daemon on the same root, explicitly "two writers to the same append-only logs would race." And the shipped desktop's live path uses the *non-truncating* `read_from` (`cache.rs:467, 480, 489, 533`).
- **Against it:** `read_from`'s own doc comment states the correct rule at `:211-213` — "a reader never truncates a log another process may be actively appending to" — which means the authors identified this hazard, fixed it in one function, and left it in the other. `read_all` is still called from `evo_doctor.rs:249`, a **separate process**, and from the daemon's `persistence.rs:70, 486, 586, 689`. Running the doctor while the daemon is mid-append discards the daemon's in-flight record: the daemon's next append lands after the truncation point, so the record is lost rather than corrupted.

**FAIL, bounded.** Maximum loss is one record per occurrence, and the occurrence requires a concurrent diagnostic run. The fix is to give `read_all` the semantics `read_from` already has. Two lines, and it is the one item in this report I would be comfortable calling safe to change without a contract question — which is precisely why I have not changed it, per the standing instruction.

### 9.6 There is no purge, in a system whose own architecture promises one

`grep -rni "purge|delete_all|forget"` across `crates/` returns **zero hits** outside tests. Meanwhile:

- `ARCHITECTURE.md:142` — "nothing is ever mutated or deleted except **explicit user-requested purge**."
- `ARCHITECTURE.md:225` — "The Observation log is the most sensitive data in the system by construction — it is a record of everything the user did."
- **Article VII** and **Law XIII** commit to local-first privacy and user ownership.

The user cannot delete an Observation, cannot delete a Workspace, and cannot stop capture from the interface. **ABSENT.** This is also half the answer to your Settings symptom: Settings is empty partly because there is nothing to configure (§12) and partly because the operations a user would most want — forget this, stop watching that — do not exist at any layer, so there was nothing for a settings screen to expose.

### 9.7 Five of fourteen crates are unreachable from the shipped binary

Resolved from `Cargo.toml` dependency edges across the workspace:

| Crate | Lines | Depended on by | What it was for |
|---|---|---|---|
| `evo-retrieval` | 232 | **nobody** | The **Retrieval Engine** — one of the four engines named at `ARCHITECTURE.md:139-157` |
| `evo-replay` | 650 | **nobody** | **Replay** — Law VII, and `ARCHITECTURE.md:180-189`'s entire mechanism for improvement |
| `evo-knowledge` | 842 | **nobody** | The **Knowledge** primitive (`:61-62`) |
| `evo-history` | 641 | **nobody** | Committed historical understanding |
| `evo-api` | 314 | **nobody** | The external API surface (§10) |

Total **2,679 lines, 7.4% of the 36,041-line source tree**, compiled as workspace members and linked by nothing. `evo-desktop` also has no dependents, correctly — it is the binary.

**Correction, caught while writing §13 and published here rather than quietly fixed.** My first draft of this paragraph said that because `evo-replay` is orphaned, "there is currently no mechanism by which a better formation rule could be applied to existing history." **That is wrong.** Replay is implemented and working — it just does not live in `evo-replay`. `evo-daemon/src/workspace_replay.rs` is 542 lines, exposes four public functions (`replay_workspaces_from_root`, `replay_current_continuation_surface`, `replay_continuation_surface_from_observations`, `replay_workspaces_from_observations`), re-derives Workspaces purely from the Observation log using the same `form_workspace_with_evidence` the live pipeline uses, and is exercised by tests at `runtime.rs:1501, 1617, 1784, 1842` plus three examples. Law VII is satisfied. Had I trusted the dependency graph alone I would have published a serious false negative.

What is actually true is narrower and still worth reporting: there are **two** replay implementations, and the orphan is the one written to the frozen spec. `evo-replay::replay` (`replay.rs:90`) is the IS-0013-conformant form, `replay(corpus, formation_fn) -> Workspace` — note the singular return, which is itself evidence that IS-0013 was written assuming one Workspace per replay. The working implementation in `evo-daemon` returns `Vec<Workspace>`. So the capability exists, the spec-conformant version is dead, and no test proves the live one satisfies IS-0013.

**Retrieval** is the genuinely absent one: `evo-retrieval` is a 232-line boundary type whose own doc comment concedes "RFC-0007 does not prescribe the retrieval algorithm, so this crate defines only the public service surface required by the architecture." One of the four engines named at `ARCHITECTURE.md:139-157` is a struct with no behavior, depended on by nobody. Same for **Knowledge**: a primitive at `:61-62` with 842 lines of model and no producer, consumer, or log writer (§9.3).

---

## 10. Phase 8 — The API surface

### 10.1 There is no HTTP API, and no frozen document asks for one

`grep -rn "axum|hyper|warp|tide|actix|TcpListener|HttpServer|route("` across every `Cargo.toml` and every `src/*.rs` returns exactly one hit — `theme.rs:614`, `visuals.hyperlink_color`. There is no web server, no HTTP client, no listening TCP socket, and no serialization framework in the tree.

Your brief asked me to name the eleven REST endpoints. For the record, so the absence is explicit rather than implied: **none of `GET /workspaces`, `GET /workspaces/{id}`, `GET /resume-candidates`, `POST /continue`, `POST /restore`, `GET /restorations/{id}`, `GET /observations`, `GET /artifacts`, `GET /moments`, `GET /tasks`, or `POST /feedback` exists in any form.** Neither does any HTTP method, path, router, or handler.

I record this as **correct, not as a defect.** No frozen document specifies an HTTP API; `ARCHITECTURE.md:223-230` and Article VII make on-device processing non-negotiable, and a listening socket carrying the Observation log — "the most sensitive data in the system by construction" (`:225`) — would be a liability rather than a feature. The two endpoints in your list that concern me are `GET /moments` and `GET /tasks`. `ARCHITECTURE.md:66-67` heads its section "Derived Views (**not primitives**)" and places Moments and Tasks there: they are *not* first-class persisted entities, but they *are* permitted as computed, cached, fully disposable views. (I originally read that line as an outright prohibition; §3.2 records the correction.) So an endpoint named `/moments` would not violate the architecture by existing — it would violate it only by returning something stored.

The code is on the safe side of that either way: `grep` finds no `Moment` type, no `Task` type, and no `StorageObjectKind::Task`. The word "moment" appears only as English prose in comments and as a UI label for a Snapshot's timestamp (`detail.rs:968` "Latest moment"). That is a vocabulary risk worth watching — it is the interface naming a Snapshot after a different concept — but it is not a concept collapse, because nothing computational is built on it and Law XVI is satisfied.

### 10.2 The real API surface: three write paths and one shared read path

What Evo actually has, and it is coherent:

**Writes** — three Unix domain sockets under the storage root, each accepting one canonical declaration and forwarding it to the capture pipeline as a `MacOSSignal`:

| Socket | Entry point | Declaration |
|---|---|---|
| designation | `designation.rs:183` `submit_designation` | `OBS-WORK-DESIGNATED` (RFC-0011) |
| grouping | `grouping.rs:213` `submit_grouping` | `OBS-WORK-GROUPED` (RFC-0012) |
| continuation surface | `continuation.rs:218` `submit_continuation_surface` | `OBS-CONTINUATION-SURFACE` (RFC-0013) |

Socket paths are derived deterministically by hashing the storage root (`continuation.rs:38-39`, FNV-1a) to stay inside the platform's socket path length limit — a small, correct detail.

**Reads** — none. The desktop does not call the daemon to read; it reads the same append-only logs directly through `CanonicalIndex` (`app.rs:154`, `cache.rs`). There is no read API because the log *is* the read API.

This is a local-first design that matches the architecture, and the asymmetry is principled: writes must be serialized through the single locked writer, reads need only the file.

### 10.3 `evo-api` presupposes two capabilities the corpus refuses to provide

The 314-line orphan crate's entire behavior is:

```rust
pub fn continue_work(&self, reference: Reference) -> Result<RestorationResult, ApiError> {
    if reference.as_str().trim().is_empty() { return Err(ApiError::InvalidReference); }
    Err(ApiError::CapabilityUnavailable)
}
pub fn resume_candidates(&self) -> Result<Vec<ResumeCandidate>, ApiError> {
    Err(ApiError::CapabilityUnavailable)
}
```

Two observations that go beyond "it is a stub."

**Its response types are shaped around a model the architecture rejected.** `ResumeCandidate` (`responses.rs:9-13`) has exactly three fields: `title`, `summary`, `reference`. `IS-0011` §4 closes the Workspace's canonical components and gives it no title — that is CONFLICT J (§3.2) — so neither display field can be read off canonical state.

**Correction, from the same re-reading recorded in §3.2.** I first wrote that `ARCHITECTURE.md:66-67` "names narrative summaries among the things Evo does not produce," and used that to call these two fields unpopulatable. That was wrong: `:66-67` and `:171` classify narrative text as a **derived, disposable, computed-on-demand view**, which is permitted. So `title` and `summary` are not forbidden — they are simply **derived values that this crate's shape presents as if they were canonical**, which is a different and lesser defect. The type is not unfillable; it is unlabelled as to provenance, and a reader cannot tell from it that both fields must be recomputable and droppable. Under Law XVI that distinction is the whole question, so the type is misleading rather than illegal.

**Its tests lock the stub in place and mis-describe themselves.** Three tests exist (`api.rs:44-69`). One asserts constructibility. Two assert `Err(ApiError::CapabilityUnavailable)` — that is, they assert that the capability does not work, so implementing it would break them. And `continue_work_rejects_invalid_reference` (`:56-62`) constructs a **valid** reference (`Reference::new("valid")`) and asserts `CapabilityUnavailable`; the `InvalidReference` branch its name advertises is never exercised. The test name is a claim the test body does not support.

**Verdict: ABSENT capability, and the crate should either be built against the current model or deleted.** Keeping it costs nothing at runtime and costs something real in comprehension, because `ResumeCandidate` is the name a reader would search for when looking for §6's missing candidacy predicate, and finding it here leads nowhere.

---

## 11. Phase 9 — Decisions, suggestions, and learning

This section is short because there is almost nothing to audit, and §0.2's legend commits me to saying **ABSENT** plainly rather than padding an absence into an analysis.

`RFC-0007 — Retrieval Contract`, `RFC-0008 — Knowledge Contract`, and `RFC-0009 — Learning Contract` are all marked **Accepted, Version 1.0**. Their implementing capabilities are, respectively: an orphan crate (`evo-retrieval`, 232 lines, §9.7), an orphan crate (`evo-knowledge`, 842 lines, §9.7), and nothing at all.

Concretely, across `crates/*/src`:

- **No suggestion exists.** Nothing proposes work to the user. Home is a complete list (§6.1), not a suggestion set.
- **No suppression exists.** A user cannot tell Evo that something is not work, or not worth resuming. The only "dismiss" in the tree (`detail.rs:539-567`) dismisses an execution report from the screen and is recorded nowhere (§9.4).
- **No feedback is captured.** There is no eleventh Observation kind for user correction; the canonical ten are closed.
- **No Decision is ever written** (§9.3): `StorageObjectKind::Decision` has zero references outside `storage.rs`.
- **No learning loop is possible**, because a learning loop needs recorded outcomes and Evo persists none (§9.4).

**ABSENT, across the board.** I want to be careful about what follows from this, because "no learning" is easy to state as a failure and it is not straightforwardly one. `CONSTITUTION.md` Article III prefers silence to confident error, and a system that suggests nothing cannot suggest wrongly; `ARCHITECTURE.md:26` lists prediction among the non-goals; and `:159-160` puts Prediction outside the engine set entirely, as an Experience-layer consumer. Evo's current silence is therefore *constitutionally safe*.

What is not safe is the combination with §6: Evo suggests nothing explicitly while implicitly asserting resumability for every Workspace it has ever formed. That is the worst of both positions — no learning, and no restraint either. The candidacy predicate in §6.3 is the minimum that makes the silence honest, and notably it needs neither a Decision log nor a learning loop to exist, which is why it is the one recommendation in this report that is actionable today.

One further note for the record, since the brief asked whether any decision is recorded anywhere: Workspace **formation** decisions are made (`workspace_decision.rs`, `formation.rs:102-113`) and are genuinely explainable at the moment they are made, because they are recomputable by replaying the log. They are simply never *recorded* as decisions. Under `ARCHITECTURE.md:39` ("State is never stored. State is derived… by replaying a confidence-scored interpretation function over an immutable observation log") that is arguably the intended design — the Decision log at `:63-64` and `:168` says otherwise. I flag the tension rather than resolving it: it is a fourth instance of the pattern this whole audit keeps finding, where `ARCHITECTURE.md` mandates a mechanism that no lower-level document ever specified.

---

## 12. Phase 10 — Noise, hardcoded assumptions, and local-first

Your brief made a specific demand here that I want to answer directly before the evidence, because it is the demand that constrains every possible fix: **no hardcoded application, domain, or category assumptions merely to hide the current examples.** The short answer is that Evo currently honors that constraint almost perfectly, and pays for it in a way nobody seems to have noticed.

### 12.1 There is exactly one noise filter, it is honest about being scope rather than judgment, and it has three rules

`macos_fsevents.rs:454-473`:

```rust
pub fn is_user_visible_path(path: &str, home: &str) -> bool {
    if !path.starts_with(home) { return false; }               // rule 1
    for component in path.strip_prefix(home).components() {
        if let Component::Normal(name) = component {
            if name.starts_with('.') { return false; }         // rule 2
            if name == "Library" { return false; }              // rule 3
        }
    }
    true
}
```

Three rules: under `$HOME`, no dot-prefixed component, no component named `Library`. That is the entire filesystem noise policy of the product.

Its doc comment (`:448-453`) is careful in exactly the way the Architectural Laws require, and I want to quote it because it is doing real work: *"This is capture scope (which files the stream considers), not a semantic judgment about importance: every path that passes is a genuine witnessed write whose meaning is unchanged."* That sentence is the difference between Law IV (Observation and Interpretation Must Never Be Confused) being honored and being violated. A filter that dropped paths because they *seemed unimportant* would be interpretation smuggled into capture. A filter that bounds stream volume is scope. The code is on the right side of that line and says so. **PASS.**

One structural detail deserves credit because getting it wrong would be silent and expensive. In `classify_event_path` (`:413-446`) the reflog check at `:420` runs **before** the visibility filter at `:438`. A git reflog lives at `<repo>/.git/logs/HEAD`, whose `.git` component would be rejected by rule 2. Had the two checks been ordered the other way, Evo would witness no commits at all — and `OBS-COMMIT-MADE` plus repository membership are the *only* signals that currently group multiple files into one body of work (§5.1). The single most load-bearing grouping signal in the product survives on the ordering of two `if` statements twelve lines apart. `macos_fsevents.rs:726` `membership_witness_precedes_content_signal` tests the adjacent ordering property; nothing tests this one directly. **PASS, untested, and worth a regression test.**

### 12.2 The filter admits whole application bundles, and this is where your "obvious internal files" symptom comes from

`grep -rn 'photoslibrary|xcodeproj|node_modules|DerivedData|\.build|Contents/|target/'` across `crates/*/src` returns **zero** matches other than Apple framework `#[link]` attributes. There is no bundle exclusion, no package-directory exclusion, no build-output exclusion, and no vendor-directory exclusion anywhere in the tree.

Consequently, every one of the following is a `FileSaved` Observation, and then an Artifact, and then a Workspace, and then a row on Home asserting resumability:

| Path | Passes because |
|---|---|
| `~/Documents/Photos.photoslibrary/database/Photos.sqlite-wal` | no dot component, no `Library` component |
| `~/dev/App.xcodeproj/project.xcworkspace/xcuserdata/…/UserInterfaceState.xcuserstate` | same |
| `~/dev/site/node_modules/lodash/fp/curryN.js` | same |
| `~/dev/rust/target/debug/incremental/…/dep-graph.bin` | same |
| any autosave, lockfile, or WAL sidecar not beginning with `.` | same |

Note what these have in common: they are *directories the operating system and its tools treat as opaque units*. A `.photoslibrary` is one document that happens to be a directory; an `.xcodeproj` is one project; `node_modules` and `target` are machine-generated caches. A build running in the background can emit thousands of qualifying writes per minute with no user present at all, and each one is eligible to become a distinct Workspace, because the co-membership signals (§5.1) do not fire for non-git paths.

**FAIL** — but I want to be careful about which clause it fails, because this is precisely where a lazy fix would violate your constraint. It does **not** fail a noise clause, because no frozen document specifies one. It fails `CONSTITUTION.md` Article III (trust over completeness) and Law VI (under-interpretation over over-interpretation) *by way of §6*: a `.xcuserstate` write becoming a resumable body of work is a false positive on Home, and §6.2 already established that a false positive there is a product failure.

And it fails them **only because §6 has no candidacy predicate.** That matters for the remedy. If Evo had the resume-candidacy step that `COGNITIVE_MODEL` Principle II describes, none of these paths would need a hardcoded exclusion list at all — a WAL sidecar written once by a background process has no continuation, no repeated engagement, and no declaration, so a general candidacy rule would exclude it *without Evo ever knowing what a `.photoslibrary` is.* An exclusion list would hide these five examples and leave the mechanism intact for the sixth. **The right fix for the noise symptom is not a noise filter.** That is the most useful single conclusion in this section, and it is the direct answer to your prohibition.

There is one narrow exception I would still argue for on capture-scope grounds rather than judgment grounds: a rule that treats *any* directory component with an extension known to the platform as a package (via `NSWorkspace`'s own `isFilePackage:`, not a literal list) is a platform fact, not a category assumption — the same kind of fact as "this path is a symlink." That distinction is available and Evo does not use it. I record it as an option, not a recommendation.

### 12.3 Two things the filter excludes that it may not intend to, and a doc/behavior mismatch

**Rule 3 is broader than its own documentation.** The doc says "the home `Library` area" — singular, implying `~/Library`. The loop rejects a component named `Library` **at any depth**. So `~/dev/MyGame/Library/`, `~/Music/Ableton/Factory Library/`, and any project directory that happens to contain a folder called `Library` are silently invisible. `macos_fsevents.rs:593` `home_library_paths_produce_no_file_signal` and `:605` `non_library_directories_still_capture` both test the top-level case only, so the depth behavior is unspecified by test and mis-stated by comment. **FAIL — documentation**, and a latent capture gap.

**Cloud-synced documents are entirely invisible.** On modern macOS, iCloud Drive, OneDrive, Dropbox, and Google Drive all mount under `~/Library/CloudStorage/…`, and iCloud Drive additionally under `~/Library/Mobile Documents/`. Rule 3 rejects all of them. For a user whose documents live in iCloud Drive — a very common configuration — Evo witnesses **no file saves at all**, and would present an empty or near-empty Home while capture reports itself healthy. Neither `daemon_status.rs` nor the Settings sheet can distinguish "capture is running and there is nothing to see" from "capture is running and the entire document tree is filtered out." **FAIL**, and unlike §12.2 this one produces silence rather than noise, which makes it harder to notice and worse for the product promise.

One positive interaction, recorded so it does not get broken later: Evo's own storage root is currently outside `$HOME` (§9.2) and the platform-correct destination `~/Library/Application Support/Evo/` would be excluded by rule 3, so Evo does not and will not witness its own log writes. The feedback loop is closed under both the current and the intended location. That is luck in the first case and correctness in the second.

### 12.4 Hardcoded assumptions: PASS, and better than PASS

`grep -rn -i 'Chrome|Safari|Firefox|Slack|Notion|Figma|VSCode|Xcode|github|google\.com'` across `crates/*/src` returns 38 hits. **Every one is a comment or a test fixture.** `adapters/macos.rs:197-198` (`"com.apple.dt.Xcode"`, `"Xcode"`) is inside `#[cfg(test)]` at `:161`; the ~30 hits in `evo-observation/src/evidence.rs` are all test fixture values; `macos_url_poller.rs:288-292, :439-451` name Chrome and Safari only in explanatory comments and one assertion string.

**There is no runtime predicate anywhere in the product that branches on an application name, bundle identifier, domain, file extension, or content category.** Not in capture, not in formation, not in restoration derivation, not in preflight, not in execution.

Two places show this was a deliberate discipline rather than an accident:

- **Browsers are identified by an accessibility role, not by identity.** `macos_url_poller.rs` `role_is_web_area` tests for `AXWebArea` — the platform's own generic role for a web view. Any application that hosts a web view qualifies; no application is named. That is the difference between a platform fact and a category assumption, and Evo is on the correct side of it.
- **A URL is recognized by scheme prefix only.** `is_http_url` is `value.starts_with("https://") || value.starts_with("http://")`. No domain list, no allowlist, no title parsing.

And one place shows the discipline extending past hardcoding into epistemic honesty, which is worth quoting because it is the strongest single piece of code I read in this audit. `witnessed_url_transition` emits a navigation signal only when the same pid was previously seen with a *different* URL:

> *"The first observation of a pid is a baseline, never a navigation: the URL may have changed while Evo was not watching that application, so emitting would claim a witness that did not occur."*

That is Law XVII (Epistemic Separation) implemented correctly at the level of a single boolean. **PASS**, unreservedly.

### 12.5 Local-first: PASS, and the dependency list is the proof

The entire 14-crate workspace depends on **three** external crates: `eframe 0.35` and `egui 0.35` (in `evo-desktop` only), and `uuid` (in `evo-observation`, `evo-workspace`, `evo-knowledge`, `evo-history`). That is the whole third-party surface.

There is no HTTP client, no HTTP server, no TLS, no `tokio`, no `async`, no `serde` in any source file, no database, no telemetry, no crash reporter, and no analytics. `grep` for `reqwest|hyper|ureq|curl|openssl|rustls|sqlite|rusqlite|sled|redb` across every `Cargo.toml` returns nothing. The only sockets in the tree are three Unix domain sockets under the storage root (§10.2). The only network-capable code is the platform call that hands a URL to the user's default browser (`macos.rs:298-325`, `NSURL` + `openURL:`), which is the operating system opening a link on the user's behalf — the same act as clicking it.

Against `ARCHITECTURE.md:223-230`, `CONSTITUTION.md` Article VII, and Law XIII: **PASS.** This is not "no network calls found"; it is "the network is not reachable from this binary." A small curiosity for the record: `evo-observation/Cargo.toml` enables `uuid`'s `serde` feature while no file in that crate mentions serde, so a serialization framework is being compiled in and never used. Harmless, and worth deleting.

### 12.6 Why Settings is almost empty — the answer is not a UI answer

Your brief lists "Settings exposes almost no behavioral configuration" as an observed defect. It is a real observation and the cause is not in the interface.

`shell.rs:303-310` states the design intent verbatim: *"a small sheet over a dimmed window, holding three facts and no invented options… Evo has almost nothing to configure, and pretending otherwise would imply the product is somewhere other than where it actually is."*

I checked whether that is true, and it is. Across `crates/evo-workspace` and `crates/evo-restoration` — the two crates that decide what a body of work *is* and what resuming it *means* — there is **not one tunable value**. `grep -rniE 'threshold|\bidle\b|timeout|session_gap|window_secs|decay|\bweight\b|half_life|recency'` across both returns four hits, and all four are doc comments **denying** that such a rule exists: `derivation.rs:554` and `:598` ("No ranking, recency, or confidence rule is applied") and `:1054` ("No guessed surface from membership, recency, or anything"). The word `score` appears only as `ConfidenceScore` (`workspace_decision.rs:28-58`), which is RFC-0012/IS-0014 evidential strength and, per W-7, explicitly not importance.

The only numeric constants in the formation path are `co_membership.rs:46` `REPOSITORY_MEMBERSHIP_STRENGTH = 0.8` and `:50` `WORK_GROUPED_STRENGTH = 1.0`, which are RFC-0012's two frozen co-membership strengths, and `confidence.rs:43/46` (`0.0`, `1.0`), which are the bounds of the confidence type. There is no session-gap duration, no recency window, no minimum-engagement count, no ranking weight — because there is no session concept, no recency logic, no engagement counting, and no ranking (§6.1, §11).

The full configuration surface of the product is: three environment variables, all documented as developer-only (`EVO_STORAGE_ROOT`, `EVO_DESKTOP_PID`, `EVO_REAL_CAPTURE_KEEP`), and no config file of any kind — `grep` finds no TOML, JSON, or plist read anywhere.

**So Settings is empty because there is nothing to configure, and there is nothing to configure because the mechanisms a user would want to influence do not exist.** The Settings sheet is the most accurate screen in the application: it is correctly reporting the size of the product's decision surface. Building a settings screen with options in it would be building the options first. Combined with §9.6 — no purge, no forget, no way to stop capture — the honest description is that Settings has nothing to show because Evo currently makes no decisions a user could disagree with, and offers no operations a user could perform on what it has recorded.

**PASS on the interface, and it is a symptom, not a defect.** It resolves upward into §14.

---

## 13. Phase 11 — The test audit

**Standing limitation, restated because it bounds everything in this section:** no test in this repository was executed (§0.3). Every verdict below is an assessment of *what a test asserts, read against the code path it exercises* — never of whether it currently passes. **NOT VERIFIED — ENVIRONMENT** applies to all 708.

### 13.1 The numbers, and why they should not reassure anyone

708 `#[test]` functions and 1,523 assertions across `crates/*/src`, distributed: `evo-observation` 107, `evo-workspace` 90, `evo-daemon` 77, `evo-desktop` 75, `evo-artifact` 74, `evo-execution` 67, `evo-restoration` 65, `evo-capture` 40, `evo-knowledge` 37, `evo-storage` 20, `evo-replay` 20, `evo-history` 17, `evo-api` 11, `evo-retrieval` 8. Plus one integration file (`evo-capture/tests/macos_event_source.rs`) and 28 `examples/`.

That is a real test suite, not a token one, and the density is highest exactly where correctness is hardest — storage framing, preflight, execution ordering, restart determinism. But note the distribution against §9.7: **65 of those tests are in orphan crates** (`evo-knowledge` 37, `evo-history` 17, `evo-api` 11, and 8 in `evo-retrieval`), plus 20 in `evo-replay`, testing code no shipped binary links. 85 tests, 12% of the suite, protect nothing that runs.

The deeper problem is what your brief asked me to look for, so I applied its question literally to every major behavior: **could this test pass while the product is still wrong?** Three times the answer was yes, and one of those three is serious.

### 13.2 Three tests that pass while the product is wrong

**(a) The ordering tests cover the execution path the desktop does not take — and the interface describes the untaken path's order.** This is a new finding and it sharpens §8.3 considerably.

`state.rs:735-796` `run_execution` branches:

```rust
if !outcome.continuation_surface().is_empty() {
    return match selection {
        Some(selection) => execute_selection(selection, locators, executor, executor),  // :764
        ...
    };
}
// no declared surface:
execute(&request, locators, executor, executor)                                          // :795
```

`execute` (`engine.rs:239-277`) calls `ordered_targets`, which pushes the Resume Point first (`:145`). `execute_selection` (`:154-211`) calls `preflight_selection`, which sorts by `ArtifactId` (`preflight.rs:236-238`, with the comment "Execution attempts follow this order."). Two live paths, two different orders.

Now the tests. `engine.rs:386 executes_resume_point_first_then_context_chain` and `:468 ordered_targets_puts_resume_point_first` both assert Resume-Point-first — and both exercise **only** `ordered_targets`/`execute`, i.e. the branch that runs when **no** continuation surface is declared. Six tests exercise `execute_selection` (`:533, :571, :599, :614, :938, :963`); the strongest of them, `execution_attempts_are_deterministically_ordered` (`:938`), asserts *determinism* — which is satisfied by an `ArtifactId` sort, because deterministic and wrong-ordered are perfectly compatible properties.

And then the part that makes it a product defect rather than an internal inconsistency: `detail.rs:401-421` `plan_line` builds the sentence Evo shows the user — "focus window X · open file Y · open URL Z" — from `evo_execution::ordered_targets(&request)`, **unconditionally, regardless of which branch will actually run.** So when a user has declared a continuation surface, which is the entire intended workflow, Evo displays the Resume-Point-first plan and then executes in `ArtifactId` order. The green tests assert the order in the sentence, not the order of the acts.

**FAIL.** It fails `IS-0019` RP-4 and `IS-0021` §25.9 (determinism is preserved, but the *derived* order is not the executed order) and `RFC-0006` Requirement 7 (Explainability — an explanation that does not describe the act is not an explanation). Per your constraint I have changed nothing; I note only that the cheapest correct fix is to make `plan_line` describe the path that will run, and the *right* fix is for both paths to share one ordering function.

**(b) `scenario_3_same_repository_many_files_stays_honest` asserts the opposite of what its name implies, and `scenario_r1` asserts the opposite of `scenario_3`.** Both are green.

`scenarios.rs:234-242` — three files under `/Users/alice/Evo/src/` → `assert_eq!(workspaces.len(), 3, "no canonical evidence links the three files")`.
`scenarios.rs:537-577` — four members of one repository → `assert_eq!(workspaces.len(), 1, "all members of one repository share one Workspace")`.

Both are correct as unit tests: they differ in whether the harness emits `OBS-REPOSITORY-MEMBERSHIP`, and the formation rule genuinely should behave differently. But `run_scenario` (`:107`) is a configuration **the shipped product never runs in** — `macos_fsevents.rs:297-315` emits repository membership on the live path. So the numbered scenario suite, which reads as the product's behavioral specification, contains a green scenario named "same repository, many files, stays honest" whose assertion is that the same repository produces many Workspaces. A reader auditing the suite for the 100 FILES guarantee would find that test, read the name, read the number 3, and draw exactly the wrong conclusion. **Not a code defect. A specification-legibility defect in the one file most likely to be read as the specification.**

**(c) `api.rs:56-62 continue_work_rejects_invalid_reference` never constructs an invalid reference.** Covered in §10.3; listed here because it is the third instance of the same class — a test whose name is a claim its body does not support.

### 13.3 Your 20 required tests, mapped against what exists

| # | Required behavior | What exists | Verdict |
|---|---|---|---|
| 1 | 100 artifacts, 4 actual work bodies | nothing at scale. Nearest: `scenario_r1` (4 members → 1 Workspace), `scenario_3` (3 files → 3). The only ≥100 construct in the tree is `bench_formation.rs:33` — see below | **ABSENT** |
| 2 | Multiple resources in one work body | `scenario_r1` (1 Workspace, 4 Artifacts); `formation.rs:592 distinct_artifacts_in_the_same_repository_join_one_workspace`; `runtime.rs:1541 work_grouped_connects_unrelated_witnessed_subjects` | **PASS** |
| 3 | Multiple sessions contributing to one work body | no Session primitive exists; the concept is carried by Snapshots. `scenario_3:255-262` (one Workspace gains a 2nd Snapshot, others unchanged); `scenario_6` (50 witnesses); `snapshot.rs:157`; `persistence.rs:1798` | **PASS by substitution** |
| 4 | Same application, unrelated work | `scenario_5_same_application_unrelated_work_stays_separate`; `scenario_r2_different_repositories_stay_separate` ("even when their files are edited through the same application") | **PASS** |
| 5 | Noise artifacts not becoming resume candidates | impossible — no candidacy concept (§6.1). The five filter tests (`macos_fsevents.rs:573, :581, :593, :605, :615`) test *capture scope*, which is a different claim | **CANNOT EXIST** |
| 6 | Meaningful work surviving across days | `scenario_6_long_lived_workspace_survives_without_identity_drift`; `selection.rs:641 three_month_surface_switch_keeps_old_work_historical`; `:688` | **PASS** |
| 7 | Continuation ≠ membership | `selection.rs:370, :422, :466, :723`; `engine.rs:841 historical_resources_are_never_passed_to_execution`; `runtime.rs:1697` | **PASS — best-covered behavior in the repository** |
| 8 | One continuation point, many resources | `runtime.rs:1697 continuation_surface_intersects_per_workspace_and_survives_restart`; `state.rs:1788`; `state.rs:1520` (label collapses 3 → "+1 more") | **PASS, partial** — tested at 2–4 members only |
| 9 | Partial restoration | `engine.rs:713 mixed_selection_partial_execution_succeeds`; `:777 unavailable_target_does_not_prevent_another_target`; `verify_selective_restoration.rs` (31 asserts) | **PASS** |
| 10 | Unavailable resources | eleven tests: `macos.rs:677, :745, :761`; `preflight.rs:309, :318, :357, :370`; `selection.rs:466`; `engine.rs:428, :571`; `detail.rs:1111 a_missing_preflight_entry_is_never_reported_as_reopenable` | **PASS** |
| 11 | Sequential restoration | `engine.rs:386, :405, :468, :938` — all on the unused path; see §13.2(a). No staging, pacing, or cap exists at all (§8.4) | **FAIL** |
| 12 | Dismissal / suppression | `detail.rs:539-567` dismisses a report from the screen and records nothing (§9.4). No canonical suppression | **CANNOT EXIST** |
| 13 | Feedback changing future decisions | nothing (§11) | **CANNOT EXIST** |
| 14 | Empty states | `state.rs:972, :1077, :1979, :2145`; `workspace_replay.rs:370`; `engine.rs:599`; `derivation.rs:1057 no_declaration_means_empty_surface`; `shell.rs:462`; `daemon/ui.rs:382` | **PASS** |
| 15 | No false resume candidates | impossible for the same reason as #5. This is the single most important absent test in your list, because §6.2 is the report's central FAIL | **CANNOT EXIST** |
| 16 | Corrupted / missing storage | nine tests, and they *discriminate* torn tail from mid-log corruption: `storage.rs:427, :457, :490, :509, :571, :591, :617, :635, :654` | **PASS — exemplary**, with §9.5's caveat |
| 17 | Migration correctness | no migration exists by design (`ARCHITECTURE.md:189`). The correct substitute is replay: `workspace_replay.rs:370` + `engine.rs:963 restart_replay_does_not_alter_derived_selection_or_execution` + `runtime.rs:1222 restart_reproduces_restoration_derivation` | **PASS by substitution**, with one gap — see below |
| 18 | Restart / recovery | `runtime.rs:1119, :1222, :1697`; `engine.rs:963`; `cache.rs:821, :883, :998 index_survives_drop_and_rebuild`; `storage.rs:529, :571`; `persistence.rs:1798` | **PASS — strongest cluster in the tree** |
| 19 | Cold start | `shell.rs:278-290` first-run marker + `:462 a_missing_home_directory_shows_the_gate_rather_than_skipping_it`; `workspace_replay.rs:370`; `state.rs:972` | **PASS, partial** |
| 20 | Realistic end-to-end capture → memory → work → resume → restore | five harnesses: `verify_real_capture.rs` (449 lines, 22 asserts), `verify_live_capture.rs` (482, 32), `verify_selective_restoration.rs` (308, 31), `verify_workspace_ui.rs` (378, 27), `verify_preflight.rs` (425, 21) | **PRESENT, not CI-reachable** |

Three entries need more than a table cell.

**On #1, the nearest artifact encodes the defect as the expectation.** `bench_formation.rs:33-34` iterates `workspace_count` over `[1_000, 10_000, 50_000, 100_000]`, building that many **distinct single-artifact Workspaces**, and its own comment calls this "the steady-state shape." The repository's performance work was therefore designed around a steady state of up to a hundred thousand one-file Workspaces. That is the 100 FILES symptom written into the codebase's own assumptions — not as a bug report, as a baseline. The benchmark has zero assertions, so nothing there can fail. I consider this the strongest single piece of evidence in the audit that §6's missing candidacy step was never an oversight in the interface but an absence in the model.

**On #17, the one migration-shaped risk that does apply is untested.** Replay makes data migration unnecessary, correctly. But `observation_schema.rs:98-108` carries a schema version, and §8.6's remedy for the window-title trap requires a v2 `window_focus` schema. There is no test that a v1 record still replays to the same Workspace identity once a v2 exists alongside it. That is the exact shape of failure Law VIII and Law VII exist to prevent, and it is the test I would write first if the v2 schema is going to be added.

**On #20, "present" is doing a lot of work.** Files under `examples/` are not compiled or run by `cargo test`. Each of these five must be invoked by hand, individually, and several require macOS Accessibility permission to have been granted to the invoking binary. So Evo's only end-to-end proofs are manual, single-shot, and unrunnable in CI or in this environment. **NOT VERIFIED — ENVIRONMENT.**

### 13.4 21 of the 28 examples assert nothing

| Asserts | Files |
|---|---|
| 32 | `verify_live_capture.rs` |
| 31 | `verify_selective_restoration.rs` |
| 27 | `verify_workspace_ui.rs` |
| 22 | `verify_real_capture.rs` |
| 21 | `verify_preflight.rs` |
| 1 | `bench_reload.rs`, `verify_resume.rs` |
| **0** | the other 21, totalling **4,133 lines** |

The zero-assert set includes things whose names promise verification: `verify_co_membership.rs` (145), `verify_collectors.rs` (183), `verify_continuation_surface.rs` (152), `verify_scenarios.rs` (303), `verify_store.rs` (38), `verify_execution.rs` (49), and `evo_doctor.rs` (264). These print. A human must read the output and decide. `qa_layout.rs` (746) and `qa_capture.rs` (435) are visual harnesses where that is appropriate; `verify_co_membership` and `verify_continuation_surface` are behavioral, and a printing harness cannot fail.

`verify_resume.rs` is the sharpest case: 122 lines, **one** assertion, and it is the file whose name matches Evo's entire product promise.

I am not going to call this a FAIL, because the classification depends on how these are used and I do not know your process. But I will state the consequence plainly: **of the eight or so behaviors this audit found broken or absent, not one would be caught by anything that runs automatically.** §13.2(a)'s ordering divergence, §12.3's cloud-storage blindness, §9.2's temp-directory root, §6's Home over-claim — each is invisible to `cargo test`, and each would be visible to a human reading `verify_*` output attentively. That is a process dependency, not a test suite.

### 13.5 What I would add, in order, and why this order

Given the constraint that I change no code, this is a recommendation and nothing more. Ranked by *defects caught per test written*:

1. **A candidacy test that cannot pass today** (#15/#5). Assert that a single `FileSaved` on `~/Documents/Photos.photoslibrary/database/Photos.sqlite-wal`, with no continuation and no declaration, does not appear on Home. It will fail, and it should — a red test is the correct representation of §6.2, and it converts the report's central finding into something the repository itself asserts.
2. **One ordering test on the surface path** (#11). Assert that `execute_selection` attempts the Resume Point first. It fails today, and it is four lines.
3. **A `plan_line` fidelity test.** Assert that the described plan equals the executed attempt order, on both branches. This is the test whose absence let §13.2(a) exist.
4. **The 100 FILES test at scale** (#1). 100 file saves across 4 repositories with membership evidence → assert exactly 4 Workspaces. It should pass today (§5.2), and it is the regression guard for every future change to formation.
5. **A schema-coexistence replay test** (#17), before any v2 schema is introduced.
6. **A capture-coverage assertion for cold start** (#19/§12.3). Assert that a path under `~/Library/CloudStorage/…` is either witnessed or reported as excluded — the current silence is what makes this failure mode invisible.

Two of those six are red on the day they are written. That is the point of them.

---

## 14. The four mechanisms, the eight concepts, and the verdict

### 14.1 One pattern explains most of this report

Ten of the eleven phases kept arriving at the same shape, from different directions, and it is not the shape I expected when I started.

**`ARCHITECTURE.md` — authority level 3, which asserts its own supremacy at line 7 ("If an RFC contradicts this document, the RFC is wrong") — names four mechanisms that sit between raw formation and the user. No document below level 3 specifies any of them. None of the four is implemented. Every one of the symptoms in your brief is downstream of their absence.**

That is a different diagnosis from "the backend is incomplete" and a much better one, because it tells you where the work is. The parts of Evo that were specified at levels 4 and 5 — the Observation log, Artifact identity, Workspace formation, co-membership, the three declaration paths, restoration derivation, preflight, execution honesty, replay, storage framing — are largely built, and built with unusual care (§14.4). The parts that exist only as sentences in `ARCHITECTURE.md` are uniformly missing. The repository's completeness tracks its specification depth almost perfectly. That is the signature of a project that implemented its RFCs faithfully and never wrote RFCs for four paragraphs of its own architecture.

### 14.2 The four

| # | Mechanism | Mandated at | Status | Symptoms it explains |
|---|---|---|---|---|
| 1 | **Workspace↔Workspace relationship link with a strength score** | `:151`, `:264` (permanent decision 5) | **ABSENT** — 17 grep hits, all prose in doc comments (§3.3, CONFLICT K) | one-resource Workspaces; empty Context Chain; no multi-resource reconstruction |
| 2 | **Disposable Experience-layer ranking / home-screen ordering cache** | `:170`, `:199`, `:159-160` | **ABSENT** — no ranking, no ordering, no candidacy (§6.1) | Home asserting resumability 24–120 times for 4 bodies of work; noise files as work |
| 3 | **Layered, progressive restoration — Layers 1, 3, 4** | `:211`, `:156-158` | **ABSENT** — Layer 1 absent, 2 split, 3 absent, 4 absent, 5 PASS (§8.2) | Continue opening everything at once with no cap, no staging, no on-demand tier |
| 4 | **The Decision log** | `:63-64`, `:128`, `:168` | **ABSENT** — `StorageObjectKind::Decision` has zero references outside `storage.rs` (§9.3, §9.4) | nothing recorded about what Evo offered, restored, or was told; no suppression; no learning |

**1 — the relationship link.** This is the mechanism by which many Workspaces become one body of work *without a hard merge*, which is exactly the operation the 100 FILES test requires and exactly the operation `:264` forbids doing destructively. Its absence is why co-membership is all-or-nothing: RFC-0012 gives two binary signals at strengths 0.8 and 1.0 (`co_membership.rs:46, :50`), and if neither fires there is no weaker register in which two Workspaces can be "probably related." `ARCHITECTURE.md:151` describes precisely that weaker register and no RFC ever defined it. This is also the missing middle term in CONFLICT N (§7.2): PRODUCT's "quietly learns which resources belong together" and RFC-0013's Competing-World rejection of every inferred grouping are reconcilable *if* the inferred result is a scored link rather than a merge — which is what the architecture said in the first place.

**2 — the ranking cache.** §3.4 established that surfacing is not forbidden but authorized, layered, and bounded, with an exact acceptance test at `:170`: *"can it be silently dropped and regenerated without the user noticing anything beyond a brief delay?"* A resume-candidacy predicate and a Home ordering both pass that test trivially, because both are pure functions of the log. `COGNITIVE_MODEL` Principle II supplies the concept ("open engagements… Openness is a property of an engagement. It is not a separate object.") and Law XVI independently fixes its permitted form ("relationships, derived values, or transient computational state"). So this one needs **no contract amendment at all** — which is why §6.3 remains the single actionable recommendation in this report.

**3 — layered restoration.** `:211` is unusually specific: *"Layer 1, context — the Snapshot's summary, shown instantly. Layer 2, the single primary artifact, opened first. Layer 3, supporting artifacts, **capped to a small number**. Layer 4, reference artifacts, opened last or on demand. Layer 5, historical context, never opened automatically."* Layer 5 is implemented and correct (`selection.rs:142, :179-181` — `historical` is never preflighted and never executed). Layers 1, 3, and 4 do not exist, and `grep -rn '\.take\(|MAX_|cap\b|limit'` across `evo-execution` and `evo-restoration` returns **zero hits**, so the words "capped to a small number" have no counterpart anywhere in the code. This is the whole of Scenario F's **FAIL**: asked whether Evo reopens all twelve resources at once, the answer from the code is yes, and the architecture said no in a sentence that names the cap.

**4 — the Decision log.** Every stage of the architecture's own pipeline terminates in it (`:128`: "Restored Workspace + Decision log entry"), it is one of the five primitives (`:63-64`), and it is persisted by design (`:168`). Nothing writes one. The consequence chain is in §9.4: no record of what was offered, no record of what was restored, no record of what the user dismissed, therefore no possible learning loop and no possible explanation after the fact.

**One clause that resolves more than it appears to.** `ARCHITECTURE.md:200`: *"A correction from the user is evidence. It updates confidence on an Attachment like any other signal. **It does not require its own subsystem.**"* This dissolves most of what §11 reported as a missing learning apparatus. Evo already implements correction-as-evidence twice — `OBS-WORK-DESIGNATED` corrects which resource is the Resume Point, `OBS-WORK-GROUPED` corrects which resources belong together — and both work. What is missing is not a learning subsystem; it is **one more declaration of the same kind, meaning "this is not work I will return to."** Under `:200` that is an ordinary evidence write, not an eleventh architectural concept. It does need a canonical Observation kind, and the ten are closed, so it needs an RFC — but a small one, and shaped exactly like RFC-0011 and RFC-0013, both of which are already accepted precedents for "a witnessed user act about their own work."

### 14.3 Time To Productive — the metric whose absence let all four go unnoticed

`ARCHITECTURE.md:217`: *"**The one product metric:** Time To Productive — wall-clock time from the Resume click to the primary artifact being open and in the foreground. **Every architectural decision in this document exists, ultimately, to make that number smaller.**"*

`grep -rniE 'time_to_productive|\bttp\b|productive'` across `crates/` returns **zero hits.** Nothing measures it, nothing records it, nothing could compute it retroactively, because §9.4 established that no execution is ever persisted — there is no timestamp for the Resume click and none for the target reaching the foreground.

(Precision note, caught in the verification pass: my first draft said "across the entire repository." That is wrong — `ARCHITECTURE.md:252` defines TTP in its glossary and `docs/design/UI-REVIEW-0001.md:66` discusses it at length. The metric is well documented and entirely unimplemented, which is a sharper statement than the one I originally made.)

**ABSENT**, and I want to be careful about how much weight to put on that, because `:213` is explicit that the *deeper* question is deliberately not architected: *"Whether restoration truly helped the user resume productive work is a harder question the architecture does not claim to answer with certainty — it is something to learn from real usage, not something to architect false confidence around."* So the architecture is not asking for a satisfaction score. It is asking for one wall-clock duration, which `:213` says in the same breath *can* be known "mechanically and honestly."

The reason this belongs in the conclusion rather than in §8 is that TTP is the one measurement under which all four missing mechanisms become visible as costs rather than as omissions. A Home with 118 rows raises time-to-productive because finding the right row is the slow part. Opening twelve resources unpaced raises it. An `UNAVAILABLE` window that could have been reopened as a document raises it. And a system with no Decision log cannot tell you that any of that happened. Evo is currently unable to observe the only number its architecture says every decision exists to reduce — which is a fair description of why four specified mechanisms could go missing without anything in the repository objecting.

### 14.4 What is actually true about Evo, stated as plainly as the failures

Your closing rule was *"do not optimize for making the repository look clean."* The symmetric obligation is not to make it look worse than the evidence supports, so this list is part of the finding, not a courtesy.

The specific things your brief suspected of being sloppy are, on the evidence, clean:

- **No model decides identity or attachment.** No inference library, no embeddings, no LLM call anywhere. `:264` constraint 7 is honored absolutely, because there is nothing in the binary that could violate it.
- **No full-history scan.** `cache.rs:385-410` is index-narrowed; `formation.rs:44-53` documents it as deliberate (§1.3). I expected a violation here and did not find one.
- **Nothing fails silently in restoration.** Five explicit statuses, each carrying the executor's own reason verbatim to the screen (`detail.rs:587-598`), eleven tests on the unavailable paths, and `preflight.rs:318` explicitly named `renamed_or_moved_file_is_unavailable_with_no_fuzzy_recovery` — Evo refuses to guess at a moved file rather than opening the wrong one.
- **Partial restoration is first-class**, exactly as `:215` requires (`engine.rs:713, :777`).
- **Repository co-membership genuinely works** (§5.2). My "100 files → 100 Workspaces" claim was overstated and §1 corrects it: for git-tracked files the grouping is real and tested at four members.
- **Storage is careful.** `append(true)` plus `sync_data()` per record, a single-writer pid lock, and a framing parser that *discriminates* a torn tail from mid-log corruption rather than treating both as damage (`storage.rs:130-180`, nine tests).
- **Replay works** (§9.7 correction) — 542 lines, wired, tested at four call sites, satisfying Law VII.
- **The reflog check precedes the noise filter** (§12.1), which is the only reason commit evidence exists at all.
- **The URL collector refuses to claim an unwitnessed transition** (§12.4) — the single most epistemically careful piece of code in the repository.
- **Local-first is structural, not aspirational** (§12.5): three external crates, no network reachable from the binary.
- **Settings is honest** (§12.6). It is empty because the decision surface is empty, and it says so in its own doc comment.

The failures in this report are not carelessness. They are, with two exceptions, a single specification gap with four faces.

The two exceptions — the findings that are independent of the pattern and that I would treat as urgent on their own terms — are **§9.2**, the canonical log defaulting to the OS temporary directory in four separate places, and **§13.2(a)**, the interface describing one restoration order while executing another.

### 14.5 The eight concepts: which ones actually stayed apart

Your brief named eight concepts that must not collapse. Verdict on each, from the code:

| Concept | Status |
|---|---|
| **Observation** | **Distinct and correct.** `evo-observation`, ten closed canonical kinds, append-only, witnessed-only. |
| **Artifact** | **Distinct and correct.** `evo-artifact`, identity as a provisional derived hypothesis, replayable. |
| **Session / Moment** | **ABSENT as a concept.** No primitive, no type, no boundary. Its work is done implicitly by Snapshot, which is why required test #3 could only be answered by substitution (§13.3). Not collapsed into something else — simply never introduced. `ARCHITECTURE.md:66-67` permits it as a derived view; nothing derives it. |
| **Work identity / body of work** | **Formally distinct, behaviorally collapsed into Artifact.** This is the important one. `Workspace` and `Artifact` are separate types with separate identities and separate logs — the type system is clean. But in the data, whenever neither co-membership signal fires, a Workspace holds exactly one Artifact forever, so the two concepts are *distinct in the model and identical in the record*. `bench_formation.rs:33-34` treats up to 100,000 single-artifact Workspaces as "the steady-state shape" (§13.3), which is the repository conceding the point in its own performance assumptions. **The collapse you suspected is real, and it is in the data, not in the code.** |
| **Continuation** | **Distinct, and the best-defended concept in the system.** RFC-0013's surface is genuinely not membership: `selection.rs:370, :422, :723` and `engine.rs:841` all assert that a resource can belong to a Workspace and be excluded from restoration. Five tests specifically prove membership ≠ continuation. **PASS**, and notably this is the concept your brief most suspected of collapsing. |
| **Resume candidate** | **ABSENT — collapsed into work identity.** Home is `workspaces.iter().map(...)` with no filter, sort, or take (`app.rs:255-270`), so *"Evo witnessed this"* and *"this is work you may want to resume"* are the same claim, made by the same list. This is §6.2 and it is the report's central FAIL. |
| **Restoration plan** | **Distinct but split in two.** `RestorationSelection` (surface path) and `ExecutionRequest`/`ordered_targets` (designation path) are two plan shapes with two different orders, and the interface describes one while running the other (§13.2(a)). One concept, two incompatible representations. |
| **Restoration execution** | **Distinct and honest, but never persisted.** `ExecutionReport` with five statuses is a real, well-tested object that exists only as a value in memory (§9.4). It is a concept Evo holds and never remembers. |

Three clean, one absent by omission, one absent by collapse, one collapsed in data, one split, one unpersisted.

### 14.6 Every symptom in your brief, traced to a backend cause

You asked me not to assume these were frontend defects. None of them is.

| Your observation | Actual cause |
|---|---|
| Home shows huge numbers of individual files as bodies of work | §6.1 no candidacy step + mechanism 2 absent; §12.2 no package exclusion admits build and library internals |
| Opening many files makes many things appear resumable | same; Home is a total 1:1 map of formation output |
| Some Workspaces contain only one resource | §5.1 only two co-membership signals exist and neither fires for URLs, window titles, or non-git files; **mechanism 1 absent** is the specified answer. **And `IS-0011:72` defines a Workspace as requiring *multiple* Artifact histories** (CONFLICT A), so these objects do not satisfy the definition of the primitive they are instances of |
| A Workspace frequently has only one "focused window" continuation | §7.4 all three continuation signals are user declarations; window focus is the highest-volume Observation, so it dominates whatever surface exists |
| Restore/Continue often just focuses an already-open window | §8.6 the `WindowTitle` locator is the most frequently available locator, for the same volume reason |
| If that window isn't open, the resource is UNAVAILABLE | §8.6, and this is the most fixable high-value defect in the report: `macos_event_source.rs:356-357, :452, :648-659` **witnesses the owning pid**, and `:462-478` emits only `{subject, observed_at}`, discarding it. Evo threw away the evidence that would have let it reopen the application. Law II:52. Remedy is an additive schema v2 (`observation_schema.rs:98-108`) — no contract amendment, no data migration, replay handles the rest |
| Little evidence Evo can reconstruct multi-resource context | **mechanisms 1 and 3 absent**; Context Chain is hard-empty, Layers 1/3/4 do not exist |
| Settings exposes almost no behavioral configuration | §12.6 — accurate reporting of an empty decision surface, not a UI defect |
| Captured artifacts include obvious noise and internal files | §12.2 no package/build/vendor exclusion; and the correct fix is candidacy, not a noise list (your own constraint forces this) |
| Risk of confusing "Evo witnessed this" with "work you may want to resume" | §6.2 — realized, not merely risked. This is the FAIL to fix first |

### 14.7 Verdict

| Phase | Verdict |
|---|---|
| 1 — intended model | reconstructed; **5 new ARCHITECTURAL CONFLICTS** (J §3.2, K §3.3, L §3.4, M §3.5, N §7.2) on top of A–I from the RFC/IS layer (§3.6). **L resolves** by layer scoping — ranking is authorized as a disposable Experience-layer cache; **M resolves into a FAIL**, not a contradiction — the layers are specified and three of five are unreachable |
| 2 — traceability | 11 PASS, 6 FAIL, 2 ABSENT, 1 split across 20 contract lines |
| 3 — work identity / 100 FILES | **PASS for git-tracked files, FAIL for URLs, window titles, and non-git files** |
| 4 — resume candidacy | **FAIL** — the concept does not exist; Home over-claims by construction |
| 5 — continuation | **PASS on distinctness**; **FAIL** on the surface deciding execution with no clause authorizing it; **CONFLICT N** unresolved |
| 6 — restoration | Layer 5 **PASS**, Layers 1/3/4 **ABSENT**; Scenarios A **FAIL-partial**, B **PASS**, C **PASS**, D **PASS-with-caveat**, E **PASS**, F **FAIL** |
| 7 — storage | framing and durability **PASS**; **FAIL** on the temp-directory root; Knowledge, Decision, execution history, purge all **ABSENT** |
| 8 — API | no HTTP API, and that is **correct**; `evo-api` is an **ABSENT** capability with misleading tests |
| 9 — decisions / learning | **ABSENT across the board**, constitutionally safe in isolation, dangerous in combination with phase 4 |
| 10 — noise / hardcoding / local-first | hardcoding **PASS** (better than PASS); local-first **PASS**; noise **FAIL** in two opposite directions — packages admitted, cloud storage excluded |
| 11 — tests | 708 tests, real coverage where specified; **3 tests pass while the product is wrong**; **4 of your 20 cannot exist**; 21 of 28 examples assert nothing |

**Overall: Evo is a carefully built implementation of an incompletely specified architecture.** It does not fabricate, does not guess, does not call models, does not phone home, and does not fail silently. What it does is assert, on its primary screen, that everything it has ever witnessed is work you might want to resume — and that single unmade distinction is what produces almost every symptom you observed.

**What I did not do, deliberately.** No file under `crates/` was modified. I did not add a resume-candidacy predicate, did not cap restoration, did not move the storage root, did not fix the ordering divergence, did not write an exclusion list, and did not resolve CONFLICT N or CONFLICT J. Three of those are blocked on decisions that are yours; the rest are blocked on your review of this report, which is what you asked for.

**The four decisions I need from you, in dependency order.**

1. **May Home omit a Workspace?** (§6.4) Everything in phase 4 depends on this one word. `:170` authorizes *ordering* explicitly; *omission* is authorized nowhere and forbidden nowhere. Nothing else can be built until this is answered, and it is one sentence.
2. **The storage-root migration** (§9.2) — copy-forward, replay-forward, or accept the loss and tell the user. The destination is not in question; Law VIII means the transition is.
3. **CONFLICT N** (§7.2) — does PRODUCT's "quietly learns" survive RFC-0013's Competing-World rejection? I believe mechanism 1 is the reconciliation, but that is an architectural judgment, not an audit finding, and under your own precedence rule PRODUCT outranks the RFC.
4. **The `window_focus` schema v2** (§8.6) — the highest value-per-unit-risk change available, and the only one of the four that is purely additive.
