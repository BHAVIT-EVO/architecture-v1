//! Home: your work, ready to continue.
//!
//! The product promise rendered: when you come back, you see your work —
//! what it is, where you left off, and one action to return. No engine
//! jargon, no domain names as titles, no "threads." Just: this is what
//! you were doing, and here's how to get back.
//!
//! Design principles (from the category research):
//! * The resumption card is the product — goal, context, action
//! * Names are auto-generated and contextual, never domain names
//! * One primary CTA ("Continue") — everything else is secondary
//! * Quiet by default — no interruptions, just a clean list
//! * Click to see what will open — full transparency before action

use crate::state;
use crate::theme::{self, Role, Severity};
use crate::ui::{self, Control};

use evo_daemon::threads::DisplayThread;

use std::collections::HashMap;
use std::time::SystemTime;

/// Memoized retrieval material for the remembered-work list.
#[derive(Default)]
pub struct Memo {
    signature: Vec<(String, usize)>,
    haystacks: Vec<String>,
    kind_flags: Vec<[bool; 4]>,
}

impl Memo {
    fn ensure(
        &mut self,
        workspaces: &[evo_workspace::workspace::Workspace],
        subjects: &HashMap<String, String>,
        kinds: &HashMap<String, String>,
    ) -> &Self {
        let signature: Vec<(String, usize)> = workspaces
            .iter()
            .map(|workspace| (workspace.id().to_string(), workspace.snapshots().len()))
            .collect();
        if signature == self.signature {
            return self;
        }
        self.signature = signature;
        self.haystacks = Vec::new();
        self.kind_flags = Vec::new();
        for workspace in workspaces {
            let mut haystack = String::new();
            let mut flags = [false; 4];
            if let Some(subject) = subjects.get(&workspace.id().to_string()) {
                haystack.push_str(subject);
            }
            if let Some(kind) = kinds.get(&workspace.id().to_string()) {
                haystack.push(' ');
                haystack.push_str(kind);
                flags[0] = kind.contains("Document");
                flags[1] = kind.contains("Web");
                flags[2] = kind.contains("Code");
                flags[3] = kind.contains("Terminal");
            }
            self.haystacks.push(haystack.to_lowercase());
            self.kind_flags.push(flags);
        }
        self
    }
}

// ─── Your Work (the primary surface) ─────────────────────────────────────────

/// The "Your Work" section: the product's core promise.
///
/// Each work item is a clickable card that expands to show exactly what
/// will reopen, with a single "Continue" action. The card carries:
/// * A descriptive name (what the work IS, not a URL)
/// * A one-line description ("Working across X, Y, and Z")
/// * The resources that will reopen (shown before action, not after)
/// * Time information (sessions, last active)
/// What the person asked for from a work card.
pub enum WorkAction {
    /// Continue: restore the work at this engine-thread index and enter
    /// its pod.
    Continue(usize),
    /// Leave the active pod, restoring the desktop exactly as it was.
    Leave,
    /// Open the pod browser: the active work's pages hosted inside
    /// Evo's own window (Stage 3 web surfaces).
    OpenBrowser,
    /// Merge: declare two works one work (a confirmed proposal).
    Merge {
        subject_a: String,
        subject_b: String,
        other_name: String,
    },
}

pub fn work_section(
    ui: &mut egui::Ui,
    works: &[DisplayThread],
    _expanded: &mut Option<usize>,
    restore_note: Option<&str>,
    search_query: &mut String,
    merge_proposals: &[(String, String, String, String, usize)],
    active_pod: Option<&str>,
) -> Option<WorkAction> {
    let mut restore_requested = None;
    if works.is_empty() {
        return None;
    }
    ui::gap(ui, theme::S4);
    ui.scope(|ui| {
        ui.set_width(ui.available_width());
        ui::raised(ui, |ui| {
            ui.set_width(ui.available_width());
            ui::eyebrow(ui, "Your work");
            ui::display(ui, "Continue where you left off");

            // The active pod strip: entering made a place; this is how
            // the person leaves it (or notices where they are). The name
            // can be a long page title; the strip truncates it honestly.
            if let Some(active) = active_pod {
                ui::gap(ui, theme::S2);
                let mut leave_requested = false;
                let mut browser_requested = false;
                let short: String = {
                    let mut chars = active.chars();
                    let head: String = chars.by_ref().take(48).collect();
                    if chars.next().is_some() {
                        format!("{head}\u{2026}")
                    } else {
                        head
                    }
                };
                ui.horizontal(|ui| {
                    ui.label("\u{25CF}");
                    ui.strong(format!("In: {short}"));
                    ui.weak("\u{00B7}");
                    if ui.small_button("Leave pod").clicked() {
                        leave_requested = true;
                    }
                    ui.weak("\u{00B7}");
                    if ui.small_button("Pages").clicked() {
                        browser_requested = true;
                    }
                    ui.weak("\u{00B7}");
                    ui.weak("\u{2318}\u{21E7}E cycles pods");
                });
                if leave_requested {
                    restore_requested = Some(WorkAction::Leave);
                }
                if browser_requested {
                    restore_requested = Some(WorkAction::OpenBrowser);
                }
            }

            // Named retrieval: "find my pitch deck" or "reopen Evo"
            ui::gap(ui, theme::S2);
            let search_response = egui::TextEdit::singleline(search_query)
                .hint_text("Search your work...")
                .desired_width(ui.available_width())
                .show(ui)
                .response;
            if search_response.changed() && !search_query.trim().is_empty() {
                // Trigger retrieval on typing
            }
            if let Some(note) = restore_note {
                ui::gap(ui, theme::S2);
                ui::caption(ui, note);
            }
            ui::gap(ui, theme::S3);

            // When searching, filter to matching works.
            // The search does prefix matching on all searchable text:
            // title, narrative, trail (names + raw URLs), restore_set,
            // pending items, and completed items. Every word in the query
            // must match somewhere in the work (AND semantics).
            //
            // Matching is punctuation-insensitive: "arcteryx" must find
            // "Arc'teryx", and "dont" must find "don't". People type words,
            // not typography.
            let query = search_query.trim().to_lowercase();
            let matching: Vec<(usize, &DisplayThread)> = if query.is_empty() {
                works.iter().enumerate().collect()
            } else {
                let searchable: Vec<String> = works
                    .iter()
                    .map(|w| {
                        let mut s = String::new();
                        s.push_str(&w.name);
                        s.push(' ');
                        s.push_str(&w.understanding.narrative);
                        s.push(' ');
                        for entry in &w.understanding.trail {
                            s.push_str(&entry.name);
                            s.push(' ');
                            s.push_str(&entry.raw);
                            s.push(' ');
                        }
                        for resource in &w.bundle.restore_set {
                            s.push_str(resource);
                            s.push(' ');
                        }
                        for item in &w.understanding.pending {
                            s.push_str(item);
                            s.push(' ');
                        }
                        for item in &w.understanding.completed {
                            s.push_str(item);
                            s.push(' ');
                        }
                        normalize_for_search(&s)
                    })
                    .collect();
                works
                    .iter()
                    .enumerate()
                    .zip(&searchable)
                    .filter(|(_, searchable)| {
                        // AND semantics: every token must appear somewhere
                        query
                            .split_whitespace()
                            .all(|token| searchable.contains(&normalize_for_search(token)))
                    })
                    .map(|(indexed, _)| indexed)
                    .collect()
            };

            if matching.is_empty() && !query.is_empty() {
                ui::caption(ui, &format!("No work matches \"{}\".", search_query.trim()));
            } else {
                let last = matching.len().saturating_sub(1);
                for (pos, (index, work)) in matching.iter().enumerate() {
                    work_card(
                        ui,
                        work,
                        *index,
                        &mut restore_requested,
                        merge_proposals,
                    );
                    if pos < last {
                        ui::gap(ui, theme::S2);
                        ui::hairline_inset(ui, theme::M3 as f32);
                        ui::gap(ui, theme::S2);
                    }
                }
            }
        });
    });
    restore_requested
}

/// One work card using egui's native CollapsingHeader — click to expand.
fn work_card(
    ui: &mut egui::Ui,
    work: &DisplayThread,
    index: usize,
    restore_requested: &mut Option<WorkAction>,
    merge_proposals: &[(String, String, String, String, usize)],
) {
    // The header shows the phase + title + when the person was last here.
    let recency = time_ago(work);
    let header = if recency.is_empty() {
        format!(
            "{} \u{2014} {}",
            work.understanding.phase.label(),
            work.name
        )
    } else {
        format!(
            "{} \u{2014} {}  ·  {}",
            work.understanding.phase.label(),
            work.name,
            recency
        )
    };
    egui::CollapsingHeader::new(egui::RichText::new(&header).size(15.0).strong())
        .default_open(false)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            // The narrative: what a person reads to pick up where they left off
            ui::gap(ui, theme::S1);
            ui::caption(ui, &work.understanding.narrative);

            // The chronological trail: what the person touched, in order
            if !work.understanding.trail.is_empty() {
                ui::gap(ui, theme::S2);
                ui::eyebrow(ui, "What you were doing");
                for entry in &work.understanding.trail {
                    ui.horizontal(|ui| {
                        ui.label("\u{2022}");
                        ui::caption(ui, &format!("{} {}", entry.action.phrase(), entry.name));
                    });
                }
            }

            // What was done — honest evidence, never invented
            if !work.understanding.completed.is_empty() {
                ui::gap(ui, theme::S2);
                ui::eyebrow(ui, "What happened here");
                for item in &work.understanding.completed {
                    ui.horizontal(|ui| {
                        ui.label("\u{2022}");
                        ui::caption(ui, item);
                    });
                }
            }

            // The next step
            ui::gap(ui, theme::S2);
            ui::eyebrow(ui, "Next step");
            ui::caption(ui, &work.understanding.next_step);

            // The primary action
            ui::gap(ui, theme::S3);
            if ui.button("Enter this pod \u{2192}").clicked() {
                *restore_requested = Some(WorkAction::Continue(index));
            }

            // Merge proposals: the engine found this work co-attended with
            // another across sittings. Offering, never deciding — confirming
            // declares them one work through the daemon.
            //
            // A proposal is matched to a card by RESOURCE MEMBERSHIP (the
            // proposal's subject is a raw witness of one of this work's
            // pages), never by display name: the ledger identity and the
            // chrome-stripped card name are different vocabularies, and name
            // equality silently never matched (measured on the live log).
            for (subject_a, subject_b, name_a, name_b, sittings) in merge_proposals {
                let owns_a = work_owns(work, subject_a);
                let owns_b = work_owns(work, subject_b);
                let (other_name, other_subject, my_subject) = if owns_a && !owns_b {
                    (name_b.clone(), subject_b.clone(), subject_a.clone())
                } else if owns_b && !owns_a {
                    (name_a.clone(), subject_a.clone(), subject_b.clone())
                } else {
                    continue; // not this work's proposal (or already one body)
                };
                ui::gap(ui, theme::S2);
                if ui
                    .button(format!(
                        "Looks related to \"{other_name}\" ({sittings} shared sessions) — merge?"
                    ))
                    .clicked()
                {
                    *restore_requested = Some(WorkAction::Merge {
                        subject_a: my_subject,
                        subject_b: other_subject,
                        other_name,
                    });
                }
            }
        });
}

/// Whether a work owns the given proposal subject: the subject is a raw
/// witness (URL, document path, or window title) of one of the work's
/// member intervals. Membership, not name.
fn work_owns(work: &DisplayThread, subject: &str) -> bool {
    if subject.trim().is_empty() {
        return false;
    }
    work.member_subjects.iter().any(|m| m == subject)
}

/// When the person was last in this work: "12 minutes ago", "3 days ago".
/// The DisplayThread carries the engine's witnessed last-activity time;
/// absent evidence renders nothing rather than a guess.
fn time_ago(work: &DisplayThread) -> String {
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
        return format!(
            "{minutes} minute{} ago",
            if minutes == 1 { "" } else { "s" }
        );
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours} hour{} ago", if hours == 1 { "" } else { "s" });
    }
    let days = hours / 24;
    format!("{days} day{} ago", if days == 1 { "" } else { "s" })
}

/// Lowercases and drops punctuation so search matches words, not typography:
/// "arcteryx" finds "Arc'teryx". Alphanumerics and whitespace survive;
/// everything else vanishes (a dropped apostrophe must not become a space,
/// or "arcteryx" would not find "arc teryx").
fn normalize_for_search(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect()
}

/// A readable name for a resource: domain for URLs, filename for paths.
fn short_name(resource: &str) -> String {
    if resource.starts_with("http") {
        let url = resource
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let domain = url.split('/').next().unwrap_or(url);
        let domain = domain.trim_start_matches("www.");
        let parts: Vec<&str> = url.splitn(3, '/').collect();
        if parts.len() > 1 && !parts[1].is_empty() {
            return format!("{}/{}", domain, parts[1]);
        }
        return domain.to_string();
    }
    if resource.starts_with("file://") || resource.starts_with('/') {
        let path = resource.trim_start_matches("file://");
        let file = path.rsplit('/').next().unwrap_or(path);
        return file.to_string();
    }
    resource.chars().take(40).collect::<String>()
}

// ─── Legacy Home (below the new section) ─────────────────────────────────────

pub fn screen(
    ui: &mut egui::Ui,
    workspaces: &[evo_workspace::workspace::Workspace],
    cards: &[state::WorkspaceCard],
    subjects: &HashMap<String, String>,
    kinds: &HashMap<String, String>,
    stale: bool,
    selected: &mut Option<evo_workspace::workspace_id::WorkspaceId>,
    retrieval: &mut Retrieval<'_>,
) {
    ui::scroll(ui, |ui| {
        let searching = !retrieval.query.trim().is_empty();
        let narrowed = searching
            || *retrieval.kind != state::WorkKindFilter::All
            || *retrieval.period != state::WorkPeriodFilter::All;

        let work_total = cards.iter().filter(|card| card.standing.is_work()).count();
        let remembered_total = cards.len() - work_total;

        let visible = filter(workspaces, cards, subjects, kinds, retrieval);

        heading(
            ui,
            visible.len(),
            work_total,
            remembered_total,
            searching,
            narrowed,
        );

        if cards.len() > 1 || narrowed {
            ui::gap(ui, theme::S5);
            retrieval_bar(ui, retrieval);
        }
        ui::gap(ui, theme::S5);

        if visible.is_empty() {
            if narrowed {
                nothing_matched(ui, retrieval, narrowed);
            } else {
                nothing_is_work_yet(ui);
            }
            return;
        }

        ui.scope(|ui| {
            if stale {
                ui.set_opacity(0.62);
            }
            ui::raised(ui, |ui| {
                ui.set_width(ui.available_width());
                let last = visible.len() - 1;
                for (index, card) in visible.iter().enumerate() {
                    if work_row(ui, card).clicked() {
                        *selected = Some(card.id.clone());
                    }
                    if index != last {
                        ui::gap(ui, theme::S2);
                        ui::hairline_inset(ui, theme::M3 as f32);
                        ui::gap(ui, theme::S2);
                    }
                }
            });
        });

        ui::gap(ui, theme::S5);
        ui::provenance(
            ui,
            "Work you can continue, in the order Evo recorded it — those with \
             a marked continuation first. Nothing here is ranked, scored, or \
             promoted for being recent. Anything Evo has only witnessed in \
             passing it keeps by name, off this list; a search finds it.",
        );
        ui::gap(ui, theme::S6);
    });
}

fn heading(
    ui: &mut egui::Ui,
    visible: usize,
    work_total: usize,
    remembered_total: usize,
    searching: bool,
    narrowed: bool,
) {
    ui::eyebrow(ui, "Your work");
    ui::display(ui, "Continue where you left off");
    if searching {
        ui::caption(ui, &matches_phrase(visible));
    } else if narrowed {
        if visible == work_total {
            ui::caption(ui, &format!("{}.", bodies(visible)));
        } else {
            ui::caption(
                ui,
                &format!("{} of {}", bodies(visible), bodies(work_total)),
            );
        }
    } else if work_total == 0 {
        ui::caption(
            ui,
            &format!(
                "Nothing to continue yet — but Evo remembers {}.",
                things(remembered_total)
            ),
        );
    } else {
        ui::caption(ui, &format!("{} to continue.", bodies(work_total)));
        if remembered_total > 0 {
            ui::caption(ui, &also_remembered(remembered_total));
        }
    }
}

fn retrieval_bar(ui: &mut egui::Ui, retrieval: &mut Retrieval<'_>) {
    ui::search_field(ui, retrieval.query, "Search your work", "evo-home-search");
    ui::gap(ui, theme::S3);
    ui.horizontal_wrapped(|ui| {
        for filter in state::WorkKindFilter::all() {
            if filter == state::WorkKindFilter::All {
                continue;
            }
            if ui
                .selectable_label(*retrieval.kind == filter, filter.label())
                .clicked()
            {
                *retrieval.kind = if *retrieval.kind == filter {
                    state::WorkKindFilter::All
                } else {
                    filter
                };
            }
        }
    });
}

fn work_row(ui: &mut egui::Ui, card: &state::WorkspaceCard) -> egui::Response {
    ui.set_width(ui.available_width());
    ui.horizontal(|ui| {
        ui.heading(&card.title);
    });
    ui.allocate_response(egui::vec2(ui.available_width(), 1.0), egui::Sense::click())
}

fn nothing_matched(ui: &mut egui::Ui, retrieval: &mut Retrieval<'_>, narrowed: bool) {
    ui::caption(
        ui,
        &format!("No work matches \"{}\".", retrieval.query.trim()),
    );
    let _ = narrowed;
}

fn nothing_is_work_yet(ui: &mut egui::Ui) {
    ui::caption(
        ui,
        "Nothing has risen to work yet. Use your computer normally and Evo \
         will recognize the bodies of work you engage in.",
    );
}

fn filter<'a>(
    _workspaces: &'a [evo_workspace::workspace::Workspace],
    cards: &'a [state::WorkspaceCard],
    _subjects: &HashMap<String, String>,
    _kinds: &HashMap<String, String>,
    retrieval: &mut Retrieval<'_>,
) -> Vec<&'a state::WorkspaceCard> {
    let query = retrieval.query.trim().to_lowercase();
    cards
        .iter()
        .filter(|card| {
            if query.is_empty() {
                card.standing.is_work()
            } else {
                card.title.to_lowercase().contains(&query)
            }
        })
        .take(50)
        .collect()
}

pub struct Retrieval<'a> {
    pub query: &'a mut String,
    pub kind: &'a mut state::WorkKindFilter,
    pub period: &'a mut state::WorkPeriodFilter,
    pub memo: &'a mut Memo,
}

fn bodies(count: usize) -> String {
    match count {
        1 => "1 body of work".to_string(),
        n => format!("{n} bodies of work"),
    }
}

fn things(count: usize) -> String {
    match count {
        1 => "1 thing".to_string(),
        n => format!("{n} things"),
    }
}

fn matches_phrase(count: usize) -> String {
    format!("{count} match{}.", if count == 1 { "" } else { "es" })
}

fn also_remembered(count: usize) -> String {
    format!("Also remembers {} by name.", things(count))
}

// ─── Permission screens (unchanged) ───────────────────────────────────────────

pub fn permission_required(ui: &mut egui::Ui, detail: Option<&str>, open_settings: impl FnOnce()) {
    ui::gap(ui, theme::S5);
    ui::raised(ui, |ui| {
        ui.set_width(ui.available_width());
        ui::eyebrow(ui, "Permission required");
        ui::display(ui, "Evo needs Accessibility to see your work");
        ui::gap(ui, theme::S2);
        ui::caption(
            ui,
            detail.unwrap_or(
                "Grant Accessibility in System Settings so Evo can observe \
                 which window you are working in.",
            ),
        );
        ui::gap(ui, theme::S3);
        if ui.button("Open System Settings").clicked() {
            open_settings();
        }
    });
    ui::gap(ui, theme::S5);
}

pub fn empty(ui: &mut egui::Ui, capturing: bool) {
    ui::gap(ui, theme::S5);
    ui::raised(ui, |ui| {
        ui.set_width(ui.available_width());
        ui::eyebrow(ui, if capturing { "Capturing" } else { "Waiting" });
        ui::display(
            ui,
            if capturing {
                "Evo is learning your work"
            } else {
                "Evo is not capturing yet"
            },
        );
        ui::gap(ui, theme::S2);
        ui::caption(
            ui,
            if capturing {
                "Use your computer normally. Evo observes which applications \
                 and resources you engage with, and begins to recognize \
                 bodies of work after a few sessions."
            } else {
                "The capture worker is not running. Restart Evo to begin \
                 capturing your work activity."
            },
        );
    });
    ui::gap(ui, theme::S5);
}
