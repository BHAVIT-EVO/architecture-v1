# Evo — the boundary layer for AI agents

**Live demo:** https://bhavit-evo.github.io/evo-boundary-demo/

Every agent today is **told** where its work begins and ends — you point it at a
folder. None of them **derive** it. Evo derives the boundary locally, without a
model, and lets any agent ask.

This repo is the demo. It runs a **synthetic corpus** so that publishing it does
not publish anyone's browsing history: eight invented bodies of work for an
invented person.

---

## Try this, in order

1. **Ask** `where did I look for flats to rent?` — it answers `flat`, shows the
   evidence, and names the work it nearly picked instead.
2. **Ask** `what is the capital of france?` — it **refuses**. A boundary that
   guesses is not a boundary.
3. **Select two works → Merge by inference** — refused. There is no inference
   path that fuses two bodies of work.
4. **Then → Declare them the same work** — the verifier goes **red** and the
   build fails with `exit 1`.
5. **Look at the header** — `2 of 8 claimed`. Six works are greyed out, marked
   *refused*.

Steps 3–5 are the thesis: you can try to break it, and it catches you.

---

## Why only 2 of 8

The engine is built to prefer silence. **A wrong merge is irreversible; a
refusal costs nothing.** If it fuses a trek with a jacket the user has to undo
it, and there is no general undo for a merge. If it says nothing, the user loses
a convenience.

The gate asserts *no fusion*, *no leisure anchor*, and *determinism*. It never
asserts coverage. Six refusals is the design, not the score.

---

## What is real here

- The engine: 16 Rust crates, deterministic fold over an event log, offline, no
  HTTP client anywhere, no model in the decision path.
- The gate: fails the build when two bodies of work fuse.
- The corpus behind the local demo: 10,392 recorded events, two days, one
  machine, 164 resources labelled by hand.
- The HTTP service and the MCP server.

## What is not

- This page's corpus is **synthetic**.
- The two buttons demonstrate the *gate* — they simulate a fusion and show the
  verifier rejecting it. They do not write to the engine.
- No users, no revenue, no retention. The boundary is proven on one person.

---

## Run the real thing

```bash
git clone https://github.com/BHAVIT-EVO/architecture-v1   # private
cd architecture-v1 && git checkout arena/2873f4c9-architecture-v1
cd exports/ship-happens
python3 evo_server.py serve      # http://localhost:8765
python3 mcp_probe.py "where did I look for hiking clothes"
```

Python standard library only. No installs, no keys, no network.

---

## The jurisdiction layer

It does not learn. **The person legislates and the engine adjudicates**,
permanently. Six kinds of declaration exist in the engine — same work, separate,
not mine, classified, dismissed, member state. `SeparateWork` is a standing
prohibition: two works declared separate can never be rejoined by inference.

Every resource is **derived**, **declared**, or **refused**. There is no fourth
state.

Learning would make the boundary unreproducible, and you would not give an agent
permission to act on an unreproducible boundary.

---

Built at Ship Happens, Juspay Gurugram, with the AWS Student Builder Group.
