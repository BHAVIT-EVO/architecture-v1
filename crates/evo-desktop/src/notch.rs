//! The notch engine Evo renders into the macOS notch/Dynamic Island
//! surface (see `notch_view` in the UI layer): a Live-Activity-style
//! content priority system, ported in spirit from jackson-storm's
//! DynamicNotch (GPL-3.0 — concepts only, no code copied), plus Evo's
//! voice listening state machine on top.
//!
//! # Model
//!
//! Content items compete for the notch by integer `priority` (higher
//! wins). Two kinds:
//!
//! - **Live activity** — persistent while active (Evo's rack summary, a
//!   work window indicator). Shown in compact form; expandable.
//! - **Temporary** — preempts the live activity for a duration, then the
//!   highest-priority suspended live activity is restored (wake
//!   listening, outcomes, HUD-style alerts).
//!
//! The **voice state machine** rides on top: wake gesture → listening
//! (partial transcript streams) → command heard → outcome → done. When
//! the resolution is ambiguous or misses but recent work exists, the
//! phase becomes [`VoicePhase::Choice`]: a swipeable candidate carousel
//! (each candidate restorable by `work_id` via the daemon's CHOOSE
//! verb). Beside it sits the **works browse** (`ContentId::EvoBrowse`,
//! [`NotchEngine::show_works`]): the rack as a list a person steps
//! through and opens without the main window. The engine exposes
//! [`NotchEngine::voice_event`] for the driver's stream and
//! [`NotchEngine::tick`] for timers; the view layer reads
//! [`NotchEngine::model`] every frame.
//!
//! # Coexistence with other notch apps
//!
//! The engine never assumes it owns the notch. [`SurfaceDecision`]
//! answers "may I claim the strip?" from a probe of whoever else is
//! there (see `notch_window::occupancy_probe`); when another notch app
//! owns the strip Evo renders as a floating card below it instead, and
//! yielded surfaces get their space back when Evo's activity ends.

use std::time::{Duration, Instant};

// ─── Content model ───────────────────────────────────────────────────────────

/// Stable identity for notch content (DynamicNotch's `id`/`stackID`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentId {
    /// Legacy identity retained for verification/backwards-compatible input.
    /// Normal idle is intentionally hidden: the notch host must order its
    /// panel out instead of painting a fallback pill or idle glyph.
    EvoIdle,
    /// Live listening surface: wake gesture, partial transcript, capture.
    EvoListening,
    /// A commandEO outcome card (Restored/NotFound/Failed/Brief).
    EvoOutcome,
    /// Evo's rack summary (n works, live while the rack has content).
    EvoRack,
    /// The candidate-choice carousel (a temporary like EvoOutcome).
    EvoChoice,
    /// The works-browse carousel: the rack as a resumable list (a
    /// temporary like EvoChoice, opened by explicit user intent).
    EvoBrowse,
}

/// How long a temporary content item stays before the engine restores
/// the underlying live activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tenure {
    /// Persistent while active (until `hide` is called).
    Persistent,
    /// Auto-hide after this duration.
    For(Duration),
}

/// A notch content item: identity, priority, tenure, and expansion.
#[derive(Debug, Clone)]
pub struct Content {
    pub id: ContentId,
    pub priority: i32,
    pub tenure: Tenure,
    /// Whether tapping the compact form expands the full surface.
    pub expandable: bool,
    /// Started_at for tenure timers (set by the engine on show).
    pub shown_at: Instant,
}

impl Content {
    /// An unshown content item (shown_at is stamped when displayed).
    pub fn new(id: ContentId, priority: i32, tenure: Tenure) -> Self {
        Self {
            id,
            priority,
            tenure,
            expandable: false,
            shown_at: Instant::now(),
        }
    }

    /// A content item shown right now.
    pub fn new_shown(id: ContentId, priority: i32, tenure: Tenure, now: Instant) -> Self {
        Self {
            shown_at: now,
            ..Self::new(id, priority, tenure)
        }
    }
}

/// Module-scope priority constants (DynamicNotch's content priorities).
pub mod priority {
    /// The idle glyph (always lose).
    pub const IDLE: i32 = 1;
    /// The rack summary live activity.
    pub const RACK: i32 = 6;
    /// The works-browse carousel (temporary; explicit user intent, so it
    /// preempts the rack, but an outcome still outranks it).
    pub const BROWSE: i32 = 8;
    /// The outcome card (temporary).
    pub const OUTCOME: i32 = 9;
    /// The live listening surface (temporary, highest Evo priority).
    pub const LISTENING: i32 = 10;
    /// The candidate-choice carousel (temporary; preempts the outcome
    /// card priority so an Ambiguous answer always carries its picks).
    pub const CHOICE: i32 = 11;
}

// ─── Voice state machine ─────────────────────────────────────────────────────

/// The listening lifecycle, driven entirely by voice events and time.
#[derive(Debug, Clone, PartialEq)]
pub enum VoicePhase {
    /// Nothing heard; the surface is hidden until there is real content.
    Idle,
    /// A wake gesture fired; capturing the follow-up, partial streaming.
    Listening,
    /// The follow-up command was transcribed and dispatched.
    Heard {},
    /// An outcome came back (shown as a card, auto-collapses).
    Outcome,
    /// The answer carried candidates: a swipeable carousel, one focused
    /// at `index`. The surface auto-collapses like an outcome card, but
    /// the person can pick long before it does.
    Choice {
        candidates: Vec<evo_voice::VoiceCandidate>,
        /// The focused candidate (0-based, wraps).
        index: usize,
    },
}

/// Visualization state for the view layer each frame.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceState {
    pub phase: VoicePhase,
    /// The newest partial transcript, when listening/heard.
    pub partial: Option<String>,
    /// Outcome copy retained for the notch renderer. The voice plumbing still
    /// carries the canonical `VoiceOutcome`; this is only display data.
    pub outcome: Option<OutcomeState>,
    /// When the phase began (for pulsing/animate decisions).
    pub since: Instant,
}

/// Textual outcome content the notch can render without reaching into the
/// daemon or the desktop window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeState {
    pub eyebrow: String,
    pub title: String,
    pub detail: String,
}

// ─── Works browse ────────────────────────────────────────────────────────────

/// How long an idle browse carousel stays before collapsing. Long enough
/// to look across the rack; a step refreshes it, so browsing is never cut
/// off mid-look.
const BROWSE_TENURE: Duration = Duration::from_secs(20);

/// One work in the notch's browse carousel, already composed for display:
/// the caller folds the daemon's facts into the subtitle line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseEntry {
    /// The daemon's stable numeric work id (the CHOOSE key).
    pub id: String,
    /// What the work is about, from its own witnessed material.
    pub title: String,
    /// The supporting line under the title (status · sittings · reason).
    pub subtitle: String,
}

/// An open browse carousel: the works and the focused index (0-based,
/// wraps on step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseSession {
    pub entries: Vec<BrowseEntry>,
    pub index: usize,
}

impl BrowseSession {
    /// The focused entry, when the session has one.
    pub fn focused(&self) -> Option<&BrowseEntry> {
        self.entries.get(self.index)
    }
}

// ─── Engine ──────────────────────────────────────────────────────────────────

/// The notch presentation engine: content scheduling + voice phases.
#[derive(Debug)]
pub struct NotchEngine {
    /// Active live activities, priority-ordered (index 0 = highest).
    live: Vec<Content>,
    /// The current temporary item, when one is showing.
    temporary: Option<Content>,
    /// The live activity suspended by the temporary item.
    suspended: Option<Content>,
    /// The voice lifecycle.
    pub voice: VoiceState,
    /// The open works-browse carousel, when one is showing. Kept in sync
    /// with the `EvoBrowse` temporary: the session's data lives here, the
    /// scheduling (priority, tenure, preemption) rides `temporary`.
    browse: Option<BrowseSession>,
    /// Coexistence: does Evo actually own the strip right now, or is it
    /// floating beside someone else's notch UI?
    pub owns_strip: bool,
}

const VOICE_LISTENING_TENURE: Duration = Duration::from_secs(60);

impl Default for NotchEngine {
    fn default() -> Self {
        Self {
            live: Vec::new(),
            temporary: None,
            suspended: None,
            voice: VoiceState {
                phase: VoicePhase::Idle,
                partial: None,
                outcome: None,
                since: Instant::now(),
            },
            browse: None,
            owns_strip: true,
        }
    }
}

impl NotchEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows a live activity (Persistent) at a priority; replaces any
    /// existing activity with the same id. Hidden temporaries win the
    /// visible slot, but this stays scheduled underneath.
    pub fn show_live(&mut self, id: ContentId, priority: i32, expandable: bool) {
        self.live.retain(|c| c.id != id);
        self.live.push(Content {
            expandable,
            ..Content::new(id, priority, Tenure::Persistent)
        });
        self.live.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Removes a live activity.
    pub fn hide_live(&mut self, id: ContentId) {
        self.live.retain(|c| c.id != id);
    }

    /// Shows a temporary item: preempts the highest live activity for
    /// `tenure`, restoring it after (DynamicNotch's alert semantics).
    /// `now` is injected so tests drive the clock deterministically.
    pub fn show_temporary(&mut self, id: ContentId, priority: i32, tenure: Tenure, now: Instant) {
        // Re-showing the same temporary just refreshes its timer.
        if self.temporary.as_ref().is_some_and(|c| c.id == id) {
            self.temporary = Some(Content {
                id,
                priority,
                tenure,
                shown_at: now,
                ..*self.temporary.as_ref().unwrap()
            });
            return;
        }
        // Push down whatever temporary is showing first.
        if let Some(current) = self.temporary.take() {
            self.temporary = Some(Content::new(current.id, current.priority, current.tenure));
        }
        if self.suspended.is_none() {
            self.suspended = self.live.first().cloned();
        }
        self.temporary = Some(Content {
            expandable: false,
            ..Content::new_shown(id, priority, tenure, now)
        });
    }

    /// Hides a temporary item immediately and restores the suspended
    /// live activity (used when the voice phase resolves early). Hiding
    /// the browse content also closes the browse session: the data must
    /// never outlive the surface that shows it.
    pub fn hide_temporary(&mut self, id: ContentId) {
        if self.temporary.as_ref().is_some_and(|c| c.id == id) {
            self.temporary = None;
            if id == ContentId::EvoBrowse {
                self.browse = None;
            }
            self.restore_suspended();
        }
    }

    fn restore_suspended(&mut self) {
        if let Some(suspended) = self.suspended.take() {
            if self.live.iter().any(|c| c.id == suspended.id) {
                // Still active; nothing to do — visibility derives from
                // live + temporary each frame.
            }
        }
    }

    /// Advances timers: expires the temporary when its tenure elapses.
    /// Call once per frame (or once per second).
    pub fn tick(&mut self, now: Instant) {
        if let Some(temporary) = &self.temporary {
            let expired = match temporary.tenure {
                Tenure::Persistent => false,
                Tenure::For(d) => now.duration_since(temporary.shown_at) >= d,
            };
            if expired {
                self.temporary = None;
                // An expired browse is over: no stale session may linger
                // behind the restored live activity.
                self.browse = None;
                self.restore_suspended();
                // Voice phases: expiry returns the surface to idle.
                if self.voice.phase != VoicePhase::Idle {
                    self.voice = VoiceState {
                        phase: VoicePhase::Idle,
                        partial: None,
                        outcome: None,
                        since: now,
                    };
                }
            }
        }
    }

    /// Next expiry that must wake the host even if no new input arrives.
    pub fn deadline(&self) -> Option<Instant> {
        self.temporary
            .as_ref()
            .and_then(|content| match content.tenure {
                Tenure::Persistent => None,
                Tenure::For(duration) => content.shown_at.checked_add(duration),
            })
    }

    /// What the view renders: the visible content, preemption-aware.
    pub fn model(&self) -> Option<&Content> {
        if let Some(temporary) = &self.temporary {
            if temporary.id != ContentId::EvoIdle {
                return Some(temporary);
            }
        }
        self.live
            .iter()
            .find(|content| content.id != ContentId::EvoIdle)
    }

    /// True when the visible content carries its expanded form (the
    /// voice surface can expand even a compact glyph).
    pub fn is_expanded(&self) -> bool {
        self.browse.is_some()
            || matches!(
                self.voice.phase,
                VoicePhase::Listening | VoicePhase::Heard { .. } | VoicePhase::Choice { .. }
            )
    }

    // ── Voice event intake ────────────────────────────────────────────────

    /// Feeds one driver event into the voice lifecycle.
    pub fn voice_event(&mut self, event: &evo_voice::VoiceEvent, now: Instant) {
        match event {
            evo_voice::VoiceEvent::WakeGesture { .. } => {
                // A fresh wake is a new session: it preempts an open
                // browse carousel (which rides the same temporary slot).
                self.browse = None;
                self.voice = VoiceState {
                    phase: VoicePhase::Listening,
                    partial: None,
                    outcome: None,
                    since: now,
                };
                self.show_temporary(
                    ContentId::EvoListening,
                    priority::LISTENING,
                    Tenure::For(VOICE_LISTENING_TENURE),
                    now,
                );
            }
            evo_voice::VoiceEvent::Listening { partial } => {
                // Refresh the tenure: an active speaker keeps listening.
                self.voice.partial = partial.clone();
                self.voice.outcome = None;
                if self.voice.phase == VoicePhase::Listening {
                    self.show_temporary(
                        ContentId::EvoListening,
                        priority::LISTENING,
                        Tenure::For(VOICE_LISTENING_TENURE),
                        now,
                    );
                }
            }
            evo_voice::VoiceEvent::HeardCommand { command } => {
                self.voice = VoiceState {
                    phase: VoicePhase::Heard {},
                    partial: Some(command.clone()),
                    outcome: None,
                    since: now,
                };
                // Keep the expanded surface alive while the final local
                // Whisper pass and command dispatch complete.
                self.show_temporary(
                    ContentId::EvoListening,
                    priority::LISTENING,
                    Tenure::For(VOICE_LISTENING_TENURE),
                    now,
                );
            }
            evo_voice::VoiceEvent::Outcome(outcome) => {
                // Any new outcome is fresher than a browse: the result
                // card (or choice carousel) takes the surface.
                self.browse = None;
                // A candidate-bearing outcome becomes the choice
                // carousel instead of the plain outcome card — the
                // candidates are carried, never discarded.
                let candidates: Vec<evo_voice::VoiceCandidate> = match outcome {
                    evo_voice::VoiceOutcome::Ambiguous { candidates } => candidates.clone(),
                    evo_voice::VoiceOutcome::NotFound { recent } => recent.clone(),
                    _ => vec![],
                };
                if candidates.is_empty() {
                    let outcome_state = match outcome {
                        evo_voice::VoiceOutcome::Restored { title, summary, .. } => OutcomeState {
                            eyebrow: "RESTORED".into(),
                            title: title.clone(),
                            detail: summary.clone(),
                        },
                        evo_voice::VoiceOutcome::Failed { reason } => OutcomeState {
                            eyebrow: "VOICE ERROR".into(),
                            title: "Couldn't restore".into(),
                            detail: reason.clone(),
                        },
                        evo_voice::VoiceOutcome::NoCommand => OutcomeState {
                            eyebrow: "NO COMMAND".into(),
                            title: "Nothing heard".into(),
                            detail: "Try saying hey Evo again.".into(),
                        },
                        evo_voice::VoiceOutcome::Brief { count, narration } => OutcomeState {
                            eyebrow: "EVO BRIEF".into(),
                            title: format!("{count} work{}", if *count == 1 { "" } else { "s" }),
                            detail: narration.clone(),
                        },
                        evo_voice::VoiceOutcome::Ambiguous { .. }
                        | evo_voice::VoiceOutcome::NotFound { .. } => OutcomeState {
                            eyebrow: "EVO".into(),
                            title: "No matching work".into(),
                            detail: "Try a more specific description.".into(),
                        },
                    };
                    self.voice = VoiceState {
                        phase: VoicePhase::Outcome,
                        partial: None,
                        outcome: Some(outcome_state),
                        since: now,
                    };
                    self.show_temporary(
                        ContentId::EvoOutcome,
                        priority::OUTCOME,
                        Tenure::For(Duration::from_secs(5)),
                        now,
                    );
                } else {
                    self.voice = VoiceState {
                        phase: VoicePhase::Choice {
                            candidates,
                            index: 0,
                        },
                        partial: None,
                        outcome: None,
                        since: now,
                    };
                    self.show_temporary(
                        ContentId::EvoChoice,
                        priority::CHOICE,
                        Tenure::For(Duration::from_secs(12)),
                        now,
                    );
                }
            }
        }
    }

    /// Steps the focused candidate by ±1 with wraparound. No-op unless
    /// the phase is `Choice`. `+1` = the next work (drag left visually).
    pub fn choice_step(&mut self, delta: isize, now: Instant) {
        if let VoicePhase::Choice { candidates, index } = &mut self.voice.phase {
            let count = candidates.len();
            if count == 0 {
                return;
            }
            *index = ((*index as isize + delta).rem_euclid(count as isize)) as usize;
            // A swipe keeps the surface alive: refresh the tenure so a
            // browse is never cut off mid-look.
            self.show_temporary(
                ContentId::EvoChoice,
                priority::CHOICE,
                Tenure::For(Duration::from_secs(12)),
                now,
            );
        }
    }

    /// The focused candidate, when in the Choice phase.
    pub fn focused_candidate(&self) -> Option<&evo_voice::VoiceCandidate> {
        match &self.voice.phase {
            VoicePhase::Choice { candidates, index } => candidates.get(*index),
            _ => None,
        }
    }

    // ── Works browse ─────────────────────────────────────────────────────

    /// Opens the works-browse carousel: the rack as a list a person can
    /// step through and open from. A Browse is explicit user intent, so
    /// it preempts listening — a new wake gesture is a new session. The
    /// surface auto-collapses after [`BROWSE_TENURE`] unless a step
    /// refreshes it.
    ///
    /// An empty list is not content: it dismisses any open browse rather
    /// than paint an empty carousel.
    pub fn show_works(&mut self, entries: Vec<BrowseEntry>) {
        if entries.is_empty() {
            self.browse = None;
            self.hide_temporary(ContentId::EvoBrowse);
            return;
        }
        let now = Instant::now();
        self.browse = Some(BrowseSession { entries, index: 0 });
        // The browse is the surface now: an in-flight listening capture
        // ends with its temporary, and only a fresh wake starts a new one.
        self.voice = VoiceState {
            phase: VoicePhase::Idle,
            partial: None,
            outcome: None,
            since: now,
        };
        self.show_temporary(
            ContentId::EvoBrowse,
            priority::BROWSE,
            Tenure::For(BROWSE_TENURE),
            now,
        );
    }

    /// Steps the focused work by ±1 with wraparound. No-op unless a
    /// browse session is open. `+1` = the next work (drag left visually).
    /// A step refreshes the tenure so browsing is never cut off mid-look.
    pub fn browse_step(&mut self, delta: isize) {
        self.browse_step_at(delta, Instant::now());
    }

    /// [`Self::browse_step`] with the clock injected, for deterministic
    /// tenure tests (every engine timer takes `now`; the public step
    /// rides the real clock).
    fn browse_step_at(&mut self, delta: isize, now: Instant) {
        let count = match &self.browse {
            Some(session) => session.entries.len(),
            None => return,
        };
        if count == 0 {
            return;
        }
        if let Some(session) = self.browse.as_mut() {
            session.index = ((session.index as isize + delta).rem_euclid(count as isize)) as usize;
        }
        self.show_temporary(
            ContentId::EvoBrowse,
            priority::BROWSE,
            Tenure::For(BROWSE_TENURE),
            now,
        );
    }

    /// The open browse session, when one is showing.
    pub fn browse(&self) -> Option<&BrowseSession> {
        self.browse.as_ref()
    }

    /// Whether the engine has any Evo content to show at all.  Idle is a
    /// genuinely empty state: the notch host must order its panel out rather
    /// than leave a transparent helper window in the z-order.
    pub fn has_content(&self) -> bool {
        self.model().is_some() || self.voice.phase != VoicePhase::Idle || self.browse.is_some()
    }

    /// Dismisses the current surface and returns to hidden idle.
    ///
    /// This is deliberately stronger than `hide_temporary`: a person who
    /// taps Close (or presses Escape) asked to leave the notch surface, so a
    /// suspended rack/idle item must not immediately reappear underneath the
    /// collapse animation.
    pub fn dismiss(&mut self, now: Instant) {
        self.live.clear();
        self.temporary = None;
        self.suspended = None;
        self.browse = None;
        self.voice = VoiceState {
            phase: VoicePhase::Idle,
            partial: None,
            outcome: None,
            since: now,
        };
    }
}

// ─── Coexistence decision ────────────────────────────────────────────────────

/// Where Evo's surface renders this session, decided by a probe of the
/// strip (who else is in the notch area?).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceDecision {
    /// No other window in the strip: Evo owns the notch surface.
    Own,
    /// Another notch app's window is in the strip: Evo floats a card
    /// below it instead of covering it.
    Float,
}

/// Decides from an occupancy count how Evo should present. The count is
/// deliberately named for what it represents: only elevated, visible foreign
/// notch owners may yield the strip. Platform probes must filter ordinary app
/// windows before calling this boundary.
pub fn decide_surface(foreign_notch_owners: usize) -> SurfaceDecision {
    if foreign_notch_owners == 0 {
        SurfaceDecision::Own
    } else {
        SurfaceDecision::Float
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> NotchEngine {
        NotchEngine::new()
    }

    fn candidates() -> Vec<evo_voice::VoiceCandidate> {
        vec![
            evo_voice::VoiceCandidate::work("7", "Yesterday's notes"),
            evo_voice::VoiceCandidate::work("9", "Morning writing"),
        ]
    }

    #[test]
    fn idle_engine_has_no_visible_content() {
        let mut e = engine();
        e.tick(Instant::now());
        assert!(e.model().is_none());
        assert!(!e.has_content(), "idle is hidden, not an idle glyph");
        assert_eq!(e.voice.phase, VoicePhase::Idle);
    }

    #[test]
    fn legacy_idle_item_cannot_make_the_surface_visible() {
        let mut e = engine();
        e.show_live(ContentId::EvoIdle, priority::IDLE, false);

        assert!(e.model().is_none(), "idle is not a visible content item");
        assert!(!e.has_content(), "idle must stay fully hidden");
    }

    #[test]
    fn temporary_content_exposes_a_deadline_for_idle_timer_wakeup() {
        let mut e = engine();
        let now = Instant::now();
        e.show_temporary(
            ContentId::EvoOutcome,
            priority::OUTCOME,
            Tenure::For(Duration::from_secs(5)),
            now,
        );
        assert_eq!(e.deadline(), Some(now + Duration::from_secs(5)));

        e.show_live(ContentId::EvoRack, priority::RACK, true);
        e.hide_temporary(ContentId::EvoOutcome);
        assert_eq!(e.deadline(), None);
    }

    #[test]
    fn dismiss_clears_live_and_temporary_content_to_hidden_idle() {
        let mut e = engine();
        let now = Instant::now();
        e.show_live(ContentId::EvoRack, priority::RACK, true);
        e.voice_event(&evo_voice::VoiceEvent::WakeGesture { score: None }, now);
        assert!(e.has_content());

        e.dismiss(now + Duration::from_millis(1));

        assert!(e.model().is_none(), "dismiss must not restore the rack");
        assert!(!e.has_content(), "dismiss returns to hidden idle");
        assert_eq!(e.voice.phase, VoicePhase::Idle);
        assert!(e.voice.partial.is_none());
        assert!(e.voice.outcome.is_none());
    }

    #[test]
    fn wake_gesture_starts_listening_and_preempts_the_rack() {
        let mut e = engine();
        e.show_live(ContentId::EvoRack, priority::RACK, true);
        assert!(matches!(e.model().map(|c| c.id), Some(ContentId::EvoRack)));

        let now = Instant::now();
        e.voice_event(
            &evo_voice::VoiceEvent::WakeGesture { score: Some(0.4) },
            now,
        );
        assert_eq!(e.voice.phase, VoicePhase::Listening);
        assert!(e.is_expanded());
        // The temporary listening surface preempts the rack.
        assert!(matches!(
            e.model().map(|c| c.id),
            Some(ContentId::EvoListening)
        ));
    }

    #[test]
    fn listening_partial_updates_and_refreshes_tenure() {
        let mut e = engine();
        let now = Instant::now();
        e.voice_event(&evo_voice::VoiceEvent::WakeGesture { score: None }, now);
        // Simulate the initial tenure aging out without a new event. The
        // listening surface now outlives the driver's 10-second capture cap.
        let later = now + Duration::from_secs(13);
        e.tick(later);
        assert_eq!(
            e.voice.phase,
            VoicePhase::Idle,
            "expired listening returns to idle"
        );

        // With partials arriving, each refreshes the tenure.
        e.voice_event(&evo_voice::VoiceEvent::WakeGesture { score: None }, now);
        e.voice_event(
            &evo_voice::VoiceEvent::Listening {
                partial: Some("open my".to_string()),
            },
            now + Duration::from_secs(3),
        );
        e.tick(now + Duration::from_secs(3) + Duration::from_secs(4));
        assert_eq!(
            e.voice.phase,
            VoicePhase::Listening,
            "a fresh partial within tenure keeps listening"
        );
    }

    #[test]
    fn heard_command_then_outcome_card_then_idle() {
        let mut e = engine();
        let now = Instant::now();
        e.voice_event(&evo_voice::VoiceEvent::WakeGesture { score: None }, now);
        e.voice_event(
            &evo_voice::VoiceEvent::HeardCommand {
                command: "open my writing".to_string(),
            },
            now,
        );
        assert_eq!(e.voice.phase, VoicePhase::Heard {});
        assert_eq!(e.voice.partial.as_deref(), Some("open my writing"));

        e.voice_event(
            &evo_voice::VoiceEvent::Outcome(evo_voice::VoiceOutcome::Restored {
                title: "Writing".to_string(),
                summary: "Opened 1".to_string(),
                offered: vec![],
            }),
            now,
        );
        assert_eq!(e.voice.phase, VoicePhase::Outcome);
        assert!(matches!(
            e.model().map(|c| c.id),
            Some(ContentId::EvoOutcome)
        ));

        e.tick(now + Duration::from_secs(6));
        assert_eq!(e.voice.phase, VoicePhase::Idle, "outcome expires to idle");
    }

    #[test]
    fn ambiguous_outcome_opens_the_choice_carousel() {
        let mut e = engine();
        let now = Instant::now();
        e.voice_event(
            &evo_voice::VoiceEvent::Outcome(evo_voice::VoiceOutcome::Ambiguous {
                candidates: candidates(),
            }),
            now,
        );
        match &e.voice.phase {
            VoicePhase::Choice { candidates, index } => {
                assert_eq!(candidates.len(), 2);
                assert_eq!(*index, 0, "starts focused on the first candidate");
            }
            other => panic!("expected Choice, got {other:?}"),
        }
        assert!(e.is_expanded(), "choice surface is expanded");
        assert_eq!(
            e.model().map(|c| c.id),
            Some(ContentId::EvoChoice),
            "choice preempts as a temporary"
        );
        assert_eq!(e.focused_candidate().map(|c| c.work_id.as_str()), Some("7"));
    }

    #[test]
    fn not_found_with_recent_work_also_opens_the_choice_carousel() {
        let mut e = engine();
        let now = Instant::now();
        e.voice_event(
            &evo_voice::VoiceEvent::Outcome(evo_voice::VoiceOutcome::NotFound {
                recent: candidates(),
            }),
            now,
        );
        assert!(matches!(e.voice.phase, VoicePhase::Choice { .. }));
    }

    #[test]
    fn candidate_free_outcomes_stay_the_plain_outcome_card() {
        let mut e = engine();
        let now = Instant::now();
        for outcome in [
            evo_voice::VoiceOutcome::Failed {
                reason: "sharp noise, no words".to_string(),
            },
            evo_voice::VoiceOutcome::NoCommand,
            evo_voice::VoiceOutcome::Restored {
                title: "Done".to_string(),
                summary: "Opened 1".to_string(),
                offered: vec![],
            },
        ] {
            e.voice_event(&evo_voice::VoiceEvent::Outcome(outcome), now);
            assert_eq!(
                e.voice.phase,
                VoicePhase::Outcome,
                "no candidates → the plain outcome card stays"
            );
            assert!(matches!(
                e.model().map(|c| c.id),
                Some(ContentId::EvoOutcome)
            ));
        }
    }

    #[test]
    fn choice_step_wraps_both_directions_and_noops_outside_choice() {
        let mut e = engine();
        e.choice_step(1, Instant::now());
        assert_eq!(e.voice.phase, VoicePhase::Idle, "no-op outside Choice");

        let now = Instant::now();
        e.voice_event(
            &evo_voice::VoiceEvent::Outcome(evo_voice::VoiceOutcome::Ambiguous {
                candidates: candidates(),
            }),
            now,
        );
        e.choice_step(1, now);
        assert_eq!(e.focused_candidate().map(|c| c.work_id.as_str()), Some("9"));
        e.choice_step(1, now);
        assert_eq!(
            e.focused_candidate().map(|c| c.work_id.as_str()),
            Some("7"),
            "wraps forward"
        );
        e.choice_step(-1, now);
        assert_eq!(e.focused_candidate().map(|c| c.work_id.as_str()), Some("9"));
        e.choice_step(-1, now);
        assert_eq!(e.focused_candidate().map(|c| c.work_id.as_str()), Some("7"));
        e.choice_step(-1, now);
        assert_eq!(
            e.focused_candidate().map(|c| c.work_id.as_str()),
            Some("9"),
            "wraps backward"
        );
    }

    #[test]
    fn choice_carousel_expires_to_idle() {
        let mut e = engine();
        let now = Instant::now();
        e.voice_event(
            &evo_voice::VoiceEvent::Outcome(evo_voice::VoiceOutcome::Ambiguous {
                candidates: candidates(),
            }),
            now,
        );
        e.tick(now + Duration::from_secs(13));
        assert_eq!(e.voice.phase, VoicePhase::Idle, "choice expires to idle");
    }

    #[test]
    fn swipe_refreshes_the_choice_tenure() {
        let mut e = engine();
        let now = Instant::now();
        e.voice_event(
            &evo_voice::VoiceEvent::Outcome(evo_voice::VoiceOutcome::Ambiguous {
                candidates: candidates(),
            }),
            now,
        );
        // Just inside expiry…
        e.tick(now + Duration::from_secs(11));
        assert!(matches!(e.voice.phase, VoicePhase::Choice { .. }));
        // …a swipe arrives, which must reset the clock.
        e.choice_step(1, now + Duration::from_secs(11));
        e.tick(now + Duration::from_secs(11) + Duration::from_secs(6));
        assert!(
            matches!(e.voice.phase, VoicePhase::Choice { .. }),
            "a swipe keeps the carousel alive"
        );
        // And it still expires eventually.
        e.tick(now + Duration::from_secs(11) + Duration::from_secs(13));
        assert_eq!(e.voice.phase, VoicePhase::Idle);
    }

    fn works() -> Vec<BrowseEntry> {
        vec![
            BrowseEntry {
                id: "7".to_string(),
                title: "Yesterday's notes".to_string(),
                subtitle: "active · 3 sittings · You left off in Editor.".to_string(),
            },
            BrowseEntry {
                id: "9".to_string(),
                title: "Morning writing".to_string(),
                subtitle: "dormant · 12 sittings · Three sittings this week.".to_string(),
            },
        ]
    }

    #[test]
    fn show_works_opens_the_browse_carousel_as_a_temporary() {
        let mut e = engine();
        assert!(e.browse().is_none(), "no browse until a listing arrives");

        e.show_works(works());

        let session = e.browse().expect("browse opens");
        assert_eq!(session.entries.len(), 2);
        assert_eq!(session.index, 0, "starts focused on the first work");
        assert_eq!(session.focused().map(|w| w.id.as_str()), Some("7"));
        let content = e.model().expect("browse is visible content");
        assert_eq!(content.id, ContentId::EvoBrowse);
        assert_eq!(content.priority, priority::BROWSE);
        assert!(e.is_expanded(), "the browse surface is expanded");
        assert!(e.has_content());
        assert!(e.deadline().is_some(), "the host wakes for browse expiry");
        assert_eq!(
            e.voice.phase,
            VoicePhase::Idle,
            "browse is not a voice phase"
        );
    }

    #[test]
    fn show_works_preempts_listening() {
        let mut e = engine();
        let now = Instant::now();
        e.voice_event(&evo_voice::VoiceEvent::WakeGesture { score: None }, now);
        assert_eq!(e.model().map(|c| c.id), Some(ContentId::EvoListening));

        e.show_works(works());

        assert_eq!(
            e.model().map(|c| c.id),
            Some(ContentId::EvoBrowse),
            "the explicit browse takes the surface from listening"
        );
        assert_eq!(
            e.voice.phase,
            VoicePhase::Idle,
            "the displaced listening session does not linger"
        );
    }

    #[test]
    fn a_new_wake_gesture_preempts_browse() {
        let mut e = engine();
        e.show_works(works());

        e.voice_event(
            &evo_voice::VoiceEvent::WakeGesture { score: None },
            Instant::now(),
        );

        assert!(
            e.browse().is_none(),
            "a fresh wake drops the browse session"
        );
        assert_eq!(e.voice.phase, VoicePhase::Listening);
        assert_eq!(e.model().map(|c| c.id), Some(ContentId::EvoListening));
    }

    #[test]
    fn a_new_outcome_preempts_browse() {
        let mut e = engine();
        e.show_works(works());

        e.voice_event(
            &evo_voice::VoiceEvent::Outcome(evo_voice::VoiceOutcome::Restored {
                title: "Writing".to_string(),
                summary: "Opened 1".to_string(),
                offered: vec![],
            }),
            Instant::now(),
        );

        assert!(e.browse().is_none(), "an outcome is fresher than a browse");
        assert_eq!(e.voice.phase, VoicePhase::Outcome);
        assert_eq!(e.model().map(|c| c.id), Some(ContentId::EvoOutcome));
    }

    #[test]
    fn browse_step_wraps_both_directions_and_noops_without_a_session() {
        let mut e = engine();
        e.browse_step(1);
        assert!(e.browse().is_none(), "no-op without a browse session");

        e.show_works(works());
        e.browse_step(1);
        assert_eq!(
            e.browse()
                .and_then(BrowseSession::focused)
                .map(|w| w.id.as_str()),
            Some("9")
        );
        e.browse_step(1);
        assert_eq!(
            e.browse()
                .and_then(BrowseSession::focused)
                .map(|w| w.id.as_str()),
            Some("7"),
            "wraps forward"
        );
        e.browse_step(-1);
        assert_eq!(
            e.browse()
                .and_then(BrowseSession::focused)
                .map(|w| w.id.as_str()),
            Some("9")
        );
        e.browse_step(-1);
        assert_eq!(
            e.browse()
                .and_then(BrowseSession::focused)
                .map(|w| w.id.as_str()),
            Some("7")
        );
        e.browse_step(-1);
        assert_eq!(
            e.browse()
                .and_then(BrowseSession::focused)
                .map(|w| w.id.as_str()),
            Some("9"),
            "wraps backward"
        );
    }

    #[test]
    fn browse_step_refreshes_the_tenure() {
        let mut e = engine();
        e.show_works(works());
        let shown = e.model().expect("browse shows").shown_at;
        // Just inside expiry…
        e.tick(shown + Duration::from_secs(19));
        assert!(e.browse().is_some(), "inside the tenure");
        // …a step arrives, which must reset the clock.
        e.browse_step_at(1, shown + Duration::from_secs(19));
        e.tick(shown + Duration::from_secs(19) + Duration::from_secs(6));
        assert!(e.browse().is_some(), "a step keeps the browse alive");
        // And it still expires eventually.
        e.tick(shown + Duration::from_secs(19) + Duration::from_secs(21));
        assert!(e.browse().is_none());
        assert!(e.model().is_none());
    }

    #[test]
    fn browse_expiry_returns_to_the_pre_browse_state() {
        // Over the rack: the live activity underneath comes back.
        let mut e = engine();
        e.show_live(ContentId::EvoRack, priority::RACK, true);
        e.show_works(works());
        let deadline = e.deadline().expect("browse registers a deadline");
        assert_eq!(e.model().map(|c| c.id), Some(ContentId::EvoBrowse));

        e.tick(deadline);

        assert!(e.browse().is_none(), "expiry closes the browse session");
        assert_eq!(e.model().map(|c| c.id), Some(ContentId::EvoRack));
        assert_eq!(
            e.deadline(),
            None,
            "a persistent rack has no expiry to wake for"
        );

        // With nothing underneath: hidden idle, panel ordered out.
        let mut idle = engine();
        idle.show_works(works());
        idle.tick(idle.deadline().expect("browse registers a deadline"));
        assert!(idle.browse().is_none());
        assert!(idle.model().is_none());
        assert!(!idle.has_content(), "expired browse orders the panel out");
        assert_eq!(idle.deadline(), None);
        assert_eq!(idle.voice.phase, VoicePhase::Idle);
    }

    #[test]
    fn dismiss_clears_the_browse_session() {
        let mut e = engine();
        e.show_works(works());
        assert!(e.has_content());

        e.dismiss(Instant::now());

        assert!(e.browse().is_none());
        assert!(e.model().is_none());
        assert!(!e.has_content());
        assert_eq!(e.deadline(), None);
        assert_eq!(e.voice.phase, VoicePhase::Idle);
    }

    #[test]
    fn empty_works_open_nothing_and_close_an_open_browse() {
        let mut e = engine();
        e.show_works(vec![]);
        assert!(e.browse().is_none());
        assert!(!e.has_content(), "an empty browse is not content");
        assert_eq!(e.deadline(), None);

        e.show_works(works());
        assert!(e.browse().is_some());
        e.show_works(vec![]);
        assert!(
            e.browse().is_none(),
            "an empty listing dismisses the open browse"
        );
        assert!(e.model().is_none(), "and its temporary is gone with it");
        assert!(!e.has_content());
    }

    #[test]
    fn temporary_expires_and_restores_live_content() {
        let mut e = engine();

        // (the rest of this test continues below)
        e.show_live(ContentId::EvoRack, priority::RACK, true);
        e.voice_event(
            &evo_voice::VoiceEvent::WakeGesture { score: None },
            Instant::now(),
        );
        assert!(matches!(
            e.model().map(|c| c.id),
            Some(ContentId::EvoListening)
        ));
        e.tick(Instant::now() + Duration::from_secs(13));
        assert!(matches!(e.model().map(|c| c.id), Some(ContentId::EvoRack)));
    }

    #[test]
    fn surface_yields_only_when_the_probe_finds_a_foreign_owner() {
        assert_eq!(decide_surface(0), SurfaceDecision::Own);
        assert_eq!(decide_surface(1), SurfaceDecision::Float);
    }
}
