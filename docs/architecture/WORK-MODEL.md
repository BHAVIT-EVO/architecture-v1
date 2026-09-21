# The Work Model

Status: replacement architecture implemented in the relevant projection path;
capture/state and transport execution remain bounded by available evidence.
Authority for `evo-engagement`, `evo-workspace`,
`evo-retrieval`, `evo-restoration`.

Every number in this document was measured against the real Observation log on
this machine: 1245 observations, 425 canonical resources, 61 sittings,
2026-08-18 → 2026-08-21. Not against fixtures.

## 0. Decision — the former implementation was fundamentally incapable; the
replacement is now authoritative

The former resource/context partition was removed. Work identity is now formed
from overlapping, non-transitive nuclei and propagated as one stable `WorkId`
through Engagement, Workspace, retrieval, and restoration projections.

Measured on the same corpus:

- 140 occurrence contexts are derived;
- 108 contain at most three human-attended subjects;
- 129 contexts nevertheless become one connected component;
- that component projects as a 136-member Workspace containing unrelated code,
  job-search, communication, media, meetings, and machine output;
- PlayStation research in one sitting is split into several small bodies because
  its contexts do not recur enough to satisfy the cross-sitting merge rule.

Removing one bridge condition preserves all 95 unit tests but leaves the
129-context component intact. Forming work only from selective seeds eliminates
the giant component but produces 54 projected Workspaces, including WhatsApp and
Login. Therefore no local threshold change solves the problem: connected
components over resources or contexts force an exclusive, transitive identity
decision before the evidence supports one.

The exact abstraction that must be replaced is:

```text
pairwise compatibility → transitive connected component → one Engagement
```

The replacement is:

```text
occurrence context
  → graded links to zero, one, or several work hypotheses
  → work-local artifact relevance
  → separate restoration role/state
```

### 0.1 Canonical derived objects

These are derived, local, deterministic objects. They are not new Observation
schemas and are not persisted as ground truth.

```rust
type ContextId = u128; // digest of the ordered canonical acts in the context
type WorkId = u128;    // digest of the hypothesis's first accepted nucleus

struct WorkHypothesis {
    id: WorkId,
    contexts: BTreeMap<ContextId, WorkContextLink>,
    signature: BTreeMap<String, f64>,
    first_active: SystemTime,
    last_active: SystemTime,
    standing: Standing,
}

struct WorkContextLink {
    relevance: f64,
    contradiction: f64,
    evidence: Vec<WorkEvidence>,
}

struct WorkArtifactLink {
    subject: String,
    relevance: f64,
    attributed_contexts: BTreeSet<ContextId>,
    evidence: Vec<WorkEvidence>,
    role: ResourceRole,
}

enum WorkEvidence {
    Declared,
    RepeatedSelectiveCoactivity,
    LocalHumanCoactivity,
    DistinctiveVocabulary,
    SharedStructure,
    SharedOnlyUbiquitousSurfaces,
    CompetingContextSignature,
    MachineOnly,
}
```

`relevance` and `contradiction` are separate measurements. Negative evidence is
not represented by merely lowering a positive score, because “no evidence” and
“evidence these are different works” must remain distinguishable and explainable.

### 0.2 Formation algorithm

The implementation MUST perform these stages in this order:

1. **Canonicalize observations and artifacts.** Keep the current IdentityIndex,
   Episode segmentation, AttentionLedger, Declarations, and AffinityGraph.
2. **Build occurrence contexts.** A context is a bounded local run of canonical
   acts. It is evidence, never a body of work and never an exclusive bucket.
3. **Measure context selectivity.** For every subject, compute its context
   prevalence and inverse-context frequency. A surface present across many
   different context signatures contributes little identity evidence without
   being removed from memory.
4. **Measure signed context compatibility.** Positive evidence is direct human
   coactivity, repeated selective composition, and corroborating witnessed
   vocabulary/structure. Negative evidence is a bridge consisting only of
   ubiquitous surfaces, mutually incompatible selective cores, machine-only
   activity, or strong competing affiliations. Time and recency may bound local
   evidence; neither may establish same-work identity.
5. **Create nuclei without transitive closure.** Start from mutually supported
   context pairs or one sufficiently deep/coherent context. Merge two hypotheses
   only when:
   - each hypothesis has selective evidence supporting the other;
   - mean positive compatibility clears the formation floor;
   - no context pair clears the contradiction veto;
   - the merged signature remains internally coherent.
   A single bridge can therefore never merge A with C through B.
6. **Assign contexts multi-label.** Score every context against every hypothesis.
   Retain every link above the memory floor. More than one hypothesis may retain
   the same context; none is also valid.
7. **Aggregate artifact links per work.** Only activity from attributed contexts
   contributes to the artifact's work-local attention, last-seen state, and
   specificity. The same canonical artifact may produce independent links to X,
   Y, and Z.
8. **Project standing.** `Remembered` means evidence retained without claiming a
   body of work. `Continuable` requires a coherent human-grounded hypothesis;
   either return across contexts or one sustained multi-artifact context is
   sufficient. `Restorable` additionally requires an unambiguous work-local
   stopping point. A repository, repeated sitting, or predefined category is
   never required.

### 0.3 Work identity and evolution

`WorkId` comes from the immutable founding names of the accepted nucleus, not
from a mutable canonical artifact name or Workspace projection. Repeated
contexts converge to the same identity; membership may expand, contract, and
overlap without changing it. Full split/merge lineage aliases are not yet
persisted; any future durable lineage must be added before derived hypotheses
are stored independently of replay.

### 0.4 Belonging, restoration, and captured state

Artifact relevance answers “did this belong to the work?” and MUST be retained
independently of restoration action.

Restoration role answers “what should happen now?”:

- `Continuation`: the last meaningful human occurrence attributed to this work;
- `Primary`: other high-confidence places necessary for immediate continuation,
  capped by `max_primary`;
- `Supporting` / `Reference`: remembered and shown, closed by default;
- `Context`: retained history with no claim that it took part.

Continuation does not require recurrence. A single sustained shopping/research
sitting can have a stopping point. Rich state such as cursor, function, terminal
buffer, selected spreadsheet cell, or chat turn can only be restored when the
Observation schema captures it; until then Evo reports the last witnessed
artifact and must not fabricate finer state.

### 0.5 Retrieval

The existing IDF/specificity retrieval scorer survives, but it MUST score
`WorkHypothesis.signature`, explicit aliases, and witnessed artifact vocabulary,
not a contaminated Workspace member set. A clear winner resolves; close winners
remain `Ambiguous`; zero evidence is `NotFound`. Successful historical aliases
may be learned locally as declarations, never inferred silently from an LLM.

### 0.6 Exact implementation boundary

| File | Required change |
| --- | --- |
| `crates/evo-engagement/src/engagement.rs` | Keep `ActivityContext`, `Participant`, `Standing`, roles, and assembly concepts; consume authoritative hypotheses and preserve exact occurrence contexts. |
| `crates/evo-engagement/src/hypothesis.rs` | New `WorkHypothesis`, signed context evidence, nucleus formation, deterministic split/merge lineage. |
| `crates/evo-engagement/src/membership.rs` | Multi-label context-to-work and artifact-to-work aggregation is currently implemented in the Engagement assembly path; a dedicated module remains optional. |
| `crates/evo-engagement/src/affinity.rs` | Keep resource evidence; add context-selectivity and contradiction measurements. Resource affinity becomes evidence, not a clustering command. |
| `crates/evo-engagement/src/lib.rs` | Pipeline becomes identity → episodes → contexts → hypotheses → memberships → engagements. |
| `crates/evo-workspace/src/projection.rs` | Project `WorkId`; stop deriving identity from the earliest artifact sighting. Attach every retained artifact link, regardless of auto-open role. |
| `crates/evo-retrieval/src/retrieval.rs` | Score work signatures and aliases; keep current ambiguity contract. |
| `crates/evo-restoration/src/derivation.rs` | Consume the work-local continuation and roles; keep insufficient outcomes. |
| `crates/evo-daemon/src/understanding.rs` | Carry hypothesis explanations/aliases beside Workspace, title, and standing. |
| `crates/evo-desktop/src/state.rs`, `app.rs`, `ui.rs` | Consume role-separated restoration selection; Home may still use presentation search, while daemon retrieval exposes stable work resolution. |

No canonical-data migration is required. Understanding is already a pure
projection of the append-only Observation log. The derived Workspace identity
format changes, so any optional derived cache must be invalidated and rebuilt;
the Observation and Artifact stores remain untouched.

### 0.7 Acceptance suite

Implementation is complete only when deterministic replay passes all of these:

- three interleaved coding projects with a shared terminal and AI conversation;
- mixed coding and research with a one-time reference;
- one-sitting browser-heavy purchase research returning as one body;
- document-centric and terminal-heavy work;
- incidental chat/music/application surfaces that belong weakly but found
  nothing and auto-open nowhere;
- one artifact belonging strongly to two works;
- rapid X → Y → X switching inside one Episode;
- long-gap return to the same work;
- one large work with many supporting resources;
- many works sharing one hub without a giant component;
- retrieval of X, then research paper, then travel planning from one unchanged
  reconstruction;
- forward, reversed-arrival, and full replay equivalence.

The real-corpus gates are behavioral, not count targets: no catch-all component,
PlayStation research retained coherently, shared resources present in more than
one work, every human-attended resource either linked or explicitly Remembered,
and auto-open bounded independently of membership size.

---

## 1. What Evo is

Evo reconstructs **what a person was trying to accomplish**, so they can return
to it. It is not an activity log, a dashboard, a history viewer, or a folder
generator. The only thing a person should have to remember is what they want to
do; the environment that thought belongs in should already be there.

A person holds several bodies of work at once, and they overlap. Evo's job is to
tell them apart, keep them alive across days, and put the person back into one
of them — selectively.

Evo learns what matters from the person's own behaviour. It contains no list of
applications, domains, professions, file types, or categories.

## 2. What is fundamentally wrong with the current model

**Membership is computed as a partition of resources.** `cluster()` runs
connected components plus average-linkage agglomeration over the whole affinity
graph and returns disjoint sets. Each set becomes at most one `Engagement`, at
most one `Workspace`. A resource therefore belongs to exactly one body of work,
by construction.

That single decision produces every failure, and all of it is measured:

| What the log shows | Measured |
| --- | --- |
| Resources in more than one body of work | **0 of 425** |
| Resources with measurable affinity to **two or more** bodies of work | **152 of 425** |
| Relationships discarded to satisfy the partition | **every one but the strongest, per resource** |
| Clusters formed but never presented as work | **330** |
| Resources with witnessed human attention that reached no body of work | **24** (6 min of attention) |
| Bodies of work with no resume point | **9 of 11** |
| Bodies of work with a complete restoration plan | **2 of 11** |
| Resources restoration would open, in the whole corpus | **5** |

Three named consequences:

1. **Forced choice destroys the ambiguity the product exists to represent.**
   `https://claude.ai/chat/0743…` measures 0.64 to one body of work, 0.58 to a
   second, 0.54 to a third. It was assigned to one and the other two
   relationships were thrown away. This is the API-docs example, failing 152
   times in four days.

2. **Silent loss is the default path.** A cluster that fails
   `is_significant` is pushed to `encountered` and leaves the product. 330
   clusters and 341 resources ended there. The one-time PDF read for thirty
   seconds is not "kept but not promoted" — it is gone.

3. **Because the cluster is the unit, incidental resources title the work.**
   Nine of eleven bodies of work are named after a single member. The afternoon
   spent drafting `RFC-0011-continuation-evidence-contract.md` is called
   **"‎WhatsApp"**. The `pipeline.py` development work is called
   **"www.google.com/search"**. The job application work is called
   **"Login"**. The mechanism intended to prevent this (`choose_title`,
   shared vocabulary, locality fallback) measures nothing on real history and
   falls through. Contamination is not a cosmetic problem here; the incidental
   resource has become the identity of the work.

Two further defects follow from the same root:

- `AffinityEvidence::score()` returns `0.0` when interleaving and co-episode
  are both zero, **even when lexical or structural evidence is positive**. That
  destroys a record instead of downranking it.
- Work identity is `FNV(founding sighting of the earliest-witnessed member)`.
  Under a partition, membership churn changes which member is earliest, so
  identity does not survive the work evolving.

And retrieval does not exist: `Retrieval::retrieve` discards its trigger and
returns every workspace. "Take me back to Automation X" cannot resolve.

## 3. The model that replaces it

**A body of work is a Thread: a seeded, overlapping, graded membership over the
evidence graph.** Resources are never partitioned.

```
RAW OBSERVATION → ARTIFACT IDENTITY → EVIDENCE → RELATIONSHIPS
  → SEEDS → THREADS (overlapping) → SPECIFICITY → STANDING
  → WORK IDENTITY → RELEVANT CONTEXT → RESUME POINT → RESTORATION PLAN
```

**Seeds.** A resource can *centre* a body of work only if a person was
witnessed doing something there: `human_acts >= min_revisits` (they came back)
or a sitting's worth of attention (they really worked there once). On the real
log that is ~100 of 425 resources. Everything else can join work but cannot
define it. Nothing about *what* the resource is enters this test.

**Thread formation.** Clustering still exists — average linkage at
`cohesion_floor` — but it runs **only over the seed subgraph**. Two attended
loci that are not sufficiently related stay separate: that is what keeps two
tasks in one Git repository distinct. Churn cannot inflate cohesion or form
giant clusters, because churn is not a seed.

**Membership is overlapping and graded.** For each thread `T` and *every*
resource `R`, `strength(R,T) = max over seeds s in T of affinity(R,s)`. `R`
joins `T` when that clears `affinity_floor` — and joins every other thread it
also clears. Admission by max, not mean: a reference page legitimately relates
to one locus of the work, not to all of them. There is no forced choice
anywhere in the pipeline.

**Specificity replaces the idea of a noise list.**

```
specificity(R,T) = strength(R,T) / Σ over all threads T' of strength(R,T')
```

A resource in one thread scores 1.0. A resource present beside six unrelated
bodies of work at similar strength scores ~0.17 in each. Purely a ratio of
measured affinities — no name, no domain, no category. This is how the music
player stays out of the automation work: not because Evo knows what a music
player is, but because *its presence does not discriminate between the
person's bodies of work*. And the same resource, inside work that is genuinely
about it, has high specificity there and leads.

Specificity gates: **titling** (a resource that belongs more elsewhere cannot
name this work), **leadership**, and **auto-open**. It never gates
**membership** — the relationship is always kept.

**Standing: three separate claims, never collapsed.**

```
Remembered   Evo witnessed this. Retrievable by name. Never auto-opened,
             not presented on Home as work.
Continuable  A real body of work: presented, named, with candidate context.
Restorable   Continuable, and one unambiguous continuation locus →
             safe to auto-open.
```

`encountered` is deleted. Every group that forms becomes a Thread carrying a
Standing. Nothing leaves the product. `50 remembered ≠ 50 opened` is expressed
by Standing plus role, not by discarding.

**Work identity survives evolution.** A thread is identified by the founding
sighting of its **earliest-founded seed**. Monotone in an append-only log:
gaining a seed never changes it, and an attended returned-to locus does not
stop being one. Membership can churn freely underneath a stable identity.

**Retrieval is real.** A trigger phrase is scored against each thread's own
vocabulary — the witnessed names of its members, IDF-weighted across threads,
each member's contribution weighted by its specificity. A token that appears in
eight threads carries almost nothing; a distinctive one decides. When the top
two candidates are within a margin, Evo returns `Ambiguous` and asks *which
one* rather than restoring the wrong work confidently.

**No LLM.** Every quantity above is a count, a duration, a ratio of counts, or
an IDF over witnessed strings. Deterministic, replayable, local. An LLM would
be an explicit architectural decision and none is needed here.

## 4. Automation X / Y / Z

Three automations interleaved through one afternoon, resumed the next day.

Each automation has at least one attended, returned-to locus — its script, its
console, its ticket. Those are seeds. Seeds of different automations are not
related above `cohesion_floor` to each other (different vocabulary, different
containers, interleaving that does not survive NPMI), so they form three
threads. Shared machinery — the same terminal, the same runner docs, the same
credentials page — joins **all three** with graded strength and low
specificity, so it is remembered in each and leads none. Next day, the trigger
"Automation X" matches X's distinctive vocabulary (the automation's own name
appears in X's member names and nowhere else), IDF makes that decisive, and X
resolves alone. Each thread keeps its own identity across the day gap because
its earliest-founded seed is unchanged.

## 5. Incidental resources

The music player, the messaging window, the browser, the mail tab are members
of every thread they were measurably beside — the record is kept, because "what
else was I looking at" is a real question. Their specificity in each is low
because they are beside everything, so:

- they cannot title a thread,
- they cannot be a seed, so they cannot found or identify one,
- they cannot be a continuation, so they never auto-open,
- they are reported as ubiquitous rather than deleted.

No production code path names any of them. The specificity of a music player in
a thread *about music* is high, and there it leads.

## 6. Shared artifacts

The API documentation page relates to work X at 0.91 and work Y at 0.57. Both
memberships are kept, with those strengths, and both are grounded: affinity is
a weighted combination of interleaving co-presence (a cosine over per-sitting
profiles), cross-sitting NPMI, IDF-weighted shared vocabulary of witnessed
names, and shared container — all counted, none scored by a model. Its
specificity is 0.61 in X and 0.39 in Y: strong support in both, leadership in
neither unless the rest of the evidence puts it there. Restoring X offers it;
restoring Y offers it; neither restoration had to take it from the other.

## 7. Selective restoration

Membership answers *what belongs to this work*. Restoration answers *what to
put in front of the person now*. They are separate functions over the same
record, and the second is deliberately much smaller.

Remember broadly: every measurable relationship is a member. Restore
selectively: rank members by role, then specificity, then attention within this
thread, then recency; auto-open only `Continuation` and `Primary`, capped at
`max_primary`; everything else is **withheld but named** — offered, not opened.
A thread with no unambiguous continuation locus is `Continuable`, not
`Restorable`: Evo says where the work is and declines to guess where it resumes,
rather than nominating the longest-attended window present.

## 8. What changes

| Crate / file | Change |
| --- | --- |
| `evo-engagement/src/params.rs` | new limits: membership floor, titling specificity floor, retrieval margin |
| `evo-engagement/src/affinity.rs` | `score()` stops hard-zeroing evidence it can still measure |
| `evo-engagement/src/engagement.rs` | `cluster()` partition → `seeds()` + seed-graph clustering + overlapping `memberships()`; new `Standing`; `Participant::specificity`; `encountered` deleted; titling gated on specificity |
| `evo-workspace/src/projection.rs`, `deterministic.rs` | identity from earliest-founded seed; a resource may attach to several Workspaces |
| `evo-retrieval/src/*` | implemented: IDF + specificity scoring, `Resolved` / `Ambiguous` / `NotFound` |
| `evo-restoration/src/derivation.rs` | resume-point eligibility reads specificity and Standing |
| `evo-daemon/src/understanding.rs`, `ui.rs` | carry Standing and thread vocabulary; wire retrieval |
| `evo-daemon/examples/` | a real-data harness that proves the properties above on the persisted log |
| `docs/` | this document; RFC-0014 R13 and the affinity commentary corrected on exclusion-under-uncertainty |

No migration is required for stored data. Understanding is already a pure
projection of the append-only Observation log — nothing derived is persisted —
so replay-equivalence is preserved by construction and the corpus is untouched.
