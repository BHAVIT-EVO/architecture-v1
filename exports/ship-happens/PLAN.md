# Ship Happens — revised plan after the FAQ

**Event:** Sat 31 Oct 2026, 9:30–18:30 IST, Juspay, Gurugram. AWS Student Builder Group × ADIGPS.
**Freeze:** 4:00–4:15 PM. **Demos:** 4:30. **Awards:** 6:00.

## What the FAQ changed

| FAQ line | Effect on the plan |
|---|---|
| "Can we start building before? Absolutely… get a head start." | **Prior work is allowed.** My earlier warning was wrong. Show Evo as it is. Still say one sentence about what is new today — not as a confession, as a comparison. |
| "Your project **must use** the tools/credits associated with your chosen track." | **This is now the only disqualifier.** Evo currently integrates nothing. A perfect demo with no sponsor tool loses to a mediocre demo with one. |
| "You can combine credits… compete under one primary track." | Use two tools, declare one track. Cheapest possible compliance: a real integration you can point at on screen. |
| "No fixed problem statements… tracks give a direction." | No rubric to game. Judging is on the thing in front of them. |
| "Less pitching, more shipping." | Demo > deck. Script shortens; screen time lengthens. |

## The one rule that still applies

Ship Happens allows prior work. It does **not** say you may claim it. Keep
one sentence in the submission — "the engine is months old; today's work is
the boundary explorer and the query layer" — because a judge who discovers
the repo is older than today will re-read everything else you said. Doing it
in your own sentence costs nothing and removes the only way to lose.

## Track choice — the decision rule

Pick the track whose sponsor tool you can show **inside the product**, not
beside it. Then use a second sponsor tool to build it.

1. **If Sarvam credits arrive → primary: Culture Coded.**
   The demo is `ask.py --voice "main kal kis kaam par tha"`: a Hindi question,
   answered by the local boundary, spoken back through Sarvam. The model does
   language; the boundary does meaning. That split is the architecture claim.
2. **If no Sarvam credits → primary: the AWS/Kiro track.**
   Actually build today's code in Kiro and show the spec → task list → diff.
   Kiro is a build tool, so compliance costs the product nothing. Do **not**
   add a network call to the core to satisfy it.
3. Either way, **say which one integration you are pointing at** when you
   submit. Do not make the judge work out why you qualify.

## What is already built

| File | What it is |
|---|---|
| `boundary.html` | Standalone explorer. Lanes of real events, boundary cards, two buttons. "Merge by inference" refused; "declare the same work" fuses and the verifier fails with `exit 1`. No server, no install, works offline. |
| `boundary.json` | The computed boundary artifact: 10,392 events → 8 works (6 real, 2 excluded). |
| `ask.py` | Query CLI. Ask in English; gets the work, the evidence, the share, and **what it rejected**. Refuses when nothing matches. `--voice` speaks via Sarvam. |

## Demo beats (3 min)

1. **0:00** The second job nobody pays for: rebuilding state after every interruption.
2. **0:25** `boundary.html` — this is my actual weekend. Six jobs on one machine.
3. **0:50** The claim: agents are *told* their boundary (a folder). Evo *derives* it. Content can't give it — two people, identical files, different jobs.
4. **1:20** Break it: inference merge → refused.
5. **1:40** Break it properly: declare → gate fails → `exit 1`. "You can break it and it catches you."
6. **2:10** `ask.py "arcteryx for the trek"` → trek, 65%, **also considered: gear(2pt)**. "It tells you what it rejected."
7. **2:30** Hindi voice query via Sarvam (if credits). Nothing left the laptop to compute the boundary.

## Cut list — do not do these today

- Auto-capture on a judge's machine (permissions, minutes, can fail on stage).
- Any network call inside the engine.
- A slide deck. The screen is the deck.
- Editing the engine. Freeze at 1:00 PM and rehearse.

## Submission text

> **Evo — the boundary layer for AI agents.** Every agent today is *told*
> where its work begins and ends: you point it at a folder. None *derive*
> it, and content can't tell you — two people can have identical files and
> different jobs. Evo derives the boundary locally, as a side effect of a
> product used every morning, and exposes it for agents to query.
>
> **Today:** the boundary explorer and the query layer, on a real
> 10,392-event corpus — with a verifier you can try to break.
> **Before today:** the grouping engine (16 Rust crates, no HTTP client).
> **Track tool:** [Sarvam / Kiro] — [one line on what it does here].
