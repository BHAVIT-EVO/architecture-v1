# 08 — Permissions & Legal Digest

**Source artifact:** `uploads/Executive Summary.pdf` (research report on macOS permission friction, privacy-law exposure, and legal risk for software that observes user activity)
**Total pages:** 15. Body = pages 1–12 (~11.5 pages). Sources begin mid-page 12 (nos. 1–17), run through pages 13–15 (nos. 18–116). **116 numbered sources.**
**Report self-timestamp:** "as of August 2026" (legal analysis explicitly dated).
**Digested:** 2026-09-01. Permanent record for the permissions/onboarding and legal-risk slides.

**Body citation range:** inline refs run roughly [1]–[70]. **Sources ~[71]–[116] (about 46 of 116, ~40%) are never cited in the body** — padding.

**Scope gap up front:** the task asked for macOS 14 / 15 / 26 coverage. The report covers Monterey (12.3) through Sequoia (15.1). **It does not cover macOS 26 (Tahoe) at all.** Any Tahoe-era permission claim is out of this report's scope.

---

## 1. THE HEADLINE: there is no permission drop-off data

This is the single most important finding and it is a negative.

> "Direct quantitative data is scarce... No published A/B test data, no funnel analytics from major apps (Raycast, CleanShot, etc.)... This is a genuine evidence gap."

**There are no permission-funnel conversion numbers in this report.** No drop-off percentage, no abandonment rate, no before/after for any onboarding pattern. If the deck needs "X% of users abandon at the Screen Recording prompt," this report cannot supply it and says so honestly.

The only qualitative signal is one developer's testimony, sourced to a personal blog:

> "the single biggest friction point in the user experience, and there's no programmatic workaround."
> — sourced to `dev.to/thehwang` [3]

| What was asked | What the report has | Confidence |
|---|---|---|
| Permission drop-off / abandonment rate | **Nothing — explicit evidence gap** | n/a |
| A/B test data from Raycast, CleanShot, etc. | **None exists / not published** | n/a |
| Qualitative "biggest friction point" claim | One dev blog quote [3] | Low (single anecdote) |

---

## 2. Onboarding patterns (how shipping apps ask for permissions)

Qualitative comparison, no conversion data attached to any row.

| App | Permissions requested | Timing | How they explain it |
|---|---|---|---|
| Raycast | Accessibility only | Immediate | "Why this is needed" copy |
| CleanShot X | Screen Recording + Accessibility | Just-in-time | Custom modal before the OS prompt |
| Rectangle | Accessibility only | Immediate | Minimalist |
| Bartender | Accessibility only | Immediate | + Help link |
| Loom | Screen Recording + Microphone | Just-in-time | Video explainer |
| Krisp | Microphone, then Accessibility | Staged | Sequential asks |
| Rewind | Screen Recording + Accessibility | Aggressive | Local-only landing page as reassurance |

**Read:** just-in-time asks with a custom pre-prompt modal (CleanShot X) and local-only reassurance (Rewind) are the recurring best-practice patterns. No numbers prove any of them convert better — this is pattern description, not measured optimization.

---

## 3. macOS version timeline (permission mechanics)

| Version | Date | Relevant permission change |
|---|---|---|
| Monterey 12.3 | 2022 | ScreenCaptureKit introduced |
| Ventura 13.x | 2022 | — |
| Sonoma 14.x | 2023 | `SCContentSharingPicker` made public |
| Sequoia 15.0 | Sept 2024 | **Monthly re-authorization prompt for Screen Recording** (weekly in beta before backlash) |
| Sequoia 15.1 | Oct 2024 | Re-auth cadence reduced for "regularly used" apps |
| Sequoia 15.x | 2024–25 | **Self-signed / non-notarized apps fail** the flow |

**This is the concrete Apple-friction signal the competitive report missed:** Sequoia added recurring re-auth prompts for screen capture, beta was weekly, backlash forced Apple to relax it. If Evo continuously observes the screen, this cadence is a live UX tax. **Note again: no macOS 26 / Tahoe data here.**

---

## 4. App Store viability (the distribution problem)

| Claim | Detail | Source quality |
|---|---|---|
| Mac App Store cannot ship Accessibility-API apps | Sandboxing forbids it | **Developer FORUM post [4], not official Apple docs** |
| — verbatim | "It is not possible to use the accessibility API from a sandboxed app..." | Forum [4] |
| Feb 2026: Apple rejecting Accessibility-API apps under **Guideline 2.4.5** | Cited as a recent enforcement trend | **Vendor blog `whispernotes.app` [5] only** |
| — Whisper Notes verbatim | "Accessibility features are intended to help users with different capabilities... Apps may not use features designed to increase accessibility for other purposes." | Vendor blog [5] |
| Apps distributing outside the App Store | TextExpander, Keyboard Maestro, Alfred | — |
| Developer Program cost | $99/yr | — |
| App Store commission | 15–30% | — |

**Both load-bearing distribution claims are weakly sourced** — a developer forum and a single vendor's blog, not `developer.apple.com`. The "Feb 2026 rejection wave under 2.4.5" is exactly the kind of claim that needs Apple's own guideline text or a first-party rejection notice before it goes near an investor.

---

## 5. Privacy law — the controller question (most important legal finding)

| Regime | Report's conclusion | Basis | Confidence |
|---|---|---|---|
| **GDPR** | **Likely a data controller even for local-only processing** | Article 4(7) definition; EDPB Opinion 22/2024 | Report's stated view |
| GDPR minority view | "Zero access" (never touches data) might avoid controller status | Untested theory | **Explicitly untested** |
| **CCPA/CPRA** | Likely "service provider" or "contractor," but unsettled | CCPA service-provider provisions | Unsettled |

**Bottom line for the deck's privacy claim:** local-only processing does **not** clearly exempt a vendor from GDPR controller obligations. The report's read is that determining purposes and means of processing likely makes you a controller regardless of where computation happens. The "we never see the data, so we're not a controller" argument exists but is untested in law. **Do not assert "local-only means no GDPR obligations" as settled — the report does not support that.**

---

## 6. Wiretapping / consent-to-record exposure

| Item | Detail |
|---|---|
| Federal baseline | ECPA — one-party consent |
| All-party ("two-party") consent states | **12 listed:** CA, CT, DE, FL, IL, MD, MA, MT, NH, OR, PA, WA |
| Source for the 12-state list | **`basilai.app` [7] + `recordinglaw.com`, not the statutes themselves** |
| Open question | Whether screen-only observation (no audio) even triggers wiretap statutes |

**The 12-state list is sourced to an SEO/AI content site and a recording-law blog, not to the state penal codes.** The list is directionally standard, but if a number or a state goes on a compliance slide it should be checked against the actual statute.

---

## 7. The four questions that need a lawyer (report's own list)

Report frames these as unresolved and requiring counsel, "as of August 2026":

| # | Question | Which counsel |
|---|---|---|
| 1 | GDPR controller status for a local-only observer | EU privacy counsel |
| 2 | Whether screen-only (no-audio) recording triggers wiretap statutes | US counsel |
| 3 | Whether local-only processing avoids a HIPAA Business Associate Agreement | Healthcare counsel |
| 4 | CCPA "service provider" vs. "business" classification | California counsel |

These four are the report's actual answer to "what needs a lawyer." All four are genuinely unsettled; none has a clean yes/no in the document.

---

## 8. Privacy positioning language (for the trust/messaging slide)

Successful privacy-first self-descriptions the report collected:

| Company | Line |
|---|---|
| Signal | "We don't know anything about you" |
| Proton | "Privacy by design" |
| Obsidian | "Your data is saved locally on your device. No account is required, no telemetry data is collected" |
| DuckDuckGo | "We don't track you" |
| Apple | "Privacy is a fundamental human right" |
| Tailscale | "Zero-trust networking" |

Cited privacy failures (what not to do): Facebook / Cambridge Analytica; Google "Incognito"; Zoom (held encryption keys while claiming E2E); Grindr.

**Obsidian's line is the closest template** for a local-first desktop tool: it names local storage, no account, and no telemetry in one sentence.

---

## 9. Report Quality Problems

This report is **more honest than most in the batch** (it declares the drop-off evidence gap outright), but its sourcing is heavily contaminated by SEO/AI content farms, and one source is disqualifying.

### 1. A social-media AI bot is cited as a source
Source **[67] = `x.com/grok/status/2028967400578646334`** — a Grok (X's AI) status post, cited as the source for a claim about a Tailscale security audit. **An AI chatbot's tweet is not a source.** Do not reproduce anything resting on [67].

### 2. Load-bearing legal facts sourced to SEO/AI content farms, not primary law
The 12-state wiretap list comes from `basilai.app` [7] and `recordinglaw.com`, not statutes. The broader source base is thick with keyword-domain content mills: `basilai.app`, `crawlix.app`, `moonbase.sh`, `my-ssl.com`, `groovyweb.co`, `sirion.ai`, `konarkpro.com`, `curvecompliance.com`, `zight.com`, `filarr.com`, `zpxe.com`, `tryskilly.app`, `screenify.studio`, `specswriter.com`, and more. For a report whose entire value is legal precision, primary sources (GDPR text, EDPB Opinion 22/2024, CCPA regulations, state penal codes, `developer.apple.com`) are largely absent.

### 3. The two App Store distribution claims rest on a forum post and a vendor blog
"MAS can't ship Accessibility apps" → developer **forum** [4]. "Apple rejecting under Guideline 2.4.5 in Feb 2026" → vendor blog **`whispernotes.app` [5]**. Neither is Apple's own documentation. Both are decisions the go-to-market plan depends on; both need first-party confirmation.

### 4. No macOS 26 (Tahoe) coverage despite the task asking for it
The task specified macOS 14/15/26. The report stops at Sequoia 15.1 (Oct 2024). Every Tahoe-era permission behavior is simply missing — treat this report as silent on the current OS, not reassuring about it.

### 5. The most useful number does not exist
Permission drop-off/abandonment data — the thing an onboarding slide most wants — is absent, and the report says so. That honesty is a virtue, but it means **no conversion figure from this report can be cited**, because there isn't one.

### 6. Citation padding
Roughly 46 of 116 sources (~40%), the [71]–[116] range, are never cited in the body. The source count overstates research depth by roughly 40%.

---

## 10. What is safe to use vs. what must be re-verified

| Claim | Confidence | Notes |
|---|---|---|
| There is no published permission drop-off / A/B data | **High** | Report states it as a genuine evidence gap |
| Onboarding patterns (just-in-time, pre-prompt modal, local-only reassurance) | **Medium** | Described accurately; no conversion data behind them |
| Sequoia 15.0 added recurring (monthly; weekly-in-beta) Screen Recording re-auth prompts | **Medium-high** | Concrete, matches known Apple history; verify exact cadence |
| Local-only processing likely still makes you a GDPR controller | **Medium** | Report's reasoned view via Art. 4(7) + EDPB 22/2024; the "zero-access" exemption is untested |
| The four questions needing counsel (GDPR / wiretap / HIPAA BAA / CCPA class) | **High (as open questions)** | Correctly framed as unresolved |
| 12 all-party consent states list | **Medium — verify against statutes** | Sourced to `basilai.app` + `recordinglaw.com`, not law |
| MAS cannot ship Accessibility apps | **Low — verify** | Developer forum, not Apple docs |
| Apple rejecting Accessibility apps under 2.4.5 (Feb 2026) | **Low — verify** | Single vendor blog |
| Anything about macOS 26 / Tahoe | **Not covered** | Out of scope in this report |
| Anything resting on source [67] (x.com/grok) | **Do not use** | AI chatbot tweet |

### Open verification tasks
1. Pull the actual GDPR Article 4(7) text and EDPB Opinion 22/2024 to confirm the controller reasoning.
2. Confirm the 12 all-party-consent states against current statutes (list changes; some states' case law is contested).
3. Get first-party confirmation of the Guideline 2.4.5 Accessibility-rejection trend (Apple docs or a real rejection notice), not a vendor blog.
4. Add macOS 26 / Tahoe permission behavior — this report does not cover it.
5. Have counsel answer the four open questions before any compliance claim ships.
