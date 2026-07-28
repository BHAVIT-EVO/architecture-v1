# Architectural Laws

**Status:** Frozen  
**Version:** 1.0

---

# Purpose

These laws define the permanent architectural constraints of Evo.

The Constitution defines what Evo believes.

The Cognitive Model defines why continuity is possible.

The Product defines what Evo exists to accomplish.

The Architecture defines how Evo realizes those goals.

These laws define the boundaries that architecture must never violate.

They are independent of programming language, implementation, frameworks, storage engines, machine learning models, and user interface.

Every architectural decision must satisfy these laws.

---

# Law I — Reality Precedes Interpretation

Observable reality exists independently of Evo.

Evo never creates reality.

Evo only observes it.

All interpretation must originate from observable evidence.

No interpretation may exist without supporting evidence.

---

# Law II — Evidence Is Sacred

Observable evidence is the permanent foundation of the system.

Evidence is immutable.

Evidence is never rewritten.

Evidence is never discarded because an interpretation changes.

Every higher-level structure must remain derivable from preserved evidence.

---

# Law III — Interpretation Is Disposable

Every interpretation produced by Evo is provisional.

Workspaces.

Attachments.

Knowledge.

Restore plans.

Suggestions.

Any future interpretation.

All are replaceable.

Improving interpretation must never require rewriting evidence.

---

# Law IV — Observation And Interpretation Must Never Be Confused

Observation records what happened.

Interpretation explains what it may mean.

These are fundamentally different operations.

The architecture must preserve this distinction permanently.

No interpreted state may ever masquerade as observed fact.

---

# Law V — Preserve Uncertainty

When observable evidence does not justify certainty, the architecture must preserve ambiguity.

The system must never invent confidence that evidence does not support.

Uncertainty is legitimate system state.

Not failure.

---

# Law VI — Under-Interpretation Is Better Than Over-Interpretation

Attaching too little context is recoverable.

Inventing incorrect context damages trust.

Whenever uncertainty cannot be honestly resolved, the architecture must prefer incomplete interpretation over incorrect interpretation.

Silence is preferable to fabrication.

---

# Law VII — Replay Must Always Be Possible

Because evidence is immutable and interpretation is disposable, the complete interpretation layer must be reproducible.

Improving the system must require only replaying evidence through improved interpretation.

Historical evidence never changes.

Only understanding improves.

---

# Law VIII — History Must Never Change

Historical facts remain historical.

Snapshots represent what Evo believed at a specific moment.

They are historical records.

Replay may improve future interpretation.

Replay must never rewrite historical history.

History is preserved.

Understanding evolves.

---

# Law IX — The User Owns Judgment

Evo reduces reconstruction effort.

It does not replace human judgment.

Architectural decisions must preserve the user's authority over meaning, priorities, goals, and decisions.

The system assists.

It never decides on behalf of the user.

---

# Law X — Minimize Reconstruction, Never Thinking

The purpose of Evo is not to eliminate thinking.

The purpose of Evo is to eliminate unnecessary re-thinking.

Novel reasoning belongs to the user.

Repeated reconstruction belongs to Evo.

---

# Law XI — Every Decision Must Be Explainable

Every interpretation produced by the architecture must remain attributable to observable evidence.

The system must always be capable of explaining why an interpretation exists.

Explanation is a first-class architectural requirement.

Not a debugging feature.

---

# Law XII — Preserve Layer Separation

Each architectural layer has a single responsibility.

Observation captures reality.

Interpretation constructs meaning.

Retrieval locates relevant information.

Restoration rebuilds working capability.

Learning improves future interpretation.

No layer may silently assume responsibilities belonging to another.

---

# Law XIII — Architecture Must Remain Local-First

All architectural decisions must preserve local ownership of data.

The architecture must never require cloud infrastructure, remote computation, external identity, or continuous connectivity to function correctly.

Synchronization may extend the system.

It must never become a prerequisite for the system.

---

# Law XIV — Capability Is The Objective

The purpose of restoration is not perfect recall.

The purpose of restoration is to restore the user's capability to continue meaningful activity.

Every architectural optimization should ultimately reduce the effort required to produce the next meaningful action.

---

# Law XV — Simplicity Is A Constraint

Every permanent abstraction introduces permanent cost.

The architecture must contain the smallest set of concepts necessary to explain and implement Evo.

No abstraction may exist without irreducible responsibility.

Whenever two concepts explain the same phenomenon, the simpler model must be preferred.

---

## Law XVI — Identity Law

Only concepts that require stable identity across time may exist as computational objects.

A concept requires stable identity if future computation must be able to refer to the same conceptual entity across multiple observations, computations, or system states.

Concepts that merely describe, rank, evaluate, or relate computational objects MUST NOT themselves become computational objects.

Those concepts SHALL instead be represented as relationships, derived values, or transient computational state.

This law exists to prevent unnecessary ontological growth and preserve a minimal, evolvable architecture.

---

## Law XVII — Epistemic Separation

The architecture MUST preserve a strict separation between witnessed information and inferred information.

Observed information SHALL never be represented as inferred information.

Inferred information SHALL never overwrite, replace, or mutate observed information.

Every higher-level computational object MUST remain distinguishable from the observations from which it was derived.

This separation is fundamental to replay, auditability, explainability, and future reinterpretation.

---

# Summary

These laws exist to protect Evo from architectural drift.

Technology will evolve.

Programming languages will evolve.

Models will evolve.

User interfaces will evolve.

These laws should not.

Any future architectural proposal that violates these laws must first justify why the law itself should change.

The burden of proof belongs to the proposed change, not to the existing architecture.