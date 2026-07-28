# TRACE-0001 — Continue My Work

**Status:** Canonical Design Reference  
**Version:** 1.0

---

# Purpose

This document describes the canonical end-to-end behavior of Evo when restoring a user's work.

It is not a specification.

It is not an implementation.

It is a behavioral trace used to validate the Foundation, Architecture, and RFCs.

Every RFC should make at least one step of this trace possible.

No RFC should contradict this trace.

---

# Scenario

Date: 18 July 2031

The user has used Evo continuously for five years.

Their system contains millions of observations, thousands of workspaces, and hundreds of ongoing bodies of work.

Nothing is currently open.

The user says:

> "Hey Evo, continue my work with Evo."

Everything below describes only observable behavior.

No implementation details are assumed.

---

# Phase 1 — Receive the Request

Evo receives the spoken request.

At this moment, Evo knows only:

- the user's utterance;
- the current environment;
- the current system state.

The request has not yet been interpreted.

"Evo" is only a reference.

It is not yet associated with any specific body of work.

---

# Phase 2 — Resolve the Reference

Evo determines what the user means by "Evo".

Possible candidates may include:

- engineering;
- product design;
- RFC writing;
- marketing;
- fundraising;
- hiring;
- investor demonstrations.

Evo evaluates every candidate independently.

No candidate is assumed to be correct merely because it exists.

If one interpretation is clearly supported by the available observations, Evo selects it.

If multiple interpretations remain genuinely plausible, Evo asks the smallest possible clarifying question.

Evo never silently guesses when meaningful ambiguity remains.

---

# Phase 3 — Identify the Workspace

Once the reference has been resolved, Evo identifies the body of work the user intends to continue.

This identifies the workspace.

No restoration has occurred yet.

Selecting a workspace does not imply restoring every associated artifact.

The workspace defines only *what* the user wishes to continue.

It does not define *how* continuation will occur.

---

# Phase 4 — Understand the Current State

Evo examines the selected workspace.

Its objective is not to reconstruct history.

Its objective is to determine what remains necessary for meaningful continuation.

Evo distinguishes between:

- work that has already been completed;
- work that is no longer relevant;
- work that remains immediately useful.

Historical association alone is insufficient to justify restoration.

---

# Phase 5 — Construct the Restore Plan

Before modifying the user's environment, Evo constructs a Restore Plan.

The Restore Plan identifies:

- what should be restored;
- what should remain closed;
- why each decision was made.

Every restored resource must contribute to immediate continuation.

Every omitted resource must remain recoverable.

The plan is intentionally minimal.

Its purpose is capability restoration, not exhaustive reconstruction.

---

# Phase 6 — Execute the Restore Plan

Only after planning completes does restoration begin.

Applications open.

Documents reopen.

Repositories return.

Browser tabs appear.

Terminal sessions resume.

The restored environment reflects the Restore Plan.

No unrelated work is introduced.

---

# Phase 7 — Continue Naturally

The user sees the restored environment.

Without searching.

Without remembering.

Without manually reopening forgotten resources.

The user naturally performs the next meaningful action.

This marks successful continuation.

---

# Phase 8 — Learn

As the user continues working, Evo observes new activity.

New observations are recorded.

Future interpretations improve.

Historical observations remain unchanged.

Only future understanding evolves.

---

# Behavioral Guarantees

A compliant Evo implementation guarantees that:

- every restoration begins by resolving the user's reference;
- ambiguity is handled honestly rather than guessed;
- restoration follows planning rather than search;
- restoration minimizes reconstruction effort;
- every restoration decision is explainable;
- historical observations remain immutable;
- interpretation remains replaceable;
- capability restoration is the objective.

---

# Behavioral Failures

The following constitute failures of behavior:

- restoring the wrong body of work;
- restoring unrelated resources;
- restoring completed work while omitting immediately relevant work;
- maximizing historical recall instead of restoring capability;
- being unable to explain restoration decisions;
- relying on unsupported assumptions instead of observable evidence.

---

# Success Criterion

The behavioral trace succeeds when the following interaction occurs.

The user says:

> "Hey Evo, continue my work with Evo."

The user then immediately continues meaningful work without reconstructing context themselves.

The restoration becomes effectively invisible.

The user experiences continuation rather than restart.

---

# Relationship to the RFC Series

This document is not normative.

It exists to validate the RFCs.

Every RFC should enable one or more phases of this behavioral trace.

If an RFC cannot be connected to this trace, its necessity should be questioned.

If a phase of this trace cannot be explained by the RFCs, the specification is incomplete.