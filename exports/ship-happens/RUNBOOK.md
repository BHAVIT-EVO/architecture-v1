# Ship Happens — the complete runbook

**Sat 31 Oct 2026 · Juspay, Gurugram · 9:00 AM – 6:30 PM IST**
AWS Student Builder Group × ADIGPS

You do not need to invent anything today. Everything in this document is
either already built, or is a prompt you paste and a decision you make.

---

## 0. The whole day in eight lines

| Time | You are doing |
|---|---|
| **9:00** | Check in. Open the laptop. Do the pre-flight (§2). |
| **9:30** | Kickoff. Ask one question (§3). Pick your track (§4). |
| **10:10** | Sprint 01. Build the voice layer in Kiro (§5). Two prompts. |
| **2:00** | Lunch. Actually eat. Stop debugging. |
| **2:30** | Sprint 02. Rehearse three times. Record a backup video (§6). |
| **4:00** | Code freeze. Submit (§7). Nothing after this exists. |
| **4:30** | Top projects demo. Three minutes (§8). |
| **6:00** | Awards. Then talk to every judge you can find. |

The thing being judged is **one HTML file, one terminal, and one voice**.
Everything else is preparation for those three minutes.

---

## 1. What you are actually presenting

Four surfaces. Know which is which, because a judge will ask.

| # | Surface | What it is | Built when |
|---|---|---|---|
| 1 | `boundary.html` | The explorer. Real corpus, six works, the boundary map, the query box, and two buttons that let a judge try to break it. | Today |
| 2 | `ask.py` / `serve.py` | The query layer. Question in, work + evidence + what it rejected out. Refuses when nothing matches. | Today |
| 3 | The Sarvam voice layer | Ask in Hindi, hear the answer in Hindi. The model does language; the boundary does meaning. | **Today, in Kiro** |
| 4 | The engine | 16 Rust crates, ~138,000 lines, the grouping and the gate. | Months |

Only #3 is new code you will write in the room. Say so:

> "The engine is months old. Today I built the layer that lets you talk to
> it — the explorer, the query surface, and the voice path through Sarvam."

That sentence is not a confession. It is the reason a judge trusts the
other three minutes.

---

## 2. Pre-flight — first ten minutes at the venue (9:00–9:10)

Do this before anything else, in this order.

1. **Plug in.** Find a socket. A hackathon laptop at 20% at 4 PM is a lost demo.
2. **Open a terminal** and `cd` into the repo. Run:

   ```bash
   cd architecture-v1/exports/ship-happens
   python3 serve.py
   ```

   You should see `http://localhost:8765`. Open it in Chrome.
3. **Click through all four things**: the lanes render, the cards render,
   ask `arcteryx for the trek`, then select two cards and press both buttons.
   If any of it fails, fix it now, not at 4:20.
4. **Kill the server** (`Ctrl-C`) and reopen `boundary.html` by double-clicking
   the file. It must work with no server. That is your fallback if anything
   on the machine misbehaves on stage.
5. **Turn off notifications.** Mac: Focus → Do Not Disturb. A WhatsApp banner
   over your demo is the cheapest way to lose.
6. **Phone**: charge it, silence it, keep it as the backup camera for §6.

**Carry to the stage:** laptop, charger, phone (charged), and nothing else.
No clicker, no adapters you haven't tested.

---

## 3. The one question to ask at kickoff (9:30)

Ask it out loud, in the room, early:

> "For the track requirement — we integrate Sarvam in the product. Is that
> the right level of integration, or do you want it visible in the demo?"

Why this exact wording:

- It confirms you qualify, **before** you spend six hours.
- "Visible in the demo" is the answer you want, and now it's on record.
- It signals you read the FAQ. Organisers remember the person who read the FAQ.

Second question, only if the room is calm:

> "When we submit, should we say which track tool we used, or is that obvious?"

Write both answers down. They go in the submission.

---

## 4. Picking your track (9:40)

**Default answer: the track that maps to Sarvam — Culture Coded.**

Reasons:

- It is the only track where Evo's addition is *visible in the product* rather
  than in your build process.
- A Hindi voice query on an Indian laptop in a Gurugram room lands. A Kiro
  spec file does not.
- You satisfy it with an adapter that does **not** put a network call in the
  engine. Your no-HTTP-client claim survives.

**If the credits don't appear** — say so plainly if asked, and do not fake it —
fall back to the AWS/Kiro track and make Kiro visible instead: build the voice
layer *in Kiro*, and keep the spec → tasks → diff open in a tab. Kiro is a
build tool, so compliance costs the product nothing.

**What you must never do:** add an HTTP client to the engine to satisfy a
track. Your entire differentiator is that the boundary is computed locally.
One `reqwest::Client` in `evo-threads` and the pitch is dead.

The split you are demonstrating:

```
  [ laptop ]                                    [ network ]
  events → boundary (rules, local, no model)  →  Sarvam: language only
                ↑ the claim                          ↑ the track tool
```

Say it in one line: **"Sarvam changes the language the answer is spoken in.
It does not decide the answer."**

---

## 5. Sprint 01 (10:10–2:00) — the build, with prompts

### 5.1 Set up Kiro (10:10–10:30, hard stop)

1. Open `kiro.dev`, download, install.
2. Sign in with **AWS Builder ID**. If credits were handed out at check-in,
   redeem them now.
3. Open the repo folder in Kiro.
4. Create the steering file. Kiro reads `.kiro/steering/*.md` automatically
   on every request, so this is how you stop it from writing bad code.

**Prompt 1 — paste into Kiro's chat, once:**

```
Create a steering file at .kiro/steering/product.md with this content,
verbatim. Do not add to it.

---
# Evo

Evo derives a person's work boundary locally and exposes it for agents to
query. A "work" is a body of related activity on one machine over time.

Hard architectural rules. Never violate these; if a task seems to require
it, stop and say so instead:

1. The boundary is computed locally by deterministic rules. No model
   decides it. The same input must produce the same output twice.
2. No hard merges of stored state. Two works are only fused by an explicit
   human declaration. There is no inference path that fuses works.
3. Split is always available and free. Merge is never free and never
   automatic. A wrong merge is irreversible; a wrong split is one click.
4. The intelligence layer is replaceable. Any external API (Sarvam, LLM,
   cloud) may consume the boundary output. Nothing external may influence
   how the boundary is computed.

Code style: Python standard library only for demo scripts. No pip installs.
No frameworks. Every external call must degrade gracefully to a local
answer when the network or an API key is missing.
---

Then show me the file and confirm it was written.
```

Why this prompt matters: Kiro will otherwise happily add a `requests`
dependency, an async framework, or a model call inside your grouping logic.
The steering file makes those violations visible instead of silent.

### 5.2 The spec for today's actual work (10:30–10:50)

**Prompt 2 — paste into Kiro:**

```
Write a spec for one feature. Do not write code yet; produce
requirements.md, design.md and tasks.md.

FEATURE: voice query of the work boundary.

What the user does:
- Types or speaks a question about their own past work, e.g.
  "which trek was I planning", "what did I build yesterday",
  "main kal kis kaam par tha".
- Sees which work the question belongs to, the evidence that decided it,
  how much of the total evidence points at that work, and which other
  works were considered and rejected.
- Hears the answer spoken aloud in Hindi via Sarvam text-to-speech.

Constraints, non-negotiable:
- The boundary decision is made by local deterministic rules over
  boundary.json. No model is involved in choosing the work.
- Sarvam is used ONLY for text-to-speech of the already-decided answer.
  It must be optional: if SARVAM_API_KEY is unset or the call fails, the
  text answer must still appear and the demo must still work.
- Python standard library only. http.server is acceptable. No Flask.
- The existing files boundary.html, boundary.json, ask.py and serve.py
  are already written. Extend them; do not rewrite them.

Design questions to answer in design.md:
- Where does the query string get parsed, and why is tokenisation the only
  step that touches language?
- How does confidence become a single percentage, and what does that
  percentage mean when two works score close together?
- What happens when nothing matches, and why is refusing correct?
- Why is the Sarvam call the last step and not the first?
- What is the failure path when the network is down at 4:30 PM?
```

### 5.3 Build it (10:50–11:40)

**Prompt 3 — after you've read tasks.md:**

```
Implement task 1 and task 2 from tasks.md. Show me the diff before writing
files. For the Sarvam call, use:

POST https://api.sarvam.ai/text-to-speech
header: api-subscription-key: $SARVAM_API_KEY
body: {"inputs": ["<text>"], "target_language_code": "hi-IN",
       "speaker": "anushka", "model": "bulbul:v2"}
response: {"audios": ["<base64 wav>"]}

If those field names are rejected by the API, read the error body, check
the docs, and fix the payload only. Do not restructure the code. Do not
add a retry loop longer than one attempt — a demo that hangs is worse
than a demo that falls back.
```

**Test the key before you rely on it (11:40):**

```bash
export SARVAM_API_KEY=your_key_here
cd architecture-v1/exports/ship-happens
python3 ask.py --voice "main kal kis kaam par tha"
```

- Sound comes out → you are done with the hard part. Go to §5.5.
- No sound, text answer still appears → **that is a passing demo.** The
  fallback is the feature. Note it and move on.
- Nothing works → delete the voice step from the demo, keep the text
  query, and go to the Kiro fallback in §4.

**Do not spend more than 20 minutes on a failing API key.** Time-box it,
say the fallback line, move on.

### 5.4 If you have time (11:40–1:00) — pick at most one

- **Add three seed questions** as chips in `boundary.html`, so a judge can click
  instead of typing. Cheap, high value. *(Prompt: "Add three example question
  chips above the ask box...")*
- **Add a `--voice` result card** that shows the Hindi text alongside the
  English answer, so the Sarvam step is visible on screen rather than only
  audible.
- Nothing else. Do not touch the engine.

### 5.5 Freeze and rehearse (1:00–2:00)

- **1:00 — stop coding.** Commit everything.
- **1:10 — first rehearsal.** Out loud, standing, timer running. You will be
  at 4:30. Cut.
- **1:30 — second rehearsal.** Do not read from notes. If you lose a sentence,
  keep going; that's what the third is for.
- **1:45 — third rehearsal**, record it on your phone.
- **2:00 — lunch.** Eat. Do not debug over lunch; you will break something
  and spend Sprint 02 fixing it.

---

## 6. Sprint 02 (2:30–4:00)

1. **2:30 — record the backup demo.** Screen recording, no talking needed.
   Four minutes covering everything. Phone camera pointed at the laptop is
   acceptable. This is your insurance against projector failure, missing
   HDMI, or the API being down.
2. **3:00 — write the submission** (§7). Do it now, not at 3:58.
3. **3:20 — hostile Q&A drill.** §9. Use the prompt, not your imagination.
4. **3:45 — final walkthrough** with the actual file you will open on stage.
   One browser window. One tab. Fullscreen. Close everything else.
5. **4:00 — submit.**

---

## 7. The submission (3:00–3:20)

Forms differ, but every field you're likely to see is covered.

**Project name**

```
Evo — the boundary layer for AI agents
```

**One line**

```
Evo derives where one body of work ends and the next begins, locally, so
agents can act on a real machine without a human hand-labelling everything.
```

**Description** (paste as-is)

```
Every AI agent today is TOLD where its work begins and ends: you point it at
a folder. None of them DERIVE it. That's why agents can't safely act on a
real desktop — they don't know that this repo is one job and that deck is
another. No amount of model scale fixes it, because the boundary isn't in
the content. Two people can have identical files and be doing different jobs.

Evo derives the boundary locally, as a side effect of a product people use
every morning, and exposes it for agents to query.

What you can try in the demo:
- The explorer: two days of real activity, 10,392 events, six distinct works
  derived from 164 hand labels.
- Ask the boundary: type a question, get the work, the evidence that decided
  it, and what it rejected. It refuses when nothing matches.
- Break it: try to merge two works by inference — refused. Declare them the
  same and the verifier fails the build with exit 1.

Built today: the explorer, the query layer, and the Sarvam voice path.
Built before today: the grouping engine (16 Rust crates, local-first, no HTTP
client). We're stating that plainly because the interesting part is the
architecture, not the timeline.

Track tool: Sarvam — text-to-speech of the boundary answer in Hindi. The
model changes the language; it does not decide the answer.
```

**Tech stack**

```
Rust (16 crates, the engine) · Python 3 stdlib (demo layer) · Sarvam AI
(text-to-speech) · no cloud, no model in the decision path
```

**Repo link** — `https://github.com/BHAVIT-EVO/architecture-v1`
Point them at `exports/ship-happens/` so they land on the demo, not 138,000
lines of Rust.

---

## 8. The demo (4:30) — three minutes

Fullscreen. One tab. Say this, roughly — not word for word.

**0:00 — the problem, in one breath**

> "Every knowledge worker has a second job nobody pays for: rebuilding what
> they were doing before the last interruption. It isn't a search problem.
> The answer isn't on the web — it's the state of your own machine."

**0:20 — show the lanes**

> "This is my actual weekend. Ten thousand events, 164 hand labels, one
> machine. The engine found six separate bodies of work. A codebase. A trek.
> A pitch. Gear research. Two more. Same machine, same two days, six jobs."

**0:45 — the claim**

> "Here's what nobody has. Every agent today is *told* its boundary — you
> point it at a folder. None of them *derive* it. And you can't get it from
> content, because two people can have identical files and different jobs.
> Work can mean anything to a person."

**1:05 — ask it a question**

Type: `arcteryx for the trek`

> "Trek. Sixty-five percent of the evidence. And look at the bottom — it
> tells me what it rejected: gear, two points. A boundary that hides its
> second guess isn't auditable."

Then: `what is the capital of france`

> "It refuses. A boundary that guesses is not a boundary."

**1:35 — break it**

Select `trek` and `gear` → **Merge by inference** → *refused*.

> "No inference path. A wrong merge is irreversible; a wrong split is one
> click. So only a human can fuse two works."

→ **Declare them the same work** → verifier goes red, `exit code 1`.

> "And when I do declare it, the build dies. Those assertions are in the
> repo. That's not a test suite — it's a verifier, and it's the part I'd
> defend."

**2:10 — the voice**

Ask in Hindi, `--voice`. Let it speak. Let the silence sit.

> "Sarvam changed the language. The boundary was already decided, on this
> laptop, by rules. Nothing about the decision left the machine."

**2:30 — close. Then stop talking.**

> "Every agent shipping right now has memory. None of them have
> jurisdiction. That's the layer, and it's the piece nobody else is
> building."

**Rules for the stage:** never apologise, never say "sorry, it usually
works", never explain a bug you didn't need to show. If something fails,
switch to the recorded video without comment and keep narrating.

---

## 9. The questions you will be asked

Drill these out loud before 3:45.

| They ask | You say |
|---|---|
| "You built this today?" | "The engine is months old. Today I built the layer that lets you talk to it — explorer, query, voice. I'd rather be clear about that than have you find it." |
| "Why would anyone use this?" | "Because their agents are useless on a real machine without it. You can't hand-label a thousand folders for every employee. The boundary has to be derived, once, passively." |
| "Isn't this just a folder?" | "A folder is declared by a person in advance from a menu. This is derived from behaviour, and it's the thing an agent can query. Different artifact." |
| "How do you know it's accurate?" | "164 resources I labelled by hand, and the engine scores against them. 83% of claims are held back rather than asserted. It refuses more than it says." |
| "What about Claude Cowork / Recall?" | "They carry context forward. This defines the boundary that context lives inside. Complementary — a context engine with no jurisdiction is guessing." |
| "Where's the AI?" | "Deliberately not in the decision. The boundary must be deterministic and auditable; that's the product. AI consumes it, and there's an agent layer on the roadmap — no MCP server exists yet, and I won't claim one." |
| "Business model?" | "Per-seat, prosumer first. The wedge is people who lose an hour a day to this. I'm not going to pretend I have enterprise pricing." |
| "What's next?" | "Ten people outside the company running it for thirty days. Right now the boundary is proven on exactly one person." |

If you don't know: **"I don't know — here's how I'd find out."** Never invent
a number. Judges forgive ignorance and remember fabrication.

---

## 10. Contingencies

| If | Then |
|---|---|
| Sarvam key never arrives | Text query only. Say: "the voice path is built and gated behind a key; here's the text answer." Do not apologise twice. |
| Network is down at demo time | Everything except voice is local. That is the point — say it out loud. |
| Projector fails | Phone video, played from the laptop. Already recorded at 2:30. |
| Organiser says "new work only" | Show the git log: today's commits are today's. Offer the Kiro session history. |
| You're asked to cut to 90 seconds | Problem (15s) → ask a question (20s) → break it, `exit 1` (35s) → close (20s). Cut the lanes and the voice. |
| You don't make top projects | Stay for awards, then talk to every judge and organiser in the room. Three conversations are worth more than the prize. |

---

## 11. Prompt appendix — the other tools

### To Claude / ChatGPT — tighten the demo narration

```
Here is a 3-minute hackathon demo script. Rewrite it to be spoken, not read:
shorter sentences, no subordinate clauses, no words I'd stumble over out
loud. Keep every number exactly as written — do not invent or round
anything. Keep the order of the beats. Cut at most 15% of the length.
Output only the script.

<script>
```

### To Claude / ChatGPT — the hostile judge drill

```
You are a judge at an Indian student hackathon. You are technical, you have
seen 40 AI projects today, and you are tired and sceptical. I have 3 minutes.

Interrogate me. One question at a time. After each answer, tell me in one
line what a sceptical judge would think of that answer, then ask the next
question. Escalate: start with what it does, then accuracy, then why this
isn't just a folder, then why it won't be built by an incumbent, then the
business. Do not be encouraging. Stop after 8 questions and give me the two
weakest answers.
```

### To Claude / ChatGPT — the "did I overclaim" check

```
Below is what I plan to say in a hackathon demo. For each claim, mark it:
FACT (verifiable in the repo), INFERENCE (my reasoning), or OVERCLAIM
(unsupported). Be harsh. List only the OVERCLAIMs at the end.

<demo script>
```

### To Kiro — if you have 20 spare minutes

```
Show me every place in this repository that makes a network call or opens a
socket. List file, line, and what it connects to. If there are none, say so
explicitly and tell me which assertions in the test suite prove it.
```

You want the answer "none, and here is the test that fails the build if one
appears." That answer is a demo beat on its own.

---

## 12. After 6:30

Send one message to every judge and organiser you spoke to, the same evening,
while they still remember your face:

> "Thanks for the question about [thing they actually asked]. Here's the repo
> if you want to poke at it: [link]. Happy to show the gate failing on your
> machine if you're curious."

Then, tomorrow, put today's commits behind you and go back to the only thing
that matters: ten people outside the company running it for thirty days.
