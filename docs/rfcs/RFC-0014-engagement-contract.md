# RFC-0014 — Engagement Contract

Status: Accepted

Version: 2.0

Supersedes:

- RFC-0012 (Canonical Co-Membership Evidence) — entirely
- RFC-0003 Requirement 2, second clause, and RFC-0003 Non-Goals (see RFC-0003 v2.0)

Amends:

- Architecture §3, §4, §5, §6, §7, §9, §12
- IS-0011 (Workspace Model), IS-0012 (Workspace Formation), IS-0013 (Workspace Replay)

Depends On:

- Constitution
- Cognitive Model
- Product
- Architecture
- Architectural Laws
- RFC-0000
- RFC-0001 (Observation Contract)
- RFC-0002 (Artifact Identity Contract)
- RFC-0003 (Workspace Formation Contract, v2.0)
- RFC-0011 (Continuation Evidence Contract)
- RFC-0013 (Continuation Surface Declaration)

Implemented By: `evo-engagement`, `evo-workspace::projection`, `evo-daemon::understanding`

---

# Amendment Record

## v2.0 — Engagement identity is a work hypothesis, not a connected component

Real-corpus validation after v1.1 found that the resource partition had been
removed but recreated one level higher: occurrence contexts were joined by
pairwise compatibility and then transitively closed into connected components.
On 140 contexts, 129 became one component and projected as a 136-member
Workspace. The same rules fragmented coherent one-sitting purchase research.

The following v1.x statements are superseded:

- an Engagement is not necessarily a set formed by clustering Resources or
  contexts;
- one Resource is not categorically incapable of representing work;
- return across sittings is not categorically necessary when one sustained,
  coherent context already witnesses a body of work;
- lack of a second sitting must not prevent derivation of the last meaningful
  work-local stopping point.

Normative replacement:

1. Engagement identity MUST be represented by a `WorkHypothesis` carrying
   graded links to occurrence contexts.
2. A context MAY link to zero, one, or several hypotheses.
3. Hypothesis formation MUST preserve positive and negative evidence separately.
4. Pairwise compatibility MUST NOT be made transitive by connected components.
   A merge requires two-way selective support and MUST be vetoed by sufficiently
   strong contradictory context evidence.
5. Artifact membership MUST be aggregated from work-attributed occurrences, not
   from the artifact's whole lifetime or whole sitting.
6. Belonging and restoration role MUST remain independent. A retained Artifact
   MAY be closed by default without weakening or deleting its membership.
7. `Continuable` MAY be established by recurrence or by one sustained coherent
   multi-artifact context. `Restorable` additionally requires one unambiguous
   last meaningful work-local occurrence.
8. A single-artifact hypothesis MAY be `Continuable` only when direct human
   evidence or declaration distinguishes it from a merely focused/idle surface;
   otherwise it remains `Remembered`.

The authoritative algorithm, data structures, implementation boundary, and
acceptance suite are in `docs/architecture/WORK-MODEL.md` §0.

## v1.1 — Requirement 8 gains an evidential rule for unattended changes

v1.0 named the five roles and forbade collapsing them, but said nothing about
*how* a Resource arrives at one. That was a hole of the same kind this RFC exists
to close: the contract described a distinction and left the mechanism to be
invented below it, and what got invented was wrong in a way only a real
Observation log revealed.

The amendment is recorded in three stages because it was found in three, each
correcting the one before it against measurement. Nothing is removed silently;
the superseded reasoning is quoted, including two claims of my own that later
measurement falsified.

### Amendment 1 — Zero attention removes a Resource from leadership, not from participation

**Old rule (implementation under v1.0).** A participant with no measured
attention was **Context** — "present, but never attended. Evo will not claim it
was worked in."

**Why it prevented the product from working.** Attention accrues only to
attentional Acts, and only across the interval to the next Act in the same
sitting. A change carries no interval, so a Resource that is only ever *changed*
has exactly zero attention however often it is changed. The rule therefore
reported the document a person saved four times a sitting across two sittings as
merely encountered — and **Context** is excluded from the restoration Context
Chain, so the file the work was *about* was not among the context Evo offered
when resuming it. That is §4's two ends collapsed into one: *merely encountered*
and *interacted with as part of something* both answered "not worked in", which
is precisely what Requirement 8 forbids.

**New rule.** Leadership and participation are separate questions and MUST be
asked separately. For a participant with no measured attention:

- if a person was witnessed acting on it during this work, Evo could not measure
  *how long* but knows *that* it happened: **Reference**;
- otherwise all Evo saw was a change it cannot attribute to anyone, and the
  question becomes whether the changes happen *because* this work is happening.

### Amendment 2 — Locality must be measurable before it can promote

**Old rule (Amendment 1 as first written).** An unattended change promoted to
**Supporting** when its *locality* — the share of the sittings it was witnessed
in that were sittings of this work — reached a strict majority.

**Why it prevented the product from working.** Validated against a real
Observation log, the rule admitted pure machine churn: several
`…/event-log-data/logs/FUS/<uuid>-…-eap.log` files and a photo library's internal
caches appeared as participants in a person's body of work, with zero attention
and locality `1.00`. The reasoning was locally valid and the measurement was
wrong. A share computed over a single witnessed sitting is `1.0` whatever the
Resource is, so locality was reporting the *absence of contrary evidence* as the
*presence of supporting evidence*.

**New rule.** Locality MUST be informative before it may promote — the Resource's
witnessed life must span enough sittings for the share to distinguish anything.
This costs a genuine draft nothing, since a Resource the work keeps returning to
is witnessed in several sittings by definition. It is a question about whether
evidence exists, not about its magnitude, so it adds no parameter
(Requirement 5).

### Amendment 3 — A change must be corroborated by attention, not by other changes

**Old rule (Amendments 1–2).** As above: locality, once measurable, was
sufficient.

**Why it prevented the product from working.** Amendment 2 was necessary and, on
the same real log, **not sufficient**. It was written believing the ephemeral
logs were "written once under a freshly generated name and never seen again, so
they are witnessed in exactly one sitting". Re-derived, that was true of the
photo library's caches — one witnessed sitting each, correctly demoted — and
**false** of the JetBrains telemetry logs: each was witnessed in *two* sittings,
satisfied the new admissibility test, and remained **Supporting** in a person's
body of work with zero attention. That claim is corrected here rather than left
standing.

The reason those Resources survived is that they corroborate *each other*. A
tool writing several similarly-named files per run gives each of them a large
shared vocabulary with the other three, so a corroboration test taken over *all*
members is satisfied entirely within a set the person never touched.
Corroboration among all members is the right test for **membership** and the
wrong one for **attributing a change**. And a telemetry log listed in Home's
sidebar as something the work was *used with* is §1's failure in miniature: a
Resource surfaced as work on the strength of having been written nearby.

**New rule.** An unattended change MAY be promoted to **Supporting** only if
something the person was demonstrably active in corroborates it — attention, or a
witnessed human act, on a member sharing its wording, location, or a declaration.
The two conditions are independent and both necessary:

- **locality** refuses churn that is *co-located* with the work but runs
  regardless of it;
- **attention-corroboration** refuses churn that is merely *coincident* with it.

Below either, Evo says the Resource was present and claims nothing further:
**Context**, which keeps its meaning and stays reachable. Neither condition names
an application, path, domain, or extension (Requirement 3), and neither adds a
parameter (Requirement 5). Where Evo measured no attention at all, nothing
promotes — which is the honest answer, and Requirement 13's required behaviour.

**Known cost, accepted deliberately.** Distinctive shared vocabulary is measured
in absolute IDF nats, so a token every Resource on the machine carries is worth
zero. In a corpus small enough that every token is universal, a person's own
document and a telemetry log are genuinely indistinguishable in the available
evidence, and the document is reported **Context**. This is a choice between two
errors made in the direction of under-claiming, pinned by
`a_change_with_no_evidence_but_timing_is_reported_as_merely_present`. The
Resource remains a member of the body of work and remains listed on Home; what it
loses is its place in the restoration Context Chain.

**Reconciliation.** Requirement 8 below carries the new rule normatively.
Architecture §11 and IS-0011 describe role assignment consistently with it. The
mechanism is `evo_engagement::engagement::assign_roles`, which carries the same
record at the point of definition (Requirement 5, clause 2).

**Tests.** `a_repeatedly_changed_resource_took_part_in_the_work` (a genuine draft
still promotes where evidence exists),
`a_change_with_no_evidence_but_timing_is_reported_as_merely_present` (the small
corpus case, pinned as a trade), and the real-log harness
`evo-daemon/examples/probe_attribution.rs`, which is why the discriminator was
chosen by measurement rather than by reasoning.

---

# Abstract

This RFC defines **Engagement**: the level of Evo's semantics that answers *what
was going on*, sitting between the resources Evo witnessed and the bodies of
work it presents as resumable.

Engagement exists because the previous model had no such level, and its absence
was the single root cause of Evo's central product failure. Without a level that
reasons about relationships between resources, the only available claim about a
resource was that it had been seen before. That is a counter, not a claim about
work, and everything a computer touches twice satisfies it. Home therefore
mirrored observation history: one Workspace per resource, a music player and a
chat window and a search each presented as a body of work to resume.

This RFC defines what Engagement must guarantee, independent of implementation.

---

# Motivation

## The semantic ladder

Evo's semantics form a ladder. Each rung is a strictly stronger claim than the
one below it, and the rungs are not interchangeable:

```
Observation    what was witnessed
Evidence       what the witnessing establishes
Resource       the thing that was witnessed
Engagement     what was going on
Workspace      a body of work worth returning to
Restoration    getting back into it
```

The prior architecture named the first three rungs and the last two. It did not
name the middle one. With no rung between *Resource* and *Workspace*, the
implementation stepped directly from one to the other, and the step had to be
justified by something. The only thing available was recurrence — "witnessed
twice" — so that became the rule.

Collapsing a rung does not remove the question the rung answered; it answers it
badly. The observable consequence was a Home surface that listed resources and
called them work.

## What Engagement must answer instead

> Were these things being used *together*, for *something*, by a person who was
> actually *there*?

That is a claim about relationships among heterogeneous resources over time. It
cannot be derived from any resource in isolation, which is precisely why no
per-resource rule could ever have produced it.

---

# Definitions

**Act.** One witnessed interaction with one Resource at one moment, carrying a
*character* that records what kind of witnessing it was. Derived from an
Observation; never itself persisted as ground truth.

**Character.** Whether an Act is evidence of a *person's* attention
(`Attentional`, `Deliberate`) or merely of something happening (`Incidental`).
A file rewritten by a background process and a window a person chose to look at
are both witnessed; they are not the same evidence, and the distinction is
structural rather than a list of applications.

**Episode (sitting).** A maximal run of Acts with no gap longer than the
*presence horizon*. An Episode is the unit within which Evo may claim the person
was continuously present.

**Attention.** Time credited to a Resource for an interval during which it was
the subject of the person's attention, credited only when the interval is short
enough to be one continuous stretch of attending. Silence is not attention.

**Affinity.** A bounded measure of how related two Resources are, composed from
independent kinds of evidence and normalized against ubiquity.

**Engagement.** A set of Resources that the evidence best explains as having
been used together for one thing, together with each Resource's measured role in
it.

**Participant.** One Resource's membership in one Engagement, carrying the
measurements the membership and role were decided from.

**Role.** A Participant's importance *to resuming*, distinct from its importance
to the work.

**Workspace.** A projection of an Engagement into the restorable form the rest
of the system consumes. Under this RFC a Workspace is derived, not stored.

---

# Behavioral Contract

## Requirement 1 — A body of work is a claim about relationships

An Engagement MUST be derived from relationships *among* Resources.

No property of a single Resource considered alone — how often it was witnessed,
how long it was attended, what it is called, what application produced it, what
scheme or extension it carries — MUST EVER be sufficient to originate an
Engagement.

**Consequence.** A single Resource, however heavily attended, is not a body of
work. Sustained attention to one window across many sittings yields nothing to
resume, because there is no context to reload.

---

## Requirement 2 — Three independent necessary conditions

Evo MUST NOT present an Engagement as resumable work unless all three of the
following hold independently:

1. **Relationship.** More than one Resource participates.
2. **Return.** At least one participant was attended by a person on more than
   one occasion — the *return* in activity → interruption → return.
3. **Depth.** There was at least one single sitting in which the Engagement
   received sustained attention.

Each condition MUST be necessary, not contributory. They MUST NOT be combined
into a single score in which a large value of one compensates for the absence of
another, because each expresses a different limit on what Evo can honestly
claim, and a weighted sum would let one limit be bought off with another.

**Depth MUST be a per-sitting quantity, never a lifetime total.** A lifetime
total is inflated by frequency, so a habit — glancing at the same few things for
seconds at a time, dozens of times — accumulates more of it than an afternoon of
genuine work, and Evo would present the habit as the work.

---

## Requirement 3 — No knowledge of applications, domains, or professions

No stage of Engagement derivation MUST contain, consult, or be tuned against:

- application or process names;
- domains, hosts, or URL patterns;
- file extensions, directory names, or path prefixes;
- any notion of "productive" or "unproductive" software;
- any model of what a profession's work looks like;
- any allowlist or blocklist of any of the above.

This is prohibitive, not aspirational. The same music player is background noise
for one person and the entire job for another; the same messaging application is
an interruption on Monday and the substance of the work on Tuesday. A system
that encodes either reading is wrong for whoever it guessed against.

**The one principle admitted in its place:**

> Something that correlates with everything correlates with nothing.

Every relatedness signal MUST be normalized against ubiquity, so that a Resource
present beside all activity ends related to none of it, while the same Resource
used with one specific set of things and nothing else becomes a full participant.
This mechanism never needs to know what it is looking at.

**Consequence.** A Resource is excluded from a body of work by the *structure of
the evidence about it*, never by its identity. The same Resource that forms no
work when attended alone becomes a full participant — including a Primary — when
the evidence places it inside one.

---

## Requirement 4 — Temporal structure is admissible evidence

Temporal relationships between Acts ARE legitimate evidence of work, and
Engagement derivation MAY depend on them.

This reverses the prior contract's rejection of temporal proximity (RFC-0003
Requirement 2, amended in v2.0). Work has temporal structure: activity,
continuation, interruption, return, related activity, progression. Refusing to
read it discards the strongest available signal that two heterogeneous resources
belong to one thing — often the *only* signal, since a document's name and a
terminal's name genuinely have no words in common.

Two constraints bound the reversal:

- **Proximity alone MUST NOT constitute a body of work.** "Everything within N
  minutes is one Workspace" is forbidden: it merely relocates the failure from
  one-resource-per-Workspace to one-sitting-per-Workspace.
- **Presence MUST NOT be inferred across silence.** An interval longer than the
  bound for one continuous stretch of attending contributes nothing, rather than
  contributing its bound. Crediting a long gap is how "left open overnight"
  comes to outrank real work.

---

## Requirement 5 — Every parameter states the limit it expresses

Engagement derivation MAY use numeric parameters. Every parameter MUST:

1. express a **limit of what Evo can honestly claim to have witnessed**, not a
   preference about applications, content, or professions;
2. carry that justification in the source, at the point of definition;
3. be **configurable**; and
4. be **varied by tests**, so that behaviour is demonstrably a function of the
   evidence rather than of the number.

A parameter that cannot be given a semantic justification of this kind is a
magic threshold and MUST NOT be introduced. Where a parameter encodes a product
judgement rather than an evidential limit, it MUST say so plainly rather than
present itself as a measurement.

---

## Requirement 6 — Grouping must both converge and separate

Engagement derivation MUST satisfy both directions, and neither at the other's
expense:

- **Convergence.** Heterogeneous Resources used together for one thing — a
  document, a terminal, a browser page, a spreadsheet, a drawing, a resource
  type Evo has never encountered — MUST converge into one Engagement.
- **Separation.** Two unrelated activities MUST remain distinct even when they
  share an application, a directory, a repository, a host, or a sitting.

Grouping MUST NOT use a linkage rule under which a single incidental
relationship fuses two otherwise-unrelated groups. Two activities inside one
repository, and two activities inside one application, are the cases this
requirement exists to protect.

---

## Requirement 7 — No resource type may require its own code

Engagement derivation MUST operate on Resources whose type it does not
recognize.

A Resource type Evo has never encountered MUST participate in attention,
relatedness, grouping, roles, and naming without any type-specific code being
added. Where a Resource's shape cannot be determined from the evidence, it MUST
be treated as an opaque subject and MUST still participate.

**Consequence.** Support for a new kind of work is not a feature to be
implemented per application. It is the default.

---

## Requirement 8 — Internal role structure

Not everything relevant to a body of work is equally important to restoring it.
An Engagement MUST distinguish, as separate and separately-derived states:

| Role | The claim it makes |
|---|---|
| **Continuation** | where the work resumes |
| **Primary** | important to resuming |
| **Supporting** | belongs to this body of work |
| **Reference** | interacted with as part of this |
| **Context** | merely encountered alongside it |

These MUST NOT be collapsed into a single boolean, a single score, or a
membership flag. Each answers a different question, and the restoration
selection depends on the distinction.

Role MUST be distinct from strength-of-membership. How firmly the evidence
places a Resource inside a body of work, and how important that Resource is to
resuming it, are different quantities and MUST be separately available.

## 8.1 — Attention decides leadership; participation is decided separately

Measured attention MAY decide which members lead a body of work
(**Continuation**, **Primary**). It MUST NOT be the sole test of whether a member
took part in it at all.

The two must be separated because attention is only measurable for Acts that
carry an interval. A Resource Evo only ever saw *change* has zero attention
however often it changed, so a rule reading attention alone answers *merely
encountered* for a document the person saved repeatedly — collapsing the two ends
of §4 that Requirement 8 exists to keep apart.

## 8.2 — An unattended change requires attributing evidence, not merely coincident evidence

Where Evo measured no attention on a member and witnessed no human Act on it, all
it holds is that the Resource *changed* during this work. Such a member MAY be
presented as belonging to the body of work (**Supporting**) only if the evidence
attributes the change to the work rather than merely placing it alongside it.
Both of the following MUST hold:

1. **The change is local to this work.** The share of the Resource's witnessed
   life that falls inside this body of work must be a strict majority — the
   changes happen when this work happens, not regardless of it — **and** that
   share MUST be measurable: a share computed over too few witnessed sittings to
   distinguish anything MUST NOT be read as evidence. Absence of contrary
   evidence is not presence of supporting evidence.

2. **The change is corroborated by attention.** Something the person was
   demonstrably active in — a member with measured attention, or one Evo witnessed
   a human Act on — must share this Resource's wording, location, or a
   declaration. Corroboration taken over *all* members is the correct test for
   membership and MUST NOT be used here: a set of Resources nobody ever attended
   can satisfy it entirely among themselves, which is how machine churn reaches a
   surface that promises work.

Failing either condition, the member is **Context**: Evo says it was present and
claims nothing further. Neither condition may consult application identity, path
shape, domain, or file extension (Requirement 3), and neither introduces a
parameter beyond the locality majority itself (Requirement 5).

Where Evo measured no attention anywhere, no unattended change promotes. That is
required, not incidental: Requirement 13 forbids substituting a fabricated
structure for a missing measurement.

---

## Requirement 9 — Naming is a claim about subject, not position

Where Evo presents a name for a body of work, that name MUST be:

1. **witnessed** — a name Evo actually observed, or a slice of one. Evo MUST NOT
   compose, summarize, or invent wording;
2. **chosen for what the members have in common**, where the evidence supports
   such a choice; and
3. **honest about its own basis** — whether the choice rested on measured shared
   vocabulary MUST be reportable.

Attention share MUST NOT be used to choose a name. Attention share answers
*where was the person*, which is a claim about position; a name answers *what is
this*, which is a claim about subject. Substituting the first for the second
presents a measurement as a meaning — and it is the specific substitution that
produced the failure this RFC exists to end.

Where no subject evidence is available, the name MUST be taken from a Resource
the evidence never witnessed outside this body of work, in preference to one it
did. This is a difference in kind rather than a threshold: a Resource the person
also uses elsewhere is not what *this* work is called, however much of this
work's time it happened to hold.

A body of work Evo chose to present MUST NOT be left nameless.

---

## Requirement 10 — Selective restoration

Restoration MUST open the minimum that re-establishes the work, and MUST
preserve access to the rest.

Only Resources whose role claims they are where the work continues, or important
to resuming it, MUST be opened. Every other participant MUST remain reachable as
context without being opened.

Opening every member of a body of work is a defect, not thoroughness: a person
returning to sixteen resources at once has been handed the reload cost the
product exists to remove.

---

## Requirement 11 — Declaration outranks inference

An explicit statement by the user about their own work is ground truth and MUST
outrank every measurement defined by this RFC.

Where a declaration speaks, Engagement derivation MUST NOT contradict it,
silently narrow it, or undo it on later evidence. Where a declaration does not
speak, inference proceeds normally: Evo MUST NOT require declaration for ordinary
operation, because observing and reconstructing without being told is the entire
value of the product.

Declaration order carries meaning where the user supplied an order.

---

## Requirement 12 — Determinism and replay equivalence

Engagement derivation MUST be a pure function of the canonical Observation
history and the parameters.

It MUST NOT depend on wall-clock time, randomness, arrival order, iteration
order of unordered collections, or any state outside its inputs. Replaying a log
MUST reproduce identical Engagements, identical roles, and identical names.

Live processing and replay processing MUST be the same computation, not two
implementations that are intended to agree. Where a parameter set or derivation
rule changes, the change MUST be versioned such that a historical result remains
reproducible.

---

## Requirement 13 — Honest degradation

Where a signal is unavailable, Engagement derivation MUST produce a weaker claim,
never a fabricated one.

Removing or disabling any relatedness signal MUST reduce what Evo asserts —
fewer bodies of work, weaker roles, a name reported as not-vocabulary-derived —
and MUST NOT cause Evo to substitute a guess. Absence of evidence MUST be
reported as absence, not filled in.

---

## Requirement 14 — Explainability

Every conclusion MUST be traceable to the evidence that produced it.

For each participant, the measurements its membership and role were decided from
MUST remain available after derivation. It MUST be possible to answer "why is
this here, and why is it ranked this way" from retained values rather than by
re-deriving or guessing.

Where the evidence for a claim is weak, Evo MUST say so rather than present the
claim at full strength.

---

# Forbidden Behavior

An implementation of this RFC MUST NEVER:

- originate a body of work from a property of a single Resource;
- present observation history as bodies of work;
- consult application identity, domain, extension, or profession at any stage;
- maintain an allowlist or blocklist of applications, sites, or content;
- introduce a numeric threshold without a stated evidential justification;
- combine the three necessary conditions into a compensating score;
- credit attention across an interval too long to be one stretch of attending;
- fuse two groups on a single incidental relationship;
- require type-specific code for a new Resource type;
- name a body of work by attention share;
- compose, summarize, or invent a name;
- open every member of a body of work on restore;
- contradict or silently undo an explicit user declaration;
- require declaration for ordinary operation;
- depend on wall-clock time, randomness, or arrival order;
- fabricate a signal that is unavailable;
- discard the measurements a conclusion was derived from.

---

# Architectural Consequences

## Workspace becomes a projection

Under this RFC a Workspace is a **projection of an Engagement**, derived from the
Observation log on read. It is not a persisted, incrementally-mutated container.

This is not a departure from Evo's computational model — it is the first
implementation that obeys it. Architecture §2 states that state is never stored
but derived by replaying an interpretation function over an immutable log. The
persisted-Workspace rules in Architecture §3, §6, and §7 were an explicit
exception carved out of §2 for restoration speed, and that exception is what
permitted observation history to be mirrored into Home: once a Workspace existed
as a durable row created on first sighting, nothing downstream could unmake it,
and every later reading of the evidence had to be reconciled against a decision
taken when the evidence was one observation long.

Deriving instead of storing means a better interpretation improves *all* history
immediately, with no migration and no reconciliation. See Architecture §3, §6,
§7 as amended.

## Formation is not per-observation

Because a body of work is a claim about relationships across a whole history, it
cannot be decided one Observation at a time against a small local candidate set.
Derivation reads the history. See Architecture §5 as amended, which previously
required the opposite.

## Repository co-membership loses its special status

RFC-0012 made shared repository membership a distinguished relational signal.
Under this RFC it is one structural relationship among several, carrying no
privileged weight, for two reasons:

1. It is Git-specific, and most work is not in a repository. Designing the
   relational model around it makes browser+PDF+terminal work, or
   spreadsheet+browser+notes work, second-class.
2. It over-groups precisely where separation matters most: two unrelated
   activities in one repository share it completely (RFC-0014 Requirement 6).

RFC-0012 is superseded in full.

---

# Non-Goals

Engagement is not responsible for:

- determining the user's goals, motivations, or mental state;
- judging whether work was valuable, productive, or complete;
- predicting future work;
- summarizing or narrating what the user did;
- executing restoration.

Engagement answers one question:

> "Which Resources does the evidence best explain as having been used together
> for one thing, how much does each matter to resuming it, and is there enough
> evidence to say so at all?"

---

# Compatibility

Any implementation conforms if it preserves every behavioral guarantee herein.
No specific segmentation rule, affinity composition, clustering algorithm, or
parameter value is prescribed — only that whatever is used satisfies
Requirements 1–14 and states its own justification.

---

# Rationale

The failure this RFC responds to was not a bug and would not have been fixed by
any number of bug fixes. Every layer worked as specified; the specification had
a hole in the middle of it, and the layers faithfully carried a resource-shaped
claim all the way to a surface that promised work.

The correction is to name the missing level and give it the only question it can
answer honestly — whether these things were being used together, for something,
by someone who was there — and then to insist that every mechanism serving that
question be blind to what it is looking at. Blindness is not a limitation here.
It is the only property that lets one model serve a lawyer, a machinist, a
student, and someone doing something nobody anticipated, without having guessed
about any of them.
