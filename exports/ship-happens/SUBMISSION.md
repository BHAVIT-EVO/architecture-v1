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

HTML/CSS/JS — the public field report and the console: a hand-written scroll
engine, two canvas scenes, a typing transcript, self-hosted type. No
frameworks, no CDN, no dependencies, no build step.

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

## 5. Video post link — no video, post instead

The form will not take prose in a field that wants a URL. So give it a URL:
post the demo on LinkedIn and paste the post link.

### The post

Attach four screenshots (cover · console · gate failing · ask panel), then
paste this. It is ~1,720 characters — inside LinkedIn's 3,000 limit, inside the
1,300–2,500 band that performs best, and the hook lands inside the ~140
characters visible on mobile before "see more".

```
I didn't record a demo video for the hackathon.
I built a site you can break instead.

Evo has been a repo and a few screenshots on here for a while. Today it was a
demo table at Ship Happens, Juspay — one day, about six hours to make the case.

The layer every agent is missing is the least glamorous one: jurisdiction.

Every agent today is told where its work begins and ends. You point it at a
folder. None of them derive it — and you can't fix that with a bigger context
window, because the boundary isn't in the content. Two people can have
identical files open and be doing completely different jobs.

So: derive it locally. Deterministically. No model in the decision. Then let
any agent ask.

The number I'd defend isn't coverage.

On my machine the engine claims 2 of 8 bodies of work.
It refuses the other 6.

That is not a failure to find them. A wrong merge is irreversible; a refusal
costs nothing. A boundary that guesses is not a boundary.

Three things I'd point at:

→ It doesn't learn. The person legislates — declare two works separate, and
nothing in inference can ever put them back together.
→ 16 Rust crates, no HTTP client anywhere, enforced by a test that walks every
manifest so the claim can't quietly rot.
→ Any agent can ask over MCP: list_works, find_work, resume_work. Kiro was the
client.

You can trip the verifier yourself. Select two works, force them together,
watch the build fail with exit code 1:
https://bhavit-evo.github.io/evo-boundary-demo/

Ten chapters, a console, and the engine's real assertions. No install, no keys.

Which would you rather ship: an agent that knows more, or one that knows where
it's allowed to stand?

#BuildInPublic #AI #MCP #Rust #Hackathon
```

### Getting the link

1. Post with visibility **Anyone** — not connections-only, or judges outside
   your network hit a login wall.
2. On the post, the three dots menu → **Copy link to post**.
3. Paste that into the form. It looks like
   `https://www.linkedin.com/posts/bhavit-...-activity-7...`.
4. Open it in a private window to confirm it renders logged out.

### After posting

Reply to every comment in the first hour — comments are weighted far more
than likes. If the organisers have a page (AWS Student Builder Group, ADIGPS),
tag them in a comment rather than the post body; a reshare from them puts the
work in front of judges twice.

---

## 6. Anything the judges need to know to try it

```
WHAT THIS IS
Evo derives a person's work boundary — where one body of work ends and the
next begins — on the local machine, and exposes it so any agent can query it.
Deterministic rules, not a model. The same log replayed twice produces the same
answer twice.

WHY IT EXISTS
Agents can't safely act on a real desktop because they don't know which files
belong to which job. Scale doesn't fix that: the boundary isn't in the content.
Two people can have identical files and be doing different jobs.

START HERE — no install, no keys
https://bhavit-evo.github.io/evo-boundary-demo/
  /             the field report, ten chapters, about three minutes
  /demo.html    the console: eight works, two claimed, six refused
  /source.html  seven excerpts from the engine — the declaration enum, the
                gate assertions, the no-network test, the MCP surface

WHAT TO PRESS
1. demo.html — select two works, press "Declare them the same". The verifier
   goes red and prints exit code 1. That failure is the product.
2. Ask "where did I look for flats to rent?" — it answers with evidence, the
   share of evidence, and the work it nearly picked instead.
3. Ask "what is the capital of france?" — it refuses. A boundary that guesses
   is not a boundary.

WHY ONLY 2 OF 8
Eight bodies of work were labelled by hand. The engine independently claimed
two and would not assert the other six. A wrong merge is irreversible; a
refusal costs nothing.

THE ENGINE
16 Rust crates, a deterministic fold over an event log. No model in the
decision path, and no HTTP client anywhere — enforced by a test that walks
every manifest in the workspace, so the claim cannot quietly rot.

SOURCE
The repository is private because boundary.json holds one person's real
browsing. The engine is real; that corpus is not publishable. Ask and we will
add you to it.
```

---

## Order of operations, now

1. **Record** (20 min). Nothing else matters until the file exists.
2. Upload, post, copy the post link.
3. Fill the form. Submit early — 3:00 PM, not 3:58.
4. Rehearse the three minutes once more, out loud.

The video is the only field you cannot write from memory. Do it first.
