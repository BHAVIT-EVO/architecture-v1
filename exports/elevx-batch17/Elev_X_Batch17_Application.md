# Elev X! Ignite — Batch 17 Application

**Copy each block into the form field it names. The notes under each block are for you, not for them.**

---

## Read this first — four things the research changed

**① You are pitching a B2B incubator, not a VC.** Their words: *"a boutique tech incubator for early-stage B2B founders"* whose job is to take you *"from a working prototype to paid enterprise deployment."* Phase 3 is *"lighthouse customer acquisition, repeatable sales pattern, fundable metrics."* Every answer below has an enterprise buyer in it, because a consumer Mac app does not fit their funnel. The individual is still your wedge — but you describe it as the thing that generates the asset, not as the customer.

**② The competition is brutal and almost entirely AI.** Batch 16 took **9 startups from 650+ applicants** (≈1.4%), and roughly **90% of applicants** propose AI. Being good is not the filter. Being *the only one describing a category* is the filter.

**③ Two of their eight focus areas are yours.** *AI Automation & Governance* and *Privacy & Cybersecurity*. Jurisdiction is governance. Local-first is privacy. Lead with the first, because governance is the word a corporate accelerator can put in a press release.

**④ Batch 16 already accepted the nearest company to you.** One of their nine was **High Fidelity Labs** — *"Gnomon, an AI-native reasoning engine that helps teams carry context forward as their work evolves."* This is either a threat or your best argument, depending entirely on how you frame it. The file below frames it: **Gnomon carries context *forward*; Evo defines the boundary that context lives *inside*.** Summary versus jurisdiction. That distinction is real, it is defensible, and it is the most sophisticated thing you can say to this program.

---

## 1. Elevator pitch — max 100 words

**Format required:** `[Startup] helps [target customer] solve [problem] by [solution], so they can [outcome].`

**Paste this (81 words):**

> Evo helps enterprises deploying AI agents solve the jurisdiction problem: no agent knows where one body of work ends and the next begins, so it cannot safely act on a real desktop. Evo derives that boundary by watching work happen on-device — no cloud, no HTTP client — as a side effect of a product people use every morning. Agents query the boundary instead of being handed a folder, so teams can deploy agents on employee machines without hand-labelling every job.

**Why this and not the consumer version.** Three reasons, and they are the same three that lost you the NEA meeting if you invert them:

- It follows their grammar literally — startup / target / problem / solution / outcome — so a reviewer scanning 650 applications registers it in one pass.
- It leads with **jurisdiction**, as you wanted, but attaches it to a buyer who can write a cheque. "Enterprises deploying AI agents" is a person with a budget. "Knowledge workers" is not.
- It contains one falsifiable technical claim — *no HTTP client* — in the middle of the pitch. That sentence is the only thing in eighty words a technical reviewer can go and check.

**The clause you must be able to defend.** *"Agents query the boundary"* is the roadmap, not today's build. There is no MCP server and no socket listener in the repo — in fact `crates/evo-desktop/tests/trust_constraints.rs` asserts that no `TcpListener` is opened, which means the honesty of the current build is itself a **test**. If someone says "so agents query it today?", the answer is: *"No. The boundary is derived today; exposing it is deliberately blocked until the boundary is trustworthy. The test that fails the build if we break that promise is in the repo."* That answer is stronger than the claim.

---

## 2. Biggest current challenge — free text, no limit

**Paste this (about 240 words):**

> The boundary is proven on exactly one person. Our verifier corpus is 10,392 real events and 164 hand labels across six bodies of work, taken from two days on my own machine. That is enough to fail the build the moment two works merge, and nowhere near enough to prove the map generalises. On top of that, nobody outside the two of us has run the current build on their own machine even once — so every claim we make about daily use is an inference from research, not a measurement.
>
> That is the binding constraint, because our whole thesis is that this boundary can only be learned from someone's real, accumulated work. We have had 300+ conversations with PMs, founder's-office interns, engineers and strategy analysts who describe this exact pain, and 19 of 26 people at a hackathon said they would use it daily if it existed. That is intent. It is not retention, and we do not present it as retention.
>
> What we need next is ten people outside the company running it for thirty days on their own machines, and the discipline to publish whatever breaks. The second constraint follows from the first: we have no repeatable way to reach those ten people. We can build the thing. We have not yet proven we can distribute it.

**Why this answer, specifically.** A weak application says "our biggest challenge is scaling." A strong one names a constraint that is *true*, *specific*, and *solvable by the program you are applying to*. This one does all three — the ask (ten real users, thirty days) is exactly what their Phase 2 "real customer pilots" exists to provide. You are not asking for money in this field; you are describing the experiment that would settle the question, and handing them the role they would play in it.

Note what it deliberately does **not** do: it does not say "we need funding." Funding is a checkbox later; in this field it reads as a lack of imagination.

---

## 3. Funding — "Do you already have investors?"

**Answer: No.**

### Ownership and Funding History

| Shareholder | Instrument | Ownership | Amount invested | Valuation |
|---|---|---|---|---|
| Bhavit Saini | Common stock (founder) | 50% | $0 | N/A |
| Manavv [full legal name] | Common stock (founder) | 50% | $0 | N/A |
| — | — | — | — | — |

**If they ask for rounds:** *N/A — no rounds, no SAFEs, no convertible notes, no outside capital of any kind. Founders hold 100%.*

**If there is a notes field, add:**

> Evo is a Delaware C-corporation, incorporated before any fundraising. Founders hold 100% on common stock; no outside capital, no SAFEs, no notes, and no option pool has been created yet. There is nothing to unwind and no legacy terms.

**Why say Delaware out loud.** Their eligibility gate is explicit: *"Must be a US entity before Phase 2 begins (March 2027)."* You clear it today, three months before applications close. That is a small but real reason to advance you over a team that has not started — so make it visible rather than leaving it implied.

---

## 4. Reason to apply — checkboxes

| Option | Check it? | Why |
|---|---|---|
| **Funding** | ✅ | Up to $250K across phases, up to 11%. You are raising $1.5M; this is real money against it. |
| **Mentorship** | ✅ | Their pitch is engineers, researchers and GTM operators, not generic coaches. |
| **Branding / Credibility** | ✅ | NEC is 125 years old, 45,000 patents, $22B revenue. That name does work you cannot buy at pre-seed. |
| **Access to NEC's technologies** | ✅ **Strongest one** | See the note below. This is the box that separates you from 640 other applicants. |
| **Access to NEC's networks** | ✅ | 55+ international markets, and a corporate parent that has already deployed one portfolio company internally. |
| **Other** | ⬜ Leave blank | Everything you'd want is already in the five above. Ticking "Other" without a distinct reason reads as indecision. |

### Note on "Access to NEC's technologies" — say this in the free-text box, not here

This is the single most valuable thing the research turned up, so it deserves its own paragraph in box 8. Short version for your head:

- NEC has **cotomi**, its own LLM — small, power-efficient, built to run **on-premises** so data never leaves the building.
- NEC launched an **AI Platform Service** in April 2026 aimed squarely at *financial institutions, defense and local government — customers who "cannot move data externally."*
- On **23 April 2026**, NEC became **Anthropic's first Japan-based global partner** and began deploying **Claude Cowork** to roughly **30,000 employees** across three verticals: finance, manufacturing, local government.

Read those three together. **NEC is about to run a desktop agent on 30,000 machines, in exactly the regulated sectors where "which client does this document belong to" is a compliance question rather than a convenience — and Cowork's projects are hand-made folder containers with no derived boundary.** That is your pilot, and it already has a name inside their own company.

---

## 5. Are you applying to other accelerators?

**Answer: Yes.**

Do not soften it. Applying widely is the correct behaviour and every reviewer knows it. The only bad answer is a No nobody believes, or a Yes with an apology attached.

---

## 6. Team — four Yes/No questions

| Question | Answer | Reasoning |
|---|---|---|
| **Relevant industry experience?** | **No** | ⚠️ Confirm against your own CV before submitting. If either of you has worked in enterprise software, developer tools or productivity SaaS, this becomes **Yes** — say so. If not, No is survivable: their own page says *"traction is a bonus, not a prerequisite,"* and 300+ customer conversations are not industry experience, they are customer discovery. Do not conflate the two on a form that gets checked. |
| **Prior founding experience?** | **No** | ⚠️ Confirm this is the first company for both of you. |
| **Prior pre-seed fundraising by any founder?** | **No** | Consistent with box 3. $0 raised. |
| **Experience building software / AI products?** | **Yes** | Unambiguous. 16 Rust crates, ~138,000 lines, a working macOS build, a deterministic engine, on-device voice and embedding models, and a test suite of 1,272 tests across 59 suites. This is not a claim — it is a repository. |

**The pattern to be aware of: three No, one Yes.** That is the honest profile of a first-time technical founder, and it is the profile their Phase 1 is designed for — an equity-free four-week sprint for ~30 teams. Do not pad it. Your compensating evidence is the fourth answer, and it is unusually strong for this stage.

---

## 7. Attendance

Two Y/N fields. Answer both.

| Field | Answer |
|---|---|
| **Can the team attend online (Pacific Time)?** | **Yes** |
| **Can the team attend in person (Silicon Valley — week of 22 March 2027 and week of 12 April 2027)?** | **Yes** |

**Then handle the visa reality in box 8, not here.** A Y/N field cannot hold a caveat, and a caveat typed into one reads as doubt. A documented plan in the free-text box reads as competence. The plan is in the next section.

### The visa maths — know it before you are asked

You have no US visa. The relevant category for two short business trips is **B-1**, not the I-941 parole route (which is for *running* the company from the US, not for attending a two-week intensive).

Current interview wait times in India, recorded August–September 2026:

| Post | B-1/B-2 wait | Notes |
|---|---|---|
| Hyderabad | ~12 months | longest |
| Mumbai | ~11–11.5 months | |
| New Delhi | 8–10 months | |
| Chennai | ~7–7.5 months | |
| Kolkata | ~6–6.5 months | shortest |

Today is 30 October 2026. The in-person weeks are roughly **4.7 and 5.4 months away.** Even Kolkata's six-and-a-half-month queue lands in **mid-May 2027** — after both intensives.

**So the honest position is: it is achievable but it is not automatic, and it only works if you start now.** Three things make it real:

1. **File the DS-160 and pay the MRV fee this week** — the queue starts when you join it, not when you book. Book the earliest slot at Chennai or Kolkata.
2. **Take the $750 premium interview pilot** if India participates — it ran 1 July to 31 December 2026 and is designed for exactly this: 7–9 month queues, appointments in days at a cost.
3. **Ask them directly** whether Phase 2 can be attended remotely if the stamp does not land. Asking turns a hazard into a plan. Not asking turns it into a surprise in March.

---

## 8. "Anything you would like to tell or ask us?" — optional, no limit

This is the highest-leverage box on the form. Three paragraphs, three separate jobs. **Paste this (~330 words):**

> **On eligibility and logistics.** Evo is already a Delaware C-corporation, so the Phase 2 US-entity requirement is met well ahead of March 2027. One honest flag: neither founder currently holds a US visa. We are filing for B-1 visitor visas this week for the two in-person intensives, and we can attend Phase 2 remotely in Pacific Time if a consular appointment does not land in time. We would rather raise that now than have it surface in March.
>
> **On fit with NEC.** NEC's AI Platform Service is built for financial institutions, defence and local government — customers who cannot move data externally — and NEC's own cotomi model is designed to run on-premises. In April this year NEC became Anthropic's first Japan-based global partner and began deploying Claude Cowork to roughly 30,000 employees across finance, manufacturing and local government. That means NEC is about to run a desktop agent on 30,000 machines, in precisely the sectors where "which client does this document belong to" is a compliance question rather than a convenience — and Cowork's projects are folder containers a person creates by hand, with no derived boundary. That is the gap Evo fills, and it is a pilot we could run inside NEC's own Client Zero programme rather than in the abstract.
>
> **On where we sit relative to Batch 16.** High Fidelity Labs' Gnomon carries context forward as work evolves. Evo defines the boundary that context lives inside. They are complementary, not competing: a system that knows where one job ends can make any context engine more useful, and a jurisdiction layer is only worth building if somebody is already solving the summary problem well.
>
> **One question for you.** For a company at our stage — strong technical prototype, no revenue, no outside capital — is Phase 2 realistically reachable, or does the funnel in practice select for teams already in paid pilots? We would rather calibrate now than optimise for the wrong gate.

**Why each paragraph is there:**

- **Paragraph 1** clears their only hard gate (US entity) and names the visa risk before they find it. NEA taught you that a constraint you disclose costs you a little and a constraint they discover costs you everything.
- **Paragraph 2** is the paragraph that gets you a second read. Nobody else in 650 applications will have connected NEC's own Anthropic deployment to a missing jurisdiction layer. It uses their language — Client Zero, cotomi, on-premises — and it points at a pilot inside their own company rather than a hypothetical market.
- **Paragraph 3** pre-empts the objection a reviewer will otherwise form on their own: *"isn't this the same as the Gnomon company we just funded?"* Answering it first turns a threat into evidence that you understand your category.
- **Paragraph 4** is the one most founders leave out, and it is the one that creates a reply. A question they can answer makes the application a conversation. (It is also the exact thing you did not do at NEA.)

---

## 9. Before the 15-minute pitch session

Their process, from the program page:

| Date | What |
|---|---|
| **Oct 5 – Dec 11, 2026** | Applications, rolling review |
| **Nov 4, 12, 18 & Dec 2, 2026** | **45-minute live info sessions** |
| **Dec 1, 2026 – Jan 15, 2027** | **15-minute live online pitch** |
| **Jan 22, 2027** | Decisions |
| **Feb 1 – 26, 2027** | Phase 1 — online, ~30 teams, equity-free |
| **Mar 22 – May 14, 2027** | Phase 2 — hybrid, Silicon Valley, 6–10 teams, $20K |
| **Jun 1, 2027 →** | Phase 3 — ~9 months, 1–3 teams, up to $200K |

**Attend one webinar.** 4 November is five days away. This is the direct fix for what went wrong at NEA: you walked into that call not knowing the format, and read a script into a camera that was off. Elev X publishes its format and holds a session where you can ask what it rewards. Forty-five minutes now is worth more than the next four documents I could write you.

**The 15-minute pitch is the real gate, not this form.** Fifteen minutes, online, with the same people who will decide. Plan: two minutes of pitch, one minute of demo, twelve minutes answering *their* questions — and open by asking what they most want to understand in the time available.

---

## 10. Things only you can confirm

Do not submit until these are right:

1. **Manavv's full legal name** and that the split is exactly 50/50 on the cap table.
2. **The entity name** as filed in Delaware (is it "Evo" something? use the exact registered name).
3. **Relevant industry experience** — your honest answer, not mine. I do not know your CV.
4. **Prior founding experience** for both founders.
5. Whether any advisor, friend or family money has ever touched the company — even ₹50,000 changes box 3 from "no outside capital" to something you must disclose.
6. That your pitch deck exists and is current, because their form says *"around 5 minutes to complete if you already have a pitch deck"* — they expect one to be attached.

---

### Sources

Program terms, phases and dates: `elev-x.secure-platform.com/ignite` and `elev-x.com/elev-x-ignite/` (Batch 17, application window 5 Oct – 11 Dec 2026). Batch 16 selection (9 from 650+) and company descriptions: NEC X press release, 5 October 2026. NEC–Anthropic partnership and the 30,000-employee Claude Cowork deployment: announced 23 April 2026, reported by Anthropic, NEC, Cybernews and ITBrief Asia. NEC AI Platform Service for on-premises regulated customers: NEC, 24 April 2026. B-1 interview waits in India: State Department wait-time data recorded August–September 2026.

**Grades.** Program mechanics and dates — A. Batch 16 companies — A. NEC–Anthropic 30,000 deployment — A (multi-source, primary press release). cotomi running on-premises — B (secondary summaries of NEC material; verify before quoting the model specs aloud). The claim that NEC's Claude Cowork rollout creates a jurisdiction gap — C. That last one is *your inference*, not a published fact. Say it as an inference in the room: *"it looks to me like…"* rather than *"NEC has a problem."*
