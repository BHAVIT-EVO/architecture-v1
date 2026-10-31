# Ship Happens — submission answers

Paste-ready. Everything below is true of the repo as committed.

---

## 1. Your project in one sentence

```
Evo is the boundary layer for AI agents: it derives where one body of work ends
and the next begins, on the device, without a model, so agents can act on a real
machine without a human labelling every folder first.
```

Why this sentence and not a feature list: every other team is describing what
their agent *does*. This describes what every agent is *missing*. One sentence,
one claim, no adjectives.

---

## 2. Which primary track

```
Kiro (AWS)
```

Pick whichever dropdown option names **Kiro** or the AWS tooling track. If the
list shows "Culture Coded" (Sarvam) as a separate option, do not pick it — those
credits never landed and you did not integrate Sarvam.

If the form has a "why this track" box:

```
Evo is an MCP server. Kiro is an MCP client. The integration is that an AWS
product, running in someone else's IDE, asks my boundary which body of work a
resource belongs to — and gets an answer with evidence and a refusal. The tool
consumes the product rather than the product depending on the tool.
```

---

## 3. What did you build it with?

```
Rust — the engine: 16 crates, deterministic, offline, no HTTP client, no model
in the decision path.

Python 3 (standard library only) — the boundary service: HTTP API on :8765 and
an MCP server over stdio. No frameworks, no pip installs.

Model Context Protocol — three tools exposed to any agent: list_works,
find_work, resume_work.

Kiro (AWS) — the MCP client used in the demo. The agent asks where it is
before it acts.

HTML/CSS/JS — a single-file product UI, no build step, no dependencies.

Data — 10,392 recorded events from one machine over two days, and 164 resources
labelled by hand. The engine claims 2 of the 8 works and refuses the other 6.
```

---

## 4. Working website / hosted project link

**This is live. Paste it as is.**

```
https://bhavit-evo.github.io/evo-boundary-demo/
```

Public, no login, no install, no keys. Three pages: the field report
(`index.html`), the full console (`demo.html`), and the concept film
(`film.html`). All of it runs a synthetic corpus, so publishing it exposes
nobody's browsing.

`BHAVIT-EVO/architecture-v1` is **private** — a link to it 404s for every
judge. Only the Pages URL above works. If a judge asks for source, say so and
send the file; do not make the repository public, because `boundary.json`
holds real browsing.

### Updating the live site

```bash
python3 tools/build_site_v2.py     # only after editing site/src/
bash tools/deploy_site.sh
```

Pages rebuilds in about a minute. Check with:

```bash
curl -s -o /dev/null -w "%{http_code}\n" https://bhavit-evo.github.io/evo-boundary-demo/
```

### What to paste

```
https://bhavit-evo.github.io/evo-boundary-demo/

No install, no keys. Ask "where did I look for flats to rent?" then ask
"what is the capital of france?" — it refuses. Then trip the verifier and
watch the build fail with exit 1. Synthetic corpus, so publishing it exposes
nobody's browsing. Engine, refusal behaviour and verifier are real.
```

---

## 5. Video post link — no video

The form asks for a social post. If you are not recording, do not leave it
blank and do not fake it. **Post the demo as screenshots with the caption
below.** A post with four images and a real writeup is a legitimate submission,
takes three minutes, and needs no microphone.

Screenshots to take, in this order — the first one is the one that sells it:

1. The cover, full screen — *Every agent is told where its work begins. Evo
   derives it.* Nothing else looks like this.
2. Chapter 04, the dark one — eight bodies of work, two claimed, six refused.
3. The gate: press **Declare them the same work** and capture
   `exit code 1 — the build fails`.
4. The ask panel after `where did I look for flats to rent?` — evidence,
   percentage, and *refused by the engine* on the same card.

Take them at 1440 px wide so the type has room. The dark chapter needs the
browser window at least 1200 px or the labels crowd.

### Caption

```
I built the thing AI agents are missing.

Every agent today is told where its work begins and ends. You point it at a
folder. None of them derive it — and you can't get it from content, because two
people can have identical files and be doing different jobs.

Evo derives that boundary locally, on one machine, deterministically, with no
model in the decision. Then it lets any agent ask.

Three things you can do in the demo:
— Ask it something. It answers with evidence, and tells you which work it
  nearly picked instead.
— Ask it something that isn't work. It refuses.
— Try to merge two bodies of work. Inference can't. When you force it, the
  build fails on purpose.

The number I'd defend isn't coverage. It's that the engine claims 2 of the 8
works on my machine and refuses the other 6. A wrong merge is irreversible; a
refusal costs nothing. A boundary that guesses is not a boundary.

It doesn't learn, either — the person legislates. Declare two works separate
and nothing in inference can ever put them back together.

Engine: 16 Rust crates, no HTTP client anywhere, deterministic.
Built at Ship Happens, Juspay Gurugram, with the AWS Student Builder Group.

Demo: https://bhavit-evo.github.io/evo-boundary-demo/
Writeup: DEMO.md

#BuildInPublic #AI #Agents #MCP #Rust
```

If the field is genuinely mandatory and you will not post: put the demo URL
there and note *"written walkthrough in DEMO.md — no video"*. An honest empty
field costs less than a fabricated link.

---

## 6. Anything the judges need to know to try it

```
WHAT THIS IS

Evo derives a person's work boundary — where one body of work ends and the next
begins — on the local machine, and exposes it so any agent can query it. The
decision is made by deterministic rules, not a model. The same log replayed
twice produces the same answer twice.

WHY IT EXISTS

Agents can't safely act on a real desktop because they don't know which of your
files belong to which job. You can't fix that with scale: the boundary isn't in
the content. Two people can have identical files and be doing different jobs.

TRY IT IN TWO MINUTES (no install, no keys)

  git clone https://github.com/BHAVIT-EVO/architecture-v1
  cd architecture-v1 && git checkout arena/2873f4c9-architecture-v1
  cd exports/ship-happens
  python3 evo_server.py serve          # -> http://localhost:8765

Open localhost:8765. Ask "where did I look for hiking clothes?" — it answers
"gear", shows the evidence, and tells you it nearly chose "trek" instead. Ask
"what is the capital of france?" — it refuses. That refusal is the product.

The header will read "2 of 8 claimed". Eight bodies of work were labelled by
hand; the engine independently claimed two and would not assert the other six.
That is not a bug. A wrong merge is irreversible, a refusal costs nothing.

THE PROTOCOL, WITHOUT AN IDE

  python3 mcp_probe.py "where did I look for hiking clothes"

Prints the MCP handshake, lists the three tools, and shows the answer with the
evidence and the rejected work. This is the same exchange an agent gets.

THE BREAK-IT

Open exports/ship-happens/boundary.html by double-clicking it — no server
needed. Select two works, press "Merge by inference": refused. Then press
"Declare them the same work": the verifier goes red and the build fails with
exit 1. Those are the real assertions from crates/evo-threads.

WHAT IS REAL AND WHAT IS NOT — please read this before judging

Real: the engine (16 Rust crates), the corpus (10,392 events, two days, one
machine), the 164 hand labels, the derived boundary, the verifier, the MCP
server, the service, the UI.

Not real: the two buttons in boundary.html demonstrate the gate — they simulate
a fusion and show the verifier rejecting it. They do not write to the engine.

Not built yet: the engine implements six kinds of human declaration (same work,
separate, not mine, classified, dismissed, member state) and SeparateWork is a
standing prohibition — inference can never rejoin two declared-separate works.
The service exposes no endpoint for those yet. Do not claim otherwise.

Not claimed: users, revenue, retention. Nobody outside the company has run it.
The boundary is proven on exactly one person.

BUILT TODAY vs BEFORE

Before: the engine, the corpus, the verifier.
Today: the boundary service, the MCP server, the product UI, the Kiro
integration.
```

---

## Order of operations, now

1. **Record** (20 min). Nothing else matters until the file exists.
2. Upload, post, copy the post link.
3. Fill the form. Submit early — 3:00 PM, not 3:58.
4. Rehearse the three minutes once more, out loud.

The video is the only field you cannot write from memory. Do it first.
