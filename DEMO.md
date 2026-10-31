# DEMO — the boundary layer

**This document is the demo.** There is no video. Everything a three-minute
recording would have shown is written down here, and every claim in it can be
checked in the repository in under two minutes.

---

## The one sentence

> Every agent today is **told** where its work begins and ends — you point it at
> a folder. None of them **derive** it. Evo derives it locally, without a model,
> and lets any agent ask.

---

## Why this is not another agent

An agent that can write code is not new. An agent that knows **which of your
files belong to the same job** is. Without that, an agent acting on a real
desktop is guessing — it opens the wrong document, edits the wrong branch,
sends the wrong client's file.

And it cannot be fixed with a bigger model, because **the boundary is not in
the content.** Two people can have identical files open and be doing
completely different jobs. The boundary lives in how the work arrived, what was
made, what was returned to, and what was only glanced at.

That is what Evo computes.

---

## What you can do in sixty seconds

The demo runs on a **synthetic corpus** — eight invented bodies of work for an
invented person, so nobody's browsing history is published.

| Try | What happens |
|---|---|
| Ask *"where did I look for flats to rent?"* | Returns `flat`, shows the evidence, and names the work it nearly picked instead |
| Ask *"what is the capital of france?"* | **Refuses.** No work matches, and it says so |
| Select two works → **Merge by inference** | **Refused.** No inference path exists |
| Then → **Declare them the same work** | The verifier goes **red** and the build fails with `exit 1` |
| Look at the header | `2 of 8 claimed` — and six greyed rows marked *refused* |

The last two lines are the entire thesis: **you can try to break it, and it
catches you.**

---

## Run it locally

```bash
git clone https://github.com/BHAVIT-EVO/architecture-v1
cd architecture-v1
git checkout arena/2873f4c9-architecture-v1
cd exports/ship-happens
python3 evo_server.py serve          # -> http://localhost:8765
```

Python 3 standard library only. No pip install, no keys, no network.

```bash
# the protocol, without an IDE
python3 mcp_probe.py "where did I look for hiking clothes"
```

That prints a real MCP handshake — `initialize`, `tools/list`, `tools/call` —
and the answer with its evidence and the work it rejected.

```bash
# the offline explorer — double-click, no server at all
open boundary.html
```

### The engine

```bash
cargo run -p evo-threads --bin boundary_dump -- \
  --events tools/threads-regression/sample_events.json \
  --out exports/ship-happens/threads.json
```

Replays 10,392 recorded events through `Engine::apply` and writes one brief per
work. On this corpus the engine mints **2 works and refuses the other 6**.

---

## Architecture

```
  sample_events.json      10,392 recorded events, two days, one machine
        |
        |   cargo run -p evo-threads --bin boundary_dump
        v
  evo-threads ENGINE     Rust · deterministic · offline · no HTTP client
        |
        |   Engine::briefs()  ->  Vec<WorkBrief>
        v
  threads.json           the one contract between the engine and everything else
        |
        |   python3 evo_server.py      (one process, two mouths)
        v
  evo_server.py
        |
        +--- serve :8765   ->  evo_ui.html    a human asks
        |
        +--- mcp   stdio   ->  Kiro · any   an agent asks
```

The engine decides. The service makes the decision **addressable**. Nothing
else is allowed to have an opinion.

---

## What is real, and what is not

Being precise about this is the point. A demo you can trust is worth more than
a demo that impresses.

### Real

- **The engine.** 16 Rust crates, deterministic fold over an event log, no
  model, no network, no HTTP client anywhere.
- **The gate.** `crates/evo-threads/tests/ground_truth.rs` asserts: no two
  works fuse, no leisure resource anchors a thread, and the same log replays to
  the same threads — twice. It fails the build when it is violated.
- **The corpus.** 10,392 events, 13–15 September 2026, one machine, one person.
- **The labels.** 164 resources labelled by hand.
- **The boundary.** Derived by the engine from the events, not written by hand.
- **The MCP server**, the HTTP service, the product UI.

### Simulated

- **The two buttons** in the explorer demonstrate the *gate*. They simulate a
  fusion so you can watch the verifier reject it. They do not write to the
  engine.

### Not working yet

- The engine implements **six kinds of human declaration** — same work,
  separate work, not mine, classified, dismissed, member state. `SeparateWork`
  is a standing prohibition: two works declared separate can never be rejoined
  by inference. **The service exposes no endpoint for these yet.**

### Not claimed

- Users, revenue, retention. Nobody outside the company has run it. The
  boundary is proven on exactly one person.

---

## The jurisdiction layer

This is the part that is easy to describe wrongly, so here it is precisely.

Every resource is in exactly one of three states. There is no fourth.

```
  DERIVED    the engine placed it, from evidence it witnessed
  DECLARED   the person overrode or confirmed it, and the log holds that
  REFUSED    the engine declined to place it at all
```

Evo does not *learn*. It does not tune weights or adapt. **The person
legislates and the engine adjudicates**, permanently — a declaration is written
to the log and honoured on every replay from then on.

From `crates/evo-threads/src/types.rs`:

| Declaration | The person says | The engine does |
|---|---|---|
| `SameWork` | "These belong together" | The only merge path in the system |
| `SeparateWork` | "These are separate" | **A standing prohibition** — inference can never rejoin them |
| `NotMine` | "Not my work" | Permanent quarantine |
| `Classified` | "This is media, not work" | Gates founding and titling only |
| `WorkDismissed` | "Don't show this" | Evidence kept, hidden from the rack |
| `MemberState` | A lifecycle correction | Applies to one member |

Learning would make the boundary unreproducible. An unreproducible boundary is
not something you would give an agent permission to act on.

---

## Why only 2 of 8

The single most misunderstood number here, so it gets its own section.

Eight bodies of work were labelled by hand. The engine independently claimed
two and refused to assert the other six. That is not a coverage failure — the
gate never asserts coverage.

**A wrong merge is irreversible. A refusal costs nothing.**

If the engine fuses a trek with a jacket, the user has to undo it, and there is
no general undo for a merge. If it says nothing, the user loses a convenience
and nothing else. The engine is built to prefer silence.

Ask instead: *was it wrong on any of the two it claimed?* That is the number
worth defending.

---

## Numbers

| | |
|---|---|
| 10,392 | recorded events, 13–15 Sept 2026, one machine |
| 164 | resources labelled by hand |
| 8 | bodies of work, human-labelled |
| 2 | claimed by the engine |
| 6 | refused rather than guessed |
| 0 | HTTP clients in the engine — a test fails the build if one appears |
| 1 | person the boundary is proven on |

50,000 observations / 1,000 resources → 874 ms to build, 4.2 ms to reload
(231×). The quadratic term (~2 µs/pair) is a real ceiling and is documented as
such.

---

## Built when

**Before:** the engine, the corpus, the verifier, the Rust tests.
**At Ship Happens:** the boundary service, the MCP server, the product UI, the
Kiro integration.

Stated plainly because the architecture is the interesting part, not the
timeline.

---

## Repository map

```
crates/evo-threads/src/           the engine
  bin/boundary_dump.rs            events -> threads.json
  bin/regression.rs               the gate: exits 1 if two works fuse
  tests/ground_truth.rs           no fusion, no leisure anchor, deterministic
exports/ship-happens/             everything demoable
  evo_server.py                   HTTP service + MCP server
  evo_ui.html                     product surface
  mcp_probe.py                    a real MCP client
  boundary.html                   offline explorer
site/index.html                   the public demo (synthetic corpus)
tools/build_site.py               regenerates it
```

---

## One line to close

> Every agent shipping right now has memory. None of them have jurisdiction.
> That is the layer.
