//! The rack: Home as a deck of track cards (DESIGN.md: The rack).
//!
//! Up to three workspaces are presented as cards — index, serif initial,
//! display-voice title, resource summary, honest marks, data-voice age —
//! and everything beyond the third is one stacked deck row that expands in
//! place. Work is never hidden behind a modal; the stack is one click away,
//! visibly, or not at all.
//!
//! The cards bind to the engine's presentation of real work
//! (`evo_daemon::threads::DisplayThread`), which is the same material the
//! pod system enters; a card's identity is its engine thread. Order is the
//! caller's canonical order; this module never re-ranks.

use crate::theme::{self, Role};
use crate::ui::{self, Control};

use evo_daemon::threads::DisplayThread;

/// How many cards sit on the deck at once (DESIGN.md: three).
const DECK_SIZE: usize = 3;

/// What the rack asks the shell to do.
#[derive(Debug, PartialEq, Eq)]
pub enum RackAction {
    /// Open the indexed work as the centre stage.
    Open(usize),
    /// Continue the indexed work directly from the card.
    Continue(usize),
    /// Open an effect-free resource picker, keyed by engine identity.
    RestoreSelected(u64),
    /// Confirm the merge proposal at this index.
    Merge(usize),
    /// Begin renaming the work with this engine identity (RFC-0014): the
    /// shell collects the person's word and sends it through the daemon's
    /// canonical naming channel.
    Rename(u64),
    /// The person's own classification of a work's surface
    /// (OBS-SURFACE-CLASSIFIED): the surface is not work. The engine will not
    /// found or name a work by it; nothing is deleted.
    NotWork(u64),
    /// Remove a work from the default Home surface without deleting its
    /// evidence or identity.
    Dismiss(u64),
}

/// One rack card's derived presentation, computed from a [`DisplayThread`].
pub struct TrackCard {
    /// The card's position on the deck (1-based, like a track list).
    pub index: usize,
    /// The engine thread this card opens.
    pub thread: DisplayThread,
}

/// Renders the rack. Returns the user's request, if any.
///
/// `works` arrives in canonical order (continuable first, deterministic
/// within groups — the engine's own order is already that). The stack's
/// open/closed state is transient presentation owned by `stack_open` at the
/// shell. `query` is the search field's text, owned by the shell: a non-empty
/// query narrows the rack to the works that match, by the person's own name
/// first (RFC-0014) and the work's derived material after.
pub fn rack(
    ui: &mut egui::Ui,
    works: &[DisplayThread],
    merge_proposals: &[(String, String, String, String, usize)],
    stack_open: &mut bool,
    merge_armed: &mut Option<usize>,
    query: &mut String,
) -> Option<RackAction> {
    let mut action = None;
    if works.is_empty() {
        return None;
    }

    // ── The rack is titled "Your work", so it presents work ────────────────
    //
    // A record the engine did not found on identity (`is_work == false`) is
    // *remembered*: it stays in the engine, it is renameable, it can be
    // grouped and searched. But it is not a body of work, and a card that
    // offers Continue and then refuses it — "This record is not currently
    // eligible as work" — is worse than no card. So those records are held out
    // of the deck and the stack and disclosed in one quiet line: demoted, never
    // hidden. Their indices are carried rather than the threads themselves so
    // that a redraw does not clone the list.
    let presentable: Vec<usize> = works
        .iter()
        .enumerate()
        .filter(|(_, work)| work.is_work)
        .map(|(index, _)| index)
        .collect();
    let remembered = works.len() - presentable.len();

    ui::gap(ui, theme::S4);
    ui::eyebrow(ui, "Your work");
    ui::display(ui, "Continue where you left off");
    ui::gap(ui, theme::S3);

    if presentable.is_empty() {
        // Nothing has been founded as work yet. Say so, and say what is being
        // held back, rather than drawing a rack of refusals.
        ui::caption(
            ui,
            &format!(
                "Evo has not founded any work yet. {remembered} {} witnessed and \
                 remembered, but not claimed as a body of work.",
                if remembered == 1 {
                    "record is"
                } else {
                    "records are"
                }
            ),
        );
        return action;
    }

    // Search narrows an already-meaningful list (DESIGN.md: the search field
    // stays quiet — a shallow well in the environment). The person's own
    // names match because they ARE the card titles (RFC-0014 read-back).
    ui::search_field(ui, query, "Find a work by name", "evo-rack-search");
    ui::gap(ui, theme::S4);

    let needle = query.trim().to_lowercase();
    if !needle.is_empty() {
        let matches: Vec<&DisplayThread> = presentable
            .iter()
            .map(|&index| &works[index])
            .filter(|work| work_matches(work, &needle))
            .collect();
        if matches.is_empty() {
            ui::caption(
                ui,
                &format!(
                    "Nothing in Evo's work matches “{}”. The rack keeps only real work; a download or a stray search is not here.",
                    query.trim()
                ),
            );
            return action;
        }
        ui::caption(
            ui,
            &format!(
                "{} of {} works match “{}”",
                matches.len(),
                presentable.len(),
                query.trim()
            ),
        );
        ui::gap(ui, theme::S2);
        for (position, work) in matches.iter().enumerate() {
            if let Some(requested) = stack_row(ui, work, position) {
                action = Some(requested);
            }
            if position + 1 < matches.len() {
                ui::gap(ui, theme::S2);
                ui::hairline_inset(ui, theme::M3 as f32);
                ui::gap(ui, theme::S2);
            }
        }
        return action;
    }

    let on_deck = presentable.len().min(DECK_SIZE);
    let in_stack = presentable.len() - on_deck;

    // The deck: one row of up to three cards when there is room, wrapping
    // to two rows on narrow windows. Nothing is ever cut off — the grid
    // reflows, it does not truncate.
    ui.columns(on_deck.max(1), |columns| {
        for (slot, column) in columns.iter_mut().enumerate() {
            let work = &works[presentable[slot]];
            if let Some(requested) = track_card(column, work, slot) {
                action = Some(requested);
            }
        }
    });

    // The stack: everything past the third card, one click away. An open
    // stack lists the full deck below it; a closed one is a single row. The
    // row is drawn in the stage panel-strip family — instrument stroke,
    // label, count in the data voice, chevron glyph — so overflow reads as
    // the same object species the stage's panels are (G2/G6).
    if in_stack > 0 {
        ui::gap(ui, theme::S4);
        let stack_label = if *stack_open {
            "Hide the stack"
        } else {
            "More in the stack"
        };
        ui.horizontal(|ui| {
            if ui::control(ui, Control::quiet(stack_label)).clicked() {
                *stack_open = !*stack_open;
            }
            ui.add_space(theme::S1);
            ui.label(theme::data(
                in_stack.to_string(),
                Role::Data.size(),
                theme::TERTIARY,
            ));
            ui.label(theme::text(
                Role::Body,
                if *stack_open { "\u{2191}" } else { "\u{2193}" },
                theme::TERTIARY,
            ));
        });
        if *stack_open {
            ui::gap(ui, theme::S3);
            for (position, &index) in presentable.iter().enumerate().skip(on_deck) {
                if let Some(requested) = stack_row(ui, &works[index], position) {
                    action = Some(requested);
                }
                if position + 1 < presentable.len() {
                    ui::gap(ui, theme::S2);
                    ui::hairline_inset(ui, theme::M3 as f32);
                    ui::gap(ui, theme::S2);
                }
            }
        }
    }

    // Merge proposals, below the deck. Two-step confirm — the lesson of
    // the stray click that declared a merge for real: the first press only
    // ARMS the proposal, swapping it to an explicit Confirm / Decline pair;
    // Confirm alone declares, and Decline, a click anywhere else, or a
    // proposal list that shrank all disarm without firing.
    if !merge_proposals.is_empty() {
        ui::gap(ui, theme::GAP_SECTION);
        ui::eyebrow(ui, "Looks related");
        let any_click = ui.input(|i| i.pointer.any_click());
        let pointer_at = ui.input(|i| i.pointer.latest_pos());
        let mut armed_zone: Option<egui::Rect> = None;

        for (index, (subject_a, subject_b, name_a, name_b, sittings)) in
            merge_proposals.iter().enumerate()
        {
            if *merge_armed == Some(index) {
                // Armed: the offer is replaced by its consequence and the
                // only two controls that can act on it. Confirm is the one
                // primary on the screen; Decline is a real alternative.
                ui::gap(ui, theme::S2);
                let armed_scope = ui.scope(|ui| {
                    ui.set_width(ui.available_width());
                    ui::caption(
                        ui,
                        &format!(
                            "Declare \u{201C}{name_a}\u{201D} and \u{201C}{name_b}\u{201D} one work? \
                             They share {sittings} witnessed sessions; the record \
                             joins on the next refresh.",
                        ),
                    );
                    ui::gap(ui, theme::S2);
                    let responses = ui::control_row(
                        ui,
                        vec![Control::primary("Confirm"), Control::secondary("Decline")],
                    );
                    if responses[0].clicked() {
                        *merge_armed = None;
                        action = Some(RackAction::Merge(index));
                    }
                    if responses[1].clicked() {
                        *merge_armed = None;
                    }
                    (responses[0].rect, responses[1].rect)
                });
                let (confirm_rect, decline_rect) = armed_scope.inner;
                armed_zone = Some(confirm_rect.union(decline_rect));
                if cfg!(test) {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("rack-merge-confirm-rect"), confirm_rect)
                    });
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("rack-merge-decline-rect"), decline_rect)
                    });
                }
                let _ = subject_a;
                let _ = subject_b;
            } else {
                ui::gap(ui, theme::S2);
                let label = format!(
                    "Looks related to \u{201C}{name_b}\u{201D} ({sittings} shared sessions) — merge \u{201C}{name_a}\u{201D}?"
                );
                let response = ui::control(ui, Control::secondary(&label));
                if cfg!(test) {
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new(("rack-merge-proposal-rect", index)),
                            response.rect,
                        )
                    });
                }
                if response.clicked() {
                    // Arming only. The merge request itself is unreachable
                    // from this press.
                    *merge_armed = Some(index);
                    armed_zone = Some(response.rect);
                }
            }
        }

        // A click anywhere outside the armed proposal's controls disarms,
        // exactly like dismissing a popover.
        if let (Some(_), Some(at)) = (*merge_armed, pointer_at) {
            let inside = armed_zone.is_some_and(|zone| zone.contains(at));
            if any_click && !inside {
                *merge_armed = None;
            }
        }
        // A proposal list that shrank disarms honestly: the index may no
        // longer mean what the user armed.
        if let Some(index) = *merge_armed
            && index >= merge_proposals.len()
        {
            *merge_armed = None;
        }
    }

    ui::gap(ui, theme::S5);
    ui::provenance(
        ui,
        "Work you can continue, in the order Evo recorded it — those with a \
         marked continuation first. Nothing here is ranked, scored, or \
         promoted for being recent.",
    );
    action
}

/// One track card: the artwork tile, the display-voice title, the summary,
/// the data-voice age. Returns the user's request. `index` is the work's
/// position in the caller's canonical list — the identity the action
/// carries back.
fn track_card(ui: &mut egui::Ui, work: &DisplayThread, index: usize) -> Option<RackAction> {
    let mut action = None;
    // The card's widget id is a global contract, not a layout artifact: it
    // is derived from the work's identity alone, so a screen reader, a
    // harness, or a future remote-driver can address the card without
    // re-deriving egui's internal layout ids.
    let id = card_widget_id(work.thread_id);

    // Hover bookkeeping: the card lifts under the pointer, so the wash and
    // shadow for frame n come from the hover observed on frame n − 1.
    let was_hovered: bool = ui.data(|data| data.get_temp(id).unwrap_or(false));
    let _lift = ui.ctx().animate_bool_with_time(
        id.with("evo-track-lift"),
        was_hovered,
        theme::RESPONSE_TOUCH,
    );

    let frame = egui::Frame::new()
        .fill(theme::RAISED)
        .stroke(egui::Stroke::new(
            1.0,
            if was_hovered {
                theme::border_strong()
            } else {
                theme::border()
            },
        ))
        .corner_radius(egui::CornerRadius::same(theme::CORNER_SURFACE))
        .inner_margin(egui::Margin::same(16))
        .shadow(if was_hovered {
            theme::raised_shadow_hover()
        } else {
            theme::raised_shadow()
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            // The tile: serif initial over a recessed field, with the
            // 1-based position as the track number in the data voice.
            let tile_height = 64.0;
            let (tile, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), tile_height),
                egui::Sense::hover(),
            );
            let painter = ui.painter();
            painter.rect_filled(tile, egui::CornerRadius::same(9), theme::RECESSED);
            let initial: String = work
                .name
                .chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "\u{00B7}".to_string());
            painter.text(
                egui::pos2(tile.right() - 14.0, tile.bottom() - 6.0),
                egui::Align2::RIGHT_BOTTOM,
                &initial,
                theme::display_font(44.0),
                theme::TEXT,
            );
            painter.text(
                egui::pos2(tile.left() + 10.0, tile.top() + 8.0),
                egui::Align2::LEFT_TOP,
                format!("{:02}", index + 1),
                theme::data_font(10.0),
                theme::MUTED,
            );

            ui::gap(ui, theme::S3);
            // The display voice: what this work is.
            ui.label(theme::display(&work.name, Role::Title.size(), theme::TEXT));
            ui::gap(ui, theme::S1);

            // The full evidence stays in the work detail. The deck is a
            // scan surface: a long pending-item list must not turn one card
            // into a wall of text beside two short cards.
            ui.label(theme::text(
                Role::Caption,
                card_summary(&work.understanding.narrative),
                theme::MUTED,
            ));
            ui::gap(ui, theme::S2);

            // The footer: the phase the evidence supports, in the teal
            // voice, and the data-voice age.
            ui.horizontal(|ui| {
                ui.label(theme::text(
                    Role::Eyebrow,
                    work.understanding.phase.label().to_uppercase(),
                    theme::TEAL,
                ));
            });
            // The reason this work holds the position it holds, when there is
            // one the person cannot already read off the card. Only a witnessed
            // open loop qualifies — the age stamp below states recency, and a
            // ranking score is deliberately never shown as a reason, because a
            // number that decides an order must be checkable by the person.
            // Carried in the teal voice: record evidence, not status.
            if let Some(reason) = work.urgency_reason.as_deref() {
                ui.horizontal(|ui| {
                    ui.label(theme::text(Role::Caption, reason, theme::TEAL));
                });
            }
            if let Some(summary) = lifecycle_summary(work) {
                ui.horizontal(|ui| {
                    ui.label(theme::text(Role::Caption, summary, theme::TEAL));
                });
            }
            ui.horizontal(|ui| {
                ui.label(theme::text(Role::Data, age_stamp(work), theme::TERTIARY));
            });
        })
        .response;

    let response = ui.interact(frame.rect, id, egui::Sense::click());
    // The card's accessible identity is the work it opens: an anonymous
    // clickable region reads as nothing to a screen reader or to any
    // interaction harness driving the accessibility tree.
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, work.name.clone())
    });
    let hovered = response.hovered();
    if hovered != was_hovered {
        ui.data_mut(|data| data.insert_temp(id, hovered));
    }

    // Whole card is one hit target (D4): a click opens the stage.
    if response.clicked() {
        action = Some(RackAction::Open(index));
    }
    // Separate from the card hit target: this press must not open/enter work.
    if restore_selected_control(ui, work) {
        action = Some(RackAction::RestoreSelected(work.thread_id));
    }
    // The person's word for their own work (RFC-0014): also outside the
    // card hit target, so opening the rename field never opens the work.
    if rename_control(ui, work) {
        action = Some(RackAction::Rename(work.thread_id));
    }
    // The person's own class for a surface (OBS-SURFACE-CLASSIFIED): also
    // outside the card hit target, so a class never opens the work.
    if not_work_control(ui, work) {
        action = Some(RackAction::NotWork(work.thread_id));
    }
    if dismiss_control(ui, work) {
        action = Some(RackAction::Dismiss(work.thread_id));
    }
    action
}

/// A stack row: the same content as a card, laid out as a list row.
fn stack_row(ui: &mut egui::Ui, work: &DisplayThread, index: usize) -> Option<RackAction> {
    let mut action = None;
    // Same global-id contract as the card.
    let id = stack_row_widget_id(work.thread_id);
    let (response, _) = ui::row(ui, id, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(theme::text(
                Role::Data,
                format!("{:02}", index + 1),
                theme::TERTIARY,
            ));
            ui.add_space(theme::S2);
            ui.label(theme::display(&work.name, Role::Title.size(), theme::TEXT));
        });
        ui.horizontal(|ui| {
            ui.label(theme::text(
                Role::Caption,
                &work.understanding.narrative,
                theme::MUTED,
            ));
        });
        // Same reason line as the card, same voice.
        if let Some(reason) = work.urgency_reason.as_deref() {
            ui.horizontal(|ui| {
                ui.label(theme::text(Role::Caption, reason, theme::TEAL));
            });
        }
        if let Some(summary) = lifecycle_summary(work) {
            ui.horizontal(|ui| {
                ui.label(theme::text(Role::Caption, summary, theme::TEAL));
            });
        }
        ui.horizontal(|ui| {
            ui.label(theme::text(Role::Data, age_stamp(work), theme::TERTIARY));
        });
    });
    if response.clicked() {
        action = Some(RackAction::Open(index));
    }
    if restore_selected_control(ui, work) {
        action = Some(RackAction::RestoreSelected(work.thread_id));
    }
    if rename_control(ui, work) {
        action = Some(RackAction::Rename(work.thread_id));
    }
    if not_work_control(ui, work) {
        action = Some(RackAction::NotWork(work.thread_id));
    }
    if dismiss_control(ui, work) {
        action = Some(RackAction::Dismiss(work.thread_id));
    }
    action
}

fn restore_selected_control(ui: &mut egui::Ui, work: &DisplayThread) -> bool {
    ui::gap(ui, theme::S2);
    let response = ui
        .push_id(("rack-scoped-restore", work.thread_id), |ui| {
            ui::control(
                ui,
                Control::quiet("Restore selected…")
                    .enabled(!crate::scoped_restore::eligible_resources(work).is_empty()),
            )
        })
        .inner;
    if cfg!(test) {
        ui.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("rack-scoped-restore-rect", work.thread_id)),
                response.rect,
            )
        });
    }
    response.clicked()
}

/// The rename affordance: one quiet control per card. The field itself is
/// rendered by the shell (app.rs) against the armed work — this only asks.
fn rename_control(ui: &mut egui::Ui, work: &DisplayThread) -> bool {
    ui::gap(ui, theme::S1);
    ui.push_id(("rack-rename", work.thread_id), |ui| {
        ui::control(ui, Control::quiet("Rename…")).clicked()
    })
    .inner
}

/// The classification affordance: one quiet control per card. The person's own
/// word that a surface is not work. The shell resolves the work's strongest
/// member and sends it through the daemon's canonical classification channel;
/// nothing is sent until the person presses.
fn not_work_control(ui: &mut egui::Ui, work: &DisplayThread) -> bool {
    ui::gap(ui, theme::S1);
    ui.push_id(("rack-not-work", work.thread_id), |ui| {
        ui::control(ui, Control::quiet("Not work…")).clicked()
    })
    .inner
}

/// The reversible presentation correction: this work is not useful on the
/// default Home surface right now. It is distinct from `Not work…`, which
/// classifies one surface and changes only founding/titling semantics.
fn dismiss_control(ui: &mut egui::Ui, work: &DisplayThread) -> bool {
    ui::gap(ui, theme::S1);
    ui.push_id(("rack-dismiss", work.thread_id), |ui| {
        ui::control(ui, Control::quiet("Dismiss from Home")).clicked()
    })
    .inner
}

fn lifecycle_summary(work: &DisplayThread) -> Option<String> {
    if work.skipped_members.is_empty() {
        return None;
    }
    let derived = work
        .skipped_members
        .iter()
        .filter(|member| member.source == evo_threads::MemberStateSource::Derived)
        .count();
    let inferred = work
        .skipped_members
        .iter()
        .filter(|member| member.source == evo_threads::MemberStateSource::Inferred)
        .count();
    let declared = work
        .skipped_members
        .iter()
        .filter(|member| member.source == evo_threads::MemberStateSource::Declared)
        .count();
    let mut parts = Vec::new();
    if declared > 0 {
        parts.push(format!("{declared} declared"));
    }
    if derived > 0 {
        parts.push(format!("{derived} commit-derived"));
    }
    if inferred > 0 {
        parts.push(format!("{inferred} inferred"));
    }
    Some(format!("Skipped members: {}", parts.join(", ")))
}

/// Whether one work answers a search query: a case-insensitive match over
/// the card title (the person's own name when they gave one — RFC-0014),
/// the narrative, and the trail's witnessed names. Substring, no ranking
/// machinery: the rack's list is already canonical, search narrows it.
fn work_matches(work: &DisplayThread, needle: &str) -> bool {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    work.name.to_lowercase().contains(&needle)
        || work
            .understanding
            .narrative
            .to_lowercase()
            .contains(&needle)
        || work
            .understanding
            .trail
            .iter()
            .any(|entry| entry.name.to_lowercase().contains(&needle))
}

fn card_summary(narrative: &str) -> String {
    const LIMIT: usize = 140;
    let mut chars = narrative.chars();
    let prefix: String = chars.by_ref().take(LIMIT).collect();
    if chars.next().is_none() {
        return prefix;
    }
    let boundary = prefix.rfind(char::is_whitespace).unwrap_or(prefix.len());
    format!("{}…", prefix[..boundary].trim_end())
}

// ────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────

/// The widget id a rack card answers to, derived from the work's identity
/// alone (see `track_card`). Public to the crate's tests.
fn card_widget_id(thread_id: u64) -> egui::Id {
    egui::Id::new(("evo-track-card", thread_id))
}

/// The widget id a stack row answers to.
fn stack_row_widget_id(thread_id: u64) -> egui::Id {
    egui::Id::new(("evo-stack-row", thread_id))
}

/// The age stamp in the data voice: "2m ago", "2h ago", "3d ago".
/// Absent evidence renders nothing rather than a guess.
fn age_stamp(work: &DisplayThread) -> String {
    let Some(last_active_ms) = work.last_active_ms else {
        return String::new();
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let secs = now_ms.saturating_sub(last_active_ms) / 1000;
    if secs < 60 {
        return "just now".to_string();
    }
    let minutes = secs / 60;
    if minutes < 60 {
        return format!("{minutes}m ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    format!("{days}d ago")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_summary_keeps_full_detail_elsewhere_and_truncates_utf8_safely() {
        assert_eq!(
            card_summary("Short observed activity."),
            "Short observed activity."
        );
        let long = format!("Observed activity: {}", "नोट्स ".repeat(40));
        let short = card_summary(&long);
        assert!(short.ends_with('…'));
        assert!(short.chars().count() <= 141);
        assert_eq!(long, format!("Observed activity: {}", "नोट्स ".repeat(40)));
    }

    fn work(id: u64, name: &str) -> DisplayThread {
        DisplayThread {
            thread_id: id,
            name: name.to_string(),
            bundle: evo_threads::ResumeBundle {
                resume_point: "app.rs".to_string(),
                resume_reason: "Last seen: app.rs".to_string(),
                restore_set: vec![],
                open_deltas: vec![],
            },
            is_work: true,
            engagement: 1.0,
            has_open_loop: false,
            urgency_reason: None,
            understanding: crate::understanding_fixture(name),
            last_active_ms: Some(1_000),
            member_subjects: vec![],
            skipped_members: vec![],
        }
    }

    #[test]
    fn search_matches_the_persons_own_words_not_derived_vocabulary_only() {
        // The person named this work; searching by their word must find it,
        // even though every derived string says otherwise (RFC-0014).
        let mut thesis = work(1, "main.rs — evo — Visual Studio Code");
        thesis.understanding.title = "main.rs".into();
        thesis.understanding.narrative = "You were working in main.rs.".into();
        thesis.name = "Thesis rewrite".into();

        assert!(work_matches(&thesis, "thesis"));
        assert!(work_matches(&thesis, "THESIS REWRITE"));
        assert!(work_matches(&thesis, "main.rs"));
        assert!(!work_matches(&thesis, "spreadsheet"));
        assert!(work_matches(&thesis, ""));
    }

    #[test]
    fn a_query_narrows_the_rack_and_reports_its_matches() {
        let works = [
            work(1, "Thesis rewrite"),
            work(2, "Building — ZCode"),
            work(3, "Thesis figures"),
        ];
        let matches: Vec<&DisplayThread> =
            works.iter().filter(|w| work_matches(w, "thesis")).collect();
        assert_eq!(matches.len(), 2, "both thesis works answer their own word");
        assert!(matches.iter().all(|w| w.thread_id != 2));
    }

    fn frame_input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            events,
            ..Default::default()
        }
    }

    /// Renders one frame of the rack and returns the action it produced.
    #[allow(clippy::too_many_arguments)]
    fn frame(
        ctx: &egui::Context,
        works: &[DisplayThread],
        proposals: &[(String, String, String, String, usize)],
        stack_open: &mut bool,
        merge_armed: &mut Option<usize>,
        action: &mut Option<RackAction>,
        events: Vec<egui::Event>,
    ) {
        let mut query = String::new();
        let _ = ctx.run_ui(frame_input(events), |ui| {
            if let Some(requested) = rack(ui, works, proposals, stack_open, merge_armed, &mut query)
            {
                *action = Some(requested);
            }
        });
    }

    /// Reads a rect the rack stashed for the harness, if any.
    fn stashed_rect(ctx: &egui::Context, key: egui::Id) -> Option<egui::Rect> {
        ctx.data_mut(|data| data.get_temp::<egui::Rect>(key))
    }

    fn press_release_events(at: egui::Pos2) -> [Vec<egui::Event>; 3] {
        [
            vec![egui::Event::PointerMoved(at)],
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        ]
    }

    #[test]
    fn a_pointer_click_on_a_card_dispatches_open_for_that_work() {
        // The full egui input pipeline, headless: the synthetic move/press/
        // release travels the same path a desktop pointer event takes, so
        // this pins the card's hit-testing and dispatch wiring exactly as
        // the shell runs it.
        let works = [
            work(1, "Building — ZCode"),
            work(2, "Researching — Arc'teryx"),
            work(3, "Writing — Pitch"),
            work(4, "Reviewing — PRs"),
        ];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = false;
        let mut action = None;

        let mut merge_armed = None;
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );

        // Aim at the real widget: read its laid-out rect back from egui.
        let response = ctx
            .read_response(card_widget_id(1))
            .unwrap_or_else(|| panic!("card 1 did not report its widget"));
        let center = response.rect.center();
        let [move_ev, down_ev, up_ev] = press_release_events(center);

        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            move_ev,
        );
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            down_ev,
        );
        assert_eq!(action, None, "no dispatch before release");
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            up_ev,
        );

        assert_eq!(action, Some(RackAction::Open(0)));
    }

    #[test]
    fn a_pointer_click_on_a_stack_row_dispatches_open_for_its_index() {
        let works = [
            work(1, "Building — ZCode"),
            work(2, "Researching — Arc'teryx"),
            work(3, "Writing — Pitch"),
            work(4, "Reviewing — PRs"),
        ];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        // The stack is already expanded (the toggle is a plain Control
        // shared with every other screen; the rows are what carry identity).
        let mut stack_open = true;
        let mut action = None;

        let mut merge_armed = None;
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );
        let response = ctx
            .read_response(stack_row_widget_id(4))
            .unwrap_or_else(|| panic!("stack row 4 did not report its widget"));
        let center = response.rect.center();
        let [move_ev, down_ev, up_ev] = press_release_events(center);

        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            move_ev,
        );
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            down_ev,
        );
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            up_ev,
        );

        assert_eq!(action, Some(RackAction::Open(3)));
    }

    fn proposal(name: &str) -> (String, String, String, String, usize) {
        (
            format!("subject-a-{name}"),
            format!("subject-b-{name}"),
            "This work".to_string(),
            name.to_string(),
            3,
        )
    }

    #[test]
    fn scoped_action_is_separate_from_card_open_and_keyed_by_thread_id() {
        let mut works = [work(42, "Scoped work"), work(7, "Other")];
        works[0].bundle.restore_set = vec!["file:///notes/exact.md".into()];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = false;
        let mut merge_armed = None;
        let mut action = None;
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );
        let rect = stashed_rect(&ctx, egui::Id::new(("rack-scoped-restore-rect", 42_u64))).unwrap();
        let card = ctx.read_response(card_widget_id(42)).unwrap().rect;
        assert!(
            !card.intersects(rect),
            "scope button is outside the Open/enter hit target"
        );
        for events in press_release_events(rect.center()) {
            frame(
                &ctx,
                &works,
                &[],
                &mut stack_open,
                &mut merge_armed,
                &mut action,
                events,
            );
        }
        assert_eq!(action, Some(RackAction::RestoreSelected(42)));
    }

    #[test]
    fn a_record_that_is_not_work_is_held_out_of_the_rack() {
        // The rack is titled "Your work", so it presents work. A record the
        // engine did not found on identity (`is_work == false`) is still
        // remembered — it can be renamed, grouped and searched — but it is not
        // a card, because a card that offers Continue and then answers
        // "This record is not currently eligible as work" is worse than no card
        // at all.
        let mut works = [
            work(1, "Real work"),
            work(2, "Remembered only"),
            work(42, "More work"),
        ];
        works[1].is_work = false;
        // A non-empty restore set, so the record is held out because it is not
        // *work*, not because it had nothing to offer.
        works[1].bundle.restore_set = vec!["file:///notes/remembered.md".into()];
        for work in &mut works {
            work.bundle
                .restore_set
                .push("https://example.test/page".into());
        }
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = false;
        let mut merge_armed = None;
        let mut action = None;
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );
        assert!(
            stashed_rect(&ctx, egui::Id::new(("rack-scoped-restore-rect", 2_u64))).is_none(),
            "a record that is not work must not be presented as a card"
        );
        for id in [1_u64, 42] {
            assert!(
                stashed_rect(&ctx, egui::Id::new(("rack-scoped-restore-rect", id))).is_some(),
                "work {id} must still be presented"
            );
        }
    }

    #[test]
    fn scoped_action_in_stack_and_disabled_empty_cards() {
        let mut works = [
            work(1, "Empty"),
            work(2, "Also empty"),
            work(3, "Other"),
            work(42, "Stack work"),
        ];
        works[3].bundle.restore_set = vec!["https://example.test/exact".into()];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = true;
        let mut merge_armed = None;
        let mut action = None;
        frame(
            &ctx,
            &works,
            &[],
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );
        for id in [1_u64, 2, 42] {
            let rect = stashed_rect(&ctx, egui::Id::new(("rack-scoped-restore-rect", id))).unwrap();
            for events in press_release_events(rect.center()) {
                frame(
                    &ctx,
                    &works,
                    &[],
                    &mut stack_open,
                    &mut merge_armed,
                    &mut action,
                    events,
                );
            }
            if id != 42 {
                assert_eq!(action, None, "an empty card cannot dispatch or open");
            }
        }
        assert_eq!(action, Some(RackAction::RestoreSelected(42)));
    }

    #[test]
    fn one_click_on_a_merge_proposal_arms_but_never_declares() {
        // The stray-click lesson: the offer's first press must be harmless.
        let works = [work(1, "Building — ZCode")];
        let proposals = [proposal("Other work")];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = false;
        let mut merge_armed = None;
        let mut action = None;

        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );
        let rect = stashed_rect(&ctx, egui::Id::new(("rack-merge-proposal-rect", 0)))
            .unwrap_or_else(|| panic!("the proposal did not report its rect"));
        let [move_ev, down_ev, up_ev] = press_release_events(rect.center());

        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            move_ev,
        );
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            down_ev,
        );
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            up_ev,
        );

        assert_eq!(action, None, "the arming press must not dispatch a merge");
        assert_eq!(merge_armed, Some(0), "the press must arm the proposal");
    }

    #[test]
    fn confirm_dispatches_the_merge_and_disarms() {
        let works = [work(1, "Building — ZCode")];
        let proposals = [proposal("Other work")];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = false;
        let mut merge_armed = Some(0);
        let mut action = None;

        // Frame 1 renders the armed pair (no clicks).
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );
        assert_eq!(action, None);
        let confirm = stashed_rect(&ctx, egui::Id::new("rack-merge-confirm-rect"))
            .unwrap_or_else(|| panic!("Confirm did not report its rect"));
        let [move_ev, down_ev, up_ev] = press_release_events(confirm.center());

        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            move_ev,
        );
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            down_ev,
        );
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            up_ev,
        );

        assert_eq!(action, Some(RackAction::Merge(0)), "Confirm alone declares");
        assert_eq!(merge_armed, None, "confirming disarms");
    }

    #[test]
    fn decline_disarms_without_declaring() {
        let works = [work(1, "Building — ZCode")];
        let proposals = [proposal("Other work")];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = false;
        let mut merge_armed = Some(0);
        let mut action = None;

        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );
        let decline = stashed_rect(&ctx, egui::Id::new("rack-merge-decline-rect"))
            .unwrap_or_else(|| panic!("Decline did not report its rect"));
        let [move_ev, down_ev, up_ev] = press_release_events(decline.center());

        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            move_ev,
        );
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            down_ev,
        );
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            up_ev,
        );

        assert_eq!(action, None, "Decline must not declare");
        assert_eq!(merge_armed, None, "Decline disarms");
    }

    #[test]
    fn a_click_outside_the_armed_proposal_disarms_it() {
        let works = [work(1, "Building — ZCode")];
        let proposals = [proposal("Other work")];
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut stack_open = false;
        let mut merge_armed = Some(0);
        let mut action = None;

        // Render the armed pair once so the dismiss-outside rule has a zone.
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            vec![],
        );
        // Click far from the Confirm/Decline pair: on the environment.
        let elsewhere = egui::pos2(450.0, 690.0);
        let [move_ev, down_ev, up_ev] = press_release_events(elsewhere);

        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            move_ev,
        );
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            down_ev,
        );
        frame(
            &ctx,
            &works,
            &proposals,
            &mut stack_open,
            &mut merge_armed,
            &mut action,
            up_ev,
        );

        assert_eq!(action, None, "an outside click must not declare");
        assert_eq!(merge_armed, None, "an outside click dismisses the arm");
    }
}
