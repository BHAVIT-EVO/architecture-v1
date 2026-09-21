# Evo — Perplexity Research Prompt Set
### Everything still worth researching before we draft the pre-seed deck

**Date:** 2026-08-31
**Purpose:** Fill the evidence gaps in `EVO_INVESTOR_RESEARCH_FOUNDATION.md` with citable, dated, third-party sources so every number in the deck can survive a partner asking "where did that come from?"

---

## How to run these

**Order matters.** Prompts 1–4 are load-bearing — the deck cannot be honestly drafted without them. Prompts 5–9 make the deck stronger and pre-arm you for Q&A. Prompts 10–13 are polish and objection-handling. If you only have time for four, run **1, 2, 3, 4**.

**One prompt per conversation.** Don't chain them in a single Perplexity thread — the model starts blending sources and losing citations. Fresh thread each time.

**Use Deep Research / Pro mode** where available. Regular mode will summarize SEO content farms; deep mode actually retrieves.

**Save each report separately** as `research/01-market-sizing.md`, `research/02-context-switching.md`, etc. When you hand them back, tell me which prompt number each corresponds to.

**A note on what these prompts deliberately don't say.** Each prompt describes Evo only as much as needed to get a useful answer — a local-first Mac app that observes your work and restores context. None of them describe the internal architecture, the Place/Moment model, the decision layer, or the execution pipeline. Perplexity is a third-party service and there's no reason to put the actual design into it. Keep it that way if you edit these.

**A note on honesty.** Every prompt tells the model to flag gaps rather than fill them. Read the reports with the same suspicion — if a number appears without a named source and a date, treat it as unusable. We are not putting an unsourced TAM on a slide.

---

# TIER 1 — Load-bearing (run these first)

---

## Prompt 1 — Bottom-up market sizing for a Mac-first prosumer tool

```
You are a market research analyst preparing a defensible bottom-up market size for a venture pitch. Rigor matters more than a big number.

CONTEXT: The company is building a paid macOS desktop application for knowledge workers. It runs locally on the user's Mac, observes what they work on, and lets them return to any project with their files, tabs, and applications restored to where they left off. It is a prosumer subscription product (individual purchase, not enterprise sales) expected to price somewhere in the $10–25/month range. Primary market is the United States, expanding to other English-speaking markets.

RESEARCH THESE QUESTIONS:

1. macOS installed base and user population
   - How many active macOS devices are there globally and in the US, as of the most recent available data? Cite Apple's own disclosures (earnings calls, 10-K) where possible, and note where analyst estimates differ from Apple's numbers.
   - What share of macOS users are using the Mac for professional/work purposes versus personal? Any survey data.
   - macOS market share among specific professional segments: software developers, designers, product managers, writers/researchers, consultants, academics. Stack Overflow Developer Survey, JetBrains Developer Ecosystem Survey, and Design Tools Survey are good sources here — give me the actual percentages and the year.

2. Knowledge worker population
   - How many knowledge workers are there in the US? Give me the definition used by each source, because definitions vary wildly. Prefer BLS occupational data or academic definitions over consulting-firm claims.
   - How many of those work in roles that involve juggling many simultaneous projects/documents/tools?

3. Comparable product scale — what do prosumer Mac tools actually reach?
   For each of these, find the most recent publicly disclosed user count, paying-customer count, and revenue (ARR): Raycast, Setapp, Obsidian, Things, Bear, CleanShot X, Alfred, Superhuman, Cron/Notion Calendar, Craft, Arc browser (before shutdown), Rectangle/Magnet, Bartender.
   - Distinguish free users from paying users clearly.
   - Note which numbers are company-disclosed versus estimated by third parties.
   - This is the single most important part of this research: I need to know the realistic ceiling for a paid Mac utility.

4. Bottom-up construction
   - Using only figures you found above, construct a bottom-up TAM / SAM / SOM. Show the arithmetic explicitly at each step so I can audit it.
   - Then give me the honest pessimistic version too.

OUTPUT FORMAT:
A table for each numbered section with columns: Figure | Source (publication + author if available) | Date of data | Definition/methodology used | Geography | Confidence (high/medium/low).
Then a short prose section on the bottom-up build with the arithmetic shown.
Then a section titled "CONFLICTS AND GAPS" listing every place where sources disagree, and every question above you could not answer with a real source.

SOURCE RULES:
- Prioritize: company disclosures, SEC filings, earnings call transcripts, government statistics, peer-reviewed research, well-known developer/designer surveys, reputable tech press (not press-release reprints).
- Explicitly reject and do not cite: market-research-report listing sites that sell $4,000 PDFs with no methodology (Grand View, Market Research Future, Verified Market Reports, and similar), SEO content farms, AI-generated listicles, and any page that cites a number without naming its origin.
- If a figure only exists in low-quality sources, say so and mark it unusable rather than reporting it.
- Never estimate a number yourself and present it as found. If you're inferring, label it INFERENCE and show your reasoning.
```

---

## Prompt 2 — The evidence base for context loss and work fragmentation

```
You are a research librarian assembling the empirical evidence base for a specific claim. I need real, citable studies — not business-blog folklore.

THE CLAIM I need to support or refute: that knowledge workers lose a significant, measurable amount of productive time to context switching, task resumption, and reconstructing where they left off in their work.

RESEARCH THESE QUESTIONS:

1. Task interruption and resumption
   - Find the primary peer-reviewed research on interruption cost and task resumption lag. I specifically want the original papers by Gloria Mark (UC Irvine) on interruption and task switching, and any work on "resumption lag" / "task resumption failure."
   - For each study: what was actually measured, sample size, methodology, and the actual finding with its real numbers.
   - CRITICAL: The widely-circulated claim that "it takes 23 minutes and 15 seconds to refocus after an interruption" is attributed to Gloria Mark. Trace this to the actual paper. What did the study really measure, what was the sample, and is the popular framing accurate? I need to know whether I can use this number honestly or whether it's been distorted.

2. Context switching cost
   - Peer-reviewed or well-methodologized research on the cognitive and time cost of switching between tasks and between applications/tools.
   - Research on "attention residue" (Sophie Leroy's work) — the actual findings.
   - Any research measuring how long knowledge workers spend per day on tool/window/tab switching. Note methodology: telemetry-based studies are far more credible than self-report surveys, so flag which is which.

3. Tool sprawl and application switching — industry telemetry
   - Studies based on actual observed usage data (not surveys) on the number of applications knowledge workers use per day, and the frequency of switching between them. Sources like Asana's Anatomy of Work, Qatalog/Cornell research, Harvard Business Review's tab-switching study, RescueTime's aggregate data, Microsoft's Work Trend Index.
   - For each: was this telemetry or self-report? Sample size? Who funded it, and do they sell a product that benefits from the finding? Flag vendor-funded research explicitly.

4. Project resumption specifically
   - This is the narrowest and most important question: is there any research on the cost of returning to a project after days or weeks away — as distinct from momentary interruption? Software engineering research on "onboarding cost" of returning to unfamiliar code may be relevant. Also any research on "context reconstruction."
   - If this research does not exist, say so clearly. That's a genuinely useful finding.

5. Counter-evidence
   - Find credible critiques of the productivity-loss-from-interruption literature. Where do researchers disagree? Is the effect overstated? Are there studies showing interruption is neutral or beneficial?

OUTPUT FORMAT:
For each study: Study name | Authors | Year | Publication venue | Sample size and population | Methodology (telemetry / self-report / lab experiment / survey) | The actual finding with real numbers | Who funded it | Whether the popular framing of it is accurate | Direct link.
Then a section "STRONGEST CITABLE CLAIMS" — the 3–5 findings that are best supported and safest to quote publicly.
Then a section "CLAIMS TO AVOID" — widely-repeated numbers in this space that do not survive checking, with the reason.
Then "GAPS" — what I asked that has no real research behind it.

SOURCE RULES:
- Peer-reviewed and primary sources first. Always go to the original paper, never to a blog post summarizing it.
- When a statistic is widely repeated, trace it to origin and tell me if the chain breaks.
- Vendor-funded research is admissible but must be labeled as such every time.
- Do not fabricate citations. If you cannot find a paper matching a description, say so explicitly. Do not construct plausible-sounding titles, authors, or DOIs.
```

> Prompt 2 is the one most likely to change the deck. If the interruption literature turns out to be weaker than commonly assumed, the problem slide must be built on demonstration rather than statistics — which is a better slide anyway.

---

## Prompt 3 — Where the money actually is: pre-seed and seed funding in this sector

```
You are a venture capital analyst building a funding-landscape map. I need named rounds with dates, not general trends.

CONTEXT: The company is raising a pre-seed round in the US (SF and NY primary). It builds a local-first macOS application that observes a user's work and restores project context — sitting somewhere between personal AI, developer/prosumer tooling, and operating-system-level software.

RESEARCH THESE QUESTIONS, covering roughly January 2025 through the present:

1. Comparable financings
   Find every pre-seed and seed round you can in these adjacent categories:
   - Personal AI / AI memory / personal knowledge and context systems
   - Local-first and on-device AI software
   - macOS-native prosumer tools and developer productivity tools
   - AI-native desktop applications and "AI operating system" / OS-layer software
   - Work context, session management, and workspace tooling
   For each: Company | What it actually does in one sentence | Round stage | Amount | Date announced | Lead investor | Other participating investors | Valuation if disclosed | Product maturity at the time of the raise (idea / prototype / private beta / public launch / revenue) | Team size and founder background | Source link.

2. Pre-seed round benchmarks, current
   - What is the current median and range for pre-seed round size and pre-money valuation for US software startups, and specifically for AI-labeled startups? Use sources that publish actual data: Carta, AngelList, Peter Walker's Carta data posts, PitchBook-NVCA Venture Monitor, Crunchbase News quarterly reports.
   - How have these moved over the last 18 months? Is the market tightening or loosening for pre-seed?
   - What dilution is standard at pre-seed right now? Typical SAFE terms — valuation cap ranges, discount, post-money vs pre-money SAFEs, whether MFN is common.

3. The consumer/prosumer software question
   - How much venture funding is going into consumer and prosumer software subscriptions versus B2B/enterprise right now? Is prosumer currently in or out of favor with US seed investors?
   - Find investors or partners who have publicly stated a position on funding consumer/prosumer subscription software in 2025–2026, in either direction. Quote them with links.

4. What died
   - Which companies in these adjacent categories raised and then shut down, pivoted, or were acqui-hired in the last two years? Specifically: Rewind AI / Limitless, Humane, Mem, Arc / The Browser Company, Workona, and any others you find.
   - For each: what they raised, from whom, what happened, and any public post-mortem or founder explanation of why. Link to the founder's own words where they exist.

OUTPUT FORMAT:
Section 1 as a single large sortable table. Sections 2–4 as tables plus brief prose. Every row needs a source link and a date.
End with a section "WHAT THIS MEANS FOR A PRE-SEED RAISE IN THIS SPACE" — 5–8 observations that follow directly from the data you found, clearly labeled as your interpretation rather than fact.

SOURCE RULES:
- Prefer: company announcements, investor blog posts, SEC Form D filings, TechCrunch/Axios Pro Rata/Information/Business Insider reporting, Crunchbase and PitchBook records, Carta's published data.
- Note when a round was reported but never officially confirmed.
- If you cannot verify an amount or a lead investor, leave the cell blank and mark it UNVERIFIED. Do not guess.
- Do not pad the list with companies that merely sound adjacent. Relevance beats volume — but do tell me about anything genuinely surprising you found nearby.
```

---

## Prompt 4 — Named investors and partners, not firms

```
You are a fundraising researcher building a target list. Firm names are useless to me — I need individual humans who write first checks, and evidence of what they care about.

CONTEXT: Pre-seed raise, US (SF and NY primary), for a local-first macOS application that observes a user's work and restores project context. Two technical co-founders. Working product, demo video, and a waitlist; no revenue yet. Categories it touches: personal AI, local-first software, prosumer/developer tooling, OS-layer software, privacy-preserving on-device AI.

RESEARCH THESE QUESTIONS:

1. Individual investors with a documented thesis in this space
   Find specific named partners, angels, and solo GPs who have BOTH (a) written pre-seed or seed checks in the last 24 months into personal AI, local-first software, prosumer/desktop tools, developer tools, or privacy-preserving AI, AND (b) publicly written or spoken about why they find that area interesting.
   For each person: Name | Firm | Role/title | Investments that demonstrate the thesis (with dates) | What they've publicly written or said, with a direct quote and link | Typical check size and stage | Whether they lead rounds or only follow | How they prefer to be approached (cold email, warm intro, application form, office hours, Twitter/X DM) | Link to their public writing.
   Prioritize people who have actually led pre-seed rounds. Someone who only participates in $5M seeds is not useful for a first check.

2. Firms and programs that specialize in first checks
   - US pre-seed-specific funds active right now, and solo-GP funds. For each: check size, stage, sector focus, whether they lead, application process, notable investments.
   - Accelerators and programs relevant to consumer/prosumer AI and developer tools: current terms, batch timing, application deadlines, and notable alumni in adjacent categories. Include Y Combinator, Neo, South Park Commons, AI Grant, Betaworks, and any others you find.

3. What this specific kind of investor says about this specific kind of company
   Find public writing — essays, podcast appearances, tweet threads, investor memos — by named investors on:
   - Local-first and on-device AI as an investment thesis
   - Why personal AI / AI memory products have or haven't worked
   - Their views on Rewind, Microsoft Recall, or the "computer that remembers" category specifically
   - Whether privacy-first architecture is a real moat or just a feature
   - Their views on prosumer subscription software as a venture category
   Give me direct quotes with links and dates. This is more valuable to me than any list — I want to know how these people actually think, in their own words.

4. Anti-targets
   Which investors or firms have publicly expressed skepticism about consumer productivity software, prosumer subscriptions, local-first architecture, or personal AI? I want to know who not to waste a first meeting on, and what their objection is.

OUTPUT FORMAT:
Section 1 and 2 as tables. Section 3 as a quote bank — one quote per entry with speaker, date, source, link, and one line on why it matters. Section 4 as a short table.
End with "TOP 15 INDIVIDUALS TO APPROACH, RANKED" with a one-sentence rationale each, based strictly on evidence you found.

SOURCE RULES:
- Only include people whose investment activity and public position you can both actually verify with links.
- Do NOT include a partner just because their firm invested in something adjacent — I need the individual's involvement or writing.
- Do not fabricate quotes. Every quote must be verbatim from a linked, dated source. If you're paraphrasing, mark it clearly as a paraphrase.
- Flag anyone whose situation has changed (left the firm, fund not currently deploying, firm wound down).
```

---

# TIER 2 — Strengthens the deck and pre-arms Q&A

---

## Prompt 5 — Pricing and monetization benchmarks for prosumer Mac software

```
You are a pricing analyst. I need real, verifiable benchmarks for a paid macOS prosumer subscription product.

CONTEXT: A paid macOS application for knowledge workers, sold directly to individuals, likely $10–25/month. Local-first — the software runs on the user's machine, which means low marginal serving cost but also no usage-based pricing lever.

RESEARCH THESE QUESTIONS:

1. Actual pricing of comparable products
   For each: current price points and tiers, whether subscription or one-time or hybrid, free tier or trial structure, any team/enterprise tier, and pricing history (has it gone up?).
   Products: Raycast, Superhuman, Setapp, Obsidian, Craft, Bear, Things, CleanShot X, Alfred, Notion, Notion Calendar, Linear, Granola, Cursor, Warp, Fig, Rewind (before shutdown), Arc/Dia, TablePlus, Timing, Rize, RescueTime.
   Note especially: which of these successfully charge a *subscription* for a *local* Mac utility, and how they justify it.

2. Conversion and retention benchmarks
   - Free-to-paid conversion rates for prosumer desktop software and freemium SaaS. Give me ranges and where each figure comes from.
   - What conversion rate is considered good versus poor for: free trial with credit card, free trial without credit card, freemium with no time limit, and waitlist-to-activation.
   - Monthly and annual churn benchmarks for prosumer/consumer subscription software. What's considered healthy?
   - Any disclosed retention or churn figures for the products in question 1.

3. Waitlist mechanics and conversion
   - What percentage of waitlist signups typically convert to active users, and then to paying customers, for prosumer software launches? Find real case studies with real numbers — Superhuman, Arc, Raycast, Granola, Linear, Cron are all known for waitlist launches.
   - What waitlist size do investors and operators treat as a meaningful demand signal at pre-seed? Find investors stating a view on this.
   - Known tactics that inflate waitlist numbers without indicating real demand, and how sophisticated investors detect them. I want to know what a skeptical partner will probe.

4. Distribution economics
   - Mac App Store versus direct distribution: revenue split, discoverability, technical restrictions (sandboxing, entitlements), and what successful Mac utilities actually choose and why.
   - Typical CAC for prosumer software acquired via content, community, Product Hunt, and paid channels. Flag how unreliable these figures are.
   - Payment/subscription infrastructure options and their costs: Paddle, Stripe, RevenueCat, Lemon Squeezy — and tax/VAT handling implications for a solo-founded company selling internationally.

OUTPUT FORMAT:
Question 1 as a large table. Questions 2–4 as tables with a Source and Date column on every row plus brief prose interpretation.
End with "PRICING RECOMMENDATION RANGE" — what the evidence supports for a product like this, with the reasoning, clearly labeled as your inference.

SOURCE RULES:
- Pricing must come from the company's current pricing page, with the date you checked it.
- Conversion and churn benchmarks must name their source and sample. Reject any "industry average" with no methodology.
- Company-disclosed metrics (founder interviews, podcasts, blog posts, ARR announcements) are strongly preferred over third-party estimates. Label which is which.
- Where you find no reliable data, say so.
```

---

## Prompt 6 — How Mac prosumer tools actually got their first 1,000 to 10,000 users

```
You are researching go-to-market case studies. I want mechanics and numbers, not marketing platitudes.

CONTEXT: A two-person team launching a paid macOS application for knowledge workers. No marketing budget. They need a credible answer to "how will you acquire users?" and a real plan to get to their first thousand.

RESEARCH THESE QUESTIONS:

1. Case studies with actual mechanics
   For each of these companies, find out how they got their earliest users — the specific channels, sequence, and numbers, ideally from founder interviews, podcasts, blog posts, or Twitter threads:
   Raycast, Superhuman, Arc / The Browser Company, Granola, Obsidian, Craft, Linear, Cron, Warp, Cursor, Rewind, Things, CleanShot X, Rize, Bear.
   For each: What was the launch sequence? Was there a waitlist, and how was it seeded? What role did Product Hunt, Hacker News, Twitter/X, Reddit, Discord/Slack communities, newsletters, or individual creators play? What were the actual numbers at each stage, and how long did it take? What did the founders say did NOT work?

2. Launch platform reality check
   - Product Hunt: what does a top-5 daily launch actually deliver in traffic and signups today, in 2026, versus 2020? Has its impact declined? Find recent data and founder accounts.
   - Hacker News: what does a front-page Show HN deliver? Any documented numbers?
   - Reddit: which subreddits matter for Mac software, and what are their rules about self-promotion?
   - Are there Mac-software-specific channels that matter — newsletters, YouTube channels, review sites, Setapp bundling, MacStories/Daring Fireball style coverage? Who are the actual gatekeepers and how do people reach them?

3. Waitlist and invite mechanics
   - How do successful waitlist launches actually work operationally? Referral-based queue jumping, batched invites, manual onboarding calls, invite codes.
   - Superhuman's manual onboarding approach specifically: what did they do, what did it cost them per user, and did it scale?
   - What converts a waitlist signup into an active user? Case studies with numbers.

4. Developer and prosumer community dynamics
   - How do Mac-focused developer and power-user communities discover new tools? Any survey or research on this.
   - What role does open source, or partial open source, play in adoption of local-first software? Case studies where opening part of the codebase drove trust and adoption.

OUTPUT FORMAT:
Section 1 as one detailed entry per company: Company | Launch date | Channels in sequence | Numbers achieved and timeframe | What founders said worked | What founders said failed | Source links.
Sections 2–4 as tables plus prose.
End with "PLAYBOOK PATTERNS" — what repeats across the successful cases, and "WHAT NO LONGER WORKS" — tactics that worked in 2019–2021 and reportedly don't now.

SOURCE RULES:
- Founder first-person accounts are the gold standard. Prioritize podcast transcripts, founder blog posts, Indie Hackers interviews, Twitter/X threads, Lenny's Newsletter interviews.
- Growth-agency blog posts and generic "10 ways to launch" content: do not cite.
- Give me the actual numbers when they exist, and say "no numbers disclosed" when they don't. Don't fill gaps with plausible-sounding estimates.
```

---

## Prompt 7 — Competitive landscape, current as of today

```
You are a competitive intelligence analyst. I need the state of this market as of right now, including anything that launched or was funded recently.

CONTEXT: The company builds a local-first macOS application that continuously observes a user's work, understands which activities belong to which project, and then restores the files, tabs, and applications needed to resume a project. Everything runs on the user's machine — no cloud required.

RESEARCH THESE QUESTIONS:

1. Direct and near-direct competitors, current status
   For each of the following, tell me the current status as of today, what it actually does, its pricing, platform, whether it's local or cloud, its funding, and any recent news:
   Microsoft Recall / Windows Copilot+ features, Rewind AI / Limitless, Apple's own features (Spotlight, Stage Manager, Continuity, Sequoia/Tahoe additions, Apple Intelligence — and specifically anything Apple has shipped or announced around app state restoration, activity history, or on-device work context), Workona, Mem, Reflect, Heptabase, Granola, Dia / The Browser Company, Raycast (and its AI features), Session/Sessions browser extensions, Tab Session Manager, Shift, Rambox, Sunsama, Motion, Amie, Screenpipe, Open Recall, Windrecorder, and any open-source "screen memory" projects.

2. New entrants — this is the most important part
   What has launched, been funded, or come out of stealth in the last 12 months in: computer activity memory, work context restoration, AI-native desktop environments, local-first personal AI, desktop agents that manage windows and applications, and personal knowledge systems built on observed behavior rather than manual notes?
   Include Y Combinator batch companies, Product Hunt launches, and things circulating on Hacker News. I especially want anything I might not have heard of.

3. Platform risk — Apple and Microsoft
   - What has Apple shipped or publicly signaled around on-device AI, activity history, app state restoration, and cross-device work continuity in the last 18 months? Include WWDC announcements and developer documentation changes.
   - Has Apple changed anything about Accessibility API access, Screen Recording permissions, or app sandboxing that would affect a third-party app that observes user activity? Any signals about future restrictions?
   - Current status of Microsoft Recall: is it on by default, what were the security findings, what did researchers demonstrate, and what is public sentiment now?
   - Assess the "Apple could just build this" risk with actual evidence about Apple's historical behavior toward this class of utility. Which Mac utilities has Apple effectively absorbed, and which have survived alongside OS features?

4. Post-mortems
   - Deep detail on why Rewind AI pivoted away from the Mac product to the Limitless pendant. Find founder statements. What did they learn about demand, retention, and privacy objections? Any disclosed retention or usage numbers.
   - Public reaction to Microsoft Recall — security researcher findings, journalist coverage, regulator response. What specifically did people object to?
   - The Browser Company's shift from Arc to Dia and the acquisition: founder explanations of what didn't work about Arc.

OUTPUT FORMAT:
Section 1 as a comparison table: Product | Company | What it does | Local or cloud | Platform | Price | Funding raised | Current status | Last meaningful update | Source.
Section 2 as a list with a one-paragraph description each, flagging which are genuinely close to the described product.
Sections 3–4 as prose with citations.
End with "WHITE SPACE ASSESSMENT" — based only on what you found, what does no existing product do? And "MOST DANGEROUS COMPETITOR" with reasoning.

SOURCE RULES:
- Product capabilities must come from the product's own site or documentation, or hands-on reviews. Not from competitors' comparison pages.
- Date-stamp everything. This category moves fast and stale information is worse than none.
- If a product appears abandoned, check its changelog, release notes, and social accounts before saying so, and give me the evidence.
```

---

## Prompt 8 — Permission friction, privacy, and legal reality for observation software on macOS

```
You are a technical and regulatory analyst. I need to understand the real-world friction and legal exposure for macOS software that observes user activity.

CONTEXT: A macOS application that requires elevated permissions — Accessibility API and possibly Screen Recording — to observe which applications and documents the user is working with, and to restore them later. All data stays on the user's device, encrypted locally. No cloud upload, no account required.

RESEARCH THESE QUESTIONS:

1. Permission friction, quantified
   - What is known about user drop-off when macOS apps request Accessibility or Screen Recording permission? Any data, developer accounts, or case studies with numbers.
   - How do successful apps that need these permissions handle onboarding? Look at how Raycast, CleanShot, Rectangle, Bartender, Loom, Krisp, and Rewind ask for permissions. What are the patterns in the ones that convert well?
   - Has Apple made these permissions harder to grant over recent macOS versions? Document the changes version by version.
   - Are there known Apple review or notarization issues for apps requesting these permissions? What gets rejected?

2. Distribution constraints
   - Can an app requiring Accessibility and Screen Recording permissions ship on the Mac App Store? What are the sandboxing restrictions and how do they conflict with these capabilities?
   - What do apps in this category actually do — App Store, direct with notarization, or both? Trade-offs.
   - What are the current requirements and costs for notarization, code signing, and Developer Program membership?

3. Privacy law and compliance for local-only processing
   - Under GDPR, does software that processes personal data entirely locally on a user's own device, with no transmission to the vendor, create controller obligations for the vendor? What is the actual legal analysis and where is it uncertain?
   - Same question under CCPA/CPRA and other US state privacy laws.
   - What happens when the observed data incidentally captures third parties — for example a colleague's name in a document, or content of a video call? Any guidance or case law.
   - Are there specific rules about capturing screen content in regulated contexts: healthcare (HIPAA), legal privilege, financial services, education (FERPA)?
   - Do any jurisdictions restrict or require disclosure for software that records or monitors computer activity, even when self-installed? Wiretapping and two-party consent statutes may be relevant.

4. Enterprise and procurement blockers
   - If individual employees install this on work Macs, what commonly blocks it? MDM restrictions, endpoint security software, IT allowlists, DLP policies.
   - How do prosumer tools handle the bottom-up adoption path onto managed devices? Case studies.
   - What security review requirements typically appear once a tool spreads inside a company — SOC 2, vendor security questionnaires, penetration tests — and at what point does that become unavoidable?

5. Positioning lessons
   - How do privacy-focused local-first products communicate their privacy posture in a way users actually believe? Look at Signal, Proton, Obsidian, DuckDuckGo, Apple's own privacy marketing, Tailscale.
   - What specific language, proof mechanisms, and third-party validation (audits, open source, verifiable builds) do users and reviewers respond to?
   - Conversely: what privacy claims have blown up on companies when contradicted? Case studies of privacy-positioning failures.

OUTPUT FORMAT:
Structured sections with tables where comparative, prose where analytical. Every legal claim needs a source, and where the law is genuinely unsettled, say so plainly rather than picking an answer.
End with "TOP RISKS RANKED BY SEVERITY AND LIKELIHOOD" and "MITIGATIONS THAT COMPARABLE COMPANIES ACTUALLY USE".

SOURCE RULES:
- Apple technical claims: cite Apple developer documentation, release notes, or WWDC sessions.
- Legal claims: cite regulator guidance, statutes, law firm analyses, or academic work. Note jurisdiction every time.
- This is not legal advice and I understand that — but I need to know where the real questions are so I can ask a lawyer the right ones. Flag which items genuinely need counsel.
- Do not state a confident legal conclusion where practitioners disagree.
```

---

## Prompt 9 — What investors expect to see at pre-seed, in evidence terms

```
You are a fundraising analyst. I need to know the current evidence bar for a pre-seed round, stated as concretely as the data allows.

CONTEXT: Two technical co-founders, US pre-seed raise (SF/NY). A working macOS product, a demo video, and a waitlist in progress. No revenue. Category is prosumer/personal AI software.

RESEARCH THESE QUESTIONS:

1. The evidence bar
   - What do pre-seed investors in 2025–2026 actually require to write a first check into a pre-product-revenue company? Find named investors stating their criteria explicitly, with quotes and links.
   - How has this bar changed over the last two years? Is "team plus vision" still fundable, or has the market moved to requiring usage traction? Find data or investor statements on both sides.
   - Specifically for AI-labeled companies: is there still an AI premium, or has scrutiny increased? Find data on round sizes and valuations for AI versus non-AI at pre-seed.

2. Traction metrics that matter pre-revenue
   - Which non-revenue metrics do pre-seed investors treat as real signal? Waitlist size, activation rate, D1/D7/D30 retention, DAU/WAU ratio, session frequency, organic referral, NPS, qualitative user love.
   - What thresholds are considered strong for each, for a prosumer product? I want numbers with attribution.
   - Retention benchmarks specifically: what does good D30 retention look like for prosumer desktop software? What do investors consider a "product people actually use"?
   - Which metrics do experienced investors consider vanity metrics, and what do they ask instead?

3. Team evaluation at pre-seed
   - How do pre-seed investors evaluate a two-technical-co-founder team with no prior exit? What signals do they look for?
   - How is founder-market fit actually assessed, and how do founders best demonstrate it? Find investors describing what convinces them.
   - What are documented red flags in founding teams at pre-seed? Equity splits, part-time founders, unclear roles, missing commercial capability.
   - How do investors view a team with no go-to-market or design co-founder building a consumer-facing product?

4. Process and materials
   - What is the current expected materials package at pre-seed: deck length, whether a data room is expected, whether a financial model is expected and at what depth, memo versus deck.
   - Typical process length and number of meetings from first contact to closed pre-seed round.
   - What questions do pre-seed investors most commonly ask in a first meeting? Find actual lists from investors.
   - What are the most common reasons pre-seed investors pass? Find investors describing their own pass reasons.

5. Demo-driven pitches
   - For products that are hard to explain in words, how have founders successfully used live demos or demo videos in fundraising? Case studies where the demo was the deciding factor.
   - What makes a demo work versus fall flat in an investor meeting? Find investor commentary on demos specifically.
   - Optimal demo video length and structure for investor consumption. Any evidence on this.

OUTPUT FORMAT:
Tables with a Source and Date column for benchmark figures. Quote bank format for investor statements — speaker, role, firm, date, verbatim quote, link.
End with "THE BAR, STATED PLAINLY" — your synthesis of what this specific company would need to show, labeled as inference, with the evidence each point rests on.

SOURCE RULES:
- Investor statements must be verbatim and linked. Fabricated or paraphrased-as-quoted material is worse than useless here.
- Benchmark numbers need a named source with a sample. Reject unattributed "industry standard" claims.
- Where investors contradict each other — and they will — show the disagreement rather than averaging it away.
```

---

# TIER 3 — Polish, language, and objection handling

---

## Prompt 10 — Retrievable pre-seed and seed decks, analyzed slide by slide

```
You are a research analyst studying pitch deck artifacts. I need actual decks I can look at, not advice about decks.

TASK: Find publicly available pitch decks from companies that raised pre-seed or seed rounds, prioritizing these categories: prosumer and consumer software subscriptions, developer tools, personal AI and AI memory products, local-first software, macOS/desktop applications, and any company introducing a genuinely new product category rather than competing in an established one.

FOR EACH DECK YOU FIND:
- Company, round stage, amount raised, date, investors
- Where the deck is publicly available, with a direct link
- Whether it is the real deck used to raise, or a later reconstruction/redesign (this matters enormously — many "famous decks" circulating online are cleaned-up versions)
- Slide-by-slide breakdown: what is on each slide, in order, with the actual headline text where you can read it
- Total slide count
- Approximate word count on the densest slide
- How they handled: the opening slide, the problem, the product explanation, market sizing, competition, traction, team, and the ask
- Whether product screenshots, diagrams, or abstract graphics dominate
- Anything structurally unusual

ALSO RESEARCH:
1. Which specific decks are actually publicly available for study? Good sources: the company's own blog, founder posts, Slidebean's collection, Sequoia's published decks, Pitch's gallery, DocSend's research posts, Airbnb/Uber/Front/Buffer style published decks, and founders who have blogged "here's the deck that raised our seed."
2. Any research or data on deck structure and investor behavior — DocSend's Startup Index work on time spent per slide and slide order, and any similar studies. What does the data say about which slides get attention and what correlates with successful raises?
3. Founders who have published a written account of their pre-seed raise alongside their deck — the narrative of what worked and what they'd change.

OUTPUT FORMAT:
One structured entry per deck. Then a comparative table: Company | Stage | Slide count | Slide order as a compact sequence | Notable structural choice.
Then "PATTERNS ACROSS DECKS" — what recurs in the successful ones.
Then "WHAT THE DATA SAYS" — findings from DocSend-style research with citations.
Then "DECKS I COULD NOT FIND" — be explicit about what isn't publicly available. I would rather have five real decks than twenty summaries.

SOURCE RULES:
- Only include decks you can actually link to. If you cannot find the artifact, do not describe it from memory or reputation.
- Clearly separate "real deck used to raise" from "redesigned version" from "template inspired by."
- Do not substitute pitch-deck-advice blog content for actual decks. I want primary artifacts.
```

---

## Prompt 11 — How new categories were introduced in the founders' own earliest words

```
You are a language and positioning researcher. I want to see how companies that created new categories described themselves at the very beginning — before the category existed and before they had the vocabulary they later became known for.

TASK: For each company below, find the EARLIEST publicly available self-description — from their first website (use the Internet Archive Wayback Machine), their launch announcement, their first funding announcement, their Show HN or Product Hunt launch, and their founders' earliest public explanations. Then compare it to how they describe themselves today.

Companies: Figma, Notion, Linear, Superhuman, Airtable, Retool, Vercel, Stripe, Slack, Roam Research, Obsidian, Arc / The Browser Company, Raycast, Granola, Cursor, Replit, Tailscale, Rewind, Descript, Loom.

FOR EACH:
- Earliest one-line description, verbatim, with the date and the archived URL
- Their launch announcement headline, verbatim
- The single sentence they used to explain the product when nobody had a mental model for it
- Whether they named a new category, positioned against an existing one, or used an analogy ("X for Y")
- Their current one-line description, for contrast
- How long it took them to arrive at the language they became known for
- Any founder account of the positioning struggle — did they publicly discuss having trouble explaining the product?

ALSO RESEARCH:
1. Slack's positioning history specifically — Stewart Butterfield's internal memo "We Don't Sell Saddles Here" is essential. What did it argue, and how did it shape their language?
2. Are there documented cases where a company's initial category framing actively hurt them, and changing it unlocked growth? Case studies with evidence.
3. Any research or investor writing on category creation versus category entry — is creating a new category actually advantageous, or is it a costly mistake most of the time? Find both sides, including skeptics of "category design" as a strategy.
4. The specific problem of explaining infrastructure or platform products that don't map to an existing user behavior. How have founders solved this?

OUTPUT FORMAT:
One entry per company with verbatim quotes, dates, and archive links. Then a comparative table: Company | Earliest self-description | Current self-description | Strategy used (new category / analogy / against-incumbent / job-to-be-done).
Then "WHAT WORKED" — patterns in the descriptions that succeeded at making an unfamiliar product graspable.
Then "THE CASE AGAINST CATEGORY CREATION" — the strongest evidence-based argument that inventing a new category is a mistake.

SOURCE RULES:
- Use the Wayback Machine and give me the archived URL with its snapshot date for every early description.
- Quotes must be verbatim. This research is worthless if the wording is approximated — the exact words are the point.
- If you cannot retrieve a company's earliest description, say so rather than reconstructing it.
```

---

## Prompt 12 — The objections: why investors pass on companies like this

```
You are preparing an adversarial briefing. Your job is to find the strongest real arguments against funding this company, in investors' own words.

CONTEXT: A pre-seed company building a local-first macOS application for knowledge workers that observes their work and restores project context. Two technical co-founders, working product, no revenue, prosumer subscription model, privacy-preserving on-device architecture, Mac-only initially.

RESEARCH THESE QUESTIONS:

1. Stated pass reasons in this category
   Find investors publicly explaining why they don't invest in, or are skeptical of:
   - Consumer and prosumer productivity software
   - Single-platform (Mac-only) software businesses
   - Local-first architecture as a business model, including concerns about no network effects and no data moat
   - Personal AI and AI memory products, especially post-Rewind
   - Products requiring invasive permissions
   - Subscription utilities that an OS vendor could absorb
   Give me verbatim quotes with links and dates.

2. The specific structural objections, researched
   For each of these, find the actual evidence and the counter-evidence:
   - "Apple will just build this." Historical base rate: how often has Apple absorbed a category-defining Mac utility, and how often have third parties survived? Concrete examples both ways.
   - "Local-first means no data moat and no network effects." How have local-first companies (Obsidian, Tailscale, Signal, Ollama, 1Password before cloud) built defensibility? Any that achieved venture-scale outcomes?
   - "Prosumer subscriptions don't reach venture scale." What is the largest outcome achieved by a prosumer desktop subscription business? Find real examples and revenue figures.
   - "Users won't grant those permissions." Evidence for and against.
   - "This is a feature, not a company." Find investor writing on how they distinguish the two, and cases where the feature critique was wrong.
   - "Rewind already tried this and failed." What is the strongest version of this objection, and what is the strongest evidence-based rebuttal?

3. Base rates
   - What proportion of pre-seed companies raise a subsequent seed round? Failure and graduation rates by stage. Cite Carta or PitchBook data.
   - Any data on outcomes specifically for consumer/prosumer software startups versus B2B at the earliest stages.

4. What convinced skeptics
   - Find cases where an investor publicly described being initially skeptical of a company in an adjacent category and then changing their mind. What changed it? These stories are the most useful thing in this entire prompt.

OUTPUT FORMAT:
Section 1 as a quote bank: Objection | Who said it | Role and firm | Date | Verbatim quote | Link.
Section 2 as one entry per objection: The objection stated in its strongest form | Evidence supporting it | Evidence against it | How comparable companies answered it | Assessment of whether it's fatal, serious, or manageable.
Sections 3–4 as tables and prose.
End with "THE THREE HARDEST QUESTIONS" — the objections with the least available rebuttal, stated bluntly.

SOURCE RULES:
- Be genuinely adversarial. Do not soften objections or pre-emptively rebut them in the same breath. I need to know what I'm walking into, and a flattering report is a liability.
- Every quote verbatim and linked.
- Where I've asked for a rebuttal, only give one if real evidence supports it. "No good rebuttal found" is a valid and valuable answer.
```

---

## Prompt 13 — Exit landscape and the return case

```
You are an M&A and outcomes analyst. A pre-seed investor needs to believe there is a path to a fund-returning outcome. I need the evidence for what those outcomes look like in this neighborhood.

CONTEXT: A local-first macOS application for knowledge workers in the personal AI / work context / prosumer productivity space, currently at pre-seed.

RESEARCH THESE QUESTIONS:

1. Acquisition comps
   Find acquisitions in these categories from roughly 2019 to present: productivity and prosumer software, browsers, personal AI and AI memory, developer tools, note-taking and knowledge management, desktop utilities, macOS software.
   For each: Acquirer | Target | Price | Date | Target's revenue and users at acquisition if known | Target's total funding raised | Multiple paid if calculable | Strategic rationale | Source.
   Include at minimum: Atlassian/The Browser Company, Meta/Limitless (Rewind), HP/Humane, and find everything else you can.
   I especially want the ones where a small team with modest revenue achieved a meaningful outcome.

2. Who acquires in this space
   - Which acquirers have been most active in productivity, personal AI, and desktop software? What is each one's pattern — do they buy revenue, technology, or teams?
   - Are there acquirers with a specific stated interest in on-device AI or privacy-preserving technology?
   - What do acquihires versus real acquisitions typically look like in price and structure in this category?

3. Independent outcomes
   - Which prosumer software companies have reached significant scale without acquisition? Revenue figures where disclosed. Obsidian, Sublime Text, Panic, Rogue Amoeba, Things/Cultured Code, Setapp/MacPaw, JetBrains are starting points.
   - Are there bootstrapped or small-team Mac software companies with substantial revenue? This matters as a downside-protection argument.
   - Any IPOs or large independent outcomes in prosumer productivity.

4. The venture math
   - What outcome size does a pre-seed fund need for a single investment to return the fund? Explain the arithmetic for typical pre-seed fund sizes and ownership targets.
   - What ownership does a pre-seed investor typically need at entry, and what does dilution through later rounds do to it?
   - Find investor writing on how they assess whether a prosumer software company can reach venture scale.

OUTPUT FORMAT:
Section 1 as a comprehensive table sorted by price descending. Sections 2–3 as tables. Section 4 as prose with worked arithmetic.
End with "THE HONEST RETURN CASE" — based only on the comps found, what outcome range is realistic for a company like this, and what would have to be true for the high end. Label clearly as inference.

SOURCE RULES:
- Acquisition prices must be sourced. Distinguish confirmed prices from reported/rumored ones, and say when a price was never disclosed.
- Revenue figures at acquisition: company-disclosed or credible reporting only, never estimated.
- Do not inflate the picture. If the comps in this category are mostly small, that is the finding and I need to know it.
```

---

## What I deliberately did NOT ask you to research

Worth knowing why these are absent, so you don't spend Perplexity credits on them:

**Generic pitch deck advice.** "What are the 10 slides in a pitch deck" content is infinite, contradictory, and already covered in the Deck Constitution section of the research foundation. Prompt 10 asks for actual deck artifacts instead, which is the version of this question that has real answers.

**Top-down TAM from market research reports.** The "$XXX billion productivity software market growing at 14% CAGR" figures come from report vendors with no disclosed methodology, and experienced pre-seed investors read them as a signal you haven't thought about your market. Prompt 1 builds bottom-up from real comparables instead. If a slide needs a market number, it should be one we can derive and defend.

**Evo's own technical validation.** Whether the architecture works isn't a research question — it's an engineering question already answered by the working product, and the demo video will carry it better than any citation.

**Competitor feature checklists.** A feature-by-feature comparison grid invites the "this is a feature, not a company" read. Prompt 7 asks about category position and white space instead.

**Design and visual research.** Already covered in the research foundation, and honestly the deck's visual language should come from Evo's own product surface rather than from imitating other decks.

---

## After the reports come back

Give me the reports and tell me which prompt each answers. Then I'll:

1. Reconcile them against `EVO_INVESTOR_RESEARCH_FOUNDATION.md` and flag everywhere the new evidence contradicts what's currently written there — including anything that means we should abandon the recommended positioning.
2. Build the slide-by-slide deck outline with every claim traced to a specific source in your reports, so you can see exactly what each slide rests on.
3. Draft the deck copy.
4. Flag any slide where the evidence isn't strong enough to make the claim, so we either soften it or cut it rather than putting something indefensible in front of a partner.

Partial is fine — if you get through Tier 1 only, that's enough to start the outline.

---

# SECOND BATCH — run after the first 13 (added 2026-09-01)

The first 13 reports are digested and reconciled (`EVO_EVIDENCE_RECONCILIATION.md`). These two are follow-ups, not exploration. **Prompt 14** substantiates the developer-tooling+platform framing we've now committed to, and — critically — establishes how comparable tools monetize, because the pricing research found *no precedent for a purely-local, no-cloud subscription*. **Prompt 15** is a narrow verification pass: five specific claims that the first batch left laundered through aggregators, each of which will otherwise go onto a slide. Same rules as before — one prompt per fresh thread, Deep Research mode, demand a source and a date for every figure, and no description of Evo's internals. Save the results as `research/14-devtool-comps.md` and `research/15-verification.md` and tell me which is which.

---

## Prompt 14 — Developer-tooling comparables: scale, funding, pricing, and the local-vs-cloud split

```
You are a venture and product analyst building a comparables file on modern developer-tooling companies. I need verifiable scale, funding trajectory, and — most important — exactly what each company charges for and whether the paid value lives locally or in the cloud.

CONTEXT: The company builds a local-first macOS application that observes a user's work and restores project context. It is positioning itself as developer / technical-knowledge-worker tooling that expands into a platform, and it needs to understand how the successful developer tools of the last decade actually scaled and monetized — especially the ones that began as a local or single-user tool and later attached a paid cloud, AI, or team layer.

COMPANIES TO COVER: Raycast, Warp, Cursor (Anysphere), Linear, Retool, Vercel, Tailscale. Add any other developer tool you consider a strong structural comparable, clearly labeled as your addition.

RESEARCH THESE QUESTIONS:

1. Scale and trajectory
   For each company: the most recent dated figures you can source for users, paying customers, and ARR/revenue; year founded; and the publicly documented growth milestones ("X users by year N"). Distinguish free users from paying customers, and company-disclosed numbers from third-party estimates.

2. Funding history
   For each: every disclosed round — stage, amount, date, lead investor, post-money valuation if public — from the first institutional check to the most recent. I want the trajectory, not just the latest headline. Source every row.

3. Pricing model and the local-vs-cloud split — THIS IS THE MOST IMPORTANT SECTION
   For each company:
   - Is the core product installed and run locally, in the cloud, or hybrid? Be specific about which parts run where.
   - What exactly does the paid tier charge for? Isolate whether the recurring price buys local functionality, or a cloud / sync / AI / collaboration / team layer sitting on top of a free or local base.
   - When they introduced paid tiers, what was the monetization hook — team/collaboration, cloud sync, AI features, usage/compute, or enterprise admin/security?
   - For any tool that began as a purely local single-user product: how did it justify a subscription, and what did it have to add to do so?
   I am specifically testing the hypothesis that no successful developer tool charges a recurring fee for purely local, single-user, no-cloud functionality. Confirm or refute it with evidence, per company.

4. Narrow-to-platform arc
   For each: what was the narrow initial product and its earliest one-line self-description, and how did it expand into a broader platform? Roughly how long did that take? Cite the earliest description with a date where you can.

5. The macOS / developer beachhead
   - What share of professional software developers use macOS as their primary machine? Cite the Stack Overflow Developer Survey and the JetBrains Developer Ecosystem Survey with the year and the exact percentage.
   - For the local/desktop tools above (Raycast, Warp, Tailscale, Cursor), what is known about their platform split and how they defined their earliest target user?

OUTPUT FORMAT:
Sections 1 and 2 as tables, one row per company, every cell with a source and a date. Section 3 as one structured entry per company answering each sub-question explicitly, then a summary table: Company | Local / Cloud / Hybrid | What the paid tier actually buys | Monetization hook | Charges for purely-local single-user value? (Y / N / partial). Sections 4–5 as tables plus brief prose.
End with "WHAT THIS MEANS FOR A LOCAL-FIRST TOOL'S BUSINESS MODEL" — 5–8 observations that follow strictly from the evidence, labeled as your interpretation, on how a local-first product should structure a paid tier.

SOURCE RULES:
- Scale and revenue: company disclosures, founder interviews, earnings/press, or clearly-labeled third-party estimates. Never present an estimate as a disclosed figure.
- Funding: SEC Form D, company/investor announcements, Crunchbase/PitchBook records. Mark anything reported-but-unconfirmed as UNVERIFIED.
- Pricing: the company's current pricing page, with the date you checked it. Pricing history from archived pages or dated announcements.
- Reject SEO content farms, AI listicles, and $4,000-report vendors. If a figure only exists in low-quality sources, mark it unusable rather than reporting it.
- Do not fabricate. Where you cannot verify, leave the cell blank and say so.
```

---

## Prompt 15 — Primary-source verification pass on five load-bearing claims

```
You are a fact-checker. Your only job is to trace five specific claims to primary sources, confirm or correct them, and refuse to guess. Each of these will otherwise go into an investor document, so a wrong number is worse than a blank.

For EACH item below I need: the primary source (the original filing, press release, pricing page, archived page, or the person's own words — not an aggregator or SEO summary), the exact figure or wording as that source states it, the date, a direct link, and an explicit confidence level. If the primary source cannot be found, say so plainly and mark the claim UNVERIFIED. Do not substitute a secondary summary for a primary source.

1. Pre-seed dilution and round benchmarks (Carta primary data)
   - Current median and range for US pre-seed round size and pre-money valuation, and standard founder dilution at pre-seed. Figures must be traceable to Carta's own published data (carta.com/data, Carta's State of Private Markets, or Peter Walker's Carta posts that show the underlying chart) — NOT to blogs citing Carta. Give the report/post title, date, and link.
   - Standard SAFE terms at pre-seed right now: post-money vs pre-money, typical valuation-cap ranges, discount, MFN prevalence. Cite Y Combinator's SAFE documentation or a Carta/legal primary.

2. Pre-money valuation benchmarks (PitchBook primary)
   - US pre-seed/seed pre-money valuation medians and how they've moved over the last 18 months, traceable to the PitchBook-NVCA Venture Monitor or an official PitchBook release. Give the specific report edition and date. If only a paywalled or secondary version exists, say so.

3. The Atlassian / The Browser Company acquisition price — resolve a discrepancy
   - Two figures circulate: ~$610M (widely reported at announcement, Sep 2025) and $488.3M (reportedly Atlassian's filing at close, Oct 2025). Find Atlassian's official disclosure — press release, 8-K / 10-Q, or earnings material — and tell me the exact figure it states, the date, and the context (gross vs. net of cash acquired, cash vs. total consideration). I need to know which number to use and why they differ. Also confirm whether the same ~$610M figure has been erroneously attached to the Meta / Limitless (Rewind) deal in secondary coverage.

4. Rewind → Limitless → Meta, in the founder's own words
   - Find Dan Siroker's (and/or his co-founder's) own public statements explaining the pivot from Rewind (the Mac product) to the Limitless pendant, and any statement around the Meta acquisition and the Mac product's shutdown (reported Dec 2025). I want verbatim quotes with source and date — blog posts, interviews, podcasts, X posts, official announcements. If no first-person explanation exists for a given step, say so; do not infer motives from press coverage.

5. Two competitor capabilities — do they restore state, or only remember?
   - Hansel (macOS, launched on Product Hunt ~Aug 14 2026; positioned "remember everything you've worked on, find past context, and answer questions about your workday; data stays encrypted on your Mac"): does it actually restore a prior working state — reopening the files, tabs, and applications for a project — or does it only search and answer questions about past activity? Use the product's own site, docs, changelog, or a hands-on review.
   - Microsoft Skill Recorder (open-source, MIT, launched ~Jul 29 2026; records screen sessions and reconstructs them as ordered steps / a SKILL.md for Copilot): does its reconstruction restore a working project state on the user's machine, or does it only produce an agent script/skill from the recording? Read the actual GitHub repository and its documentation.
   For each, state plainly which of "remembers/searches," "produces a script," or "restores working state" it does, with the evidence.

OUTPUT FORMAT:
One structured entry per numbered item: Claim | Primary source (title, publisher, date) | Exact figure/wording as stated | Direct link | Confidence (high / medium / low / UNVERIFIED) | Notes on any discrepancy. No prose synthesis needed — accuracy and traceability are the entire deliverable.

SOURCE RULES:
- Primary sources only for the load-bearing figure in each item: the original filing, the company's own page, the person's own words, or the actual code repository. Aggregators (getlatka, owler, privco, tracxn), SEO content sites, and "X cites Carta"-style secondhand posts are not acceptable as the primary citation, though you may note them as corroboration.
- Quotes must be verbatim, dated, and linked. Do not paraphrase and present as a quote.
- "I could not verify this from a primary source" is the correct, valuable answer when true. Never fill a gap with a plausible number or a reconstructed quote.
```
