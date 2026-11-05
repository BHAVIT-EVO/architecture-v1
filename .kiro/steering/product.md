# Evo — engineering steering

Read this before every task in this repository. These are not preferences.
If a task appears to require breaking a rule, stop and say so instead.

## What Evo is

Evo derives a person's work boundary locally — where one body of work ends
and the next begins — and exposes it for other programs to query. A "work"
is a body of related activity on one machine over time.

The claim being defended: **the boundary is derived, not declared, and it is
computed without a model.**

## Hard rules

1. **Deterministic.** The boundary is computed by local rules. No model
   decides it. The same input must produce the same output twice, bit for bit.

2. **No hard merges.** Two works fuse only on an explicit human declaration.
   Nothing in inference may reach a merge. A wrong merge is irreversible; a
   wrong split is one click.

3. **Ambiguity is retained, never guessed.** When two works tie, keep both.
   When attention has no production and no recurrence, mint nothing.

4. **External services consume; they never influence.** Sarvam, an LLM, a
   cloud API — any of them may read the boundary and speak about it. None may
   affect how it is computed.

5. **No network calls in `crates/`.** A test asserts that no `TcpStream`,
   `TcpListener` or `UdpSocket` is ever opened
   (`crates/evo-desktop/tests/trust_constraints.rs:202`). Never add one.

6. **Never edit `crates/` during the event.** The engine is frozen. If you
   find a bug, report it; do not fix it.

## Code style for anything you write

- Python: standard library only. No pip, no frameworks, no async.
- Every external call degrades gracefully. If a key is missing or the network
  is down, return the local answer. A demo must never show a stack trace.
- Small files, one responsibility, no abstractions invented for a single call
  site.

## Repository map

```
crates/evo-threads/src/          the engine — FROZEN
  bin/regression.rs              the gate: exits 1 if two works fuse
  bin/boundary_dump.rs           events -> threads.json  (the export path)
exports/ship-happens/            everything demoable — EDIT HERE
  evo_server.py                  HTTP service + MCP server
  evo_ui.html                    product surface
  mcp_probe.py                   MCP client used in the demo
  boundary.html                  offline explorer, no server needed
  threads.json                   engine output (generated)
  boundary.json                  hand-label fallback
tools/threads-regression/        the recorded corpus and hand labels
```

## Vocabulary — use these words, not synonyms

| Use | Not |
|---|---|
| work | thread, project, task, context |
| boundary | grouping, clustering, dedupe |
| derived | inferred, predicted, learned |
| declaration | override, merge request |
| refused / held back | missed, fails, error |
| resume point | last state, snapshot |

"A work the engine refused" is correct. "A work the engine missed" is not —
silence is a designed outcome, not a failure.
