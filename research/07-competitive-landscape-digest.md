# 07 — Competitive Landscape Digest

**Source artifact:** `uploads/You are a competitive intelligence analyst. I need.pdf` (Perplexity research report)
**Total pages:** 12. Page 1 = the original prompt verbatim. Pages 2–9 = body (~8 pages). Sources begin mid-page 9 and run to page 12 (~3.5 pages, **150 numbered URLs**).
**Report self-timestamp:** "Sources timestamped as of August 31, 2026. This market moves quickly — revisit in 90 days."
**Digested:** 2026-08-31. Permanent record for the competition slide.

**Body citation range:** inline refs run [1]–[107]. **Sources 108–150 (43 of 150, 29%) are never cited in the body** — they are padding.

---

## 1. Report's own Executive Summary (verbatim substance)

> "The market for local-first, activity-aware desktop applications is heating up rapidly. As of August 2026, the most significant development is **OpenAI's Computer History feature (launched August 13, 2026)**, which directly competes with your value proposition by tracking Mac activity across apps and websites. Microsoft's Recall has been redesigned and is shipping opt-in on Copilot+ PCs. Rewind AI pivoted to hardware (Limitless Pendant) and was acquired by Meta in December 2025, shutting down the Mac app. Apple has shipped incremental improvements but no direct competitor yet. Several Y Combinator batches and open-source projects are emerging in this space."

---

## 2. Full competitor comparison table (as reported)

Columns preserved from the report. "Last meaningful update = August 2026" with no event named means the report is recording *when it checked*, not a release — see Report Quality Problems.

| Product | Company | What it does | Local or cloud | Platform | Price | Funding raised | Current status | Last meaningful update |
|---|---|---|---|---|---|---|---|---|
| Microsoft Recall | Microsoft | Screenshots every few seconds, searchable timeline of activity; redesigned with encryption, Windows Hello auth, VBS enclave | Local (encrypted) | Windows 11 (Copilot+ PC required) | Free with Windows | N/A (Microsoft) | Shipping opt-in after redesign; delayed broad rollout due to security concerns | Aug 2026 security patches; feature active on Copilot+ PCs |
| **ChatGPT Computer History** | OpenAI | Records clicks, keystrokes, app switches via macOS Accessibility API; builds searchable timeline; processes on OpenAI servers (48hr retention) | Cloud processing (temp), local storage | macOS (ChatGPT desktop app) | ChatGPT Pro/Business/Enterprise only | N/A (OpenAI) | **Launched August 13, 2026**; opt-in; not available EEA/Switzerland/UK | Aug 14, 2026 launch |
| Rewind AI (Mac app) | Rewind AI / Limitless | Screen + audio recording, searchable timeline, AI summaries | Local | macOS | Was ~$10–20/mo | ~$30M (Series B, 2023) | **Shut down December 19, 2025; kill switch activated** | Dec 2025 shutdown |
| Limitless Pendant | Limitless (ex-Rewind) | Wearable pendant records/transcribes conversations; 100hr battery, beam-forming mics | Cloud (encrypted) | Hardware + iOS/Android | $99 one-time | ~$30M | **Acquired by Meta December 2025**; sales ceased; team → Reality Labs | Dec 2025 acquisition |
| Workona | Workona | Browser tab workspace manager; saves/restores tab sessions by project | Cloud sync | Chrome, Edge, Safari, Firefox | Free (5 workspaces); Pro ~$7–8/mo | Unknown | Active; tightened free tier limits 2024–2025 | Aug 2026 pricing verified |
| Mem | Mem | AI note-taking; auto-organizes notes; Mem Agent for proactive follow-up | Cloud | Web, macOS, iOS, Windows | Free (25 notes/mo); Pro $12/mo; Proactive $99/mo | ~$130M (Series C, 2023) — **suspect, see QA** | Active; Mem 2.0 Oct 2025; Mem Agent Aug 2026 | Aug 11, 2026 (Mem Agent launch) |
| Reflect | Reflect | AI journaling/notes; E2E encrypted; daily notes with AI chat | Cloud (E2E encrypted) | macOS, iOS, Web | $10/mo (annual); 14-day trial | Unknown | Active | Aug 2026 pricing verified |
| Heptabase | Heptabase | Visual note-taking with whiteboards; AI agent, tutor, transcription | Cloud | Web, macOS, Windows, iOS, Android | Pro $11.99/mo; Premium $23.99/mo; Premium+ $71.99/mo | Unknown | Active; added Premium tiers Dec 2025 | Aug 2026 pricing verified |
| Granola | Granola | AI note-taking for meetings; transcribes, summarizes, action items | Cloud | Web, macOS, Windows | ~$10–14/mo | **Unknown — research failure** | Active | Aug 2026 |
| Dia | The Browser Company (Atlassian) | AI-native browser; Skills, Memory, Slack/email integration, tab groups, task suggestions | Cloud sync | macOS (GA); Windows beta waitlist | Free; enterprise tiers in progress | $610M acquisition (Atlassian, Oct 2025) | Active; **Arc in maintenance mode since May 2025** | Aug 2026 weekly releases |
| Raycast | Raycast | Keyboard-first launcher; AI chat, commands, agents, memory, **screen awareness** | Cloud sync (Pro) | macOS, Windows (beta) | Free; Pro $8/mo (annual); Advanced AI +$8/mo | **Unknown — contradicted by Report 2** | Active; **v2.0 launched August 2026 out of beta** | Aug 25, 2026 (v2.0 GA) |
| Session (Session Buddy) | Session | Browser tab/session manager for Chrome | Local | Chrome extension | Free; donations | Unknown | MV2 shutdown; support issues reported | Aug 2026 (MV3 replacement needed) |
| Tab Session Manager | Various | Saves/restores browser tab sessions | Local | Chrome, Firefox, Safari extensions | Free; some paid ~$2.99/mo | N/A | Active; multiple variants | Aug 2026 |
| Shift | Shift | Unified inbox; email, calendar, apps in one place | Cloud | macOS, Windows, Linux, Web | Free; Pro ~$9/mo | Unknown | Active | Aug 2026 |
| Rambox | Rambox | App launcher; organizes web apps and desktop apps | Local | macOS, Windows, Linux | Free; Pro ~$5/mo | Unknown | Active | Aug 2026 |
| Sunsama | Sunsama | Daily planner; guided rituals, manual time blocking, calendar integration | Cloud | Web, macOS, Windows, iOS, Android | $22/mo; $204/yr ($17/mo) | Unknown | Active; independent | Aug 2026 pricing verified |
| Motion | Motion | AI auto-scheduling; calendar, tasks, projects, meetings | Cloud | Web, macOS, Windows, iOS, Android | Pro AI $19–29/mo; Business AI $29–49/mo | Unknown | Active; **pivoted to B2B "AI Employees"** | Aug 2026 pricing verified |
| Amie | Amie | Calendar + tasks; "happy calendar" philosophy | Cloud | Web, macOS, Windows, iOS, Android | ~$9/mo; $149 lifetime | Unknown | Active | Aug 2026 |
| Screenpipe | Screenpipe (open-source) | Continuous screen + audio capture; searchable AI memory; local storage | Local | macOS, Windows, Linux | Free (source-available) | Unknown | Active on GitHub | Aug 2026 |
| Open Recall | Various | Open-source Recall alternatives; local screenshot capture | Local | Windows, macOS | Free | N/A | Active on GitHub | Aug 2026 |
| Windrecorder | Yuka Friends | Open-source screen recorder for AI agent skills | Local | macOS, Windows | Free (MIT) | N/A | Active on GitHub | Aug 2026 |
| Microsoft Skill Recorder | Microsoft | Records screen sessions; reconstructs as reusable AI agent skills (SKILL.md) | Local | macOS (primary), Windows 11 | Free (MIT, GitHub) | N/A (Microsoft) | **Launched July 29, 2026**; requires Copilot access | Aug 2026 (1.8k+ GitHub stars) |
| OpenHistory | Independent | Open-source Mac app; tracks workday; **on-device inference**; hourly/daily summaries | Local | macOS | Free (MIT) | N/A | Launched August 2026 | Aug 19, 2026 (X/Twitter announcement) |
| Ambient Context | dragthelake (independent) | Menu bar app; reads focused window text via Accessibility API; writes markdown files | Local | macOS | Free (GitHub) | N/A | **Show HN August 25, 2026** | Aug 25, 2026 |
| **Logical** | Logical (YC F25) | Desktop-resident AI copilot; observes work context across Gmail, Slack, Mail, Excel; suggests actions | Local | macOS, Windows | Unknown | YC F25 | Active; YC F25 batch | Aug 2026 |
| **Hansel** | Hansel | Remembers what you worked on; finds past context; answers questions about workday; encrypted on Mac | Local | macOS | Unknown | Unknown | **Launched August 2026** | Aug 14, 2026 (Product Hunt) |
| OpenHuman | TinyHumans | Open-source AI agent; reads connected tools every 20 min; writes markdown memory files | Local | macOS, Windows, Linux | Free (GPL-3.0) | Unknown | Launched May 13, 2026; ~36k GitHub stars | Aug 2026 |
| Project SKY | Independent | Ambient AI companion for Windows; spatial screen perception, voice, long-term memory | Local | Windows | Unknown | Unknown | Launched August 2026 | Aug 21–30, 2026 (Product Hunt) |
| Zep | Zep | Open-source graph memory for AI apps; Graphiti library | Local/Cloud | API, SDK | Free (open-source) | Unknown | Active; pivoted to open-source Graphiti | Aug 2026 |
| Contrive | Contrive | Unified search + action layer across work apps; keyboard-first desktop command bar | Cloud | Desktop app | Unknown | Unknown | Launched August 2026 | Aug 29, 2026 (Product Hunt) |
| Egoist Machines | Egoist Machines (YC S26) | User-owned memory + preferences across AI apps | Local/Cloud | Unknown | Unknown | YC S26 | YC Summer 2026 batch | Aug 2026 |

**Apple's own features** were handled in prose, not the table. See Section 4.

---

## 3. NEW ENTRANTS — last 12 months

The report's own two-tier split. This is the section that matters most, and the headline is a **timeline collapse**: nine entrants in the observation-layer category in the five weeks from July 29 to August 29, 2026.

### 3a. Flagged by the report as "Genuinely Close to Your Product"

| Entrant | Date | What it is | Local/cloud | How close to observe→classify→restore | Sourcing |
|---|---|---|---|---|---|
| **ChatGPT Computer History** (OpenAI) | Aug 13, 2026 | Records clicks, keystrokes, app switches via macOS Accessibility API; searchable timeline; 48hr server retention, not used for training. Opt-in, Pro/Business/Enterprise, blocked in EEA/CH/UK | Cloud processing, local storage | **Closest on observation. Zero on restoration.** Search/summarize only. No project classification | Best in report — 9 press sources incl. The Register [120]; **no OpenAI primary doc** |
| **Hansel** | Aug 14, 2026 (Product Hunt) | "Remember everything you've worked on, find past context, and answer questions about your workday. Your data stays encrypted on your Mac." | Local (macOS) | **The single most dangerous unverified entry.** Same target user, same Mac-local privacy pitch, same "restore past context" language. Nobody checked whether it restores | **One source only** — producthunt.com/products/hansel-2 [86] |
| **Logical** (YC F25) | 2026 | Desktop-resident AI copilot; observes working context across Gmail, Slack, Apple Mail, Excel; suggests context-aware actions | Local | Observation + suggestion, app-connector-based not OS-wide. Action layer, not restoration | **One source only** — startupspies.com blog [85]. No ycombinator.com company page |
| **OpenHistory** | Aug 19, 2026 | Open-source Mac app, "automatically tracks your entire workday" with **on-device inference**; hourly + daily summaries; MCP to local agents | Local (MIT) | Observation + on-device summarization. No restoration, no project model. **Directly attacks the "local-first + on-device" differentiator as free/MIT** | One source — x.com/zachtratar [81] |
| **Ambient Context** | Aug 25, 2026 (Show HN) | Menu bar app reads focused-window text via Accessibility API every few seconds; writes one plain markdown file per day; for AI agents to query. No screenshots, no OCR | Local (GitHub) | Pure capture primitive. Proves the capture layer is now a weekend project | 3 sources, all HN/X aggregators [82][83][84] |
| **OpenHuman** (TinyHumans) | May 13, 2026 (PH #1) | Open-source GPL-3.0 agent reads connected tools every 20 min; writes markdown memory files openable in Obsidian. Claimed ~36k GitHub stars | Local | Tool-connector memory, not desktop observation | One blog [87], **no github.com link for the 36k claim** |
| **Microsoft Skill Recorder** | Jul 29, 2026 | Open-source MIT desktop app records screen sessions (window switches, URLs, optional narration); GitHub Copilot CLI reconstructs as intent + ordered steps in SKILL.md for Microsoft Scout/Copilot. 1.8k+ stars | Local | **Structurally the closest to restoration** — it turns observed sessions into replayable ordered steps. Microsoft shipping this on macOS-primary is a platform signal | 5 sources, all SEO/aggregator [76]–[80] |
| **Screenpipe** | ongoing | Source-available continuous screen + audio capture; searchable AI memory; all local | Local | Capture + search. No classification, no restoration | kitploit mirror [74], **not the GitHub repo** |
| **Project SKY** | Aug 21, 2026 (PH) | "Ambient AI companion for Windows" — spatial screen perception, conversational voice, long-term ambient memory. "Lives on OS, not browser" | Local | Windows-only. Same thesis, wrong platform | 2 sources [88][89] |

### 3b. Adjacent, not direct

| Entrant | Date | Why it is not direct |
|---|---|---|
| Egoist Machines (YC S26) | 2026 | "User-owned memory and preferences that work across every AI app" — portability across AI apps, not desktop activity capture |
| Zep / Graphiti | ongoing | Open-source graph memory library/infrastructure, not an end-user product |
| Contrive | Aug 29, 2026 | Unified search + action layer, keyboard-first command bar. Search, not activity memory |
| Mem Agent | Aug 11, 2026 | Proactive AI tracking todos in notes/meetings, "Push-to-Remember" capture. Note-based memory, not observed activity |
| Dia (Atlassian, Oct 2025, $610M) | 2025–26 | AI-native browser with Memory, Skills, tab groups. **Browser-centric, not OS-level** |
| **Raycast v2.0** | Aug 25, 2026 | Added AI agents, memory, **screen awareness**. "Still primarily a launcher, but moving toward OS-level AI layer" |

### 3c. Verdict on a true direct competitor

**No product in the report does the full loop: observe → classify by project → restore files/tabs/apps.**

But the observation layer went from scarce to commoditized in five weeks, and it is now available free (OpenHistory, Ambient Context, Screenpipe, Skill Recorder are all MIT/open) or bundled (ChatGPT). **The observation layer is no longer a moat. The moat is classification + restoration.**

Two entrants must be verified hands-on before the competition slide ships:
1. **Hansel** — same platform, same privacy posture, same "restore past context" copy, one Product Hunt page as the entire evidence base. If Hansel restores context, the "no one does this" claim is dead.
2. **Microsoft Skill Recorder** — already converts observed sessions into ordered replayable steps. That is restoration in a different wrapper, shipped by Microsoft, on macOS, MIT-licensed.

---

## 4. PLATFORM RISK — Apple

### What Apple shipped (last 18 months, as reported)

| Release | Date | Relevant contents | Report's verdict |
|---|---|---|---|
| macOS 15 Sequoia | Sept 2024 | iPhone Mirroring, window tiling, Passwords app, Apple Intelligence on compatible hardware | "**No app state restoration or activity history**" |
| macOS 26 Tahoe | Sept 2025 | Liquid Glass UI, expanded Spotlight actions (hundreds of system actions), Phone app, Live Activities from iPhone, Apple Intelligence expansion. Spotlight gains **Clipboard mode (8hr history)**, Actions mode, app-specific search | "**No native app state restoration or cross-device work continuity beyond existing Continuity features**" |
| Apple Intelligence | WWDC 2024, shipping late 2024–2025 | On-device LLM via **Foundation Models framework** (iOS 26/macOS 26); developers call Apple's on-device model with no API keys or cloud | "**No activity history or screen observation API exposed to third parties**" |
| WWDC 2026 | June 2026 | Local AI on Apple Silicon with **MLX framework**; AI agents can run entirely on-device | "Focus on developer tools, not end-user activity tracking" |

### Accessibility API / Screen Recording / sandboxing

Report's finding, verbatim substance: "**No public evidence of Apple restricting Accessibility API access for screen observation as of August 2026.** ChatGPT Computer History, Ambient Context, and OpenHistory all use Accessibility API successfully. However, **Tahoe 26 added warnings for background apps and tighter control over menu bar apps — signals of increased scrutiny, not bans.**"

**This is the weakest conclusion in the report and it errs in the direction that flatters us.** See Report Quality Problems #7.

### "Apple could just build this" — risk rating

Report says **"Medium risk, not imminent."** Reasoning:
- Apple has shown interest in on-device AI (Apple Intelligence, Foundation Models)
- Apple has added clipboard history (8hr) and activity-like features in Spotlight
- Apple has **not** shipped anything resembling continuous activity observation or app state restoration
- If Apple builds it, it would be: opt-in only; on-device (Private Cloud Compute for heavy tasks); tied to Apple Intelligence
- **Evidence cited:** "Apple has not acquired any activity-tracking startups, nor hinted at such features in WWDC 2024–2026 keynotes. The closest is Spotlight's clipboard history (8hr) and Actions — useful but not a memory system."

### Historical base rate: absorbed vs survived

| Absorbed by Apple | Survived alongside the OS feature |
|---|---|
| Basic window management (Split View, Stage Manager) | Window management — **Rectangle, Magnet, Moom** |
| Password management (Passwords app) | Launchers — **Raycast, Alfred** |
| Note-taking (Notes app improvements) | Note-taking — **Obsidian, Notion, Bear** |
| Screenshot tools (Screenshot app) | Clipboard managers — **Paste, CopyClip** |
| Clipboard (Universal Clipboard → Clipboard history in Spotlight) | Menu bar organizers |

**Report's key insight:** "Apple tends to build **good-enough** native versions of popular categories but rarely matches the depth of specialized tools. Raycast, Rectangle, and Obsidian have thrived despite Apple's native offerings."

Sourced to a podcast episode [104], putback.app [105], shiftplus.app [106]. Directionally credible, **zero rigor**, and it omits the strongest historical examples (Sherlock/Watson, Konfabulator→Dashboard, Growl→Notification Center, f.lux→Night Shift, Duet/Air Display→Sidecar, iStat Menus). Those would strengthen the argument. Do not present this base rate as researched.

---

## 5. Microsoft Recall — current status

### As of August 2026
- **Opt-in only** (flipped from initial opt-out design after backlash)
- Requires **Copilot+ PC (report says Snapdragon X Elite/Plus — stale, see QA)**
- Screenshots encrypted with **Windows Hello authentication + VBS enclave**
- **All processing local; no cloud upload**
- Still facing security researcher criticism: **March 2026 report by Alexander Hagenah** described decrypted content being interceptable from an unprotected rendering process **post-authentication**. Microsoft disputes this as within documented design.

### Security findings timeline

| Date | Finding |
|---|---|
| May 2024 | Plaintext local storage, easily accessible database — immediate backlash |
| Late 2024–2025 | Redesign: encryption, Windows Hello, VBS enclave, opt-in |
| May 2024 | **Kevin Beaumont** — showed malware could exfiltrate the entire Recall database |
| May 2024 | **Alexander Hagenah** — built **"TotalRecall"** tool to extract screenshots; viral demo; database extraction without admin privileges |
| Aug 2025 | **Tom's Hardware** and **The Register** retested post-redesign: encryption improvements confirmed, but **sensitive-info filter can miss card/SSN formats** |
| March 2026 | Post-redesign: researcher claims decrypted content still accessible during active sessions |

### Public sentiment
- "Overwhelmingly negative at launch; dubbed **'spyware'** by press" (The Verge, Wired, Ars Technica: "spyware," "privacy nightmare")
- "Redesigned architecture addressed plaintext criticism but not all concerns"
- **"Narrative shifted from 'unacceptable' to 'improved but still concerning'"**
- "Enterprise adoption cautious; consumer uptake unclear"; many enterprises disabled Recall via Group Policy

### Regulator response
- EU privacy regulators raised concerns under GDPR re: continuous monitoring, data retention, user consent
- **No formal bans or formal action as of August 2026**, ongoing scrutiny and ongoing dialogue with Microsoft

### What people specifically objected to (the five objections — use these verbatim on the privacy slide)
1. **Opt-out by default** — users had to actively disable during setup
2. **Plaintext storage** — initial design stored screenshots unencrypted
3. **No granular control** — couldn't exclude specific apps/sites initially
4. **Perpetual recording** — no way to pause without disabling entirely
5. **Copilot+ PC requirement** — felt like a forced hardware upgrade

---

## 6. POST-MORTEM: Rewind AI → Limitless → Meta

### Timeline as reported

| Date | Event |
|---|---|
| May 2023 | Raised Series B at ~$350M valuation (~$30M total raised) |
| 2024 | Rebranded to Limitless, launched Pendant hardware ($99) |
| **Dec 5, 2025** | Meta announced acquisition (**"~$610M reported, though this may include other considerations"**) |
| **Dec 19, 2025** | Rewind Mac app's screen/audio recording **permanently disabled via kill switch (not a graceful sunset)** |
| 2026 | Pendant sales ceased; team joined Meta Reality Labs; existing devices supported through 2026 |

### The four stated reasons for the pivot

The report's framing, verbatim: **"Public evidence indicates the pivot was driven by:"**

1. **Privacy objections** — Mac app faced constant privacy concerns; users uncomfortable with continuous screen recording
2. **Retention issues** — **high churn after initial novelty; users didn't find enough value in the searchable timeline**
3. **Hardware as privacy solution** — Pendant records only conversations (audio), not full screen — narrower scope, clearer value prop
4. **Acquisition opportunity** — Meta seeking wearables talent for Reality Labs; better exit than continuing standalone

**Report's key lesson:** "Continuous screen recording on Mac faced **insurmountable privacy friction** for mainstream adoption. Hardware narrowed the scope to meetings/conversations, which users found more acceptable."

### Disclosed numbers

- **"No public retention/usage numbers disclosed."**
- Series B at $350M valuation "suggests strong investor confidence pre-pivot"
- "Acquisition by Meta validates the team/tech, **not necessarily the Mac app's viability**"

### Critical caveat on this post-mortem

**There is not one Dan Siroker quote in the report.** The prompt demanded founder statements; the report delivered four unattributed bullets prefaced by the hedge "public evidence indicates." Its six sources for the entire Rewind/Limitless section are `earkeep.com/alternatives/limitless`, `everpaper.app/docs/blog/best-rewind-alternatives-2026`, `ai-market-watch.com/company/limitless`, `layer3labs.io/gear/reviews/limitless-pendant`, `therundown.ai/tools/limitless`, `registry.deploy.report/companies/limitless` — **six SEO/aggregator/alternatives pages, zero primary sources.** No Siroker blog post, no podcast, no X thread, no Limitless changelog, no Meta or Limitless press release.

**Treat reasons 1–4 as a plausible hypothesis, not evidence.** Reason 2 (retention/churn) in particular carries no number and no attribution, and it is the reason a pre-seed investor will press hardest on. If the deck asserts "Rewind failed on retention," we are asserting an unsourced claim. The defensible facts are only: the Mac app was killed by kill switch on Dec 19, 2025; the company had pivoted to audio-only hardware; the team was acquired into Reality Labs.

---

## 7. POST-MORTEM: The Browser Company, Arc → Dia → Atlassian

### Timeline

| Date | Event |
|---|---|
| **May 27, 2025** | **Josh Miller (CEO) published an open letter: Arc had "fallen short" of mainstream adoption due to complexity; active development halted** |
| June 11, 2025 | Dia launched as invite-only beta (AI-first, simpler than Arc) |
| Sept 4, 2025 | Atlassian announced acquisition (~$610M) |
| Oct 8, 2025 | Dia opened to everyone on macOS (no waitlist) |
| **Oct 20, 2025** | Acquisition closed; **Atlassian filing shows $488.3M purchase price** |
| 2026 | Dia gets weekly releases; Windows beta waitlist; Arc in maintenance (Chromium security updates only) |

### Founder explanations
- **Arc's problem:** "Too complex for mainstream; steep learning curve; 'reimagined browser' didn't resonate broadly"
- **Dia's bet:** "AI as core primitive, not add-on; simpler UX; focused on knowledge workers"
- **Acquisition rationale:** "Atlassian wanted an AI browser to compete with Microsoft Edge Copilot; better distribution than standalone"

**Report's key lesson:** "**Browser re-architecture is extremely hard — users don't want to learn new browsing paradigms.** AI-native positioning (Dia) found more traction than UI reimagining (Arc)."

Note the two acquisition figures: announced ~$610M vs. filed $488.3M. Use **$488.3M** if citing a number, and note the same **$610M** figure is attached to the Meta/Limitless deal in this same report — see QA #4.

---

## 8. WHITE SPACE ASSESSMENT (report's conclusion, verbatim substance)

"Based on the research, here's what **no existing product does**:"

1. **Local-first, project-aware context restoration** — ChatGPT Computer History, Hansel, OpenHistory, and Ambient Context track activity but **don't automatically restore files, tabs, and apps** to resume a project. They're memory/search tools, not context restoration engines.
2. **On-device AI with no cloud dependency** — ChatGPT Computer History processes on OpenAI servers (48hr retention). OpenHistory, Ambient Context, and Screenpipe are local but **lack sophisticated project classification**. Differentiator: everything on-device, no cloud, project-aware.
3. **Automatic project classification** — existing tools track activity chronologically but **don't understand which activities belong to which project. "This is your core IP opportunity."**
4. **Mac-native, privacy-first positioning** — Recall is Windows-only and privacy-damaged; ChatGPT Computer History requires a Pro subscription and cloud processing. "Mac users who want local-first, no-subscription, privacy-respecting tools are underserved."
5. **Seamless restoration workflow** — no competitor offers "**one-click resume project**" that relaunches apps, restores tabs, and reopens files. Tab managers (Workona, Session) do this for browser tabs only, not full desktop context.

**Assessment of the assessment:** internally consistent with the table — nothing in 31 products restores full desktop state, and nothing classifies by project. But it is an argument from absence built on a source base that is largely SEO content, and **the negative was never verified against the two products most likely to falsify it (Hansel, Skill Recorder).** Claim 2 is also weakening fast: OpenHistory already ships on-device inference under MIT, so "on-device" alone is not defensible for long — only "on-device + project classification + restoration" is.

---

## 9. MOST DANGEROUS COMPETITOR (report's conclusion)

### #1 — ChatGPT Computer History (OpenAI)

1. **Direct feature overlap** — records clicks, keystrokes, app switches via Accessibility API; builds a searchable timeline. "Exactly your core observation layer."
2. **Distribution advantage** — bundled with the ChatGPT desktop app (Pro/Business/Enterprise); **"800M+ weekly active users globally"**
3. **Brand trust** — "OpenAI has more consumer trust than unknown startups; users may accept cloud processing from OpenAI but not from you."
4. **Momentum** — launched Aug 13, 2026; "this is fresh. OpenAI is actively investing in desktop AI."
5. **Exploitable weaknesses** — requires Pro subscription ($20/mo+); cloud processing; not available in EEA/Switzerland/UK; **doesn't restore context, only searches and summarizes; no project classification**

### #2 — Apple
"Not shipping anything now, but if Apple adds 'Work Context' to Apple Intelligence (on-device, free, integrated), it could absorb your category. However, Apple's privacy stance and historical behavior suggest they'd build a **good-enough** version, not a deep one — leaving room for specialized tools."

### #3 — Microsoft
"Recall is damaged but not dead. If Microsoft fixes security concerns and expands to non-Copilot+ PCs, it could regain traction. However, **the privacy stigma is lasting**."

### Where I disagree with the ranking

The report ranks by brand size. Ranked by *actual threat to our specific wedge*:

- **Raycast is under-rated at "adjacent."** It is the only company in the table that already owns our exact user (Mac power users), already has paid distribution, and in v2.0 (Aug 25, 2026) shipped **agents + memory + screen awareness** — the three primitives adjacent to ours. Apple absorbing us is a 2028 problem; Raycast shipping "Workspaces" is a 2027 problem.
- **Hansel is under-rated at "genuinely close."** It is the only entrant with our exact positioning on our exact platform, and the report has one Product Hunt page on it. Unverified is not the same as not-a-threat.
- **The real finding is commoditization, not any single competitor.** Jul 29 → Aug 29, 2026: Skill Recorder, Mem Agent, ChatGPT Computer History, Hansel, OpenHistory, Project SKY, Ambient Context, Raycast v2.0, Contrive. Nine launches in five weeks. The strategic read is that "we observe your work" is table stakes by 2027 and the entire defensible claim is **project classification + one-click restoration**.

### Report's strategic recommendations (as given)
1. **Ship fast** — "You have a 6–12 month window before OpenAI adds restoration features."
2. **Emphasize local-first** — "This is your moat."
3. **Project classification is your IP** — "Invest heavily here. No competitor does this well."
4. **Price aggressively** — ChatGPT requires $20/mo Pro; a one-time purchase or <$10/mo attracts price-sensitive users.
5. **Build Mac-first reputation** — "Mac users are underserved. Own this niche."
6. **Watch YC batches** — "Logical (F25), Egoist Machines (S26) are well-funded and moving fast."

(Note #6 contradicts the table, which records both companies' funding as "Unknown.")

---

## 10. Report Quality Problems

The report was given three explicit source rules. It violated all three.

> "Product capabilities must come from the product's own site or documentation, or hands-on reviews. **Not from competitors' comparison pages.**"
> "**Date-stamp everything.** This category moves fast and stale information is worse than none."
> "If a product appears abandoned, **check its changelog, release notes, and social accounts** before saying so, and give me the evidence."

### 1. The source base is predominantly SEO content farms and comparison pages — the exact prohibited class

Of 150 sources, the overwhelming majority are aggregator, "best X of 2026," or "X alternatives" pages. Named violations of the "not from competitors' comparison pages" rule:

`luci.memories.ai/blog/microsoft-recall-alternatives-2026` · `earkeep.com/alternatives/limitless` · `everpaper.app/docs/blog/best-rewind-alternatives-2026` · `marqly.com/alternatives/workona` · `marqly.com/blog/best-tab-managers-2026` · `speakwiseapp.com/blog/mem-ai-vs-reflect` · `pointofai.com/compare-ai-tools/mem-vs-reflect-notes` · `layer3labs.io/comparisons/limitless-vs-bee-vs-omi` · `tooldirection.com/heptabase-pricing-2026` · `saascrmreview.com/best-second-brain-apps` · `aitoolalliance.com/best-ai-note-taking-second-brain-apps-2026` · `smallbizbenchmark.com/sunsama-vs-motion` · `pipeline.zoominfo.com/sales/sunsama-vs-motion` · `macside-ai.com/en/blog/raycast-pro-alternative` · `citeme.io/ressources/best-ai-browsers-2026-compared` · `storyflow.so/blog/best-ai-native-productivity-tools-2026` · `memeburn.com/best-ai-note-taking-app` · `lound.ai/blog/best-day-one-alternatives-2026` · `foraithings.com/articles/best-ai-tools-for-note-taking-2026` · `myflexnote.com/blog/best-ai-note-taking-apps` · plus five `neurolaunch.com` and five `itechguides.com` listicles.

### 2. Keyword-exact and synthetic-looking domains carrying load-bearing facts

`yespress.io/tags/productivity/companies` · `ultrathink.ai/news/openai-computer-history-macos-tracking` · `ai-market-watch.com/company/limitless` · `registry.deploy.report/companies/limitless` · `officialairankings.com/reviews/...` · `unknowncharges.com/charge/raycast` · `ki-auf-dem-mac.pages.dev/subscription-comparisons/provider/raycast` · `startupspies.com` · `lemstudio.co/yc-companies/egoist-machines-32129` · `hunted.space/dashboard/project-sky` · `opensourcedrop.com` · `agentry.news` · `skillsllm.com` · `vibecrowd.ai` · `openagentskill.com` · `rssatlas.com/feeds/1` · `trendshift.io/repositories/26695` · `gittrend.io` · `dodecaidr.pro` · `putback.app` · `shiftplus.app` · `daftei.com` · `wukihow.com` · `websearchapi.ai` · `summify.io` · `machash.com` · `shareuhack.com` · `wawasanai.com` · `atlasworkspace.ai` · `hokai.io` · `macappsdaily.com` · `systemaic.com` · `temporal.day` · `spinnable.ai` · `reassign.ai` · `bondapp.io` · `elephas.app` · `preuve-style` AI-blog domains throughout. Source [135] is `learnofchrist.com/resources/reflect` — a religious site cited for a notes app.

**Almost no primary sources appear.** Present and legitimate: `raycast.com/changelog/windows/2-0` [48], `raycast.com/changelog` [49], `support.heptabase.com` pricing FAQ [32], `producthunt.com` product pages [24][86][88][89][91], `news.ycombinator.com` [142][143], `theregister.com` [120], `microsoft.com/msrc` [112], `bleepingcomputer` [113], `securityweek` [114], `cyber.gc.ca` [111], `apps.apple.com` listings, three `developer.apple.com` URLs. That is roughly 15–20 of 150.

### 3. Wrong factual detail: Kevin Beaumont's handle

The report writes **"Kevin Beaumont (@GrusOnSecurity)"**. Beaumont's handle is **@GossiTheDog**. `@GrusOnSecurity` is not his account. A fabricated identifier on the most-quoted researcher in the Recall story. Do not reproduce this attribution anywhere.

### 4. The $610M number appears on two unrelated acquisitions

Meta/Limitless is given as "~$610M reported, **though this may include other considerations**" and Atlassian/Browser Company is given as "~$610M" (later corrected in the same report to a filed $488.3M). The $610M figure is the widely reported Atlassian/Browser Company number. **Its appearance on the Meta/Limitless deal is almost certainly number bleed.** Do not cite a Limitless acquisition price.

### 5. Funding figures are unreliable — one probable fabrication, several research failures

| Product | Report says | Problem |
|---|---|---|
| Mem | **~$130M (Series C, 2023)** | Mem's publicly known raise is ~$23.5M Series A (2022, OpenAI Startup Fund). A $130M Series C is not corroborated by any source in the list. **Treat as probably fabricated.** |
| Granola | **Unknown** | Granola raised publicly and prominently in 2025. It is the most relevant funded comparable for a Mac-native prosumer AI app. Recording it as "Unknown" is a research failure at the exact point we most need a benchmark. |
| Raycast | **Unknown** | **Report 2 in this same batch cites `techcrunch.com/2020/10/29/...raycast-raises-2-7m-from-accel`.** The two reports contradict each other. |
| Workona, Heptabase, Reflect, Sunsama, Motion, Amie, Session, Screenpipe, Hansel, Logical, Project SKY, Contrive, Egoist Machines, Zep | Unknown | 14 of 31 rows have no funding data. The funding column is close to useless. |
| Logical, Egoist Machines | Unknown in table, **"well-funded" in the recommendations** | Direct internal contradiction. |

### 6. Product status claims that are stale or unsourced against the report's own rules

- **Microsoft Recall hardware requirement** — "Requires Copilot+ PC (**Snapdragon X Elite/Plus**)". Stale/incomplete: Recall shipped to Intel Core Ultra and AMD Ryzen AI Copilot+ PCs after the Snapdragon-first launch. Do not put "Snapdragon-only" on a slide.
- **"May 2024 launch: Plaintext local storage"** — Recall was *announced* in May 2024 and pulled before general release. The plaintext demos were against preview builds. Calling it a launch mis-tells the sentiment story.
- **Session Buddy "MV2 shutdown; support issues reported"** — this is an abandonment claim. The rule required a changelog, release notes, or social-account check. Evidence given: one Reddit thread [59] and `superchargebrowser.com` [60]. **Rule violated on the one row where it explicitly applied.**
- **Screenpipe "Active on GitHub"** — cited to a `kitploit.com` mirror, not to the repository. No commit date, no release tag.
- **Open Recall** — company recorded as "Various," no repository, no link, no version. This row is not a product; it is a category with a row.
- **Windrecorder** — cited to `openagentskill.com`, not its GitHub repo.
- **OpenHuman "~36k GitHub stars"** — cited to a personal blog with no github.com link. Implausibly high for a May 2026 launch; verify before quoting.
- **"Last meaningful update: August 2026"** on 12+ rows — this records when the report ran, not a shipped release. It presents the freshness of the search as the freshness of the product. Directly against "date-stamp everything."

### 7. The platform-risk conclusion is under-evidenced, and errs in our favour

The report concludes "no public evidence of Apple restricting Accessibility API access." That is an argument from absence, and the report never checked the documentation the prompt asked it to check. Sources [125]–[127] are `developer.apple.com/news/releases/?id`, `developer.apple.com/news/releases/?id=08172026i`, and `developer.apple.com/news/` — generic index URLs, **and none of the three is cited anywhere in the body.** So the strongest claim in the platform-risk section rests on zero examined release notes.

**Known omissions the report should have surfaced, all of which cut against us:**
- **macOS 15 Sequoia's recurring Screen Recording permission prompts.** Sequoia betas introduced weekly re-authorization prompts for screen-capture permission; developer backlash forced Apple to relax the cadence before release. This is the single most concrete Apple signal in the last 24 months against third-party apps that continuously observe the screen, and the report does not mention it once.
- **ScreenCaptureKit's displacement of `CGDisplayStream`** and the general TCC tightening trajectory.
- **Mac App Store review posture on Accessibility API usage**, which bears directly on our distribution strategy (direct-download vs. MAS).
- No WWDC session numbers, no API deprecation lists, no `developer.apple.com/documentation` diffs — despite the prompt explicitly requesting "developer documentation changes."

Read the platform risk as **higher than "medium, not imminent."** The correct statement is: we do not know, because the report did not look at Apple's documentation.

### 8. Y Combinator claims are not verified against YC's registry

**Logical (YC F25)** rests on `startupspies.com/blog/logical-yc-f25-first-fully-sri-lankan-team`. **Egoist Machines (YC S26)** rests on `lemstudio.co/yc-companies/egoist-machines-32129`. Source [1] is `https://www.ycombinator.com/companies/industry/Artificial Intelligence` — **a malformed URL containing a literal space, pointing at a directory index, not at either company.** Neither company has a `ycombinator.com/companies/<name>` link. Both YC claims are unverified.

### 9. The load-bearing claim has no primary source

**ChatGPT Computer History is the report's "most dangerous competitor" and the entire premise of its executive summary.** It is supported by nine press articles including The Register [120] (`openai-ditches-recall-style-screenshot-surveillance-for-friendly-keylogging` — which independently corroborates the Accessibility-events mechanism rather than screenshots). That is the best-sourced item in the report.

But: **there is no `openai.com` or `help.openai.com` link anywhere in 150 sources.** The specifics we would put on a competition slide — 48-hour server retention, Pro/Business/Enterprise gating, EEA/Switzerland/UK exclusion, "not used for training" — all come from secondary press and SEO pages (`ultrathink.ai`, `elephas.app`). **Verify every one of those four specifics against OpenAI's own release notes before it goes on a slide.** If we misstate OpenAI's retention policy in front of an investor who uses the product, we lose the room.

### 10. Citation padding

Sources 108–150 (43 of 150) are never cited in the body. Several are irrelevant to any claim made (`learnofchrist.com`, `podcasts.apple.com` episode listings, `discussions.apple.com` threads, `support.google.com` Chrome threads, a Vietnamese forum thread, `arxiv.org/pdf/2608.05784.pdf` with no accompanying claim). The source count overstates the research depth by roughly 30%.

### 11. Apple's own features never made it into the comparison table

Section 1 was specified as a table covering, among others, "Apple's own features (Spotlight, Stage Manager, Continuity, Sequoia/Tahoe additions, Apple Intelligence)." Apple appears only in Section 3 prose. Spotlight, Stage Manager, and Continuity have **no row**, so the table cannot support a claim about what Apple's native stack does or doesn't do — which is exactly the claim the competition slide needs.

---

## 11. What is safe to put on a slide vs. what must be re-verified

| Claim | Confidence | Notes |
|---|---|---|
| Rewind's Mac app was killed by kill switch on Dec 19, 2025; team went to Meta Reality Labs | **High** | Multiple sources; event-level facts |
| Arc halted active development May 27, 2025; Josh Miller open letter cited complexity and failure to reach mainstream; Atlassian filing $488.3M | **High** | Named, dated, specific |
| Microsoft Recall is opt-in-only after backlash; the five specific objections; "spyware" press framing; no formal regulator action | **High** | Well-corroborated public record |
| No product currently observes + classifies by project + restores files/tabs/apps | **Medium** | Consistent across 31 products, but the negative was never tested against Hansel or Skill Recorder |
| ChatGPT Computer History exists, launched Aug 13 2026, uses Accessibility API, has no restoration | **Medium-high** | The Register corroborates the mechanism; **verify retention/gating/geo specifics against OpenAI docs** |
| Apple has not shipped activity history or app-state restoration | **Medium** | Plausible, but derived from absence of search results, not from documentation |
| Apple has not restricted Accessibility API / Screen Recording | **Low — do not assert** | Report never checked Apple docs and missed the Sequoia screen-recording prompt episode |
| Rewind pivoted because of retention/churn | **Low — do not assert as fact** | Zero founder quotes, zero numbers, six SEO sources. Frame as "publicly reported reasons include" or drop |
| Mem raised $130M Series C | **Do not use** | Probably fabricated |
| Meta paid ~$610M for Limitless | **Do not use** | Almost certainly bled from the Atlassian/Browser Company deal |
| "Kevin Beaumont (@GrusOnSecurity)" | **Do not use** | Wrong handle |
| Recall is Snapdragon-only | **Do not use** | Stale |
| Granola/Raycast/Workona funding | **Do not use** | Recorded as Unknown; Report 2 contradicts on Raycast |

### Open verification tasks before the competition slide ships
1. Install and use **Hansel** (macOS, Product Hunt Aug 14 2026). Does it restore context or only answer questions? This single test decides whether the white-space claim survives.
2. Read **Microsoft Skill Recorder**'s repo. Ordered-step reconstruction from observed sessions is functionally adjacent to restoration.
3. Pull **OpenAI's own** release notes for Computer History — retention, gating, geography, training use.
4. Check **Apple's** actual documentation: Sequoia/Tahoe TCC and screen-recording permission changes, ScreenCaptureKit deprecations, Accessibility API review posture. The report did not.
5. Get real funding numbers for **Granola** and **Raycast** — they are our closest prosumer-Mac comparables and the report has neither.
6. Verify **Logical** and **Egoist Machines** on ycombinator.com/companies directly.
