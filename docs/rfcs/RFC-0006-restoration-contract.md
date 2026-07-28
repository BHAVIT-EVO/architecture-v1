# RFC-0006 — Restoration Contract

**Status:** Accepted

**Version:** 1.0

**Depends On:**
- Constitution
- Cognitive Model
- Architectural Laws
- RFC-0000
- RFC-0001
- RFC-0002
- RFC-0003
- RFC-0004
- RFC-0005

---

# Abstract

This RFC defines how Evo transforms its current committed understanding into user-visible assistance.

Restoration is the architectural process through which Evo reduces the user's cognitive reconstruction cost, allowing work to continue without requiring the user to manually rebuild context.

Restoration consumes understanding.

It never creates or modifies understanding.

---

# Motivation

Understanding alone provides no value.

The purpose of Evo is not to possess knowledge.

The purpose of Evo is to help the user continue work.

Without Restoration, Evo remains an observer with perfect memory but no practical usefulness.

Restoration exists to transform architectural understanding into assistance while preserving user agency.

---

# Definition

Restoration is the process that converts the current committed Workspace into externally visible assistance.

Its objective is to minimize the user's reconstruction cost while preserving the user's control over the work.

Restoration does not determine what Evo believes.

Restoration determines how Evo assists.

---

# Scope

This RFC defines:

- when Restoration operates;
- what Restoration consumes;
- what Restoration produces;
- the behavioral guarantees Restoration must preserve.

This RFC intentionally does not define:

- UI design;
- application launching;
- browser automation;
- operating-system APIs;
- ordering algorithms;
- ranking algorithms;
- interaction design.

---

# Behavioral Contract

## Requirement 1 — Current Understanding

Restoration MUST operate exclusively upon the current committed Workspace.

Historical Understanding MAY be consulted only for explanation.

Historical Understanding MUST NOT determine the current restoration.

---

## Requirement 2 — Assistance

Restoration SHALL produce externally visible assistance that enables the user to continue work.

Restoration MUST NOT exist solely to expose architectural state.

Understanding is translated into assistance.

---

## Requirement 3 — Reconstruction Cost

The objective of Restoration SHALL be minimizing total reconstruction cost.

Restoration MUST NOT maximize restored state.

Restoration MUST NOT assume that restoring more context necessarily produces better assistance.

---

## Requirement 4 — User Agency

Restoration SHALL preserve the user's authority over the work.

Restoration MAY prepare context.

Restoration MUST NOT autonomously continue, complete, or modify user work unless explicitly requested.

Evo assists.

The user acts.

---

## Requirement 5 — Progressive Restoration

Restoration SHALL permit assistance to be delivered incrementally.

Complete restoration is not required before useful assistance may begin.

Restoration may progress as the user's needs become clearer.

---

## Requirement 6 — Architectural Independence

Restoration MUST NOT modify:

- Observations;
- Artifact identity;
- Workspace formation;
- Interpretation;
- Historical Understanding.

Restoration consumes architectural understanding.

It does not produce it.

---

## Requirement 7 — Explainability

Every restoration presented to the user MUST remain explainable using the current committed understanding.

If questioned, Evo SHALL explain why the restored context was selected.

Explanations of historical restorations SHALL use Historical Understanding in accordance with RFC-0005.

---

# Guarantees

Every compliant implementation guarantees:

- restoration begins from current understanding;
- restoration minimizes reconstruction cost;
- user agency is preserved;
- restoration never mutates architectural knowledge;
- every restoration remains explainable.

---

# Forbidden Behaviour

Restoration MUST NEVER:

- rewrite Observations;
- reinterpret Workspace during restoration;
- silently modify user work;
- maximize restored artifacts without regard for reconstruction cost;
- replace user decision-making;
- depend upon implementation-specific assumptions.

Violation of any of these behaviours invalidates compliance with this RFC.

---

# Architectural Consequences

Restoration establishes the boundary between architectural understanding and user collaboration.

Everything preceding Restoration concerns what Evo knows.

Everything following Restoration concerns how Evo helps.

Restoration is therefore the first architectural process whose output is directly experienced by the user.

---

# Identity Criterion

A Restoration is identified by the current committed Workspace from which it is generated together with the user request it satisfies.

Different user requests over the same Workspace constitute different Restorations.

Changes in implementation do not change Restoration identity.

---

# Non-Goals

Restoration is not responsible for:

- collecting observations;
- inferring identity;
- forming Workspaces;
- evolving interpretation;
- preserving historical understanding;
- learning.

Restoration answers one architectural question only:

> "Given Evo's current understanding, what assistance best enables the user to continue their work with the least reconstruction effort?"

---

# Compatibility

Any implementation conforms to this RFC if it preserves every behavioural guarantee defined herein.

No operating system is prescribed.

No application model is prescribed.

No user interface is prescribed.

Behavioural compatibility is independent of implementation.

---

# Rationale

Evo exists to reduce the cognitive cost of resuming work.

Architectural understanding alone cannot achieve that objective.

Restoration completes the architectural chain by transforming understanding into assistance while preserving the user's ownership of their work.

This separation allows Evo's intelligence to improve indefinitely without changing the collaborative relationship between the system and the user.

---

# Self-Critique

## Assumptions

This RFC assumes the existence of a current committed Workspace as defined by RFC-0003 and RFC-0004.

## Deliberate Omissions

This RFC intentionally omits RestorePlan generation, ranking algorithms, ordering strategies, UI behaviour, operating-system interaction, and automation mechanisms.

These belong to implementation.

## Architectural Boundary

Restoration consumes understanding.

It neither creates nor modifies architectural knowledge.

---

# Architectural Law

> **Restoration shall minimize reconstruction cost while preserving the user's agency.**