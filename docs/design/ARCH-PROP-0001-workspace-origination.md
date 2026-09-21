# ARCH-PROP-0001 — Workspace Origination: When Is Something Actual Work?

**Status:** **NOT ADOPTED.** Superseded by RFC-0014 (Engagement Contract). Retained as a record — see the Decision Record below. Never was authority; is not authority now.
**Method:** every claim below is grounded in a clause of the frozen corpus (cited as `doc §clause` / `doc:line`) and, where behavioral, in source read at `crates/…:line`. No production code was written; no authority document was modified.
**Builds on:** BE-AUDIT-0001, BE-TRACE-0001, ARCH-GAP-0001, the four grounded fixes, the D1–D4 implementation phase (743 tests passing), and the Phase C adversarial validation (Cases A–G2).

---

# Decision Record — not adopted

This proposal asked exactly the right question and diagnosed the failure
correctly. Its central finding stands and was independently confirmed:

> "The current D1 floor (`prior_witness`) therefore conflates Artifact history
> with Workspace legitimacy — that is the root of the 100-workspaces Case D."

That is the same root cause RFC-0014 and Architecture Amendment 1 identify. The
diagnosis is kept on the record for that reason.

**The proposed remedy was rejected.** It answered "when is something actual work"
with a disjunction of two sufficient conditions:

> "(R) **relational continuity** — canonical `RepositoryMembership` evidence
> (RFC-0012, strength 0.8), **or** … the user's declaration (RFC-0011
> WorkDesignated, RFC-0012 WorkGrouped, RFC-0013 ContinuationSurface)"

Both branches are unacceptable, for reasons that are about the product rather
than about the corpus:

**The declaration branch inverts the product.** Requiring a declaration before
ordinary work can be recognized means Evo cannot reconstruct anything it was not
told about — and observing and reconstructing *without being told* is the entire
value of the product. The proposal acknowledges the cost in one line ("It costs
the single-resource non-repo engagement (one declaration)"), but the cost is not
one declaration: it is a declaration for every body of work a person ever has,
which is a manual bookkeeping product wearing Evo's name. Declaration must remain
*authoritative where it speaks* and *unnecessary where it does not* (RFC-0014
Requirement 11).

**The repository branch designs the model around Git.** It makes the one
Git-specific signal the sole automatic path to being recognized as work.
Everything not in a repository — a researcher's browser, PDF, and terminal; a
student's document, spreadsheet, and chat; a designer's drawing, folder, and
reference page — would then require a declaration under the other branch. The
proposal's own worked example concedes this shape ("Relational continuity only
(RFC-0012)" for the developer row). It also over-groups where separation matters
most: two unrelated activities in one repository share membership completely and
at full strength. RFC-0012 is now superseded in full for these reasons.

**Why the disjunction looked forced at the time.** The proposal reasoned inside
the frozen corpus, which is exactly what it set out to do, and the corpus then
forbade the third option: Architecture §5 restricted attachment evidence to "a
small, recent, local candidate set — never the full history", and RFC-0003
Requirement 2 forbade reading temporal proximity. With relational evidence across
history ruled out and temporal structure ruled out, the only remaining candidates
genuinely were declaration and repository membership. The proposal's conclusion
follows validly from its premises; the premises were the problem.

**What was adopted instead.** Both of those premises were changed rather than
worked around (Architecture Amendment 1; RFC-0003 v2.0 Amendments 1 and 2). Work
is recognized from **relationships among resources across the whole history**,
gated by three independent necessary conditions — more than one participant, a
return by a person on more than one occasion, and one sitting of sustained
attention — none of which any single resource can satisfy alone. This keeps the
proposal's correct diagnosis, discards `prior_witness`, and requires neither a
declaration nor a repository.

Normative authority: **RFC-0014 (Engagement Contract)**.

Everything below is the original proposal, unchanged, for the record. Its
citations of RFC-0012 and IS-0012 refer to documents that are now superseded.

---

# 0. The single question

> **WHEN SHOULD EVO DECIDE THAT SOMETHING IS ACTUAL WORK WORTH RESTORING, RATHER THAN SOMETHING THE USER MERELY TOUCHED?**

The answer this proposal defends, traced through the corpus:

> **Evo should decide that something is actual work exactly when canonical evidence establishes it as part of a body of work — and under the corpus's own evidence discipline, only two evidence classes can do that: the user's declaration (RFC-0011 WorkDesignated, RFC-0012 WorkGrouped, RFC-0013 ContinuationSurface) and relational continuity (RFC-0012 repository membership). Repeated observation of the same Artifact is evidence of Artifact history, not of engagement, and must never by itself originate a Workspace.**

Everything below is the argument, the alternatives, and the exact contract.

---

# 1. The four concepts, exactly as the corpus defines them

| Concept | Corpus definition | Authority |
|---|---|---|
| **Observation** | An immutable, witnessed, append-only fact: "at time T, Evo observed X." Never interpreted at write time. The only sacred table. | `ARCHITECTURE.md:47-52`, §3; `ARCHITECTURE.md:33-36` §2 |
| **Artifact** | A persistent-identity thing that exists independently of any single Observation (file, repo, URL, document). Observations reference Artifacts by stable ID. Identity resolution is lookup and deduplication, **not inference**. | `ARCHITECTURE.md:53-55`; IS-0004 R-2 |
| **Engagement** | A coherent period of directed activity toward some part of reality. Open engagements remain meaningfully continuable after attention leaves them. **Openness is a property of an engagement, not a separate object.** Engagements are never directly observed; they are inferred from observable evidence. | COGNITIVE_MODEL Principles I–II, VI–VIII |
| **Workspace** | Evo's current best explanatory hypothesis that **a collection of Artifact histories** collectively describe the evolution of **one coherent body of work**. Derived, disposable, replayable, persisted only for restoration speed. | RFC-0003 §Definition; IS-0011 §3; `ARCHITECTURE.md:56-58` |

**The load-bearing sentence** appears three times in the corpus:

> "…multiple Artifact histories collectively describe the evolution of one coherent body of work." — RFC-0003 §Abstract, §Definition; IS-0011 §3 (line 72)

The Workspace is defined by a *collection* of histories explaining *one* coherent body. A Workspace that contains exactly one Artifact forever is a degenerate case of the definition — and when the record produces thousands of them, the Workspace has collapsed into the Artifact (BE-AUDIT-0001 §14.5: "distinct in the model and identical in the record").

---

# 2. Answers to the ten questions

## A. The distinction between Observation / Artifact / Engagement / Workspace

- **Observation** is a witnessed fact — the log. It answers "what did Evo see?"
- **Artifact** is persistent identity — the deduplication of Observations across time. It answers "is this the same external entity?"
- **Engagement** is a cognitive property of the human — coherent directed activity. It is **never directly observed** (COGNITIVE_MODEL Principle VI); the observer infers it from evidence.
- **Workspace** is Evo's computational stand-in for an inferred open engagement — the hypothesis that some Artifact histories share one body of work (RFC-0003).

Critical asymmetry: Observation and Artifact are facts; Engagement and Workspace are interpretations. Law IV (Observation and interpretation must never be confused) and Law XVII (epistemic separation) require the boundary between "the user touched this" (Observation) and "this is a body of work" (Workspace) to remain structural, not cosmetic.

## B. Does the frozen architecture require every Workspace to represent an engagement?

**Yes.** The Workspace is the corpus's named computational representation of the open engagement (ARCH-GAP-0001 Part 1 Q1: "the unit is therefore already named by the corpus"). `ARCHITECTURE.md:58`: "the single durable … derived container representing an **ongoing body of work**." RFC-0003 §Definition: "one **coherent** body of work." A Workspace that asserts resumability for a one-off touch violates Constitution Article III ("occasionally silent is preferable to confidently wrong") and Law VI (under-interpretation over over-interpretation) — BE-AUDIT-0001 §6.2 classified the pre-D1 behavior as a FAIL against exactly those clauses.

## C. What evidence can legitimately establish engagement?

The corpus has already run this question as a **Competing-World Test** twice:

- **RFC-0010** (Work Continuity): requires evidence *distinguishable from observation order, recency, frequency, co-membership, focus, titles, modification, commits, temporal grouping, runtime state, similarity, LLM output* (RFC-0010 Requirement 4, Prohibited Evidence Sources).
- **RFC-0011** (Continuation Evidence): a 14-row candidate table. Every automatic signal — order, frequency, focus, titles, modification, commits, co-membership, temporal co-occurrence, unresolved conditions, supersession, similarity, runtime state, attachment confidence — produces *identical evidence* in WORLD A (genuine continuation) and WORLD B (mere association). **Only row 14 survives: explicit user declaration** (RFC-0011 §Evidence Candidate Analysis).
- **RFC-0012** (Co-membership): repository membership survives as a *relational continuity signal* — a repository is a witnessed, version-controlled history of co-evolution (strength 0.8, deliberately not 1.0, because a repo may hold several bodies of work). WorkGrouped, the user's explicit declaration, is conclusive (1.0).

**Conclusion:** under the corpus, the only legitimate engagement evidence is **the user's word** and **witnessed relational structure (repository membership)**. Everything else was already rejected by name.

## D. Is "prior witness" sufficient evidence of engagement, or merely evidence of Artifact history?

**Merely evidence of Artifact history.**

- RFC-0003 Req 5 requires formation to explain the *observed evolution of Artifact histories* and forbids basing formation *solely* on instantaneous system state. That makes a prior witness *necessary* for a minimal historical basis — but nothing in the corpus says a repeated observation *establishes a body of work*.
- Run the Competing-World Test on "witnessed twice": WORLD A (user genuinely works on file F across sessions) and WORLD B (user happened to open F twice, unrelated moments) produce **identical evidence** — "F observed at t1 and t2." By RFC-0011 rows 1–2 (observation order and frequency rejected) and RFC-0010 Req 4, this evidence cannot distinguish engagement from encounter.
- The Workspace definition requires a **collection of histories** explaining **one coherent body**. Two observations of one Artifact are one history, and they say nothing about coherence of a body.

**This is the root of Case D.** The D1 implementation made `prior_witness` the origination floor. `prior_witness` is indistinguishable from frequency/order — the exact evidence the corpus rejects. The D1 floor satisfies the *letter* of RFC-0003 Req 5 (not instantaneous) but not the *definition* of Workspace (collection → coherent body), and not RFC-0010 Req 4 (distinguishable from order/recency/frequency). D1 fixed the *first-touch* half of the problem; it left "opened twice" as an unquestioned truth. **The implementation has conflated Artifact history with Workspace legitimacy.**

## E. Does repeated observation prove continuing work?

**No — only that the user encountered the same resource again.** RFC-0010 distinguishes Chronology (A: "B happened after A") from Work Continuity (E: "the work continues with B") and explicitly prohibits collapsing them. RFC-0011 rejects both observation order (row 1) and frequency (row 2) as continuation evidence. A repeated observation is a fact about the *resource*; it is not a fact about the *user's engagement*.

## F. Could a Workspace be provisional before becoming an actual Workspace?

Two different meanings must be separated:

1. **Provisional in the "disposable interpretation" sense** — already true. RFC-0003 Req 6 (Provisional Identity), Law III (Interpretation Is Disposable): every Workspace is a hypothesis, replay may produce a different one, historical Observations never change.
2. **Provisional as a distinct canonical lifecycle state** (e.g., `Provisional → Active`) — a **new canonical semantic state on the Workspace**, which is an architectural decision no frozen document authorizes. `IS-0011` freezes the Workspace component set (Identity, Lifecycle, Attachment Set, Snapshot History); adding a state is a frozen-model change.

The honest reading: **no second state is needed.** The "provisional register" already exists as the AttachNowhere outcome plus the derived index's unattached-Artifact tracking (D1): an Artifact with history but no confirmed body is witnessed, indexed, searchable, and declarable — without being a Workspace. Proposal B below shows that any genuinely principled "promotion" rule reduces to the same evidence floor as Proposal A.

## G. Could explicit user declaration be the origination authority for standalone resources?

**Yes — and it is the strongest option available.** RFC-0011 row 14 is the only surviving Competing-World evidence. The user's own word is a *witnessed fact* ("the user declared continuation at S"), not an inference; it needs no Evo guess; it is deterministic and replayable (a declaration is a canonical Observation in the append-only log). PRODUCT.md's own interaction model is built on this: "If you want to continue your presentation… say so. Everything after that becomes Evo's responsibility."

The current boundary to amend: RFC-0011 §4 line 261 — "a WorkDesignated observation SHALL NOT by itself establish an Artifact or a Workspace." This proposal keeps the *Artifact* half (a declaration never creates identity — the content Observations establish the Artifact) and amends the *Workspace* half: a declaration, together with the Artifact's existing content Observations, MAY justify origination. That is a minimal, targeted amendment to a clause RFC-0011 itself flags as a boundary, not a rewrite.

## H. Could WorkGrouped be extended to establish origination without inventing semantics?

**Yes.** WorkGrouped is already the user's conclusive declaration (1.0, RFC-0012). The only reason it cannot originate today is the Origination Invariant: "co-membership evidence extends Workspaces; it never originates them." That invariant was written to prevent *automatic* membership from manufacturing Workspaces. A user-declared WorkGrouped pair is the opposite of automatic inference — it is the user's judgment (Law IX). The minimal amendment: **the user's WorkGrouped declaration, together with the content Observations that establish the declared Artifacts, MAY justify origination of a Workspace containing them; no automatic co-membership evidence ever originates.** No new evidence class, no new primitive, no new schema — the accepted `OBS-WORK-GROUPED/v1` and its daemon channel (already delivered to the desktop in D2) are reused verbatim.

## I. Would requiring declaration for standalone Workspaces make Evo too manual?

This is the honest tradeoff, and it must not be hand-waved.

- **The cost:** a user who works on one non-repository spreadsheet for three days and never declares anything would see *silence* on Home. PRODUCT.md says "Evo accepts work exactly as it happens" and "Evo remains invisible until needed." A strict declaration-only regime contradicts the *tone* of that promise for the single-resource case.
- **The mitigation (and the corpus's own answer):** the product's described interaction *is* a declaration — "You decide the intention. Evo reconstructs everything required to continue" (PRODUCT.md). The existing "Mark to continue" (designation) and "Related work — your call" (WorkGrouped) surfaces make declaring a one-click act, and the declaration retroactively brings the full witnessed record into a Workspace. Constitution Article III explicitly prefers this silence over confidently wrong Workspaces.
- **The decisive point:** the alternative — keeping any automatic origination signal — re-admits evidence the corpus already rejected (Section 2.D). There is no third register. Evo either asks the user's judgment (Law IX) or guesses with rejected evidence (Law VI).

The proposal therefore keeps **repository membership automatic** (the one automatic signal the corpus accepts, and the one the scenarios require not to regress) and makes **declaration the origination authority for everything else**.

## J. Is there a principled canonical evidence model that preserves automatic behavior while avoiding 100 Workspaces?

**Yes — and it is not a compromise; it is what the corpus already implies.**

> **Origination floor = the surviving Competing-World evidence classes, and only those:**
> (R) **relational continuity** — canonical `RepositoryMembership` evidence (RFC-0012, strength 0.8), **or**
> (D) **user declaration** — the Artifact is named in a current `WorkDesignated`, a `WorkGrouped` pair, or the current `ContinuationSurface` (RFC-0011/0012/0013).
> **Prior witness alone never originates.**

- **Automatic behavior preserved exactly where the corpus allows it:** repositories (100 files in one repo → 1 Workspace; Case E passes unchanged; Scenario 3 of the mandate "Do not regress").
- **100 unrelated standalone resources, two observations each → 0 Workspaces** (Case D becomes 0, not 100). The user's word, or a repo, is required to claim a body of work.
- No recency, no frequency, no time windows, no app identity, no similarity, no LLM, no new primitive, no new evidence class, no omission from Home (fewer *Workspaces* exist — nothing is hidden). Deterministic and replayable: all inputs are canonical Observations in the log.

---

# 3. Competing proposals

## PROPOSAL A — Declaration/relational-continuity origination (recommended)

**Behavioral rule:** A content Observation establishes Artifact A. Formation runs the unchanged Attachment Evaluation against existing Workspaces. Only when the decision would otherwise be *RecognizeNew* does the origination floor apply:

```
OriginationDecision(A) =
    ORIGINATE        if repository_membership(A)                     // (R) relational continuity, RFC-0012
                     or A is named in a current WorkDesignated,      // (D) user declaration
                        a WorkGrouped pair, or the current
                        ContinuationSurface
    ATTACH_NOWHERE   otherwise                                       // witnessed, indexed, declarable; no Workspace
```

`prior_witness(A)` is **not** part of the floor. It remains Artifact-history evidence (it maintains the Artifact and enables later deterministic re-derivation) but can never, by itself, originate a Workspace.

| Event | Behavior under A |
|---|---|
| First observation, standalone, no declaration | AttachNowhere. Artifact canonical, indexed, searchable. No Workspace, no Home row. |
| Repeated observation, standalone, no declaration | AttachNowhere still. History advances at the Artifact layer only. **No Workspace.** |
| Repository membership | Originate on first content Observation (0.8 relational continuity, unchanged from RFC-0012/D1). |
| WorkGrouped declaration (user) | Declaration + the content Observations of both Artifacts → one Workspace containing both (amend Origination Invariant per §2.H). Deterministic: the declaration is a canonical Observation; replay reproduces it. |
| WorkDesignated (user, "Mark to continue") | Declaration + the Artifact's content Observations → Workspace (amend RFC-0011 §4 Workspace half per §2.G). |
| Browser tab / terminal window | Never originates alone. Joins a body only via declaration or by belonging to a body's declared/repo surface. |
| 100 unrelated files, 1× or 2× each | **0 Workspaces.** Each is witnessed; none is claimed as work. |
| Two tasks in one repository | 1 Workspace (repo); the user's designation/surface discriminates the current task — unchanged, already verified (Case F; daemon designation tests). |

- **Existing RFCs remain valid:** RFC-0003 (Definition, Req 5, Req 8 untouched); RFC-0010 (unchanged); RFC-0013 (unchanged — surface is Layer 3, declaration-driven). Two clauses are **amended**: RFC-0011 §4 (Workspace half of the "SHALL NOT establish" sentence) and RFC-0012 Origination Invariant (user-declared co-membership MAY originate; automatic co-membership never does). IS-0012 Stage 3's floor clause (already amended in D1) is further amended to state this floor.
- **New RFC/IS required:** a small IS amendment is sufficient — the floor is a Formation rule, and IS-0012 is the Formation spec. No new RFC is strictly required; a one-page RFC-0014 "Workspace Origination Contract" would document it at contract level if the community prefers.
- **Storage:** no new primitive, no new persisted object, no schema change. Workspaces persist exactly as today.
- **Replay:** deterministic. The floor is a pure function of canonical Observations. Full replay of an unchanged log under the new floor produces fewer Workspaces — exactly what replay is for (Law VII; `ARCHITECTURE.md:118-122` §7).
- **Determinism:** all inputs canonical; identical inputs → identical origination.
- **Migration:** none needed. No canonical data is rewritten. Old *Snapshots* remain historical facts (Law VIII — snapshots are never rewritten); the *interpretation* (Workspace set) changes under the new floor on next replay, which is the architecture's designed improvement path.
- **UX:** Home shrinks to declared/repo bodies of work. Empty state stays honest ("nothing to continue yet"). "Mark to continue" and "Related work — your call" become the primary origination affordances (both already built).
- **Failure modes:** user never declares → silence (Constitution III, correct-by-design); user declares everything → many Workspaces (Law IX: the user's judgment is authoritative; Evo obeys); repo misdetection → capture-level evidence problem, unchanged.
- **Product tradeoff:** the single-resource non-repo engagement (one spreadsheet, 3 days) requires one declaration to appear. That is the price of never guessing. It is disclosed, not hidden.

## PROPOSAL B — Provisional-engagement state, promoted by evidence

**Behavioral rule:** introduce a distinct register — "witnessed, engagement unconfirmed" — tracked in the derived index (which D1 already implements as unattached Artifacts), optionally presented in a clearly-separated Home section that is explicitly **not** Workspaces and asserts **no** resumability. Promotion to Workspace happens only when canonical evidence establishes engagement.

| Event | Behavior under B |
|---|---|
| First observation | Witnessed-only register. |
| Repeated observation | Stays witnessed-only. **No Workspace.** |
| Repo membership | Promote → Workspace (unchanged). |
| WorkGrouped / designation | Promote → Workspace (same amendment as A). |
| 100 unrelated files ×2 | 0 Workspaces; possibly a "recently seen" list. |
| Two tasks in one repo | 1 Workspace (unchanged). |

- **Existing RFCs:** same two amendments as A. **Plus** a Home presentation decision (what the witnessed register looks like) — a UI/UX change the mandate otherwise avoids ("do not redesign the frontend").
- **New RFC/IS:** same as A, plus an IS-0019/IS-0021 presentation note if the register is surfaced.
- **Storage / replay / determinism / migration:** identical to A (the register is derived, never canonical — Law XVI).
- **UX:** richer than A — the user sees "Evo saw this but doesn't yet claim it's work," which is maximally honest. Risk: the register must be *visually incapable* of being mistaken for a Workspace, or Home simply re-introduces the noise it was meant to remove.
- **Failure modes:** same as A, plus a new failure — the witnessed register becoming a de-facto Workspace list through UI confusion (Law IV violation if presentation implies interpretation).
- **Product tradeoff:** strictly more honest than A, at the cost of a UI surface the mandate says not to redesign. **Under the current mandate ("no frontend redesign"), B collapses to A + a presentation nicety.**

## PROPOSAL C — A new engagement primitive / evidence class

**Behavioral rule:** introduce a new canonical concept (e.g., `Engagement`) or a new evidence class that "witnesses engagement."

| Consideration | Assessment |
|---|---|
| What would witness it? | The user's declaration — which already exists (RFC-0011/12/13). Any *automatic* witness (duration, count, interval, recency, focus) fails the Competing-World Test by the exact arguments of RFC-0011 rows 1–12. |
| New primitive? | **Forbidden.** `ARCHITECTURE.md:47` — "Five primitives. Nothing else is persisted as ground truth." Law XVI — only concepts requiring stable identity may become objects. COGNITIVE_MODEL Principle II — "Openness is a property of an engagement. It is **not a separate object**." |
| New evidence class? | Forbidden by the same test that rejected every automatic candidate; and if the class is *declaration-based*, it is Proposal A with a new name (Law XV: prefer the simpler model). |
| Verdict | **Rejected on corpus grounds** — not on preference. C either duplicates A or violates the five-primitive closure, Law XVI, and COGNITIVE_MODEL II. |

---

# 4. Product tests (the eight users)

| # | User | Does a Workspace exist? | When does it originate? | Home shows | Restorable | User action | Inference? | Deterministic? |
|---|---|---|---|---|---|---|---|---|
| 1 | Student, 100 unrelated PDFs while researching | No (under A/B; under C: n/a) | Never, unless declared | Nothing (or the declared subset) | Nothing (until declared) | Optional declaration | None | Yes |
| 2 | Developer, 50 files across several repos | Yes — one per repo | First content Observation per repo (membership) | The repos | Repo members via surface/designation | None required | Relational continuity only (RFC-0012) | Yes |
| 3 | One spreadsheet, three days, never declared | Under A/B: **No** (honest silence) | Only if declared | Nothing for it | Nothing (record survives; one click brings it back) | One declaration ("Mark to continue") | None | Yes |
| 4 | One document opened once, never returned | No | Never | Nothing | Nothing | None | None | Yes |
| 5 | Two independent tasks in one repo | Yes — 1 Workspace | First repo content Observation | The repo | Task A or B per designation/surface | Designation to discriminate the current task | None (membership only; task discrimination is user-declared) | Yes |
| 6 | Research across tabs + files + terminal | Under A: one Workspace after the user declares | At the WorkGrouped/surface declaration | That body | The declared surface | One declaration | None (user's word) | Yes |
| 7 | User says these belong together | Yes — 1 Workspace | At the WorkGrouped declaration | The body | Declared surface | The declaration itself | None | Yes |
| 8 | Returns to a previous task after days | Yes (repo or declared); silence if neither | Already originated; record persists | The body | Surface + Resume Point | None (or one declaration if never claimed) | None | Yes |

Every row: no recency, no frequency, no time window, no app identity, no similarity, no LLM, no new primitive, no Home omission. The only automatic origination is repository membership; every other body of work is the user's declared judgment.

---

# 5. Recommendation

## 5.1 Choose: PROPOSAL A

**Why it best satisfies Evo's actual product promise:** the promise is continuity of *thought* ("once a thought has begun, it never has to begin again," Constitution Preamble; "the fastest path back into meaningful work," PRODUCT.md). Continuity of thought is a property of the person (COGNITIVE_MODEL), and the only evidence the corpus accepts for it is the person's own word plus witnessed relational structure. A therefore makes Evo *honest*: every Workspace on Home is either a repository (a real, witnessed structure) or something the user explicitly claimed. It eliminates the demonstrated defect (Case D: 100 workspaces from 100 twice-touched files) at the formation layer — exactly where the mandate says the decision belongs — and it preserves every already-working mechanism: repo formation, WorkGrouped, designation, continuation surface, plan==execution, replay, determinism, Home ordering.

**What it sacrifices:** automatic recognition of the *single-resource, non-repository* engagement. The user who works on one spreadsheet for three days without ever declaring sees silence until they say so. This is disclosed as the price of never guessing, and it is the exact tradeoff the corpus's own evidence discipline mandates (RFC-0010, RFC-0011, Law VI, Constitution III). Evo keeps the full record and makes the declaration one click.

## 5.2 Exact authority documents that must change

| Document | Clause | Change |
|---|---|---|
| RFC-0011 | §4 Resolution Rule, sentence "a WorkDesignated observation SHALL NOT by itself establish an Artifact or a Workspace" | Keep the Artifact half; amend the Workspace half: "…SHALL NOT by itself establish an Artifact; it MAY, together with the content Observations that establish the designated Artifact, justify Workspace origination when that Artifact belongs to no Workspace (RFC-0012; ARCH-PROP-0001)." |
| RFC-0012 | Origination Invariant + sufficiency floor | Amend: automatic co-membership (repository membership) MAY justify origination of a Workspace for the member; the user's WorkGrouped declaration, together with the content Observations establishing both declared Artifacts, MAY justify origination of one Workspace containing them; **no other co-membership evidence originates; repeated observation of an Artifact alone never originates.** |
| IS-0012 | Stage 3 (origination floor, as amended in D1) | Replace the `prior_witness` disjunct with: origination is justified iff relational continuity (repository membership) or user declaration (WorkDesignated / WorkGrouped / ContinuationSurface) exists for the Artifact. |
| ARCHITECTURE.md | none | `:150` already mandates the three outcomes; no change. |
| CONSTITUTION / PRODUCT / LAWS / COGNITIVE_MODEL / RFC-0003 / RFC-0010 / RFC-0013 / IS-0011 / IS-0019 / IS-0021 | none | Unchanged. |

No new primitive, no new evidence class, no new persisted object, no schema change.

## 5.3 Minimum required RFC/IS amendment (draft text)

**IS-0012, Stage 3 — Workspace Decision, origination clause (replacing the D1 floor):**

> A content Observation that establishes a previously unseen Artifact MAY originate a new Workspace only when the Artifact carries canonical origination evidence: (a) canonical `RepositoryMembership` evidence (RFC-0012 relational continuity), or (b) the Artifact is named in a current `WorkDesignated` declaration, a `WorkGrouped` declaration pairing it with another witnessed Artifact, or the current `ContinuationSurface` declaration (RFC-0011/0012/0013). Repeated observation of the same Artifact is Artifact-history evidence only and SHALL NOT, by itself, justify origination. All inputs are canonical Observations; the decision is deterministic and replayable. Co-membership evidence never originates a Workspace except through (a) or (b).

## 5.4 Behavioral contract (exact)

1. **Attachment unchanged.** A content Observation attaches to an existing Workspace exactly as today (IS-0012 Stage 2; confidence, ties, same-artifact authority — all untouched).
2. **Origination floor.** `RecognizeNew` fires iff origination evidence (R) or (D) above holds for the incoming Artifact.
3. **AttachNowhere otherwise.** The Observation and Artifact remain canonical; the derived index keeps the Artifact (subject, kind, timeline) for deterministic future re-derivation; no Workspace, no boundary announcement, no Home row.
4. **Declaration-triggered origination.** A `WorkDesignated` / `WorkGrouped` / `ContinuationSurface` Observation naming a witnessed Artifact that belongs to no Workspace causes deterministic origination of a Workspace containing that Artifact (and, for WorkGrouped, its declared partner), derived from the canonical log. This is replayable: the declaration is a canonical Observation.
5. **Never retroactive.** Declarations never merge or destroy existing Workspaces (RFC-0012 Backward Compatibility; Case G2 unchanged).
6. **All downstream contracts fixed.** Continuation Surface, Resume Point, Context Chain (empty), Blockers (zero), plan announcement, execution order, Home ordering, replay, determinism, local-first — none change.

## 5.5 Acceptance tests that prove the contract

All deterministic, added to `crates/evo-workspace/src/scenarios.rs` and the daemon suite, red-first:

1. **`case_d_revised`** — 100 unrelated standalone resources × 2 observations → **0 Workspaces** (was 100). The regression test for this decision.
2. **`case_b_revised`** — one standalone resource × 2 observations → **0 Workspaces** (was 1). Documents the semantic change.
3. **`origination_requires_declaration_or_membership`** — for each of: prior witness only, repo membership only, WorkGrouped declaration, WorkDesignated declaration — assert origination exactly for the last three.
4. **`work_designated_originates_standalone_workspace`** — file witnessed once, then designated → 1 Workspace containing the file, Resume Point = the file, deterministic on replay.
5. **`work_grouped_originates_pair`** — file + URL, each witnessed once, then WorkGrouped → 1 Workspace containing both (declaration is origination authority; no prior witness needed). Distinct from Case G2 (which stays non-retroactive).
6. **`repo_origination_unchanged`** — Case E (100 files, one repo) → 1 Workspace; Scenario 3 of the mandate not regressed.
7. **`two_tasks_in_one_repo_discriminated`** — Case F unchanged: 1 Workspace, designation discriminates.
8. **`replay_equivalence_under_new_floor`** — live pipeline == full replay over identical logs for AttachNowhere, declaration-origination, repo-origination, and prior-witness-never-originates.
9. **`declaration_is_never_retroactive`** — Case G2 unchanged (existing test stays green).
10. **Existing tests revised** (assertions change by design, listed in the implementation order): `case_b`, `case_d`, `scenario_1` (second focus no longer originates — the fixture must add a declaration or repo evidence), `scenario_2`, `scenario_5`, `scenario_6`, `scenario_7`, `scenario_8`, and the daemon runtime tests that drive single-subject double-witness formation (`window_focus` end-to-end, `designation_establishes_complete_plan`, continuation-surface tests) — each gains a declaration or membership step so the tested surface (designation/surface/restoration) still has a Workspace to act on. None of these tests is weakened; their fixtures are brought under the new, stricter floor.

## 5.6 Everything that remains unresolved

1. **Whether the community prefers an IS amendment or a new RFC-0014.** The substance is identical; the container is a process choice.
2. **The RFC-0011 "designation originates" amendment** is the one place this proposal *extends* a declaration's power. It is flagged as a deliberate, minimal change to a boundary clause — not assumed. If the community rejects it, standalone designation falls back to "origination requires membership or WorkGrouped," and user #3 (one spreadsheet) additionally needs a WorkGrouped-style declaration.
3. **Workspace↔Workspace relationships** (ARCHITECTURE.md:151; CONFLICT N) remain deferred — unrelated to this decision, still the named next layer.
4. **Real OS-level restoration of browser/window targets** remains environment-unverified (no Accessibility/automation in this environment); execution failure reporting is verified honest.
5. **Case D's old count (100) is a consequence of the frozen corpus's silence on the floor — not of a clause.** This proposal closes that silence with the corpus's own evidence discipline; if the community reads RFC-0003 Req 5 as *mandating* prior-witness sufficiency, that reading must be stated and this proposal revisited. The proposal's reading is: Req 5 sets a *necessary* floor (historical basis, never instantaneous state); it does not set a *sufficient* one.

---

# 6. One-paragraph summary

Evo should treat something as work worth restoring exactly when canonical evidence establishes it as part of a body of work — and the corpus's own Competing-World Tests (RFC-0010, RFC-0011, RFC-0012) leave only two evidence classes that can do that honestly: **the user's declaration** and **witnessed repository membership**. Repeated observation of the same resource is Artifact history, not engagement, and "opened twice" is indistinguishable from the frequency/order evidence the corpus already rejected. The current D1 floor (`prior_witness`) therefore conflates Artifact history with Workspace legitimacy — that is the root of the 100-workspaces Case D. The smallest principled fix is Proposal A: keep repository membership automatic; make WorkDesignated/WorkGrouped/ContinuationSurface the origination authority for everything else; demote prior witness to Artifact-history evidence; and let every Workspace on Home be either a repository or something the user claimed. It costs the single-resource non-repo engagement (one declaration), preserves every working mechanism, changes exactly two boundary clauses, and needs no new primitive, no new evidence class, no new schema, and no Home filter — because the wrong Workspaces stop existing at the formation layer.
