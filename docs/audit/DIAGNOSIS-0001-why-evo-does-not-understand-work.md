# DIAGNOSIS-0001 — Why Evo Does Not Yet Understand Work

**Status:** Diagnosis. No production code changed.
**Method:** the live derivation code run against this machine's real canonical
Observation log — 1245 Observations, 490 witnessed subjects, spanning 2026-08-16
to 2026-08-21.
**Harness:** `crates/evo-daemon/examples/diagnose_real_history.rs` (read-only;
added by this diagnosis) and the pre-existing `replay_real_history.rs`.
**Scope:** everything from Observation to what Restoration would open.

This document exists because the mission brief requires a written diagnosis
before the architecture changes. It reports what the current code *actually
does* on real evidence, not what its documents say it does. Where the two
disagree, the measurement wins.

---

## 0. Executive finding

Formation has largely been fixed. Importance has not, and identity never was.

| | before this generation | now (measured) |
|---|---|---|
| Workspaces from 1245 Observations | 490 | **9** |
| single-resource Workspaces | most | **0** |
| members Restoration holds back | 0% | **83.2%** |

Those three numbers say the grouping rewrite worked. The product is still wrong,
and the reason is narrower and more specific than "grouping is bad":

1. **Restoration importance is still an attention measurement.** Evo will open
   `‎WhatsApp` and label it *"where you left off"* for an afternoon of drafting
   RFC-0011, and will auto-open `Spotify Premium` as a place a job-assignment
   analysis was happening. The title layer was fixed; the resume-point layer
   reads the same signal the title layer was fixed for reading.
2. **Identity is unsettled, and every layer downstream inherits the damage.**
   One browser tab becomes three members of one Workspace. One Google Meet call
   becomes two separate Workspaces. `pipeline.py — candidate 2` splits off from
   `pipeline.py` and is thrown away entirely.
3. **Evo is never silent.** 9 of 9 derived Workspaces produce a *complete*
   restoration plan with a next step — 100%. There is no reachable code path
   by which Evo says "I do not know where this resumes."
4. **Membership rests on one signal.** 94 of 95 membership decisions were
   decided by `Interleaved` — "used together in the same sitting." The
   four-kind weighted affinity graph is, on real evidence, a single-kind
   temporal-proximity model: exactly the heuristic the brief forbids.
5. **Importance dilutes as a body of work grows.** A 3-member Workspace opens
   3 resources. A 29-member Workspace opens 1. The gate is a *share of a total
   that grows with membership*, so the bigger and more real the body of work,
   the less of it Evo can justify opening.
6. **There is no continuation summary.** The restoration contract asks for a
   concise workspace-level statement of what the person was doing. What exists
   is one of two compile-time constant strings.

**No fake data reaches production.** Every `Design Brief — Canvas` /
`Research Notes — Reading List` / `Spotify` / `WhatsApp` string in the
repository is inside a `#[cfg(test)]` module, a doc comment, or a diagnostic
example — verified file by file (§6). Those names appeared on Home because they
were *genuinely observed windows on this machine*. The root cause was never a
seeded fixture; it was that Evo could not tell an incidental window from work.

---

## 1. The measured pipeline

```
observation.log (1245 records, append-only, canonical)
   │
   ├─ evo_engagement::interpret        → 1245 Acts, 0 Declarations
   │     ActCharacter from frozen schema name only
   │
   ├─ IdentityIndex::learn             → 490 subjects → 427 resources
   │     ✗ DEFECT 2 — folds within one naming system only
   │
   ├─ segment_episodes                 → 61 sittings (presence_horizon 20m)
   ├─ AttentionLedger::build           → 4h 18m attributable attention
   │     credited only for intervals ≤ 10m, all-or-nothing
   │
   ├─ AffinityGraph::build             → 4 weighted evidence kinds
   │     ✗ DEFECT 4 — 98.9% of real decisions are Interleaved alone
   │
   ├─ cluster() → average_linkage()    → 32 clusters
   ├─ is_significant()                 → 9 claimed, 23 refused
   │     ✗ DEFECT 5 — Return is an any-member test
   │     ✗ DEFECT 6 — ≥2 members kills real single-resource work
   │
   ├─ assign_roles()                   → Continuation/Primary/Supporting/…
   │     ✗ DEFECT 1 — leading_share is still attention
   │     ✗ DEFECT 3 — max_primary=3 against a size-dependent share gate
   │
   ├─ name_the_continuation()          → always promotes exactly one
   │     ✗ DEFECT 7 — makes silence unreachable
   │
   ├─ project_engagements()            → 9 Workspaces, 95 Attachments
   └─ derive_restoration_plan()        → 9 Complete, 0 Insufficient
         ✗ DEFECT 8 — NextStep is a constant string
```

Nothing derived is persisted. `workspace.log` and `restoration.log` are no
longer written or read; the only remaining references are assertions that they
must *not* exist (`runtime.rs:1062`, `workspace_replay.rs:243`). Replay
equivalence therefore holds by construction, and the earlier generation's
490 fake Workspaces cannot come back. Two stale files from that generation do
still sit in the real storage root (§6.3).

---

## 2. The real-data report (mission Task 2, items 1–14)

### 1. Raw Observation count — 1245

| count | schema | share |
|---|---|---|
| 783 | `OBS-FILE-SAVED` | 62.9% |
| 302 | `OBS-WINDOW-FOCUS-GAINED` | 24.3% |
| 160 | `OBS-URL-NAVIGATED` | 12.9% |

Zero declarations of any kind. Every RFC-0011/0012/0013 mechanism —
designation, `WorkGrouped`, `RepositoryMembership`, Continuation Surface — has
never fired once on real evidence. **Every product truth that depends on the
user telling Evo something is, in practice, dead code.**

Provenance context is empty on 100% of Observations: no Observation carries
which application, process, window, or tab produced it. This single gap is what
makes Defect 2 unrepairable downstream (§3.2).

### 2. Unique Artifacts — 490 witnessed subjects → 427 resources

63 names (12.9%) folded away. 13 resources needed several names to identify;
the worst are the fan-outs identity was written to fix:

| names | resource |
|---|---|
| 29 | `https://www.google.com/search?q=s+%3D+s%5B1%3A%5D.strip%28%29&gs_lcrp=…` |
| 13 | `01_CODE_line_by_line_walkthrough.md.pdf` (`– Page N of 27`) |
| 7 | `05_Round2_GenericInterview_Prep_FULL.pdf` |
| 3 | `Job assignment analysis and deadline - Claude` (`- 1.2 GB`, `- 1.5 GB`) |

That the 3-name Claude fold is *incomplete* is Defect 2.

### 3. Workspaces — 9

427 resources → 9 Workspaces = **0.0211 Workspaces per observed resource**.
23 further clusters cohered and were then refused as work.

### 4. Workspace size distribution

| Workspaces | members |
|---|---|
| 2 | 3 |
| 1 | 4 |
| 1 | 5 |
| 1 | 8 |
| 2 | 9 |
| 1 | 25 |
| 1 | 29 |

### 5. Single-artifact Workspaces — **0 of 9 (0.0%)**

CASE 1 passes. This is the single clearest success of the current generation.

### 6–9. Resource kind

Classified by `SubjectShape` — Evo's own structural vocabulary, derived from the
frozen schema, naming no application, domain, or extension.

| resources | share | kind |
|---|---|---|
| 325 | 76.1% | filesystem path — **file** |
| 53 | 12.4% | addressable location — **browser resource** |
| 49 | 11.5% | named surface — **application window** |
| 0 | 0.0% | opaque identifier — **repository evidence** |

By how the acts were witnessed: **102 resources (23.9%) ever had a human act on
them; 325 (76.1%) were only ever witnessed as unattributable change.** Three
quarters of the corpus is machine churn. No repository evidence exists at all,
which means the previous generation's central grouping signal — repository
membership — has never once been available.

### 10. Workspaces with meaningful continuation — **9 of 9 (100%)**

Every Workspace yields `Complete` + a next step. Presented as a success metric
this looks perfect. It is Defect 7: the number is 100% because insufficiency is
unreachable, not because the evidence was strong.

### 11. Resources opened automatically — **16**

### 12. Resources observed but not opened

- **79 of 95 members held back (83.2%)** — the membership/importance split is
  structurally real and working.
- **332 of 427 observed resources (77.8%) are in no Workspace at all.**

Restoration size against Workspace size:

| members | opened | share |
|---|---|---|
| 3 | 1 | 33% |
| 3 | 3 | 100% |
| 4 | 1 | 25% |
| 5 | 3 | 60% |
| 8 | 3 | 38% |
| 9 | 2 | 22% |
| 9 | 1 | 11% |
| 25 | 1 | 4% |
| 29 | 1 | 3% |

The brief asks that restoration size not scale with Workspace size. It does not
— but for the wrong reason, and in the wrong direction (§3.3).

### 13. Why each resource is a member

| memberships | strongest witnessed tie |
|---|---|
| 94 | `Interleaved` — used together in the same sitting |
| 1 | `Lexical` — named for the same thing |
| 0 | `Recurring`, `Structural`, `Declared` |

### 14. Why each resource is or is not a restoration target

| members | role | opens? |
|---|---|---|
| 9 | `Continuation` | **yes** |
| 7 | `Primary` | **yes** |
| 35 | `Supporting` | no |
| 22 | `Reference` | no |
| 22 | `Context` | no |

---

## 3. The defects, each grounded in real derived output

### 3.1 — DEFECT 1: restoration importance is still an attention measurement

`engagement.rs::assign_roles` gates Primary on
`leading_share >= primary_attention_share` where
`leading_share = attention_share × locality`. `name_the_continuation` then
promotes the most recently attended Primary to `Continuation`, which
`derive_restoration_plan::sole_continuation` takes at face value as the resume
point.

Real output, the body of work correctly titled
`RFC-0011-continuation-evidence-contract.md — Untracked`:

```
OPEN  [Continuation] ‎WhatsApp
        attention 8m, locality 0.75, lead 0.387, revisits 4
OPEN  [Primary] RFC-0011-continuation-evidence-contract.md — Untracked
        attention 3m, locality 1.00, lead 0.175, revisits 2
OPEN  [Primary] evo
        attention 2m, locality 1.00, lead 0.152, revisits 2
```

WhatsApp holds 8 of the work's 17 minutes, cleared the 0.15 gate at 0.387, and
was attended last. **Evo will open WhatsApp and tell the user that is where they
left off drafting an RFC.** The RFC itself — locality 1.00, the thing the work
is named after — ranks below it.

The same class of failure, in the Claude body of work:

```
OPEN  [Primary]      Spotify Premium   attention 20m, locality 0.86, lead 0.290, revisits 7
OPEN  [Continuation] claude.ai/chat/…  attention 13m, locality 1.00, lead 0.232, revisits 6
```

And in the interview-preparation body of work, `opens 3/3`:

```
OPEN  [Primary] Meet - kxc-vads-gdp – Microphone recording
```

A video call that has ended is a restoration target. **You cannot resume a
call.** Nothing in the model distinguishes a resource whose state persists (a
file, a document, a chat thread) from one that was a live event.

`engagement.rs:1017` records that this exact failure — an RFC afternoon titled
"‎WhatsApp" — was the reason `choose_title` was rewritten. It was fixed in one
place. The resume point reads the same signal and was not.

**Root cause.** The model has exactly one notion of importance — measured
attention, discounted by locality — and asks it to answer two different
questions: *what is this work about* and *where does it resume*. Locality
already encodes "this resource is not shared with other work," which WhatsApp
partly fails (0.75) and Spotify partly fails (0.86); but the attention term is
large enough to carry them anyway. No amount of retuning fixes this, because
duration of attention genuinely does not distinguish a companion from a
workpiece. A messaging window open beside an RFC accrues real minutes.

### 3.2 — DEFECT 2: identity is unsettled, and no later layer can repair it

**One tab, three members of one Workspace.** The Claude body of work contains:

- `Job assignment analysis and deadline - Claude - 1.2 GB` — Primary, 13m
- `Job assignment analysis and deadline - Claude` — Supporting
- `https://claude.ai/chat/0743595c-ef5d-466f-8cac-90a462eccd57` — Continuation, 13m

and a *fourth* identity, `… - Claude - 1.5 GB`, was refused as work entirely
(1m, 3 human acts). One browser tab, four identities, three roles, two
outcomes, its attention split four ways. `- 1.2 GB` did not fold because
`min_field_values: 3` requires the stem to have been witnessed under ≥3 names
and it was seen under 2 — a threshold about corpus size deciding a question
about identity.

**One meeting, two Workspaces.**

- Workspace 4: `meet.google.com/kxc-vads-gdp` — 9 members, 34m, 1 sitting
- Workspace 7: contains `Meet - kxc-vads-gdp – Microphone recording` — 16m

Same call, same meeting code, two Workspaces. `IdentityIndex` splits names into
stem + attributes using punctuation. The address stem is
`https://meet.google.com/kxc-vads-gdp`; the window stem is `Meet`. They are
different naming systems and **can never merge under the current rule, at any
threshold.** The evidence that would link them — which application or tab
produced each Observation — is absent from 100% of real Observations.

**A workpiece split off and discarded.** `pipeline.py — candidate 2` is a refused
one-member cluster (1m attention, 4 revisits, 5 human acts). `pipeline.py` is the
title of the healthiest body of work Evo derived (29m, 3 sittings). The same file
in the same editor, held apart by identity, then discarded by the ≥2-member gate.
**Real work, with human acts on it, is invisible to the product.**

**Volatile state survives into names Home shows.** `Job assignment analysis and
deadline - Claude - 1.2 GB` (a memory readout) and `Bhavitsaini_Resume.pdf – 1
page` (a page counter) are Workspace titles. Mail fragments four ways:
`Inbox (2,886)`, `Inbox (2,884) - 878 MB`, `Inbox (2,871) - 942 MB`,
`Inbox (2,877)` — an unread count and a memory readout, both per-sighting state
the folding rule exists to remove, each surviving because its stem was seen
under too few names.

**Root cause.** Identity is inferred from the *shape of name strings*, using a
corpus-size threshold, within a single naming system. Three separate failures
follow: incomplete folding (thresholds), impossible folding (incommensurable
stems), and no folding evidence at all (empty provenance). Identity has to be
settled before anything is measured, and it is not.

### 3.3 — DEFECT 3: importance dilutes as a body of work grows

`attention_share` is a share of the *engagement's* total attention.
`primary_attention_share` is a fixed 0.15. In a 29-member Workspace, reaching
0.15 requires holding a seventh of all attention — so in practice no one does,
and only the `Continuation` (promoted unconditionally) opens.

| members | opened |
|---|---|
| 3 | 3 (100%) |
| 25 | 1 (4%) |
| 29 | 1 (3%) |

CASE 5 asks for a 20+-resource body of work where the resume-necessary
resources auto-restore and the rest stay available. Evo opens **one**. The
23-member PlayStation Workspace and the 29-member Pluely Workspace each open a
single resource; the 3-member interview Workspace opens everything.

The `max_primary: 3` cap is doing nothing on large bodies of work — the share
gate binds first. Restoration size is bounded, but by dilution rather than by a
judgement about what is needed to resume.

### 3.4 — DEFECT 4: membership rests on one signal, and it is the forbidden one

94 of 95 memberships were decided by `Interleaved`. One by `Lexical`. Zero by
`Recurring`, `Structural`, or `Declared`.

`AffinityGraph` weights four kinds (0.45 / 0.25 / 0.20 / 0.10). Three of them
contribute essentially nothing on real evidence:

- `Declared` — 0 declarations exist.
- `Structural` — needs a witnessed container; no repository evidence exists.
- `Recurring` — needs `min_co_episodes: 2`; most real resources are seen in one
  sitting (`revisits=1` dominates every large Workspace).
- `Lexical` — near-zero across heterogeneous real work, exactly as the brief
  predicts.

So the production model is: **resources are related if they were used close
together in time.** The brief forbids this in as many words. The weighted graph
reads as a principled multi-evidence model and behaves as a single-signal one.

What it produces when unchecked: a **119-member cluster** of JetBrains caches,
Music Library, and Photos Library files, plus clusters of 54, 54, 42, and 18
more of the same. They are refused afterwards for having 2s of attention — but
the graph itself has no ability to tell churn from work. It is rescued by the
significance gate, not by its own evidence.

### 3.5 — DEFECT 5: significance is an any-member test

`is_significant` requires *some* member with ≥ `min_revisits` human acts. In the
23-member PlayStation Workspace nearly every member has `revisits=1`; one Google
search reached 2 and carried the whole cluster. The doc-comment intent —
"thirty things opened once each in one afternoon produce no return" — is
defeated because Return is satisfiable by any single member.

Consequences on real data: `vidbox.vc/watch/tv` (6m of TV) and `Pluely` (a
download and install: Finder and System-Settings windows for `Applications`,
`Recents`, `Privacy & Security`, `Sound`, three `.dmg` files, four
`Unconfirmed *.crdownload` temporaries) are both presented under
**"Continue working on."**

### 3.6 — DEFECT 6: the ≥2-member gate discards real single-resource work

`participants.len() < 2` returns false immediately. CASE 2 asks that real
single-resource work be representable. Refused clusters that are plainly real:

| refused | attention | human acts |
|---|---|---|
| `pipeline.py — candidate 2` | 1m | 5 |
| `Job assignment analysis and deadline - Claude - 1.5 GB` | 1m | 3 |
| `Automation Engineering Intern Application – Bhavit Saini` + `Invitation: Bhavit- round 2 @ Thu Aug 20` | 23s | 3 |
| `IS-0001-Observation.md — Modified` + `Inbox (2,884) - 878 MB` | 34s | 2 |
| `Visual Studio Code` + `sales_lines` | 40s | 3 |

The job-application pair is the sharpest case: the interview-preparation
Workspace *was* claimed, and the application itself and its calendar invitation
— the same body of work — were refused. Real work is being split between a
claimed Workspace and a discarded cluster.

Also visible: `Downloads`, `data`, `rfc`, `AirDrop`, `loginwindow` as
one-member clusters. Generic single-word window titles are being treated as
resources when they are containers or system surfaces. The corpus has no way to
say "this name identifies nothing."

### 3.7 — DEFECT 7: Evo cannot be silent

Product truth 15: *Evo must prefer silence over confidently wrong restoration.*

`name_the_continuation` promotes exactly one Primary to `Continuation` whenever
any Primary exists. `sole_continuation` then always finds exactly one candidate,
so `derive_restoration_plan` always returns `Complete` with a `NextStep`. The
`Insufficient` arms — `0 candidates`, `multiple candidates`, and the carefully
written `MULTIPLE_CANDIDATES_REASON` / `NO_CANDIDATE_REASON` /
`NEXT_STEP_MISSING_REASON` strings — **are unreachable for any Workspace the
projection emits.**

Measured: 9 of 9 `Complete`, 9 of 9 with a next step, 0 `Insufficient`.

The machinery for honest insufficiency exists and is well built. Nothing can
reach it. Combined with Defect 1, this means Evo will confidently open WhatsApp
and assert it is where the RFC work resumes, with no mechanism to hesitate.

### 3.8 — DEFECT 8: there is no continuation summary

The restoration contract asks for a *concise workspace-level continuation
summary*. What is produced is one of two compile-time constants:

```rust
const NEXT_STEP_CONTINUATION_DESCRIPTION: &str =
    "This is the resource you were last working in as part of this work, so \
     this is where the work continues.";
const NEXT_STEP_DESIGNATED_DESCRIPTION: &str =
    "The user designated this resource as the work to continue.";
```

Every Workspace, forever, shows the same sentence. This honors "do not invent
summaries" by saying nothing at all. The evidence to say something real is
already measured and thrown away: sittings, span, attention, roles, the shape of
return. `Engagement::explanation()` already produces
`"17 min of attention across 7 resources in 3 sittings"` — grounded, honest,
non-fabricated — and Restoration does not use it.

### 3.9 — DEFECT 9: members with no witnessed tie to the work

`vidbox.vc/watch/tv` includes `[Context] locality=0.00 — https://ai-arena.twocc.in/`
— a resource active in **no** sitting of that work, still a member. Product
truth 17: *same time period ≠ same workspace.*

Machine churn also reaches membership, correctly demoted to `Context` so it never
opens, but still listed as what the work consisted of: four JetBrains
`event-log-data/logs/FUS/*-eap.log` files in the `pipeline.py` work; Photos
Library `photolibraryd/caches/clientservertransaction/*` and Spotlight `tmp.*`
in the PlayStation and Pluely works.

### 3.10 — DEFECT 10: a body of work can render an empty "You were working in"

Workspace 4 (`meet.google.com/kxc-vads-gdp`) has a `Continuation` and **zero**
`Primary` members. `replay_real_history.rs:103` and the desktop Home filter that
section on `ResourceRole::Primary` alone, so the heading renders with nothing
under it while a `Continuation` exists and would open. A presentation bug, but it
is the section that answers "where was I."

---

## 4. What is genuinely correct, and must survive any rewrite

Not everything failed, and the parts that work are the parts a rewrite is most
likely to break.

**`pipeline.py` — 65 min, 5 resources, 3 sittings.** `pipeline.py` (29m,
locality 1.00) as Continuation, `01_CODE_line_by_line_walkthrough.md.pdf` (18m,
locality 1.00) as Primary, with the Google search and its result window held
back. A real coding-and-studying session: heterogeneous resources, near-zero
lexical similarity, correct roles, opens 2 of 9. **This is CASE 3 passing on
real evidence with no application knowledge.**

**Interview preparation — 27 min.** `Bhavitsaini_Resume.pdf`,
`05_Round2_GenericInterview_Prep_FULL.pdf`, and the Meet recording clustered
together. Correct grouping; the ended call as a restoration target is Defect 1.

**Workspace 9 — `Meet - xae-jgzs-xsx`** with `_tmp_popultae.rs` and two candidate
JSON files: a coding session during a call, correctly held together.

**Clustering holds large heterogeneous sets together.** The 23-member PlayStation
set is *cohesive* — it is one shopping/decision-making session and average-linkage
kept it as one. CASE 5's clustering half works. What fails is the significance
gate and the "Continue working on" framing.

**The membership/importance split is real.** 83.2% of members held back;
`opens_on_restore()` confined to `Continuation | Primary`; `ContextChain`
excluding `Context`. The brief's most important distinction is *architecturally
present*. It is the inputs to the importance decision that are wrong, not the
existence of the distinction.

**Identity folding does real work.** 63 names folded, a 29-name search fan-out
and a 13-name PDF pagination fan-out both collapsed. The mechanism is sound; its
reach is too short.

**Derivation is genuinely replayable.** Nothing derived is persisted; the daemon
re-derives from the canonical log on restart; `understanding::derive` is pure.
CASE 8 holds by construction.

**Machine churn never opens.** All 22 `Context` members stay closed. The
`ActCharacter` distinction — attributable to a person vs. not — is the one
classification in the system that is honest, and it works.

---

## 5. Root cause

Four sentences.

**One.** Identity is inferred from name-string shape within a single naming
system, using corpus-size thresholds, with no provenance evidence — so the same
thing becomes several resources, and several resources cannot become one thing.
Every measurement downstream divides one resource's evidence among its fragments.

**Two.** The system has one scalar notion of importance — attention, discounted
by locality — and asks it to answer two different questions. Duration of
attention does not distinguish a workpiece from a companion, so companions win
the resume point whenever they are open long enough.

**Three.** Every threshold is a share of a total that grows with the size of the
body of work, so the larger and more real the work, the less of it Evo can
justify opening; and every gate is satisfiable by any single member, so the
weakest evidence in a cluster can license the whole cluster.

**Four.** Insufficiency is unreachable. Because exactly one member is always
promoted to `Continuation`, Evo always produces a confident answer, and the
well-built machinery for saying "I do not know" can never run.

The old architecture failed at formation — it counted sightings and called the
result work. The current architecture fixed formation and left importance
reading the same signal formation used to read. The next architecture has to
settle identity first, then decide importance from evidence about *what kind of
thing this is to this work* rather than *how long it was in front of the person*.

---

## 6. Fake, fixture, and fallback data (mission Task 3)

### 6.1 — Verified: no fake data in any production path

Every occurrence of `Canvas`, `Design Brief`, `Research Notes`, `Reading List`,
`Spotify`, `WhatsApp`, `Notion`, `Figma` in the repository, classified by
comparing its line number against the file's `#[cfg(test)]` boundary:

| file | `mod tests` at | hits at | verdict |
|---|---|---|---|
| `evo-daemon/src/persistence.rs` | 581 | 832, 856, 879, 922 | **test** |
| `evo-desktop/src/state.rs` | 995 | 1417, 1516, 1600, 2081 | **test** |
| `evo-desktop/src/state.rs` | — | 343 | **doc comment** |
| `evo-execution/src/macos.rs` | 654 | 668–708 | **test** |
| `evo-execution/src/resource.rs` | 120 | 126–193 | **test** |
| `evo-execution/src/locator.rs` | 79 | 84–113 | **test** |
| `evo-daemon/src/runtime.rs` | 615 | 935 | **test** |
| `evo-daemon/src/understanding.rs` | — | 99–100 | **doc comment** |
| `evo-workspace/src/projection.rs` | — | 106 | **doc comment** |
| `evo-engagement/src/engagement.rs` | — | 455, 996, 1017 | **doc comment** |
| `evo-daemon/examples/verify_scenarios.rs` | — | 15, 342–400 | **diagnostic example, names used as input** |

No production `if app == …`, no blacklist, no whitelist, no seeded Workspace, no
hardcoded category, no demo mode. The only classification in production is
`ActCharacter::of_schema`, which matches on *frozen Observation schema names*
(`OBS-WINDOW-FOCUS-GAINED`, `OBS-URL-NAVIGATED`, `OBS-COMMIT-MADE`) — Evo's own
vocabulary for kinds of evidence, defaulting unknown schemas to the conservative
`Incidental`. A resource type invented next year classifies itself.

### 6.2 — Verified: there is no frontend to hold fallback data

The brief anticipated a TypeScript frontend with fallback arrays. There is none.
The repository contains **zero** `.ts`, `.tsx`, `.js`, `.jsx`, `.html`,
`.svelte`, `.vue` files and no `package.json`. `evo-desktop` is a native Rust
`eframe`/`egui` application reading `evo_daemon::cache::UnderstandingCache`,
which calls `understanding::derive` on the real log
(`cache.rs:195`). The only `placeholder` in the UI is an `egui` text-field hint
string (`ui.rs:712`).

### 6.3 — The actual root cause of the fake-looking Home, and a real finding

`Design Brief — Canvas` and `Research Notes — Reading List` **are in the real
Observation log** — 42s of attention, 8 human acts, currently a refused cluster.
They were genuinely observed windows during manual QA of Evo itself. Home showed
them because Evo could not tell an incidental window from work, which is Defects
1 and 5, not a fixture leak. Fixing the root cause means fixing importance; there
is nothing to delete.

**One real finding:** the storage root still holds derived state from the
previous generation —

```
497,670 bytes  workspace.log      (stale; nothing reads it)
1,046,606 bytes restoration.log    (stale; nothing reads it)
248,392 bytes  snapshot.log
53,900 bytes   attachment.log
```

1.5 MB of Workspace and Restoration claims written by a build that no longer
exists. Nothing reads them, so they cannot reach Home — but any machine that ran
an older build carries them, and a migration should remove them rather than leave
contradictory derived state on disk beside the canonical log.

---

## 7. Documents that will need amendment

Recorded here, not yet changed.

| document | contradiction |
|---|---|
| IS-0021 §25.2, §25.5 | Resume Point / Next Step are derived from `ResourceRole::Continuation`, which is elected by attention recency. The contract needs importance evidence that is not an attention measurement, and a reachable insufficiency path. |
| IS-0021 §25.5 | Next Step is a constant string. The contract asks for a workspace-level continuation summary grounded in evidence. |
| RFC-0011 / RFC-0012 / RFC-0013 | Designation, `WorkGrouped`, `RepositoryMembership`, Continuation Surface: zero occurrences in real evidence. Each may remain as an override, but none may be load-bearing. |
| `evo-engagement/src/params.rs` | The admissibility rule ("a parameter expresses a limit of what Evo can honestly claim to have witnessed") is sound, but `primary_attention_share` as a share of a growing total violates it in effect: it encodes a preference about workspace size. |
| Observation schemas (IS-0003) | Provenance context is empty on 100% of real Observations. Cross-naming-system identity is impossible without it. This is a capture-layer gap, and it is the one gap that cannot be closed by better inference. |

---

## 8. Verification status

**VERIFIED** — measured by running production derivation code against the real
canonical log on this machine:

- All 14 report items in §2.
- Defects 1, 2, 3, 4, 5, 6, 7, 8, 9 — each from real derived output, quoted above.
- No fake data in any production path (§6.1), no frontend fallback data (§6.2).
- Derived state is not persisted; the daemon re-derives on restart.
- `cargo build --offline --workspace` exits 0.

**NOT VERIFIED** — requires the new behavioral model and implementation:

- Defect 10 (empty "You were working in" section) is inferred from the display
  filter and Workspace 4's role table; Home has not been rendered against it.
- Whether restoration *execution* actually opens what the plan names. Nothing in
  this diagnosis ran `evo-execution` against a live desktop.
- CASE 7 end-to-end: return, restore, observe what opens, compare to captured
  activity.
- Replay equivalence beyond construction — no differential test was run this
  session.

**ENVIRONMENT BLOCKED** — nothing. Every measurement this diagnosis needed was
obtainable.

---

## 9. What comes next

1. Define the new behavioral model: identity settled before measurement; a
   notion of importance that is not attention; a reachable insufficiency path;
   evidence-grounded continuation summaries.
2. Amend IS-0021 and the RFCs listed in §7 with the contradiction and the reason.
3. Write the adversarial behavioral fixtures **first** — the eleven scenarios the
   brief names, plus one for each defect above, each asserting on real failure
   modes rather than on whatever implementation follows.
4. Implement.
5. Re-run this diagnostic and publish before/after for every number in §2.
6. Real-Mac validation: capture, leave, inspect, restore, observe, compare —
   across several kinds of work.
