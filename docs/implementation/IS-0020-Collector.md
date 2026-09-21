IS-0020 — Collector

Status: Frozen
Depends On:

* Constitution
* Product
* Architecture
* Architectural Laws
* Observation Model
* IS-0001 — Observation
* IS-0002 — Observation Language
* IS-0003 — Observation Schemas

⸻


1. Purpose

The Collector is Evo’s boundary between observable platform reality and the canonical Observation system.

The Collector observes platform-level signals, translates those signals into Evo’s canonical Observation Language, constructs Candidate Observations, and submits those Candidates to the Observation Acceptance pipeline defined by IS-0001.

The Collector exists to witness reality.

It does not determine what that reality means.

⸻

2. Architectural Position

The Collector operates before Observation Acceptance.

Its position in the architectural pipeline is:

Platform Reality
      │
      ▼
Platform Signal
      │
      ▼
Collector
      │
      ├── translation into Observation Language
      ├── Provenance construction
      └── Candidate Observation construction
      │
      ▼
IS-0001 — Observation Acceptance
      │
      ├── Validation
      ├── Canonicalization
      ├── Identity Assignment
      ├── Integrity Verification
      └── Persistence
      │
      ▼
Accepted Observation

The Collector SHALL NOT replace, bypass, or duplicate the Observation Acceptance pipeline.

IS-0001 remains the sole architectural entry point through which observational evidence becomes an accepted Observation. IS-0001-Observation.md

⸻

3. Scope

The Collector SHALL:

1. observe supported platform signals;
2. preserve directly observed information;
3. translate platform-specific signals into canonical Observation Language concepts;
4. construct the required Provenance;
5. associate the appropriate Observation Schema;
6. construct Candidate Observations;
7. submit Candidate Observations to IS-0001.

The Collector SHALL NOT:

* interpret evidence;
* infer user intent;
* classify work;
* determine importance;
* identify artifacts;
* form Workspaces;
* create Knowledge;
* perform Retrieval;
* perform Restoration;
* modify accepted Observations;
* replace Observation Acceptance.

This separation follows the explicit responsibilities and non-responsibilities of IS-0001 and the platform-independence requirements of IS-0002.  

⸻

4. Definitions

4.1 Platform Signal

A Platform Signal is information directly exposed by the operating system, platform, or an authorized capture mechanism indicating that an observable event occurred.

A Platform Signal is platform-specific.

A Platform Signal SHALL NOT enter the canonical Observation system unchanged.

⸻

4.2 Collector

A Collector is the component responsible for translating supported Platform Signals into Candidate Observations.

A Collector MAY be platform-specific.

The Candidate Observation produced by a Collector SHALL NOT be platform-specific.

⸻

4.3 Candidate Observation

A Candidate Observation is the transient representation submitted to IS-0001 for acceptance.

Every Candidate Observation SHALL contain:

* Evidence;
* Provenance;
* Observation Schema Identifier.

This requirement is inherited directly from IS-0001. IS-0001-Observation.md

⸻

4.4 Platform Adapter

A Platform Adapter is an implementation boundary responsible for obtaining or normalizing platform-specific signals for the Collector.

Platform Adapters MAY differ between operating systems.

Platform Adapters SHALL NOT alter the canonical Observation Language.

⸻

5. Collector Input

The Collector accepts Platform Signals from supported capture mechanisms.

The specification SHALL NOT require a particular operating-system API, framework, entitlement, permission mechanism, or implementation technology.

Those are implementation concerns.

A Platform Signal MAY contain platform-specific information necessary to faithfully represent the observed event.

The Collector SHALL preserve directly observed information required by the applicable Observation Schema and Provenance contract.

The Collector SHALL NOT invent missing information.

⸻

6. Collector Output

For every Platform Signal that can be faithfully represented by a known Observation Schema, the Collector SHALL produce a Candidate Observation containing:

Candidate Observation
├── Evidence
├── Provenance
└── Observation Schema Identifier

The Candidate Observation SHALL use only concepts defined by IS-0002.

The applicable Schema SHALL be defined by IS-0003 or a schema specification derived from it.

The Collector SHALL NOT introduce a new canonical concept while processing a Platform Signal.

IS-0002 explicitly requires platform-specific observations to be translated into canonical concepts and prohibits Collectors from extending or redefining the Observation Language. IS-0002 — Observation Language.md

⸻

7. Translation

The Collector SHALL translate Platform Signals into the canonical Observation Language before submitting them to IS-0001.

Translation SHALL preserve the directly observed meaning of the source signal.

Translation SHALL NOT:

* infer intent;
* classify activity;
* infer task membership;
* infer Workspace membership;
* infer importance;
* infer artifact identity;
* enrich evidence with unsupported information;
* remove directly observed evidence merely because it appears unimportant.

The Collector SHALL prefer incomplete representation over fabricated representation when a Platform Signal does not provide information required for a complete semantic representation.

This follows Laws V and VI: uncertainty must be preserved and under-interpretation is preferable to over-interpretation. ARCHITECTURAL_LAWS.md

⸻

8. Provenance

Every Candidate Observation SHALL contain Provenance as defined by the Provenance contract in IS-0001.

The Collector SHALL construct Provenance from directly available capture circumstances.

The Collector SHALL NOT fabricate provenance.

The Collector SHALL NOT replace missing provenance with estimated or inferred values.

Once constructed, Provenance SHALL be passed unchanged to IS-0001 except for permitted canonical representation.

IS-0001 is responsible for permanent preservation of Provenance. IS-0001-Observation.md

⸻

9. Observation Schema Selection

The Collector SHALL associate each Candidate Observation with exactly one applicable Observation Schema.

The Collector SHALL NOT invent schemas dynamically.

The Collector SHALL NOT modify the semantic definition of an existing Schema.

If a Platform Signal cannot be represented by a known applicable Schema, the Collector SHALL NOT fabricate a Candidate Observation.

The signal MAY be reported as a capture failure according to implementation requirements, but no unsupported Observation SHALL enter IS-0001.

⸻

10. Evidence Preservation

The Collector SHALL preserve observable evidence faithfully.

The Collector SHALL NOT:

* rewrite evidence to make it more meaningful;
* summarize evidence;
* classify evidence;
* merge separate observations into a semantic conclusion;
* manufacture missing evidence;
* replace evidence with an interpretation.

The Collector may perform representation translation necessary to express directly observed information using the Observation Language.

Such translation SHALL NOT change the witnessed content.

This follows Law II — Evidence Is Sacred — and IS-0001’s requirement that canonicalization preserve Evidence.  

⸻

11. No Interpretation

The Collector SHALL NOT perform semantic interpretation.

In particular, the Collector SHALL NOT determine:

* what the user intended;
* what task the user was performing;
* which Workspace the activity belongs to;
* whether an Artifact is important;
* whether an activity represents progress;
* whether an activity is relevant;
* whether two observations represent the same higher-level entity.

These responsibilities belong to downstream architectural layers.

The Collector records reality; it does not explain reality.

⸻

12. No Artifact Resolution

The Collector SHALL NOT assign Artifact Identity.

It SHALL NOT determine whether two observed entities are the same Artifact.

It SHALL NOT construct canonical Artifacts.

Observed information may contain the canonical information necessary for downstream Artifact processing, but Artifact identity remains outside the Collector.

This preserves the boundary between Observation and Artifact interpretation.

⸻

13. No Workspace Formation

The Collector SHALL NOT:

* create Workspaces;
* modify Workspaces;
* assign observations to Workspaces;
* determine Workspace boundaries;
* infer sessions;
* infer tasks.

The Collector produces observations only.

⸻

14. No Importance or Noise Decisions

The Collector SHALL NOT decide whether an observed event is important.

The Collector SHALL NOT classify an event as useful or useless based on its semantic interpretation.

The current architecture does not assign semantic noise filtering to the Collector.

Therefore the Collector SHALL NOT introduce semantic filtering rules unless a higher-level specification explicitly assigns that responsibility to it.

This is important because the audit found that noise filtering and deduplication were not specified by the frozen documents. evo-runtime-fact-audit.md

⸻

15. Duplicate Signals

The Collector SHALL NOT merge distinct observations merely because they appear similar.

The Collector SHALL NOT treat semantic equivalence as permission to discard evidence.

Any future duplicate-suppression behavior SHALL be specified explicitly and SHALL NOT alter the meaning of observable evidence.

Until such behavior is separately specified, the Collector SHALL preserve observed signals rather than applying semantic deduplication.

This preserves Law II and the explicit IS-0001 rule that Observation Acceptance itself does not deduplicate observations. IS-0001-Observation.md

⸻

16. Rate Limiting and Sampling

The Collector SHALL NOT sample, aggregate, or rate-limit Platform Signals in a way that changes their observational meaning unless such behavior is explicitly defined by a future specification.

Performance optimizations SHALL NOT silently change the set or meaning of observations entering Evo.

A Collector implementation MAY use implementation-level scheduling or buffering mechanisms where necessary for reliable operation, provided that those mechanisms do not alter the semantics of Candidate Observations.

⸻

17. High-Fidelity Capture

The Collector SHALL NOT use raw high-fidelity visual capture as the ground truth of Evo’s observational record.

Screenshots and screen recordings SHALL NOT become canonical evidence.

If a platform signal requires a derived lightweight representation, the representation SHALL itself satisfy the Observation Language and applicable Observation Schema.

The original high-fidelity visual source SHALL NOT become persistent observational ground truth.

This preserves the privacy boundary established by the Architecture.

⸻

18. Privacy

The Collector SHALL operate under Evo’s local-first privacy requirements.

Collected information SHALL remain under the user’s control.

The Collector SHALL collect only information necessary to represent supported observational facts.

The Collector SHALL NOT transmit observational evidence to an external service as part of the normal capture pipeline.

The Collector SHALL NOT introduce a cloud dependency into Observation capture.

Privacy is an architectural constraint, not an optional feature, consistent with the Constitution’s Article VII. CONSTITUTION.md

⸻

19. Failure Handling

A Platform Signal SHALL NOT become a Candidate Observation if the Collector cannot represent it faithfully using:

* a known Observation Language representation;
* a known Observation Schema;
* valid Provenance.

The Collector SHALL NOT repair the signal by inventing missing information.

The Collector SHALL NOT manufacture a plausible Observation merely to maintain continuity.

Failure to represent a signal SHALL result in no Candidate Observation being submitted for that signal.

The failure MAY be surfaced to the runtime according to implementation requirements, but such reporting SHALL NOT itself become observational evidence unless separately specified.

⸻

20. Submission to Observation Acceptance

The Collector SHALL submit each constructed Candidate Observation to IS-0001.

The Collector SHALL NOT mark a Candidate Observation as an Accepted Observation.

Acceptance occurs only through IS-0001.

The Collector SHALL treat acceptance and rejection as the responsibility of the Observation module.

IS-0001 defines the acceptance state machine and guarantees atomic acceptance or rejection. IS-0001-Observation.md

⸻

21. Persistence

The Collector SHALL NOT own canonical Observation persistence.

Persistence of accepted Observations belongs to IS-0001.

The Collector MAY maintain transient operational state necessary to perform capture, but such state SHALL NOT become canonical observational history unless explicitly accepted through IS-0001.

This preserves the distinction between capture and durable Observation.

⸻

22. Platform Independence

The Collector architecture SHALL support platform-specific implementations without allowing platform-specific concepts to enter the canonical Observation Language.

For example:

Platform A ─┐
            ├── Collector Translation ──> Canonical Observation
Platform B ─┤
            │
Platform C ─┘

Equivalent observable events from different platforms SHALL map to the same canonical semantic representation when they represent the same Observation Language concept.

This is required by IS-0002’s platform-independence and canonical-representation principles. IS-0002 — Observation Language.md

⸻

23. Platform Adapters

Platform-specific capture mechanisms SHALL remain isolated from the canonical Observation system.

A Platform Adapter SHALL:

* receive platform-specific signals;
* expose directly observed information;
* avoid semantic interpretation;
* provide information necessary for Collector translation.

A Platform Adapter SHALL NOT:

* create Workspaces;
* identify Artifacts;
* infer Tasks;
* construct Knowledge;
* rank events;
* make user-level decisions.

Platform Adapters SHALL NOT modify IS-0002.

⸻

24. Supported Observation Scope

The Collector specification does not define the complete vocabulary of runtime facts.

The set of supported observational facts SHALL be determined by the Observation Language and corresponding Observation Schemas.

Adding a new observational fact SHALL require:

1. a canonical Observation Language concept where necessary;
2. an applicable Observation Schema;
3. a supported platform capture mechanism;
4. conformance with this Collector specification.

The Collector implementation SHALL NOT invent additional fact types merely because the underlying operating system exposes additional signals.

This keeps the product smaller than the vision, as required by Constitution Article IX. CONSTITUTION.md

⸻

25. Determinism

Given the same:

* Platform Signal;
* Collector configuration;
* Observation Language version;
* Observation Schema version;

the Collector SHALL produce behaviorally equivalent Candidate Observation content.

Platform-specific implementation details SHALL NOT alter the canonical meaning of equivalent observations.

⸻

26. Immutability Boundary

The Collector may construct transient Candidate Observations.

Once submitted to IS-0001, the Collector SHALL NOT modify the Candidate Observation as part of acceptance.

Once accepted, the resulting Observation SHALL be immutable.

The Collector SHALL never modify accepted Observations.

⸻

27. Architectural Invariants

The following invariants SHALL always hold.

C-1 — Reality

The Collector only records observable reality.

C-2 — No Interpretation

The Collector never interprets observational evidence.

C-3 — Canonical Language

Every Candidate Observation uses the canonical Observation Language.

C-4 — Schema

Every Candidate Observation identifies exactly one applicable Observation Schema.

C-5 — Provenance

Every Candidate Observation contains Provenance.

C-6 — Acceptance Boundary

Only IS-0001 may accept a Candidate Observation.

C-7 — Evidence Preservation

The Collector never intentionally alters the witnessed content of Evidence.

C-8 — Layer Separation

The Collector never performs Artifact, Workspace, Knowledge, Retrieval, or Restoration responsibilities.

C-9 — Uncertainty

Missing information is never fabricated.

C-10 — Privacy

The Collector never makes high-fidelity visual capture persistent ground truth.

C-11 — Locality

Normal Observation capture remains local to the user’s system.

C-12 — Platform Independence

Platform-specific capture mechanisms never become part of the canonical Observation Language.

⸻

28. Collector State Machine

The Collector SHALL conceptually follow:

Platform Signal
      │
      ▼
Signal Validation
      │
      ▼
Canonical Translation
      │
      ▼
Provenance Construction
      │
      ▼
Schema Association
      │
      ▼
Candidate Observation
      │
      ▼
IS-0001
      │
      ├──────────────┐
      ▼              ▼
  Accepted        Rejected

The Collector SHALL NOT introduce an additional semantic interpretation state between Platform Signal and Candidate Observation.

⸻

29. Non-Responsibilities

The Collector SHALL NOT:

* interpret;
* classify;
* rank;
* infer intent;
* infer tasks;
* infer Workspaces;
* identify Artifacts;
* construct Knowledge;
* retrieve information;
* restore work;
* modify history;
* generate explanations of meaning;
* decide what the user was doing;
* decide what the user should do;
* replace Observation Acceptance.

⸻

30. Conformance

A Collector implementation conforms to IS-0020 if and only if:

1. it observes only supported platform signals;
2. it translates those signals into the canonical Observation Language;
3. it constructs Candidate Observations containing Evidence, Provenance, and an applicable Schema Identifier;
4. it does not perform interpretation;
5. it does not perform downstream architectural responsibilities;
6. it does not fabricate missing information;
7. it submits Candidate Observations to IS-0001;
8. it does not bypass Observation Acceptance;
9. it preserves the privacy constraints of Evo;
10. all Collector invariants remain true.

Failure to satisfy any requirement constitutes non-conformance.

⸻

End of Specification