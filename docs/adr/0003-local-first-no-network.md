# ADR 0003 — Local-first, with no network client at all

**Status:** accepted

## Context

Evo reads window titles and hears speech. A server would make sync, accounts
and remote models possible, and would also make every privacy claim conditional.

## Decision

No HTTP or TLS client exists in the workspace. Models load from beside the
executable. Text travels over a per-user Unix socket to a local daemon. There
is no account, no sync and no telemetry transport. `trust_constraints.rs`
fails the build if a transport, a screen-capture crate or a non-loopback socket
appears.

## Consequences

- "Stays on this Mac" is true by construction, not by policy.
- Crash reports and analytics can only be written locally; a transport would
  need a new ADR, a new event list and a change to the chip's words.
- The optional `fastembed` feature is the one exception; it stays off by
  default and builds that ship must leave it off.
