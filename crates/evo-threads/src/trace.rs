//! The decision log: every attribution the engine makes, with the evidence
//! that carried it.
//!
//! Product statement: a decision the user cannot inspect is a decision the
//! user cannot trust. Every birth, assignment, fork, and promotion is recorded
//! with its witness vector, and `explain()` renders one plain sentence per
//! decision. The forensic instrument and the user-facing rationale are the
//! same artifact.

use crate::types::{AnchorKind, EpisodeId, Interaction, ThreadId};
use serde::{Deserialize, Serialize};

/// A single named piece of evidence. Witnesses are existentials — "this
/// specific thing happened" — never estimated rates, because days of data
/// cannot underwrite a statistic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Witness {
    /// The episode's anchor belongs to this thread and no other live thread.
    /// The strongest evidence the stream can offer.
    SharedUncontestedAnchor(String),
    /// The anchor exists in two or more live threads. Recorded for honesty —
    /// and structurally prevented from voting for any of them.
    SharedContestedAnchor(String),
    /// A non-anchor participant recurs in the thread's profile, weighted by
    /// inverse thread-participation: the more works a resource hangs around,
    /// the less its presence says.
    ConfigOverlap { resource: String, weight: f64 },
    /// The episode began soon after the thread's last known activity.
    SameDayResumption { gap_ms: u64 },
    /// The episode produced weight of its own that no thread has ever seen.
    NewProduction { weight: u64 },
    /// A resource re-formed across several distinct sessions.
    RecurrenceReforms { resource: String, episodes: usize },
    /// The person declared it.
    UserPin,
    /// The person forbade it.
    UserNeverLink { a: String, b: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParkReason {
    /// Attention without production, recurrence, or declaration. Retained,
    /// never minted into a work: a 47,000-second film is still not a project.
    ConsumptionOnly,
    /// Real production below the birth floor — identity must cost more than a
    /// reflex. Retained; may be born later via recurrence or a declaration.
    WeakProduction,
    /// Two or more threads tied as destinations. Alternatives retained; no
    /// guess was made.
    AmbiguousFork,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Decision {
    EpisodeStarted { episode: EpisodeId, at: u64, resource: String },
    EpisodeClosed { episode: EpisodeId, at: u64 },
    EpisodeReopened { episode: EpisodeId, at: u64, via: String },
    /// A short production-free detour kept as context; it voted nowhere.
    DipAbsorbed { episode: EpisodeId, start: u64, end: u64, resources: Vec<String> },
    /// Something was seen and deliberately not counted, with the reason.
    EvidenceDropped { resource: String, reason: String },
    /// A landing already credited once came down again; the repeat updated
    /// state but minted no new production.
    RepeatedDownload { resource: String, normalized: String, first_at: u64 },
    /// An Execution-restored surface received its first real input and flipped
    /// to person evidence from that instant only.
    Handoff { resource: String, at: u64, via: Interaction },
    ThreadBorn { thread: ThreadId, at: u64, witnesses: Vec<Witness> },
    EpisodeAssigned { thread: ThreadId, episode: EpisodeId, witnesses: Vec<Witness> },
    EpisodeParked { episode: EpisodeId, reason: ParkReason },
    Fork { episode: EpisodeId, candidates: Vec<ThreadId>, witnesses: Vec<Witness> },
    AnchorPromoted { thread: ThreadId, resource: String, kind: AnchorKind, strength: f64 },
    AnchorContested { resource: String, threads: Vec<ThreadId> },
    ThreadMerged { survivor: ThreadId, absorbed: ThreadId, declaration: String },
    DeclarationApplied { summary: String },
    ResourceQuarantined { resource: String },
}

/// Render one decision as one plain sentence. Used by `Engine::explain`.
pub fn render(d: &Decision) -> Option<String> {
    match d {
        Decision::EpisodeStarted { episode, resource, .. } => Some(format!(
            "Episode {episode} began around `{resource}`."
        )),
        Decision::EpisodeClosed { episode, .. } => Some(format!("Episode {episode} closed.")),
        Decision::EpisodeReopened { episode, via, .. } => Some(format!(
            "Episode {episode} was reopened: `{via}` returned, so the interruption never became a new work."
        )),
        Decision::DipAbsorbed { episode, .. } => Some(format!(
            "A brief detour inside episode {episode} was kept as context; it counted for nothing."
        )),
        Decision::EvidenceDropped { resource, reason } => Some(format!(
            "`{resource}` was seen but not counted: {reason}."
        )),
        Decision::RepeatedDownload { resource, normalized, .. } => Some(format!(
            "`{resource}` landed where `{normalized}` had already landed; the first landing counted, this repeat is state only."
        )),
        Decision::Handoff { resource, .. } => Some(format!(
            "`{resource}` was restored by the product and became real evidence only when you first touched it."
        )),
        Decision::ThreadBorn { thread: t, witnesses, .. } => Some(format!(
            "Work {t} was born: {}.",
            render_witnesses(witnesses)
        )),
        Decision::EpisodeAssigned { thread: t, episode, witnesses } => Some(format!(
            "Episode {episode} joined work {t}: {}.",
            render_witnesses(witnesses)
        )),
        Decision::EpisodeParked { episode, reason } => {
            let why = match reason {
                ParkReason::ConsumptionOnly => {
                    "attention without production or recurrence is not a work"
                }
                ParkReason::WeakProduction => {
                    "its production was brief and terminal; identity costs more than a reflex"
                }
                ParkReason::AmbiguousFork => "two works tied as its home and guessing is not allowed",
            };
            Some(format!("Episode {episode} was kept unattributed: {why}."))
        }
        Decision::Fork { episode, candidates, .. } => Some(format!(
            "Episode {episode} could belong to works {candidates:?}; all alternatives were retained, none chosen."
        )),
        Decision::AnchorPromoted { thread: t, resource, kind, .. } => {
            let how = match kind {
                AnchorKind::Mutation => "you produced on it",
                AnchorKind::Recurrence => "you kept returning to it across separate sessions",
            };
            Some(format!("`{resource}` became an anchor of work {t}: {how}."))
        }
        Decision::AnchorContested { resource, threads } => Some(format!(
            "`{resource}` now anchors works {threads:?}; it will decide for none of them."
        )),
        Decision::ThreadMerged { survivor, absorbed, .. } => Some(format!(
            "Work {absorbed} was merged into work {survivor} — because you said so, the only way a merge can happen."
        )),
        Decision::DeclarationApplied { summary } => Some(summary.clone()),
        Decision::ResourceQuarantined { resource } => Some(format!(
            "`{resource}` was quarantined at your word and will never be evidence again."
        )),
    }
}

fn render_witnesses(ws: &[Witness]) -> String {
    let mut parts: Vec<String> = ws.iter().map(render_witness).collect();
    parts.sort();
    parts.dedup();
    parts.join("; ")
}

fn render_witness(w: &Witness) -> String {
    match w {
        Witness::SharedUncontestedAnchor(r) => {
            format!("it shares your anchor `{r}`, which belongs to no other work")
        }
        Witness::SharedContestedAnchor(r) => {
            format!("`{r}` is shared across works and was not allowed to decide")
        }
        Witness::ConfigOverlap { resource, weight } => format!(
            "`{resource}` recurs in this work's company (weight {:.0})",
            weight
        ),
        Witness::SameDayResumption { gap_ms } => {
            format!("it resumed {} minutes later the same day", gap_ms / 60_000)
        }
        Witness::NewProduction { weight } => {
            format!("you produced {weight} units of work no thread had seen")
        }
        Witness::RecurrenceReforms { resource, episodes } => {
            format!("`{resource}` re-formed across {episodes} separate sessions")
        }
        Witness::UserPin => "you declared it belongs here".to_string(),
        Witness::UserNeverLink { a, b } => format!("you declared `{a}` and `{b}` separate"),
    }
}
