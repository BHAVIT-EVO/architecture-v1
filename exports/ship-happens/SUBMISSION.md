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

There is no hosted instance, and that is the product's claim rather than a gap.
Paste the repo and say so in one line:

```
https://github.com/BHAVIT-EVO/architecture-v1/tree/arena/2873f4c9-architecture-v1/exports/ship-happens

No hosted link, deliberately: the boundary is computed on the device and nothing
about the decision leaves the machine. Two commands to run it:

  cd exports/ship-happens && python3 evo_server.py serve     # localhost:8765
  python3 mcp_probe.py "where did I look for hiking clothes" # the protocol
```

If the form rejects a non-URL or you have twenty spare minutes after the
recording is safe, you can deploy to Render — but **never with the real
corpus**. `boundary.json` contains this machine's actual browsing. A public
instance must run a synthetic day. Do not do this before the backup recording
exists.

---

## 5. Video post link

This field needs a **social post**, not a raw file. Do it in this order:

1. Record 2–3 minutes on the laptop (block D2 of the plan). Phone camera is fine.
2. Upload to YouTube — **public**, not unlisted, because the form asks for a
   post link and unlisted links look evasive.
3. Post it on LinkedIn or X with the caption below.
4. Paste the **post** URL into the form.

### Caption to post

```
I spent today building the thing AI agents are missing.

Every agent today is told where its work begins and ends. You point it at a
folder. None of them derive it — and you can't get it from content, because two
people can have identical files and be doing different jobs.

Evo derives that boundary locally, on one machine, deterministically, with no
model in the decision. Then it lets any agent ask.

Yesterday it was a Rust library. Today it has a mouth: an HTTP service and an
MCP server. Kiro — an AWS product that has never heard of my laptop — asks it
where I left off and gets an answer with evidence, including the work it nearly
picked instead.

The most interesting number from today: the engine claims 2 of the 8 bodies of
work on my machine and refuses to assert the other 6. That's by design. A wrong
merge is irreversible; a refusal costs nothing. A boundary that guesses is not
a boundary.

Built at Ship Happens, Juspay Gurugram, with the AWS Student Builder Group.
Engine: 16 Rust crates, no HTTP client anywhere in it.
Service and MCP: Python standard library.

#BuildInPublic #AI #Agents #MCP #Rust
```

Keep the post honest about the timeline — the "yesterday it was a library" line
does that in six words and reads as confidence rather than a confession.

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
