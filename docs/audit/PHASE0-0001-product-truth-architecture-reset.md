# PHASE0-0001 — Product Truth: Can Evo Restore Arbitrary Interleaved Work?

**Status:** Phase 0 audit — findings and proposed architecture. Nothing implemented.
**Date:** 2026-08-25
**Scope:** The whole system, end to end, as implemented.
**Authority:** This document proposes amendments. It does not itself amend anything.

---

## 0. Answer

**No. The current architecture cannot deliver the product contract, and the reason is
not a set of bugs.**

Three load-bearing abstractions each contradict an explicit product requirement:

| Abstraction | Product requirement it contradicts |
|---|---|
| Membership is a hard partition (agglomerative clustering) | One artifact must be able to participate in several bodies of work |
| A group that fails the significance test is *refused* into a dead end | Evo must never silently discard potentially relevant context |
| The Resume Point requires an *exclusive* member, and exclusivity requires recurrence | Work done once, in one sitting, must still be restorable |

Measured against the only real corpus available (1245 observations, 61 sittings,
425 resources, this machine): **2 of 11 bodies of work can be restored at all
(18.2%)**, and both of the two are habits rather than tasks. The other nine
produce nothing to open. Separately, **341 of 425 observed resources reach no body
of work at all**, and **152 of 425 are measurably related to two or more bodies of
work but were forced into exactly one**.

The single most telling number: **of the two bodies of work this person demonstrably
returned to after more than a day away — the exact case the product exists to serve
— neither has a resume point.** The two that do have one were both finished inside a
single day (§3.8).

All tests pass. That is the point: the tests encode the current model, and the
current model is not the product.

---

## 1. Method, and what is deliberately marked UNKNOWN

Everything numeric below was produced by running the real persisted Observation
log through the exact code path the daemon runs.

- `crates/evo-daemon/examples/diagnose_real_history.rs` — existing, read-only.
- `crates/evo-daemon/examples/audit_work_model.rs` — **new, read-only**, written for
  this audit to ask questions the pipeline is not built to answer. It opens the
  canonical store, decodes, derives, prints. It writes nothing. Sections: [A]
  multi-membership · [B] interleaving · [C] resume-point blockers · [D] what is
  lost · [E] nameability · [F] identity fragmentation · [G] evidence base ·
  [H] provenance attribution · [I] multi-day work and long gaps.

Store: `~/Library/Application Support/Evo/storage`. No production code was
modified for this audit.

One measurement error was made and corrected in the course of this audit, and it is
worth recording because the cause is itself a finding: the resume-point cross-tab in
§3.8 first looked bodies of work up **by title**, which silently conflated the two
that are both titled `www.google.com/search` and reported three resume points where
there are two. Indexing by position fixed it. The duplicate-title defect in §3.3
corrupted the diagnostic built to measure it.

**Marked UNKNOWN, not estimated:**

- **Auto-open precision and recall.** There is no labelled record of what this
  person considered one body of work. Precision and recall are therefore not
  computable. They can only be *bounded* by the exclusion measured in §10.
- **Resume-point correctness.** Same reason: correctness needs a ground-truth
  "where I actually left off". Nine of eleven bodies of work have no resume point
  at all, which is a *coverage* failure and is measurable; whether the two that
  exist are *right* is not.
- **Observations never captured.** The capture layer drops events before they
  reach the log (§3.9) and records no count of what it dropped. The size of that
  loss is unknown and currently unknowable from the store.
- **Whether 11 is the right number of bodies of work.** Unknown. The audit can
  show that specific groups were wrongly refused; it cannot establish the true
  count.

---

## 2. The Product Contract

Stated as guarantees, because a guarantee can be tested and a promise cannot.

### 2.1 What Evo must guarantee

**G1 — Work identity.** Evo maintains a set of *bodies of work*, each of which has
a durable identity that survives restarts, membership change, and the work being
interrupted for days.

**G2 — Separation under interleaving.** When a person alternates between bodies of
work X, Y, Z within a single sitting and across sittings, Evo keeps X, Y and Z
distinct. Temporal proximity, frequency, application identity, domain identity and
repository membership are each insufficient alone and none may be decisive.

**G3 — Shared participation.** One artifact may participate in several bodies of
work simultaneously, with a different role and a different relevance in each.

**G4 — Reference resolution.** A person can refer to a body of work in their own
words ("take me back to the automation", "the research from yesterday", "the work
around this document") and Evo resolves the reference, or says honestly that it
cannot and offers the candidates.

**G5 — Restorability of any real work.** A body of work is restorable if it
happened. Not only if it recurred; not only if it was long; not only if one of its
members was used nowhere else.

**G6 — Resume state.** For each body of work Evo can state where the person left
it: the last place their attention actually was inside that work.

**G7 — Graded restoration.** Restoration produces four distinct sets, never one:
the resume point; the immediate open set (small, bounded, only what is needed to
re-enter); available supporting context (one gesture away, not opened); and
historical context (recorded, findable, never offered). Every artifact lands in
exactly one of these, with a reason.

**G8 — No terminal exclusion.** *The safety property.* Nothing that has any
evidence of relevance may be dropped where the product cannot reach it. False
inclusion costs the user one ignorable item. False exclusion costs them the work.
Where confidence is insufficient to open something, the answer is a weaker
*disposition*, never a smaller *membership*.

**G9 — Uncertainty is representable.** Evo can say "I don't know", "this is
probably part of this work", and "I cannot name this work" — and must, rather than
substituting a guess that reads as certainty.

**G10 — Explainability and replay.** Every conclusion traces to the witnessed
evidence that produced it, and re-deriving from the same canonical history
produces the same result.

**G11 — Domain neutrality.** No application, domain, title, file type,
profession, workflow or category may participate in a work decision. Software
development, research, writing, finance, design, study, administration, shopping
and workflows that do not exist yet are all served by the same rules.

**G12 — Local-first.** No cloud dependency, no external service for core
understanding, no shipping activity to a remote model.

### 2.2 The product test, restated as an obligation

Given interleaved work X, Y, Z over an arbitrary number of sittings with
overlapping resources, and the request "take me back to X", Evo must answer nine
questions: what the work was; which artifacts participated; which were central;
which were supporting; where the person left it; what to open now; what to keep
available; what the person was doing with the important artifacts; and what to do
when the evidence is uncertain.

The current architecture can answer four of the nine. §3 says which.

---

## 3. The Architecture As Implemented

Not as the documents describe it. This is the code path, stage by stage, with what
each stage destroys.

```
OS events
  └─ capture            macos_fsevents / macos_url_poller / focus  → FsSignal, URL, window title
  └─ Observation        4 frozen schemas, hex-encoded append-only log
  └─ interpret()        Observation → Act{Resource, ActCharacter, at}   + Declarations
  └─ segment_episodes() Acts → Episode[]  (gap > presence_horizon = new sitting)
  └─ IdentityIndex      witnessed subject → canonical subject
  └─ AttentionLedger    per resource: attention, per-sitting attention, sittings, human sittings
  └─ AffinityGraph      pairwise evidence: interleave .45 / co-episode .25 / lexical .20 / structural .10
  └─ cluster()          BFS components → average-linkage UPGMA   ← HARD PARTITION
  └─ EngagementSet      assemble() + is_significant()            ← TERMINAL REFUSAL
  └─ assign_roles()     Continuation / Primary / Supporting / Reference / Context
  └─ project_workspaces Engagement → Workspace (Attachments + Snapshots)
  └─ derive_restoration ResumePoint, ContextChain, NextStep, Blockers, ContinuationSurface
  └─ select_restoration OpenNow / AvailableIfNeeded / HistoryOnly
  └─ evo-desktop        Home cards, substring filter, detail pane
     evo-retrieval      STUB — ignores its trigger, returns every Workspace
```

### 3.1 Capture → Observation

**Enters:** file writes under `$HOME`; http/https navigations; window-focus
changes; git HEAD reflog appends.
**Discarded, permanently:** everything under `~/Library` and every path with a
dot-component (`is_user_visible_path`, `macos_fsevents.rs:454`); every non-http
scheme (`is_http_url`, `macos_url_poller.rs:365`); every application's internal
state; clipboard, selection, scroll, edit position; which window a file save
happened in.
**Assumption introduced:** that user-visible work does not live in `~/Library` or
in dot-directories. This is false for app-container document stores and for
dotfile-centric work.
**Uncertainty represented:** none. A dropped event leaves no trace, so the loss is
not merely unrecoverable — it is unknowable. This is the only place in the whole
pipeline where exclusion is *permanent*, and it is not recorded.

Also fixed at this boundary: a `FILE-SAVED` carries no owning-window provenance by
construction (§4 D), so the strongest available link between a change and the place
it was made is never captured.

### 3.2 Act and sitting segmentation

**Enters:** Observation.
**Transformed:** schema → `ActCharacter`. `Attentional` = focus/navigate.
`Deliberate` = commit. `Incidental` = file save **and every schema not recognised
by this build**.
**Discarded:** nothing structural.
**Assumption:** a file save is not human evidence. Defensible — the OS and every
background process also write files — but on this corpus it means **62.9% of all
acts (783 of 1245) carry no attention and no human standing**. The single most
common thing a person does to a document is invisible as evidence of them doing it.
**Uncertainty:** the unknown-schema default is `Incidental`, which is the
*weakest* reading. Honest, and it means a new collector's evidence is silently
demoted until someone edits an enum.

### 3.3 Identity

**Enters:** witnessed subject strings.
**Transformed:** decoration stripped when the same decoration appears under
several stems. 490 witnessed → 425 canonical (13.3% folded).
**Residual failure, measured:** 24 near-duplicate families cover **283 of 425
canonical resources (66.6%)**. `Inbox (2,886)`, `Inbox (2,884) - 878 MB`,
`Inbox (2,871) - 942 MB` and `Inbox (2,877)` are four separate resources. Two
distinct Workspaces are both titled `www.google.com/search`. Identity folding
handles decorative *suffixes*; it does not handle a counter embedded in a title.
**Consequence:** the corpus statistics that affinity depends on (ubiquity, IDF)
are computed over a population that is roughly half noise — 219 of 425 resources
are Photos Library internals.

### 3.4 Attention ledger

**Enters:** Episodes.
**Rule:** attention accrues only to Attentional acts, only for the gap to the next
act in the same sitting, only when that gap ≤ `max_interval_attention` (600s).
**Discarded:** all attention on resources that are only ever written. A document
edited for an hour in an app Evo cannot see focus for measures zero.
**Uncertainty:** under-claims deliberately. Correct direction, and it is
load-bearing for the failures in §3.7.

### 3.5 Affinity

**Enters:** Episodes, ledger, declarations.
**Produces:** per pair, four measured signals and an `EvidenceKind`.
**This layer is sound.** It measures relation, keeps the evidence, and names
nothing. It is the one stage that already represents a graded, overlapping
relation — every resource has a measured affinity to every other.
**And the next stage throws that away.**

### 3.6 Clustering — **the first fatal stage**

`cluster()` (engagement.rs:787) takes connected components over above-floor edges,
then merges within each component by average linkage while the average clears
`cohesion_floor`. Every subject lands in exactly one group.

`EngagementSet::containing()` (engagement.rs:641) is `.find(...)` — the API itself
assumes at most one.

**Measured:** 425 of 425 resources appear in exactly **1** group. Resources shared
across two or more groups: **0**.

(Read "group" strictly: every cluster the partition produced, *claimed or refused*.
So all 425 resources are in exactly one group while only 84 are in a Workspace —
the other 341 are in groups that §3.7 refuses. The two numbers measure different
things and both matter.)

**What that costs, measured:** **152 of 425 resources (35.8%) have affinity ≥ the
relatedness floor to members of two or more claimed bodies of work.** Every one was
assigned to one and the other relationships were discarded. Examples, verbatim:

```
https://mail.google.com/mail/u/3/
    0.60  mail.google.com/mail/u/3
    0.58  Job assignment analysis and deadline - Claude
    0.45  Login
https://claude.ai/chat/0743595c-…
    0.64  Job assignment analysis and deadline - Claude
    0.58  mail.google.com/mail/u/3
    0.54  Login
```

A 0.60/0.58 split is a coin flip, decided irreversibly, silently. This directly
fails **G3** and the stress scenario "one artifact used across multiple bodies of
work".

**No RFC requires this.** I checked: no document in `docs/` mandates disjoint
membership. The partition is an artifact of choosing an agglomerative algorithm,
not an architectural decision anyone took. That is good news — it can be replaced
without contradicting accepted authority.

### 3.7 Significance — **the second fatal stage**

`is_significant()` requires all three of: ≥2 members; some member attended in >1
sitting; ≥`sustained_sitting_attention` (120s) of attention inside a *single*
sitting. Groups that fail go to `EngagementSet::encountered()`.

**`encountered()` is read by two diagnostics and nothing else.** Not by
`project_workspaces`, not by Restoration, not by the desktop. A refused group is
gone from the product.

**Measured:** 330 clusters refused. Of those, **6 had ≥2 members and ≥2 witnessed
human acts**:

```
7 members,  7 human acts, 50s   Inbox (2,886) · agentrouter.org/console ·
                                /console/log · /console/personal · +3
2 members,  8 human acts, 42s   Design Brief — Canvas · Research Notes — Reading List
2 members,  3 human acts, 40s   Visual Studio Code · sales_lines
2 members,  2 human acts, 34s   IS-0001-Observation.md — Modified · Inbox (2,884)
```

The second is two documents the person moved between across two separate sittings
with eight human acts — a writing-plus-research pairing, which is one of the named
stress scenarios. It is discarded because 42 seconds is less than 120.

**And separately:** **24 resources that a person was witnessed acting on reach no
claimed body of work at all**, carrying 6 minutes of witnessed attention —
`Downloads` (85s, 2 sittings, 5 human acts), `Design Brief — Canvas` (28s, 2
sittings, 4 human acts), the whole `agentrouter.org/console` session. Counting
every observed resource, **341 of 425 are in no Workspace whatsoever.**

This fails **G8** outright. It is not a threshold that is slightly wrong; refusal
being *terminal* is the defect.

### 3.8 Roles and the Resume Point — **the third fatal stage**

`ResourceRole::Primary` requires `is_exclusive() && returned()`.
`Continuation` (the Resume Point) is promoted only from a `Primary`.
`exclusive` requires **≥120s of attention in ≥2 distinct sittings**, all of which
must be inside this work or uncontested.

**Measured: 9 of 11 bodies of work have no Resume Point, and in every one of the
nine the reason is identical — no member reached two sittings of real attention.**

```
mail.google.com/mail/u/3       deepest single sitting 1044s   qualifying sittings: 1
Meet - kxc-vads-gdp            deepest single sitting  992s   qualifying sittings: 1
Login                          deepest single sitting  505s   qualifying sittings: 1
Meet - xae-jgzs-xsx            deepest single sitting  496s   qualifying sittings: 1
‎WhatsApp                       deepest single sitting  490s   qualifying sittings: 1
vidbox.vc/watch/tv             deepest single sitting  393s   qualifying sittings: 1
```

**Seventeen minutes of continuous, measured, witnessed attention produces no
resume point, because the person only did it once.** The model can restore habits.
It cannot restore tasks. That fails **G5** and it fails the product test directly:
"user works on X, Y, Z, closes laptop, take me back to X" — if X happened in one
sitting, Evo returns nothing.

Recorded null result: the `elsewhere` clause of exclusivity — the part that
punishes interleaving — **never fires on this corpus**, because exclusivity is
already denied earlier by the recurrence requirement. Interleaving is not yet the
binding constraint; recurrence is. Both are wrong, and the ordering matters for the
fix.

**And the failure lands hardest exactly where restoration matters most.** Cross-tabbing
resume-point coverage against whether a body of work was actually resumed after a
day or more away:

```
        resume point: no   multi-day: yes  [0] ‎WhatsApp
        resume point: yes  multi-day: no   [1] www.google.com/search
        resume point: yes  multi-day: no   [4] Job assignment analysis and deadline
        resume point: no   multi-day: yes  [8] www.google.com/search
        …7 more, all: no / no
```

**Both bodies of work the person demonstrably came back to after more than a day
away have no resume point. Both bodies of work that do have one were finished
inside a single day.** The dwelling test does not select for "work worth resuming"
— it selects for work that was *deeply* attended twice, which on this corpus is
uncorrelated with, and here inversely related to, actually being resumed. `‎WhatsApp`
spans days and still fails, because its 490 seconds of attention all fell in one
sitting.

This is the sharpest available refutation of the current model: the two cases that
*are* the product ("I left this and came back days later") are the two cases it
cannot serve.

### 3.9 Projection to Workspace

Faithful. Every member is attached whatever its role; roles decide opening, not
belonging; identity is the founding sighting so it survives interpretation
improving. `members_across_history()` sources membership from the Attachment Set,
which is the canonical superset. **This layer is correct and should be kept.**

Note what it means though: a Workspace is a *projection*. It contains no
information the Engagement did not have. So a Workspace cannot be the primitive
work understanding emerges from — it is already downstream of the decision. The
directive asked whether Workspace is the right primitive. It is a fine
*representation* and it is the wrong *inference primitive*, because by the time it
exists the partition has already happened.

### 3.10 Restoration derivation and selection

**This layer is the best part of the system and needs almost no change.**
`select_restoration` produces exactly the four-way shape **G7** asks for:
`OpenNow` / `AvailableIfNeeded` / `HistoryOnly`, each with a fixed, honest reason
sentence, and nothing is dropped — every withheld member is listed. The immediate
open set is bounded by `max_primary: 3` plus one Continuation, so a 30-member body
of work cannot open 30 things.

Its inputs are the problem, not its logic:
- With no declaration, the immediate set = the *most recent Snapshot's* members
  whose role opens on restore. If no role opens, nothing opens. Hence 5 resources
  opened across 11 bodies of work; **79 of 84 members held back (94.0%)**.
- **Declarations in the real corpus: 0 groupings, 0 designations, 0 continuation
  subjects.** The entire declaration path — the one thing that can override a bad
  inference — has never been exercised by the real user. Any design that leans on
  it to fix inference quality is leaning on something that is empirically not there.

### 3.11 "What was I doing with this?"

`NextStep` is one of **exactly two fixed strings**:

```
"The user designated this resource as the work to continue."
"This is the resource you were last working in as part of this work, so this is
 where the work continues."
```

There is no representation of activity anywhere in the information model — no
reading vs writing vs comparing vs debugging. Requirement (8) of the product test,
"what the user was doing with the most important artifacts", is **not
representable**. Not badly done: absent.

### 3.12 Retrieval — **absent**

```rust
pub fn retrieve(&self, trigger: impl Sized, current_committed_workspaces: &[Workspace])
    -> Result<Vec<WorkspaceId>, RetrievalError> {
    let _ = trigger;                      // ← the trigger is discarded
    ...returns every workspace id...
}
```

`evo-retrieval` is depended on by no other crate. The only thing resembling
retrieval in the product is `home.rs:343`: a lowercase **substring match** over
concatenated member subjects.

So "Hey Evo, take me back to Automation X" would need the literal characters of a
resource name. **G4 has no implementation.** And it could not work anyway: **2 of
11 bodies of work have a title resting on vocabulary their members share**; the
other nine borrow one member's name (`‎WhatsApp`, `Login`, `Pluely`,
`vidbox.vc/watch/tv`, `Meet - xae-jgzs-xsx`), and two of them borrow the *same*
name. There is nothing to refer to.

### 3.13 Gap analysis — every guarantee, against the implementation

Deliverable 3, stated against the contract in §2.1 so each verdict is checkable.

| | Guarantee | Verdict | Evidence |
|---|---|---|---|
| **G1** | Work identity | **Partial** — identity is durable, but nine of eleven bodies of work have no referable name and two collide | §3.3, §3.12 |
| **G2** | Separation under interleaving | **Met** — 13 sittings had ≥2 bodies of work live and the strands stayed apart | §7 |
| **G3** | Shared participation | **Fails structurally** — 0 of 425 resources in >1 body of work; 152 forced choices | §3.6 |
| **G4** | Reference resolution | **Absent** — `retrieve` discards its trigger; the product uses a substring match | §3.12 |
| **G5** | Restorability of any real work | **Fails** — 2 of 11; 17 minutes of single-sitting attention restores nothing | §3.8 |
| **G6** | Resume state | **Fails** — 9 of 11 have none, all for the same reason | §3.8 |
| **G7** | Graded restoration | **Met** — three dispositions, bounded open set, every withheld member reasoned | §3.10 |
| **G8** | No terminal exclusion | **Fails** — 330 refused groups unreachable; 24 attended resources orphaned | §3.7 |
| **G9** | Uncertainty representable | **Partial** — representable in Restoration, destroyed in membership and naming | §3.6, §3.7 |
| **G10** | Explainability and replay | **Met** — replay-deterministic; every stage but clustering explains itself at the margin | §3.14 |
| **G11** | Domain neutrality | **Met in inference, breached at capture** — no app/domain/profession in any work decision; `~/Library` and dot-paths silently unobserved | §4 G |
| **G12** | Local-first | **Met** — no network in any inference path | — |

Four of twelve met, two partial, one met-with-a-caveat, five failed. The five
failures are not independent: **G3**, **G5**, **G6**, **G8** and half of **G9** all
trace to the three abstractions in §0, and **G4** is simply unbuilt.

### 3.14 Per-stage answers to the required questions

| | Obs | Act | Ident | Attn | Affin | Cluster | Signif | Roles | Wkspc | Restore | Retrieval |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Explainable | ✓ | ✓ | ✓ | ✓ | ✓ | partly | ✓ | ✓ | ✓ | ✓ | n/a |
| Replayable | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | n/a |
| Two works distinguishable | — | — | — | — | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| Artifact in several works | ✓ | ✓ | ✓ | ✓ | **✓** | **✗** | ✗ | ✗ | ✗ | ✗ | ✗ |
| Relevance can change over time | ✓ | ✓ | — | ✓ | ✓ | **✗** | ✗ | ✗ | partly | ✗ | ✗ |
| Supporting without being resume point | — | — | — | — | — | — | — | ✓ | ✓ | ✓ | — |
| Can represent "I don't know" | ✓ | partly | ✓ | ✓ | ✓ | **✗** | **✗** | ✓ | ✓ | ✓ | ✗ |

The column where the ✗s begin is `cluster`. Everything to its right inherits them.

---

## 4. Root Causes, By Category

**A — Incorrect abstraction (the real cause; three instances).**

1. *Membership modelled as a partition rather than a relation.* Affinity already
   produces a graded relation; clustering collapses it to an assignment, and the
   collapse is irreversible and unexplainable at the margin.
2. *Significance modelled as existence rather than prominence.* "Not confident
   enough to show this on Home" was implemented as "this does not exist".
3. *Exclusivity and recurrence used as preconditions for having a resume point.*
   "Where did I leave off" is a question about the *last witnessed moment*, not
   about repetition. Requiring repetition to answer it is a category error.

**B — Information discarded too early.**
The affinity graph knows 152 resources have multiple homes. The cluster step
destroys that. The ledger knows the last thing attended in every sitting. The role
step destroys that behind an exclusivity gate. Both facts are needed downstream and
both are gone before anything downstream runs.

**C — Missing subsystem, not a defect.**
Work identity, naming, reference resolution and activity description do not exist.
`evo-retrieval` is a typed stub. These are honest gaps, correctly not faked.

**D — Insufficient data (real, and smaller than it looks).**
62.9% of acts are Incidental with no attention. Attention cannot distinguish
focused-and-working from focused-and-playing.

On provenance, the precise position matters and my first reading of it was too
broad. The plumbing exists: `Provenance::context` round-trips, and
`MacOSSignal::WindowFocusGained` populates it with the owning-process pid when one
is available (`crates/evo-capture/src/adapters/macos.rs:116-122`). But
**`MacOSSignal::FileSaved` passes `HashMap::new()` by construction**
(`adapters/macos.rs:134`), so a file save can never carry the window it happened
in — and file saves are 63% of this corpus. That is the architecturally load-bearing
half, and it is certain from the code rather than inferred from the data.

Measured: **0 of 1245 observations in this corpus carry any provenance context, and
no context key appears at all.** Whether that 0% is data vintage (this corpus spans
2026-08-16 to 2026-08-21 and the pid change landed around 2026-08-17) or a live
failure (`process_identifier` always `None`) **cannot be distinguished from the
store, and is marked UNKNOWN.** The conclusion for the product does not depend on
which: even with the pid path working perfectly, the 63% of evidence that is file
saves remains unattributable.

These bound how good inference can be — but they do **not** explain the failures in
§3.6–3.8, which are caused by discarding evidence Evo already has.

**E — Implementation bugs.** None material found in this pass. Identity's
inability to fold an embedded counter (§3.3) is the closest, and it is a
capability gap rather than a bug.

**F — Document contradictions.** Two, both real:

1. **RFC-0014 Requirement 13 (Honest degradation) instructs the wrong degradation.**
   It says that where a signal is unavailable Evo must produce a weaker claim,
   and spells that out as "**fewer bodies of work**, weaker roles". Producing
   fewer bodies of work is exactly the silent exclusion **G8** forbids. The
   intent is right and the mechanism is wrong: degrade the *claim*, not the
   *membership*.
2. **No document states the false-exclusion safety property at all.** Nothing in
   `docs/` says inclusion is preferred to exclusion under uncertainty. The
   affinity module states the opposite as a virtue: "the only direction this can
   err in remains exclusion." Under the product contract that sentence describes
   the more damaging direction, not the safe one.

**G — Hardcoded assumptions used in work decisions.** Full inventory, whole repo,
production code only (tests and examples excluded):

| Location | Assumption | Verdict |
|---|---|---|
| `macos_fsevents.rs:454` `is_user_visible_path` | work never lives in `~/Library` or any dot-path | **Real, and it is a terminal exclusion.** Capture scope, so unrecoverable. Must be documented as a product limit and made configurable. |
| `macos_url_poller.rs:365` `is_http_url` | web work is http/https only | Real but minor. `file://` and custom schemes are invisible. |
| `macos_fsevents.rs:483` `is_head_reflog_path` + reflog parsing | git is the repository tool | Real, and it is *enrichment*, not a filter: no git means no `COMMIT-MADE`, not less work. Acceptable; note the asymmetry that one profession gets a richer schema. |
| `resource.rs:110` `classify_resource_identity` | four schemas → four resource kinds | **Not a violation.** Dispatch on Evo's own frozen schema vocabulary, no app or domain. |
| `home.rs` `WorkKindFilter{Files,Windows,Urls,Commits}` | four presentational categories | **Not a violation.** Derived from schema; presentation only; participates in no work decision. |
| everything else | — | Swept `crates/*/src/**` for literal app names, domains, extensions, profession terms. **None found.** Strings are schema names, macOS framework symbols, UI labels, storage paths. |

The domain-neutrality constraint (**G11**) is, apart from capture scope, genuinely
honoured. This is the strongest part of the existing design and the redesign must
not spend it.

---

## 5. Proposed Architecture

### 5.1 The one change everything else follows from

> **Stop deciding. Start grading.**
>
> Membership becomes a *relation* between a resource and a body of work, carrying a
> grade, its evidence, and a role — never an assignment. Every categorical
> decision (open / don't open, show / don't show) moves to the presentation
> boundary, where a person sees it and can override it.

Restated as an invariant the pipeline must satisfy:

> **No terminal exclusion before the presentation boundary.** Any resource with any
> evidence of relevance to a body of work remains reachable from that body of work,
> at worst as history.

### 5.2 What is kept unchanged

Observation and its four frozen schemas · `Act`/`ActCharacter` · sitting
segmentation · `IdentityIndex` · `AttentionLedger` · **the whole AffinityGraph and
its evidence model** · the Workspace projection and its Attachment/Snapshot
semantics · **`select_restoration`'s three dispositions and their reason
sentences** · determinism and replay-equivalence · local-first · domain neutrality.

That is most of the system. The redesign is narrow and deep, not wide.

### 5.3 What is replaced

```
  cluster() → EngagementSet{engagements, encountered} → assign_roles()
```
becomes
```
  seed()      → overlapping candidate bodies of work
  grade()     → per (resource, body) membership: grade + evidence + role
  situate()   → per body: sitting timeline, resume state, activity per member
  name()      → per body: identity + a name, or an honest refusal to name
```

### 5.4 Seeding — overlapping, from the same evidence

A **seed** is a *locus of returned attention* and its cohesive neighbourhood.

1. A resource is a **locus** if the person was witnessed attending it — it has
   measurable attention, or a human act, in at least one sitting. (No recurrence
   requirement. This is what unblocks single-sitting work.)
2. For each locus `r`: `N(r) = {r} ∪ { x : affinity(r, x) ≥ affinity_floor }`.
3. Keep `N(r)` if its internal mean pairwise affinity ≥ `cohesion_floor`;
   otherwise shrink it by dropping its weakest member and retest. This uses the
   existing floors and introduces no parameter.
4. Deduplicate: two neighbourhoods collapse only when each is *largely contained*
   in the other. Distinct neighbourhoods that merely overlap stay distinct — which
   is precisely what agglomerative merging cannot do.
5. Declarations merge unconditionally, as today (RFC-0014 R11).

Seeds overlap by construction, so **G3** holds structurally rather than by
threshold. This is not a new score: it reads the same affinity graph as a relation
instead of forcing it into a partition. Deterministic — the loci are enumerated in
canonical order, the floors are the existing floors, ties break canonically.

### 5.5 Grading — the evidence model, not a score

For each `(resource, body)` pair with any evidence, membership carries **four
independent facts, kept separately and never multiplied into one number**:

| Fact | What it is | What it proves | What it does **not** prove |
|---|---|---|---|
| **Attention here** | measured attention in this body's sittings, per sitting | the person spent time in this, inside this work | nothing about importance to *resuming* |
| **Tie** | the strongest `EvidenceKind` to another member: declared / interleaved / co-present / shared-wording / structural | *why* Evo thinks this belongs, in one word | that it belongs *only* here |
| **Locality** | share of this resource's witnessed life falling inside this body | that the resource's life is mostly here | anything, when measured over one sitting |
| **Corroboration** | whether any tie rests on more than bare co-presence | that this is part of the work rather than beside it | that it is central |

Keeping them separate is the point. A single blended score cannot answer "why is
this here", cannot be argued with, and cannot degrade honestly — which is what
RFC-0014 R14 asks for and what a score silently defeats.

**Grade** is the mean measured affinity to the rest of the body (today's
`belonging`), used only for *ordering within a disposition*, never as a gate.

**No pair with evidence is ever discarded.** A weak pair becomes a low grade and a
`HistoryOnly` disposition. This is the mechanism by which **G8** holds.

### 5.6 Situating — resume state without recurrence

Per body of work, compute a **sitting timeline**: for each sitting, which members
were live and how much attention each drew.

**Resume state = the last member the person actually attended, in the last sitting
this body of work was live in.** A witnessed fact. No recurrence test, no
exclusivity test, no threshold.

Exclusivity does not disappear — it stops being a *gate* and becomes a *qualifier*
on the claim, which is where it always belonged:

- member's life is mostly here → "you were last working in X"
- member also lives elsewhere → "you were last in X, which you also use elsewhere"
- nothing attended in the last live sitting → "I can't tell where you left this"
  (**G9**, and honest)

This alone converts the measured 9-of-11 resume-point failure into a resume point
for every body of work that had any attended member, which on this corpus is all
of them.

### 5.7 Activity — what the person was doing, without reading content

Derived from act character and timing alone, over this body's sittings, as a small
closed set of grounded statements:

| Witnessed pattern | Statement |
|---|---|
| repeated `Incidental` inside sittings where the person was attending nearby | "you kept changing this" |
| repeated `Attentional`, no changes | "you kept coming back to read this" |
| one `Attentional`, no return | "you looked at this once" |
| `Deliberate` | "you committed this" |
| present, never attended, never changed | "this was open beside the work" |

App-agnostic, content-free, explainable, and it is the *most* Evo can honestly say
from the evidence it has. It answers product-test question (8) at the strength the
evidence supports and no further.

### 5.8 Naming and identity — the missing subsystem

Identity stays as it is (founding sighting hash; stable across membership change).

Naming, in strict order of authority:

1. **The person's own words.** A name they typed is canonical. There is already a
   declaration channel; naming becomes a fifth kind of declaration.
2. **Distinctive vocabulary shared by ≥2 members.** Exists today
   (`shared_vocabulary`, `titled_by_shared_vocabulary`); fires on 2 of 11 because
   the token basis is weak on URL-heavy corpora. Improving the tokenizer
   (path/URL/title segmentation, still with no domain list) is a contained fix.
3. **Refuse to name.** "Unnamed work — 6 resources, last active Tuesday", shown
   with its members. **Never borrow one member's name.** Borrowing is what
   produced two bodies of work both called `www.google.com/search` and an
   afternoon of coursework called `Spotify Premium`.

### 5.9 Reference resolution — the missing product surface

`evo-retrieval` gets a real implementation: phrase → ranked candidate bodies of
work, matching against declared names, then thread vocabulary, then member
subjects, then time expressions ("yesterday") resolved against the sitting
timeline. Ties are surfaced as a disambiguation, never resolved by guessing.

**Where a local model would be needed, and why not yet.** Token matching resolves
"the pipeline work" only when "pipeline" appears in a subject. It cannot resolve
"the automation I was doing" → `pipeline.py`. That gap is real and it is the *only*
place in this architecture where lexical evidence is provably insufficient. If it
is closed, the shape is: a **local**, on-device embedding of *subject strings and
declared names only* — never document contents, never network, optional, and
degrading to lexical matching when absent. **Not adopted in Phase 0.** It is not
architecturally necessary until (1) and (2) are implemented and measured, and doing
it now would spend the local-first guarantee for a benefit not yet demonstrated.

### 5.10 Restoration — mostly already right

`select_restoration` keeps its logic. Its inputs change:

- **Resume point** — from §5.6, always present when any member was attended.
- **Immediate open set** — resume point + members whose role opens, capped
  (`max_primary`). Bounded, so a 30-member body opens ≤4 things.
- **Available if needed** — members the person used, not opened.
- **History only** — members witnessed beside the work. Includes everything the
  old model refused, which is how they become recoverable again.

One addition: a **confidence band per disposition**, so uncertainty reaches the
user instead of being resolved on their behalf — "probably part of this work" is a
sentence Evo must be able to say (**G9**).

### 5.11 Is Workspace still the right primitive?

**No — and it stays anyway, as an output.** Work understanding must emerge from the
graded, overlapping membership relation of §5.4–5.7. A Workspace is that relation
*resolved for presentation*: attachments, snapshots, identity. It is a good
representation, it is already correct (§3.9), and it is downstream of the decision,
so it cannot be where the decision is made.

The practical consequence: because one resource may now belong to several bodies of
work, several Workspaces may share an Attachment. Nothing in the Workspace layer
forbids that. `EngagementSet::containing()` returning one result is the API that
must change.

---

## 6. Information Model

| Concept | Now | Proposed |
|---|---|---|
| **Observation** | canonical, immutable, 4 frozen schemas | unchanged |
| **Artifact** | identity of a resource across witnessed names | unchanged |
| **Work** | `Engagement`: a set, produced by a partition | **`WorkThread`**: a seed plus a graded membership map. Threads may overlap. |
| **Membership** | implicit — in the set or not | **explicit `(resource, thread)` relation** carrying attention-here, tie, locality, corroboration, grade, sittings |
| **Role** | 5 variants, importance-ordered | unchanged, but derived per-thread and never gating existence |
| **Work state** | absent | **sitting timeline**: which members were live per sitting, with attention |
| **Resume point** | requires an exclusive, recurring member | **last attended member of the last live sitting** — a witnessed fact |
| **Activity** | absent (2 fixed sentences) | **closed set of grounded statements** from act character + timing (§5.7) |
| **Restoration candidate** | member with an opening role in the latest snapshot | member with a disposition and a reason — four sets, nothing dropped |
| **Confidence** | `ConfidenceScore` = mean affinity; unused downstream | **band attached to every disposition and every name**, surfaced in the UI |
| **"I don't know"** | representable in Restoration (`Insufficient`), not in membership | representable **everywhere**: unnamed work, uncertain membership, unknown resume state |

---

## 7. Membership Model — How X, Y and Z Stay Apart

Not another score. The claim is about *what the evidence proves*.

**Why the four signals are enough, and what each cannot do.**

- **Interleaving** (profile cosine over co-presence company) proves *the person
  moved between these two things within one stretch of attention*. It cannot
  distinguish a thing present beside everything — which is why it is a profile
  cosine and not a count.
- **Co-episode** (NPMI over sittings) proves *recurrence of the pairing*, which
  temporal proximity alone cannot.
- **Shared vocabulary** (IDF-weighted distinctive tokens) proves *subject
  relatedness independent of timing*, which is what separates two tasks that
  happen in the same application at the same time.
- **Structural containment** proves *co-location*, which is the weakest and is
  weighted 0.10 accordingly — repository membership cannot be the universal
  solution and is not treated as one.

**Separation** comes from requiring cohesion of a *neighbourhood* rather than
transitivity of edges: one bridging resource cannot fuse X and Y, because the mean
pairwise affinity across the union does not clear the floor. **Convergence** comes
from the neighbourhood being defined by a locus of attention, so heterogeneous
resource kinds join on the same evidence.

**Interleaving is handled by not resolving it.** X, Y and Z alternating in one
sitting produce three overlapping neighbourhoods around three different attention
loci. The shared resource is graded into all three, with a different role in each.
The current model must pick one; the proposed model does not have to, and therefore
cannot pick wrong.

Measured on real data: the corpus does interleave — **13 of 61 sittings had two or
more bodies of work live in them, one had seven.** The evidence for the requirement
is in the user's own history.

One more measured fact that must shape expectations: **41 of 61 sittings (67%) had
zero claimed bodies of work live in them.** Two thirds of this person's computer
time produces nothing the product can offer. Some of that is genuinely not work.
Some of it is §3.7. The redesign should be measured on this ratio.

---

## 8. Restoration Model — Why This Opens And That Does Not

| Set | Rule | Reason given to the user |
|---|---|---|
| **Resume point** | the last member attended in the last live sitting | "this is where you left off" |
| **Open now** | resume point + members whose life is mostly here and that the person returned to, capped at `max_primary` | "the work also happens here" |
| **Available if needed** | members the person used but that are not where the work continues | "you used this; it is one gesture away" |
| **History only** | members witnessed beside the work with nothing further tying them | "this was here; kept so you can find it" |

**Why capping is safe and refusing is not.** A cap on the *open* set costs one extra
click. Exclusion from *membership* costs the work. So the cap stays and the
exclusion goes.

**Uncertain artifacts.** An artifact Evo is unsure about is included at the lowest
disposition its evidence supports, with a confidence band, and is never opened. It
is visible in "what else was here". This is the concrete form of **G8**: uncertainty
lowers the disposition, never the membership.

**Noise does not become chaos.** The Photos Library churn that dominates this
corpus lands at `HistoryOnly` with the lowest grade: never on Home, never opened,
reachable only by explicitly asking a body of work what else was around. Preserved
and out of the way — which is the distinction between conservative inclusion and
a mess.

---

## 9. Arbitrary-User Stress Scenarios

| # | Scenario | Now | Proposed | Residual risk |
|---|---|---|---|---|
| 1 | Three simultaneous software projects | Partial. Separation works if names differ; shared config/tooling forced into one project (measured: 152 contested resources) | Works. Shared tooling graded into all three | Three projects with near-identical vocabulary rely on interleaving alone |
| 2 | Research + communication + coding | **Fails.** Communication is where the coin-flip lands: `mail…u/3` 0.60 vs `Job assignment` 0.58 | Works. Mail is a member of both, `AvailableIfNeeded` in each | Communication is genuinely ambient; expect over-inclusion, which is the safe direction |
| 3 | Writing + browser research + PDFs | **Fails, measured.** `Design Brief — Canvas` + `Research Notes — Reading List`, 8 human acts, 2 sittings — refused for having 42s < 120s | Works. Seeded from either document as a locus | PDFs opened once are `HistoryOnly` — correct, they are not where writing continues |
| 4 | Design + implementation | Fails as #1 + #3 | Works | Design tools often save into `~/Library` → **invisible at capture** (§3.1) |
| 5 | Spreadsheet-heavy work | **Fails.** A spreadsheet edited for an hour with focus unobservable measures zero attention (§3.4) | Improved but limited: `Incidental` changes still carry no attention | **Real data gap.** Needs the capture-layer attribution fix, not architecture |
| 6 | Personal shopping / decision research | Fails. This corpus shows it: `www.playstation.com/…/games` is a 24-member body of work with 0 resume point and 0 opened | Works. Browsing has a locus and a last-attended page | Decision research is legitimately diffuse |
| 7 | A completely unknown workflow | **Already works.** No app, domain or type appears in any work decision (§4 G) | Unchanged | Unknown schemas default to `Incidental`, so a new collector is demoted until an enum arm is added |
| 8 | One artifact across several bodies of work | **Structurally impossible.** 0 of 425 shared | Works by construction | — |
| 9 | Many unrelated incidental applications | Works — this is what the current design is tuned for, and it is why it over-excludes | Works, and noise lands at `HistoryOnly` rather than being deleted | Home ranking must be prominence-aware, since membership no longer filters |
| 10 | Long-running work over many days | Partial. Snapshot history models it; the immediate set is scoped to the latest sitting only. **Measured: 2 of 11 bodies of work span a day-long gap; neither has a resume point** | Works. Resume from the last live sitting; earlier sittings remain available | Corpus spans only 3.8 days — behaviour over weeks is **UNMEASURED** |
| 11 | Work resumed after a long gap | **Fails, measured 2 for 2.** The only two bodies of work the person actually returned to after >24h are both unrestorable (§3.8) | Works. Resume state does not depend on recurrence, so a gap of any length is irrelevant to it | Names decay; a long gap makes a borrowed name useless, which is why §5.8 refuses to borrow |

---

## 10. Real-Data Validation

All from this machine's actual persisted history, via the daemon's own code path.

| Metric | Measured | Contract |
|---|---|---|
| Observations | 1245 | — |
| Schema mix | 63% FILE-SAVED · 24% WINDOW-FOCUS · 13% URL-NAVIGATED · 0 COMMIT-MADE | — |
| Act character | 37.1% human evidence · 62.9% Incidental | — |
| Sittings | 61 | — |
| Witnessed subjects → canonical resources | 490 → 425 (13.3% folded) | — |
| **Residual identity fragmentation** | **283 of 425 (66.6%) in 24 near-duplicate families** | should trend to 0 |
| Bodies of work claimed | 11 | — |
| Size distribution | 2,3,3,3,4,5,7,8,10,15,24 | — |
| Singleton rate | 0 of 11 | — |
| **Resources in ≥2 bodies of work** | **0 of 425** | **must be > 0 — G3** |
| **Resources measurably related to ≥2 bodies of work** | **152 of 425 (35.8%), all forced into one** | forced-choice loss → 0 |
| Sittings with ≥2 bodies of work live | 13 of 61 (max 7 at once) | evidence the requirement is real |
| **Sittings with 0 bodies of work live** | **41 of 61 (67%)** | should fall |
| **Bodies of work with a resume point** | **2 of 11 (18.2%)** | **→ 11 of 11 — G5, G6** |
| Cause of every resume-point failure | no member reached 120s of attention in ≥2 sittings | abstraction, not threshold |
| Corpus span | 3.8 days, 61 sittings, widest inter-sitting gap 0.6 days | — |
| Bodies of work resumed after >24h away | 2 of 11 | — |
| **Of those 2, how many have a resume point** | **0** | **→ 2 of 2 — this is the product** |
| **Provenance context on any observation** | **0 of 1245 (0.0%); no context keys present at all** | `FILE-SAVED` cannot carry one *by construction*; see §4 D |
| Resources auto-opened, all bodies of work | 5 | small is correct; zero is not |
| Members held back | 79 of 84 (94.0%) | should fall as resume points appear |
| **Clusters refused as work** | **330, reaching no Workspace, no Restoration, no UI** | **refusal must stop being terminal — G8** |
| **Refused groups with ≥2 members and ≥2 human acts** | **6** | → 0 |
| **Attended resources reaching no body of work** | **24, carrying 6 min of witnessed attention** | **→ 0 — G8** |
| Resources in no Workspace at all | 341 of 425 (80.2%) | must become reachable |
| **Bodies of work named from shared vocabulary** | **2 of 11; two share the identical borrowed name** | **→ every one named or honestly unnamed — G4** |
| Declarations in the corpus | 0 groupings, 0 designations, 0 continuations | inference cannot lean on declarations |
| Auto-open precision / recall | **UNKNOWN** — no ground truth exists | must stay marked unknown |
| Resume-point correctness | **UNKNOWN** — coverage is measurable, correctness is not | must stay marked unknown |
| Observations dropped at capture | **UNKNOWN** — not recorded anywhere | should become recorded |
| Behaviour over weeks or months | **UNKNOWN** — the corpus spans 3.8 days | must stay marked unknown |

**Verbatim evidence of the false-exclusion failure**, since it is the most important
finding:

```
REFUSED AS WORK — 2 members, 8 human acts, 42s attention
    Design Brief — Canvas          (attention 28s, 2 sittings)
    Research Notes — Reading List  (attention 14s, 2 sittings)

REFUSED AS WORK — 7 members, 7 human acts, 50s attention
    Inbox (2,886) · agentrouter.org/console · /console/log · /console/personal · +3
```

Both are recognisable bodies of work. Both are unreachable from the product.

---

## 11. Documents That Must Change

Ordered by how much depends on them. Nothing below is amended by this document.

| # | Document | Change | Why |
|---|---|---|---|
| 1 | **New RFC-0015 — Work Membership and Restoration Safety** | **Create.** States the false-exclusion safety property (**G8**) as a normative contract: no terminal exclusion before the presentation boundary; uncertainty lowers disposition, never membership; every observed resource remains reachable from some body of work. | The property appears in no document, and it is the constraint the whole redesign serves. |
| 2 | **RFC-0014 — Engagement Contract** → v2.0 | **Amend, substantially.** (a) R13 "Honest degradation": replace "fewer bodies of work" with degrading the *claim* — confidence, role, disposition, name — never membership. (b) New requirement: membership is a graded relation and a resource MAY belong to several bodies of work. (c) New requirement: the resume state is the last witnessed attention in the last live sitting; recurrence and exclusivity are qualifiers, not preconditions. (d) New requirement: naming must be grounded or refused, never borrowed from one member. (e) Retire `encountered` as a terminal category. | R13 as written mandates the primary failure. The rest are the contract for §5. |
| 3 | **RFC-0003 — Workspace Formation Contract** → v3.0 | **Amend.** State that a Workspace is a projection of a graded work relation, that several Workspaces may share an Attachment, and that Workspace is explicitly *not* the inference primitive. | Removes the ambiguity the current text leaves about where understanding is made. |
| 4 | **RFC-0007 — Retrieval Contract** → v2.0 | **Replace the body.** Currently defines a boundary and no algorithm, and the implementation discards its trigger. Specify: phrase → ranked candidates over declared names, thread vocabulary, member subjects, time expressions; disambiguation instead of guessing; explicitly rule out remote inference. | **G4** has no contract and no implementation. |
| 5 | **New RFC-0016 — Work Identity and Naming** | **Create.** Authority order for names (declared > shared vocabulary > honest refusal), the prohibition on borrowing a member's name, and identity stability across renaming. | Naming has no owning document; 9 of 11 titles are borrowed and two collide. |
| 6 | **New RFC-0017 — Activity Evidence** | **Create.** The closed set of grounded activity statements derivable from act character and timing, with the explicit prohibition on content inspection. | Product-test question (8) has no representation, and this is the boundary where content scraping would otherwise creep in. |
| 7 | **RFC-0006 / IS-0019 / IS-0021 — Restoration** | **Amend, narrowly.** Resume point no longer requires an exclusive member; add the confidence band per disposition. The four-set shape and reason sentences stand. | Restoration's logic is sound; only its preconditions change. |
| 8 | **IS-0011 — Workspace Model** | **Amend.** Membership is graded and may be shared; `containing()` returns a ranked list, not an option. | The API encodes the partition. |
| 9 | **New IS-0022 — Work Thread Derivation** | **Create.** Implementation spec for §5.4–5.7: seeding, grading, situating, with determinism and replay requirements. | Replaces the clustering half of the engagement spec. |
| 10 | **IS-0012 / IS-0013** (already superseded) | **Confirm superseded**, and check nothing in force cites them. | Housekeeping; prevents accidental resurrection. |
| 11 | **RFC-0001 / IS-0020 — Observation & Collector** | **Amend.** Capture scope (`~/Library`, dot-paths, non-http schemes) must be *declared* as a product limit, made configurable, and its exclusions recorded so the loss is known rather than silent. | The only place where exclusion is permanent and unrecoverable. |
| 12 | **`docs/foundation/ARCHITECTURAL_LAWS.md`, `COGNITIVE_MODEL.md`** | **Amend.** Add the safety property as a law. Correct the claim that erring toward exclusion is safe. | Foundational text currently asserts the opposite of the product requirement. |
| 13 | **`docs/architecture/ARCHITECTURE.md`** | **Amend.** Redraw the pipeline: relation, not partition; where uncertainty is carried; retrieval as a real stage. | It describes the intended system, not this one. |
| 14 | **ARCH-PROP-0001 (not adopted), RFC-0012 (superseded)** | **Leave.** Records of rules found wrong. | Useful history, no authority. |
| 15 | **`docs/audit/BE-AUDIT-0001.md`, `BE-TRACE-0001.md`, `ARCH-GAP-0001.md`** | **Mark historical.** They audit the *superseded* RFC-0012 co-membership pipeline (`evo-workspace/src/{formation,co_membership,workspace_decision,confidence}.rs`), which the engagement model replaced. Their findings are not wrong; their subject no longer exists. | Prevents a future reader treating them as a description of the current system. |
| 16 | **`docs/audit/DIAGNOSIS-0001-why-evo-does-not-understand-work.md`** | **Keep; supersede its forward-looking sections only.** It measures the same 1245-observation corpus and its headline — "Formation has largely been fixed. Importance has not, and identity never was" — is *consistent with and narrower than* this document. This report supersedes its remedy proposals, not its measurements. | Two audits of one corpus must not offer competing plans. |

### 11.1 Relationship to the existing audit corpus

This document does not contradict DIAGNOSIS-0001; it goes past it. DIAGNOSIS-0001
asked whether importance and identity were working and found they were not. This one
asks whether the *shape* of the model can express the product at all, and finds that
it cannot — which reframes DIAGNOSIS-0001's "importance is broken" as a symptom of
§4 A rather than a defect to be tuned.

---

## 12. Implementation Plan

**Not to start until the architecture in §5 is accepted.** Ordered so that each
stage is independently verifiable against the real corpus, and so the highest-value
fix lands first.

**Stage 0 — Instrumentation (read-only, no production change).**
Extend `audit_work_model` into the standing acceptance gate: multi-membership rate,
resume-point coverage, attended-orphan count, refused-with-evidence count, naming
coverage, contested-resource count. These become the numbers every later stage is
judged on. Record today's values as the baseline.

**Stage 1 — Resume state without recurrence.** (§5.6) Highest value, smallest
change, no data-model change. Expected effect on real data: resume-point coverage
2/11 → 11/11; auto-opened resources 5 → ~11–20; held-back share falls from 94%.
Gate: no body of work loses a resume point it had; `verify_*` examples stay green;
`replay_real_history` stays deterministic.

**Stage 2 — Stop terminal refusal.** (§5.1, §5.5) `encountered` groups become
low-confidence threads or their members attach as `HistoryOnly`. Gate: attended
resources reaching no body of work 24 → 0; refused-with-evidence 6 → 0; nothing new
auto-opens (inclusion must not leak into the open set).

**Stage 3 — Graded, overlapping membership.** (§5.4, §5.5) The real change:
`cluster()` → seed + grade; `containing()` returns a ranked list; Workspace
projection allows shared Attachments. Gate: contested resources 152 → 0 forced
choices; separation preserved (`verify_preflight` still yields two distinct bodies
of work); replay-equivalence holds; no body of work merges that was distinct.

**Stage 4 — Identity fragmentation.** (§3.3) Fold embedded counters and
volatile numeric decoration. Gate: 66.6% near-duplicate coverage falls
substantially; no two bodies of work share a title for want of identity.

**Stage 5 — Naming and honest refusal.** (§5.8) Improve the tokenizer for paths,
URLs and titles; add naming as a declaration; refuse rather than borrow. Gate:
every body of work is named from shared vocabulary, from the person, or explicitly
unnamed. Borrowed names: 9 → 0.

**Stage 6 — Activity statements.** (§5.7) Gate: every member of the open set
carries a grounded statement; no content is read; no statement exceeds the evidence.

**Stage 7 — Reference resolution.** (§5.9) Implement `evo-retrieval` for real and
wire the desktop to it. Gate: the phrases named in the product test resolve, or
Evo says it cannot and lists candidates. Explicitly out of scope: any remote call.

**Stage 8 — Capture scope, declared.** (§3.1) Make scope configurable and record
exclusions so the loss is measurable. Gate: the store can answer "what did you not
look at".

**Deferred, and to be revisited only with measurements in hand:** local semantic
representation of subject strings (§5.9); attaching owning-window provenance to
`FILE-SAVED` (`adapters/macos.rs:134` passes `HashMap::new()`, which is the blocker
behind stress scenario 5 and the reason 63% of the evidence is unattributable);
determining whether the measured 0% provenance rate is data vintage or a live pid
failure; distinguishing focused-and-working from focused-and-idle.

---

## 13. What This Audit Did Not Establish

Stated plainly, because the alternative is to imply more confidence than exists.

- Whether the eleven bodies of work Evo currently finds are the *right* eleven.
  Unknown; no ground truth.
- Whether the proposed seeding produces more true bodies of work on this corpus.
  It has not been run — Phase 0 forbids implementing it. The measurements in §10
  bound what it must fix, not what it will find.
- How much work the capture layer never saw. Not recorded.
- How any of this behaves over weeks or months. The corpus spans 3.8 days, so
  "work resumed after a long gap" was testable only at the one-to-two-day scale.
  What happens to naming, ranking and resume state after a month is unmeasured.
- Whether one person's 1245 observations generalise. They do not, on their own.
  Every claim above that rests on this corpus is labelled with its numbers so it
  can be re-tested against another.
