# Evo — Pre-Seed Pitch Deck Build Prompt

**Version:** 1.1 · **Date:** 2026-09-01
**What this is:** A complete, self-contained brief for building Evo's pre-seed pitch deck. Everything the builder needs is inside this document — it does not assume access to any other file.
**Provenance:** Every factual claim below was reconciled from 13 commissioned research reports, then adversarially re-verified against the source digests. Confidence levels and prohibitions are stated inline and are not optional.

**v1.1 changelog — read this if you saw v1.0.** An adversarial verification pass found seven claims in v1.0 that an informed investor could disprove. All are corrected here: the Apple platform signal is downgraded from HIGH to *do-not-assert*; the "funded decks average 10–12 slides" justification is deleted (SEO-blog sourced, explicitly blacklisted, and paired with a causal claim that exists in no source); the five-week competitor list is corrected; Superhuman, Craft, and Obsidian figures are corrected or removed; the Arc exit price is standardized; and the "what investors do with decks" section now separates genuinely-sourced findings from SEO-laundered ones. Two well-sourced findings that v1.0 missed have been added — the purpose-slide gatekeeper effect and Raycast's three-year free runway.

---

# PART 0 — HOW TO USE THIS PROMPT

Copy everything from **PART 1** to the end into the AI you want to build the deck. It is written to be executed, not interpreted. Each slide specifies its purpose, its exact headline, its body content, the evidence it rests on, and what must not appear on it.

**Read PART 1 (Prohibitions) before anything else.** It is the part that keeps this deck from getting the founder embarrassed in a partner meeting. Several claims that sound obviously true — a "23 minutes to refocus" statistic, "local-first is our data moat," "Rewind failed" — were checked and do not survive. They are banned, not discouraged.

**A note on the confidence labels in Part 6.** They are load-bearing. A HIGH-confidence fact can go on a slide as an assertion. A MEDIUM fact can go on a slide as a range or a hedge. A LOW fact does not go on a slide at all — it goes in the founder's head as a risk. Do not promote a fact up a level because it would make a slide stronger. Several claims in this document are deliberately labeled weaker than they were in earlier drafts, because the underlying research was traced to SEO content farms rather than primary sources.

---

# PART 1 — HARD PROHIBITIONS (read first, violate none)

These are absolute. If following an instruction later in this document would require breaking one of these, stop and flag it instead.

**1. Never invent traction, market numbers, customer evidence, investor interest, or product capabilities.** Not as a placeholder that looks real, not as an "illustrative example," not as a rounded guess. If a number is not supplied to you in this document or by the founder, write the marker `[[NEEDS INPUT: description]]` and move on. A visibly empty slot is a task; an invented number is a liability that can end a raise.

**2. Never use a "minutes lost to context switching" statistic.** Specifically banned: "23 minutes to refocus," "23 minutes 15 seconds," "15–30 minutes per context switch," and every variant. Reason: the figure is attributed to Gloria Mark but appears in none of her papers. The nearest real finding (Mark, Gonzalez & Harris, CHI 2005, N=24) is 25 min 26 sec of *elapsed clock time until interrupted work was resumed* — during which subjects completed 2.26 other work tasks — with a standard deviation of 54 min 48 sec, more than double the mean. It does not measure refocus or recovery time. Worse, the same author's CHI 2008 study (N=48) found interrupted tasks were completed *faster* (20.31/20.60 min vs 22.77 baseline, p<.05) with no increase in errors. **A time-savings ROI claim can be refuted from the primary literature.** Sell the cognitive and emotional burden of reconstruction instead.

**3. Never claim local-first is a data moat, or that Evo accumulates proprietary data.** There is no cross-user corpus and no network effect — that is a direct consequence of the architecture. Local-first is a **trust** advantage. The defensibility to claim is: the technical insight (project classification plus reliable native restoration), user trust, and the per-user switching cost of accumulated context and configuration. Investors will probe this specific point; conceding it cleanly reads as sophistication, and claiming a moat that does not exist reads as naivety.

**4. Never claim Rewind failed on retention, demand, or usage.** No retention or usage figures were ever disclosed, and no founder statement attributing failure to those causes was found. The only defensible statements: Rewind pivoted to a wearable (Limitless), Limitless was acquired by Meta, and the Mac product was wound down (reported Dec 2025). Anything beyond that is inference dressed as fact, and a partner who followed that company will know it.

**5. Never use category-creation language.** Banned phrases: "a new category of software," "a new computing primitive," "we're creating a category," "the first ever." Reason: of every company studied, exactly one led with category-creation language — Arc ("a brand new category of software," April 2022) — and Arc is the one whose active development was halted and whose company was absorbed by an acquirer. Every winner started narrow and concrete: Loom was "easy-to-use screen recorder," Tailscale was "a fully functional mesh VPN that can be deployed in minutes," Raycast was "a command-line inspired native app to save developers time on non-coding tasks." The grand language came later, after the narrow product worked.

**6. Never claim "no one else is doing this."** It is false as of August 2026. Nine products in this space launched between 2026-07-29 and 2026-08-29, including OpenAI's ChatGPT Computer History (Aug 13, 2026). Capture is table stakes. The defensible claim is narrower and stronger — see Slide 5.

**7. Never put a top-down TAM on a slide.** No "$X billion productivity software market growing at Y% CAGR." Experienced pre-seed investors read those figures as evidence the founder has not thought about their market. Market sizing is bottom-up, with visible arithmetic, or it is absent.

**8. Never state that local-only processing exempts Evo from privacy law.** The likely position is that the vendor is a controller regardless (GDPR Art 4(7); EDPB Opinion 22/2024). This needs counsel, and a confident legal claim on a slide is an unnecessary risk.

**9. Never assert that Apple will not or cannot move into this space, and never present Apple coexistence as a researched base rate.** The underlying research reached a pro-Evo conclusion on thin evidence and omitted the strongest counterexamples. See Part 6 and Appendix A1 for the honest version. This is the prohibition most likely to be violated by accident, because the flattering version is what the source research says.

**10. No stock photography, no generic AI-brain imagery, no architecture diagrams as hero visuals.** Product screenshots outperform diagrams. If a visual would be decorative, leave the space empty and let the type carry it.

**11. Do not exceed 13 slides in the main deck.** Everything else goes to appendix. Depth belongs in the appendix; the main deck is a narrative. (See Part 5 for why 13 and not some more confident-sounding number.)

---

# PART 2 — WHAT EVO IS

A local-first macOS application. It observes what the user works on, understands which activity belongs to which project, and restores a project's working state — the files, tabs, windows, and applications — so the user can resume without rebuilding context by hand. Processing and storage are on-device and encrypted; no cloud dependency and no account are required to use the core product. Encrypted cross-device sync via iCloud Drive is on the roadmap, not shipped.

**Stage:** working macOS product, demo video in production, waitlist in progress. Two technical co-founders. No revenue.

**What to say about mechanism, and what not to.** Describe *what it does for the user*. Do not put the internal architecture, data model, or component names on a slide. One clean screenshot and one sentence beat a system diagram. There is a widely-repeated claim that investors spend less time on the product slide than any other; it traces to an SEO blog with no stated sample and should not be quoted, though its *ordering* is directionally consistent with real DocSend findings. Treat it as a reason not to build the deck as a feature tour — not as a fact to cite.

**The better-sourced finding, and the one to actually design around:** TechCrunch's analysis of pitch-deck viewing behaviour found the "why are you doing this" slide functions as a **gatekeeper** — investors spend disproportionate time on it, it is often a single sentence on the intro slide, and they use it as a filter before deciding whether to read on. The same analysis found investors spent 24% less time on decks in 2022 than 2021, with the increase concentrated on the purpose slide and the decrease on product and business model *at pre-seed*. **Consequence: Slide 1 is not a formality. It is the highest-leverage slide in the deck.** Build it accordingly.

---

# PART 3 — POSITIONING: TWO REGISTERS, IN THIS ORDER

This is the single most important structural decision in the deck, and it is settled. Evo has two ways of describing itself and the order is not interchangeable.

**Register 1 — the opening self-description.** Narrow, concrete, the first sentence anyone hears:

> **"Evo gets you back into any project — every file, tab, and window — exactly where you left off."**

**Register 2 — the investor/architecture argument.** Only after the job has landed. **The rule, stated once and applied everywhere: Register 2 language appears for the first time on Slide 7 (Market), and nowhere before it.**

> "Evo is the continuity layer for how you work. Current systems remember files and processes; Evo remembers *work*. It starts with restoration, and the same understanding expands into anticipation and workflow automation. Local-first by design."

**Why the order matters.** The infrastructure framing is the venture-scale argument and it must appear in the deck — but leading with it is the Arc move (Prohibition 5). The concrete job earns the right to the bigger claim.

**The category test that settles it** (usable verbatim, from the research):

> "If the buyer already understands the problem and can name several existing solutions, you are probably entering a category. If the first sales conversation must establish why the problem matters at all, you are attempting category creation."

Evo is **entering** a category. Buyers already know this pain and can already name adjacent tools. So the deck should treat the problem as understood and move fast to the product — not spend three slides proving the problem exists.

**Positioning to avoid entirely:** AI productivity tool (crowded, sounds like a feature of something else), screen recorder (privacy red flag), session manager (too shallow), computer-use agent (implies autonomous action), personal knowledge management (implies note-taking), new category (Prohibition 5).

---

# PART 4 — MARKER CONVENTION (how to handle everything uncertain)

The founder explicitly wants best-effort draft content wherever certainty is missing, clearly marked. Use exactly these four markers, and make them visible on the slide itself — not buried in speaker notes.

| Marker | Means | What you do |
|---|---|---|
| `[[NEEDS INPUT: x]]` | A fact only the founder has | Leave the slot visibly empty with the marker. **Never** fill with a plausible number. |
| `[[DRAFT — CONFIRM: x]]` | Your best-judgment draft | Write the best version you can, then attach the marker. |
| `[[BLOCKED: x]]` | Needs a decision or new research first | Write the strongest provisional version, mark it, and state in one line what would unblock it. |
| `[[UNVERIFIED: x]]` | A claim sourced but not primary-verified | Include only if load-bearing; mark it so it gets checked before the deck is shown. |

**Use exactly one marker per slot.** Never combine them (no `[[NEEDS INPUT / BLOCKED: …]]`). If a slot is both missing a fact and awaiting a decision, it is `[[BLOCKED]]` — the decision comes first.

**On every slide containing any marker**, add a visible banner in the top-right corner, small but legible, in amber or grey — not red:

> `DRAFT — this slide needs founder attention before this deck is shown to an investor`

And in that slide's speaker notes, write a short block: what is provisional, why, and what specifically would resolve it. The founder should be able to open the deck and see, at a glance, exactly which slides are finished and which are pending.

**Also produce a consolidated punch list** as the final page of the deck and as a separate section of the copy document: every marker in the deck, grouped by type, in slide order, with what is needed to close it. This is the founder's to-do list.

---

# PART 5 — DESIGN AND WRITING DIRECTION

**Format.** 16:9. Main deck of **10–13 slides** plus a clearly separated appendix.

**On slide count, and why there is no confident number here.** The research produced no trustworthy benchmark, and the builder should know that rather than inherit a fake precision. What the sources actually say: a "funded decks average 10–12 slides" figure circulates widely but traces to a single SEO blog with no sample, contradicts DocSend's own published seed-deck length (high teens to ~20), and contradicts the real decks in the same report (Airbnb 14, Buffer 13, Front seed ~11, Momentum 19, Uber 25) — **it is blacklisted, do not cite it or reason from it.** A separate "8–10 slides for pre-seed" figure traces to three deck-consultancy blogs and is contradicted within the same report by CRV's "10 to 15 slides." **CRV is the only named investment firm in the evidence base, so 10–15 is the most defensible range available, and 13 sits inside it.** Target 11–13. Never justify the length by citing a statistic.

**Front-load, and never put the ask last and nowhere else.** One vendor study claims only 58% of decks are viewed to completion; it is unverified SEO content and must not be quoted, but the mitigation costs nothing and the downside is asymmetric. So: state the raise in one line on Slide 1, and give it a full slide at the end. If a partner reads only the first slide, they should still know what is being asked for.

**Headlines are assertions, not labels.** "Capture just became table stakes" — not "Market Context." The reader must be able to read only the headlines, in order, and follow the entire argument. Test this explicitly before finishing: extract the headlines into a list and check that they narrate the story alone. If they don't, rewrite them.

**Density.** One idea per slide. Target under 40 words of body copy per slide. If a slide needs more, it is two slides or it is appendix.

**Voice.** Founder voice, first person plural, specific and plain. The strongest pre-seed decks read like a technical founder explaining something they understand deeply, not like a consultant's template. No buzzwords, no "revolutionary," no "seamless," no "leveraging." Short sentences. Concrete nouns.

**Visual direction.** Restrained and native-feeling — this is a Mac product for people with taste in Mac products, and the deck's craft is itself evidence the team can build a well-made thing. Real product screenshots as the primary visual language. Generous whitespace. One accent colour used sparingly. A clean system typeface. The deck should feel like the product, and both should feel like they belong on macOS.

**Every numeric claim gets an inline source.** Small, subtle, at the point of the claim or in a footer — publication and year at minimum. This is a credibility signal in itself: it tells a partner that every number in the deck has a traceable origin. Where Part 6 labels a figure an estimate, the slide must say "est." too.

---

# PART 6 — THE EVIDENCE PACK

Everything the builder is permitted to assert, with confidence levels. **Do not go beyond this pack for factual claims.** If a slide seems to need a fact that is not here, mark it `[[NEEDS INPUT]]`.

### Competitive landscape

- **Nine products in this space launched 2026-07-29 to 2026-08-29** (HIGH on the pattern, MEDIUM on individual entries): Microsoft Skill Recorder (Jul 29), Mem Agent, OpenAI ChatGPT Computer History (Aug 13), Hansel (Aug 14), OpenHistory (Aug 19), Project SKY (Aug 21), Ambient Context (Aug 25), Raycast v2.0 (Aug 25), Contrive. **Do not add to this list.** Three products that appear in the wider survey are deliberately excluded and must not be counted in the five-week claim: OpenHuman launched May 2026 (outside the window), Screenpipe is ongoing rather than a launch, and Logical has no month-level launch date. The strength of this claim is the tight window — padding it breaks it.
- **None of the 31 products surveyed performs the full loop** of observe → classify by project → restore files, tabs, and applications. Confidence: MEDIUM — the negative was consistent across all 31 but was not tested hands-on against two entrants (below).
- **OpenAI ChatGPT Computer History** (HIGH — best-sourced competitor in the research, nine press sources including The Register): records clicks, keystrokes, app switches via the macOS Accessibility API; builds a searchable timeline; processes on OpenAI servers with 48-hour retention; opt-in; ChatGPT Pro/Business/Enterprise only; not available in EEA/Switzerland/UK. **Search and summarize only — no restoration, no project classification.** `[[UNVERIFIED: the 48-hour retention figure and the tier availability should be re-checked against OpenAI's own documentation before going on a slide — these are the details most likely to have changed since the research ran, and the ones an investor is most likely to know.]]`
- **Microsoft Recall** (HIGH): Windows-only; privacy-damaged following security research findings.
- **Rewind → Limitless → Meta** (HIGH): pivoted to a wearable pendant; acquired by Meta; Mac product wound down (reported Dec 2025). Raised >$33M. See Prohibition 4 for what may not be said.
- **Arc / The Browser Company** (HIGH on the events): active development halted 2025-05-27; Josh Miller's open letter cited product complexity and failure to reach mainstream users; acquired by Atlassian. **On the price — use $488.3M or no number.** ~$610M was widely reported at announcement (Sep 2025), but Atlassian's filing at close reportedly shows $488.3M. Use $488.3M consistently everywhere in the deck, or omit the figure; do not let the two numbers appear in different places. The ~$610M figure also appears erroneously attached to the Meta/Limitless deal in secondary coverage — never cite a Limitless price.
- **Hansel** — `[[UNVERIFIED]]`: macOS, launched Product Hunt ~Aug 14 2026, positioned "remember everything you've worked on, find past context, and answer questions about your workday. Your data stays encrypted on your Mac." Whether it restores working state is unknown; evidence base is a single Product Hunt page. **This is the most dangerous unverified entry — same platform, same privacy posture, adjacent copy.**
- **Microsoft Skill Recorder** — `[[UNVERIFIED]]`: open-source MIT, launched ~Jul 29 2026, 1.8k+ GitHub stars (as at the research date — this number only moves up, so state it as "1.8k+ as of Aug 2026" or re-check). Records screen sessions and reconstructs them as ordered steps in a SKILL.md for Copilot. Structurally the closest thing to restoration, but it appears to produce an agent script rather than restore a user's working state.
- **Raycast** (HIGH, primary-sourced) — treat as the most serious competitive risk, not an "adjacent tool." It already owns the exact target user (Mac power users), already has paid distribution, and v2.0 (Aug 25, 2026) shipped agents, memory, and screen awareness — the three primitives adjacent to Evo's. Apple absorbing this category is a 2028 problem; Raycast shipping "Workspaces" is a 2027 problem.

### Apple platform risk — LOW confidence, **do not assert on any slide**

This is the section where the source research is weakest and, by its own admission, "errs in the direction that flatters us." Prohibition 9 exists because of it.

- What the research found: no public evidence of Apple restricting Accessibility API access for screen observation as of Aug 2026 — ChatGPT Computer History, Ambient Context, and OpenHistory all use it successfully. macOS 26 Tahoe (Sept 2025) added warnings for background apps and tighter menu-bar app control, and expanded Spotlight with hundreds of system actions and an 8-hour clipboard history, but shipped no native app-state restoration or cross-device work continuity beyond existing Continuity features.
- **What the research missed, and it cuts against us:** macOS Sequoia introduced recurring Screen Recording re-authorization prompts. In the betas these were weekly; developer backlash forced Apple to relax the cadence before release. **Do not state the final shipped cadence — the research does not establish it.** This is the single most concrete Apple signal in the last 24 months against third-party apps that continuously observe the screen, and the source report does not mention it once.
- **The "Apple ships good-enough versions and specialized tools thrive anyway" argument is not researched.** It was sourced to a podcast and two app landing pages, with zero rigor, and it omits the counterexamples that would test it — Sherlock/Watson, Konfabulator→Dashboard, Growl→Notification Center, f.lux→Night Shift, Duet and Air Display→Sidecar. Raycast, Rectangle, and Obsidian *have* thrived alongside native features, so the pattern is real in both directions. **Present it as a two-sided pattern the founder has thought about, never as a base rate.**

### Business model (HIGH confidence — this is load-bearing)

- **No precedent exists for a purely-local, no-cloud Mac subscription.** Every local Mac tool that charges recurring attaches it to cloud, sync, AI, collaboration, or a bundle (Raycast, Bear, Setapp, CleanShot Cloud). Every pure-local no-cloud tool is one-time purchase (Things, Alfred, TablePlus). Warp open-sourced its local client, with the explicit reasoning that value moved higher up the stack.
- **Raycast's monetization timeline is the closest usable precedent** (HIGH — primary, founder interview): launched beta Oct 2020, monetized **May 2023** — roughly three years free — having raised $2.7M seed and a $15M Series A (Nov 2021) in the meantime. The founder described the long free period as "weird, scary." This is the honest shape of the model Evo is proposing, from the company that owns Evo's exact user.
- Conversion benchmarks, MEDIUM confidence, ranges only: free trial without credit card ~8–9%; freemium without time limit ~2–5%.
- **Waitlist-to-paying: ~3% pre-launch, ~10% once live** (MEDIUM). So a 1,000-person waitlist implies roughly 30 paying customers pre-launch. Usable framing, verbatim: *"a waitlist measures topic interest, not purchase intent."*
- **Distribution is forced to be direct** (HIGH). Apps requiring Accessibility permissions cannot ship via the Mac App Store under sandboxing restrictions. This is a constraint to state plainly, not a weakness to hide.

### Market ceiling (HIGH on the tension, and the comparables are more useful than v1.0 claimed)

- Bottom-up sizing under a **prosumer-Mac-utility** framing yields a defensible SOM of only **$1–4M ARR over 3–5 years**, with a realistic category ceiling of $5–40M ARR. This is why the deck is framed as **developer / technical-knowledge-worker tooling with a platform expansion path** instead. The expansion argument must be load-bearing in the narrative, not a closing afterthought.
- **Comparable ceilings — use these exact figures, they were corrected:**
  - **Superhuman: ~$36M ARR (est.), and no paying-user count.** The reports conflict — one says ~70,000 paying (late 2024), another says 20,000+ users at the same ARR, and at ~$30/mo that ARR implies roughly 100k seats. **Do not put a Superhuman user count on a slide in either direction.** The usable point is that it reached this ARR on premium pricing, not utility pricing.
  - **Craft: no user or revenue figure may be used.** The company disclosed nothing and is described in the research as "notoriously private"; the precise-looking figures that circulate (>50,000 paying, ~$7.6M ARR) are third-party estimates that a second report directly contradicts. Known facts only: $8M Series A (Apr 2021), Mac App of the Year 2021, built on design polish and an Apple relationship. **Craft proves nothing either way — say so rather than filling the gap.**
  - **Obsidian: 1.5M+ MAU (2026), $0 raised (refused VC), ~$300–350M valuation (est.), under 10% annual churn, 1,700+ plugins, community-led via Discord (10K in 6 months → 110K+ by 2023).** No ARR figure is established — do not assert one. This is the cleanest local-first comp and the *interesting* fact is that it reached this scale on zero venture money, which cuts both ways and should be handled honestly.
  - **Arc: 1–5M users at $0 revenue.** Most Mac tools that reached real scale were free.
- `[[BLOCKED: share of professional developers using macOS as primary machine — approximately 33%, but this must be verified against the Stack Overflow Developer Survey and JetBrains Developer Ecosystem Survey with the exact figure and year before it goes on a slide.]]`

### Return math (HIGH — verified arithmetic; appendix and investor targeting only, never a slide)

- Pre-seed ownership 8–15%; dilution path to roughly 4–7% at exit. Worked case: 15% initial → ~6% at exit.
- Modal outcome in this neighbourhood is **$0–50M**; $500M+ is the rare ceiling, not the plan.
- At 6% exit ownership, the Arc/Atlassian comp returns **~$29M on the $488.3M filing figure** (or ~$36.6M on the widely-reported ~$610M). **Either way it returns a $25M fund, not a $50M fund.** Consequence: this raise should target small pre-seed funds and solo GPs, not large multistage firms. This is a targeting decision, not a slide.

### What investors do with decks — **attribution matters here, keep these tiers separate**

**Tier 1 — genuine TechCrunch analysis** (`techcrunch.com/2022/09/22/science-of-pitch-decks`), no sample size published, but a real outlet doing real analysis. These are the findings to design around:
- Team slide present in **100%** of decks, successful and failed alike. Financials present in only ~25% overall — **but none of the failed decks had financials.** A financials/use-of-funds slide is mandatory.
- A competition slide was **often missing from unsuccessful decks.** A competition slide is mandatory.
- The **"why are you doing this" purpose slide is a gatekeeper** — disproportionate viewing time, often one sentence on the intro slide, used as a filter before reading further.
- Investors spent **24% less time** on decks in 2022 vs 2021; more time on purpose, less on product and business model at pre-seed.

**Tier 2 — a16z Speedrun, primary.** Usable verbatim: **"Demos over decks."** The demo video is the primary weapon; the deck's job is to get it watched and to frame what it means. (Note the honest tension with Tier 1: "demos over decks" and "investors skim the product slide" point in different directions. The resolution is that the *deck* should not be a product tour, while the *demo* should carry the product — which is exactly the structure specified in Part 7.)

**Tier 3 — SEO/vendor content, blacklisted for citation, usable only as a tiebreaker on craft decisions.** The "10–12 slide average," the "under 2 minutes of viewing time," the "58% completion rate," the "15 seconds per page after the first slide" (misattributed to Papermark), and the "investors spend least time on the product slide" ordering all sit here. **None may appear in the deck, its appendix, or its speaker notes as a cited fact.**

### The pre-seed evidence bar (MEDIUM — qualitative only)

Team, plus insight, plus a working demo, plus **at least one real demand signal** (activation, letter of intent, pilot, or organic growth). "Vision alone" fails screening. Revenue is not required. **Every numeric threshold in the source research was aggregator-laundered and must not be cited as a benchmark.**

---

# PART 7 — SLIDE-BY-SLIDE SPECIFICATION

Build in this order. Each slide states its job, its headline, its content, and its prohibitions.

---

## Slide 1 — Title and purpose

**Job:** State the narrow, concrete promise in one sentence a stranger understands immediately — and state the raise. Per Part 2, this is the highest-leverage slide in the deck: the purpose slide is a gatekeeper and some readers will not go further.

**Headline (use as written):**
> **Evo gets you back into any project — every file, tab, and window — exactly where you left off.**

**Supporting line:** `Local-first. On-device. macOS.`

**The raise, in one quiet line** (repeated in full on Slide 13): `Raising [[NEEDS INPUT: amount]] [[NEEDS INPUT: instrument]]`.

**Also on slide:** Evo wordmark; `[[NEEDS INPUT: founder names and roles]]`; `[[NEEDS INPUT: contact email]]`; date; `[[BLOCKED: one-line beachhead descriptor — see Slide 7; provisionally "for people who work across many projects at once"]]`.

**Prohibited here:** the words "continuity layer," "cognitive," "platform," "category," "AI-powered." Register 2 language does not appear before Slide 7 (Part 3).

---

## Slide 2 — The problem

**Job:** Make the reader feel the reconstruction ritual in under ten seconds. Treat the problem as already understood (Part 3) — this is one slide, not three.

**Headline (use as written):**
> **Your computer forgets your work the moment you stop working.**

**Body — the reconstruction ritual, concrete and physical.** Name what a person actually does when returning to a project after a week away: hunting for the right files, reopening the tabs they had, remembering which branch and which document and what they were about to do next. The point is the *burden of rebuilding*, not elapsed time.

**Draft body copy (use, refine, keep the register):**
> Come back to a project after a week and nothing is where you left it. The files, the tabs, the half-finished thought — you rebuild it all by hand, every time, before you can do any real work.

**Evidence discipline:** No statistics on this slide. Prohibition 2 applies without exception. The research found that **no study exists** on the cost of resuming a project after days or weeks away — the literature on momentary interruption is a different phenomenon and partly cuts the other way. This problem must be *demonstrated and quoted*, never cited.

**Strongest possible version of this slide:** two or three verbatim quotes from real users describing reconstruction in their own words. That is `[[NEEDS INPUT: 2–3 verbatim user quotes about reconstruction pain, with role and permission to quote]]`. Until those exist, use the draft copy above and mark the slide DRAFT.

---

## Slide 3 — What Evo does

**Job:** Land the mechanism in one sentence and one image. Keep it short deliberately — at pre-seed, investor attention on the product slide is falling, and the demo does the persuading.

**Headline (use as written):**
> **One click, and the whole project comes back.**

**Body — three points maximum:**
- Evo watches how you work and learns which files, tabs, and apps belong to which project. No tagging, no manual saving, no setup ritual.
- Pick a project. Evo reopens its working state as you left it.
- Everything stays on your Mac, encrypted. No account required.

**Visual:** one real screenshot of the restoration moment. `[[NEEDS INPUT: product screenshot — the continuation/restore surface]]`

**Prohibited here:** architecture diagrams, internal component names, data-model explanation, feature lists longer than three items.

---

## Slide 4 — Demo

**Job:** Get the demo watched. "Demos over decks" (a16z Speedrun) — for a product whose value is kinesthetic, the video does the persuading.

**Headline (use as written):**
> **Thirty seconds is enough to see it.**

**Content:** a large still frame from the demo, a QR code, and a short link. One line stating what the viewer will watch: closing an entire project, then restoring it in a single action.

`[[NEEDS INPUT: demo video URL and a representative still frame]]`

**Note on length:** the research established no defensible benchmark for demo video length at pre-seed — its own admission is that "no source gives a hard number." Short and focused (1–3 minutes) is the inferred guidance from "demos over decks," not a researched figure. Do not put a duration claim on the slide.

---

## Slide 5 — Why now

**Job:** Establish timing with dated evidence, and pre-empt the "isn't everyone doing this?" reflex by raising it first. Naming competitors before being asked is a credibility move.

**Headline (use as written):**
> **Watching your screen just became table stakes. Understanding your work didn't.**

**Body — three dated points:**
- Between July 29 and August 29, 2026, nine products shipped in this space — including OpenAI's, on August 13. Recording what you did is now a commodity.
- Not one of them classifies your activity into projects and restores a working state. They search and summarize; they don't put you back to work.
- On-device models now make that classification possible locally — and after Recall, doing this on the user's own machine is the only version many people will install.

**On that third point:** it is a claim about trust and installability, not about willingness to pay. **No evidence exists that users pay a premium for local processing** — the research found none. Do not write "users will only accept local" or imply privacy commands a price premium.

**Prohibited:** "no one is doing this" (Prohibition 6). The claim is precise: capture is commoditized, *the loop* is not. Confidence on the negative is MEDIUM — pair it with the `[[UNVERIFIED]]` markers for Hansel and Skill Recorder rather than overstating.

---

## Slide 6 — Why this is hard, and what protects it

**Job:** Answer the defensibility question honestly and specifically, before it is asked as an objection.

**Headline (use as written):**
> **Recording is easy. Knowing what belongs together is the hard part.**

**Body:**
- The engineering difficulty is classification — deciding which of thousands of daily actions belong to the same body of work — and then restoring native application state reliably enough that people trust it.
- Defensibility is threefold: the classification and restoration quality itself, user trust in a product that never sends work off the machine, and the accumulating context that makes leaving costly for each user.

**State the limitation explicitly, in the founder's own voice** — this converts a probe into a display of judgment:
> Local-first means we hold no user data and gain no cross-user data advantage. Our moat is execution quality, trust, and per-user switching cost — not a data asset.

**Prohibited:** Prohibition 3 in full. No "proprietary data," no "data moat," no "network effects," no "flywheel."

---

## Slide 7 — Market

**Job:** Show a defensible bottom-up path to a venture-scale outcome without a fabricated TAM. **This is where Register 2 language may appear for the first time.**

**Headline (use as written):**
> **A wedge with a precise first user, and a path past it.**

**Status: `[[BLOCKED]]`.** Two inputs are missing and this slide cannot be finalized without them. Build the strongest provisional version, mark it clearly, and state what unblocks it.

**Structure to build (show the arithmetic on the slide):**
1. The beachhead: `[[BLOCKED: beachhead definition. "Developers" is too broad — only ~33% of developers are on macOS, and that figure itself needs primary verification. Needs a precise first user: which kind of technical worker, doing what, juggling how many concurrent projects, on a Mac. Provisionally draft as "senior engineers and technical leads on macOS who carry three or more active projects at once," and mark it for confirmation.]]`
2. That population × realistic penetration × price = wedge revenue. Show every step.
3. The expansion path, as the venture argument: teams and shared project context, then cross-device continuity, then Windows.

**Include this framing note as a visible sub-line**, because it is the honest core of the argument: sized as a Mac utility, this category tops out in the single-digit millions of ARR; sized as developer tooling with a team and platform path, it does not. The expansion story is load-bearing, not decorative.

**Prohibited:** any top-down market figure (Prohibition 7); any "if we capture just 1% of…" construction.

**Unblocked by:** the developer-tooling comparables research (scale, funding, and pricing trajectories for Raycast, Warp, Cursor, Linear, Retool, Vercel, Tailscale) plus a founder decision on the beachhead.

---

## Slide 8 — Competition

**Job:** Mandatory — a competition slide was often missing from unsuccessful decks. Show command of a crowded, fast-moving field and make the axis of difference unmistakable.

**Headline (use as written):**
> **Everyone remembers. Nobody puts you back to work.**

**Build a 2×2 positioning matrix.** X-axis: *remembers and searches* → *restores working state*. Y-axis: *cloud* → *on-device*. Evo occupies the restores-plus-on-device quadrant alone. Plot: OpenAI ChatGPT Computer History (cloud processing, search only), Microsoft Recall (Windows, search only), Rewind/Limitless (product wound down — mark it), Hansel `[[UNVERIFIED]]`, Microsoft Skill Recorder `[[UNVERIFIED]]`, Raycast (local, adjacent, and the one to take seriously), tab and session managers (local, shallow), Spotlight and recent-files (local, artifact-level only).

**Handle Rewind in one line, carefully:** the Mac product no longer exists (wound down, reported Dec 2025, after the pivot to a wearable and the Meta acquisition). Prohibition 4 — do not attribute a cause.

**Handle Apple in one line here, and handle it honestly** (Prohibition 9). The defensible line is narrow: Apple has shipped search, recents, and Spotlight actions, but no project-level restoration of application state. **Do not claim Apple won't or can't.** Do not present coexistence as a researched pattern. The fuller two-sided answer lives in Appendix A1, and the honest framing is that this risk can be outrun, not eliminated. Confidence tone: measured, not reassuring.

---

## Slide 9 — Business model

**Job:** State what the money is for.

**Status: `[[BLOCKED]]` — one of three open blockers, and this one needs a founder decision rather than more research.**

**Headline (draft):**
> **Free where it runs on your machine. Paid where it follows you.**

**The constraint driving this slide, stated plainly:** no purely-local, no-cloud Mac subscription has any precedent. Every local Mac tool that charges recurring attaches it to cloud, sync, AI, collaboration, or a bundle; every pure-local no-cloud tool is a one-time purchase. So the recurring price cannot be "the local software" — it has to buy something that genuinely lives beyond the single machine.

**The precedent that makes this credible, and it is worth a line on the slide:** Raycast ran free for about three years — Oct 2020 beta to May 2023 monetization — raising a $2.7M seed and a $15M Series A along the way, and only then attached a paid tier. Same user, same platform, same shape. It is the strongest available answer to "when does this make money," and it is founder-primary sourced.

**Provisional structure to draft** (`[[BLOCKED: founder must choose the paid hook. Encrypted cross-device sync is the strongest candidate and is already on the roadmap; an AI layer is second; a team tier is third and probably later.]]`):
- Free: the full local product on one Mac.
- Paid, `[[NEEDS INPUT: price]]`/month: encrypted continuity across devices — your projects follow you between machines, still end-to-end encrypted.
- Later: team tier for shared project context.

**Include the distribution reality as a sub-line:** direct distribution only, because Accessibility permissions are incompatible with Mac App Store sandboxing. Frame it as a margin advantage — no 15–30% platform cut, direct customer relationship — which is true and is the honest read.

**Prohibited:** a paid tier justified by "advanced features" or "pro features" of the local product. That is precisely the model with no precedent.

---

## Slide 10 — Go to market

**Job:** Show a specific, credible first-thousand-users plan that matches how Mac and developer tools actually spread.

**Headline (use as written):**
> **The people with this problem are already in a few rooms.**

**Body — the sequence, concrete:**
- Curated waitlist with staged invites, so early cohorts get real attention and the product is shaped by people who feel the pain most. Linear's model is the reference: handpick invitees from survey responses, roughly ten invites a week at the start.
- High-touch onboarding for the first cohorts — deliberately unscalable, and the fastest way to learn what makes someone keep it. Granola went from ~150 manually-onboarded users to 5,000 WAU in about five months on this.
- Developer and technical communities where this problem is discussed openly, plus a Show HN launch. Product Hunt as an amplifier later, not a cold start.
- Direct distribution, no app store gatekeeper.

**Include the honest waitlist framing** — stating it before an investor does is a credibility gain. A waitlist measures topic interest, not purchase intent. Pre-launch waitlist-to-paying runs around 3%, about 10% once live (MEDIUM confidence, ranges only). So `[[NEEDS INPUT: waitlist size]]` signups imply a first paying cohort in the low tens, and the number that matters is activation, not signups.

**The line that earns the most credibility here is Rewind's founder on his own 300,000-person waitlist** (primary source, First Round podcast) — quote it and own the point:
> "waitlist that very quickly atrophies. Like, you know, people are excited, they sign up and then they quickly forget." — Dan Siroker

**Prohibited:** paid acquisition as a primary channel (it does not work for indie Mac tools); presenting the waitlist as validated demand.

---

## Slide 11 — Traction and evidence

**Job:** Meet the pre-seed bar: team, insight, working demo, and **at least one real demand signal**.

**Status: `[[NEEDS INPUT]]` — this is the deck's weakest slide and the one that most determines the outcome. Build the scaffold; the founder fills it.**

**Headline (draft):**
> **What we know so far.**

**Scaffold — every row is a real number or it is not on the slide:**
- Working product on macOS, shipped and in daily use — `[[NEEDS INPUT: number of machines running Evo daily, and for how long]]`
- `[[NEEDS INPUT: D7 and D30 retention for that cohort]]`
- `[[NEEDS INPUT: waitlist size, with source attribution — where signups came from — and activation rate]]`
- `[[NEEDS INPUT: 2–3 verbatim user quotes, with roles]]`
- Demo video `[[NEEDS INPUT: views or investor reactions if meaningful]]`

**Instruction to the builder:** if the founder supplies nothing, do **not** pad this slide. Write the qualitative version — working product, what is being measured, and the honest statement that usage evidence is being gathered now — and mark it DRAFT. A thin honest slide survives a partner meeting; an inflated one does not.

**Note for the founder, put this in the speaker notes verbatim:** getting Evo onto 5–15 real machines and collecting retention plus verbatim reconstruction language is the single highest-value action available before this deck is shown. It is the critical path, and no amount of deck craft substitutes for it.

---

## Slide 12 — Team

**Job:** Establish founder-market fit and pre-empt the composition objection. A team slide appeared in 100% of decks studied, successful and failed alike — so its presence earns nothing and its content is everything.

**Headline (draft):**
> **We built this because we needed it.**

**Content:** `[[NEEDS INPUT: both founders' names, roles, and the two or three credentials that matter most — prior work, relevant technical depth, anything that explains why this specific pair can build OS-level Mac software]]`

**Founder-market fit line:** `[[NEEDS INPUT: the honest origin story — what work pattern made this problem acute enough to build for]]`. This matters more than credentials at pre-seed and should be one specific sentence, not a claim of passion.

**Write credentials as execution, not résumé.** The model from the research is Buffer's phrasing — "idea to revenue in 7 weeks," "200 to 55K users" — concrete things done, not titles held.

**Pre-empt in one line:** two technical founders and no dedicated go-to-market hire is a known question for a consumer-facing product. Address it directly — `[[DRAFT — CONFIRM: state who owns distribution and what the first hire is]]` — rather than leaving the partner to raise it.

---

## Slide 13 — The ask and use of funds

**Job:** Mandatory. None of the failed decks in the studied sample had a financials slide. This is also a repeat, not a reveal — the raise was already stated on Slide 1 (Part 5).

**Headline (draft):**
> **Raising `[[NEEDS INPUT: amount]]` to `[[NEEDS INPUT: the one milestone this buys]]`.**

**Content:**
- Amount and instrument — `[[NEEDS INPUT: target raise and structure; post-money SAFE is the current norm, cap to be confirmed]]`
- Allocation across engineering, design, and go-to-market — `[[NEEDS INPUT]]`
- Runway in months, and the specific milestones that unlock a seed round — `[[DRAFT — CONFIRM: propose retention and paid-conversion milestones from the cohort data, since those are what a seed investor will price]]`

**Model for specificity, from the one clean deck in the research** (Momentum, seed): the ask read "$4 million for 18 months of runway" with named milestones. Amount, duration, and what it buys, in one line.

**Instruction:** keep it to a small table or three lines. Precision matters more than detail; a vague ask reads as an unformed plan.

---

# PART 8 — APPENDIX (build all of it; this is where depth belongs)

### Appendix A — The three hardest questions, pre-answered

These are the objections the research rated most serious, and **each rebuttal currently rests on evidence Evo does not yet have.** Draft each answer honestly, mark it, and do not manufacture confidence. Quote each question verbatim as the header:

**A1. "If Apple ships 'search everything I did on my Mac' for free, what remains that Apple cannot or will not copy?"**

Draft: Apple has shipped search, recents, and Spotlight actions — including an 8-hour clipboard history in Tahoe — but no project-level restoration of application state, and no activity-history or screen-observation API exposed to third parties. The defensible answer is that restoration quality and trust are execution problems.

**Handle the coexistence argument as two-sided, because it is** `[[DRAFT — CONFIRM]]`. Specialized tools have thrived alongside good-enough native features (Raycast alongside Spotlight, Rectangle alongside Stage Manager, Obsidian alongside Notes). Apple has also absorbed whole categories (Sherlock/Watson, Konfabulator→Dashboard, Growl→Notification Center, f.lux→Night Shift, Duet and Air Display→Sidecar). **Present both lists.** The founder who names the counterexamples is more credible than the one who only names the wins, and a partner who knows this history will supply the counterexamples anyway.

**Name the signal that cuts against us, before a partner does:** macOS Sequoia introduced recurring Screen Recording re-authorization prompts — weekly in the betas, relaxed after developer backlash before release. Apple is tightening scrutiny on continuous screen observation. `[[UNVERIFIED: the final shipped cadence — do not state a number without checking Apple's own release notes.]]`

Close honestly: this is a risk that can be outrun, not eliminated. Prohibition 9 — no claim that Apple will not or cannot move.

**A2. "Why does this become a venture-scale company rather than a beloved $2–25 million ARR Mac utility with excellent economics but limited expansion?"**

Draft: this is the reason the framing is developer tooling with a team and cross-device path rather than a Mac utility. Be direct that as a single-machine Mac utility, the honest ceiling is single-digit-to-low-double-digit millions of ARR, and that the venture case depends entirely on the expansion to teams and shared project context.

**Use the corrected comparables** (Part 6) and note what they actually show: Superhuman reached roughly $36M ARR (est.) on a premium price point rather than utility pricing — cite no user count, the sources conflict. Obsidian reached 1.5M+ MAU with **zero** venture funding and under 10% annual churn — the cleanest local-first comp, and a genuine double-edged fact, because it shows both the affection this category earns and how little of it needed venture money. Arc reached 1–5M users at $0 revenue. Craft disclosed nothing, so it proves nothing — say so rather than filling the gap.

`[[DRAFT — CONFIRM: this is the deck's central tension and the founder should decide how forward to be about it. Recommendation: be very forward. Partners who spot an unacknowledged ceiling stop trusting the rest of the deck.]]`

**A3. "Why is this not Rewind again — an invasive personal-memory product that earns attention and funding but fails to become a durable, independent business?"**

Draft: the difference to argue is the job, not the technology — Rewind's product asked the user to search their past; Evo's puts them back to work, which is a use that recurs daily rather than occasionally. Note the architectural difference (fully local, no account) as a trust distinction. **Prohibition 4 applies:** do not assert why Rewind's product failed, because that is not publicly established. Argue the positive case for a different job to be done.

One primary-sourced detail is fair to use, because it is Rewind's founder describing his own decisions: a usage paywall that capped rewinds backfired and was switched to unlimited. That is a pricing lesson from the closest comparable, stated without claiming it explains the outcome.

**What the research found actually flips skeptics** — put this in the notes: not rebuttals, but retention, repeat usage, organic distribution, and a surface broader than the initial wedge. Which returns to Slide 11.

### Appendix B — Competitive detail
The full landscape table, dated, with the `[[UNVERIFIED]]` entries marked: product, company, what it does, local or cloud, platform, price, funding, status, last meaningful update. **Do not reproduce "last meaningful update: August 2026" for every row** — in the source research that column records when the search ran, not when the product shipped anything. Leave it blank where unknown.

### Appendix C — Privacy, permissions, and legal posture
Architecture in plain language (on-device, encrypted, no account). The permission requirements and the honest friction: macOS Sequoia introduced recurring Screen Recording re-authorization prompts (weekly in betas, relaxed before release, final cadence unverified). State that no published permission-abandonment data exists for comparable apps — a genuine gap, not a hidden weakness. `[[BLOCKED: four privacy-law questions need counsel before any legal claim appears in writing. Prohibition 8 — do not state that local-only processing removes GDPR obligations.]]`

### Appendix D — Outcomes and comparables
Exit comparables with the honest banding: modal outcome $0–50M, ceiling $500M+. Include the Arc/Atlassian comp at **$488.3M** with a footnote on the ~$610M discrepancy. **Do not put the fund-returner arithmetic in the deck** — at realistic dilution to ~6%, that exit returns roughly $29M, which returns a $25M fund and not a $50M one. That math is for the founder's investor-targeting decisions: it means this raise should target small pre-seed funds and solo GPs rather than large multistage firms.

### Appendix E — Sources
Every factual claim in the deck, with source, date, and confidence. Mark aggregator-sourced figures as estimates. **Any claim drawn from Part 6's Tier 3 list does not appear here, because it does not appear in the deck.**

### Appendix F — The punch list
Every marker from the whole deck, grouped and in slide order, with what closes each one. See Part 4.

---

# PART 9 — DELIVERABLES

1. **A slide-by-slide copy document** (markdown) — every slide's headline, body copy, visual direction, speaker notes, and markers. This is the working document for iteration; produce it first and completely.
2. **The built deck** — 16:9, main deck plus appendix, DRAFT banners rendered on every affected slide, speaker notes populated.
3. **The punch list** — as the final page and as a standalone section, grouped by marker type.
4. **A headline-only summary** — the 13 headlines in order, so the founder can check that the argument stands up read alone.

---

# PART 10 — SELF-CHECK BEFORE DELIVERING

Verify each of these explicitly and report the result. Do not skip this.

1. Zero invented numbers. Every figure traces to Part 6 or carries a marker. **Check every digit in the deck individually.**
2. No minutes-lost statistic anywhere, including speaker notes and appendix.
3. No "data moat," "proprietary data," "network effects," or "flywheel."
4. No claim about why Rewind failed.
5. No category-creation language. Register 2 language appears nowhere before Slide 7.
6. No top-down TAM.
7. No legal claim about GDPR exemption.
8. **No claim that Apple will not or cannot enter, and no coexistence base rate presented as researched** (Prohibition 9).
9. **No figure from Part 6's Tier 3 blacklist appears anywhere**, including as a justification in speaker notes. Specifically: no slide-count statistic, no deck-viewing-time statistic, no completion-rate statistic.
10. **The Arc exit price is $488.3M everywhere it appears, or appears nowhere.** No number is ever attached to Limitless.
11. **Craft carries no ARR or user figure. Superhuman carries ~$36M ARR (est.) and no user count. Obsidian carries no ARR figure.**
12. The five-week competitor count is nine, and the list matches Part 6 exactly — no additions.
13. A financials/use-of-funds slide exists. A competition slide exists. The raise also appears on Slide 1.
14. Main deck is 13 slides or fewer.
15. Headlines read alone as a coherent argument.
16. Every slide containing a marker carries a visible DRAFT banner and an explanatory speaker note. Markers are never combined.
17. The punch list captures every marker in the deck, with nothing missed.

Report any instruction in this document you could not follow, and why, rather than silently working around it.
