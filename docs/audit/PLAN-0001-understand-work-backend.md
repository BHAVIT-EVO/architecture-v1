# PLAN-0001 — What must change for Evo to understand work

**Status:** Proposed. Not authority. Authority remains ARCHITECTURE.md, the RFCs,
and the frozen corpus.
**Method:** every claim about Evo's behaviour below was read in this working tree
(`crates/…:line`) or measured against the live store on this machine. Every claim
about another product comes from `RE done/` and is tagged with its original
evidence marker. Nothing is asserted from memory.
**Inputs:** `RE done/REA_EVIDENCE_REPORT.md`, `RE done/evo-reverse-engineering/EVO-REBUILD-BLUEPRINT.md`,
`RE done/evo-reverse-engineering/evidence/*`, plus `docs/audit/DIAGNOSIS-0001`,
`docs/research/ARBITRARY-WORK-GENERALIZATION.md`, RFC-0011/0013/0014, Architecture
Amendment 1.

---

## 0. Executive summary

The RE material is correct, and its diagnosis of the *pipeline* still describes
this tree: the naming fallback still ends at a resource name
(`evo-engagement/src/engagement.rs:2686`), and the legacy Home ordering is still a
one-bit partition on "has a resume point" (`evo-desktop/src/state.rs:130`).

But three things in it are now wrong or incomplete, and acting on them as written
would rebuild things that already exist and break laws the repo has already
frozen. Stated plainly:

1. **The blueprint diagnoses the wrong live layer for two of the four complaints.**
   Home is no longer served by the Engagement/affinity pipeline. It is served by
   `evo_daemon::threads::display_threads_with_names`
   (`evo-daemon/src/threads.rs:160`), rendered by `evo-desktop/src/rack.rs:17`.
   That path **already ranks** (engagement = episodes × anchors, 14-day half-life
   decay, 30-day fade — `threads.rs:230–295`) and **already filters** some non-work
   (download-only threads, threads with nothing openable — `threads.rs:176–186`).
   The blueprint's §7 describes `order_home_workspaces`, which the rack no longer
   calls. Re-implementing a ranking is not the work; **reasoning** about the
   ranking is.
2. **The blueprint's largest recommendation — a built-in classification layer with
   `Media|Social|Communication|Task` kinds and `Productive|Unproductive` labels —
   directly contradicts Evo's frozen law.** Architecture §5 Workspace Engine:
   *"Must not: Consult application identity, domain, file extension, profession, or
   any allowlist or blocklist of these."* Adopting it as written is a constitutional
   change, not a phase. §3.3 below gives the way through that keeps the benefit.
3. **Two live findings the RE pass could not see, and they change the priority
   order.** Measured on this machine today:
   - `~/Library/Application Support/evo/storage/daemon.status` reports
     `capture=partial`, `window-focus=unavailable`,
     `detail=Accessibility permission is required to witness focused windows`.
     The single most load-bearing signal in the whole engine (`Person/Focused`)
     is currently **not being written**. The engine is being judged while starved.
   - Home is therefore showing a list **frozen at the moment focus was lost**. The
     engine projects every `OBS-WINDOW-FOCUS-GAINED` it reads
     (`threads.rs:886`, `engine_from_observations`), so the 12,576 historical focus
     events still describe the threads — but nothing new can be attributed, and
     the only records arriving now are the 23,014-strong file-churn class.
     Separately, a **second, example-only engine path still exists** that a
     reader would reasonably mistake for the live one: `engine_from_live_data`
     (`threads.rs:665`) reads `~/Library/Application Support/evo/evo.db`, a legacy
     SQLite store whose `canonical_events` table holds **144 rows ending
     2026-09-14**, plus only the log's `Typed` observations (`threads.rs:698`).
     It is called by no production code — only `examples/debug_storage.rs` and
     `examples/sqlite_probe.rs` — but its name says otherwise.

   So: the read path is canonically correct (log only — verified), and the failure
   is on the **write** side plus one dangerous decoy. **Before any redesign,
   restore capture.** Otherwise every subsequent measurement measures a broken
   input, and the fixes below will appear not to work.

---

## 0.5 Hard constraints — non-negotiable, and they override everything below

These are product invariants, not preferences. Where a phase below would violate
one, the phase is void and must be redesigned. Where a reference app in §3.1 does
something a constraint forbids, **we do not take that mechanism** regardless of how
well it works for them.

### C1 — Evo never captures the screen

No screen recording, no screenshot, no window-image capture, no OCR over pixels.
This is the sharpest trust line in the product, and it is also the one most
reference apps cross: Rize bundles `tesseract.js` `[M]` and reads text off
screenshots. **We take Rize's rule model and its deferral, and refuse its OCR.**
Consequence: any capability that would need pixels is out of scope permanently,
not deferred.

### C2 — Evo never listens

No microphone, no ambient audio, no ASR of ambient sound, no diarisation. Granola
dedicates three processes to audio + local speech + speaker separation `[M]`; we
take its *lesson about calendar context* and refuse its microphone entirely.

### C3 — Nothing ever leaves the machine

No network egress of any observation, artifact, derived view, name, embedding, or
log — not for inference, not for telemetry, not for crash reporting, not for
"anonymous" aggregates. Enforced structurally today: **the workspace has zero HTTP
client dependencies**, and the only sockets anywhere are local Unix domain sockets
for daemon↔shell IPC (`evo-daemon/src/{designation,grouping,naming,continuation,
voice}.rs`, `evo-desktop/src/scoped_restore.rs`). This must stay true, and it should
be *tested*, not merely believed (§4 Phase 0.5).
Corollary: the optional local naming model in Phase 4 rung 4 is only admissible
because it is an **in-process, on-disk** model with SHA-verified assets
(`docs/research/ARBITRARY-WORK-GENERALIZATION.md`). If a naming model ever needs a
network call, rung 4 is deleted and rung 3 becomes the ceiling. Never a cloud API.

### C4 — Evo asks for the fewest permissions it can, and never in advance

Current surface, measured from the shipped bundle and the daemon's own report:

| Permission | Why the capture layer needs it | Can it be avoided? |
|---|---|---|
| **Accessibility** | `OBS-WINDOW-FOCUS-GAINED` — the single most load-bearing signal | **No.** Without it the engine has no `Focused` events at all, and every thread is starved. Worth the ask, and worth asking for *only this*. |
| **Microphone** | The optional "Hey Evo" voice feature only | **Yes, and it must not be part of the always-on story.** See below. |
| **Input Monitoring** | `OBS-INPUT-ACTIVITY` keystroke/click counts | **Optional.** The engine treats missing input counts as absence, not as zero, which is honest. Present it as an opt-in quality upgrade, not a requirement. |
| **Screen Recording** | — | **Never requested. Not in the Info.plist, not in code.** |
| **Camera** | — | **Never requested.** |
| **Full Disk Access** | — | **Never requested.** FSEvents is scoped to the home directory (`evo-capture/src/macos_fsevents.rs`), which is the honest minimum. |

Concrete rules that follow:

1. **Never ask in advance.** A permission is requested at the moment the person
   turns on the feature that needs it, never at launch, never as a bundle of
   "grant these five things to get started".
2. **Never make a permission look required when it is optional.** The capture
   report already distinguishes `active` from `unavailable` per source
   (`evo-daemon/src/runtime.rs:503`); the UI must say *optional* where it is
   optional. Today `input-count` reports a System Settings instruction as if the
   feature were broken, when absence is a supported state.
3. **The microphone must be inert unless voice is explicitly enabled.** Today
   `scripts/build-app.sh:228` declares `NSMicrophoneUsageDescription`
   unconditionally in the bundle, and `evo-desktop`'s `voice-wake` feature
   compiles `cpal` + `whisper-rs` in. Declaring the string does not itself prompt,
   but shipping it in a build whose default experience is passive observation is
   exactly the trust problem C2 describes. **Action: the default build must not
   carry the mic at all** (§4 Phase 0.4), and the voice build must state, on the
   permission screen, that the microphone is local-only and never transmitted.
4. **State the containment, on the permission screen.** "Evo reads window titles
   and file events. It does not see your screen, it cannot hear you, and nothing
   it records leaves this Mac." A permission ask that names what is *not* taken is
   the highest-value trust copy in the product, and it is one paragraph.

### C5 — No new capture source without a consent story and an off switch

Every future signal (idle/lock detection included) must ship with: what it reads,
where it is stored, that it never leaves the machine, and a switch that turns it
off **and stops it being written** — not merely stops it being shown. Silent grey
out is not consent.

### C6 — Derived-only, and the source is discarded

If a signal ever needs a raw form to produce a derived form, the raw form is
discarded at capture time and never stored as ground truth (Architecture §10).
Today nothing in Evo needs this, because nothing in Evo reads pixels or audio —
which is the point of C1 and C2.

### 0.5.1 Consequences for the plan as originally written

| Originally proposed | Constraint | Ruling |
|---|---|---|
| OCR signal (Rize-style) | C1 | **Deleted from the plan.** Moved out of §7 (deferred) — it is not deferred, it is refused. |
| Audio / ASR / diarisation (Granola-style) | C2 | **Deleted from the plan.** Same. |
| Calendar (EventKit) as naming evidence | C4/C5 | **Kept, but conditional**: opt-in, asked for only when the person enables it, derived-only, never an input to work membership. It is a read-only, narrowly-scoped permission and it is not screen or audio. |
| Local naming model, rung 4 | C3 | **Kept.** Admissible because it is in-process and on-disk. Rung 3 must remain a complete, shippable naming path on its own so rung 4 is never load-bearing. |
| A "session snapshot" persisted object | C6 / Amendment 1 | Already refused (§5.6); C6 is a second, independent reason. |

---

## 1. The backend as it actually runs

Two interpretation paths exist in the tree. Only one is live.

```
  ┌──────────────┐
  │ 1 CAPTURE    │  macos_fsevents · macos_accessibility · url poller ·
  │              │  input counter · declarations (designation/grouping/naming)
  └──────┬───────┘
         ▼
  ┌──────────────────────────────────────────────────────────┐
  │ 2 CANONICAL LOG  observation.log (append-only, the truth) │
  └──────┬───────────────────────────────────┬───────────────┘
         │                                   │
         │  LEGACY (still compiled)          │  LIVE  ── log only, verified
         ▼                                   ▼
  evo-engagement → evo-workspace        evo-daemon::threads (bridge)
   affinity graph + clusters             engine_from_storage:879
   choose_title → display_name           → engine_from_observations:886
   order_home_workspaces (1 bit)         → project every Observation
   state.rs:130                          → evo-threads Engine
   (rack no longer calls this)           → settle → ThreadView
                                         ▼
                                    DisplayThread { name, is_work, engagement, bundle }
                                    threads.rs:151
                                         ▼
                                    evo-desktop/src/rack.rs → 3 track cards + stack

  ── DECOY, example-only, reads a month-stale store ──────────
  engine_from_live_data:665 → evo.db canonical_events (144 rows, ends 2026-09-14)
                            + log's Typed observations only
  callers: examples/debug_storage.rs, examples/sqlite_probe.rs   (no production caller)
```

Verified facts about the live path:

| Fact | Where |
|---|---|
| Home renders `DisplayThread`s, not `Workspace`s | `evo-desktop/src/rack.rs:17`, `app.rs:186` |
| `is_work = !thread.anchors.is_empty()` | `evo-daemon/src/threads.rs:240` |
| Ordering = `engagement` desc, `thread_id` asc, engagement decayed `0.5^(days/14)`, dropped past 30 days | `threads.rs:230–295` |
| Title = the **first anchor's** name: witnessed `OBS-SURFACE-NAMED` if present, else URL slug / file stem, else the raw resource truncated to 50 chars | `work_understanding.rs:143` → `present_or_structural:129` |
| "Completed" is always empty by design; "pending" comes from engine `Delta`s | `work_understanding.rs:204`, `:210` |
| Phase is a constant (`ObservedActivity`) — never derived | `work_understanding.rs:167` |
| The rack renders **every** `DisplayThread`, including `is_work == false`, but restore refuses those | `app.rs:832`, `scoped_restore.rs:21` |
| Anchors are `Mutation` (production) or `Recurrence` (≥2 sessions, never ubiquitous) | `evo-threads/src/types.rs:95`, `config.rs:100,167,180` |
| No observation schema carries media, idle, lock, calendar, task state, or a "not work" negative | `evo-observation/src/observation_schema.rs:203–215` (the complete registry) |

### 1.1 What is measured right now

| Measurement | Value | Source |
|---|---:|---|
| `observation.log` size / records | 42.9 MB / 53,124 | decode of the live log, today |
| `OBS-FILE-SAVED` | 23,014 (43.3%) | same |
| `OBS-REPOSITORY-MEMBERSHIP` | 10,070 (19.0%) | same |
| `OBS-WINDOW-FOCUS-GAINED` | 12,576 (23.7%) — **historical** | same |
| `OBS-URL-NAVIGATED` | 5,261 | same |
| `OBS-INPUT-ACTIVITY` | 1,583 | same |
| `OBS-SURFACE-NAMED` | 586 | same |
| `OBS-COMMIT-MADE` | 27 (0.05%) | same |
| Explicit user intent (`WORK-GROUPED` + `WORK-DESIGNATED`) | **7 of 53,124** | same |
| Newest records in the log | `OBS-FILE-SAVED`, all `macos_fsevents`, all Photos-Library plists and cloud-sync metadata | tail decode, today |
| `evo.db` `canonical_events` (read by no production code) | 144 rows, ends 2026-09-14 | `sqlite3`, today |
| Live read path | `display_threads_from_storage` → `engine_from_storage` → **observation.log only** | `threads.rs:310, 879, 886`; only caller is `app.rs:648` |

Two conclusions follow directly. **Capture volume is dominated by machine churn**
(62% file-system), and **the tail of the log is now ~100% churn from the photo
library** — the engine is being fed exactly the signal its own churn gate exists
to discard. And **user intent is a rounding error**, so any design that depends on
the user telling Evo things will stay empty until the affordance is cheap and
in-flow (§5.5).

---

## 2. The four complaints, re-anchored

Each complaint, the live mechanism that produces it, and whether the RE pass
diagnosed it correctly.

### 2.1 "It can't group apps by work / can't say which 2–3 of the 6 are still needed"

**Live mechanism.** `evo-threads` forms work from episodes and anchors, which is
the right shape and already solves the blueprint's §5 "partition" complaint: one
browser or one repo can serve many works without merging, ambiguity is retained
rather than guessed, and churn votes nowhere (`evo-threads/src/lib.rs` header).
`min_co_episodes`, hub-and-spoke, and `speaks_for_work` are all things the old
affinity graph needed and the threads engine does not.

**What remains genuinely broken — and it is the part the user cares about most:**

- **Membership is a single statistical set with no lifecycle.** There is no
  `Active | Completed | Parked` per member. So "of the 6 apps in this work, which
  3 are still needed?" has no answer, because *needed* is not a quantity the
  engine models.
- **The nearest thing that exists is a silent, undocumented filter.**
  `Config::restore_recency` (default 30 min, `config.rs:73`) drops a companion
  from the restore set when its last contact is older than that: *"a Google search
  tab consulted and left long before the episode ended is done; the chat the
  person was still typing in is not."* That is the correct intuition, it is
  already implemented, and **it is invisible** — the person cannot see, mark, or
  correct it. Promote it to a named state.
- **The rack renders non-work.** `is_work == false` cards still appear
  (`app.rs:832`), and clicking Continue on one yields *"This record is not
  currently eligible as work."* (`scoped_restore.rs:395`). A surface that shows
  what it then refuses is the exact experience the user describes as confusing.

### 2.2 "It makes anything a work" (the leisure YouTube video)

**Live mechanism, step by step.** The blueprint's §4.2 trace was written against
Engagement and is now out of date, but the outcome survives through a different
door:

1. A page load → `OBS-URL-NAVIGATED` → `Person/Visited` → the engine credits a
   1-second `visited_tick` (`config.rs:47`).
2. Typing in a search box on any page → `OBS-INPUT-ACTIVITY` → `Person/Typed`
   with `weight = keys` (`threads.rs:698`, `config.rs:117` `typed_unit`).
   **Typing is production**, and production is a `Mutation` anchor
   (`types.rs:100`). So a search typed into a video site's own search box anchors
   the video page.
3. Re-watch the same video across two sessions with ≥30 s of meaningful attention
   each → `Recurrence` anchor (`config.rs:100–133`).
4. Either anchor makes `is_work == true` (`threads.rs:240`) → it is presented.

So the engine is *much* better than the old model here — a single unwatched video
downloads to nothing, churn cannot anchor, and the log's own gate list already
contains `"unrelated videos stay separate"` and `"no host is a work identity"`
(`evo-ledger/src/thread_gate.rs:179–196`). **The hole that remains is exactly one:
there is no `intent` axis, so a *typed-into* or *returned-to* video page is
indistinguishable from a document.** The engine cannot say "this was consumption,
not work", because nothing anywhere — not the engine, not the schema — can
represent that claim.

Note the gate named *"no host is a work identity"*: the user's *"it names it as
the YouTube URL"* complaint is already encoded as a CI gate upstream of the
surface. The naming bug is in the *presentation* derivation, not the identity.

### 2.3 "No sense of urgency; work is displayed without order"

**Live mechanism.** The rack *is* ordered (`threads.rs:289`), by
`engagement = episodes × anchors`, decayed by recency. That is **popularity ×
recency**. It is not urgency, and it is not relevance. Concretely:

- A work you returned to 9 times over three weeks outranks the work you were
  interrupted in this morning, if the former has more episodes.
- Nothing about *unfinished* enters the number, even though the engine already
  computes it: `bundle.open_deltas` carries `UnsavedEdits`,
  `UncommittedChanges`, `DownloadedNotYetUsed`, `UnfinishedDraftSurface`
  (`work_understanding.rs:210–228`). The single best urgency signal in the system
  is already computed and currently only rendered as a sentence on the card.
- The blueprint's §7 reading (a one-bit partition that degenerates to canonical
  order) describes `state.rs:130`, which the rack does not call. Do not spend time
  there.

### 2.4 "The name is a URL / meaningless"

**Live mechanism.** `derive_title_named` (`work_understanding.rs:143`) returns the
**first anchor's** presentation name. For `https://www.youtube.com/watch?v=…` the
derivation skips `watch`, finds no candidate segment, and falls back to the domain
→ the title is **`youtube.com`**. `meaningful_name` additionally skips
`app / www / c / chat / projects / design / watch / edit / v / u / p / g / d / e`
and strips UUID-ish suffixes — so the title is never a raw URL, but it is
frequently a **domain, a file stem, or a page title**, and the ladder never
produces a description of an activity. `thread_name` (`threads.rs:439`) exists but
is dead (`let _name = thread_name(thread, &bundle);`, `threads.rs:187`) — the
title actually shown comes from the understanding layer.

**Verdict on the RE pass:** complaint ✅, mechanism ✅, but the target file is
`evo-daemon/src/work_understanding.rs`, not `evo-engagement/src/engagement.rs`.
The engagement `choose_title` is legacy and still reachable only through the
legacy path.

---

## 3. What the RE reports actually give us

### 3.0 Independent check of the evidence

Because this plan is built on the RE folder, I re-derived its load-bearing claims
from the raw artifacts rather than trusting the two synthesis documents. All
confirmed:

| Claim | Re-derived from | Result |
|---|---|---|
| ManicTime analyses `AutoTagTimelineAbsorb` / `AutoTagTimelineAppend` as two modes, with `AutoFillHelper` and a stored `AutoFillInSeconds`, plus `AutoTagRuleComparer`, `IntervalOccupancy`, `AutoTagOccupancy` | `evidence/dotnet/mt_extract.txt` (416 `AutoTag` matches) | ✅ present, exactly as quoted |
| Bunch has **140 ObjC classes** | `evidence/rea-native/bunch_dispatch.json` (`objc_classes` array length = 140; also 262 ivars, 19 protocols) | ✅ exact |
| Toggl has real idle/autotracker IPC channels | `evidence/js/toggl_autotracker.txt` — `idle-detection:dismiss`, `idle-detection:action-taken`, `autotracker:set-enabled`, `autotracker:get-enabled`, `autotracker:set-beta-enabled` | ✅ exact |
| Session Buddy keeps **separate id-maps** for saved vs previous sessions, with a versioned export envelope | `evidence/js/sb_sessions.txt` — `nxs.v3-id-map-saved-sessions`, `nxs.v3-id-map-previous-sessions`, `$.format`, `$.history.*`, and the `sessions/collections/windows/folders/tabs/links` container names | ✅ exact |
| Granola dedicates a process per evidence source, including Calendar (EventKit) | `evidence/asar/Granola/dist-electron/` — `accessibility_process`, `audio_process`, `eventkit_process`, `local_asr_process`, `local_diarization_process`, `mission_control_process`, `sqlite_process`, `tcc_process` | ✅ exact |
| Granola's store carries calendar/attendee facts | `evidence/js/granola_main.txt` — `eventkit:get-authorization-status`, `eventkit:request-access`, `eventKitEvents`, `peopleLastFetchedAt`, `meetingsMetadata` | ✅ exact |
| ManicTime bundles a real grammar parser | `evidence/ManicTime_main_strings.txt` matches `Irony` | ✅ present |
| Dia exposes pinned-tab profiles and archive states | `evidence/Dia_main_strings.txt` — `PinnedTabNavigationProfile` (×4), `ArcSessionRestorationPolicy`, `AutoArchivedTabModel`, `ArchiveUiTimeBucket` | ✅ present |

**One limitation, stated rather than hidden:** the derived directory-listing files
`evidence/ManicTime_macos_dir.txt` and `evidence/Granola_macos_dir.txt` are **empty
(8 and 10 bytes)**. Two of the claims above (Irony, Granola's process tree) therefore
could not be confirmed from the file the citation implies; I confirmed both from
other artifacts as shown. Anyone re-checking should expect those two files to be
blank and should not read that as a refutation.

### 3.1 The transferable mechanisms

| Source | Mechanism | Where it lands in Evo | Verdict |
|---|---|---|---|
| **ManicTime** (`Finkit.ManicTime.Shared.dll`, `AutoTags` namespace — [M]) | `AutoTagType::{Regular, Absorb, Append, Prepend}`; `AutoFillInSeconds` as a **stored per-rule** gap threshold; rules on their own timeline with explicit comparers; `MatchFirst` (order = priority); retroactive re-evaluation; **Automatic Categorization** as a separate axis (Productive/Unproductive/Social/Gaming/**Media & Entertainment**) | The **absorb + gap-fill** pair is the direct fix for §2.2: a 90-second anomalous interval between two sessions that agree inherits them; where they disagree the gap splits. The separate categorization axis is the seed of the missing `intent` axis. | **Take, with one change** (§3.3) |
| **Timing** ([M]) | Rules carry an explicit persisted **order** (`projectRuleOrder`); keywords are parameters *of a rule*; two-level project hierarchy; **productivity is a separate axis from project assignment** | Rule ordering and the "separate axis" idea, same as above. The hierarchy is *not* needed — `evo-threads` already models this better via anchors + companions. | **Take ordering + separate axis. Skip hierarchy.** |
| **Bunch** ([M], 140 ObjC classes / 985 impls) | Per-item `_delay`, `_waitingSnippet` (wait for evidence), `_openAt` (schedule), `_forceOpenClose`, `_closeOn`, `_ignoreInSingleBunchMode` (**this item is optional**), and **`newBunchWithOpenApps` / `copyBunchWithOpenApps`** | `_ignoreInSingleBunchMode` is the *declared* form of "which 2–3 of the 6" — a per-item property, not an inference. `newBunchWithOpenApps` is the honest snapshot primitive. Per-item delay/wait/force are real restore-fidelity gaps today. | **Take all six, expressed as declarations** (§3.4) |
| **Session Buddy** ([M], extracted schema) | `sessions → windows → folders/tabs/links`; versioned `$.format` export with explicit schema paths and a named parse error; **separate id-maps for `saved` vs `previous` sessions** (`nxs.v3-id-map-saved-sessions` / `-previous-sessions`) | The `Saved | Previous` split is the distinction Evo lacks between *what you deliberately kept* and *what was merely last seen*. Window scope preserves layout. | **Take, as a declaration kind** (§3.4) |
| **Workona** ([D]) | A space is a persistent container; **tabs are not resources** — autosaved live session vs explicit "Save as resource"; opening a space restores its most recent session; subset choice is **manual** | The most important single lesson in the whole folder: *do not infer the subset — record the session and let the person pick.* | **Take as design principle** (§5.6) |
| **Dayflow** ([V], MIT source) | `distractions[]` as a **typed field**; `titleEvidence {activities[], selectedActivity, familiarSubject}` where **minutes must sum to the card duration**; "count foreground interaction, not background windows"; "different subjects remain separate even when they share an assistant, browser, or broad project"; a title *prompt* with `ActivityCardPromptOverrides` | The interval-accounting contract is the missing discipline in Evo's understanding layer, and it is checkable (minutes must sum). The "foreground only" rule is already partly implemented as the churn gate. The prompt-override idea is how the title stays tunable without a rebuild. | **Take the contract; local generator only** (§3.5) |
| **Rize** ([D][M]) | **Most-specific-rule-wins**; **2-minute deferral** before spending inference; calendar + task-system evidence; focus = ≥15 min with ≥75% of activity in focus-configured apps/services, *or* a calendar keyword; `trackingRule` carries `category` + `alwaysBlock` | Most-specific-rule-wins and the dual-threshold focus rule are both better-formed than what Evo has. The rule object with a category + a blocking flag is the shape §3.3 needs. | **Take rules + deferral. Calendar/tasks: naming evidence only, opt-in** (§7) |
| **Toggl** ([D][M]) | Time entry = a **first-class named record** with description/project/tags; suggestions from calendar and commonly-used entries; **Dismiss is recorded negative evidence**; autotracker "arm only after N seconds in focus"; enable/disable is broadcast and persisted | Dismiss-as-evidence is the cheapest possible implementation of the missing negative signal. "Arm only after N seconds" is the same idea as Rize's deferral. | **Take. Highest leverage per line in the folder** (§5.5) |
| **Arc / Dia** ([M]) | Per-site **pinned-tab profiles**; **Archive** as a first-class state with a reason field distinguishing auto from manual | Pinned profiles = "the 3 tabs of this service that matter", owned by the user, independent of attention statistics. Archive-with-reason pairs perfectly with Evo's existing non-deletion guarantee. | **Take** |
| **Granola** ([M]) | **Capture breadth, not naming**: dedicated processes for Accessibility, **EventKit calendar**, audio + local ASR + diarisation, Mission Control, TCC permissions, SQLite; store carries `eventKitEvents`, `people`, `peopleLastFetchedAt` | The finding is the breadth. Evo has no calendar and no audio. But see the conflict in §3.3 and the privacy law in §7. | **Partial — see §7** |
| **Rambox** ([M]) | Workspaces as named, server-tracked **partitions**; per-workspace isolated browser profiles with extensions | Same idea as Arc, third instance. Confirms "a work is a declared container of durable app instances". | **Confirms §3.4** |
| **Qbserve / RescueTime** ([D], comparison only) | Qbserve logs YouTube videos and subreddits **separately rather than inferring** whether they are work, and concedes mixed-use cases cannot be resolved by rules; RescueTime offers explicit per-activity productivity plus **ignore-forever** | The honest position on media: **per site/sub-activity, user-adjustable; nobody credible infers it from playback alone.** | **Sets the bar for §2.2** |

### 3.2 What is already implemented and must not be rebuilt

Stated because the blueprint's phase list would otherwise rebuild them:

| Blueprint phase | Already present in this tree |
|---|---|
| "Phase 0 — replay harness + golden set + metrics" | `crates/evo-threads/src/bin/regression.rs` (CI gate: fusion of two labelled works fails the run), `tools/threads-regression/ground_truth_labels.json` (**164 labelled resources**), `tools/threads-regression/export_observation_log.py`, `evo-ledger/src/thread_gate.rs` (7 named behavioural gates over the real log), `evo-daemon/examples/replay_real_history.rs`, `verify_scenarios.rs`, `diagnose_real_history.rs`. |
| "Gate presentation on a minimum distinct sitting count" | `thread_birth_min_production` (`config.rs:107`), `min_anchor_recurrence = 2` (`config.rs:167`), `recurrence_anchor_max_threads = 2` (ubiquity) (`config.rs:130–135`), `resume_exclusion_ubiquity = 3` (`config.rs:147`), the download-only and nothing-openable filters (`threads.rs:176`), and the 30-day fade (`threads.rs:270`). |
| "Make distraction/negative a typed thing" | Half-present and unused for this purpose: `evo_pods::gesture::GestureOutcome::Declare(Declaration::NotMine)` (`evo-pods/src/gesture.rs`) and `evo-threads::Declaration::NotMine` already exist as a *declaration*. The plumbing is there; only the capture and the retroactive application are missing. |
| "Preserve the log-is-truth invariant, never delete evidence" | Already the law and already obeyed. |

### 3.3 The conflict, stated plainly

The blueprint's Module 3 asks for:

```rust
assign: { kind: Media | Social | Communication | Task | Reference | Tool | Deep,
          productivity: Productive | Neutral | Unproductive,
          project: Option<WorkId> }
```

as **built-in rule kinds**, matched against `App | WindowTitle | Url | DocumentPath | Repo`.

Architecture §5, Workspace Engine, *Must not*: **"Consult application identity,
domain, file extension, profession, or any allowlist or blocklist of these."**
RFC-0014 and `docs/research/ARBITRARY-WORK-GENERALIZATION.md` restate it. So the
mechanism is admissible only if the *knowledge* is not Evo's. The way through is
already written into the repo's own law:

> **RFC-0014 R11:** *"A correction from the user is evidence, and it outranks
> inference."*

Therefore:

> **A classification is a user-authored declaration with provenance — never a
> table of apps Evo ships.** Evo may hold `ClassificationRule { matcher, kind,
> productivity }` records exactly as ManicTime and Timing do, because those are
> the *person's* statements about their own machine, replayable as
> `OBS-SURFACE-CLASSIFIED` observations. What Evo must never ship is a default
> rule that says `youtube.com → Media`. The default is silence, and silence
> produces no work rather than a wrong work.

This is not a compromise that loses the benefit: ManicTime's own model is
user-authored rules, the Qbserve/RescueTime comparison says exactly this
(*"per site/sub-activity and user-adjustable"*), and it is the only version that
survives RFC-0014. It also removes the need for the blueprint's
`leisure_founded_work` parameter — there is nothing to forbid, because Evo never
asserts leisure on its own.

---

## 4. The plan

Six phases. Each is independently shippable, each is measurable on the existing
log, and **no phase changes what an observation means** except Phase 3, which
versions its schema.

### Phase 0 — Restore the input (do this first, before anything else)

**Why:** `capture=partial`, `window-focus=unavailable`. Nothing downstream can be
evaluated while `Person/Focused` is absent, and the churn gate that makes
`evo-threads` good depends on it. And a second engine path exists that reads a
month-stale store under a name that says "live" — remove it while the capture work
is already open.

| # | Change | File | Observable result |
|---|---|---|---|
| 0.1 | Surface `capture=partial` in the app itself, not only in `daemon.status`. A work list derived from a starved engine must say so. | `evo-desktop` (rack/dock), `evo-daemon/src/daemon_status.rs` | The person knows to grant Accessibility before judging the list. |
| 0.2 | **Delete the decoy path.** `engine_from_live_data` (`threads.rs:665`) and `read_sqlite_events` (`:743`) read a month-stale SQLite store and are reachable only from examples, but they are named as if they were the live engine. A future reader will wire one in and silently get a different Home. Remove them, or rename them `…_from_legacy_sqlite` and mark them test-only. | `evo-daemon/src/threads.rs:665`, `:743`; `examples/sqlite_probe.rs`, `examples/debug_storage.rs` | Exactly one engine path exists; Architecture §2/§4's "one source of truth" is enforced by the absence of an alternative rather than by convention. |
| 0.3 | Emit a heartbeat observation when the capture drops to partial (`OBS-CAPTURE-PARTIAL/v1`), so the log itself records its own blindness. | `evo-observation/src/observation_schema.rs` (registry is versioned — `:203`) | Gaps in evidence become explainable rather than mysterious. |
| 0.4 | Add the missing `Accessibility` grant to the QA checklist with the exact System Settings path, and a "Check again" that re-probes without restart. | `evo-desktop/src/app.rs`, `docs/verification/` | The most common support case is self-service. |

| 0.4 | **Keep the microphone out of the default build** (constraint C4.3). The default `evo-desktop` build and the app bundle it produces must not carry `cpal`/`whisper-rs`, and `NSMicrophoneUsageDescription` must not be declared in a bundle that does not use it. Voice stays available behind its feature flags, requested only when the person turns it on. | `scripts/build-app.sh:203–235`, `crates/evo-desktop/Cargo.toml:23–31` | The installed app asks for Accessibility and nothing else. |
| 0.5 | **Make C1–C3 enforceable.** Add the guardrail test from §4 (no screen-capture or OCR dependency, no audio dependency in the default build, no HTTP client anywhere, no camera/screen permissions in the generated `Info.plist`). | new test in `crates/evo-desktop/tests/` or `scripts/tests/` | The constraints fail the build if violated, instead of living only in prose. |

**Acceptance:** `daemon.status` reports `window-focus=active`; the log tail
contains a new `OBS-WINDOW-FOCUS-GAINED` within 5 minutes of use; `evo.db` can be
deleted with **no** change to Home (assert it in a test — today it is probably
true, and the test is what keeps it true); a grep for `engine_from_live_data`
returns no production caller; the guardrail test passes.

### Phase 1 — Make the list honest (small, high impact, no new capture)

**Why:** these three changes remove the user-visible symptoms without touching the
architecture, and each is a few lines.

| # | Change | File |
|---|---|---|
| 1.1 | **Never render a title that is a bare host or a raw identifier.** Extend the ladder: witnessed name → activity phrase (§4 Phase 4) → `Untitled — <n> places, <duration>`. Add a unit test asserting `youtube.com`, `www.google.com`, `Login`, `Search` are not returned. *(Shipped without the weekday — see §10.2.)* | `evo-daemon/src/work_understanding.rs:143`, `:306` (`meaningful_name`) |
| 1.2 | **Do not render non-work.** The rack filters on `is_work` (or shows them in a separate, clearly-labelled "Remembered" row), so the surface never offers what `scoped_restore` refuses. | `evo-desktop/src/rack.rs:57`, `app.rs:832` |
| 1.3 | **Delete the dead naming path** (`let _name = thread_name(...)`), or make it the single naming entry point. Two naming functions where one is dead is how the wrong one gets maintained. | `evo-daemon/src/threads.rs:187`, `:439` |
| 1.4 | **Give the engine's own gates a surface.** `run_thread_gates` already knows about the trek playlist, unrelated videos, and the bare-host failure (`thread_gate.rs`). Report its verdict in `evo_doctor` output *and* in the app's diagnostics. | `evo-ledger/src/thread_gate.rs`, `evo-daemon/examples/evo_doctor.rs` |

**Acceptance:** title-identifier rate = 0% (`youtube.com`, bare domain, `Login`,
`Search`, raw path, raw UUID) over all presented works in the real log. No card
renders that cannot be continued.

### Phase 2 — Urgency: a lexicographic order over witnessed facts

**Why:** Evo's own law forbids inventing a preference score, but it does not forbid
*ordering* on facts it has witnessed. The engine already computes the best
urgency signal in the system and discards it into prose.

Replace the single `engagement` key with a **lexicographic tuple**, each element a
fact already in evidence, each rendered as its reason on the card:

```
order by (
  1. has_open_loop        bundle.open_deltas is non-empty          ← already computed
  2. was_interrupted      last episode ended with an open delta
  3. resumed_recently     last_active_ms within the resume window
  4. has_declared_surface OBS-CONTINUATION-SURFACE / designation
  5. engagement           episodes × anchors, decayed   ← today's key, now last
)
```

Then, on the card: `Uncommitted changes in evo-capture · 22 min ago`. Every rank
is explainable in one line, replayable, and contains no number Evo invented.

**This is the change that fixes "no sense of urgency"** — and it requires zero new
capture, because `UnsavedEdits`, `UncommittedChanges`, `DownloadedNotYetUsed`, and
`UnfinishedDraftSurface` are already produced (`work_understanding.rs:210–228`).

Do **not** rank by raw machine-event recency (the blueprint's §7.2 correct
finding): the log's most recent events are photo-library churn, so recency alone
would rank `Photos.sqlite.lock` above the work you stopped in.

**Acceptance:** mean reciprocal rank of the work the user actually resumes next,
measured over a labelled session (extend the 164-resource corpus with the person's
next-click); `evo-threads/tests/ground_truth.rs` gains a case asserting a work with
an open delta outranks a work with more episodes and no open delta.

### Phase 3 — The `intent` axis, as declarations only

**Why:** this is the missing "not work" state — the root cause of §2.2 — done in
the only form that survives RFC-0014.

| # | Change |
|---|---|
| 3.1 | New versioned observation `OBS-SURFACE-CLASSIFIED/v1`: `{ subject, class, productivity?, source }`, where `source = UserDeclared`. Nothing else may write it in v1. |
| 3.2 | `ClassificationRule { id, priority, enabled, matcher: {field, op, value}, assign: {class, productivity?}, apply: Retroactive }` stored as **declarations**; a rule is compiled to observations on creation and re-evaluated on replay. |
| 3.3 | **Absorb + gap-fill over classifications** (ManicTime `[M]`): `absorb_max_gap` stored per rule; a short interval between two agreeing classifications inherits them; disagreement splits. This is what kills a 90-second video page inside a coding session — *without* Evo knowing what YouTube is. |
| 3.4 | Classes gate **founding and titling only**: a class never removes a resource from a thread (the non-deletion guarantee holds), it only prevents it from anchoring or naming. |
| 3.5 | Retroactive re-evaluation is free here: replay the log. No migration. |

**Acceptance:** zero works founded on a user-classified surface; the existing gate
`"no host is a work identity"` (`thread_gate.rs:179–196`) plus two new gates —
`"a classified surface founds no work"` and `"a classified surface names no work"`
— pass on the real log. Note the honest limit: with no rules yet authored, this
phase changes nothing by itself; it changes everything the first time the person
dismisses a video (Phase 5).

### Phase 4 — Naming that describes, locally

**Why:** the name is the product. Everything else is invisible if the card says
`youtube.com`.

Take Dayflow's contract `[V]` and make the **input checkable**:

```
IntervalEvidence {
  activities: [ { activity, minutes, subject, interaction_kind } ]   // MUST sum to the interval
  foreground_only: true                                              // churn never counted
  combine_recurring_visits: true
  separate_subjects_sharing_an_app: true
}
```

Then, a ladder that never degrades to an identifier:

1. **The person's own name** — `OBS-WORK-NAMED` (RFC-0014) — already exists and
   already outranks derivation. `evo-daemon/src/naming.rs`.
2. **A local activity phrase** composed from witnessed facts: interaction class
   (from `Interaction::{Typed, Saved, Downloaded, Mutated, Visited, Focused}` and
   `AnchorKind`) + the most specific *witnessed* object name. Example:
   `Typed` + `evo-capture` → *"Writing evo-capture"*; `Recurrence` on a
   document set → *"Reviewing the RFC drafts"*. This is a small closed grammar over
   evidence Evo already has. It is not an LLM and it decides nothing about identity.
3. **`Untitled — <n> places, <duration>`.** The honest fallback. *(Shipped
   without a weekday: producing a **local** date needs a timezone source this
   crate deliberately does not have, and epoch days are UTC, so a person east of
   Greenwich would see the wrong day. A name that states the wrong day is worse
   than one that states only what was measured — the card's age stamp carries the
   when.)* Its
   arrival is a signal that Phase 5's feedback is needed, and it must be *counted*
   as a defect metric.
4. **Optional, opt-in, local:** `evo-semantic` already has a profile mechanism
   (`multilingual-e5-small`, offline, SHA-verified,
   `docs/research/ARBITRARY-WORK-GENERALIZATION.md`). If a local model is
   provisioned, it may *verbalize* the phrase; it must never decide identity
   (Architecture §8). No network path, ever.

**Acceptance:** title-readability judged blind by two people on a 30-work sample
≥80%; `"Untitled"` rate measured and falling; 0% identifiers. Add both as metrics
in `evo-threads/src/bin/regression.rs`'s report and as a CI floor.

### Phase 5 — The feedback loop (7 events in 53,124 must become routine)

**Why:** highest leverage per line of code in the entire plan. Rize `[D]`, Toggl
`[D][M]`, and Dayflow `[V]` all have this; Evo has essentially none.

| Affordance | Recorded as | Effect |
|---|---|---|
| **Dismiss** a work | `OBS-WORK-DISMISSED/v1` | Off the surface; retained in the log; a repeated dismissal proposes a rule |
| **"Not work"** on a surface | `OBS-SURFACE-CLASSIFIED` (Phase 3) | Retroactive: re-runs classification over all history |
| **Rename** a work | `OBS-WORK-NAMED` (exists) | Title; a recurring rename proposes a rule |
| **Mark member complete / superseded** | `OBS-MEMBER-STATE/v1` | This is the answer to "which 2–3 of the 6" — see Phase 6 |
| **This item is optional** | per-member `optional` flag | Bunch's `_ignoreInSingleBunchMode` `[M]` |
| **Ask when uncertain** | `retrieval_margin` | Already implemented and correct. Keep. |

**Acceptance:** design-event rate ≥ 5 per 1,000 observations (today ≈ 0.13).
This is a *product* metric, not an engineering one — if it stays at 0, the
affordances are not in the flow, and the honest reading is that Evo is guessing
and the person has given up correcting it.

### Phase 6 — Per-member lifecycle, and the subset question

**Why:** this is the user's first and most important complaint, and it is last
because it depends on Phase 5's affordance existing.

Add to the engine's per-member record a state that is **derived where the evidence
exists and declared wherever the person speaks**:

```
MemberState = Active | Completed | Superseded | Optional
```

Sources, in precedence order:

1. **Declared** — Phase 5's `OBS-MEMBER-STATE`. Authoritative, per RFC-0014 R11.
2. **Derived from a commit/merge/close in the captured evidence** — `OBS-COMMIT-MADE`
   exists (27 records); `OBS-COMMIT-MADE` + an anchor's last contact ≤ that commit
   means *that member's job is to have been committed*.
3. **Derived from non-return while siblings return** — the honest inference, and it
   must be *labelled inferred* on the card. This is the promotion of today's silent
   `restore_recency` filter into a state with a name and a reason.

Restoration then runs the **activation plan**: skip `Completed` and `Superseded`;
skip `Optional` in reduced mode; keep `restore_recency` for the rest. The plan is
inspectable, so "which apps do I need to get back into this work?" becomes a
question with a visible answer rather than a statistical recomputation.

**Keep the engine's existing `restore_cap` (5) and the reduced mode (3)**: they are
cognitive limits, which is what they are documented as.

**Acceptance:** membership stability (Jaccard of presented members across two
consecutive restores with no new evidence) = 1.0; restore precision (items kept
open after 5 minutes) ≥ 80% over ~20 restores.

---

## 5. My additions beyond the RE material

Labelled as mine so they can be rejected on their merits.

**5.1 — Correct the layer before changing it.** The blueprint's §6/§7 target
`evo-engagement` and `evo-desktop/src/state.rs`. The live rack targets neither.
Reading the RE folder without checking the tree would have produced a correct
diagnosis of the wrong file, and the "no urgency" fix would have been written into
a function nothing calls. **Verify the live path before accepting a diagnosis of
the pipeline.** (This is the single most valuable thing in this document.)

**5.2 — Lexicographic urgency instead of a weighted score.** The blueprint §11.7
proposes ranking on "declared/observed facts" but shows a table of signals with no
combination rule, which invites a weighted score — and a weighted score is exactly
what Evo's admissibility rule forbids ("*a parameter is admissible only when it
expresses a limit of what Evo can honestly claim to have witnessed, not a
preference*"). A lexicographic tuple over facts needs no invented weight: element 1
is strictly more important than element 2, which is a claim about interruption and
resumption, not a preference about content. It is also what makes the reason
renderable in one line.

**5.3 — Point the interval-accounting contract at the understanding layer, not
just naming.** Dayflow's `titleEvidence.activities[]` with minutes that must sum to
the interval is checkable arithmetic. Evo has no such check anywhere, which is why
a title can be plausible and wrong. Adding "surfaces worn by this work's attention
must sum to the work's attention" makes the *understanding* falsifiable, not just
the title — and it will immediately expose attention that belongs to no thread.

**5.4 — `restore_recency` is the product's existing answer to the subset question,
and nobody can see it.** The config comment (`config.rs:73`) states the correct
principle in prose — *"a Google search tab consulted and left long before the
episode ended is done; the chat the person was still typing in is not"* — and then
implements it as an invisible filter the user can neither see nor override.
Promoting a silent filter to a named, rendered, correctable state is the highest
ratio of product improvement to code in this plan. It also means the answer to
"which 2–3 of the 6" was never missing; it was unnamed.

**5.5 — The negative signal already has plumbing; use it before adding capture.**
`Declaration::NotMine` exists (`evo-pods/src/gesture.rs`,
`evo-threads/src/types.rs`), `OBS-WORK-GROUPED` exists, and a declaration already
outranks inference (R11). So the cheapest "not work" implementation is **one new
observation schema plus a retroactive re-run**, not a new capture subsystem. Reach
for the declaration channel first for every missing signal; it is the only channel
the architecture already blesses, and it is the only one where Evo is right by
construction rather than by guess (blueprint §10.11's own conclusion).

**5.6 — Do not persist a `SessionSnapshot` object; declare a continuation
surface.** The blueprint's §11.6 specifies a persisted snapshot with `kind: Saved |
Previous`, modelled on Session Buddy. Architecture Amendment 1 withdrew the
Workspace/Snapshot exception and restored "nothing derived is persisted".
`OBS-CONTINUATION-SURFACE` and RFC-0013 already exist and already mean *"the user's
explicit restore set, deliberately uncapped, outranking inference."* So the correct
synthesis is: **extend the continuation-surface declaration to carry
`{kind: Saved | Previous, windows: [{folders/tabs}], links}`**, and get Session
Buddy's three load-bearing properties (the saved/previous split, window scope,
versioned envelope) **inside the law instead of beside it**. This is a real gap in
the blueprint, caused by reading shipped binaries rather than the repo's own RFCs.

**5.7 — Split `OBS-INPUT-ACTIVITY` carefully, because the engine's weight depends
on it.** The blueprint's Appendix D notes the record bundles
`Keys`/`Clicks`/`Scrolls` into one record with `fact_count=1`, so per-channel
intensity is not separable. It is separable without breaking anything: the schema
registry is keyed `(name, version)` (`observation_schema.rs:203`), so add
`OBS-INPUT-ACTIVITY/v2` with separate facts and keep v1 readable **purely for
replay of historical data**. Do not rewrite the log. But note the ordering trap:
`typed_unit` (`config.rs:117`) makes typing production, and typing is the signal
most likely to be miscounted, so this split should land **after** Phase 3's intent
axis, or a burst of typing in a video site's search box anchors the video harder.

**5.8 — Extend the existing gate list rather than inventing a new harness.** The
repo already has the right instrument: named behavioural gates over the real log,
with a documented policy that gates failing on both readings fail the buildunless listed in `KNOWN_BOTH_READING_FAILURES` with the phase that closes them
(`thread_gate.rs:229`). Add ours there:

| New gate | Asserts |
|---|---|
| `no work is named by a bare host or identifier` | Phase 1.1 |
| `a work with an open loop outranks a work without one` | Phase 2 |
| `a classified surface founds no work` / `names no work` | Phase 3 |
| `every presented work has a distinct-sitting floor` | §2.2 |
| `no card is rendered that cannot be continued` | Phase 1.2 |

This also means each phase's claim is falsified automatically on the next log
replay, on the user's own machine, with no fixture.

**5.9 — Two metrics the blueprint omits, both computable today.**
*Member-set stability* (restore the same work twice with no new evidence; the
presented set must be identical) — the blueprint lists it, but it is worth stressing
that it is a *correctness* property, not a quality one: an unstable member set is
unexplainable by construction regardless of whether it happens to be right.
*Attention accounted for* (§5.3): total attention across presented works must sum to
witnessed foreground attention, minus what is explicitly unaccounted. Both are
one-line computations over data the engine already produces, and both catch classes
of bug that the 164-resource label set will not.

**5.10 — A warning about the tail of the log.** Today the newest 23,014 records in
the canonical log are `OBS-FILE-SAVED` from the Photos library and cloud-sync
metadata — machine churn, with no human attached. Any change that consults
"most recent event" as a relevance signal will rank that churn first. This is
already true of the legacy `order_home_workspaces` fallback and would be true of a
naive urgency implementation. It is why Phase 2 orders on *open loops*, not on
recency, and why Phase 0's capture fix matters even though it is the least
interesting phase.

---

## 6. Conflict log — where the RE material and Evo's laws disagree

| RE recommendation | Evo's law | Resolution |
|---|---|---|
| Built-in `kind: Media/Social/Communication/Task` classification | Architecture §5: must not consult application identity, domain, file extension, profession, allowlist/blocklist | **User-authored rules as declarations** (§3.3). Evo ships no default rule. |
| Weighted urgency score (§11.7) | `params.rs` admissibility: no preference thresholds | **Lexicographic order over witnessed facts** (§5.2). |
| Persisted `SessionSnapshot` with `Saved \| Previous` | Amendment 1: nothing derived is persisted | **Extend `OBS-CONTINUATION-SURFACE`** (§5.6). |
| LLM naming service (Dayflow-style prompt) | Architecture §8: models verbalize, never decide identity; no network call | **Local closed-grammar phrase first**; local `evo-semantic` model optional and opt-in, never network (§4 Phase 4). |
| Add calendar + audio + ASR capture (Granola) | §10 Privacy: Observation log is the most sensitive data in the system; never transmitted; no raw high-fidelity capture retained | **Opt-in, derived-only, per-source consent, and never an inference input for membership** — calendar may be *naming evidence for a meeting* only. Deferred past Phase 0–5. |
| `leisure_founded_work` parameter forbidding leisure from founding work | Same as row 1 | Unnecessary: Evo never asserts leisure, so there is nothing to forbid. |
| `inference_deferral` (2 min before spending model inference) | No law conflict | **Keep as a candidate**, but it only matters once there is model inference to defer; with Phase 4's grammar there is nothing expensive to defer. Revisit with Phase 4.4. |

---

## 7. Deferred, and why

| Item | Why deferred |
|---|---|
| Calendar (EventKit) capture | Real value, real privacy cost, and it is naming evidence rather than membership evidence. Needs an explicit consent flow and a derived-only contract. After Phase 5. |
| Audio / ASR / diarisation | Same, and higher cost. The blueprint's own evidence shows Granola's naming is server-side and unverified `[?]`, so the mechanism we would be copying is the one least evidenced in the folder. |
| Idle / screen-lock capture | Genuinely missing and genuinely useful (`max_interval_attention` currently guesses from silence), but it is a *boundary* refinement, not a correctness fix, and macOS gives it cheaply. Slot it into Phase 0 as `OBS-SESSION-STATE/v1` if the Accessibility work is already open. |
| OCR / screen reading (Rize bundles `tesseract.js` `[M]`) | **Refused, not deferred — constraint C1.** Evo never reads pixels, in any derived form. Do not reintroduce this as a "research project". |
| Audio / ASR / diarisation (Granola's three processes `[M]`) | **Refused, not deferred — constraint C2.** Evo never listens. |
| ManicTime `ApplyTo` / `AutoTagType` enum member names | Unresolved in the RE pass `[?]` (field counts known: 4 and 5). Does not block anything: §3.3 uses user-authored rules, and the modes we need are `Regular` + `Absorb` + `Append`, all attested `[M]`. |
| Workona's internal restore heuristic | Minified; unrecovered `[?]`. Operative finding (subset choice is manual) is documented `[D]` and is what §4 Phase 6 implements. |

---

## 8. What would make this plan wrong

Stated up front so it can be checked rather than discovered.

1. **If Accessibility is granted and the list is already good**, then the user's
   current observations are input artifacts, and Phases 1–2 are cosmetic. Phase 0
   decides this, and it is a 10-minute check. **Do Phase 0 before accepting
   anything else in this document.**
2. **If the shipped binary is older than this tree.** The RE blueprint's Appendix D
   item 1 raises this and it is unresolved `[?]`: the installed app was v0.1.0
   (built Oct 14) while the tree is newer. If the running binary predates the
   threads bridge, the person is looking at Engagement output, and the blueprint's
   §5–§7 diagnosis is right for what they see. Confirm by checking which store the
   running agent reads (Phase 0.2 makes this explicit).
3. **If the running agent is not the tree's code.** I verified the read path in
   source, not in the shipped binary, and the daemon currently running was built
   and installed on 2026-10-15/16 from `~/Library/Application Support/Evo/agent/`.
   If that binary predates the threads bridge, the person is looking at
   Engagement output and the RE blueprint's §5–§7 diagnosis is right for what they
   see. **Confirm with the binary, not the source** (strings for
   `display_threads_from_storage`), before choosing which pipeline to fix.
4. **If the person will not author rules.** Phase 3 delivers nothing until someone
   dismisses or classifies something. If the intent is that Evo should work with no
   user input at all, then §3.3 is the wrong resolution and the honest consequence
   is a constitutional amendment to Architecture §5 — which should be a deliberate,
   documented decision, not a phase in a plan.

---

## 9. Order of work, in one line each

| Phase | One-line outcome | Depends on |
|---|---|---|
| **0** | Evo is fed complete evidence again, from one source of truth | — |
| **1** | No card shows an identifier, and no card is shown that cannot be continued | 0 |
| **2** | The list leads with what was left unfinished, and says why | 0 |
| **3** | "Not work" exists and re-runs over all history | 5 (for the affordance) |
| **4** | Titles describe an activity; `Untitled` is honest and counted | 1 |
| **5** | Correcting Evo is cheap, in-flow, and compounded | 1 |
| **6** | "Which 3 of the 6" has a visible, correctable answer | 5 |

Phases 1 and 2 can ship in the same session. Phase 0 must ship first.

---

## 10. Implementation status

Updated 2026-10-16. Everything below was built in this working tree; verification
results and remaining live-corpus failures are stated explicitly rather than
left for a reader to discover.

### 10.1 Shipped

| Phase | What was delivered | Verification |
|---|---|---|
| **0.1** | *(already existed — nothing to build)* Home already reads the daemon's capture status, renders a partial-capture notice in the header, shows a permission screen when no evidence exists, and offers "Check again" (`evo-desktop/src/{daemon,shell,app}.rs`). Confirmed rather than duplicated. | read + `daemon.status` on the live machine |
| **0.2** | Removed the decoy engine path: `engine_from_live_data`, `engine_from_sqlite`, `read_sqlite_events`, `extract_json_u64`, `extract_json_string`, `normalize_download_resource`, and the two examples that called them (`sqlite_probe.rs`, `debug_storage.rs`). It read a month-stale SQLite store, was reachable only from examples, and was named as if it were the live engine. | deleted; `cargo check --workspace --all-targets` clean |
| **0.4** | The microphone is now opt-in in the shipped bundle. `scripts/build-app.sh` defaults `EVO_VOICE=off`: the base `Info.plist` declares **no** microphone key, the voice models are not copied, and the voice agent binaries are not built or bundled. `EVO_VOICE=on` restores all of it and adds the mic key **with a local-only statement in the usage string**. | `bash -n` clean; guardrail test |
| **0.5** | Constraints C1–C3 are now executable: `crates/evo-desktop/tests/trust_constraints.rs`, 7 tests, all passing. | `cargo test -p evo-desktop --test trust_constraints --offline --locked` → 7 passed |
| **1.1 + 4** | Titles are never addresses. `meaningful_name` refuses bare hosts, generic URL interface slugs, and opaque `app:`/`term:`/`window:` identifiers. The final title ladder is witnessed name → structural name → closed-grammar activity phrase (`Writing/Updating/Reviewing/Working across N places`) → honest `Untitled — N places, <duration>`. Canonical content-free activity evidence is joined only at presentation time; it never affects identity. Added final-title metrics and a 30-case fixture. | daemon title tests and fixture pass; `title-regression` reports final Home titles |
| **1.2** | The rack presents only work. Non-work records are held out of the deck **and** the stack and disclosed in one quiet line ("Evo has not founded any work yet. N records are witnessed and remembered…"), so a card can no longer offer Continue and then refuse it. Held out, never hidden: `is_work` records are still in the engine, still renameable, still searchable. | 1 new + 1 updated rack test; 12/12 pass |
| **1.3** | Removed the dead naming path: the `let _name = thread_name(...)` binding and the duplicate `thread_name` / `meaningful_name` / `is_identifier` / `title_case` cluster in `threads.rs` (all four were reachable only from the dead binding). The canonical naming lives in `work_understanding.rs`. | same as 0.2 |
| **2** | Ordering is now a lexicographic key over witnessed facts: `has_open_loop` → coarse recency bucket → decayed engagement → thread id. `DisplayThread` gained `has_open_loop` and `urgency_reason`; the reason renders in the teal voice on both the card and the stack row. Recreation reads the work's own last human activity, never the newest record in the log. | see 10.3 |
| **3** | The `intent` axis, as declarations only. New canonical observation `OBS-SURFACE-CLASSIFIED/v1` (`{ subject, class }`, source is the writing user; nothing else may write it), registered canonical, reference-only, and a user declaration. The engine gained `Declaration::Classified`: a classified subject never anchors a work — `commit_episode`, recurrence promotion, recurrence birth, and `declare_anchor` all refuse it — and any anchor it already carried is demoted back to context the moment the class arrives, so the class is **retroactive over the whole log by replay**, with no migration. The daemon projects the record onto the engine and joins it into presentation through `classification::ClassifiedSurfaces`, so a classified surface never names a work even as a companion or a resume point. Nothing is ever deleted: the subject stays a participant and a companion — a class is not a quarantine. Evo ships no default class. The daemon also carries the **authoring channel** this record needs: a Unix-socket declaration channel (`evo-daemon/src/classification.rs`, mirroring the naming/grouping/continuation channels) that validates and forwards `MacOSSignal::SurfaceClassified` through the runtime (source `user_classification`), a `state::submit_classification` desktop wrapper, and a per-card **"Not work…"** control. A classification is now reachable end to end, not merely consumable. | see 10.3 |
| **3.2–3.3** | User-authored `ClassificationRule` evaluation is now deterministic and offline. Rules match only explicitly selected witnessed fields (`App`, `WindowTitle`, `Url`, `DocumentPath`, `Repo`) with literal `Equals`/`Contains`/`Prefix`/`Suffix`/`Wildcard` operators; priority then rule id breaks ties; silence and disagreement remain silence. Per-rule absorb/gap-fill is bounded to the interval between two agreeing direct classifications and never creates a new class. Rule output projects into the same derived `ClassifiedSurfaces` index without persisting generated classifications. No application/domain/file-type knowledge is embedded. | `cargo test -p evo-daemon --lib --offline --locked` (rule tests pass); graphify update |
| **5** | Added `OBS-WORK-DISMISSED/v1` and `OBS-MEMBER-STATE/v1`, including canonical evidence, validation, macOS translation, replay projection, a daemon-owned local feedback socket, desktop submission wrappers, and a rack **"Dismiss from Home"** affordance. Dismissal removes only the default Home presentation; replay evidence and retrieval remain intact. Repeated dismissals now produce a deterministic, user-reviewable exact structural `ClassificationRule` proposal; no rule is applied automatically. Member state is explicit and inspectable through `ActivationPlan`. | daemon, capture, observation, classification-rule, lifecycle, and desktop tests |
| **6** | Added `MemberState = Active | Completed | Superseded | Optional`, with declared > commit-derived > non-return-inferred precedence. Commit-derived completion uses only opaque repository-membership relationships; inference promotes stale non-returning members to `Completed` only after a sibling returns. Normal mode keeps the five-item cap; reduced mode uses three and skips `Optional`. `ActivationPlan` reports the source, and Home cards expose skipped-member source counts in the teal evidence voice. Derived state is ephemeral and recomputed on replay; it is never persisted as a declaration. | 6 lifecycle tests, daemon opaque-relationship projection test, full evo-threads suite, desktop suite |

### 10.2 Deliberate deviations from the plan as written

1. **No weekday in the fallback title.** The plan asked for
   `Untitled — <weekday> <n> …`. Producing a *local* date needs a timezone
   source, and `evo-daemon` has no date or timezone dependency by design (verified
   from its manifest). Epoch days are UTC, so a person five hours east of
   Greenwich would read the wrong day in the evening. The fallback therefore states
   places and duration only, and the card's own age stamp carries the when. Failing
   honest beats failing plausible.
2. **"places", not "surfaces".** A resource in this engine is a window, a URL, a
   file or an app surface; "surfaces" is the codebase's word, not a person's.
3. **The network guardrail permits loopback, it does not ban sockets.** The first
   draft of the test banned `std::net::` outright and immediately failed — on
   `evo-execution/src/browser_macos.rs`, which implements a **local Chrome DevTools
   Protocol client** over TCP to read tab titles and URLs from the local browser.
   That is the mechanism by which Evo sees a page *without* seeing the screen, so it
   is required, not forbidden. The rule encoded instead is the containment:
   `LocalDevToolsConfig` refuses any non-loopback address (`address.is_loopback()`, at
   construction and per use), and the test now requires every file that opens a
   socket to pin itself to loopback. **The guardrail caught a real property of the
   system and forced a more precise statement of the constraint — which is the
   point of having it.**
4. **`EVO_VOICE=off` is now the default bundle**, which changes what a plain
   `bash scripts/build-app.sh` produces: no `evo-agent`, no voice models, no mic
   key. This is the constraint applied literally. The voice feature is not deleted —
   `EVO_VOICE=on` builds exactly what was built before — but it is no longer what an
   ordinary build ships.
5. **`OBS-SURFACE-CLASSIFIED/v1` carries `{ subject, class }`, not the
   `productivity?` the plan sketched.** Every fact in a canonical record is a
   promise that means something; nothing in the tree consumes a productivity
   label, and recording one now would be writing down more than the system can
   act on. If a phase ever needs it (a reduced-restore rule that skips
   `Unproductive` members, say), it arrives as `v2` — the registry is keyed
   `(name, version)`, so v1 stays readable and nothing migrates.
6. **Classification rules and absorb/gap-fill are now built.** The rule engine
   remains a derived projection: it never writes generated classifications into
   the append-only log, and it never consults application/domain/file-type
   knowledge. Only the person's explicit structural field and literal rule
   values can match; disagreement produces silence. The remaining open question
   is product authoring UX for creating/editing rules, not the deterministic
   evaluator.

### 10.3 What is verified, and what is not

**Verified by running the commands:**

| Check | Result |
|---|---|
| `cargo check --workspace --all-targets --offline --locked` | clean; the only two warnings are pre-existing and in files this work did not touch (`name_index.rs:285`, `continuation.rs:347`) |
| `cargo test -p evo-daemon --lib --offline --locked` | **215 passed**, 0 failed, 1 ignored |
| `cargo check -p evo-desktop --all-targets --offline --locked` | clean; the new feedback wrapper and rack action compile |
| `cargo test -p evo-capture --lib --offline --locked` | **64 passed**, 0 failed (including feedback-schema translation) |
| `cargo test -p evo-desktop --lib --offline --locked` | **249 passed**, 0 failed |
| `cargo test -p evo-desktop --test trust_constraints --offline --locked` | **7 passed**, 0 failed |
| `cargo test -p evo-observation --offline --locked` | **139 passed**, 0 failed, 1 ignored (+ 5 doctests) |
| `cargo test -p evo-threads --all-targets --offline --locked` | all suites passed; lifecycle suite **6 passed** |
| `cargo check -p evo-threads --all-targets --offline --locked` | clean |
| `cargo test -p evo-ledger --lib --offline --locked` | **68 passed**, 0 failed |
| `cargo test -p evo-pods` | all suites pass (23, 15) |
| `cargo run -p evo-daemon --bin title-regression --offline --locked` | **53,506** canonical observations, **26** presented works, **0** Untitled, **0** identifier titles; identifier floor PASS |
| `cargo run -p evo-threads --bin regression --offline --locked -- --events tools/threads-regression/sample_events.json --labels tools/threads-regression/ground_truth_labels.json` | placement fixture PASS; 2 threads, 0 fused distinct works, leisure never anchors |
| `cargo run -p evo-ledger --release --bin regression-gate --offline --locked` | completed live replay: **53,512** observations, 7 design events, **0.13/1,000** (below the 5 floor); behavioral failures remain in the current corpus: 3 Evo-development works and a 668/783-resource mega-work |
| `graphify update .` | graph refreshed: **53,614 nodes**, **187,910 edges**, **806 communities** |
| `bash -n scripts/build-app.sh` | syntax OK |

**Not verified, stated plainly:**

- **No end-to-end measurement of the new ordering on real history.** The real log
  on this machine cannot demonstrate it while `capture=partial` and
  `window-focus=unavailable`: the engine has no new `Focused` events, so the list is
  frozen at the moment the permission was lost. Phase 2's acceptance metric (mean
  reciprocal rank of the work the person actually resumes next) is specified and
  **unmeasured**. Grant Accessibility and re-measure before judging the ordering.
 - **Some numeric gates remain corpus-dependent.** The code now exposes final-title
   metrics through `title-regression` and canonical design-event metrics through
   `regression-gate`; the checked-in 30-title fixture has 0% identifier rate and
   ≥80% readability. Live-history title/member stability and restore precision still
   require Accessibility-backed real observations.
 - **The live ledger gate is not green.** The optimized release replay completed,
   but the current machine-local corpus still fails two pre-existing behavioral
   checks: Evo development spans 3 works (limit 2), and one work owns 668 of 783
   ledger resources (mega-work threshold 60%). The design-event floor is also
   below target at 0.13/1,000; this is a product-signal result, not a fabricated
   pass. These failures are visible in the command output and were not masked.
- **`scripts/install-agents.sh` was not audited** for whether it installs the voice
  agent's LaunchAgent unconditionally. If it does, an `EVO_VOICE=off` bundle could
  still register a voice agent that has no binary behind it. Worth a check before
  the next release build. *(Since checked: it installs only `dev.evo.capture` and
  `dev.evo.ledger` — no voice agent label exists — so an `EVO_VOICE=off` bundle
  cannot register a voice agent. Closed.)*
- **Phase 3's gates pass vacuously on this machine's log.** No `OBS-SURFACE-CLASSIFIED`
  record has been authored yet, so both new gates short-circuit to `Pass` and prove
  nothing about the live corpus. What *is* proven is on fixtures: `evo-threads/tests/classification.rs`
  (4 tests) and the new `thread_gate` unit tests (2) drive the mechanism directly,
  and `evo-daemon`'s `a_classified_surface_founds_no_work_and_names_none` runs the
  whole read path — persist a classification, replay, assert neither founding nor
  titling. The real-log measurement arrives the first time the person dismisses
  something (Phase 5).
 - **The "Not work…" control is UI, verified by compilation and the wrapper test,
  not by driving the rendered app.** `evo-desktop`'s
  `submit_classification_forwards_to_the_daemon_and_reports_honestly` proves the
  shell→daemon half (a real listener, a real socket, a forwarded
  `SurfaceClassified`, and an honest rejection of a blank class); the click path
   from the card is compile-verified only, because this environment has no GUI to
   drive. Worth one manual pass before shipping.
- **The new "Dismiss from Home" control is likewise compile-verified only.** Its
  daemon socket, canonical-subject validation, replay projection, non-deleting
  dismissal semantics, and desktop wrapper are covered by tests; the rendered
  click path still needs one GUI pass.
 - **Phase 6 derived and inferred lifecycle sources are implemented conservatively.**
   They are recomputed from opaque replay evidence and never written as
   `OBS-MEMBER-STATE`. The live corpus still needs measurement to establish the
   plan's Jaccard and restore-precision targets.
 - **No RFC was written for `OBS-SURFACE-CLASSIFIED/v1`.** RFC-0011–0014 each
  introduced a schema, but the newer `OBS-SURFACE-NAMED/v1` and
  `OBS-INPUT-ACTIVITY/v1` shipped without one, so the tree's recent practice is
  mixed. The schema is fully specified in code and its tests; if the project wants
  the declaration-schema paper trail to stay complete, a short RFC-0015 is the
  right next document, and nothing in the implementation would change.

### 10.4 New finding, not in the plan

*(If an RFC-0015 is written for the classification schema above, it belongs in
`docs/rfcs/` beside RFC-0011–0014; it is a documentation gap, not a gap in the
delivered behaviour.)*

**Optional embeddings are now feature-gated.** `fastembed` is behind the explicit
`embeddings` feature in `evo-voice`, propagated only when requested by the daemon or
desktop. The default build does not link it; without the feature, semantic voice
triggers fail honestly as unavailable while exact local selection remains usable.
The feature is local/offline-only and the existing trust guardrails cover its default
off state. This removes the previous default dependency-graph concern without
pretending that an optional model is available on a machine that has not provisioned
it.
