# UI-IMPLEMENTATION-0003 — Evo Desktop: Frontend Implementation Report

**Status:** Implemented. This is a retrospective report on code that exists, not a proposal.
**Depends on:** `UI-REVIEW-0001.md` (defect register), `UI-IA-0002.md` (approved direction), `UI-VALIDATION-0001.md` (Phase 0 adjudication).
**Frozen and untouched:** Constitution, Product, Architecture, Architectural Laws, all RFC and IS semantics.
**Date:** 2026-08-17

---

## 1. What changed

The desktop shell was rebuilt as a layered crate rather than restyled. Before this work, `app.rs` was a 2,726-line file that held state, dispatch, layout, painting and copy together, and drew every semantic region as the same white rounded card on a beige field. That single fact — that a workspace, a system warning, a historical resource and a button were all the same material — is what made Evo read as a SaaS dashboard rather than a native tool. It is the thing this work set out to undo.

`app.rs` is now 583 lines and holds only state and dispatch. Beneath it sit a visual foundation (`theme.rs`, `ui.rs`) and three screen modules (`home.rs`, `detail.rs`, `shell.rs`). Six semantic surfaces now differ by material rather than by label: the environment (a banded vertical field, the only gradient in the window), the one raised surface that holds the answer, a recessed contextual plane that is *darker* than the environment it sits in, a shallow well for retrieval controls, a lifted action surface whose shadow falls upward, and a modal sheet with the deepest shadow of the five. A test asserts the recessed plane is darker than the environment and the raised one lighter, so the hierarchy cannot silently flatten back into cards.

Home was recomposed from N cards into one raised surface of rows divided by inset hairlines, with the row itself as the affordance. That removed the per-row `Open workspace →` control, the horizontal strip it reserved, and one of the two places text was being clipped. Workspace Detail was recomposed from stacked cards into a spatial two-column split whose contextual column is a recessed plane, never navigation, and which stacks under the main column below 760px rather than crushing. The pinned continue bar was removed entirely, which dissolved two layout defects with it.

Typography became a fixed eight-role ramp in which each role fixes size, weight, tracking and leading *together*. Tracking is size-specific — Display is tightened to −0.6, the all-caps eyebrow opened to +0.9, body left at 0 — and leading moves inversely to size. Spacing became a single 4px scale, mirrored once into egui's `i8` margin units so there is not a second divergent set of numbers; egui's implicit item spacing is set to zero and the layout primitives own the vertical rhythm, because implicit spacing is what makes rhythm drift. Controls size themselves from measured text and wrap rather than shrink when the window is narrow, so clipping is structurally impossible instead of merely unobserved. Motion is a critically-damped settle with a faster leaving response than arriving, and every reveal is driven from the presentation value so it can be interrupted.

Two correctness bugs were fixed along the way. The execution report no longer erases itself: transient state is now keyed to *navigation* rather than to the canonical signature, so a successful restoration producing new Observations can no longer clear the report describing it. And the weighted font families are now registered whether or not the platform face can be read, so a Mac with an unreadable system face loses nativeness rather than losing every heading on every screen.

## 2. Frontend files changed

| File | Lines | Change |
| --- | --- | --- |
| `crates/evo-desktop/src/app.rs` | 583 | Rewritten from 2,726 lines; state and dispatch only |
| `crates/evo-desktop/src/theme.rs` | 966 | Rewritten: materials, type ramp, spacing scale, motion, font binding |
| `crates/evo-desktop/src/ui.rs` | 995 | New: layout, control, surface and status primitives |
| `crates/evo-desktop/src/home.rs` | 472 | New: Home, its retrieval bar, and its empty/blocked states |
| `crates/evo-desktop/src/detail.rs` | 1,135 | New: Workspace Detail, continuation panel, execution, designation |
| `crates/evo-desktop/src/shell.rs` | 467 | New: header, capture notices, first run, Settings, storage error |
| `crates/evo-desktop/src/lib.rs` | 34 | Module registration and a module-layout note |
| `crates/evo-desktop/src/main.rs` | 22 | Window minimum declared as the width the layout degrades to |
| `tools/audit_desktop_calls.py` | 431 | New: the static self-audit described in item 7 |

`crates/evo-desktop/src/state.rs` (2,266 lines) and `crates/evo-desktop/src/daemon.rs` (411 lines) were **not** modified. The projection of canonical understanding and the command boundary are exactly as they were; every screen consumes them unchanged.

## 3. Screens substantially redesigned

All of them, and deliberately so — the brief's requirement that every screen feel like the same product cannot be met by improving some of them.

Home was recomposed as described above, along with its heading, its retrieval bar (search plus the existing kind and period filters, in a well in the environment rather than on a card), its empty-search state with an explicit way back, its reduced-presence stale state, its "nothing yet" state in both watching and not-watching variants, and its permission-required state.

Workspace Detail was recomposed into identity, resume answer, next step, blockers, the Continue action, what Evo witnessed, and the contextual continuation panel — in that priority order, with the panel recessed and secondary. Its "insufficient evidence", "no continuation yet", "designation recorded elsewhere" and "continuation draft" states were all rebuilt.

The execution surfaces were rebuilt: preflight status per target, the report with per-target outcomes, partial restoration presented as the normal case it is, and the nothing-opened case stated plainly. Designation was rebuilt as a declaration in plain language. First run is three moments on the bare environment with no surface lifted, because there is no work to raise yet. Settings became a small modal sheet over a dimmed window holding three facts. The storage-error screen was rebuilt to imitate nothing about work, because the user is not looking at their work.

## 4. Reusable visual and layout primitives introduced

`theme.rs` holds the tokens: a 4px spacing scale (`S1`–`S8`) mirrored as `i8` margins (`M1`–`M6`), semantic gap names (`GAP_SECTION`, `GAP_ENTRY`, `GAP_SUBSECTION`, `GAP_INNER`, `GAP_LINE`, `GAP_TIGHT`), corner radii per surface class, the six materials as frame constructors (`raised_frame`, `recessed_frame`, `well_frame`, `action_frame`, `sheet_frame`) with four matching shadows, the eight-role `Role` enum with `size`/`tracking`/`leading`/`font`, a `Severity` enum pairing a tint with a distinct ink, the motion constants and the `settle` curve, and `detail_split` for the two-column geometry.

`ui.rs` holds the primitives every screen composes from. Layout: `page`, `scroll`, `measure_column`, `constrained`, `gap`, `hairline`, `hairline_inset`, plus `fits` and `measure` so a caller can ask whether something will actually fit before committing to it. Material: `raised`, `recessed`, `quiet`, and `paint_environment` / `paint_scroll_edge`. Type: `label` plus the named voices `display`, `headline`, `title`, `eyebrow`, `lede`, `prose`, `caption`, `provenance`, `field`, `one_line`, and `unknown` for anything Evo has not established. Interaction: `Control` with an `Emphasis` (one primary per row, `debug_assert`ed), `control`, `control_row`, `chip`, `search_field`, `origin_mark`, `set_mark`, `row`, `disclosure`, `section`. Status: `status_word`, `notice`, `notice_with`. Motion: `transition`, a single interruptible reveal value keyed by id.

The rule behind all of it is that no screen file contains a raw pixel number that is not a token, and no screen file paints a surface directly. Layout problems were solved by adding a primitive, not by adding a magic number.

## 5. Backend changes and why they were necessary

One change, in `crates/evo-daemon/src/runtime.rs`. `handle_designation` now reads the standing Continuation Surface before re-deriving, so recording a Work Designation cannot silently drop the surface the user had already declared. This was required, not convenient: RFC-0013 Negative Case 10 establishes that designation and continuation surface do not imply each other, IS-0021 §25's derivation input list requires both to be supplied, and §25.9/§25.10 require determinism and replayability that a dropped input breaks. IS-0019 RP-4 and RM-6 and Law VII are the same constraint from the other side. It is permitted by the brief's §20 because it fixes a frontend-visible correctness bug and the frozen contracts already demanded the behaviour.

One further backend defect was found and deliberately **not** changed. In `crates/evo-daemon/src/daemon_status.rs`, `read_capture_sources` filters against a whitelist that silently drops the `"grouping"` and `"continuation-surface"` lines `runtime.rs` writes. Its only consumer is `crates/evo-daemon/examples/evo_doctor.rs`, not the frontend, so §20 does not authorise touching it. It is reported here as an out-of-scope diagnostic defect rather than fixed.

Nothing else outside `crates/evo-desktop` was modified. No new canonical object, no network dependency, no cloud service, no hardcoded application or profession assumption, and no weakened test.

## 6. Semantic constraints deliberately preserved

The three-way partition is intact and visible: Workspace membership, the declared Continuation Surface, and the derived Restoration Selection are separate regions with separate words, and the contextual panel splits "current continuation" from "related work — not in current continuation" so nothing implies that every member can be restored.

Designation and Continuation Surface remain two distinct declarations. Arity dispatch was rejected in Phase 0 and is not implemented; each has its own control and its own sentence, inside one journey.

No fabricated concept was introduced. There is no resume confidence, no continuation score, no workspace priority, no "active project", and no "current workspace state". IS-0014's `ConfidenceScore` was not repurposed.

Recency is never authority. Ordering on Home is canonical; search is plain substring matching over factual witnessed text; the filters narrow presentation only; and a provenance line under the list states in the product's own voice that nothing is ranked, scored, or promoted for being recent. The Resume Point still comes only from an explicit designation or from a single witnessed resource, and says so.

Navigation is keyed by canonical `WorkspaceId` throughout; titles are presentation and are never read back as identity. Checkbox selections in the continuation panel stay visibly transient until the user declares them, and only the declaration control makes anything canonical. Preflight status is always the status word — `READY`, `UNAVAILABLE`, `AMBIGUOUS`, `UNSUPPORTED`, taken verbatim from `PreflightStatus::label()`, plus `NOT CHECKED` where preflight produced no entry at all — never a colour alone, and the word carries an ink distinct per severity so it survives monochrome. Execution goes through the existing boundary unchanged: `state::run_execution()` → restoration selection → executor preflight → execution → per-target outcomes, with `state::submit_designation` and `state::submit_continuation_surface` as the only other command paths. Presentation state is never written into the evidence log; the first-run marker lives outside canonical storage on purpose. And where Evo does not know something, the UI says so through `ui::unknown` rather than filling the space.

## 7. Test and build results

**No Rust toolchain is available in this environment.** `cargo`, `rustc` and `rustup` are all absent, and installing one is blocked (the proxy returns HTTP 403 on the rustup download, and `apt-get download rustc` cannot locate the package). Therefore `cargo check -p evo-desktop`, `cargo test -p evo-desktop`, `cargo test --workspace` and `cargo build --workspace --examples` **could not be run**. No result for any of them is claimed, and none should be inferred from this report.

What was done instead, in place of a compiler:

Every egui and eframe API used was verified against real evidence rather than memory — either a compiling call site elsewhere in this repository, or the crates' own `.rmeta` string pools in `target/debug/deps`. Names found there are treated as positive evidence only; the pool is incomplete, so absence proves nothing and every uncertain name was cross-checked against an actual call site. `VariationCoords::new([(b"wght", 500.0)])` was confirmed verbatim from epaint's own documentation example in the rmeta.

`tools/audit_desktop_calls.py` performs the three checks a compiler catches first, across all ten desktop modules. Pass 1 resolves every `module::name(` call site against that module's `fn` signatures: **318 cross-module call sites, no unresolved names, no arity mismatches.** Pass 2 resolves every `module::CONSTANT`, `Type::Variant` and `Type { field: … }` reference: **10 modules, 16 types, 62 constants — no unknown constants, variants or struct fields.** Pass 3 resolves every bare name inside a `#[cfg(test)] mod tests` block against what that module defines or imports, since `cargo test` compiles those too: **7 test modules, every reference resolves.** Exit code 0.

The audit was itself validated before being trusted, by injecting faults into a throwaway copy of the crate and confirming each was caught at the correct line: a renamed function, a wrong argument count, a misspelled constant, a non-existent enum variant, a non-existent struct field, a test calling a deleted helper, and a test using a renamed constant. All were reported; exit code 1. Two parser bugs were found and fixed this way — blanked string literals had been destroying argument counts (70 false positives), and `=>` inside a match arm had been read as a closing angle bracket.

A dead-reference sweep across `crates/` confirms no file still references any of the twelve removed `theme::` helpers, the removed continue-bar geometry functions, or the removed animation id. All seven `evo-desktop` examples import only `evo_desktop::state`, which was not modified. No crate in the workspace sets `#![deny(...)]`, so warnings cannot turn into build failures.

The crate carries **75 unit tests** (19 in `theme.rs`, 9 in `ui.rs`, 6 in `detail.rs`, 3 each in `home.rs` and `shell.rs`, 7 in `daemon.rs`, 28 in the unmodified `state.rs`). Every one of the new tests asserts a semantic or perceptual invariant rather than a literal — that the recessed plane is darker than the environment, that tracking is size-specific rather than one value for all sizes, that leading moves inversely to size, that no two type roles are visually identical, that only a running capture reads as Good, that the settle curve never overshoots, that leaving is never slower than arriving, that every severity pairs a tint with a distinct ink, that every weighted font family resolves to real font data with or without the platform face. **None of them has been executed**, because nothing in this environment can execute them. They are written and they are honest about what they check; they are unverified.

## 8. Remaining visual limitations requiring real-machine verification

The renderer limitation is the significant one, and it is documented rather than faked. egui exposes no macOS window blur and no environment sampling, so genuine translucency over the desktop is not available. Rather than paint meaningless gradients and call them blur, spatial hierarchy is built from what the renderer actually has: layered opaque surfaces at different tones, four distinct shadows, scale, inset edges and spacing. The environment itself is painted as a banded vertical field because the renderer has no true gradient primitive. On a real Mac this will read as a quiet layered space rather than as glass, and whether that reads as *intentionally* quiet or merely flat is the single judgement that most needs a screenshot.

The type ramp needs eyes on a Retina display. Tracking and leading were chosen as a set per role, but −0.6 on a 30pt display line and +0.9 on a 10.5pt eyebrow are judgements about optical spacing that cannot be evaluated from source. Whether the weight ramp is even visible depends on egui resolving SF Pro's `wght` variation axis from `/System/Library/Fonts/SFNS.ttf`; if the axis is not applied, all four weighted families collapse to one face and the hierarchy will rest on size alone. The fallback is now safe, but "safe" is not "correct" — this needs looking at.

Control sizing is structurally sound and still unverified visually. Each control allocates exactly its measured text width plus symmetric padding, draws its galley from `rect.center() − galley.size() × 0.5`, and wraps to a full-width stack rather than shrinking when the row does not fit — so `Open workspace →`, `Open System Settings`, `Continue` and `Declare` cannot be clipped by construction. That reasoning rests on egui's text measurement matching its rendering, which only a screenshot confirms.

Narrow-window behaviour was hand-checked, not seen. At the declared 420px floor the Home row was calculated to retain roughly 172px of text measure after page padding, surface padding, row margin and the reserved trailing column; the detail view stacks below 760px. Both numbers want a real resize. The same applies to hover and pressed states, which respond on pointer-down by design, and to the scroll-edge fade where content meets the fixed capture notice.

Finally, three states could not be reached even in principle without a live daemon and real evidence: partial execution with a genuine mix of opened and unavailable targets, the ambiguous preflight outcome, and the reduced-presence stale list during a storage error. Each is implemented and each needs to be provoked on a real machine before anyone should believe it looks right.
