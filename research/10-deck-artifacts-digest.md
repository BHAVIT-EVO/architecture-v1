# 10 — Deck Artifacts Digest

**Source artifact:** `uploads/You are a research analyst studying pitch deck art.pdf` (Perplexity research report)
**Total pages:** 17. Page 1 = the original prompt verbatim. Pages 2–14 = body (~13 pages). "Sources for further study" starts mid-page 14; numbered sources run page 14 → 17 (~3.5 pages, **83 numbered URLs**).
**Digested:** 2026-08-31. Permanent record for the deck-structure decision.

**Body citation range:** inline refs run [1]–[40]. **Sources 41–83 (43 of 83, 52%) are never cited in the body** — and 34 of them are pitch-deck-advice content, the exact class the prompt banned.

**Report's own closing claim:** "This gives you **seven real, publicly available decks** with direct links, slide-by-slide breakdowns, and structural analysis."
**Honest count after audit: 4 defensible artifacts, of which 1 is advice rather than a deck. See Section 6.**

---

## 1. Deck-by-deck, as reported

### Retrievability audit key
- **A** = real artifact, working link to a credible host, breakdown derived from the artifact
- **B** = real historical artifact, but linked only via aggregators / a synthetic-looking CDN
- **C** = described from a *summary of a different document*; artifact not retrieved
- **D** = mislabeled or internally impossible; the described contents contradict the stated stage/date
- **E** = founder post retrieved, but what's presented is advice, not the deck's slides

| # | Company | Stage | Amount | Date | Investors named | Slide count | Densest slide | Real vs redesign (report's claim) | **Grade** |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Airbnb | Seed | $600K | 2009 (post-YC 2008–09) | Not named | 14 | Slides 10 & 2, ~60–80 words | "Real deck used to raise" | **B** |
| 2 | Buffer | Seed | ~$450K–$500K | 2011 | Prior investors named on team slide: AngelPad, Inspiration, Sierra Ventures, InterWest | 13 | Slides 5 & 7, ~70–90 words | "Original 2011 file… published openly as part of their 'open startup' ethos" | **B** |
| 3 | Uber (UberCab) | Pre-seed/Seed | $200K | Dec 2008 | Not named | 25 | Financial projection / rollout slides, ~100–150 words | "Original 2008 UberCab deck created by Garrett Camp" | **B / C** |
| 4 | Front | Seed (+ A, B, D) | Seed n/d; Series A $10M | 2016 (A) | Not named | Seed ~11; Series A 21 | Not given | "Real decks used to raise each round" | **C** (seed), **B** (Series A) |
| 5 | **Momentum** | Seed | $5M heading / **"$4 million" on the ask slide** | 2022 | Not named; CEO Santiago Suarez Ordoñez | 19 | Slide 16 (ask) + problem/solution, ~80–100 words | "Real deck used to fill out the seed round after a lead investor was secured without a deck" | **A** |
| 6 | June | Seed (YC W21) | Not given | 2021–22 | Not named | "6–8 slides total" + appendix — **not explicitly stated** | Not given | "Real deck used to raise after YC W21" | **E** |
| 7 | Notion | Seed | $2M | "2013/2016" — report cannot resolve | Not named | 19 | Slides 2–3 & 18–19, ~80–100 words | "Original seed deck used to raise $2M in 2013 (some sources say 2016)" | **D** |

---

### 1. Airbnb (2009) — Seed, $600K — **Grade B**

**Where available (as given):**
- `https://pitches.ai/pitch-deck/airbnb` — "embeds original PDF with slide-by-slide"
- "SlideShare and various aggregators (search 'Airbnb pitch deck 2009 PDF')" ← **this is an instruction to search, not a link. Rule violation.**
- Secondary cited: `spectup.com/resource-hub/airbnb-pitch-deck-analysis`

| # | Headline / content (as reported) |
|---|---|
| 1 | **Cover:** "Book rooms with locals, rather than hotels." — one-line value prop, no tagline soup |
| 2 | **Problem:** three pain points — price anxiety, hotels disconnect you from culture, no easy way to book with locals or become a host. Bold keywords for each |
| 3 | **Solution:** "A web platform to rent space to travelers, save money traveling, make money hosting, connect with local culture." Icons + short tiles |
| 4 | **Market validation:** "Couchsurfing: 670,000 users. Craigslist: 17,000 temporary housing listings in SF and NYC in one week." Real named comparables, not vanity TAM |
| 5 | **Market size:** "$2B+ trips booked worldwide → $560M budget/online segment → $84M serviceable share (15%)." TAM → SAM → SOM funnel |
| 6 | **Product:** search-by-city interface with listings, map pins, booking flow. **Screenshots dominate** |
| 7 | **Business model:** "10% commission on completed bookings, host fee + guest fee on each transaction." One killer metric slide with simple take-rate math |
| 8 | **Market adoption:** events, partnerships, classifieds as acquisition channels for hosts and guests. GTM before competition |
| 9 | **Competition:** 2×2 matrix — temporary housing vs hotels, transactional vs non-transactional. Airbnb in an empty quadrant |
| 10 | **Competitive advantages:** six short advantages in a grid — first transaction-based temp housing marketplace, host incentives, profiles, 3-click booking, search by price/location. Operational, not buzzwords |
| 11 | **Team:** "Brian Chesky, Joe Gebbia, Nathan Blecharczyk, design, business, and engineering from RISD/Harvard" |
| 12 | **Press:** Mashable, Webware, Josh Spear, Springwise |
| 13 | **User testimonials:** quotes from early hosts and guests with photos and locations |
| 14 | **Financial:** revenue projection path from early bookings to $200M, circles and arrows, not a dense spreadsheet |

**Handling:** Opening = one-sentence value prop. Problem = three concrete pain points, bold keywords. Product = single screenshot-heavy slide. Market = classic TAM/SAM/SOM, three repeatable numbers. Competition = 2×2 matrix, not a feature checklist. Traction = press + testimonials (pre-revenue). Team = RISD/Harvard credentials. **Ask = none; the financials slide implies the raise.**
**Visual:** screenshots on slide 6; abstract graphics on market size and financials; 2×2 for competition.
**Unusual:** no explicit ask slide; financials as visual narrative.

---

### 2. Buffer (2011) — Seed, ~$450K–$500K — **Grade B**

**Where available (as given):**
- `https://pitches.ai/pitch-deck/buffer` — aggregator
- `https://media.genppt.com/pitch-decks/buffer/buffer-pitch-deck-2011.pdf` — **synthetic-looking CDN, see QA #3**
- `https://www.slideshare.net/slideshow/buffer-seed-round-pitch-deck/246062042`
- `https://thefounderfolks.com/fundraising-decks/buffer` — aggregator, **but the report files this under "Founder posts with decks"**
- Also in source list: `bestpitchdeck.com/buffer`, `roundfunded.com/en/pitch-deck/buffer`, `datapile.co/pitch-decks/buffer-seed-2011`

| # | Headline / content (as reported) |
|---|---|
| 1 | **Cover:** Buffer wordmark and stacked-layers logo on a dark textured background. Minimal, brand only, no tagline |
| 2 | **"Social, the most important trend":** Zuckerberg's Law on sharing doubling yearly, a Donanza quote on social surpassing SEO, photo of Facebook's engagement growth curve. **Macro tailwinds before the product** |
| 3 | **Problem:** one question centered — **"How do you use social to drive traffic?"** Problem as a single provocative question |
| 4 | **Solution:** "Queue your updates" + screenshot of the Buffer web app with tweets scheduled across days and times. **Solution is the product itself** |
| 5 | **Traction:** "800 paying users, $150K annual revenue run rate, 97% margins, 55,000 users growing 40% per month, 1.5M updates buffered", with an upward growth curve. "The slide investors remember" |
| 6 | **Milestones:** timeline Jan 2011 launch → Oct 2011 traction, then green forward targets: API, 50 integrations, 100K users, 1M users by Jan 2013. **Past in gray, future in green** |
| 7 | **Business model:** "Freemium with 2% free-to-paid conversion, 5% churn ($240 LTV), up to $5 CAC per free user, $3.6M projected revenue at 1M users." Unit economics on one slide |
| 8 | **Social media landscape:** "200M daily tweets (55% with links), 4B Facebook shares per day, Zuckerberg's Law, and social traffic soon surpassing search." **Market size built from sharing volume, not a TAM spreadsheet** |
| 9 | **Social proof:** ReadWriteWeb headline — "Buffer Finds Tweet Scheduling Can Increase Clicks by 200%" |
| 10 | **Integration strategy:** "A sharing standard". 6 integrations live, talks with Reeder, Pocket, Feedly; goal to be the default share target in any app; iPhone share sheet mockup. Platform play |
| 11 | **Competition:** circular landscape map — social networks at center, competitors grouped as dashboards, intelligent sharing, sharing platforms, scheduling apps. Buffer in two quadrants. **Category map, not a comparison table** |
| 12 | **Team:** "Joel Gascoigne (idea to revenue in 7 weeks, CS master's) and Leo Widrich (200 to 55K users); advisors Guy Kawasaki and Hiten Shah; prior investors AngelPad, Inspiration, Sierra Ventures, InterWest." **Execution credentials first** |
| 13 | **Contact:** founders@bufferapp.com, minimal slide, faint logo watermark. No ask slide with terms |

**Handling:** Opening = logo only, no tagline. Problem = single provocative question. Product = real UI screenshot on slide 4. Market = built from sharing volume. Competition = circular landscape map. Traction = hard revenue + users with hockey stick, **the deck's centerpiece**. Team = execution credentials + named advisors. **Ask = none; contact email only.**
**Visual:** screenshots on 4 and 10; circular map for competition; growth curve on traction.
**Unusual:** no ask slide; **traction and unit economics appear before market size**; a macro trend slide opens the narrative.

---

### 3. Uber / UberCab (Dec 2008) — Pre-seed/Seed, $200K — **Grade B/C**

**Where available (as given):**
- `https://www.businessinsider.com/uber-original-pitch-deck-2008` — credible host
- `https://www.scribd.com/document/.../UberCab_Dec2008` — **literal ellipsis in the URL. Not a working link. Rule violation.**
- `https://www.pitchdeckhunt.com/pitch-decks/uber`
- `https://www.slideshare.net/slideshow/uber-pitch-deck-2008/79075025`
- `https://media.genppt.com/pitch-decks/uber/uber-pitch-deck-2008.pdf` — **same synthetic CDN as Buffer**
- Also in source list: `vox.com/2017/8/23/16189048/...`, `techcrunch.com/2023/02/23/sample-pre-seed-pitch-deck-uber-2008/`, `alexanderjarvis.com`

**Report's dating error:** "shared publicly on the 9th anniversary of Uber's idea (**August 2008**)." The 9th anniversary of a 2008 idea is 2017 — which is when Camp actually shared it. Internally impossible as written.

| # | Headline / content (report labels this "from available summaries") |
|---|---|
| 1 | **Cover:** UberCab — "Everyone's Private Driver" / "Next-generation on-demand car service" |
| 2 | **Problem:** "Taxis suck" — long wait times, unreliable service, payment friction |
| 3 | **Solution:** on-demand luxury car service via mobile app, pre-screened members only |
| 4 | **Market opportunity:** large urban professionals, SF and NYC launch, then nationwide |
| 5 | **Product:** mobile app interface mockups, optimized dispatching algorithms |
| 6 | **Business model:** per-ride commission, premium pricing vs taxis |
| 7 | **Competitive landscape:** taxis, limo services, private drivers — UberCab positioned as faster, more efficient |
| **8–25** | **"Various:** financial projections, team, rollout plan, technology details, 'iPhone dev license applied for Nov 28, 08', etc." |

**18 of 25 slides — 72% of the deck — are collapsed into one row.** This is not a slide-by-slide breakdown; it is a summary of the first seven slides plus a shrug.

**Handling:** Opening = "Everyone's Private Driver," aspirational. Problem = "Taxis suck," visceral. Product = mockups + dispatch diagrams. Market = SF/NYC then nationwide. Competition = taxis/limos positioned as inferior. **Traction = none, pre-launch.** Team = Camp and Kalanick credentials. Ask = $200K seed to launch in SF.
**Visual:** product mockups and diagrams dominate.
**Unusual:** 25 slides is long for pre-seed; detailed financial projections and rollout plans uncommon for a pre-launch deck.

---

### 4. Front — Seed / A / B / D — **Grade C (seed), B (Series A)**

**Where available (as given):**
- `https://techcrunch.com/2022/09/01/sample-series-d-pitch-deck-front/` — **a Series D teardown**
- "Front's own blog (Mathilde Collin published the Series A deck): `https://www.front.com/blog/...` (original post from 2016)" — **literal ellipsis. Not a working link. Rule violation, and it is the one link the prompt most wanted (company's own blog).**
- `https://www.scribd.com/document/849508445/Front-raised-10-million-with-these-21-slides` — a reproduction of a Business Insider piece on the **$10M Series A**, not the seed

**Slide counts:** Seed ~11 "per TechCrunch summary" · Series A 21 (Mathilde Collin's blog post) · Series B/C/D "varying lengths, all publicly available via TechCrunch links"

**Seed slide order (report's own words: "approximate, from TechCrunch summary"):**
1. Cover · 2. Problem · 3. Solution · 4. Product · 5. Market timing ("why now") · 6. Traction · 7. User validation · 8. Market size · 9. Team and investor slide · 10. "The ask" · 11. "Mic drop" — customer testimonial

**Handling:** Opening = company vision in one simple sentence. Problem/Solution/Product broken into multiple slides for flow. Traction = strong customer growth data. Ask = clear funding amount and use of funds. **Mic drop = customer testimonial as penultimate slide.**
**Visual:** clean, minimal; product screenshots in product slides; customer logos in traction.
**Unusual:** "Front is the only company known to have published **all** of its funding decks publicly, enabling a rare longitudinal study of deck evolution."

**The Front seed deck was never retrieved.** No link resolves to it, no amount is given, no date, no investors, no headline text, no densest-slide count. And the 11-step order above is **structurally identical to Momentum's** (Cover → Problem → Solution → Product → Why now → Traction → Validation → Market → Team → Ask → Mic drop). Both are cited to TechCrunch teardowns. **Strong likelihood the "Front seed" order was reconstructed from the Momentum teardown rather than from Front.** If so, one of the seven decks is a duplicate of another and the "mic drop" pattern rests on a single observation, not two. See QA #5.

---

### 5. Momentum (2022) — Seed, $5M / "$4M" — **Grade A. The only clean artifact.**

**Where available:** `https://techcrunch.com/2022/05/05/sample-seed-pitch-deck-momentum/` — **full deck embedded, reputable host, named founder.**
**Real vs redesign:** "the real deck used to **fill out** the seed round after a lead investor was secured without a deck. CEO Santiago Suarez Ordoñez shared it publicly for the TechCrunch teardown."

| # | Headline / content |
|---|---|
| 1 | Cover slide |
| 2–4 | **Problem slide in three parts** — broken up for flow |
| 5–7 | **Solution slide in three parts** |
| 8–10 | **Product slide in three parts** |
| 11 | **Market timing ("why now")** — distributed work, shift in how sales are done |
| 12 | Traction slide |
| 13 | User validation slide |
| 14 | Market size slide |
| 15 | Team and investor slide |
| 16 | **"The ask" slide** — "We are raising $4 million. That will give us 18 months of runway…" |
| 17 | Placeholder for demo |
| 18 | **"Mic drop" slide** — customer testimonial |
| 19 | Thank you slide |

**Handling:** Opening = clean cover. Problem/Solution/Product **each split into three slides "for easy consumption without the founder narrating."** Market timing = "why now" on distributed work and sales-process shifts. Traction = strong customer validation. **Ask = explicit, specific: "$4 million for 18 months of runway" with milestones.** Mic drop = customer testimonial penultimate.
**Visual:** clean, simple; product screenshots in product slides; customer logos in traction.
**Unusual:** the 3×3 split of problem/solution/product — a deliberate choice for **asynchronous consumption**.

**Caveats:** heading says $5M, the ask slide says $4M — unexplained. Slides 1, 12, 13, 14, 15, 17, 19 have generic labels and **no actual headline text**, so the "actual headline text" requirement is only ~40% met even for the best entry.

---

### 6. June (YC W21, 2021–22) — Seed — **Grade E: this is advice, not a deck**

**Where available:** `https://www.june.so/blog/june-seed-deck` — a genuine founder post, genuine link.
**Slide count:** "**Not explicitly stated**, but described as '6–8 slides total' with an appendix."

| # | "Headline / content" (report labels this "from founder's post") |
|---|---|
| 1 | **Two-sentence description** — "Start your deck with your two sentence description… When fundraising your biggest enemy is confusion." |
| 2–5 | **"Vertebrae" slides:** Team, Unique insight, Problem, Product, Traction — "the 3–5 points founders most want investors to remember" |
| 6–8 | **Supporting slides:** Market, Business model, Ask, etc. (in appendix) |

**Handling:** Opening = two-sentence company description. Problem, Traction, Team = "vertebrae." Product = screenshots. **Market sizing = appendix, not core story. Ask = appendix.**
**Visual:** simple, obvious slides; one idea per slide.
**Unusual — the report quotes the founder's advice as the structural finding:** "Make your slides are simple and obvious. Every word counts. Focus on 1 idea per slide. 6–8 slides total. Go through your deck and only read the headlines – does it tell a compelling story?"

**This entry contains no slide from June's deck.** Every row is the founder's framework for how to build a deck. There is no headline text, no slide count, no amount, no investors, no date, no densest-slide word count. The prompt said: "Do not substitute pitch-deck-advice blog content for actual decks." **This is that substitution, wearing an artifact's formatting.** The advice is good and the "headlines-only story test" is worth adopting — but it is advice, and it must not be counted toward "seven real decks."

---

### 7. Notion — Seed, $2M, "2013/2016" — **Grade D: discard**

**Where available (as given):** `https://upmetrics.co/pitch-deck-examples/notion` · `https://vcmatch.ai/pitch-decks/notion` · `https://thefounderfolks.com/fundraising-decks/notion` — **three aggregators. No Notion primary source. No PDF.**

| # | Headline / content (as reported) |
|---|---|
| 1 | **Cover:** "The Future of Work" — "Collaboration + productivity in one workspace." Big headline, small subhead |
| 2–3 | **Problem:** teams work in silos; tool overload (Evernote, Trello, Google Drive, Dropbox) |
| 4–6 | **Problem impact:** kills productivity, collaboration mess, hard to have shared understanding |
| 7 | **Solution:** "Brings docs, tasks, notes, and projects into one shared workspace" |
| 8 | **What Notion does:** workspace for docs, tasks, notes, team knowledge |
| 9 | **Unified platform:** replaces Trello, Confluence, Jira with one place |
| 10–11 | **Product demo:** desktop and mobile screenshots |
| 12–13 | **Social proof:** "Over a million users"; **Duolingo, Lattice, ZEET** using it |
| 14 | **Why does it matter:** as companies grow, communication gets harder |
| 15 | **Branding:** logo-only slide (cube and "N") |
| 16–17 | **Appendix transition slides:** section divider; **enterprise features (SSO, unlimited workspaces, dedicated support)** |
| 18–19 | **Company timeline (phases 1–5):** paper → PCs → Microsoft → fragmented SaaS → Notion's vision |

**Handling:** Opening = "The Future of Work," aspirational. Problem = visual, scattered app logos. Product = demo slides 10–11. Market = no traditional TAM; timeline shows market evolution. Traction = "over a million users," named customers. **Team = "not explicitly detailed." Ask = "not explicitly stated."**
**Unusual (report's own note):** logo-only slide and appendix transitions "add bulk"; timeline split across two slides "feels unnecessary."

**Why this is a D and must be discarded.** The contents are anachronistic for the stated stage. A $2M seed deck from 2013 (or 2016) cannot claim **"over a million users,"** cannot name **Lattice** as a customer, and would not carry an **enterprise SSO / dedicated-support** appendix. Those are 2019–2020 facts. This is almost certainly a later company or sales deck — or an aggregator's reconstruction — mislabeled as a seed deck. The report itself cannot date it ("2013, some sources say 2016") and lists Team and Ask as absent, which is what you would expect from a non-fundraising deck. **Do not use Notion as a seed-deck reference point.**

---

## 2. Comparative table (report's own, preserved verbatim)

| Company | Stage | Slide count | Slide order (compact sequence) | Notable structural choice |
|---|---|---|---|---|
| Airbnb | Seed ($600K) | 14 | Cover → Problem → Solution → Validation → Market size → Product → Business model → Adoption → Competition → Advantages → Team → Press → Testimonials → Financial | No explicit "ask" slide; financials as visual narrative |
| Buffer | Seed (~$450K) | 13 | Cover → Trend → Problem (question) → Solution (screenshot) → Traction → Milestones → Business model → Landscape → Social proof → Integration → Competition (map) → Team → Contact | Traction and unit economics before market size; no "ask" slide |
| Uber | Pre-seed/Seed ($200K) | 25 | Cover → Problem → Solution → Market → Product → Business model → Competition → … → Team → Financials → Rollout | 25 slides is long for pre-seed; detailed financials and rollout plans |
| Front (Seed) | Seed | ~11 | Cover → Problem → Solution → Product → Why now → Traction → Validation → Market → Team → Ask → Mic drop (testimonial) | "Mic drop" customer testimonial as penultimate slide |
| Momentum | Seed ($5M) | 19 | Cover → Problem (3 slides) → Solution (3) → Product (3) → Why now → Traction → Validation → Market → Team → Ask → Demo → Mic drop → Thank you | Problem/Solution/Product each broken into 3 slides for async consumption |
| June | Seed (YC W21) | 6–8 (+ appendix) | Two-sentence description → Team → Insight → Problem → Product → Traction → (Appendix: Market, Model, Ask) | "Vertebrae" framework: 3–5 key points, 6–8 slides total, one idea per slide |
| Notion | Seed ($2M) | 19 | Cover → Problem (2) → Impact (3) → Solution → What it does → Unified → Demo (2) → Proof (2) → Why matter → Logo → Appendix (2) → Timeline (2) | Logo-only slide and timeline split into two slides add bulk |

**Slide-count spread across the sample: 6–8, 11, 13, 14, 19, 19, 25.** Note this directly contradicts the report's own "funded decks average 10–12 slides" claim in Section 4 — only two of seven fall in that band.

---

## 3. PATTERNS ACROSS DECKS (report's conclusions, verbatim)

1. **One-sentence opening or two-sentence description** — Airbnb ("Book rooms with locals, rather than hotels"), June (two-sentence description), Front (company vision in one sentence).
2. **Problem as a visceral, relatable pain point** — Uber ("Taxis suck"), Buffer (single question: "How do you use social to drive traffic?"), Airbnb (three concrete pain points).
3. **Product screenshots early** — Buffer (slide 4), Airbnb (slide 6), Notion (slides 10–11), Momentum (slides 8–10). "Investors want to see the thing exists."
4. **Traction as the centerpiece (for seed)** — Buffer (slide 5: 55K users, $150K ARR), Front (customer growth), Momentum (user validation slide).
5. **Unit economics explicit (for SaaS)** — Buffer (slide 7: 2% conversion, 5% churn, $240 LTV, $5 CAC).
6. **Competition as a map or matrix, not a feature checklist** — Airbnb (2×2 matrix), Buffer (circular landscape map), Notion (timeline showing market evolution).
7. **Team slide with execution credentials, not résumé fluff** — Buffer ("idea to revenue in 7 weeks," "200 to 55K users"), Airbnb (RISD/Harvard), Notion (team credentials).
8. **"Mic drop" closing** — Front and Momentum both end with a customer testimonial as the penultimate slide.
9. **No explicit "ask" slide in some early decks** — Airbnb, Buffer, and Notion lack a traditional ask slide; the financials or contact slide implies the raise.
10. **Simple, minimal design** — June's founder: "Make your slides are simple and obvious. Every word counts. Focus on 1 idea per slide."

**Which patterns actually survive the audit:**

| Pattern | Survives? | Why |
|---|---|---|
| 1 — one/two-sentence opening | **Yes** | Airbnb (artifact) + June (founder's own words). Independent |
| 2 — visceral problem | **Yes** | Airbnb + Buffer + Uber, three independent artifacts |
| 3 — product screenshots early | **Yes** | Buffer sl.4, Airbnb sl.6, Momentum sl.8–10 hold even after dropping Notion |
| 4 — traction as centerpiece | **Yes, with a caveat** | Buffer is the strong case. But two of seven decks (Airbnb, Uber) were pre-revenue and led with validation/press instead. Traction-first is a *have-traction* pattern, not a universal |
| 5 — explicit unit economics | **Weak — n = 1** | Buffer only |
| 6 — competition as map/matrix | **Yes** | Airbnb 2×2 and Buffer circular map are independent and real. Drop the Notion example (D-grade) |
| 7 — execution credentials on team | **Yes** | Buffer's phrasing is the model |
| 8 — "mic drop" closing | **Weak — probably n = 1** | Front seed and Momentum are both TechCrunch teardowns and the Front order appears derived from Momentum. Likely one observation, not two |
| 9 — no explicit ask slide | **Yes for Airbnb + Buffer; drop Notion** | Note the two modern decks (Front, Momentum) both **do** have an explicit ask. The no-ask pattern is a 2009–2011 artifact, not current practice |
| 10 — simple, minimal | **Advice, not evidence** | June's blog opinion |

---

## 4. WHAT THE DATA SAYS — DocSend-style research (report's findings + audit)

| # | Claim as reported | Cited to | Sample size | Audit |
|---|---|---|---|---|
| 1 | Investors spend an average of **3 min 44 sec** on a deck on first read. "This number has been **remarkably consistent** across DocSend's annual reports" | `pitchgrade.com/blog/...docsend-data` [25], `preuve.ai/blog/startup-pitch-deck-guide` [26] | **None given** | The 3:44 figure is genuine — it is the **2015** DocSend/TechCrunch study. "Remarkably consistent" is unsupported and contradicted by DocSend's own later reports, which show viewing time **declining**. Cited to two SEO blogs while the actual primary sources sit uncited at [41]–[44] |
| 2 | Seed-stage viewing time drops to **under 2 minutes (1 min 56 sec per 2026 data)** | `natlawreview.com/press-releases/investors-spend-under-2-minutes...` [27], `start-wise.io` [28], `keysprung.com` [29] | **None given** | [27] is a **corporate press release** from a company called "Spotlight Startups," published on a press-release wire. Not DocSend, not peer research, no methodology. "2026 data" is a press release's marketing claim |
| 3 | Per-slide attention, labeled **"DocSend 2024 data"**: Team **1:02** · Financials **52s** · Traction **49s** · Problem **38s** · Solution **33s** · Business Model **30s** · Market Size **27s** · Competition **22s** · Product **21s** | `pitchgrade.com` [25] | **None given** | **Internally impossible.** These sum to **5 min 14 sec** — more than claim 1's 3:44 total and nearly triple claim 2's 1:56 seed figure. The numbers are also far above every published DocSend per-slide figure (DocSend's own research reports per-slide times in the ~15–25 second range). **Do not use this table.** The *ordering* (team/financials/traction high, competition/product low) is broadly consistent with DocSend's published findings; the *magnitudes* are not |
| 4 | **Investors spend the least time on product slides** — product is easiest to assess, so they skim it; they linger on market, financials, team | `preuve.ai` [26] | None | Ordering is directionally consistent with real DocSend findings. **Materially important for us:** it argues against building the deck around the demo |
| 5 | **"Company Purpose" slide is a gatekeeper** — investors spend more time on the "why are you doing this" slide, often one sentence on the intro slide, and use it as a filter before reading on | `techcrunch.com/2022/09/22/science-of-pitch-decks` [30], [25] | None | [30] is a genuine TechCrunch article. Best-sourced claim in the section |
| 6 | **Successful vs unsuccessful:** Team slide present in **100%** of decks (both). Financials present in only **~25%** overall, **but none of the failed decks had financials**. Competition slide **often missing from unsuccessful decks** | `techcrunch.com/2022/09/22/science-of-pitch-decks` [30] | **None given** | Genuine TechCrunch source. **This is the most actionable finding in the report:** include financials and include competition. Note the report reproduces the finding without its n |
| 7 | **Funded decks average 10–12 slides** | `preuve.ai` [26] | None | SEO blog. Contradicts DocSend's own published seed-deck length (high teens to ~20) **and contradicts the report's own sample (13/14/19/19/25)**. Do not use |
| 8 | Only **58% of decks are viewed to completion** — nearly half of investors stop before the final slide | `presentations.ai/blog/pitch-deck-statistics` [31] | None | Vendor SEO. Unverified. **If directionally true it is strategically important:** front-load, and never put the ask last |
| 9 | Average time per page drops to about **15 seconds after the first slide** ("**Papermark** data") | `presentations.ai` [31] | None | **Misattribution:** the text credits Papermark; the citation is presentations.ai. Papermark appears nowhere in the source list |
| 10 | **2022 vs 2021:** investors spend **24% less time** on decks; biggest shift is more time on the "purpose" slide and less on product and business model **at pre-seed** | `techcrunch.com/2022/09/22/science-of-pitch-decks` [30] | None | Genuine source. Consistent with #5 |

**The single worst source failure in either report:** the report's trailing list contains the actual primary DocSend research — `dropbox.com/resources/docsend-pitch-deck-research` [41], `dropbox.com/en_GB/resources/docsend-pitch-deck-research` [43], `techcrunch.com/2019/04/12/data-tells-us-that-investors-love-a-good-story` [42], `techcrunch.com/2015/06/08/lessons-from-a-study-of-perfect-pitch-decks-vcs-spend-an-average-of-3-minutes-44-seconds-on-them` [44] — **and cites none of them in the body.** It had the primary sources in hand and quoted `pitchgrade.com` and `preuve.ai` instead. The prompt asked for citations and sample sizes; **not one sample size appears anywhere in the section.**

---

## 5. DECKS I COULD NOT FIND (report's section, preserved)

The prompt asked for explicit honesty here. **The report complied, and this section is the most valuable thing in it.**

| # | Company / category | Report's finding |
|---|---|---|
| 1 | **Linear** | "No publicly available seed or pre-seed deck found. Linear's raise was widely covered, but the deck itself has not been published by the founders or in any aggregator." |
| 2 | **Raycast** | "No publicly available seed deck found. The **$2.7M seed raise from Accel** was covered by TechCrunch, but the deck itself has not been published." (cites `techcrunch.com/2020/10/29/...raycast-raises-2-7m-from-accel`, `raycast.com/blog/hello-world`) |
| 3 | **Loom** | Listed on bestpitchdeck.com ($3M, 2017), "but I could not verify a direct link to the original PDF or confirm it is the real deck vs a redesign." |
| 4 | **Figma** | Listed on bestpitchdeck.com ($3.8M, 2013), same caveat |
| 5 | **Replit** | Listed on bestpitchdeck.com ($120k, 2016), same caveat |
| 6 | **Personal AI / AI memory products** | "**No publicly available seed or pre-seed decks found** for companies in the 'personal AI' or 'AI memory' category (e.g., Mem, Rewind, etc.). These companies have raised, but their decks have not been published." |
| 7 | **Local-first software** | "**No publicly available seed or pre-seed decks found** for companies explicitly positioning as 'local-first' (e.g., Automerge, Electric, etc.)." |
| 8 | **macOS / desktop applications** | "**No publicly available seed or pre-seed decks found** for companies building macOS-first or desktop-first applications (beyond Raycast, which has not published its deck)." |
| 9 | **Genuinely new product category companies** | "Most decks I found are for companies competing in established categories (social media scheduling, ride-hailing, workplace collaboration, etc.). **Decks from companies introducing genuinely new product categories are not publicly available.**" |

**Report's note on aggregators (honest and correct):** "Sites like bestpitchdeck.com, startupfundraising.com, and pitchdecky.com list hundreds of decks, but many require signup to view, and **it is often unclear whether the deck is the real deck used to raise or a redesigned/reconstructed version.** I prioritized decks where the founder or a reputable source (TechCrunch, Business Insider) explicitly stated it was the original file."

**This is a genuine, disciplined negative result and it should drive the deck decision.** Items 6–9 are the four categories Evo sits in, and **all four returned zero artifacts.** There is no deck to copy. Raycast — our single closest analog in product, platform, and buyer — has not published its deck.

**Aggregators and research listed for further study:**

| Resource | URL | Note |
|---|---|---|
| bestpitchdeck.com | `https://bestpitchdeck.com/?stage=Seed` | 850+ decks, filterable by stage. Some require signup |
| startupfundraising.com | `https://startupfundraising.com/library/pitch-deck-examples` | 250+ decks with slide-by-slide breakdowns and embedded PDFs |
| pitchdecky.com | `https://www.pitchdecky.com/stage/seed` | Seed-stage decks |
| **opendeck.app** | (no URL given in body) | **Search slide by slide** — e.g. all "Market" slides, all "Team" slides. Most useful idea in the list; no link provided |
| Slidebean | `https://slidebean.com/pitch-deck-examples` | Airbnb, Uber, Tesla |
| DocSend Startup Index | `https://www.docsend.com/pitch-deck-metrics/` | Weekly pitch deck metrics |
| DocSend Fundraising Playbook | `https://www.docsend.com/startup-fundraising/` | Pre-seed, seed, Series A guides |
| TechCrunch teardowns | `https://techcrunch.com/tag/pitch-deck-teardown/` | Multiple teardowns with full decks embedded — **the highest-yield source in the whole report and the one it mined least** |
| June (founder post) | `https://www.june.so/blog/june-seed-deck` | Real founder post |
| Front (founder post) | `https://www.front.com/blog/...` | **Broken — ellipsis** |
| Buffer | `https://thefounderfolks.com/fundraising-decks/buffer` | **Mislabeled as a "founder post" — it is an aggregator** |

---

## 6. Report Quality Problems

The report was given four source rules and violated three of them.

> "Only include decks you can actually link to. **If you cannot find the artifact, do not describe it from memory or reputation.**"
> "Clearly separate 'real deck used to raise' from 'redesigned version' from 'template inspired by.'"
> "**Do not substitute pitch-deck-advice blog content for actual decks.** I want primary artifacts."
> "I would rather have five real decks than twenty summaries."

### 1. The honest count: 7 claimed, 4 defensible, 1 of those is advice

| Grade | Decks | Count |
|---|---|---|
| **A** — artifact retrieved, working credible link | Momentum | **1** |
| **B** — real historical artifact, aggregator/synthetic-CDN links only | Airbnb, Buffer, Uber | **3** |
| **C** — described from a summary of a *different* document; artifact not retrieved | Front (seed) | **1** |
| **D** — mislabeled; contents contradict stated stage/date | Notion | **1** |
| **E** — founder post retrieved, but advice presented as slides | June | **1** |

**One deck (Momentum) meets the bar the prompt actually set.** Three more (Airbnb, Buffer, Uber) are real, famous, historical artifacts whose contents are broadly right but whose *links* fail the rule. **Two entries (Front seed, Notion) are described from summary or reputation — precisely the prohibited behavior.** One (June) is prohibited advice substitution.

Restated for the deck decision: **we have 1 clean modern deck, 3 historical decks from 2008–2011, and 0 decks from our stage, our platform, or our category.**

### 2. Two "links" contain literal ellipses and do not resolve

- `https://www.scribd.com/document/.../UberCab_Dec2008`
- `https://www.front.com/blog/...`

Both are presented in a bulleted "Where available" list as if they were links. The Front one is the **company's own blog post**, the highest-value primary source the prompt asked for by name, and it is a placeholder. A third entry — Airbnb's — is not even a URL: "SlideShare and various aggregators (**search 'Airbnb pitch deck 2009 PDF'**)."

### 3. Both "Direct PDF" links live on the same synthetic-looking CDN

`media.genppt.com/pitch-decks/buffer/buffer-pitch-deck-2011.pdf` and `media.genppt.com/pitch-decks/uber/uber-pitch-deck-2008.pdf`. "genppt" reads as *generated PowerPoint*; the path structure is machine-templated (`/pitch-decks/<company>/<company>-pitch-deck-<year>.pdf`). A domain that programmatically serves "original" PDFs for arbitrary companies is exactly where reconstructions live. **Verify both files open and match the known originals before trusting a single slide description derived from them.**

### 4. Extensive SEO / template-farm sourcing, and 34 sources of banned advice content

Aggregator and SEO domains carrying deck descriptions: `pitches.ai` (Airbnb, Buffer, + Uber *template*), `spectup.com`, `thefounderfolks.com`, `bestpitchdeck.com`, `roundfunded.com`, `pitchdeckhunt.com`, `datapile.co`, `upmetrics.co`, `vcmatch.ai`, `pitchdecky.com`, `flowjam.com`. Note **[16] is `pitches.ai/templates/uber-pitch-deck`** — a *template* page cited as the authority for Uber's **25-slide count**. Templates are the third category the prompt asked to be kept separate, and it was used as evidence.

Data claims rest on `pitchgrade.com`, `preuve.ai`, `start-wise.io`, `keysprung.com`, `presentations.ai`, `zyner.io`, `natlawreview` (press release), `seedscope.ai`, `seedangels.ai`, `deckary.com`, `whitepage.studio`, `qubit.capital`.

**Sources 50–83 (34 of 83) are pure pitch-deck-advice and template content** — UCSD and UVic template PDFs, `stripe.com/guides/atlas/pitchdeck`, `underscore.vc`, `nextview.vc`, `visible.vc`, `antler.co`, `visme.co`, `verycreatives.com`, `runwayteam.co`, `ewor.com`, two `deckary.com` guides, two `notion.com` template pages, two Notion-hosted resource lists, three LinkedIn posts, `failory.com`, `seedlegals.com`. None is cited in the body. **They are padding, and they are padding with the one content type the prompt explicitly banned.**

### 5. The Front seed deck order is probably contaminated from Momentum

Front seed (~11): Cover → Problem → Solution → Product → Why now → Traction → Validation → Market → Team → Ask → Mic drop
Momentum (19): Cover → Problem(3) → Solution(3) → Product(3) → Why now → Traction → Validation → Market → Team → Ask → Demo → Mic drop → Thank you

Identical sequence, identical unusual vocabulary ("why now," "validation," "mic drop"). Both are attributed to TechCrunch teardowns. The Front seed deck was never retrieved, and the report labels its order "approximate, from TechCrunch summary." **The most probable explanation is that Momentum's structure was transposed onto Front.** Consequence: pattern #8 ("mic drop" closing) has **n = 1, not 2**, and the comparative table contains what is effectively a duplicate row.

### 6. Notion is anachronistic and should be removed from the sample

A "$2M seed deck, 2013 (some sources say 2016)" that cites "over a million users," names Lattice as a customer, and carries an enterprise SSO / dedicated-support appendix. Those facts are from 2019–2020. Team and Ask are both recorded as absent, which is what a sales deck looks like, not a fundraising deck. **The report could not date it, could not find a primary source, and did not question the contradiction.** Discard.

### 7. Uber's breakdown covers 7 of 25 slides

Slides 8–25 are one row reading "Various: financial projections, team, rollout plan, technology details… etc." **72% of the deck is unexamined**, while the entry is presented as a slide-by-slide breakdown with a confident 25-slide count sourced to a *template* page. The report also says the deck was shared "on the 9th anniversary of Uber's idea (**August 2008**)" — impossible; Camp shared it in 2017.

### 8. "Actual headline text where you can read it" is mostly absent

Real quoted headlines appear for Airbnb (~8 slides), Buffer (~10 slides), and Momentum (1 slide). Uber has 3 approximate ones. **Front has zero. June has zero. Notion's are paraphrase.** Across the sample, well under half the slides have the verbatim text the prompt requested.

### 9. Zero coverage of the priority categories

The prompt named the categories to prioritize: prosumer/consumer software subscriptions, developer tools, personal AI and AI memory, local-first software, macOS/desktop applications, and genuinely-new-category companies.

Delivered: two marketplaces (Airbnb, Uber), one social-scheduling SaaS (Buffer), two B2B SaaS (Front, Momentum), one product-analytics tool (June), one workplace-collaboration tool (Notion). **Zero from any priority category. Zero pre-seed decks** (Uber is retro-labeled pre-seed). **Only one deck from after 2016; five of seven are 2008–2013.** To the report's credit, its "could not find" section explains *why* — the categories genuinely have no public artifacts — but it should have said that up front instead of filling the slot with 2008 marketplace decks.

### 10. Internal numeric contradictions

- Momentum: heading **$5M**, ask slide **$4M**
- Per-slide times sum to **5:14** vs. a stated **3:44** average and a **1:56** seed average
- "Funded decks average **10–12 slides**" vs. the report's own sample of **13, 14, 19, 19, 25**
- Notion: "**2013** (some sources say **2016**)" with 2019-era contents
- Front seed: "~11 slides" from a **Series D** teardown

### 11. Half the source list is uncited

Body cites [1]–[40]. **Sources 41–83 (43 of 83, 52%) never appear inline.** The list overstates research depth by roughly 2×, and the uncited half is where the actual DocSend primary research was stranded.

---

## 7. What to actually take into the deck-structure decision

**The load-bearing finding is the negative one.** No pre-seed/seed deck exists publicly for personal AI, AI memory, local-first software, macOS-first desktop apps, or genuinely-new-category companies. Raycast — same platform, same buyer, same prosumer-subscription model — has not published its deck. **There is no template to imitate; the structure has to be reasoned from first principles plus the four historical artifacts.**

### Findings I would rely on

| Finding | Source strength | Implication |
|---|---|---|
| Team, financials, and traction get the most investor attention; **product gets the least** | DocSend ordering is real even though the magnitudes here are wrong | Do not build the deck around the demo. Product earns one screenshot slide, early, then move on |
| **None of the failed decks had a financials slide**; competition slide is often missing from unsuccessful decks | TechCrunch "science of pitch decks" [30] — genuine source | Include both. Non-negotiable |
| The "why are you doing this" / purpose line is used as a **gatekeeper filter** before investors read on | TechCrunch [30] | Slide 1 carries a single-sentence purpose. Earn the second slide |
| Roughly **half of readers never reach the last slide** | Weak source, but directionally plausible | Front-load. Never park the ask at the very end alone |
| **Product screenshots early** — Buffer sl.4, Airbnb sl.6, Momentum sl.8–10 | Three independent real artifacts | Show the running Mac app inside the first third |
| **Competition as a map/matrix in an empty quadrant**, not a feature checklist | Airbnb 2×2 and Buffer circular map, both real | Our observe/classify/restore axes map naturally onto a 2×2 — and per digest 07, nothing occupies the restoration quadrant |
| **Execution credentials, not résumés**, on the team slide | Buffer's "idea to revenue in 7 weeks" / "200 to 55K users" | State what we shipped and how fast |
| **Split problem/solution/product across multiple slides for async reading** | Momentum, the one A-grade artifact, did exactly this and the founder said why | Decks are read alone, not narrated. Build for the silent read |
| **Explicit, specific ask**: "We are raising $4M. That will give us 18 months of runway…" | Momentum, verbatim | Both modern decks have an explicit ask; only the 2009–2011 decks omit it. Include ours |
| **Headlines-only story test** | June's founder | Read only the headlines top to bottom — does it tell the story alone? Cheap and worth doing |

### Findings I would not rely on

- The per-slide seconds table (arithmetically impossible)
- "Funded decks average 10–12 slides" (contradicted by the report's own sample and by DocSend)
- "Investors spend 1:56 at seed as of 2026" (a vendor press release)
- "58% completion" and "15 seconds per page" (vendor SEO, misattributed to Papermark)
- Anything derived from the Notion entry
- The "mic drop" pattern as a two-deck pattern (probably one)

### Verification tasks before locking structure
1. Open the **Momentum** TechCrunch teardown directly — it is the only clean artifact and it deserves a full read rather than this second-hand summary.
2. Read **`june.so/blog/june-seed-deck`** as what it is: a founder's essay on deck construction. The "vertebrae" framing and the headlines-only test are usable; the "slide breakdown" in this digest is not June's deck.
3. Pull the **real DocSend research** at `dropbox.com/resources/docsend-pitch-deck-research` and the 2015 TechCrunch study. Get actual per-slide times and actual sample sizes. The report had these links and used SEO blogs instead.
4. Mine **`techcrunch.com/tag/pitch-deck-teardown/`** directly. It embeds full real decks with founder commentary, it is the highest-yield source named, and the report pulled only two teardowns from it.
5. Try **opendeck.app**'s slide-by-slide search for "competition" and "team" slides from recent prosumer/dev-tool seed rounds — that is the closest available substitute for the category decks that do not exist.
6. Verify the two `media.genppt.com` PDFs are genuine originals before citing any Buffer or Uber slide detail.
