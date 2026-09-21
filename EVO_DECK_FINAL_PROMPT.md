# Evo — Final Deck Revision Prompt (hand this to the AI that built the deck)

**Read this whole prompt before touching a slide.** You already produced a 21-slide Evo pitch deck (black background, off-white body text, yellow accents, bold condensed display headlines). That draft is *structurally* good but it is not investor-ready: it still wears a "DRAFT — needs founder attention" banner on almost every slide, several slides (Market, Business Model, Team) never got their actual content and render as blank underlines, two slides use an irrelevant stock delivery-clipboard photo, and the raise, pricing, and market numbers are still placeholders.

Your job now: **conform the existing deck to the exact target state specified below, slide by slide.** The words in this document are the final words — use them verbatim unless a line is wrapped in `⟨FOUNDER FILL⟩`, which is one of a small number of real-world measurements only the founder can supply. Keep those `⟨FOUNDER FILL⟩` placeholders styled exactly as written and change nothing else about them.

Do **not** invent user counts, retention numbers, waitlist size, revenue, demo view counts, investor interest, or founder credentials. Every number that is *not* in this document as a decided figure must stay a `⟨FOUNDER FILL⟩` placeholder. Fabricated traction is the single fastest way to fail investor diligence — leave it blank rather than guess.

---

## PART 1 — GLOBAL CHANGES (apply to every slide)

1. **Delete the draft banner everywhere.** Remove the yellow top-right line "DRAFT — this slide needs founder attention before this deck is shown to an investor" from *all* slides. It must not appear once. The running header keeps only the small section label in the top-left (e.g. `PROBLEM`, `MARKET`).

2. **Remove all inline draft meta-notes** — anything of the form `[[DRAFT — CONFIRM: …]]`, `[[BLOCKED: …]]`, `[[UNVERIFIED: …]]`, and my own notes-to-self ("Unblocked by…", "needs a founder decision", etc.). Where the note pointed at real content to add, that content is written out below. Where it was a reminder to the founder, it has moved to the internal punch list (Slide 21) and must not appear on the investor-facing slide.

3. **Keep only `⟨FOUNDER FILL⟩` placeholders** as visible unfilled items, and style them quietly — same off-white body text, not yellow, no brackets-as-alarm. They should read as "one number to drop in," not "this deck is unfinished." There are only six of them and they are listed in Part 4.

4. **Replace both stock photos.** The delivery-clipboard photo on the "What Evo does" slide and the "Demo" slide is irrelevant and reads as filler. Replace with a real product visual: a screenshot or still frame of Evo restoring a project, OR — if no clean asset exists yet — a simple typographic diagram (`files · tabs · windows · apps → one project → restored in one click`) on the black background. Never a stock photo of people.

5. **Fill the three blank slides.** Market (Slide 7), Business Model (Slide 9), and Team (Slide 12) currently show empty underlines where content should be. Populate them exactly as specified in Part 3.

6. **House style stays.** Black background, off-white body, one yellow accent, bold condensed all-caps display headlines, generous whitespace. Do not redesign — just finish and clean.

7. **Slide count and order stay the same** (Title → Problem → What Evo does → Demo → Why now → Defensibility → Market → Competition → Business model → Go-to-market → Traction → Team → The ask → Appendices A–E → Punch list). If a page 22 (closing/contact) exists, give it the same clean header treatment; do not invent new content for it.

8. **One honesty rule overrides everything:** if following an instruction below would state something as fact that the founder has not measured, keep it as a `⟨FOUNDER FILL⟩` placeholder instead.

---

## PART 2 — SLIDE-BY-SLIDE COPY (verbatim)

### Slide 1 — Title
- **Headline (fix the wording — this is the single most important copy change in the deck):**
  `EVO GETS YOU BACK INTO ANY PROJECT — EVERY FILE, TAB, AND WINDOW — EXACTLY WHERE YOU LEFT OFF.`
  (The draft says "PUTS YOU BACK INTO ANY WORK." Change "PUTS" → "GETS" and "ANY WORK" → "ANY PROJECT." This is the founder's resolved positioning line and must match exactly.)
- **Subhead:** `Local-first. On-device. macOS.`
- **Raise line (was "Raising $X via Y"):** `Pre-seed · Raising $1.5M on a post-money SAFE`
- **Footer (keep):**
  `Bhavit Saini · Founder · bhavitforwork@gmail.com`
  `Adie Stevens · Co-Founder · adie.stevens@mymail.champlain.edu`

### Slide 2 — Problem
- **Header label:** `PROBLEM`
- **Headline (keep):** `YOUR COMPUTER FORGETS YOUR WORK THE MOMENT YOU STOP WORKING.`
- **Quotes block — keep ONLY if these are genuinely real user quotes.** They are currently attributed to "Reddit Users." If they are verbatim from real posts, keep them and be ready to show the source. If they are paraphrased or illustrative, change the attribution to `— how users describe it` (drop "Reddit"), because an unsourced named attribution is a diligence risk. Quotes:
  `"The task is still there. But the thread is gone."`
  `"The apps are still open. I just can't remember which tabs, or where I left off."`
- **Closing lines (keep):**
  `Your tabs, files, apps and documents are still there.`
  `What disappears is the context connecting them.`

### Slide 3 — What Evo does
- **Header label:** `WHAT EVO DOES`
- **Headline (keep):** `ONE CLICK, AND THE WHOLE PROJECT COMES BACK.`
- **Sub-headline (keep):** `Remember what you want to do. Evo handles the rest.`
- **Body (keep):**
  `Evo learns which files, tabs, and apps belong to which project. No tagging, no manual saving, no setup ritual.`
  `Everything stays on your Mac, encrypted. No account required.`
- **Visual:** replace the stock photo per Global Change 4.

### Slide 4 — Demo
- **Header label:** `DEMO`
- **Headline (keep):** `THIRTY SECONDS IS ENOUGH TO SEE IT.`
- **Body (keep):** `Watch us close an entire project, then restore it in a single action.`
- **Visual + link:** replace the stock photo with a real still frame from the demo, and set the link to `⟨FOUNDER FILL: demo video URL⟩`. If no still exists yet, use the typographic diagram from Global Change 4 — not the stock photo.

### Slide 5 — Why now
- **Header label:** `WHY NOW`
- **Headline (keep):** `WATCHING YOUR SCREEN JUST BECAME TABLE STAKES. UNDERSTANDING YOUR WORK DIDN'T.`
- **Body (keep, verbatim):**
  `Between July 29 and August 29, 2026, nine products shipped in this space — including OpenAI's, on August 13. Recording what you did is now a commodity.`
  `Not one of them classifies your activity into projects and restores a working state. They search and summarize; they don't put you back to work.`
  `On-device models now make that classification possible locally — and after Recall, many people will only install the version that runs on their own machine.`

### Slide 6 — Defensibility
- **Header label:** `DEFENSIBILITY`
- **Headline (keep):** `RECORDING IS EASY. KNOWING WHAT BELONGS TOGETHER IS THE HARD PART.`
- **Body (keep):**
  `The hard engineering is classification — deciding which of thousands of daily actions belong to the same body of work — then restoring native application state reliably enough to trust.`
  `Defensibility is threefold: classification and restoration quality, user trust in a product that never sends work off the machine, and the context that accrues per user.`
  `Local-first means we hold no user data and gain no cross-user data advantage. Our moat is execution quality, trust, and per-user switching cost — not a data asset.`

### Slide 7 — Market  *(currently blank — fill from Part 3A)*
- **Header label:** `MARKET`
- **Headline (keep):** `A WEDGE WITH A PRECISE FIRST USER, AND A PATH PAST IT.`
- **Lead line (keep):** `Evo is the continuity layer for how you work — it starts with restoration, and the same understanding expands into anticipation and workflow automation. Local-first by design.`
- **Beachhead line (new — replaces the vague "needs a founder decision"):**
  `We start with the people who feel this most: senior and staff engineers on macOS who juggle 3+ active projects — the ones already living in 40 tabs and a dozen windows per project.`
- **Insert the bottom-up funnel and the scenario table from Part 3A** where the blank underlines are now.
- **Expansion note (keep the yellow sidebar, "EXPANSION PATH"):** `Teams and shared project context, then cross-device continuity, then Windows. The venture case lives here — not as an afterthought.`
- Delete the bottom line "Unblocked by developer-tooling comparables research plus a founder decision on the beachhead."

### Slide 8 — Competition
- **Header label:** `COMPETITION`
- **Headline (keep):** `EVERYONE REMEMBERS. NOBODY PUTS YOU BACK TO WORK.`
- **2×2 map (keep exactly):** vertical axis `On-device` (top) ↔ `Cloud` (bottom); horizontal axis `Remembers & searches` (left) ↔ `Restores working state` (right). Evo = yellow diamond, top-right quadrant. Keep the other plotted points (Raycast, Skill Recorder, Rewind (wound down), Spotlight & recents, Tab/session managers, Microsoft Recall, OpenAI ChatGPT Computer History) where they are.
- **Apple line (keep):** `Apple has shipped search, recents, and Spotlight actions — but no project-level restoration of application state. A risk to outrun, not to dismiss.`

### Slide 9 — Business model  *(currently blank — fill from Part 3B)*
- **Header label:** `BUSINESS MODEL`
- **Headline (keep):** `FREE WHERE IT RUNS ON YOUR MACHINE. PAID WHERE IT FOLLOWS YOU.`
- **Precedent line (keep):** `Raycast ran free for about three years — Oct 2020 beta to May 2023 — raising a $2.7M seed and a $15M Series A before charging. Same user, same platform.`
- **Insert the three pricing tiers from Part 3B** where the blank underlines are now.
- **Keep the yellow "LATER" sidebar:** `Team tier — Shared project context for teams. Comes later.`
- **Distribution line (keep):** `Direct distribution only — Accessibility permissions rule out the Mac App Store. No 15–30% platform cut, and a direct customer relationship.`

### Slide 10 — Go to market
- **Header label:** `GO TO MARKET`
- **Headline (keep):** `THE PEOPLE WITH THIS PROBLEM ARE ALREADY IN A FEW ROOMS.`
- **Body (keep):**
  `Curated waitlist with staged invites — Linear's model: handpick from survey responses, ~10 invites a week at the start.`
  `High-touch, deliberately unscalable onboarding for the first cohorts — the fastest way to learn what makes someone keep it.`
  `Developer and technical communities where this is discussed, plus a Show HN launch. Product Hunt as an amplifier later, not a cold start.`
  `Direct distribution, no app-store gatekeeper.`
- **Closing line (fill the waitlist blank):**
  `A waitlist measures topic interest, not purchase intent. At ⟨FOUNDER FILL: waitlist size⟩ signups, the first paying cohort is realistically in the low tens — activation is the number that matters.`

### Slide 11 — Traction & evidence
- **Header label:** `TRACTION & EVIDENCE`
- **Headline (keep):** `WHAT WE KNOW SO FAR.`
- **Rewrite as a confident block — lead with what is true, and reduce the five blanks to the four measurements only the founder has.** Instruction: keep any line whose number the founder can fill; if a number is genuinely not measured yet, delete that line rather than show an empty placeholder (a tight three-line traction slide beats a five-blank one).
  `A working macOS product, shipped and in daily use — ⟨FOUNDER FILL: number of machines running Evo daily, and for how long⟩.`
  `Fully local, encrypted, no account — nothing leaves the device.`
  `Early retention — ⟨FOUNDER FILL: D7 and D30 retention for that cohort, if measured⟩.`
  `⟨FOUNDER FILL: waitlist size, its source, and activation rate⟩.`
  `In users' words — ⟨FOUNDER FILL: 2–3 verbatim quotes with roles⟩.`

### Slide 12 — Team  *(currently blank — fill below)*
- **Header label:** `TEAM`
- **Headline (keep):** `WE BUILT THIS BECAUSE WE NEEDED IT.`
- **Two founder blocks (fill names, keep credentials as founder fill):**
  `Bhavit Saini — Founder. ⟨FOUNDER FILL: one line — prior role / relevant systems or macOS experience / why you⟩`
  `Adie Stevens — Co-Founder. ⟨FOUNDER FILL: one line — prior role / relevant experience / why you⟩`
- **Under both, a line that is true today (keep):** `Two technical founders who designed and shipped the working product themselves. Local-first by conviction, not by feature choice.`
- **Origin story (proposed draft — founder to make specific and true; keep as body text, not a placeholder):**
  `We kept losing our place. Switching between projects meant rebuilding the same tab-and-window state from memory, every time — so we built the tool that does it for us. Evo is the thing we needed and couldn't buy.`
- Delete the "no dedicated go-to-market hire / who owns distribution" note from the slide face — it moves to the punch list.

### Slide 13 — The ask  *(fill all numbers)*
- **Header label:** `THE ASK`
- **Headline (replace the placeholder headline entirely):**
  `RAISING $1.5M TO PROVE PEOPLE KEEP IT — AND PAY FOR IT.`
- **Body (three lines, verbatim):**
  `Amount & instrument — $1.5M on a post-money SAFE, $10M cap.`
  `Use of funds — Engineering & product 60%, go-to-market & community 15%, design 10%, ops, legal & infrastructure 15%.`
  `Runway & milestone — ~20 months to the seed bar: a working paid funnel and retention strong enough that growth is organic. Targets: D30 retention ~40%, free-to-paid ~5–6%, a paying base in the hundreds, distribution driven by the developer communities where the problem lives.`

---

## PART 3 — CONTENT FOR THE THREE BLANK SLIDES

### 3A — Market slide funnel + scenario table (Slide 7)

Render this as a clean left-to-right funnel, then a small three-column scenario table beneath it. All figures are labeled estimates — keep the caveat line.

**Bottom-up funnel:**
```
~30M+ professional developers worldwide        (industry estimates, e.g. SlashData)
        ↓ ~33% on macOS                         (Stack Overflow Developer Survey 2024)
~10M Mac developers
        ↓ ~30% senior / multi-project           (our beachhead assumption, to validate)
~3M reachable first users
```

**Scenario table (ARPU $120/yr):**

| Penetration of beachhead | Paying users | ARR |
|---|---|---|
| 1% | ~30,000 | ~$3.6M |
| 5% | ~150,000 | ~$18M |
| 10% | ~300,000 | ~$36M |

**Caveat line (must appear, small, under the table):**
`Illustrative bottom-up from public anchors; penetration scenarios, not forecasts. Teams pricing and cross-device/Windows expansion widen this well beyond the Mac-developer beachhead.`

### 3B — Pricing tiers (Slide 9)

Render as three tiers, left to right. The paid hook rides the roadmap's encrypted cross-device sync — **do not** describe the paid tier as paying for local-only features (there is no precedent for a pure-local subscription; the recurring charge must buy sync/AI/team).

| Free | Pro — $12/mo or $120/yr | Teams — $20/user/mo *(later)* |
|---|---|---|
| One Mac. Full local capture, classification, and one-click restoration. No account — nothing leaves the device. | Encrypted cross-device continuity, priority support, and power features. Your work follows you, still end-to-end encrypted. | Shared project context across a team. Ships after the individual product is proven. |

### 3C — (reference) Use-of-funds figures for Slide 13
$1.5M total → Engineering & product $900K (60%) · Go-to-market & community $225K (15%) · Design $150K (10%) · Ops, legal & infrastructure $225K (15%). Headcount 2 → 4–5 (1–2 senior macOS/systems engineers, 1 design-leaning product generalist; go-to-market stays founder-led through this round).

---

## PART 4 — APPENDIX SLIDES (clean, keep, minor fixes)

These are genuine investor assets — keep them. Only strip the meta-notes and soften the two unverified tags.

- **Slide 14 — "If Apple ships 'search everything I did on my Mac' for free…"** Keep the answer verbatim. In the last line, change `[[UNVERIFIED: final shipped cadence]]` to a clean parenthetical: `(cadence relaxed before release)`. Keep "A risk to outrun, not eliminate."
- **Slide 15 — "Why is this venture-scale rather than a beloved $2–25M ARR Mac utility…"** Keep the answer verbatim, including the honest ceiling and the Superhuman/Obsidian/Arc/Craft lines. Delete the yellow note `[[DRAFT — CONFIRM: this is the deck's central tension…]]`. Being forward about the ceiling is correct — the slide already does it.
- **Slide 16 — "Why is this not Rewind again…"** Keep verbatim. Keep the pricing-caution line (usage paywall backfired → switched to unlimited).
- **Slide 17 — "The landscape, dated" (competitive table).** Remove the header note `DRAFT — [unverified] rows to confirm…` and remove the inline `[unverified]` tags next to Microsoft Skill Recorder and Hansel. The "what it does" descriptions already avoid claiming those two restore state, so the table stays honest; the sources appendix (Slide 20) already notes they're unconfirmed.
- **Slide 18 — Privacy, permissions & legal.** Keep the posture verbatim, including `We do not claim local-only processing removes GDPR obligations.` Change `[[UNVERIFIED: final shipped cadence]]` to `(relaxed before release)`. Delete the `[[BLOCKED: four privacy-law questions need counsel…]]` note — it moves to the punch list.
- **Slide 19 — Outcomes.** Keep `$0–50M` and the body verbatim, including `use $488.3M`. This is the correct, honest exit framing.
- **Slide 20 — Sources & confidence.** Keep verbatim. This is a strength — leave the confidence labels (HIGH / MEDIUM / est.) in.
- **Slide 21 — Punch list.** **Mark this slide `INTERNAL — DELETE BEFORE SENDING TO INVESTORS`** at the top. It is the founder's private to-do list, not an investor slide. Update it to reflect that the raise, pricing, market math, and beachhead are now decided (see Part 5), so the only open items are the six founder fills plus: get legal counsel on the four privacy-law questions before any written legal claim; decide who owns distribution and the first hire; confirm the Reddit quotes are real or relabel them.

---

## PART 5 — ⟨FOUNDER FILL⟩ — THE ONLY SIX THINGS I DID NOT DECIDE FOR YOU

Everything else in this deck is now decided and filled. These six are real-world measurements or identity facts — inventing them would be lying to investors and would collapse in diligence, so they are the only placeholders left. Each has guidance:

1. **Demo video URL + one still frame** (Slide 4). Drop the link; pull one clean frame of a real restoration for the visual.
2. **Machines running Evo daily, and for how long** (Slide 11). State the true number even if small — "6 machines, daily, for 8 weeks" is more fundable than a vague claim. If it's just the two of you, say so and lead with retention instead.
3. **D7 / D30 retention for that cohort** (Slide 11). If measured, state it. If not, delete that line — do not estimate it.
4. **Waitlist size, source, and activation rate** (Slides 10 & 11). Use the real signup count at pitch time. At ~1,000, a ~3% pre-launch conversion implies a first paying cohort of ~30 — the deck already frames it that way honestly.
5. **2–3 verbatim user quotes with roles** (Slide 11). Real quotes from real early users, with their role (e.g. "staff engineer"). Do not write these yourself.
6. **Founder credentials — one line each** (Slide 12). Prior role, relevant systems/macOS experience, why you two specifically. Do not inflate. See the note below.

---

## PART 6 — NOTE TO THE FOUNDER (Bhavit) — READ BEFORE YOU SEND THIS PROMPT

I acted as founder and made the calls you asked me to. Here's what I decided and why, so you can sanity-check each one:

- **Raise: $1.5M on a post-money SAFE at a $10M cap (≈15% sold).** This is a standard, credible US pre-seed structure for two technical founders with a working, pre-revenue product. 15% dilution is dead-center for the stage, and it's the same assumption my exit math uses. **If you have real traction to show, a $12M cap (≈12.5%) is the defensible stretch — swap one number.** I did *not* go higher: with no disclosed traction yet, a bigger cap invites pushback you don't need.
- **Use of funds & 20-month runway:** engineering-heavy (60%) because classification + restoration quality *is* the moat; 10% design is deliberate — for a Mac prosumer/dev tool, polish is part of the moat and a selling point.
- **Pricing: Free / Pro $12/mo ($120/yr) / Teams $20/user/mo later.** The Pro hook is encrypted cross-device sync, not local features — that's the one pricing rule the research is firm on (no pure-local subscription has ever worked; the charge has to follow the user across devices).
- **Beachhead: senior/staff macOS engineers with 3+ active projects.** This is the precise first user the deck was missing. The market funnel (30M → 10M → 3M) is built from public anchors and labeled as estimates — please gut-check the "~30% senior/multi-project" assumption against what you actually see in your waitlist.
- **The origin story on Slide 12 is a draft I wrote in your voice.** Make it true and specific — it's the highest-leverage line on the team slide.
- **On credentials:** I deliberately left these blank rather than invent a pedigree. If either of you is early-career, don't paper over it — "we shipped a working macOS product that does something Apple hasn't" is a stronger signal than a thin resume. Consider a real domain email (e.g. name@evo.app) over the personal/edu addresses before you send.
- **What I refused to fill:** waitlist size, active machines, retention, user quotes, demo views. Those are the six items in Part 5. I know you asked for exact numbers everywhere — but these five are measurements of *your* product, and a made-up traction number is the one thing that reliably kills a raise once an investor asks a follow-up. Fill them with the truth and the deck is done.
