IS-0021 — Restoration Derivation

Status: Frozen
Version: 1.2

Depends On

* Constitution
* Product
* Architecture (as amended, Amendment 1)
* Architectural Laws
* RFC-0003 — Workspace Formation Contract (v2.0)
* RFC-0006 — Restoration Contract
* RFC-0014 — Engagement Contract
* IS-0011 — Workspace Model (as amended, Amendment 1)
* IS-0018 — Committed Understanding
* IS-0019 — Restoration Model

⸻

Amendment Record

Version 1.2 — introduced by RFC-0014 (Engagement Contract), consequential to
Architecture Amendment 1 and IS-0011 Amendment A1.3.

Three frozen algorithms in §25 are amended: §25.2 (Resume Point), §25.3 (Context
Chain), and §25.5 (Next Step). Also recorded: the retirement of IS-0012 from the
dependency list.

The old rule. §25.3 required an **empty** Context Chain, and §25.2 required an
**insufficiency** outcome, whenever a selected Snapshot referenced more than one
Artifact. Both were justified by the same claim, quoted from §25.3:

> "the current Snapshot model contains no canonical
> supporting/reference/continuation relationship, so Workspace membership alone
> never establishes Context relevance."

§25.5 likewise admitted only an explicit designation (RFC-0011) as continuation
evidence.

Why it prevented the product from working. The claim was **true** of the Snapshot
model as it stood, and the conclusion followed from it correctly. This is worth
stating plainly: the old rule was not careless. It was a sound conclusion from a
premise that the model has since changed.

An Attachment now carries a role (IS-0011 A1.3) — the part the Artifact played in
the body of work — added precisely because a Workspace that could not distinguish
where work happens from what merely happened to be open forced Restoration to
treat every member identically. With the new field present but the old rule left
standing, the consequence was severe and exactly backwards: a *real* body of work
has many members by definition, so any genuine body of work would hit the
more-than-one-Artifact branch, fail to establish a Resume Point, and carry no
context. Restoration became structurally unable to restore precisely in the cases
that mattered, and could only succeed for a Workspace with a single member — which
is the degenerate case that is not a body of work at all.

The new rule.

* **§25.2 (Resume Point).** Derivable when the canonical input identifies a
  designated Artifact (RFC-0011) among eligible candidates, when exactly one
  Attachment of the selected Snapshot carries the Continuation role, or when the
  Snapshot represents exactly one distinct eligible Artifact. Declared intent is
  consulted first and outranks the role (RFC-0014 Requirement 11). Attachment
  order, Attachment Confidence, ArtifactId ordering, recency, and frequency are
  never Resume Point evidence — unchanged, and still forbidden.

* **§25.3 (Context Chain).** The Attachments of the selected Snapshot that played
  a part in the work, excluding the Resume Point itself, ordered by that part and
  then by canonical Snapshot order. Attachments carrying the Context role are
  excluded: a resource that was merely present while the work happened is not
  context *for* the Resume Point, and CC-4 asks for the minimum, not the maximum.
  The complete membership set remains on the Workspace for surfaces that want all
  of it, so nothing is lost — only not opened.

* **§25.5 (Next Step).** Established by the same evidence that establishes the
  Resume Point, when that evidence says something about *continuation* — a
  designation or the Continuation role. A single eligible Artifact establishes
  *where* to resume without saying anything about continuation, so the Next Step
  is then recorded as **missing rather than manufactured**.

The invariant the old rule was protecting is retained in full: **membership never
establishes relevance.** The *role* does, and the role is canonical Attachment
state derived from witnessed evidence, not an interpretation made inside
derivation. Where the evidence names nothing, derivation still records missing
evidence rather than ranking candidates by confidence, identifier, or attachment
order (§25.8 unchanged).

Dependency change. **IS-0012 is removed from the dependency list** — it is
superseded in full (see its Supersession Record). Restoration Derivation depends
on the *Workspace model*, not on the process that produced it, so nothing in this
specification's algorithms changes as a result.

Unchanged: §25.1 (common evidence rules), §25.1A (Continuation Surface
consumption, RFC-0013), §25.4 (Blockers — still zero, and never invented from
runtime or machine state), §25.6 (completeness), §25.7 (component membership),
§25.8 (explicit insufficiency), §25.9 (determinism), §25.10 (replayability).

Implementation: `evo-restoration::derivation`, whose module documentation carries
this same change record at the point of implementation.

⸻

Related:

* Workspace Replay specification
* IS-0016 — Historical Understanding Model
* IS-0017 — Historical Commitment
* IS-0018 — Committed Understanding
* Future Restoration Execution specification

⸻

1. Purpose

This specification defines the architectural boundary responsible for deriving a canonical RestorationPlan from canonical Workspace understanding.

The Restoration Derivation layer exists to transform an immutable Workspace Snapshot into a deterministic continuation strategy that minimizes cognitive reload.

The derivation layer SHALL compute restoration understanding.

It SHALL NOT execute restoration.

It SHALL NOT modify Workspace understanding.

It SHALL NOT modify Artifacts, Observations, Attachments, or historical Snapshots.

This specification establishes the contract for deriving:

* Resume Point
* Context Chain
* Blockers
* Next Step

This specification freezes the Restoration Derivation architectural boundary.

This specification also freezes the concrete derivation algorithms for Resume Point, Context Chain, Blockers, and Next Step in Section 25.

Those algorithms operate exclusively on the canonical inputs defined by IS-0011: the selected canonical Snapshot, the Workspace referenced by that Snapshot, the Snapshot's ordered Attachment Set, and the canonical Artifacts referenced by those Attachments.

Conformance to the concrete derivation algorithms SHALL be evaluated against Section 25, as defined by Section 24 and Section 28.

Insufficient canonical evidence is a normal derivation outcome as defined by Section 25.8. It SHALL NOT be converted into a guessed RestorationPlan.

⸻

2. Scope

This specification defines:

* Restoration Derivation;
* its canonical input boundary;
* its canonical output boundary;
* derivation responsibilities;
* derivation constraints;
* deterministic behavior;
* replayability;
* relationship to Workspace;
* relationship to Snapshot;
* relationship to RestorationPlan;
* treatment of insufficient understanding;
* prohibited inputs;
* prohibited inference;
* future algorithm requirements.

This specification does not define:

* OS automation;
* application launching;
* browser restoration;
* window positioning;
* desktop interaction;
* voice interaction;
* UI presentation;
* restoration scheduling;
* restoration execution;
* Workspace Formation;
* Artifact Identity;
* Observation interpretation;
* Retrieval;
* Learning;
* a new canonical Workspace object;
* a new canonical Restoration object.

⸻

3. Definitions

Restoration Derivation

The deterministic computation that transforms canonical Workspace understanding into one canonical RestorationPlan.

⸻

Restoration Input

The canonical Workspace understanding from which Restoration is derived.

The primary architectural input SHALL be the relevant immutable Snapshot representing the Workspace understanding to be restored.

Restoration Derivation MAY consume canonical lower-layer objects referenced by that Snapshot when required by the final derivation contract.

It SHALL NOT consume raw or non-canonical runtime state.

⸻

Restoration Output

The canonical RestorationPlan defined by IS-0019.

⸻

Resume Point

The first cognitive step required to continue work.

The Resume Point represents understanding rather than interface state.

⸻

Context Chain

The ordered supporting context required to understand the Resume Point.

⸻

Blocker

An unresolved condition preventing immediate continuation.

⸻

Next Step

The immediate continuation following the Resume Point.

⸻

4. Architectural Position

Restoration Derivation occupies the following boundary:

Canonical Observation History
             │
             ▼
      Artifact Resolution
             │
             ▼
       Workspace Formation
             │
             ▼
       Workspace Snapshot
             │
             ▼
   ┌─────────────────────────┐
   │ Restoration Derivation  │
   └────────────┬────────────┘
                │
                ▼
       RestorationPlan
                │
                ▼
      Future Execution Layer

Restoration Derivation SHALL operate strictly after Workspace understanding has been established.

Restoration SHALL NOT perform Workspace Formation.

Restoration SHALL NOT perform Artifact Resolution.

Restoration SHALL NOT reconstruct Workspace membership.

Restoration SHALL NOT rerun Attachment evaluation.

This preserves the architectural requirement that Restoration consumes Workspace understanding rather than performing new Workspace inference. IS-0019 — Restoration Model.md

⸻

5. Responsibilities

Restoration Derivation SHALL:

1. consume canonical Workspace understanding;
2. identify the Workspace represented by the input;
3. derive exactly one Resume Point when the canonical input is sufficient, and otherwise fail explicitly per Section 25.2 and Section 25.8;
4. derive an ordered Context Chain;
5. derive zero or more Blockers;
6. derive exactly one Next Step when canonical continuation evidence exists, and otherwise fail explicitly per Section 25.5 and Section 25.8;
7. construct exactly one canonical RestorationPlan when every required component can be established from canonical evidence, and otherwise fail explicitly per Section 25.6 and Section 25.8;
8. preserve Workspace identity;
9. preserve all lower-layer identities;
10. remain deterministic;
11. remain replayable;
12. minimize cognitive reload;
13. preserve explainability through canonical inputs.

⸻

6. Non-Responsibilities

Restoration Derivation SHALL NOT:

* collect runtime events;
* inspect raw OS state;
* interpret raw Observations;
* construct Artifacts;
* resolve Artifact Identity;
* modify Artifact Identity;
* construct Workspaces;
* modify Workspace Identity;
* create Attachments;
* modify Attachments;
* rewrite Snapshots;
* perform Workspace Formation;
* perform Workspace Replay;
* perform Retrieval;
* perform Learning;
* call language models;
* infer user intent from non-canonical runtime information;
* launch applications;
* open files;
* open browser tabs;
* move windows;
* manipulate the desktop;
* execute operating-system actions;
* produce UI state;
* produce voice output.

This follows the existing separation in which Workspace Formation constructs Workspace understanding and Restoration consumes that understanding. IS-0012 — Workspace Formation.md

⸻

7. Canonical Input

Restoration Derivation SHALL consume canonical Workspace understanding only.

The Architecture identifies the latest Workspace Snapshot as the source from which Restoration is planned. ARCHITECTURE.md

The relevant Snapshot SHALL be immutable.

The Snapshot SHALL be treated as historical computational evidence.

Restoration Derivation SHALL NOT modify it.

The derivation operation SHALL NOT require:

* current application state;
* current window state;
* current browser state;
* current filesystem state;
* current terminal state;
* current Git state;
* current user interface state;
* raw capture events;
* unaccepted Candidate Observations;
* transient runtime caches;
* external network state.

A future execution layer MAY inspect current operating-system state when executing a RestorationPlan, but that state SHALL NOT become an input to Restoration Derivation.

⸻

8. Canonical Output

Restoration Derivation SHALL produce exactly one RestorationPlan for one Workspace input when the canonical input is sufficient to satisfy every IS-0019 invariant.

When canonical evidence cannot establish a required component, Restoration Derivation SHALL fail explicitly, identifying the unmet invariant or missing canonical input, as required by Section 16 and Section 25.8.

The resulting RestorationPlan SHALL satisfy every invariant defined by IS-0019.

It SHALL contain:

* exactly one Workspace identity;
* exactly one Resume Point;
* one ordered Context Chain;
* zero or more Blockers;
* exactly one Next Step.

The resulting RestorationPlan SHALL be immutable after construction. IS-0019 — Restoration Model.md

⸻

9. Resume Point Derivation

The derivation process SHALL produce exactly one Resume Point.

The Resume Point SHALL:

* belong to the input Workspace;
* reference canonical Artifacts only;
* represent the first cognitive step required to continue work;
* represent understanding rather than interface state;
* remain independent of operating-system execution mechanisms.

The Resume Point SHALL NOT be:

* an application;
* a window;
* a browser tab;
* a monitor layout;
* an OS action.

These requirements are inherited directly from IS-0019. IS-0019 — Restoration Model.md

Frozen Algorithm

The Resume Point selection algorithm is frozen by Section 25.2.

No implementation SHALL invent a ranking rule, recency rule, frequency rule, or semantic heuristic and claim it is canonical merely because it produces one Resume Point.

Section 25.2 defines the eligible candidate set, the selection evidence, deterministic tie-breaking, behavior when multiple candidates are equally eligible, and behavior when no valid candidate exists.

Conformance SHALL be evaluated against Section 25.2 per Section 24.

⸻

10. Context Chain Derivation

The derivation process SHALL produce one ordered Context Chain.

The Context Chain SHALL contain only canonical Artifacts belonging to the same Workspace.

It SHALL:

* contain no unrelated Artifacts;
* contain no duplicate Artifacts;
* remain deterministically ordered;
* contain only the minimum supporting context required to understand the Resume Point.

The Context Chain SHALL optimize for cognitive reload rather than historical completeness. IS-0019 — Restoration Model.md

Frozen Algorithm

The Context Chain ordering and inclusion algorithm is frozen by Section 25.3.

An implementation SHALL NOT substitute arbitrary chronological ordering, frequency ranking, UI ordering, or model-generated ordering as the canonical rule.

Conformance SHALL be evaluated against Section 25.3 per Section 24.

⸻

11. Blocker Derivation

The derivation process MAY produce zero or more Blockers.

A Blocker SHALL represent unresolved work preventing immediate continuation.

Blockers SHALL be descriptive.

Blockers SHALL NOT prescribe solutions.

Blockers SHALL originate from canonical Workspace understanding.

Restoration SHALL NOT invent Blockers. ARCHITECTURE.md

Therefore Restoration Derivation SHALL NOT create a Blocker solely because:

* an application is currently closed;
* a file is currently unavailable;
* a browser is not running;
* a window is not visible;
* the network is unavailable;
* the operating system is in a different state;
* an execution action has not yet occurred.

Those are execution/runtime conditions and belong outside Restoration Derivation.

Frozen Algorithm

The Blocker identification and classification rule is frozen by Section 25.4.

Under the current canonical Workspace/Snapshot model, the derivation produces zero Blockers. Restoration Derivation SHALL NOT invent Blocker semantics.

Conformance SHALL be evaluated against Section 25.4 per Section 24.

⸻

12. Next Step Derivation

The derivation process SHALL produce exactly one Next Step.

The Next Step SHALL:

* belong to the same Workspace;
* immediately follow the Resume Point;
* represent the immediate continuation of work;
* remain independent of implementation-specific execution mechanisms.

The Next Step SHALL NOT itself be an OS automation command.

These requirements follow IS-0019. ARCHITECTURE.md

Frozen Algorithm

The Next Step derivation rule is frozen by Section 25.5.

An implementation SHALL NOT fabricate a Next Step from arbitrary language-model output, UI state, recent application activity, or unrelated runtime information.

Conformance SHALL be evaluated against Section 25.5 per Section 24.

⸻

13. Determinism

Restoration Derivation SHALL be deterministic.

Given:

* identical canonical Workspace input;
* identical Snapshot;
* identical derivation rules;
* identical canonical lower-layer objects;

the derivation SHALL produce an identical RestorationPlan.

No derivation decision SHALL depend on:

* wall-clock time;
* random values;
* process identity;
* machine identity;
* current UI state;
* current OS state;
* network state;
* language-model sampling;
* nondeterministic iteration order.

This requirement follows the deterministic construction requirement of IS-0019. IS-0019 — Restoration Model.md

⸻

14. Replayability

Restoration Derivation SHALL remain replayable.

A RestorationPlan SHALL be reproducible from its canonical input and frozen derivation rules.

Replaying Restoration Derivation SHALL NOT modify:

* Observation history;
* Artifact Identity;
* Workspace Identity;
* Attachment history;
* historical Snapshots;
* previously constructed RestorationPlans.

A new derivation MAY produce a new RestorationPlan when the canonical input or derivation rules differ.

Historical outputs SHALL NOT be silently rewritten.

This is consistent with the Architecture’s replay philosophy and the immutability of Snapshots. ARCHITECTURE.md

⸻

15. Cognitive Reload Objective

The primary optimization objective of Restoration Derivation SHALL be:

minimize cognitive reload.

This does not mean maximizing the quantity of restored information.

It does not mean maximizing the number of Artifacts.

It does not mean reconstructing the user’s entire desktop.

It means deriving the smallest coherent continuation strategy that allows the user to understand where work stopped and what naturally follows.

IS-0019 explicitly establishes minimizing cognitive reload as the primary Restoration objective. ARCHITECTURE.md

⸻

16. Insufficient Workspace Understanding

Restoration Derivation SHALL NOT fabricate understanding when canonical Workspace evidence is insufficient.

If the available canonical Workspace understanding cannot satisfy a required RestorationPlan invariant, derivation SHALL fail explicitly rather than invent:

* a Resume Point;
* a Context Chain;
* a Blocker;
* a Next Step.

Failure SHALL identify the unmet invariant or missing canonical input.

The derivation layer SHALL NOT silently substitute:

* raw Observations;
* runtime state;
* heuristics not defined by the derivation contract;
* model-generated guesses;
* UI state;
* platform-specific assumptions.

⸻

17. Lower-Layer Integrity

Restoration Derivation SHALL treat lower-layer canonical objects as authoritative.

It SHALL NOT:

* reinterpret Artifact Identity;
* reinterpret Workspace membership;
* modify Attachment confidence;
* modify Snapshot contents;
* reconstruct Workspace state independently.

Workspace Formation already defines Workspace understanding through Workspace, Attachment set, and Snapshot history. IS-0012 — Workspace Formation.md

Restoration Derivation consumes that understanding.

It does not become a second Workspace Engine.

⸻

18. Snapshot Boundary

Snapshots are immutable historical representations of Workspace understanding.

Snapshot construction belongs to Workspace Formation.

Restoration Derivation SHALL consume the selected Snapshot and SHALL NOT construct a replacement Snapshot.

Historical Snapshots SHALL remain unchanged.

A new RestorationPlan SHALL NOT cause an existing Snapshot to change.

This preserves the distinction between:

Workspace understanding

and:

restoration understanding derived from Workspace understanding

⸻

19. Restoration Plan Immutability

Once constructed, a RestorationPlan SHALL NOT be mutated.

Any change in canonical Workspace understanding or derivation rules SHALL result in a newly derived RestorationPlan.

The previous RestorationPlan SHALL remain unchanged wherever it has become historical or otherwise committed.

This preserves the immutability requirements defined by IS-0019 and the historical-accountability model. IS-0019 — Restoration Model.md

⸻

20. Execution Boundary

Restoration Derivation SHALL end when the canonical RestorationPlan has been constructed and validated.

It SHALL NOT execute the plan.

Execution belongs to a separate architectural layer.

Execution MAY later:

* launch applications;
* restore browser tabs;
* restore windows;
* focus documents;
* interact with operating systems.

Those actions SHALL consume the RestorationPlan rather than being performed during derivation. ARCHITECTURE.md

The derivation layer SHALL therefore expose a plan, not an automation procedure.

⸻

21. Public Computational Surface

The eventual implementation SHALL expose one conceptual computation:

derive_restoration_plan(
    canonical_workspace_understanding
) -> RestorationPlan

The concrete Rust API is implementation-defined and SHALL NOT introduce additional canonical domain concepts unless a later specification requires them.

The public surface SHALL not expose internal ranking or heuristic machinery as independent architectural primitives.

⸻

22. Error Boundary

Restoration Derivation MAY fail when canonical inputs cannot satisfy the RestorationPlan contract.

A future implementation SHALL use errors only for actual contract failures.

It SHALL NOT introduce errors for ordinary absence of optional information unless the absence prevents construction of a valid RestorationPlan.

The concrete error taxonomy remains an implementation concern until a future implementation contract requires it.

⸻

23. Prohibited Derivation Strategies

The following SHALL NOT constitute canonical Restoration Derivation unless explicitly authorized by a future frozen specification:

* selecting the most recently modified Artifact merely because it is recent;
* selecting the most frequently observed Artifact merely because it is frequent;
* selecting the last focused application as the Resume Point;
* using raw OS state to construct a Resume Point;
* using browser state to construct a Context Chain;
* inferring Blockers from current machine state;
* generating a Next Step from an LLM;
* using arbitrary semantic similarity as the canonical selection rule;
* restoring every Artifact in a Workspace;
* treating all Workspace Artifacts as equally relevant;
* reconstructing the desktop as the RestorationPlan;
* performing Attachment inference during Restoration;
* performing Workspace Formation during Restoration.

These strategies either violate existing boundaries or introduce semantics not currently frozen.

⸻

24. Conformance

An implementation conforms to IS-0021 only if it implements the canonical derivation algorithms in Section 25 and produces a valid RestorationPlan whenever the canonical Workspace/Snapshot evidence is sufficient to satisfy IS-0019.

Conformance to the concrete Restoration Derivation algorithms SHALL be evaluated against the canonical algorithms frozen in Section 25.

1. It consumes canonical Workspace understanding.
2. It produces exactly one valid RestorationPlan when sufficient canonical input exists.
3. It produces exactly one Resume Point.
4. It produces an ordered Context Chain.
5. It produces zero or more canonical Blockers.
6. It produces exactly one Next Step.
7. All components belong to the same Workspace where required.
8. The result satisfies every IS-0019 invariant.
9. Derivation is deterministic.
10. Derivation is replayable.
11. Workspace understanding remains unchanged.
12. Historical Snapshots remain unchanged.
13. No OS execution occurs during derivation.
14. No raw runtime state participates in derivation.
15. No unspecified inference algorithm is presented as canonical behavior.

⸻

25. Canonical Restoration Derivation Algorithms

Restoration Derivation SHALL consume only:

* the selected canonical Snapshot;
* the canonical Workspace referenced by that Snapshot;
* the canonical Artifacts referenced by the Snapshot's Attachments;
* the canonical Attachment records contained in the Snapshot;
* the canonical designated Artifact established by the current WorkDesignated evidence (RFC-0011), when it exists, presented as canonical Artifact-level understanding;
* the Current Continuation Surface established by the latest valid ContinuationSurface declaration (RFC-0013), when it exists, presented as canonical Artifact-level understanding.

The designated Artifact is resolved by the Artifact layer's deterministic identity derivation over the canonical Observation log (RFC-0011 §4) before derivation begins; the Current Continuation Surface is resolved by the Artifact layer to the canonical Artifacts its declared subjects establish (RFC-0013 §Restoration Derivation Interaction) before derivation begins. Derivation SHALL NOT consult raw Observations or perform Artifact Resolution itself.

### 25.1A Continuation Surface Consumption (RFC-0013)

For the Workspace under derivation, the derived surface is:

> surface(W) = { artifact ∈ artifacts(W) : artifact is referenced by the Current Continuation Surface }

in canonical deterministic order (ascending ArtifactId).

Surface members outside W are ignored for W; each Workspace sees only its own declared members. This is derivation output, not canonical state.

The surface SHALL NOT gate completeness (a plan is Complete exactly when the Resume Point and Next Step are established, per §25.6). It SHALL NOT change the Resume Point, the Next Step, the Context Chain, or Blockers (§25.2–§25.5 are otherwise unchanged). It SHALL NOT change execution ordering or selection; it never authorizes opening, focusing, or launching any resource.

The algorithms in this section define the complete canonical derivation behavior that is possible from the current Workspace/Snapshot model.

They SHALL be:

* deterministic;
* replayable;
* explainable from canonical inputs;
* independent of current runtime state;
* independent of language-model generation;
* independent of interface state;
* independent of platform-specific execution;
* faithful to the information actually represented by the canonical input.

Restoration Derivation SHALL NOT assume that the Snapshot contains semantic roles or relationships that are not part of the canonical Snapshot model.

In particular, the current Snapshot model does NOT contain canonical fields for:

* primary Artifact designation;
* continuation designation;
* supporting Artifact designation;
* reference Artifact designation;
* unresolved-work representation;
* continuation relationships;
* Next Step instructions.

Restoration Derivation SHALL therefore never manufacture those distinctions from Attachment order, Attachment confidence, ArtifactId, Artifact recency, current runtime state, or any other non-canonical signal.

25.1 Common Evidence Rules

All derivation stages SHALL operate only on canonical Workspace understanding.

The canonical candidate set SHALL consist only of the canonical Artifacts represented by the selected Snapshot's Attachment Set, deduplicated by ArtifactId.

Restoration Derivation SHALL NOT:

* reconstruct Artifact Identity;
* reconstruct Workspace membership;
* modify Attachment confidence;
* modify Snapshot contents;
* reinterpret Artifact identity;
* consult raw Observations;
* consult current OS state;
* consult current application or window state;
* consult browser state;
* consult current filesystem state;
* consult Git state;
* use an LLM;
* use arbitrary semantic similarity;
* introduce hidden ranking criteria.

ArtifactId SHALL be used only as a stable identity reference and deterministic tie-breaker where a tie-breaker is explicitly required.

Attachment Confidence SHALL remain evidence of Workspace attachment only.

Attachment Confidence SHALL NOT be reinterpreted as:

* importance;
* recency;
* primary status;
* continuation status;
* resume priority;
* blocker severity.

Snapshot Attachment ordering SHALL preserve canonical ordering only.

Attachment ordering SHALL NOT, by itself, be interpreted as semantic priority unless a future frozen specification explicitly gives that ordering such meaning.

If a required RestorationPlan component cannot be established from canonical evidence, Restoration Derivation SHALL fail explicitly rather than invent the missing component.

The governing principle is:

Evo SHALL restore as much as the canonical evidence permits, and SHALL never pretend to know more than it does.

25.2 Resume Point Derivation

The Resume Point represents the canonical entry point into the Workspace from which restoration may begin.

Candidate Set

The candidate set SHALL contain all canonical Artifacts represented by the selected Snapshot.

Selection Evidence

A Resume Point MAY be selected only when the canonical input provides sufficient evidence to distinguish one Artifact as the appropriate resume point.

The current Workspace/Snapshot model provides no explicit primary or continuation designation.

Therefore the following SHALL NOT constitute Resume Point evidence:

* Attachment order alone;
* Attachment Confidence alone;
* ArtifactId ordering alone;
* Artifact recency;
* Artifact frequency;
* current application focus;
* current window focus;
* current OS state;
* raw Observation timestamps;
* current filesystem state;
* current browser state;
* Git state;
* semantic similarity;
* model-generated reasoning.

Selection

If the canonical Workspace/Snapshot input contains exactly one eligible Artifact, that Artifact SHALL be selected as the Resume Point.

If the canonical input contains more than one eligible Artifact and contains no canonical evidence distinguishing one Artifact as the resume point, Resume Point derivation SHALL fail with insufficient canonical evidence.

If the canonical input identifies a designated canonical Artifact (RFC-0011) that is among the eligible candidates, that Artifact SHALL be the Resume Point, resolving the multi-candidate insufficiency.

If a future canonical lower-layer representation explicitly identifies one Artifact as the resume point, that Artifact SHALL be selected according to that representation.

Tie-breaking

No deterministic tie-breaker SHALL be used to manufacture semantic meaning.

ArtifactId ordering MAY be used only to establish deterministic ordering of already-equivalent non-semantic collections. It SHALL NOT be used to decide which Artifact is the Resume Point.

Result

Exactly one Resume Point SHALL be produced when sufficient canonical evidence exists.

Otherwise Restoration Derivation SHALL fail.

25.3 Context Chain Derivation

The Context Chain provides supporting canonical context for the selected Resume Point.

Candidate Set

The candidate set SHALL contain canonical Artifacts belonging to the same Workspace and represented by the selected Snapshot, excluding the Resume Point.

Relevance

An Artifact SHALL be included in the Context Chain only when the canonical input explicitly establishes that Artifact as relevant supporting context for the Resume Point.

The current Snapshot model contains no canonical supporting/reference/continuation relationship.

Therefore Workspace membership alone SHALL NOT establish Context Chain relevance.

Attachment Confidence SHALL NOT establish Context Chain relevance.

Attachment ordering SHALL NOT establish Context Chain relevance.

ArtifactId ordering SHALL NOT establish Context Chain relevance.

If no canonical supporting relationship exists, the Context Chain SHALL be empty.

Ordering

Where multiple context Artifacts are already established as relevant by canonical input, their canonical Snapshot ordering SHALL be preserved.

If canonical ordering does not distinguish two otherwise equivalent context Artifacts, ArtifactId MAY be used solely as a deterministic ordering tie-breaker.

Stopping Rule

The Context Chain SHALL contain only canonical context established by the input.

It SHALL NOT expand merely because additional Workspace Artifacts exist.

It SHALL NOT attempt to reconstruct the complete Workspace.

Result

The Context Chain SHALL contain zero or more canonical Artifacts.

No Artifact SHALL appear more than once.

25.4 Blocker Derivation

Blockers represent unresolved work already established by canonical Workspace understanding.

Restoration Derivation SHALL NOT invent Blockers.

Canonical Source

The current Workspace/Snapshot model contains no canonical unresolved-work representation.

Therefore the current canonical derivation SHALL produce zero Blockers.

A Blocker SHALL be produced only if a future canonical lower-layer Workspace representation explicitly records an unresolved condition and that representation is included in the Restoration input.

Prohibited Blocker Sources

The following SHALL NOT create a Blocker:

* a closed application;
* an unavailable file;
* a missing browser;
* a missing window;
* current OS state;
* current network state;
* current filesystem state;
* Git status;
* current runtime state;
* absence of an observed action.

Result

Under the current canonical Workspace/Snapshot model:

Blockers = zero.

No inferred blocker SHALL be substituted.

25.5 Next Step Derivation

The Next Step represents the immediate continuation of work after the Resume Point.

Candidate Source

The current Workspace/Snapshot model contains no canonical continuation relationship and no canonical Next Step representation.

Therefore a Next Step SHALL be produced only when the canonical input explicitly establishes continuation information.

The following SHALL NOT constitute continuation evidence:

* Snapshot ordering alone;
* Attachment Confidence;
* ArtifactId ordering;
* Artifact recency;
* current application activity;
* current window focus;
* raw Observations;
* current OS state;
* current browser state;
* filesystem state;
* Git state;
* semantic similarity;
* model-generated output.

Designation Evidence

The canonical input MAY include a designated canonical Artifact established by the current WorkDesignated evidence (RFC-0011). When that Artifact is represented by the selected Snapshot, it is the continuation target, and its Semantic Meaning is: "the user declared that the work continues at this Artifact." The current designation is the most recent WorkDesignated observation by canonical Observation Time; a later designation supersedes all earlier ones. This supersession rule is the evidence contract's own defined semantic (RFC-0011 §5), not a chronological inference.

Insufficient Evidence

If the canonical Workspace/Snapshot understanding does not establish an immediate continuation, Restoration Derivation SHALL fail.

It SHALL NOT manufacture a generic Next Step such as:

* "continue working";
* "open the project";
* "review the files";
* "check the latest changes";
* "return to the application".

Result

Exactly one Next Step SHALL be produced only when canonical continuation evidence exists.

Otherwise Restoration Derivation SHALL fail explicitly.

25.6 RestorationPlan Construction

RestorationPlan Construction SHALL occur only after:

* a valid Resume Point has been derived;
* the Context Chain has been derived;
* Blockers have been derived;
* a valid Next Step has been derived.

When a designated canonical Artifact (RFC-0011) establishes both the Resume Point (§25.2) and the Next Step (§25.5), every required component is derivable and construction proceeds.

The resulting RestorationPlan SHALL contain exactly:

* one Resume Point;
* one Context Chain;
* zero or more Blockers;
* one Next Step.

The RestorationPlan SHALL reference only canonical objects established by the input.

RestorationPlan Construction SHALL NOT:

* create new Artifacts;
* create new Workspace membership;
* create new unresolved conditions;
* create new continuation relationships;
* modify the Workspace;
* modify the Snapshot;
* modify historical Snapshots;
* execute any restoration action.

If any required component cannot be established from canonical evidence, no RestorationPlan SHALL be constructed.

25.7 Cross-Component Consistency

The derived components SHALL describe one coherent interpretation of the same Workspace.

The Resume Point SHALL belong to the input Workspace.

The designated canonical Artifact (RFC-0011) SHALL belong to the input Workspace and be represented by the selected Snapshot; otherwise no component is derived from it for this Workspace.

Every Context Chain Artifact SHALL belong to the same Workspace.

Every Blocker SHALL originate from the same Workspace understanding.

The Next Step SHALL belong to the same Workspace and SHALL be canonically established as following the Resume Point.

No component SHALL introduce:

* a new Artifact;
* new Workspace membership;
* an invented unresolved condition;
* an invented continuation relationship;
* an inferred semantic role not present in canonical input.

Restoration Derivation SHALL remain a consumer of Workspace understanding.

It SHALL NOT become a second Workspace Formation engine.

25.8 Insufficient-Evidence Behavior

Insufficient canonical evidence SHALL be treated as a normal and explicit Restoration Derivation outcome.

It SHALL NOT be converted into a guessed RestorationPlan.

The derivation result SHALL identify which required component could not be established from canonical input.

The absence of a canonical Resume Point or Next Step SHALL NOT be hidden by selecting an arbitrary Artifact.

The absence of canonical Blockers SHALL mean zero Blockers, not inferred blockers.

The absence of canonical Context relationships SHALL mean an empty Context Chain, provided the Resume Point and Next Step can otherwise be established.

The absence of a current WorkDesignated designation SHALL mean Next Step insufficiency, never an inferred designation.

This behavior preserves the distinction between:

"What Evo knows"

and:

"What Evo could guess."

Evo SHALL prefer an explicit derivation failure over fabricated continuity.

25.9 Determinism

Given identical:

* canonical Workspace;
* canonical Snapshot;
* canonical Artifacts;
* canonical Attachments;
* frozen derivation rules;

Restoration Derivation SHALL produce an identical result.

No derivation decision SHALL depend upon:

* wall-clock time;
* random values;
* machine identity;
* process identity;
* current OS state;
* current UI state;
* current application state;
* current browser state;
* network state;
* filesystem state;
* Git state;
* model sampling;
* nondeterministic iteration order.

25.10 Replayability

Restoration Derivation SHALL be replayable.

Replaying the derivation against identical canonical Workspace/Snapshot understanding SHALL produce the same RestorationPlan or the same explicit insufficient-evidence result.

Replaying Restoration Derivation SHALL NOT modify:

* Observations;
* Artifacts;
* Workspaces;
* Attachments;
* Snapshots;
* historical understanding.

25.11 Canonical Evidence Expansion

If future Evo capabilities introduce additional canonical evidence required for stronger Restoration Derivation, that evidence SHALL be introduced through the appropriate lower-layer architectural contract.

RFC-0011 (Work Designation) is the first such contract: it defines the OBS-WORK-DESIGNATED/v1 Observation Schema (IS-0003 §4.1.5) and the resolution rule by which the current designation references an existing canonical Artifact. IS-0021 is amended here to consume that canonical evidence exactly as RFC-0011 §5 specifies.

Restoration Derivation SHALL NOT silently create such evidence itself.

This preserves the architectural direction:

Observation
    ↓
Artifact
    ↓
Workspace
    ↓
Snapshot
    ↓
Restoration Derivation
    ↓
RestorationPlan
    ↓
Execution

Each layer SHALL contribute only the information it canonically owns.

⸻

26. Architectural Rationale

Restoration exists to restore continuity of thought rather than reproduce a desktop.

The Workspace is the canonical computational representation of a coherent body of work.

The Snapshot preserves a historical understanding of that Workspace.

Restoration Derivation transforms that understanding into a continuation strategy.

The separation is therefore:

Observation
    ↓
Artifact
    ↓
Workspace
    ↓
Snapshot
    ↓
Restoration Derivation
    ↓
RestorationPlan
    ↓
Execution

Each layer answers a different question:

Observation
"What happened?"
Artifact
"What external thing was involved?"
Workspace
"What body of work does this belong to?"
Snapshot
"What did Evo understand about that Workspace at this point?"
RestorationPlan
"How should that understanding be used to continue?"
Execution
"How can that plan be carried out on this platform?"

This preserves the architectural separation between evidence, identity, work, restoration understanding, and execution. The Workspace model explicitly keeps Workspace independent of Restoration, while IS-0019 makes Restoration a consumer of Workspace understanding.  

⸻

27. Committed Understanding Boundary

Restoration Derivation SHALL NOT conflate:

* Workspace;
* Snapshot;
* RestorationPlan;
* CommittedUnderstanding;
* HistoricalUnderstanding.

Restoration Derivation produces the four canonical restoration components required by IS-0019.

Those components MAY subsequently constitute a CommittedUnderstanding according to IS-0018.

CommittedUnderstanding SHALL NOT become an alternative input to Restoration Derivation.

HistoricalUnderstanding SHALL preserve the resulting CommittedUnderstanding according to IS-0016.

Restoration Derivation SHALL NOT perform Historical Commitment.

This preserves the separation:

Workspace
   ↓
Snapshot
   ↓
Restoration Derivation
   ↓
ResumePoint
ContextChain
Blockers
NextStep
   ↓
CommittedUnderstanding
   ↓
HistoricalUnderstanding

IS-0018 defines CommittedUnderstanding as the immutable object containing exactly those four components. IS-0018 — Committed Understanding.md

⸻

28. Conformance Status

Status: Frozen

Restoration Derivation is a frozen behavioral contract.

An implementation conforms only if it:

1. implements the algorithms in Section 25;
2. produces exactly one Resume Point;
3. produces one deterministic Context Chain;
4. produces zero or more Blockers;
5. produces exactly one Next Step;
6. produces a valid RestorationPlan satisfying IS-0019;
7. consumes only canonical Workspace/Snapshot understanding;
8. never consults raw runtime state;
9. never modifies lower-layer canonical objects;
10. never performs Workspace Formation;
11. never performs Artifact Resolution;
12. never performs execution;
13. remains deterministic;
14. remains replayable;
15. fails explicitly when required canonical evidence is insufficient;
16. never substitutes unspecified inference for canonical derivation.

No implementation may introduce additional canonical Restoration concepts without a subsequent frozen architectural specification.

⸻

End of Specification