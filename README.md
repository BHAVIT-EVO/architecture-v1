<div align="center">

# EVO

### Every agent today is *told* where its work begins and ends.
### Evo *derives* it.

<br>

**[▶ Try the live demo](https://bhavit-evo.github.io/evo-boundary-demo/)** · **[Read the walkthrough](DEMO.md)** · **[How it works](#how-it-works)** · **[What is real](#what-is-real-and-what-is-not)** · **[Run it](#run-it)**

<sub>16 Rust crates · deterministic · offline · no model in the decision path · no HTTP client</sub>

<sub>No sign-up, no API key, no install — the demo runs on a synthetic corpus in your browser.</sub>

</div>

---

Ask an AI agent to "carry on with what I was doing" and watch what happens. It opens a folder — the one you typed into a box. It has no idea that `draft-v3.docx` belongs to the funding application and `notes.md` belongs to a different client entirely, because that distinction was never written down anywhere a machine could read it.

**That is the boundary problem.** Not memory. Not retrieval. Not context length. The question of *where one body of work ends and the next begins* — which is the question every agent has to answer before it is allowed to touch anything real.

Evo answers it, on the device, without a model, deterministically — and then lets any agent ask.

---

## The problem is measured, not asserted

| Finding | Source |
|---|---|
| After an interruption, knowledge workers take **23 minutes and 15 seconds** to return to the original task | Gloria Mark, Daniela Gudith & Ulrich Klocke, UC Irvine — *The Cost of Interrupted Work*, CHI 2008 |
| Attention on any single screen has fallen from **2.5 minutes (2004) to 75 seconds (2012) to 47 seconds (2023)** | Gloria Mark, UC Irvine — *Attention Span*, 2023 |
| **68%** of workers say they do not have enough uninterrupted focus time | Microsoft Work Trend Index 2023 — 31,000 workers, 31 markets |
| Employees spend **57%** of application time communicating and **43%** creating | Microsoft WTI 2023 — Microsoft 365 telemetry, trillions of signals |
| **48%** of employees and **52%** of leaders describe their work as chaotic | Microsoft WTI 2025 — 31,000 workers, telemetry to Feb 2025 |
| The most-pinged **20% of users** are interrupted roughly **every 2 minutes — 275 times a day** | Microsoft WTI 2025 — *top 20% by notification volume, not the average worker* |
| Developers need **10–15 minutes** to resume editing after an interruption; only **10%** resume within a minute | Parnin & Rugaber, *Resumption Strategies for Interrupted Programming Tasks*, 2011 — 10,000 sessions, 86 developers |

Read that last row again, because the detail underneath it is the whole thesis:

> **Only 7% of interrupted programming sessions resumed editing without first navigating through other parts of the code.** The other 93% had to *rebuild the boundary by hand* — file by file, tab by tab — before they could type a single line.

And the work itself was rarely lost: **82% of interrupted work is resumed the same day** (Mark, 2008). You get back to it. You just spend the first ten minutes becoming yourself again.

The work was never lost. **The boundary was.**

---

## Why now

Agents stopped being demos and started being deployments. The protocol that connects them went from experiment to infrastructure in about eighteen months.

| Fact | Source |
|---|---|
| **10,000+** active public MCP servers — up from roughly 1,200 a year earlier | Anthropic ecosystem update, Dec 2025 |
| **~97 million** monthly SDK downloads | MCP ecosystem reports, Q1 2026 |
| **15,926** public repos on the `mcp-server` GitHub topic | GitHub Search API, May 2026 |
| **41%** of surveyed software organisations run MCP servers in limited or broad production | Stacklok 2026 State of MCP report |
| **28%** of Fortune 500 companies run MCP servers | Reported independently by two firms, April 2026 |
| **40%** of enterprise apps will carry task-specific agents by end of 2026 | Gartner forecast |

And then the number that puts Evo exactly where the next problem is. Same year, a different hundred engineers:

> **86% of MCP usage is still experimentation or local development. Only 5% is governed, enterprise-wide production.**
> — Stacklok × Linux Foundation AAIF, July 2026, 100 senior engineers

The agents arrived, and they arrived **on the desktop** — where the work is, where the files are, and where every one of them is currently guessing. The same respondents named their blockers in order: **security and compliance (49%), governance (34%)**. Not protocol. Not model quality. **Governance.**

Meanwhile: **88% of organisations reported confirmed or suspected AI-agent incidents in the past year**, and only **23%** have a formal agent-identity strategy (Gravitee 2026; CSA/Strata, Feb 2026).

You cannot give an agent permission to act on a machine it cannot understand. **Jurisdiction comes before capability.** That gap is open right now, and it is the whole reason this exists.

<details>
<summary><b>On numbers we chose not to use</b></summary>

<br>

A widely circulated *"78% of enterprise AI teams have MCP in production"* figure has no traceable primary source and has been publicly walked back by researchers who tried to verify it. We left it out. Every figure above can be traced to a named report and a date. If a number in this README is not in this section, we do not expect you to believe it.

</details>

---

## The thing that is missing

Everyone is building agents that can *do* things. Nobody is building the thing that tells them **what they are allowed to touch.**

A folder is not a boundary. A folder is a **declaration made in advance from a menu** — you decided, before the work existed, what to call it and where to put it. Real work does not arrive that way. It accumulates: three files in a directory, a tab you kept returning to, a document you only ever read, a branch you made at 2am, and the client's email sitting in another app entirely.

And you cannot recover the boundary from content, because **two people can have identical files open and be doing completely different jobs.** The boundary is not in the bytes. It lives in how the work arrived: what you *made*, what you *returned to*, and what you merely *glanced at*.

That distinction is the entire engine.

---

## What Evo does

Evo watches ordinary computer use, segments it into episodes of attention, and joins those episodes into **bodies of work** — then exposes that structure so anything can query it.

```
  observed activity  →  episodes of attention  →  bodies of work  →  a boundary
                                                                         │
                                              ┌──────────────────────────┤
                                              │                          │
                                        an agent asks              a human asks
                                        (MCP over stdio)           (the product UI)
```

Three design commitments, each one the opposite of how this is normally done:

**1. The decision is deterministic.** No model, no embeddings, no sampling. The same event log replayed twice produces the same boundary, bit for bit. An agent cannot be given permission to act on a boundary that would come out differently tomorrow.

**2. Ambiguity is retained, never resolved.** When two works tie for an episode, both are kept and both are shown. When attention produced nothing and recurred nowhere, **nothing is minted** — a 47,000-second film is still not a project.

**3. Merging is sacred; splitting is free.** Two established works fuse only when a person says so. There is no inference path that can reach a merge. A wrong merge is irreversible; a wrong split is one click.

### The honest scoreboard

On the real corpus in this repository — 10,392 events, two days, one machine — the engine **claims 2 of the 8 bodies of work and refuses the other 6.**

That is not a coverage failure. **It is the design.**

> A wrong merge is irreversible. A refusal costs nothing.
> The engine would rather be silent than fuse a trek with a jacket.

The gate never asserts coverage. It asserts three things, and fails the build when any of them is violated:

- no two labelled works ever fuse
- no leisure resource ever anchors a work
- the same log replays to the same threads, twice

Ask the better question: **was it wrong on the two it claimed?**

---

## The jurisdiction layer

This is where Evo stops being "an app that remembers things" and becomes infrastructure.

Evo does not learn. It has no weights, no fine-tuning, no feedback loop. **The person legislates and the engine adjudicates**, permanently.

Every resource is in exactly one of three states. There is no fourth.

```
  DERIVED    the engine placed it, from evidence it witnessed
  DECLARED   the person overrode or confirmed it, and the log holds that
  REFUSED    the engine declined to place it at all
```

Six kinds of declaration exist in the engine (`crates/evo-threads/src/types.rs`):

| Declaration | The person says | What the engine does |
|---|---|---|
| `SameWork` | "These belong together" | **The only merge path in the system.** |
| `SeparateWork` | "These are separate" | **A standing prohibition** — inference can never rejoin them. |
| `NotMine` | "This is not my work" | Permanent quarantine. Never evidence again. |
| `Classified` | "This surface is of a class" | Gates founding and titling only. Keeps the attention it earned. |
| `WorkDismissed` | "Don't show this work" | Evidence retained; hidden from the rack. |
| `MemberState` | A lifecycle correction | Applies to that one member. |

`SeparateWork` is the load-bearing one. It is not a preference that tilts a score — it is a **prohibition**, recorded in the log and honoured on every replay:

> *"These are separate works; inference will never put them together."*
> — `assign.rs`

Learning would make the boundary unreproducible. And an unreproducible boundary is not something you would give an agent permission to act on.

---

## Try it

**[→ bhavit-evo.github.io/evo-boundary-demo](https://bhavit-evo.github.io/evo-boundary-demo/)**

No install. No keys. Nothing to configure. It runs a **synthetic corpus** — eight invented bodies of work for an invented person — so that publishing the demo does not publish anyone's browsing history.

Do these four things, in order:

**1. Ask it something real.**
`where did I look for flats to rent?`
→ answers `flat`, prints the evidence that decided it, shows the percentage of evidence, and names the work it *nearly* picked instead. Then look closely at the answer: **`flat` is one of the six the engine refuses to claim.** It will answer your question about the work without asserting the work exists. That restraint is the product.

**2. Ask it something that is not work at all.**
`what is the capital of france?`
→ **`Not a job.`** No work matches, and it says so instead of inventing one.

**3. Try to break it.**
Select two works → **Merge these by inference** →
> `Refused. No inference path exists. A body of work is a claim about relationships, and only a person can make it. exit 0.`

**4. Then override it — which is allowed.**
→ **Declare them the same work** → the gate table flips red at `no two works fuse` → `FUSED: {...}` → **`exit code 1` — the build fails.**

Steps 3 and 4 are the pitch. A person can always override the engine. **The engine can never override the person** — and the build refuses to ship when someone tries to let it.

<sub>The two buttons simulate the gate so you can watch it trip. They do not write to the engine. See [what is real](#what-is-real-and-what-is-not).</sub>

---

## How it works

```
  sample_events.json      10,392 recorded events · two days · one machine
        │
        │   cargo run -p evo-threads --bin boundary_dump
        ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  evo-threads  —  the engine                                  │
  │  Rust · deterministic fold over the log · offline            │
  │  no model · no network · no HTTP client anywhere             │
  └──────────────────────────────────────────────────────────────┘
        │
        │   Engine::briefs()  →  Vec<WorkBrief>
        ▼
  threads.json            the one contract between engine and everything else
        │
        │   python3 evo_server.py        one process, two mouths
        ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  evo_server.py                                               │
  │    serve :8765  ──▶  evo_ui.html      a human asks           │
  │    mcp   stdio  ──▶  any MCP agent    an agent asks          │
  └──────────────────────────────────────────────────────────────┘
```

The engine decides. The service makes the decision **addressable**. Nothing else is permitted an opinion.

Three MCP tools, and that is the whole surface:

| Tool | What an agent gets |
|---|---|
| `list_works` | every body of work on the machine, with the refused ones marked |
| `find_work` | which work a resource or phrase belongs to, with evidence and what it rejected |
| `resume_work` | the exact surfaces to reopen — the neighbourhood, not the museum |

---

## Numbers

| | |
|---|---|
| **10,392** | recorded events — two days, one machine, one person |
| **164** | resources labelled by hand |
| **8** | bodies of work, human-labelled |
| **2** | claimed by the engine |
| **6** | refused rather than guessed |
| **0** | HTTP clients in the whole workspace — enforced by a test, not a promise |
| **1** | person the boundary is proven on |

Performance, measured on this corpus:

- 50,000 observations across 1,000 resources → **874 ms** to build the boundary once
- → **4.2 ms** to reload it from the log, a **~200×** difference
- the pairwise term is **~2 µs per pair** and is genuinely quadratic — a real ceiling, documented rather than hidden

---

## What is real, and what is not

We would rather you trust the two claimed works than be impressed by eight. So, precisely:

### Real

- **The engine** — 16 Rust crates, a deterministic fold over an event log. No model, no network, no HTTP client.
- **The local-first guarantee, as code** — `crates/evo-desktop/tests/trust_constraints.rs` walks every `Cargo.toml` and every `.rs` file in the workspace and fails the build if `reqwest`, `hyper`, `ureq`, `attohttpc` or `isahc` ever appears. It is a test, not a claim in a README.
- **The gate** — `crates/evo-threads/tests/ground_truth.rs` asserts no fusion, no leisure anchor and determinism. It fails the build when violated.
- **The corpus** — 10,392 events, one machine, one person, 164 resources labelled by hand.
- **The boundary** — derived by the engine from those events, not written by hand.
- **The HTTP service, the MCP server, the product UI.**

### Simulated

- **The two buttons** in the demo demonstrate the *gate*. They simulate a fusion so you can watch the verifier reject it. They do not write to the engine.
- **The public demo's corpus is synthetic.** The real corpus contains one person's actual browsing and is never published.

### Not built yet

- The engine implements all six declaration kinds and honours them on replay. **The service exposes no endpoint for them yet** — today, declarations are made by replaying a log, not by calling an API.

### Not claimed

- No users, no revenue, no retention. **Nobody outside the company has run this.** The boundary is proven on exactly one person, and that is the next thing to fix — not the next feature to add.

---

## Run it

This repository is **private while the corpus is unredacted** — one real person's browsing lives in `boundary.json`, and it stays off the public internet. Ask, and we will add you. The public demo above is the same engine on a synthetic corpus.

```bash
git clone https://github.com/BHAVIT-EVO/architecture-v1
cd architecture-v1/exports/ship-happens

# the demo — standard library only, no pip, no keys, no network
python3 evo_server.py serve            # → http://localhost:8765
```

Ask the boundary a question without a browser:

```bash
$ python3 mcp_probe.py "where did I look for hiking clothes"

  -> initialize
  <- evo-boundary 0.1.0
  -> tools/list
  <- list_works, find_work, resume_work
  -> find_work("where did I look for hiking clothes")

  answer
    work: gear (id 3)
    evidence: anchor: clothes,hiking; concept: clothes->arcteryx
    confidence: 58%
    rejected: trek(3)
    resume: Last seen: Unisex Hiking & Trekking Clothes | Arc'teryx United States
```

That is a real transcript from this repository, not an illustration. `mcp_probe.py` starts the server as a child process, performs the actual `initialize` → `tools/list` → `tools/call` handshake, and prints both directions of the exchange so you can watch the protocol happen.

Notice what the answer contains: **the confidence, the evidence that produced it, the work it rejected, and the page to reopen.** No prose. No summary. A decision an agent can act on and a person can audit.

The offline explorer needs no server at all:

```bash
open boundary.html      # double-click, works with the wifi off
```

### The engine

```bash
cargo run -p evo-threads --bin regression -- \
  --events tools/threads-regression/sample_events.json \
  --labels tools/threads-regression/ground_truth_labels.json

cargo run -p evo-threads --bin boundary_dump -- \
  --events tools/threads-regression/sample_events.json \
  --out exports/ship-happens/threads.json
```

The regression binary is the gate: it exits **1** if two labelled works ever fuse.

---

## Repository map

```
crates/evo-threads/src/          the engine
  bin/boundary_dump.rs           events → threads.json
  bin/regression.rs              the gate: exits 1 on fusion
  tests/ground_truth.rs          no fusion · no leisure anchor · deterministic
exports/ship-happens/            everything demoable
  evo_server.py                  HTTP service + MCP server
  evo_ui.html                    the product surface
  mcp_probe.py                   a real MCP client
  boundary.html                  offline explorer, no server
site/index.html                  the public demo (synthetic corpus)
tools/build_site.py              regenerates it
docs/                            architecture, audits, research
DEMO.md                          the written walkthrough
```

---

## Roadmap

The honest next three things, in order:

1. **Ten people outside the company, running it for thirty days.** The boundary is proven on one person. Everything else is speculation until that changes.
2. **A declaration endpoint.** The engine already honours six kinds of human declaration; the service should let a person make one without replaying a log.
3. **An agent permission model** built on top of the boundary — an agent asks `find_work`, and the boundary answers with what it may touch, not just what it is.

---

## Sources

Every external number in this README, in one place, so it can be checked instead of trusted:

- Gloria Mark, Daniela Gudith & Ulrich Klocke, *The Cost of Interrupted Work: More Speed and Stress*, CHI 2008 — [doi.org/10.1145/1357054.1357072](https://doi.org/10.1145/1357054.1357072)
- Gloria Mark, *Attention Span* (2023) — the 47-second figure, down from 2.5 minutes in 2004
- Chris Parnin & Spencer Rugaber, *Resumption Strategies for Interrupted Programming Tasks*, Software Quality Journal, 2011 — [link.springer.com/article/10.1007/s11219-010-9104-9](https://link.springer.com/article/10.1007/s11219-010-9104-9)
- Microsoft Work Trend Index 2023, *Will AI Fix Work?* — 31,000 workers, 31 markets, trillions of M365 signals — [microsoft.com/worklab](https://www.microsoft.com/en-us/worklab/work-trend-index/will-ai-fix-work)
- Microsoft Work Trend Index 2025, *The Frontier Firm* — telemetry to Feb 2025, including the top-20%-by-ping-volume cohort — [microsoft.com/worklab](https://www.microsoft.com/en-us/worklab/work-trend-index/2025-the-year-the-frontier-firm-is-born)
- Stacklok, *State of MCP 2026* (41% in limited or broad production) and the Stacklok × Linux Foundation AAIF survey, July 2026 (86% experimentation or local dev; 5% governed production; 49% cite security and compliance)
- Anthropic MCP ecosystem update, Dec 2025 — 10,000+ active public servers
- Gartner, 2026 forecast — 40% of enterprise apps with task-specific agents by end of year
- Gravitee, 2026 — 88% of organisations reporting confirmed or suspected AI-agent incidents
- CSA / Strata, Feb 2026 — 23% with a formal agent-identity strategy

*Correction, if you are reading this after the event: if any figure above has since been revised or withdrawn by its source, the source wins, not this README.*

---

## Built at

Ship Happens, Juspay Gurugram · 31 October 2026 · AWS Student Builder Group × ADIGPS.

**Before:** the engine, the corpus, the verifier.
**At the event:** the boundary service, the MCP server, the product UI, the Kiro integration.

We say that plainly because the architecture is the interesting part, not the timeline.

---

<div align="center">

<br>

> Every agent shipping right now has **memory**.
> None of them have **jurisdiction**.

<br>

<sub>Evo — derive the boundary, then let anything ask.</sub>

</div>
