# Digest 02 — Problem Evidence Base (Context Switching, Interruption, Task Resumption)

**Source document:** `uploads/You are a research librarian assembling the empiri.pdf`
**Tool:** Perplexity (deep research mode)
**Total pages:** 15. **Body:** pp. 1–12 (prompt pp. 1–2; five research sections; STRONGEST CITABLE CLAIMS p. 10; CLAIMS TO AVOID + GAPS p. 11; DIRECT LINKS + "Bottom line" p. 12). **Sources:** footnote URL list begins mid-p. 12 and runs to p. 15 — **109 numbered sources**, roughly 3.5 pages of pure URLs.
**Date of digest:** 2026-08-31.

---

## Key Findings (what the report actually FOUND)

1. **The "23 minutes 15 seconds" statistic is a fabrication of the popular literature.** The report traced it and could not find it in any Gloria Mark paper. See the verbatim verdict below.
2. **The closest real number is 25 min 26 sec — and it does not mean what people think.** It is *elapsed clock time until interrupted work was resumed*, conditioned on same-day resumption (77% of cases), during which workers completed **2.26 other work tasks**. It is not focus-recovery time. Its **SD is 54 min 48 sec** — more than double the mean — so the mean is a poor summary of a wildly skewed distribution (the report reports the SD but never comments on it).
3. **There is NO research on multi-day / multi-week project resumption.** This is the report's own headline gap and the most decision-relevant finding in the document. Stated flatly: *"the research does not exist."* The product's core claim has no empirical literature behind it, for or against.
4. **The strongest interruption paper is counter-evidence, not support.** Mark, Gudith & Klocke (CHI 2008): interrupted tasks were completed **faster** than uninterrupted baseline (20.31 / 20.60 min vs 22.77 min, p<.05), with **no difference in errors**. The measurable cost was **stress, frustration, time pressure, effort and workload** (all p<.01). The cost of interruption in the best lab evidence is affective, not temporal.
5. **The best telemetry number — 1,200 toggles/day, ~4 hours/week reorienting — has undetermined provenance.** The report lists the authors as "Not explicitly named," funding as "Unclear from available sources," and then asserts in the same breath "This is not vendor-funded research." That assertion is unsupported and contradicts its own funding line. It is nonetheless the report's #2 strongest citable claim.
6. **Every industry "tool sprawl" number is vendor-funded.** Asana, Qatalog, Microsoft, RescueTime, Pluralsight, Cortex — six for six. Four of them (Asana, Qatalog, Pluralsight, Cortex) are **self-report surveys**, not telemetry, despite the prompt asking specifically for telemetry.
7. **The one mechanism that supports the product concept is Altmann & Trafton (2004): environmental cues available before an interruption reduce resumption lag.** That is the closest thing to a scientific rationale for a context-restoration tool. But the effect is measured in **seconds (3.8s vs 1.9s baseline)**, in a continuous lab task, with N=96 undergraduates, funded by the U.S. Office of Naval Research.
8. **A 2023 systematic review says the productivity-loss narrative is overstated.** Gross & von Kalben (INTERACT 2023) "explicitly calls for a more balanced approach. The productivity-loss narrative is overstated; interruptions have benefits in some contexts."

---

## THE 23-MINUTE VERDICT (verbatim from the report)

> **"CRITICAL FINDING: The exact figure "23 minutes and 15 seconds" does not appear in any of Gloria Mark's published papers that I could locate."**

> **"The closest published number is 25 minutes 26 seconds from Mark, Gonzalez & Harris (CHI 2005), which measures elapsed clock time until interrupted work was resumed (conditioned on same-day resumption, which occurred 77% of the time). During that elapsed time, workers were productively engaged in ~2 other tasks—not staring blankly recovering focus."**

> **"The "23:15" figure appears to be a later popularization that may conflate: 1. The 25:26 resumption time from the 2005 field study 2. Unpublished or secondary analyses 3. A rounded/adjusted figure for rhetorical effect"**

> **"Can you use this number honestly? No, not as stated.** If you cite "23 minutes 15 seconds to refocus," you are making a claim that:
> - Does not appear in the primary literature
> - Confuses **resumption time** (when people returned to the task) with **recovery time** (when they regained prior cognitive depth)
> - Ignores that the 25:26 figure only applies to the 77% of work resumed same-day
> - Does not account for the 2.26 intervening tasks people completed in that interval"

> **"What you can accurately cite: "When interrupted work is resumed the same day (77% of cases), the average elapsed time before resumption is approximately 25 minutes, during which workers complete an average of 2 other tasks" (Mark, Gonzalez & Harris, CHI 2005)"**

And from CLAIMS TO AVOID (p. 11), verbatim:

> **"1. "It takes 23 minutes and 15 seconds to refocus after an interruption" — This exact figure does not appear in Gloria Mark's published papers. The closest is 25:26 (Mark et al., CHI 2005), which measures elapsed time to resumption (not cognitive recovery), applies only to same-day resumptions (77% of cases), and includes time spent on ~2 other tasks."**

**Was it traced to the original paper?** Yes — the report obtained the primary PDF (`https://ics.uci.edu/~gmark/CHI2005.pdf`) and reports its actual numbers, plus the CHI 2008 primary PDF. **What the study actually measured:** ethnographic shadowing of task-switching behaviour; time in a "working sphere" before switching, interruption rate, same-day resumption rate, and elapsed time to resumption. **Sample size:** 24 information workers (7 managers, 9 analysts, 8 developers) at one IT outsourcing company; 700+ hours observed over 13 months, ~25 hours per worker across 3.5 days. **Is the popular framing accurate?** No — the report's word is **"Partially distorted"** for the 2005 paper, and for CHI 2008: *"This paper does not contain the 23-minute figure at all."*

**Two important limits on this verdict:**
- The finding is hedged: *"that I could locate."* The report did not definitively establish the 23:15 origin; it establishes only that it is absent from the papers it retrieved.
- The trace relies on secondary debunking blogs (clrtstudio.com "the-twenty-three-minute-myth", therezaali.com, geoffmazeroff.com "the-context-switching-statistic-that-got-lost-in-translation", scienceblog.com), not on a primary bibliographic audit of Mark's full corpus.

---

## Quantitative Evidence Tables — Every Study

### Section 1: Task interruption and resumption

| Study | Authors | Year | Venue | Sample size & population | Methodology | Actual finding with real numbers | Funder | Popular framing accurate? | Link |
|---|---|---|---|---|---|---|---|---|---|
| **No Task Left Behind? Examining the Nature of Fragmented Work** | Gloria Mark, Victor M. Gonzalez, Justin Harris | 2005 | CHI '05 (ACM) | **24 information workers** (7 managers, 9 analysts, 8 developers) at an IT outsourcing company | **Ethnographic shadowing / direct observation** — 700+ hours over 13 months; each worker observed ~25 hrs across 3.5 days | • Avg time in a "working sphere" before switching: **11 min 4 sec**<br>• **57%** of working spheres were interrupted<br>• **77.2%** of interrupted work resumed same day<br>• When resumed same-day, avg time until resumption: **25 min 26 sec (SD = 54 min 48 sec)**<br>• Before resuming, workers completed avg **2.26 other working spheres** | Academic (UC Irvine); no vendor funding disclosed | **"Partially distorted."** Paper reports 25:26, not 23:15. The "23 minutes 15 seconds" figure does not appear in this paper. | https://ics.uci.edu/~gmark/CHI2005.pdf |
| **The Cost of Interrupted Work: More Speed and Stress** | Gloria Mark, Daniela Gudith, Ulrich Klocke | 2008 | CHI '08 | **48 participants** (German university students, mean age 26; 15 male, 33 female) | **Laboratory experiment** (3×2 factorial: interruption context × media type) | • **Interrupted tasks completed FASTER than uninterrupted baseline**: 20.31 min same-context, 20.60 min different-context vs **22.77 min baseline** (p<.05)<br>• **No difference in errors** across conditions<br>• Significantly higher **stress, frustration, time pressure, effort, workload** in interrupted conditions (all p<.01 or better)<br>• Personality traits (openness to experience, need for personal structure) predicted faster completion of interrupted tasks | Academic; no vendor funding disclosed | **"This paper does not contain the 23-minute figure at all. It measures task completion time, not resumption lag."** | https://ics.uci.edu/~gmark/chi08-mark.pdf |
| **"Constant, Constant, Multi-tasking Craziness": Managing Multiple Working Spheres** | Victor M. Gonzalez, Gloria Mark | 2004 | CHI '04 | **"Not fully specified in secondary sources"** — field study of information workers | Field observation | Workers switched tasks approximately **every 3 minutes** on average | Academic | **Accurate** — "This is the origin of the 'every 3 minutes' claim, which is accurately represented in secondary literature." | (no direct link supplied) |
| **Task Interruption: Resumption Lag and the Role of Cues** | Erik M. Altmann, J. Gregory Trafton | 2004 | CogSci 2004 (Cognitive Science Society) | **96 Michigan State University undergraduates** (24 per experiment × 4 experiments) | **Laboratory experiment** — complex resource-allocation primary task + radar-tracking secondary task; interruption lags of 2/4/6/8 s; cue vs no-cue conditions | • **Resumption lag: 3.8 seconds vs uninterrupted inter-action interval of 1.9 seconds** (double the baseline; p<.0001)<br>• Cue availability reduced resumption lag at 6–8 s interruption lags (p<.05 at 8 s)<br>• **Environmental cues available before interruption facilitate faster resumption** | **U.S. Office of Naval Research** (grants N00014-03-1-0063, N0001400WX21058) | "Rarely cited in popular literature. The resumption lag measured here is in **seconds, not minutes**, because it measures time to first action after interruption in a continuous task—**not time to return to a task after leaving it for other work**." | https://gregtrafton.com/papers/task_interruption.pdf |

### Section 2: Context switching cost

| Study | Authors | Year | Venue | Sample & population | Methodology | Actual finding with real numbers | Funder | Popular framing accurate? | Link |
|---|---|---|---|---|---|---|---|---|---|
| **Why Is It So Hard to Do My Work? The Challenge of Attention Residue When Switching Between Work Tasks** | Sophie Leroy | 2009 | *Organizational Behavior and Human Decision Processes*, Vol. 109, No. 2, pp. 168–181 | Study 1: **84 undergraduate students**. Study 2: **"Not fully specified in available sources"** | **Laboratory experiments** — lexical decision task to measure attention residue; word puzzles + resume evaluation tasks | • **Unfinished tasks produce more attention residue than completed tasks** (goal activation: **583.63 ms vs 610.24 ms** reaction time; p=.01)<br>• **High time pressure during task completion reduces attention residue** vs low time pressure (among finished-task conditions: p=.002)<br>• Attention residue **persists throughout the subsequent task**, degrading performance | Academic (University of Minnesota); no vendor funding disclosed | **"Accurately represented in secondary literature, though often oversimplified. The key nuance is that time pressure helps close cognitive loops, reducing residue."** | https://2024.sci-hub.se/6959/475cee7acb5ba3bfbc239617bc124d00/leroy2009.pdf (piracy mirror — **do not cite this URL**; cite the journal) |
| **How Much Time and Energy Do We Waste Toggling Between Applications?** | **"Not explicitly named in HBR article (corporate research)"** | 2022 | *Harvard Business Review*, Aug 29 2022 | **137 employees across 20 teams at 3 Fortune 500 companies** | **TELEMETRY** — PC activity logging for up to 5 weeks; **3,200 days of work logs** analyzed | • **~1,200 app/website toggles per day per worker**<br>• **~4 hours per week (9% of work time) spent reorienting after switches**<br>• **65% of switches followed by another within 11 seconds** | **"Unclear from available sources; HBR does not disclose funding. This is not vendor-funded research but is based on proprietary telemetry data."** ← internally contradictory; see Gaps | (not assessed) | https://hbr.org/2022/08/how-much-time-and-energy-do-we-waste-toggling-between-applications |

**Note on the Leroy numbers:** the report presents "583.63 ms vs 610.24 ms" without stating which value belongs to which condition. In a lexical-decision paradigm, *faster* response to task-related words indicates *higher* goal activation, i.e. more residue — so 583.63 ms is presumably the unfinished-task condition. The report does not say. **Verify against the journal article before quoting the millisecond figures.**

### Section 3: Tool sprawl and application switching — industry sources (ALL vendor-funded)

| Study | Authors | Year | Venue | Sample & population | Methodology | Actual finding with real numbers | Funder — **VENDOR FLAG** | Popular framing accurate? |
|---|---|---|---|---|---|---|---|---|
| **Anatomy of Work Index** (2021, 2022, 2023, 2024) | Asana (corporate research team) | 2021–2024 | Asana corporate reports | 2021: not specified. **2022–2023: 9,615 global knowledge workers** | **SELF-REPORT SURVEY (not telemetry)** | • Average knowledge worker uses **9–13 different applications per day**<br>• Workers **switch between 10 apps 25 times per day** (2022 report)<br>• Majority of time spent on "work about work" (searching, updating, switching) | **VENDOR-FUNDED — Asana sells work management software** | "Accurately represented, but the methodology is **self-report, not observed behavior**." |
| **The Cost of Confusion in the Digital Workplace (Workgeist Report)** | Qatalog + Cornell University Ellis Idea Lab | 2021 (follow-ups 2023) | Qatalog corporate report / Cornell partnership | **1,000 workers** | **SELF-REPORT SURVEY (not telemetry)** | • **59 minutes per day** wasted searching for information across apps<br>• **43%** spend too much time switching between software<br>• **45%** say switching makes them less productive<br>• **9.5 minutes to return to productive flow after switching apps** ("widely repeated but methodological details are sparse") | **VENDOR-FUNDED — Qatalog sells an AI work hub** | **"Methodologically weak.** The 9.5-minute figure is not well-documented in peer-reviewed literature. The survey methodology is appropriate for self-reported perceptions but **cannot measure actual time loss**." |
| **Breaking Down the Infinite Workday** (special report) | Microsoft Work Trend Index team | 2025 (June 17, 2025) | Microsoft corporate report | **Millions of Microsoft 365 users** (telemetry) + **31,000 workers surveyed across 31 markets** | **TELEMETRY (M365 productivity signals) + survey** | • Employees interrupted **every 2 minutes** during core hours (9–5)<br>• **275 interruptions per day** (meetings, emails, chats)<br>• **Top 20% of users by ping volume receive the 275 figure (not average across all users)**<br>• **40% never get 30 minutes of uninterrupted focus** | **VENDOR-FUNDED — Microsoft sells Copilot and M365** | "**Mostly accurate**, but the 275 figure applies to the **top 20% of users, not the average**. The 'every 2 minutes' figure is the average interval between pings during 8-hour workdays." |
| **RescueTime aggregate productivity data** | RescueTime (corporate research) | 2023–2024 | RescueTime blog / corporate reports | **RescueTime users — self-selected; exact N not disclosed** | **TELEMETRY (time-tracking software)** | • Average knowledge worker uses **9.4 different applications daily**<br>• Specific context-switching frequency data **not fully disclosed** in available sources | **VENDOR-FUNDED — RescueTime sells time-tracking software** | "Data is credible but **limited transparency on methodology and sample characteristics**." |

### Section 4: Project resumption — the closest thing that exists (also vendor-funded)

| Study | Authors | Year | Venue | Sample & population | Methodology | Actual finding with real numbers | Funder — **VENDOR FLAG** | Popular framing accurate? |
|---|---|---|---|---|---|---|---|---|
| **State of Developer Onboarding 2024** | Pluralsight (corporate research) | 2024 | Pluralsight corporate report | **"Not fully specified; industry survey"** | **SELF-REPORT SURVEY** | • Median time-to-first-meaningful-PR: **2–4 weeks on clean codebases vs 6–12 weeks on smell-dense codebases**<br>• **72% of new developers wait 1–3 months before first meaningful PR** (Cortex 2024 survey of **50 engineering leaders**) | **VENDOR-FUNDED — Pluralsight and Cortex both sell developer productivity tools** | "**This measures new hire onboarding, not returning to unfamiliar code after time away.** The 'context reconstruction' cost is implied but not directly measured." |

**Severe provenance warning on this row:** in the footnote list these onboarding numbers trace to `onboardingcost.com/engineering`, `developeronboardingcost.com/hidden-costs`, `codesmellcost.com/methodology`, `codesmellcost.com/onboarding-drag`, `codeligence.com/research/onboarding-acceleration`, `devups.dev`, and a LinkedIn post. These are single-purpose keyword-exact domains of the type the prompt's source rules were written to exclude. **Treat the 2–4 / 6–12 week and 72% figures as unusable.**

### Section 5: Counter-evidence

| Study | Authors | Year | Venue | Sample & population | Methodology | Actual finding with real numbers | Funder | Report's assessment | Link |
|---|---|---|---|---|---|---|---|---|---|
| **Differential Effects of Interruptions and Distractions on Working Memory Processes in an ERP Study** | Bianca Zickerick et al. | 2020 | *Frontiers in Human Neuroscience* | **16 adults** (after exclusions from initial 22) | **Laboratory experiment** — EEG + behavioural; Continuous Number Task with WM load | • **No negative effect of interruptions on WM performance** under load<br>• **Performance IMPROVED after distractions** (p<.05)<br>• P3 amplitude increased after all interference types, suggesting enhanced attentional reallocation | Academic (Leibniz Research Centre, German federal funding) | **"This contradicts the dominant narrative.** Interruptions did not impair performance in this task; distractions actually improved it." | https://pmc.ncbi.nlm.nih.gov/articles/PMC7088125/ |
| **A Literature Review on Positive and Negative Effects of Interruptions** | Tom Gross, Michael von Kalben | 2023 | INTERACT 2023 (IFIP Conference on HCI) | Literature review (not primary research) | **Systematic literature review** | • **Positive effects documented:** simple tasks completed faster, information delivery, incubation/creativity, social connectedness, awareness<br>• **Negative effects documented:** time consumption, errors, stress, negative emotions, memory loss<br>• **Effects are context-dependent; interruptions are not uniformly harmful** | Academic (University of Bamberg) | **"This review explicitly calls for a more balanced approach. The productivity-loss narrative is overstated; interruptions have benefits in some contexts."** | https://cml.hci.uni-bamberg.de/~gross/publ/interact23_gross_von_kalben_interruption_lit_rev.pdf |
| **The Microstructure of Work: Understanding Productivity Benefits and Costs of Interruptions** | **"Not fully specified"** | 2022 | *Manufacturing & Service Operations Management* | Agribusiness workers (field study) | **Field experiment** | • **Pauses** (non-task-switching interruptions) **improved productivity** short- and long-term<br>• **Scheduled breaks hurt short-term but helped long-term** productivity<br>• **Task-switching interruptions hurt productivity**<br>• Different interruption types have **opposite effects** | **"Not specified in available sources"** | "This demonstrates that **not all interruptions are equal**. The blanket claim 'interruptions reduce productivity' is too coarse." | https://pubsonline.informs.org/doi/10.1287/msom.2021.1053 |
| **Effects of interventions to reduce the negative consequences of interruptions on task performance** (meta-analysis) | Jingya Guo et al. | 2021 | *Applied Ergonomics* | **33 laboratory experiments, 49 interventions** | **Systematic review and meta-analysis** | • Interventions significantly **increased primary task accuracy (SMD = 1.03, p=0.001)**<br>• Interventions significantly **reduced resumption lag (SMD = −0.51, p<0.001)**<br>• **No significant difference** for interrupting task accuracy<br>• Effects vary by intervention type and task type | Academic (University of Hong Kong) | "Shows that **interventions can mitigate interruption costs**, suggesting the effect is not immutable." | https://pubmed.ncbi.nlm.nih.gov/34273814/ |

---

## Vendor-Funded Research — Complete Flag List

| Source | Vendor | Product they sell | Methodology | Used in report's "strongest claims"? |
|---|---|---|---|---|
| Asana, *Anatomy of Work Index* | **Asana** | Work management software | Self-report survey (N=9,615) | No |
| Qatalog + Cornell, *Workgeist / Cost of Confusion* | **Qatalog** | AI work hub | Self-report survey (N=1,000) | No — placed in CLAIMS TO AVOID |
| Microsoft, *Work Trend Index / Breaking Down the Infinite Workday* | **Microsoft** | Copilot, M365 | Telemetry (millions of M365 users) + survey (31,000 workers) | **Yes — claim #3** |
| RescueTime aggregate data | **RescueTime** | Time-tracking / productivity software | Telemetry, self-selected users, N undisclosed | No |
| Pluralsight, *State of Developer Onboarding 2024* | **Pluralsight** | Developer productivity tools | Self-report survey, N unspecified | No |
| Cortex 2024 survey | **Cortex** | Developer productivity tools | Survey of 50 engineering leaders | No |
| HBR toggle study (2022) | **Undetermined** — report says funding "unclear," authors "not explicitly named," then asserts "not vendor-funded" without evidence | Unknown | Telemetry (N=137, 3,200 days of logs) | **Yes — claim #2** |

**Not vendor-funded (clean):** Mark et al. 2005 (UC Irvine); Mark et al. 2008; Gonzalez & Mark 2004; Leroy 2009 (University of Minnesota); Altmann & Trafton 2004 (**U.S. Office of Naval Research** — government, defence, not commercial); Zickerick et al. 2020 (Leibniz / German federal); Gross & von Kalben 2023 (University of Bamberg); Guo et al. 2021 (University of Hong Kong). MSOM 2022: funding not specified.

---

## STRONGEST CITABLE CLAIMS (report's own list, verbatim)

1. **"When interrupted work is resumed the same day (77% of cases), the average elapsed time before resumption is approximately 25 minutes, during which workers complete an average of 2 other tasks"** — Mark, Gonzalez & Harris, CHI 2005; N=24 info workers, 700+ hours observation.
2. **"Knowledge workers toggle between applications and websites approximately 1,200 times per day, spending nearly 4 hours per week (9% of work time) reorienting after each switch"** — HBR 2022; N=137 workers, 3 Fortune 500 companies, 3,200 days telemetry.
3. **"Employees using Microsoft 365 are interrupted every 2 minutes during core work hours (275 times per day for the top 20% of users by ping volume)"** — Microsoft Work Trend Index 2025; millions of M365 users, telemetry. *(Vendor-funded — Microsoft.)*
4. **"Unfinished tasks produce more attention residue than completed tasks, degrading performance on subsequent tasks; high time pressure during task completion reduces this residue"** — Leroy, OBHDP 2009; N=84 undergraduates, lab experiment.
5. **"Resumption lag after interruption is approximately double the uninterrupted inter-action interval (3.8 seconds vs. 1.9 seconds in a complex task); environmental cues available before interruption reduce this lag"** — Altmann & Trafton, CogSci 2004; N=96 undergraduates, lab experiment.

**My assessment of the five:** #1 and #4 are the safest (peer-reviewed, primary PDFs obtained, correctly qualified). #5 is the only one that supports the *mechanism* of the product, but is in seconds and from a lab task — do not scale it to minutes. #3 must always carry the "top 20%" and "Microsoft-funded" qualifiers. #2 is the most quotable and the most fragile: undetermined authorship and funding.

---

## CLAIMS TO AVOID (report's own list, verbatim)

1. **"It takes 23 minutes and 15 seconds to refocus after an interruption"** — "This exact figure does not appear in Gloria Mark's published papers. The closest is 25:26 (Mark et al., CHI 2005), which measures elapsed time to resumption (not cognitive recovery), applies only to same-day resumptions (77% of cases), and includes time spent on ~2 other tasks."
2. **"Interruptions always reduce productivity"** — "Counter-evidence shows interruptions can improve performance on simple tasks, enable information delivery, foster creativity through incubation, and improve social connectedness. Effects are context-dependent."
3. **"9.5 minutes to return to productive flow after switching apps" (Qatalog/Cornell)** — "This figure is widely repeated but methodologically weak (self-report survey, N=1,000; no peer-reviewed publication; sparse methodological details). Use with explicit caveats."
4. **"The average worker receives 275 interruptions per day"** — "This applies to the top 20% of users by ping volume in Microsoft's telemetry, not the average across all workers."
5. **"Developers lose X hours per week to context reconstruction after time away from a project"** — "No peer-reviewed research directly measures this. Onboarding research (2–4 weeks to first PR) is the closest proxy but does not address returning developers."

---

## Unusable or Unsourced

| Item | Why |
|---|---|
| **"23 minutes 15 seconds"** | Not in the primary literature. Do not use in any form. |
| **"9.5 minutes to return to productive flow"** (Qatalog) | Vendor-funded self-report; "methodological details are sparse"; no peer-reviewed publication |
| **"275 interruptions per day" as an average** | Applies to top 20% of users only |
| **Pluralsight/Cortex onboarding figures (2–4 wks, 6–12 wks, 72%)** | Vendor-funded, N unspecified, and the footnote trail runs through keyword-exact domains (onboardingcost.com, developeronboardingcost.com, codesmellcost.com, codeligence.com) that look synthetic |
| **RescueTime "9.4 apps daily"** | Vendor telemetry on a self-selected user base with undisclosed N and undisclosed methodology |
| **Gonzalez & Mark 2004 sample size** | Report could not determine it: "Not fully specified in secondary sources" — violating its own rule to go to the original paper. Do not cite "every 3 minutes" with an implied N. |
| **Leroy Study 2 sample size** | "Not fully specified in available sources" |
| **MSOM 2022 authors and funder** | Both "not fully specified" / "not specified." Do not cite by author. |
| **HBR 2022 authorship and funding** | Not established. If this is your headline stat, verify the authorship independently before the deck goes out. |
| **The Leroy sci-hub link** | Piracy mirror; cite *Organizational Behavior and Human Decision Processes* 109(2):168–181 instead |
| **Anything traced only to blog domains in the footnote list** | e.g. contextcost.com, cognitivethoughtengine.com, superglobalcalculator.com, aiosbrain.dev, hindsight.work, unreliant.com, codepulsehq.com, imandyoneil.com — high risk of AI-generated content restating the same folklore |

---

## GAPS (report's own list, verbatim summary) — including the critical one

1. **Multi-day/multi-week project resumption cost** — *"No peer-reviewed research measures the time or cognitive cost of returning to a project after days or weeks away. This is the narrowest and most important question you asked, and **the research does not exist**. Software engineering onboarding research (time-to-first-PR) is the closest proxy but measures new hires, not returning developers."*
2. **Context reconstruction in software engineering** — *"No studies directly measure the 'context reconstruction' cost for developers returning to unfamiliar or semi-familiar codebases after time away. Anecdotal evidence and vendor reports (Pluralsight, Cortex) exist but are not peer-reviewed."*
3. **Telemetry-based tool-switching frequency outside the Microsoft ecosystem** — *"Most telemetry data comes from Microsoft (M365) or RescueTime (self-selected users). Independent, peer-reviewed telemetry studies of application switching are scarce."*
4. **Long-term cognitive effects of chronic interruption** — *"Most studies measure immediate or same-day effects. Long-term impacts on creativity, deep work capacity, or skill development are not well-studied."*
5. **Individual differences in interruption resilience** — *"Some research (Mark 2008; Guo 2021 meta-analysis) touches on this, but systematic understanding of who is most/least affected by interruptions is limited."*

**From Section 4, the explicit non-existence statement (verbatim):**
> **"CRITICAL GAP: I found no peer-reviewed research specifically measuring the cost of returning to a project after days or weeks away (as distinct from momentary interruption or same-day resumption)."**
>
> **"No peer-reviewed studies were found that directly measure:**
> - Time cost of returning to a codebase after 1+ weeks away
> - Cognitive load of 'context reconstruction' for returning developers
> - Comparison of fresh vs. returning developer productivity on familiar-but-distant projects
>
> **This is a genuine research gap.** If you need to make claims about this, you would be extrapolating from onboarding research or interruption research—neither of which directly addresses the multi-day/multi-week resumption question."

**Report's overall bottom line (verbatim):**
> *"The empirical foundation for 'interruptions cost time' is real but nuanced. The 23-minute figure is a distorted popularization. The strongest evidence is for same-day resumption lag (~25 minutes), high-frequency toggling (~1,200/day), and attention residue from unfinished tasks. The multi-day project resumption question you care about most has no direct research—that gap is itself a valuable finding."*

### Gaps the report does not flag (my findings)
- **The report never went to the Gonzalez & Mark 2004 primary paper**, despite that being an explicit source rule, and so cannot state its N. It also could not identify the authors of the MSOM 2022 paper it cites as counter-evidence.
- **The HBR funding claim is self-contradictory** ("Unclear from available sources" → "This is not vendor-funded research"). An unverified negative should not have been asserted, particularly for a claim promoted to the top-5 list.
- **No research at all on privacy or acceptability of passive observation software** — the prompt didn't ask, but this is the largest unexamined adoption risk for the product.
- **No research on whether tools that restore context actually improve outcomes.** Guo 2021 shows interventions reduce resumption lag in *lab* settings; there is no field evidence that a software layer restoring files/tabs/apps changes any real work outcome.

---

## Contradictions to the Positioning

Positioning under test: *"a local-first cognitive layer that preserves work continuity across your computer, letting you return to any project exactly where you left off."*

| # | Contradiction | Evidence |
|---|---|---|
| 1 | **The exact problem the product solves has no measured cost.** No peer-reviewed research on multi-day/multi-week project resumption. You cannot quantify the pain you are selling relief from. | Report Section 4 CRITICAL GAP; GAPS #1, #2 |
| 2 | **The canonical field study says fragmentation is intra-day, not multi-day.** 77.2% of interrupted work was resumed the **same day**, average time in a working sphere before switching was **11 min 4 sec**. The 2005 data describes minute-to-hour churn, which macOS window restore and browser session restore already partly address — not the days-or-weeks return the product targets. | Mark, Gonzalez & Harris, CHI 2005 |
| 3 | **"Lost time" is the wrong frame.** During the 25-minute resumption gap, workers completed **2.26 other work tasks**. They were not idle. The report says so explicitly: *"not staring blankly recovering focus."* Any "X hours lost per week" slide built on this number is dishonest. | Report's 23-minute trace, p. 4 |
| 4 | **The best lab experiment found interruption makes people FASTER.** Interrupted tasks: 20.31 / 20.60 min vs 22.77 min baseline, p<.05, with no error increase. The measurable harm was stress and workload, not time. This directly undercuts a time-savings ROI pitch. | Mark, Gudith & Klocke, CHI 2008 |
| 5 | **A 2023 systematic review says the narrative is overstated.** "Interruptions are not uniformly harmful"; documented positive effects include incubation/creativity, information delivery, faster completion of simple tasks. | Gross & von Kalben, INTERACT 2023 |
| 6 | **An EEG study found interruptions did not impair working memory and distractions improved performance** (p<.05). N=16 is small, but it is peer-reviewed counter-evidence in the direction that matters. | Zickerick et al., Frontiers in Human Neuroscience 2020 |
| 7 | **Not all interruptions are equal, and some help.** Pauses improved productivity short- and long-term; scheduled breaks helped long-term. Only *task-switching* interruptions hurt. A blanket "context switching is costly" claim is refuted. | MSOM 2022 |
| 8 | **The one supporting mechanism is measured in seconds, not minutes.** Altmann & Trafton is the only study showing that pre-interruption cues reduce resumption lag — the scientific basis for context restoration. The effect: **3.8s → toward 1.9s**, in a continuous lab task with undergraduates. Do not scale it. | Altmann & Trafton, CogSci 2004 |
| 9 | **Leroy's mechanism argues for closing loops, not preserving them.** Attention residue comes from *unfinished* tasks, and **time pressure reduces residue by helping people close cognitive loops**. The therapeutic implication is "help people finish," which is at least adjacent to, and arguably in tension with, "make it painless to leave things unfinished and come back later." | Leroy, OBHDP 2009 |
| 10 | **Interruption costs are already mitigable by cheap interventions.** Guo 2021 meta-analysis: interventions reduce resumption lag (SMD = −0.51) and increase accuracy (SMD = 1.03). The report's read: "the effect is not immutable." That weakens the case that a dedicated paid product is required. | Guo et al., *Applied Ergonomics* 2021 |
| 11 | **"Local-first" has no evidence base in this document.** Zero sources on privacy preference, local-vs-cloud willingness to pay, or acceptability of passive observation. | Absence across 109 sources |

---

## The honest reframe this evidence supports

The evidence does **not** support "you lose 23 minutes / X hours per week and we give it back." It does support a narrower, defensible pair of claims:

1. **Work is objectively fragmented at high frequency.** ~1,200 app/website toggles per day, 65% of switches followed by another within 11 seconds (HBR 2022 telemetry, N=137, 3,200 days of logs); a working sphere lasts 11 min 4 sec before switching, and 57% are interrupted (Mark et al., CHI 2005). *Fragmentation is measured and real.*
2. **The cost of interruption is affective and cognitive, not primarily temporal.** Interrupted work is completed as fast or faster, but with significantly higher stress, frustration, time pressure, effort and workload (Mark, Gudith & Klocke, CHI 2008); and unfinished tasks leave attention residue that degrades the next task (Leroy, OBHDP 2009).
3. **Cues present before you leave reduce the cost of coming back** (Altmann & Trafton, CogSci 2004) — the mechanism, stated at the scale it was measured.

Sell the reduction in *cognitive burden and stress*, backed by CHI 2008 and Leroy. Do not sell recovered hours. And treat the absence of multi-day resumption research as the wedge it is: nobody has measured this problem, which is consistent with nobody having built for it — say that explicitly rather than papering over it with a borrowed statistic.
