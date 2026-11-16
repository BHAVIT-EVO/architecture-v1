# ADR 0004 — Scopes, actions and three modes

**Status:** accepted (Phase 4)

## Context

Evo will eventually act on the user's behalf (open, read, write, run). Autonomy
is not built yet, but the model has to exist first or every later feature
invents its own consent.

## Decision

- **Scopes**: folders, apps, sites — the least privilege unit is a place.
- **Actions**: open, read, write, run; write and run are "risky".
- **Modes**: ask every time, ask for risky actions (default), act within scope.
- **Decision order**: kill switch → scope → mode. A target outside every scope
  is denied in every mode; pausing denies even inside a scope.

Implemented in `crates/evo-desktop/src/security/permissions.rs`, persisted as
tab-separated text (no serialization dependency), with malformed lines skipped
and counted.

## Consequences

- A new capability needs a scope and an action before it can ship; there is no
  global "allow Evo" switch to fall back on.
- The model is testable without a UI, which is why the journeys test can cover
  first run, first wake, opening, pausing and refusal end to end.
