# Evo — Evidence Reconciliation
### What the research actually found, what it invalidated, and what the deck needs next

**Date:** 2026-08-31 (updated after full digest)
**Inputs:** 13 Perplexity reports, **all 13 now digested in full**. §1–6 below were written after the first six; **§8 covers the final seven** (pricing, GTM, evidence bar, objections, permissions/legal, category language, exits) and in two places overrides earlier assumptions.
**Per-report detail:** `research/*-digest.md`. Those files are the permanent record; this document is the interpretation.

---

## The short version

The research did its job, which means it was not kind. Four things we were going to build the deck on are gone, one competitive assumption expired three weeks ago, and the market slide as currently constructed argues *against* funding Evo. None of this is fatal. But drafting slides today would produce a deck that a competent partner dismantles in the first meeting, so the sequence has to change.

The single most useful outcome: we now know which claims are indefensible **before** putting them in front of investors rather than after.

---

## 1. What the research invalidated

### 1.1 The "23 minutes to refocus" statistic — dead

The number does not appear in any Gloria Mark paper. The closest real finding is **25 min 26 sec**, from Mark, Gonzalez & Harris, *No Task Left Behind?* (CHI 2005, N=24 information workers, 700+ hours of ethnographic shadowing).

It does not measure what everyone thinks. It measures **elapsed clock time until interrupted work was resumed**, conditioned on same-day resumption (77.2% of cases). During that gap, workers completed **2.26 other work tasks** — they were working, not recovering. The standard deviation is **54 min 48 sec**, double the mean, which makes the mean a poor summary of anything.

*Consequence:* cut it. If a partner knows this literature — and in this category some will — using it signals we didn't check.

### 1.2 There is no research on multi-day project resumption

Verbatim from the report: **"the research does not exist."** No studies on the time cost of returning to a codebase after a week, on context-reconstruction load, or on fresh-versus-returning worker productivity.

This is the exact thing Evo does. The literature neither supports nor refutes it.

*Consequence:* the problem slide cannot be evidence-cited. It has to be **demonstrated and witnessed** — the demo plus real users describing the pain in their own words. That is a better slide anyway, but it means user evidence moves from "nice to have" onto the critical path.

### 1.3 Part of the interruption literature is counter-evidence

Mark et al. (CHI 2008, N=48): interrupted tasks were completed **faster** than uninterrupted ones (20.31 and 20.60 min vs 22.77 baseline, p<.05), with no difference in error rates. The measurable cost was **stress, frustration, time pressure, effort and mental workload** (p<.01).

A 2023 systematic review (Gross & von Kalben, INTERACT) concludes interruptions are "not uniformly harmful." Zickerick 2020 (EEG) found some distractions *improved* performance.

*Consequence:* **any time-savings ROI framing can be refuted from the same author's own work.** This is a reframe, not a dead end — the honest and better-supported pitch is about the *cognitive and emotional burden of reconstruction*, not minutes recovered. "You lose 23 minutes" becomes "you dread starting again," which is also what users actually say.

### 1.4 "Nobody is doing this" expired on 2026-08-29

Nine products in this space launched between **2026-07-29 and 2026-08-29**: Microsoft Skill Recorder (Jul 29), Mem Agent, **OpenAI's ChatGPT Computer History (Aug 13)**, Hansel (Aug 14), OpenHistory (Aug 19), Project SKY (Aug 21), Ambient Context (Aug 25), Raycast v2.0 (Aug 25), Contrive.

> **Corrected 2026-09-01.** An earlier version of this list included Logical, OpenHuman and Screenpipe and omitted Mem Agent, Raycast v2.0 and Contrive. Those three do not belong in a five-week window claim: **OpenHuman launched 2026-05-13** (outside it), **Screenpipe is ongoing** rather than a launch, and **Logical has no month-level launch date** (its "Aug 2026" in the source table is when the search ran). The nine above are the digest's own list. The force of this finding is the tightness of the window — padding the list with undated or out-of-window entries destroys the claim it is meant to support.

**None of them does the full loop** — observe → classify by project → restore files, tabs and applications. Every one stops at searchable memory or Q&A over history.

*Consequence:* the white space is still real, but it has narrowed to precisely the hard part. The moat is **classification and restoration, never capture**. Capture is now table stakes and OpenAI has it. Two entrants — **Hansel** and **Microsoft Skill Recorder** — could not be verified and must be checked before the competition slide is final; if either restores state, the positioning changes.

### 1.5 The Rewind post-mortem has no primary evidence

The competitive report found **zero Dan Siroker quotes** and states plainly that no retention or usage figures were ever disclosed. Its four explanations are inference from SEO "alternatives" pages.

*Consequence:* never assert "Rewind failed on retention." An investor who knows Siroker will ask how we know, and we don't. The defensible version is narrower and still useful: Rewind raised substantially, pivoted to hardware, and the Mac product no longer exists.

### 1.6 What investors actually do with decks — three tiers, not one

> **Corrected 2026-09-01.** An earlier version of this section credited the product-slide finding to TechCrunch and called it "the one well-sourced finding in that report." That attribution was wrong, and it mattered: the product-slide claim is the *weakest* of the three, not the strongest.

**Genuinely TechCrunch-sourced** (`techcrunch.com/2022/09/22/science-of-pitch-decks`, no published sample size) — these are the findings to design around:

- **None of the failed decks had a financials slide** (financials appear in only ~25% overall). A team slide appears in 100% of decks, successful and failed alike, so its presence earns nothing.
- Unsuccessful decks **frequently omit competition**.
- The **"why are you doing this" purpose slide is a gatekeeper** — disproportionate viewing time, often one sentence on the intro slide, used as a filter before reading on.
- Investors spent **24% less time** on decks in 2022 than 2021; more time on purpose, less on product and business model at pre-seed.

**Weakly sourced, directional only:** "investors spend the least time on the product slide" traces to `preuve.ai`, an SEO blog with no stated sample. Its *ordering* is consistent with real DocSend findings, so it may be directionally right, but it must not be cited as a fact.

**Blacklisted outright:** the "funded decks average 10–12 slides" figure (same SEO blog; contradicts DocSend's published seed length of high-teens-to-20 and the report's own sample of 13/14/11/19/25), the "under 2 minutes of viewing" figure (a corporate press release), the "58% completion rate," and the "15 seconds per page" figure (misattributed to Papermark).

*Consequence:* the demo carries the *meeting*; the deck carries the argument. It needs a financials slide and a competition slide — and Slide 1 is the highest-leverage slide in the deck, not a formality. The one Tier-3 claim worth acting on without citing is the completion rate: front-load the ask, because the mitigation is free.

---

## 2. The biggest unsolved problem: the market slide argues against us

The bottom-up sizing came back with the report's own conclusions:

| Measure | Figure |
|---|---|
| Defensible SOM, 3–5 years | **$1–4M ARR** (6k–24k paying users at $180/yr) |
| Realistic category ceiling | **$5–40M ARR** |
| Pessimistic build | $1.3M TAM → $650k SAM → **$65k SOM**, annotated *"not viable as standalone business"* |

Comparable ceilings — **the reports conflict on two of these, so treat the header "company-disclosed" as false and read the notes:**

| Company | Scale | Note |
|---|---|---|
| Superhuman | **~$36M ARR**; paying-user count **disputed** | Premium email, not a utility. The pricing report says ~70,000 paying (late 2024); the GTM report says 20,000+ users at the same ~$36M ARR. Both cannot be right — at ~$30/mo, $36M implies ~100k seats. **Use the ARR with an "est." label and no user count.** |
| Craft | **No figures disclosed by the company** | The pricing report gives >50,000 paying / >1M active / ~$7.6M ARR est.; the GTM report states Craft disclosed **no user numbers** and is "notoriously private." Known facts only: $8M Series A (Apr 2021), Mac App of the Year 2021. **Attach no ARR or user figure to Craft.** |
| Obsidian | **1.5M+ MAU (2026), $0 raised**, ~$300–350M valuation (est.), <10% annual churn, 1,700+ plugins | Cleanest local-first comp. **No ARR figure is established — do not assert one.** Double-edged: it shows the affection this category earns *and* how little venture money it needed. |
| Arc | 1–5M users, **$0 revenue** | Acquired with no revenue — see the price note in §8.6 |
| Raycast | ~500k users, ~$5M est. | Figures conflict 43× within the report. Also monetized only in **May 2023** after ~3 years free (primary, founder interview) — the closest usable precedent for Evo's model |

> **Corrected 2026-09-01.** The Superhuman and Craft rows previously carried single confident figures under a "company-disclosed" header. Both were contested across reports and Craft's were never disclosed at all. An overconfident correction in the other direction would be the same error, so the conflicts are now stated rather than resolved.

And three findings that cut against the go-to-market assumption: the Mac tools that reached real scale **were mostly free** (Arc $0, Cron made free, Rectangle free); **designers are not a Mac beachhead** (Sketch 1.8% vs Figma 82.3%); **developers are only ~33% macOS**.

The report also names our failure mode in its own words: *"perceived as a 'nice-to-have' utility."*

**This is the thing to solve before anything else.** A pre-seed deck whose honest market ceiling is $5–40M ARR does not clear a venture return threshold, and the arithmetic below the top line is assumption-stacked anyway — the one government-grade figure (70.7M BLS management/professional workers, CPS 2024) is stated and then never used, while the funnel runs on unsourced 60%/50%/0.5%/50%/10% multipliers that also triple-count.

Three possible resolutions, in order of how much I'd trust them:

1. **Reframe the category upward.** "Prosumer Mac utility" is the frame that produces a $4M SOM. Developer tooling and AI infrastructure are sized and funded far more generously, and Evo has a legitimate claim to the former. The funding report's own advice: call it *"prosumer productivity"* or *"developer tooling,"* never *"consumer subscription."*
2. **Lead with the platform argument, not the wedge arithmetic.** Arc got $610M on zero revenue. The return case in this category has historically been strategic value and user base, not ARR multiples. That requires the expansion story to be genuinely load-bearing rather than a closing slide.
3. **Change the buyer.** If individual prosumer pricing caps the outcome, the team/enterprise path needs to appear in the model — which conflicts with local-first and needs thinking through, not asserting.

I have a view (option 1 combined with 2), but this is a positioning decision with real consequences and it should be yours to make.

---

## 3. What the research confirmed or supplied

**Round parameters** (needs primary re-verification — see §4):

| Item | Figure | Source as reported |
|---|---|---|
| Median SAFE cap, $250K–$1M raise | ~$10M post | Carta State of Pre-Seed 2025 |
| Median SAFE cap, $1M–$2.5M raise | ~$15M post | Carta State of Pre-Seed 2025 |
| Median pre-seed pre-money | $7.7M (Q3 2025, down from $8.0M) | PitchBook-NVCA |
| Median dilution at pre-seed | 15.5% | Carta |
| Standard instrument | Post-money SAFE, cap-only, no discount | Carta Q1 2026 |
| **Recommended target** | **$1M–$2.5M at $12–15M post-money cap** | Report's own synthesis |

**Investor targets: 7 lead-capable individuals, realistically 5.** Charles Hudson (Precursor — $250–500K, leads, cold email accepted, Fund V $66M actively deploying, backed Limitless); Manu Kumar (K9 — pre-seed only, leads, warm intro required); Brianne Kimmel (Worklife — $500K–2M, leads, cold email); Ryan Hoover + Vedika Jain (Weekend Fund — explicit "AI, prosumer tools" mandate, application form); Nico Wittenborn (Adjacent — solo GP, 48-hour decisions).

**The two most useful quotes found:**

> *"Everyone is focused on AI intelligence. I think the real battle will be AI memory... Imagine an AI that remembers your meetings, investment philosophy, long-term objectives, previous decisions, preferred communication style and every conversation you've ever had."*
> — Eric Jackson, 2026-07-06, own LinkedIn

> *"compute will continue to move on-device, with the price of more and more non-frontier models approaching $0."*
> — Menlo Ventures, 2025-12-09, own published report

**The bear case to answer, stated by a local-first advocate:**

> local-first *"is a direct attack on vendor lock-in of all forms... But the fact your employer can't exploit this technology with a new revenue model (yet) is a minor problem."*
> — Steven Deobald, 2026-01-01

**Deck structure** — the dominant order across the four genuinely retrievable decks: cover with one-sentence purpose → visceral problem → solution → product screenshot early → why now → traction → validation → market → competition as a 2×2 with an empty quadrant → team with execution credentials → explicit ask → testimonial close.

**Funding climate:** consumer seed down 31% YoY with median consumer seed under $1M, but a16z raised $15B in Jan 2026 including a consumer Apps Fund. Bifurcated, and the framing you choose determines which side you land on.

---

## 4. Research quality — treat these reports as leads, not citations

This matters as much as the findings. Across the six digested reports:

- Multiple reports **cited the exact SEO content farms their own prompts banned** (TechLila, GetLatka, Fueler.io, pitchgrade.com, preuve.ai).
- **No Carta figure was linked to carta.com.** The 15.5% dilution headline traces to a LinkedIn post by an individual; the Q2 2026 figures to another LinkedIn post; the caps to `blog.mean.ceo`.
- Investor quotes routed through **LinkedIn reposts and `yespress.io`**, a scraped-profile aggregator, rather than the speaker.
- One quote attributed to Apple as "corporate thesis" was **an SEO finance blog writing about Apple**. Apple never said it.
- Arithmetic errors: "70k × $30/mo = ~$36M ARR" (it's $25.2M); Raycast listed at both $5.2M and $120k ARR.
- **$610M attached to two different acquisitions** (Meta/Limitless and Atlassian/Browser Company; Atlassian's filing says $488.3M).
- Mem's funding given as "$130M Series C 2023" (actual: ~$23.5M Series A 2022). A fabricated security-researcher handle. Recall described as Snapdragon-only (stale).
- Deck sources included **`media.genppt.com`**, a synthetic CDN serving machine-templated PDFs as "original" decks. The Notion "2016 seed deck" cites 2019 facts and should be discarded entirely.
- Per-slide timings labeled "DocSend 2024" **sum to 5m14s against the report's own 3:44 topline** — arithmetically impossible, and no sample size given anywhere.
- 29–52% of listed source URLs are **never cited in the body** across several reports.
- **South Park Commons' Fall 2026 deadline was 2026-08-02 — already passed.**

Two reports did behave well: the deck report's "could not find" section complied honestly, and the problem-evidence report debunked the 23-minute statistic rather than repeating it. Those two sections are the most valuable output of the entire batch.

**Working rule from here: nothing goes on a slide until traced to primary source.**

---

## 5. Still outstanding

### 5.1 Seven reports unread — RESOLVED

All 13 reports are now digested; findings are in §8. Recording which report was which, since three arrived under decoy filenames:

| Report file | Actually covered |
|---|---|
| `You are a pricing analyst...` | Pricing / monetization (Prompt 5) → §8.1a, §8.2 |
| `You are researching go-to-market case studies...` | GTM playbooks (Prompt 6) → §8.3 |
| `You are a fundraising analyst...` | Pre-seed evidence bar (Prompt 9) → §8.4 |
| `Adversarial briefing` | Objections / pass reasons (Prompt 12) → §8.4 |
| `Executive Summary` | **Permissions & legal** (Prompt 8) → §8.5 |
| `Important limitation` | **Category language** (Prompt 11) → §8.6 |
| `Executive view` | **Exit comps** (Prompt 13) → §8.7 |

The pricing question that "matters most" is answered: a purely local Mac utility **cannot** sustain a venture-relevant subscription without a cloud/AI/team hook (§8.1a).

### 5.2 Verification pass — before any slide

1. **Carta State of Pre-Seed and PitchBook-NVCA** direct from source, for every round-size, valuation and dilution figure.
2. **Hansel** and **Microsoft Skill Recorder** — do either restore application state? This determines whether a true direct competitor exists.
3. **Dan Siroker's own statements** on the Rewind pivot — primary only.
4. **Atlassian/Browser Company price** — $610M or $488.3M.
5. Every investor quote we intend to use, traced to the speaker's own channel.

### 5.3 Genuine evidence gaps no report filled

- **Willingness to pay for local-first/privacy: zero evidence across 276 sources.** This is the moat claim and it is unevidenced. May not be answerable by desk research — could come from waitlist survey questions instead.
- **Acceptability of passive observation software** — no data on whether people will run it.
- **Churn and retention for any comparable product** — not one figure.
- **Multi-project knowledge workers as a population** — the report states no source measures this, so the SAM cannot survive diligence as constructed.
- **JetBrains Developer Ecosystem Survey** — explicitly requested, silently omitted, four unused URLs in the footnotes.

### 5.4 The evidence only you can generate

No amount of research substitutes for this, and given §1.2 it is now the load-bearing evidence in the deck:

- **5–15 real users** on their own machines, and their verbatim words about reconstruction pain.
- **Retention on those users** — D7 and D30, however small the sample.
- **Waitlist with source attribution** — where signups came from, and what fraction converted to active users. Investors will ask the second one.
- **The demo video**, structured as the wedge (return to a project, working in seconds), not an architecture tour.

---

## 6. Recommended sequence

1. **Decide the market framing** (§2). ✅ Done — §7 (developer tooling + platform).
2. **Finish the seven reports** — pricing first. ✅ Done — §8.
3. **Run the verification pass** (§5.2). Small, fast, prevents an unrecoverable meeting.
4. **Run 3–4 new targeted prompts** for the §5.3 gaps, plus the developer-tooling comparables digest (§8.8), aimed narrowly rather than broadly.
5. **Then build the slide outline**, every claim traced to a primary source, with an explicit flag on any slide the evidence cannot support.
6. **Draft copy last.**

We are further from a deck than we were this morning, but the deck we eventually build will survive contact. That trade is worth it.

---

## 7. Decisions taken 2026-08-31

**Market framing: developer / technical-knowledge-worker tooling, plus a load-bearing platform expansion story.** Not prosumer Mac utility. This resolves §2 by option 1 combined with option 2.

Consequences to carry into the outline:

- The word "prosumer" does not appear in the deck. Nor does "consumer subscription." The vocabulary is developer tooling and infrastructure.
- **The ~33% macOS developer share is now a live problem, not a footnote.** "Developers" cannot be the stated beachhead as-is — the segment needs defining more precisely (which kind of technical worker, doing what, on Mac, juggling how many concurrent projects). This is the first thing to nail down in the outline.
- The expansion story moves from a closing slide into the spine of the narrative. If Evo is infrastructure, the deck has to show what it becomes infrastructure *for*.
- Section 12 of `EVO_INVESTOR_RESEARCH_FOUNDATION.md` recommends "the cognitive layer that preserves work continuity across your computer." That line survives the reframe but its supporting market argument does not — the foundation's positioning section needs revising against this decision before drafting.
- The comparable set changes. Superhuman and Craft were the structural comps under the prosumer frame; under a developer-tooling frame the relevant comps are Raycast, Warp, Cursor, Linear, Retool, Vercel and Tailscale. Their scale and funding history need pulling — this is a new research gap created by the decision.

**Report processing: read all seven remaining before any outlining or drafting.** No slide work begins until the full digest exists. Blocked this session on API quota; resume with the pricing report.

### Next session, in order

1. ~~Read the seven remaining reports.~~ **Done — see §8.**
2. Add a digest for developer-tooling comparables, since the framing decision created that gap. (Still open — §8.8.)
3. Run the §5.2 verification pass. (Still open.)
4. Revise the positioning section of `EVO_INVESTOR_RESEARCH_FOUNDATION.md` against the framing decision. (Still open.)
5. Then, and only then, the slide outline.

---

## 8. The final seven reports

The first six reports (§1–4) were about the *problem* and the *market*. These seven are about *building and funding the company*: what a subscription can be built on, how the first thousand users are won, what investors demand and will attack, what's legal, what to call it, and what it's worth at exit. Two of them overturn assumptions we were carrying. The rest sharpen §2 rather than contradict it — the market ceiling keeps reappearing from new directions, which is itself a finding.

Source quality across the seven ranged widely. The exits report (13) is the strongest in the batch — arithmetic verified, primaries (Reuters, SEC, BusinessWire) carrying the load. The objections report (12) is close behind and unusually honest. The evidence-bar report (09) is the weakest: its structure is right but nearly every number is aggregator-laundered. Per-report detail and every sourcing caveat live in the digests.

### 8.1 Two findings that override earlier assumptions

**(a) A pure-local, no-cloud subscription has no precedent. The paid tier needs a cloud, AI, or team hook.**

The pricing report checked every local Mac tool that charges recurring and found the same structure every time: the subscription is attached to a cloud, AI, sync, collaboration, or bundle hook. Raycast ($8/mo) charges for AI and cloud sync; Bear ($2.99/mo) for iCloud sync; Setapp for a 200-app bundle; CleanShot X's base is a **one-time $29** and only its optional *Cloud* tier is a subscription. The tools that are purely local with no cloud — Things ($49.99), Alfred (~$45), TablePlus base ($99) — are **one-time purchases**, and Things is loudly anti-subscription. The report's own recommendation concedes it: to sustain a subscription you must "bundle at least two of AI / cloud sync / collaboration / analytics." A $10–25/mo Mac subscription with no cloud component has **zero supporting precedent in the dataset.**

The GTM report corroborates from the other side. Its local-first heroes either monetize a cloud add-on (Obsidian charges for Sync/Publish, not the app; Bear for iCloud sync) or are one-time purchases (Things). Warp open-sourced its local client in 2026 with the explicit reasoning that "the terminal was never the business… value had moved higher up the stack" — to cloud and AI.

*Consequence, and it is real:* Evo's local-first architecture is the trust story, but it cannot also be the *billing* story. A recurring price needs the roadmap's encrypted iCloud sync — or an AI layer, or team/shared-project features — to be the thing the subscription actually buys. This is not a defect; it is how every comparable does it. But the business-model slide has to name the paid hook, and "local-only subscription" is not it. This closes the open question the old §5.1 flagged as "the business model": on this evidence, a purely local Mac utility **cannot** sustain a venture-relevant subscription without a cloud/AI/team layer.

**(b) Arc is the only company that led with category-creation language, and Arc is the one that died. Lead narrow.**

The category-language report found a clean pattern across every winner it could verify: they started **narrow and concrete.** Loom was "an easy-to-use screen recorder." Tailscale was "a mesh VPN that can be deployed in minutes." Raycast was "a command-line inspired native app to save developers time on non-coding tasks." Slack was "an instant-message-based team communication tool." The grand category language — "Zero Trust Networking," "all-in-one workspace," "your home on the internet" — arrived **later, after the narrow product worked.**

The lone exception was **Arc (The Browser Company), which launched in April 2022 announcing "a brand new category of software"** — and Arc is the one put into maintenance mode and folded into Atlassian. It is the cautionary data point, not the template.

*Consequence, and this is the load-bearing tension with our own §7 decision:* the developer-tooling-plus-platform framing is right for the *return logic* an investor needs — but it must not become the *opening line.* The deck's first self-description should be a narrow, concrete "Evo is the tool that [specific job]," never "Evo is a new cognitive layer / a new computing primitive." The platform ambition belongs in the why-this-is-venture-scale argument (§2, option 2), not in the one-liner. The report even hands us the test to apply, verbatim: *"If the buyer already understands the problem and can name several existing solutions, you are probably entering a category. If the first sales conversation must establish why the problem matters at all, you are attempting category creation."* Because buyers already know the pain of losing work context and can already name Rewind and tab managers, **Evo is entering a category, not creating one** — the cheaper, faster, historically-survivable path. Slack's memo is the model: sell "the ability to ride a horse better," not the saddle.

These two do not conflict with §1.4's "the moat is classification and restoration" or with §7's "expansion story in the spine." They locate them correctly: expansion lives in the *argument*, narrow-concrete lives in the *first sentence*.

### 8.2 Pricing — beyond the crux

Setting the no-cloud-subscription finding aside, the usable pieces:

- **Free-to-paid conversion:** no-credit-card free trial converts ~8–9% median (4–6% is decent, 10–15% strong); permanent freemium 2–5%. Useful as planning ranges, but several rows are SEO-sourced — order-of-magnitude, not gospel.
- **Waitlist → paying:** **~3% pre-launch** (so 1,000 signups ≈ **30 paying**), **~10% once live.** The load-bearing figure rests on a single named analyst (Gaurav Vohra) with no primary link, so treat it as directional. His framing is the quotable part and it is a caution, not a selling point: *"A waitlist measures topic interest, not purchase intent."* This reframes the 1,000-signup waitlist goal — it is a top-of-funnel signal, and investors will ask for the activation rate underneath it.
- **Distribution is forced to direct.** Sandboxing means an Accessibility / screen-recording app effectively cannot ship on the Mac App Store, so Evo sells direct (Stripe, or a merchant-of-record like Paddle/Lemon Squeezy at ~5–6%). Upside: no 15–30% Apple cut. Downside: no App Store discovery — every user is earned. This ties directly to the GTM section.
- **No reliable prosumer-desktop CAC exists** in the report — every figure is self-flagged unreliable. We cannot model acquisition cost from this research.
- The report's recommended price ($10–15/mo) is plausible, but its own annual-discount arithmetic is wrong in two places. **Don't inherit its numbers** — set pricing from the paid hook, not from this report's math.

### 8.3 GTM — the channels that actually have evidence behind them

The case studies here are well-sourced (real founder interviews); the platform/waitlist statistics are SEO-sourced and internally inconsistent (the Product Hunt traffic table is literally backwards), so lean on the former. Two founder-name errors in the report (it's **Chris** Pedregal of Granola, not "Kevin"; the Bear quote is from a third-party YouTuber, not the founder) are a reminder to attribute carefully.

What repeatedly worked, for a two-person team with no audience:

- **Curated waitlist + staged invites.** Linear converted ~10% of a ~10,000 list into ~1,000 users over roughly a year, handpicking by survey response. Scarcity as quality control.
- **High-touch manual onboarding at small scale.** Granola hand-onboarded ~150 people before reaching ~5,000 WAU; Superhuman ran 30-minute 1:1 calls (user pays first) up to 20,000+ users before automating.
- **Community-first before launch.** Rize built ~300 passionate users before it shipped.
- **A developer-channel spike.** Warp got 10,000 waitlist signups on day one from a Show HN; Cursor's first 1,000 came from the MIT/YC network, then pure word-of-mouth to 360k+.

What no longer works: Product Hunt as a cold start (it now only *amplifies* an audience you already have — ~500 followers + 200–300 emails minimum); waitlists left to sit (they "atrophy" — said by both Rewind and Superhuman); usage-capping paywalls on a habit product (backfired for Rewind); and paid social/search ads for an indie Mac tool (Rize got nothing from them). The realistic first-1,000 motion for Evo is a curated waitlist with staged invites *or* a small pre-launch community, plus one dev-channel moment — not a launch-day "publish button."

### 8.4 The evidence bar, and the three questions the deck must survive

**The bar (report 09).** For a no-revenue pre-seed, the qualitative bar is trustworthy and the numeric bar is not. Trustworthy: investors now want team + asymmetric insight + a working demo + **at least one real demand signal** (activation, an LOI, a pilot, or organic waitlist growth). "Vision alone" no longer clears screening. Only three quotes in the entire report are clean primaries safe to quote — Pepe Agell (Pear VC): bringing data that shows customers actually want it "is stronger than anything"; Alfred Lin (Sequoia): "an outlier team, authentic and compelling insights, and positive market dynamics"; and a16z Speedrun's "Demos over decks." **Every threshold number in that report — retention percentages, waitlist sizes, DAU/MAU, the "42% AI premium" — is single-sourced or aggregator-laundered** (all the retention figures trace to one personal blog; a firm called "16VC" cited twice is unverifiable). Do not put any of them on a slide as a benchmark. Planning heuristics that are safe as orders of magnitude: a raise takes ~2–4 months and 20–60 meetings; the demo should hit the "aha" in 1–2 minutes with real usage, not mockups; bring a simple 12–24-month model and an 8–10-slide deck.

**The three hardest questions (report 12).** This is the spine of the deck's defensive half. The report's finale names three questions, rates all three *Serious*, and is candid that every rebuttal "requires evidence the company does not yet have." Verbatim:

1. **"If Apple ships 'search everything I did on my Mac' for free, what remains that Apple cannot or will not copy?"**
2. **"Why does this become a venture-scale company rather than a beloved $2–25 million ARR Mac utility with excellent economics but limited expansion?"**
3. **"Why is this not Rewind again — an invasive personal-memory product that earns attention and funding but fails to become a durable, independent business?"**

They map exactly onto earlier findings: Q1 is §1.4 (the moat is restoration and trust, never capture — 1Password, Alfred, Keyboard Maestro, Hazel, CleanMyMac and Setapp all survived Apple; the report gives examples both ways but, honestly, never quantifies an absorption base rate); Q2 is §2 and §8.7 (the pure local-first prosumer ceiling is Obsidian at ~$25M ARR — Grammarly and 1Password exceed it only because they're enterprise/team-heavy, not pure prosumer); Q3 is §1.5 (never claim Rewind failed on demand — it was *acquired*, "an outcome, not a clean product-market-fit test," with no public cohort data; differentiate on being narrower and safer than universal life-logging). What actually flips skeptics, per the report's Calm / Notion / Life360 cases, is never a clever rebuttal — it is repeated user behavior, retention, organic distribution, and a surface broader than the wedge. That is precisely the §5.4 evidence only real users can generate, which is why it sits on the critical path.

### 8.5 Permissions and legal — one hard gap, one live UX tax, one slide not to write

- **The most-wanted number does not exist.** There is no published permission drop-off / abandonment data for any comparable app — the report says so outright. We cannot claim "X% abandon at the Screen Recording prompt"; there is no such figure to cite.
- **macOS Sequoia added recurring re-authorization prompts** for Screen Recording — weekly in the betas, until developer backlash forced Apple to relax the cadence before release. **The final shipped cadence is not established by any report** (an earlier version of this line stated "monthly"; that figure is unsourced — verify against Apple's release notes before it appears in writing). Either way, for a product that observes continuously this is a live UX tax to design around, and it is the most concrete Apple signal in 24 months that cuts *against* us. The permissions report does not cover macOS 26 / Tahoe at all — treat it as silent on the current OS, not reassuring about it. (The competitive-landscape report does cover Tahoe; see §1.)
- **The slide not to write:** "local-only means no GDPR." The report's reasoned read is that determining the purposes and means of processing likely makes Evo a *controller* regardless of where computation happens; the "we never see the data" exemption is untested in law. Four questions genuinely need counsel: GDPR controller status, whether screen-only (no-audio) capture triggers wiretap statutes, HIPAA BAA exposure, and CCPA classification.
- **Onboarding craft** (patterns, not measured outcomes): just-in-time permission asks, a custom pre-prompt modal before the OS dialog (CleanShot X), and local-only reassurance (Rewind). For the trust slide, Obsidian's line is the closest template: *"Your data is saved locally on your device. No account is required, no telemetry data is collected."*
- One source in that report is disqualifying (an X/Grok AI-bot tweet used as a citation). Ignore anything resting on it.

### 8.6 Category language — see §8.1(b)

The override covers the substance. One caveat to carry: the report is honest and deliberately incomplete — it verified only ~9–10 of 20 companies and refused to fabricate archive URLs. Use its *pattern* (start narrow; Arc as the counter-example) with high confidence; do not treat its individual earliest-wording quotes as archive-verified.

### 8.7 Exits — the return math, verified, and it is the honest tension

The strongest-sourced report of the batch, and it confirms §2 rather than softening it.

- The **modal outcome** for a company like this is **$0–50M** (a small strategic acquisition). The **category-defining ceiling is $500M+**, anchored by **The Browser Company / Atlassian** — the closest comp to Evo in DNA: Mac-first, prosumer, ambitious. **On the price: ~$610M cash was widely reported at announcement (Sep 2025), but Atlassian's filing at close reportedly shows $488.3M.** Use $488.3M, or no number, and never let both appear in the same document. Meta/Limitless (Rewind) is the cautionary comp (undisclosed price, product wound down, reads as an acquihire after >$33M raised) — **never attach a price to it**, as the ~$610M figure also appears erroneously bolted onto that deal in secondary coverage. HP/Humane (~0.5× funding, distressed) anchors the floor.
- The **venture math is verified correct end to end.** At a realistic pre-seed dilution path (15% initial → ~6% at exit), that exit returns **~$29M on the $488.3M figure** (or ~$36.6M on the reported ~$610M) — **enough to return a $25M fund, not a $50M one.** Even the category-defining outcome in this space only returns a small fund. That is the true, unavoidable pitch tension.
- The bootstrapped prosumer-Mac path (Obsidian, Panic, Rogue Amoeba) tops out in the low tens of millions ARR — a good business, not a venture exit.

*Consequence:* the return story is honest only for a smaller fund or an above-median outcome — which is exactly why the §3 investor targets skew to smaller pre-seed funds (Precursor's $66M Fund V, Weekend Fund, Adjacent as a solo GP) rather than large multistage firms whose math needs a $1B+ result. Target selection and the return slide have to agree with each other, and this evidence tells us which side to stand on.

### 8.8 What §8 does to the developer-tooling framing

The §7 decision survives, now fenced by three constraints:

1. **The opening line stays narrow** (§8.1b). Developer-tooling-plus-platform is the return *argument*, not the tagline.
2. **The paid tier needs a cloud / AI / team hook** (§8.1a). The developer tools that charge subscriptions — Warp, Cursor, Linear — are all cloud/AI; none is pure-local. Evo's subscription rides the encrypted-sync / AI / team layer, and the business-model slide must say so plainly.
3. **The beachhead still needs defining.** Only ~33% of developers are on macOS (§7); nothing in the seven reports resolves which technical worker, doing what, on Mac, is the wedge. It remains the first thing to nail in the outline.

And it adds one research gap: a **developer-tooling comparables digest** (Raycast, Warp, Cursor, Linear, Retool, Vercel, Tailscale) — scale, funding history, and pricing model. The pricing and GTM reports touched these only in passing and the exits report treated them as comps, not templates. This is the §8.8 item now sitting at the top of the open-work list.
