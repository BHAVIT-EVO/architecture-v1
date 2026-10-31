# Evo — boundary service

One day, one laptop, no cloud. This folder is everything you demo.

**Start with `Evo_Ship_Happens_Build_Plan.pdf`.** It has the schedule, every
prompt, the demo script and the failure paths. This file is just the map.

## Run it

```bash
cd exports/ship-happens

python3 evo_server.py serve      # http://localhost:8765  <- the product
python3 evo_server.py mcp        # JSON-RPC over stdio    <- the protocol
python3 mcp_probe.py "where did I look for hiking clothes"
```

The service reads `threads.json` if it exists (real engine output, written by
`cargo run -p evo-threads --bin boundary_dump`), otherwise `boundary.json`
(label-derived fallback). It reports which one it used at `/health`.

**When both exist** it shows the eight hand-labelled works and marks each one
`claimed` if the engine independently arrived at it. On this corpus the engine
claims two of the eight and refuses the other six. That gap is real, it is by
design, and it is the strongest thing in the demo: a wrong merge is
irreversible, a refusal costs nothing.

The gate in `tests/ground_truth.rs` asserts no fusion, no leisure anchor and
determinism. It does not assert coverage. The engine is allowed to be silent.

## What is here

| File | What it is |
|---|---|
| `evo_server.py` | HTTP service + MCP server, standard library only. The new thing. |
| `evo_ui.html` | The product surface: works, resume bundle, ask bar, language toggle. |
| `mcp_probe.py` | A real MCP client, so the protocol demo doesn't need venue wifi. |
| `boundary.html` | Offline explorer. Opens with a double-click, no server, no install. |
| `boundary.json` | Label-derived boundary, used when the Rust build is unavailable. |
| `ask.py` | Earlier query CLI. Kept because it works with no service running. |
| `RUNBOOK.md` | The first, shorter plan. Superseded by the PDF. |

## The engine side

```bash
cargo run -p evo-threads --bin boundary_dump -- \
  --events tools/threads-regression/sample_events.json \
  --out exports/ship-happens/threads.json
```

`crates/evo-threads/src/bin/boundary_dump.rs` replays the recorded corpus
through `Engine` and writes one brief per work. It adds no inference of its
own — it is a mouth, not a brain.

## The rule

The boundary is decided by deterministic local rules. No model is in that
path. Sarvam speaks the answer; it does not choose it.

If you add a network call to `crates/`, the claim dies:
`crates/evo-desktop/tests/trust_constraints.rs:202` asserts that no
`TcpStream`, `TcpListener` or `UdpSocket` is ever opened.
