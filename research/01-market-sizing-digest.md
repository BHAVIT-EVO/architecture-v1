# Digest 01 — Market Sizing (Bottom-Up TAM for a Paid Mac Prosumer Tool)

**Source document:** `uploads/You are a market research analyst preparing a defe.pdf`
**Tool:** Perplexity (deep research mode)
**Total pages:** 15. **Body:** pp. 1–10 (prompt on pp. 1–2; four data sections + bottom-up build + CONFLICTS AND GAPS + "Bottom line for your pitch" ending on p. 10). **Sources:** footnote URL list begins mid-p. 10 and runs to p. 15 — **167 numbered sources**, roughly 5.5 pages of pure URLs.
**Date of digest:** 2026-08-31. All figures below are as the report states them; no figures added.

---

## Key Findings (what the report actually FOUND)

1. **There is no defensible large market number here.** The report's own bottom line is that the realistic ceiling for a paid prosumer Mac utility at $10–25/mo is **$5M–$40M ARR**, and the defensible SOM for a new entrant is **$1M–$4M ARR in 3–5 years (6,000–24,000 paying users at $180/yr)**. The pessimistic build lands at **$1.3M TAM revenue / $650k SAM / $65k SOM**, which the report itself annotates "not viable as standalone business."
2. **Apple does not disclose the number the whole build rests on.** Apple discloses 2.5B total active devices; it does **not** break out Mac. The ~200M active Macs figure is a third-party aggregation. The US Mac installed base (80–100M) is explicitly labelled **INFERENCE** by the report — "an assumption, not a sourced figure."
3. **The BLS knowledge-worker figure is decorative.** 70.7M (BLS CPS 2024) is stated in Step 1 and then **never used in any subsequent arithmetic**. The entire funnel runs off the inferred Mac installed base. The one high-confidence government number in the document does no work.
4. **Superhuman is the ceiling case and it is small:** ~70,000 paying customers, $36.5M ARR (late 2024). Company-disclosed, marked High confidence. That is the largest paying-user count for any single paid Mac-first productivity product in the table.
5. **Craft is the closest comparable and it is smaller still:** >50,000 paying customers, >1M active users, ~$7.6M ARR (est. 2026). Mac-first, prosumer, subscription — the report calls it "the most directly comparable."
6. **Almost every comparable number is a third-party estimate, and most sources violate the report's own source rules.** The prompt explicitly banned "SEO content farms, AI-generated listicles, and any page that cites a number without naming its origin." The comparables table is nonetheless built on TechLila, GetLatka, Fueler.io, Taskade, ToolGuide, BoringCashCow, Skillademia, XTendedView, operatorbook.dev, downloadchaos.com, inteldo.com, onepc.org, and Reddit threads. See "Unusable or Unsourced" below.
7. **JetBrains data was requested and never delivered.** The prompt named the JetBrains Developer Ecosystem Survey as a required source. Four JetBrains URLs appear in the footnote list (#28, #29, #30, #159) but **no JetBrains percentage appears anywhere in the body or tables**, and the report does not list this among its own Gaps. This is an unflagged failure.
8. **Utilities historically do not monetize.** Arc: 1–5M users, **$0 direct revenue**, acquired for ~$610M for strategic value. Cron: ~50,000 users, made **free** after acquisition. Rectangle: free. Things/Bear/Alfred/Magnet/Bartender: no disclosed paying users or revenue at all.

---

## Quantitative Evidence Tables

### Section 1 — macOS installed base and professional segment share

| Figure | Source (publication + author) | Date of data | Definition / methodology | Geography | Confidence (report's) |
|---|---|---|---|---|---|
| **2.5 billion active Apple devices** (all products) | Apple Inc. Q1 FY2026 earnings press release (Tim Cook, CEO) | January 2026 | "Active devices" = installed base of devices actively in use by customers at a given time (Apple's definition). Includes iPhone, Mac, iPad, Apple Watch, AirPods, Apple TV, HomePod. | Global | High (Apple disclosure) |
| **~200 million active Macs worldwide** | Skillademia / Apple statistics aggregation (citing Apple FY2025 Q1 breakdown) | Early 2025 | Derived from Apple's 2.35B total active devices, with Mac stated as 200M in breakdown | Global | Medium (Apple component breakdown reported by third party, **not** directly in earnings release) |
| **~100–105 million active Mac users worldwide** | Business of Apps / Apple statistics; XTendedView | 2024–2025 | "Mac user base" = active Mac users. One source states "surpassed 100 million active users in 2024." | Global | Medium (analyst aggregation, not Apple direct) |
| **US Mac market share: 14.4–15.7% of PC market** | Omdia via MacTech / AppleWorld.Today | Q2 2025 – Q4 2025 | PC **shipments** (excluding tablets); Apple Mac units sold in US as share of total PC units | United States | Medium (analyst estimate, not Apple disclosure) |
| **US macOS desktop OS share: 29.62%** | StatCounter via Telemetrydeck (reported by CommandLinux) | 2025 | Desktop OS market share based on **web traffic telemetry** | United States | Medium (third-party web traffic sample) |
| **31.8% of developers use macOS** (personal and professional) | Stack Overflow Developer Survey 2024 | 2024 | Survey of ~65,000 developers; asked which OS they use for personal and professional development | Global | High (well-known annual survey) |
| **32.7% (personal) / 32.9% (professional) of developers use macOS** | Stack Overflow Developer Survey 2025 | 2025 | Survey of ~50,000 developers from 177 countries | Global | High |
| **~30.65% of professional developers use macOS** | TechLila aggregation of Stack Overflow / JetBrains data | 2025 | Aggregated from developer surveys | Global | Medium (aggregation, not primary source) |
| **82.3% of designers use Figma as primary tool; Sketch 1.8%** | UX Tools 2024 Design Tools Survey (n=2,220 designers) | Nov 2024 – Jan 2025 | Annual survey of designers; asked primary design tool. Sketch is macOS-only. | Global | High (well-known design survey) |
| **~18% of native macOS UI designers use Sketch** | UX Tools 2024 Design Tools Survey (cited by DigiToolBook) | 2024 | Subset of macOS-native designers | Global | Medium (derived from survey) |

**Report's own notes on Section 1:**
- Apple does **not** break out Mac installed base by country in earnings releases. The 200M figure is third-party aggregation of Apple's component breakdowns, not a direct Apple disclosure.
- **No Apple disclosure gives a professional vs. personal use split for Mac users.** Must be inferred from occupational and survey data.
- Developer and designer surveys show macOS is used by roughly **one-third of developers** and a **small but loyal minority of designers** (Sketch users).

### Section 2 — Knowledge worker population (US), with each source's definition

| Figure | Source | Date | **Definition used** | Geography | Confidence |
|---|---|---|---|---|---|
| **70.7 million** in "Management, professional, and related occupations" | U.S. Bureau of Labor Statistics, Current Population Survey, Table cpsaat09 | 2024 | BLS occupational category: includes management, business, financial, professional (e.g. computer/mathematical, legal, education, healthcare practitioners) | United States | High (government statistics) |
| **40.1 million** in "Professional and related occupations" | BLS CPS Table cpsaat12 | 2024 | Subset of above: professional occupations only (excludes management/business/financial) | United States | High |
| **6.4 million** in "Computer and mathematical occupations" | BLS CPS Table cpsaat12 | 2024 | Software developers, data scientists, IT, etc. | United States | High |
| **~100 million knowledge workers in US** | GitHub Gist / aggregated research (citing BLS 71.5M management/professional + freelance estimates) | 2024–2025 | "Knowledge workers" defined as ~38–42% of total US workforce; management + professional + related occupations **plus freelancers** | United States | **Low–Medium** (aggregation, not official BLS definition) |
| **Knowledge workers = "people with freedom to use expertise to create something new"** (definition only, no count) | ADP Research Institute | April 2026 | Conceptual definition, not tied to BLS categories | United States | Medium (research institute, not government) |

**Report's own notes on Section 2:**
- **BLS does not use the term "knowledge worker"** in its official occupational taxonomy. Closest proxy is "Management, professional, and related occupations" (70.7M in 2024).
- The "100 million knowledge workers" figure is widely cited in consulting/blog content but is an **inference** from BLS categories plus freelance estimates, **not a direct BLS statistic**.
- **No source directly measures "knowledge workers juggling many simultaneous projects/documents/tools."** Must be inferred from occupational subcategories.

### Section 3 — COMPARABLE PRODUCT SCALE (the most important table; captured verbatim)

| Product | Free Users | Paying Users | Revenue / ARR | Source | Date | **Disclosure type** | Confidence |
|---|---|---|---|---|---|---|---|
| **Raycast** | ~500,000+ active users (total) | Not disclosed (freemium; Pro $10/mo) | ~$5.2M estimated annual revenue (2025) | TechLila; Startup Founder Stories | 2024–2025 | **Third-party estimate**; founder mentioned $10k MRR milestone in 2024 | Medium |
| **Setapp** | Not disclosed (subscription-only) | Not disclosed (est. **300k–500k subscribers**) | ~$40M+ ARR estimated (2024); MacPaw total ~$34.8M ARR (2025) | TechLila; GetLatka (MacPaw) | 2024–2025 | **Third-party estimate**; MacPaw is private | Medium |
| **Obsidian** | ~1M+ monthly active users (free) | Not disclosed (Sync $5/mo, Publish $10/mo) | **~$2M–$25M ARR estimated**; $2M/mo reported Sept 2025 in viral post | Taskade; Fueler.io; Reddit analysis | 2025–2026 | **Mixed**: $2M/mo claimed by founder in social post; $2M ARR from GetLatka | **Low–Medium (conflicting estimates)** |
| **Things (Cultured Code)** | Free trial (Mac) | Not disclosed (one-time: $49.99 Mac, $9.99 iPhone, $19.99 iPad) | ~€10M total lifetime revenue estimated | Reddit r/thingsapp; 9to5Mac | 2026 | **User inference; no company disclosure** | **Low** |
| **Bear (Shiny Frog)** | Free tier | Not disclosed (Pro $2.99/mo or $29.99/yr) | Not disclosed | ToolGuide; Shiny Frog site | 2025 | **No disclosure** | **Low** |
| **CleanShot X** | Not applicable (paid only) | Not disclosed | ~$10M+ lifetime revenue (pre-MacPaw); ~$100k/yr pre-2019 | TechLila; BoringCashCow; Scroll.Media | 2019–2025 | **Third-party estimate**; MacPaw acquisition | Medium |
| **Alfred** | Free tier (limited) | **20%–30% of total users** (Powerpack £34 one-time) | Not disclosed; company states "profitable on Powerpack sales" | TechLila; Running with Cranes site | 2024–2026 | **Company statement** (profitable); no revenue figure | Medium |
| **Superhuman** (email client, pre-Grammarly) | None (paid only) | **~70,000 paying customers (late 2024)** | **$36.5M ARR (late 2024); $35M (2025)** | Sacra; GetLatka; Inc. (Rahul Vohra) | 2024–2025 | **COMPANY-DISCLOSED** (founder statements) | **High** |
| **Cron / Notion Calendar** | ~50,000 users at acquisition (mostly free) | ~5,000 paying (estimated) | ~£2M ARR estimated at acquisition (2022); **made free post-acquisition** | Small Start; Contrary Research | 2022 | **Third-party estimate**; Notion made it free | Medium |
| **Craft (Luki Labs)** | Free tier | **>50,000 paying customers; >1M active users** | ~$7.6M ARR estimated (2026) | Pragmatic Engineer, "Inside a five-year-old startup's rapid AI makeover"; Fueler.io | 2025–2026 | **COMPANY-DISCLOSED (50k+ paying)**; ARR estimated | **High (paying count); Medium (ARR)** |
| **Arc Browser (The Browser Company)** | All users free | None (no paid tier) | **$0 direct revenue**; acquired for ~$610M (2025) | TechGlimmer; LinkedIn; Hacker News | 2025 | **Company-disclosed (free)**; acquisition price reported | High |
| **Rectangle / Magnet** | Free (Rectangle) / Paid (Magnet) | Not disclosed | Not disclosed | GitHub (Rectangle); Mac App Store (Magnet) | 2024–2026 | **No disclosure** | **Low** |
| **Bartender (iBoysoft)** | Free trial | Not disclosed ($20 one-time) | Not disclosed | FavTray; iBoysoft site | 2024–2026 | **No disclosure** | **Low** |

**Company-disclosed vs third-party — summary:**
- **Company-disclosed (usable in a deck with attribution):** Superhuman ~70k paying / $36.5M ARR (founder statements); Craft >50k paying customers; Arc "all users free / no paid tier" + $610M acquisition price; Alfred "profitable on Powerpack sales" (qualitative only).
- **Third-party estimate only:** Raycast users and revenue; Setapp everything; Obsidian everything; CleanShot X; Cron; Craft's ARR; Alfred's 20–30% conversion; Setapp's 300–500k subscribers.
- **No disclosure and no credible estimate:** Bear, Rectangle, Magnet, Bartender, Things paying-user count.

**Report's own "Key observations" on Section 3 (verbatim substance):**
- "Superhuman is the clearest benchmark: ~70k paying users at $30/mo = ~$36M ARR. This is a premium email client for professionals, not a utility, but it shows the ceiling for a paid Mac productivity tool."
- "Craft is the most directly comparable (Mac-first, prosumer, subscription): >50k paying users, ~$7.6M ARR."
- "Raycast (~500k users, ~$5M ARR) and Setapp (~$40M ARR across all apps) show that launcher/utility products can reach mid-single-digit to low-double-digit millions in ARR."
- "Obsidian has the widest estimate range ($2M–$25M ARR), reflecting uncertainty."
- "Things and Bear are one-time-purchase or low-subscription models; no reliable paying-user counts are disclosed."
- "Arc had 1–5M users (peak) but **zero revenue**; it was acquired for its strategic value, not its monetization."

---

## Section 4 — Bottom-up TAM / SAM / SOM arithmetic (reproduced exactly)

### Base case ("using only sourced figures")

| Step | Operation | Result | Basis |
|---|---|---|---|
| 1 | US knowledge workers (BLS proxy) | **70.7M** | BLS CPS 2024, "Management, professional, and related occupations" — **never used downstream** |
| 1 | US Mac users (inferred) | **80–100M active Macs** | ~200M active Macs globally × "US is ~40–50% of Apple's revenue historically." Report labels this **INFERENCE**: "Apple does not disclose Mac installed base by country." |
| 2 | Professional-use filter: assume **60%** of US Mac users use Mac primarily for work | 80M × 60% = **48M** | "based on developer/designer survey data showing high professional use among macOS users" — assumption, not measured |
| 3 | Multi-project filter: assume **50%** are in high-context-switching roles | 48M × 50% = **24M** | assumption (developers, designers, PMs, consultants, researchers, academics) |
| 4 | Willingness-to-pay: **0.5%–1.0%** conversion at $10–25/mo | 24M × 0.5% = **120,000 paying** (conservative); 24M × 1.0% = **240,000 paying** (optimistic) | Benchmarked against Craft (>50k paying) and Superhuman (~70k paying) |
| 5 | ARPU $15/mo = $180/yr | **TAM: $21.6M ARR** (conservative) / **$43.2M ARR** (optimistic) | 120,000 × $180 / 240,000 × $180 |
| 6 | SAM = 50% of TAM (US-focused, English-speaking, Mac-only) | 60,000 × $180 = **$10.8M ARR**; 120,000 × $180 = **$21.6M ARR** | assumption |
| 7 | SOM = 10%–20% of SAM over 3–5 years | 6,000 × $180 = **$1.08M ARR**; 24,000 × $180 = **$4.32M ARR** | "consistent with Craft's ~50k paying users over 5+ years" |

### Pessimistic build

| Step | Operation | Result |
|---|---|---|
| 1 | 100M global Mac users (low-end estimate), 30% in US | **30M US Mac users** |
| 2 | 40% are knowledge workers | **12M** |
| 3 | 30% juggle many projects | **3.6M** |
| 4 | 0.25% conversion ("more conservative than Craft/Superhuman benchmarks") | **9,000 paying users** |
| 5 | ARPU $12/mo = $144/yr | **TAM revenue: $1.3M ARR** |
| 6 | SAM (50%) | **$650k ARR** |
| 7 | SOM (10% of SAM) | **$65k ARR — "not viable as standalone business"** |

### Report's "Bottom line for your pitch" (verbatim substance)
- **Realistic ceiling** for a paid Mac utility (prosumer, $10–25/mo) based on comparables: **~$5M–$40M ARR** (Raycast to Superhuman range).
- **Defensible SOM** for a new entrant: **$1M–$4M ARR in 3–5 years** (6k–24k paying users at $180/yr), "assuming you execute well and differentiate."
- **Pessimistic case:** sub-$1M ARR "if conversion is lower than Craft/Superhuman benchmarks or if the product is perceived as a 'nice-to-have' utility rather than a must-have workflow tool."

### Arithmetic and method problems in the build (my audit, not the report's)
1. **Superhuman arithmetic is wrong.** 70,000 × $30/mo × 12 = **$25.2M**, not "~$36M ARR." To reach $36.5M at 70k customers, ARPU must be ~$43/mo. Either the paying-user count or the ARR is off; do not present "70k × $30 = $36M" in a deck.
2. **Raycast is internally inconsistent.** "~$5.2M estimated annual revenue (2025)" sits in the same cell as "founder mentioned $10k MRR milestone in 2024" ($120k ARR). These differ ~43×. The report does not reconcile them.
3. **Triple-discounting.** Step 4 already applies a 0.5% *paid conversion* filter, which by construction yields payers, not a total addressable market. Applying a further 50% (SAM) and then 10–20% (SOM) discounts payers three times. The "TAM" is not a TAM; it is a bear-case revenue forecast dressed as a market size.
4. **The funnel is 100% assumption after Step 1.** The 60%, 50%, 0.5–1.0%, 50%, and 10–20% multipliers have no cited source. Only the top-of-funnel Mac count (itself an INFERENCE) is tied to data.
5. **The BLS 70.7M is never multiplied by anything.** The build never intersects the knowledge-worker population with the Mac population; it just asserts a professional-use share of Macs.

---

## Strongest Citable Claims (from Report 1)

Ranked by defensibility. Only these should appear in the deck as sourced facts.

| # | Claim | Attribution to use | Why it holds |
|---|---|---|---|
| 1 | Apple has **2.5 billion active devices** worldwide | Apple Inc., Q1 FY2026 earnings, Tim Cook, January 2026 | Direct company disclosure. **Caveat: all-products, not Mac.** |
| 2 | **70.7 million** Americans work in "Management, professional, and related occupations" | U.S. BLS, Current Population Survey, Table cpsaat09, 2024 | Government statistic. Use the BLS category name, never "knowledge workers." |
| 3 | **6.4 million** Americans in "Computer and mathematical occupations" | BLS CPS Table cpsaat12, 2024 | Government statistic; the tightest high-confidence beachhead number in the document |
| 4 | **~33% of developers use macOS** (32.7% personal / 32.9% professional) | Stack Overflow Developer Survey 2025, ~50,000 developers, 177 countries | Well-known annual survey. **Caveat: report sourced the % via aggregator blogs, verify against stackoverflow.co before publishing.** |
| 5 | **Superhuman: ~70,000 paying customers, ~$36.5M ARR (late 2024)** | Founder (Rahul Vohra) statements via Sacra / Inc. | Only company-disclosed paying-user + ARR pair in the comparables set |
| 6 | **Craft: >50,000 paying customers, >1M active users** | Luki Labs, via The Pragmatic Engineer (2025–2026) | Company-disclosed paying count; closest structural comparable (Mac-first, prosumer, subscription) |
| 7 | **Arc had 1–5M users and $0 direct revenue; acquired ~$610M** | The Browser Company (free tier confirmed); acquisition price reported 2025 | Useful as a cautionary comparable, not a positive one |
| 8 | **A paid prosumer Mac tool realistically tops out at ~$5M–$40M ARR** | Report's own synthesis of the comparables table | Honest framing; use this rather than a fabricated big TAM |

---

## Unusable — No Provenance, or Provenance That Breaks the Report's Own Rules

The prompt banned "SEO content farms, AI-generated listicles, and any page that cites a number without naming its origin." These figures fail that test and should **not** appear in the deck.

| Figure | Why unusable |
|---|---|
| **Things: ~€10M total lifetime revenue** | Explicitly "Reddit user inference"; no company disclosure. Report itself marks Low. |
| **Setapp: 300k–500k subscribers** | Unattributed estimate; report marks "est." with no methodology |
| **Setapp: ~$40M+ ARR** | Sourced to TechLila (aggregator). Conflicts with MacPaw *total* $34.8M ARR (GetLatka) — a subsidiary cannot exceed the parent. Report flags the conflict but does not resolve it. |
| **Obsidian: $2M–$25M ARR** | 12.5× spread; the high end traces to a viral LinkedIn post. Unusable in either direction. |
| **Raycast: ~$5.2M annual revenue** | TechLila estimate; contradicted by the founder's own "$10k MRR" reference in the same cell |
| **CleanShot X: ~$10M+ lifetime revenue** | Third-party estimate via BoringCashCow / TechLila; no user count, no method |
| **Alfred: 20–30% of users buy Powerpack** | Sourced to TechLila and a personal blog ("Running with Cranes"); no method, no N |
| **Cron: ~5,000 paying users / ~£2M ARR** | Estimate; currency is stated in GBP with no explanation; Notion has not disclosed |
| **Arc: 1–5M users** | Report: "no official disclosure from The Browser Company." 5× spread. |
| **~100 million US knowledge workers** | Traced to a GitHub Gist aggregating BLS + freelance estimates. Report marks Low–Medium and calls it "an inference … not a direct BLS statistic." |
| **~30.65% of professional developers use macOS** | TechLila aggregation of surveys; superseded by the primary Stack Overflow figures |
| **~200 million active Macs** | Not an Apple disclosure. Skillademia aggregation of an Apple "component breakdown." Report marks Medium. Do not attribute this to Apple. |
| **US Mac installed base 80–100M** | Report's own label: **INFERENCE**, "an assumption, not a sourced figure" |
| **Bear, Rectangle, Magnet, Bartender — all figures** | Nothing disclosed and nothing estimated. Report marks Low across the board. |
| **ADP Research Institute "knowledge worker" definition** | A definition, not a number. Cannot be used to size anything. |

---

## Gaps and Conflicts

### Conflicts the report itself lists
| Conflict | Detail |
|---|---|
| **Mac installed base** | Apple discloses 2.5B total active devices, does not break out Mac. Third parties say ~200M active Macs. Other sources say 100M+ Mac *users*. Report notes "active devices" ≠ "users" (one user, multiple devices). |
| **Obsidian revenue** | $2M ARR (GetLatka, Fueler) vs $25M ARR (viral LinkedIn post citing $2M/mo). No official disclosure. |
| **Setapp revenue** | $4–6M annual revenue vs $40M+ ARR. MacPaw total est. $34.8M ARR (2025). Setapp-specific figures vary. |
| **US Mac installed base** | No Apple disclosure. Omdia/StatCounter give 14–16% US PC share, "but this is shipments, not installed base." Inferred 80–100M is an assumption. |

### Additional conflict the report reports but does not flag
- **29.62% vs 14.4–15.7%.** StatCounter web-traffic telemetry puts US macOS desktop share at 29.62%; Omdia unit shipments put US Mac at 14.4–15.7% of PCs. These differ ~2×. Both are in the same table with no reconciliation. Web-traffic share is biased upward by device usage intensity and by Safari/iOS-adjacent traffic; shipment share is biased by enterprise Windows fleet refresh. If you cite a Mac share number, say which methodology and expect to be challenged.

### Gaps the report itself lists (questions it could not answer)
1. **Share of macOS users using Mac for professional vs. personal purposes** — "No survey or Apple disclosure directly answers this." This is the single filter the base-case build depends on most (the 60% assumption).
2. **macOS share among product managers, consultants, academics, writers/researchers** — "No survey data found for these specific roles." Only developers and designers have published figures.
3. **Number of knowledge workers who juggle many simultaneous projects/documents/tools** — "No source measures this directly. BLS occupational data gives counts, but not work-pattern intensity." This is the Step 3 (50%) assumption.
4. **Paying user counts for most Mac utilities** — Things, Bear, Alfred, Rectangle, Magnet, Bartender: "Only estimates or no data at all."
5. **Cron revenue at acquisition** — "Notion has not disclosed exact figures."
6. **Arc browser user count** — "Estimates range from 1–5M users, but no official disclosure."
7. **CleanShot X, Bartender, Rectangle paying users** — "No reliable figures found."

### Gaps the report does NOT flag (my findings)
- **JetBrains Developer Ecosystem Survey delivered zero data.** Explicitly requested in the prompt; four JetBrains URLs sit unused in the footnotes; no percentage appears in the body. The only "JetBrains" number in the document is laundered through a TechLila aggregation.
- **No pricing-elasticity or willingness-to-pay evidence at all.** The $10–25/mo range and the 0.5–1.0% conversion rate are pure assertion. No source in the document measures conversion rates for Mac prosumer tools.
- **No churn or retention data for any comparable.** For a subscription pitch this is a material omission — ARR figures without retention say nothing about durability.
- **No US-vs-global split for any comparable.** Superhuman's ~70k and Craft's >50k are global. The build treats them as benchmarks for a US-only funnel.
- **The report's source list contains SEC filings for entirely unrelated companies** (Google #48, Ziff Davis #49, Bumble #50/#61/#143, Meta #116, Varonis #96, Joyy #57, Gamehaus #59, Cricut #131, Match #128) and market-research pages for **neodymium magnets** (#149, #150) and **"mobile bartender services"** (#147). These are keyword collisions ("Magnet," "Bartender"), indicating the retrieval was noisy. Treat any figure whose only citation is deep in this list with suspicion.

---

## Contradictions to the Positioning

Positioning under test: *"a local-first cognitive layer that preserves work continuity across your computer, letting you return to any project exactly where you left off."*

| # | Contradiction | Evidence from Report 1 |
|---|---|---|
| 1 | **The market may be too small to be venture-fundable.** Pre-seed VCs underwrite to $100M+ ARR outcomes. The report's ceiling for the *entire category* is $5–40M ARR, and the defensible 3–5 year SOM is $1–4M ARR. | Report's own "Bottom line for your pitch," p. 10 |
| 2 | **The best-case comparable is not a utility.** The report explicitly notes Superhuman "is a premium email client for professionals, **not a utility**." A continuity/restoration layer is structurally closer to Raycast (~$5M ARR est.) or Alfred (no disclosed revenue) than to Superhuman. | Section 3 key observations |
| 3 | **Mac utilities that reached real scale reached it for free.** Arc: 1–5M users, $0 revenue. Cron: made free after acquisition. Rectangle: free. The scale/monetization tradeoff in this exact category is empirically brutal. | Section 3 |
| 4 | **The report names "nice-to-have utility" as the failure mode by name.** "Sub-$1M ARR if … the product is perceived as a 'nice-to-have' utility rather than a must-have workflow tool." Continuity restoration is precisely the kind of benefit users may believe they already get from macOS window restore, browser session restore, and IDE workspace state. | Report's pessimistic case |
| 5 | **Designers are not a Mac-lock-in segment any more.** Sketch (macOS-only) is at 1.8% primary-tool share vs Figma's 82.3%. Figma is browser-based and cross-platform. A Mac-only product cannot claim the design market as a Mac-native beachhead. | UX Tools 2024 Design Tools Survey, n=2,220 |
| 6 | **Developers — the strongest segment — are only ~one third macOS.** 32.9% professional. So a Mac-only wedge forfeits two-thirds of the most tool-adopting audience from day one. | Stack Overflow Developer Survey 2025 |
| 7 | **"Multi-project knowledge workers" is not a measurable population.** The report states outright that no source measures it. The segment the product is defined around cannot be counted, which means the SAM cannot be defended under diligence questioning. | Report's Gaps #3 |
| 8 | **Nothing in the report supports "local-first" as a demand driver.** There is zero evidence in 167 sources about willingness to pay for local/private processing, or about privacy concerns with passive observation software. If local-first is a core differentiator, it is currently unevidenced — and passive observation is a *purchase objection* the report never examines. | Absence across the whole document |

---

## How to use this in the deck (recommendation)

Do **not** present a TAM. The honest, defensible move given this evidence base:
- Lead with the segment count that is a real government statistic: **6.4M US computer-and-mathematical workers** and **70.7M US management/professional/related workers (BLS CPS 2024)**.
- Show the comparable-ceiling table with the **company-disclosed** rows only (Superhuman ~70k paying; Craft >50k paying; Arc free/$0) and state plainly: *"the disclosed ceiling for a paid Mac-first prosumer tool is ~70,000 paying customers."*
- State the target as a paying-customer count, not a dollar TAM: e.g. *"10,000 paying customers at $180/yr = $1.8M ARR"* sits inside the report's defensible SOM band and is auditable.
- If a large number is required, the only structurally honest path is to argue the product is **not** a Mac utility — cross-platform, or team/enterprise — and the report contains no evidence for either expansion.
