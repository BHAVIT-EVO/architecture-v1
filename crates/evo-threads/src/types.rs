//! Public contract types for the work-thread engine.
//!
//! Product statement: the engine sees *shadows*, never content. These types are
//! the complete inventory of what a shadow may carry: who caused it, what kind
//! of act it was, what stable thing it touched, when, and at most a content-free
//! magnitude (a character count, a dwell duration). Any signal beyond this list
//! is someone else's product.

use serde::{Deserialize, Serialize};

/// Identity of a body of work. Issued exactly once per birth event; never
/// reused, never re-assigned. History may re-interpret a resource, but a
/// `ThreadId` is a promise that survives every future event.
pub type ThreadId = u64;

/// Identity of an episode: a bounded interval of attention. Internal ordering
/// only; not part of the product surface except through `trace()`.
pub type EpisodeId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    /// The person did something.
    Person,
    /// The OS or a background service did something (a disk write, a daemon).
    System,
    /// This product itself acted on the person's behalf (e.g. a restore).
    Execution,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Interaction {
    /// A window or surface received focus. `weight` may carry dwell in ms.
    Focused,
    /// A URL was navigated to / a page was viewed. No commitment implied.
    Visited,
    /// Person-origin editing/typing. `weight` is a content-free count.
    Typed,
    /// A save / commit / submission: a mutation was finalized.
    Saved,
    /// A resource changed state (disk writes). Origin plus corroboration decide
    /// whether this is the person's production or a background service's churn.
    Mutated,
    /// A file appeared via a person-initiated download.
    Downloaded,
    /// A command was executed.
    CommandRan,
    /// The person said something explicitly. `detail` carries the declaration.
    Declared,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Milliseconds since epoch. The engine never reads a wall clock; the last
    /// event's timestamp *is* the present. Out-of-order events are clamped into
    /// monotonicity rather than trusted.
    pub timestamp: u64,
    pub origin: Origin,
    pub interaction: Interaction,
    /// Globally stable identity: a file path, a URL, or a window key. The
    /// engine treats it as an opaque string and knows nothing about what kinds
    /// of strings tend to mean what.
    pub resource: String,
    pub weight: Option<u64>,
    pub detail: Option<Declaration>,
}

/// The supreme court of attribution. Sparse, rare, and never overridden by
/// inference. Declarations are the only path by which two established threads
/// may ever merge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Declaration {
    /// "These belong together." If each side lives in a different thread,
    /// those threads merge. This is the sole merge path in the system.
    SameWork { a: String, b: String },
    /// "These are separate." A standing constraint: no future assignment may
    /// place the two sides into one thread based on inference.
    SeparateWork { a: String, b: String },
    /// "This is not my work." Permanent quarantine: the resource is stripped
    /// from every view and never becomes evidence again.
    NotMine { resource: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Active,
    /// Resting, not dead. Dormancy shapes the listing; it never erases identity
    /// and never blocks resumption.
    Dormant,
}

/// Two classes of identity-carrying resources. Mutation outranks Recurrence
/// wherever they conflict: what you *made* says more about your work than what
/// you kept coming back to look at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnchorKind {
    /// Weaker: re-attended across several distinct sessions with meaningful
    /// attention, and never merely ubiquitous. Exists because real work often
    /// lives in web tools that emit no mutation events at all.
    Recurrence,
    /// Strong: the resource received person production (typed, saved,
    /// downloaded, command-backed mutation, or a declaration).
    Mutation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub resource: String,
    pub kind: AnchorKind,
    pub strength: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpisodeView {
    pub span: (u64, u64),
    /// Every attended resource with discounted attention in ms; anchors are
    /// participants too. Sorted: attention desc, then resource asc.
    pub participants: Vec<(String, f64)>,
    pub anchors: Vec<String>,
    /// Absorbed detours, kept as honest context. They cast no votes anywhere.
    pub interruptions: Vec<(u64, u64)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadView {
    pub id: ThreadId,
    pub status: Status,
    pub episodes: Vec<EpisodeView>,
    /// Identity carriers, strongest first (Mutation before Recurrence, then by
    /// strength, then by name for a stable order).
    pub anchors: Vec<Anchor>,
    /// Recurring non-anchor participants with decayed weight; the thread's
    /// memory of its context. Anchors never decay; companions do.
    pub companions: Vec<(String, f64)>,
    /// Anchors shared with other live threads. Shared identity is displayed,
    /// never hidden — and never allowed to decide anything.
    pub contested: Vec<String>,
    /// Unresolved attribution ambiguity: the candidate threads of a forked
    /// episode. The engine retains alternatives; it does not guess.
    pub fork: Option<Vec<ThreadId>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResumeBundle {
    pub resume_point: String,
    /// One honest sentence. When no state-based rule fires, this is
    /// "Last seen: X" — observed, never invented.
    pub resume_reason: String,
    /// The neighborhood, not the museum: <= restore_cap, from the thread's
    /// last episode only.
    pub restore_set: Vec<String>,
    /// Observable state deltas only; each one was witnessed by an event.
    pub open_deltas: Vec<Delta>,
}

/// Open loops derivable from the event inventory alone. The engine never
/// claims a delta it did not witness, and knows nothing about what the delta
/// "means" — an unfinished mutation surface is unfinished, whatever tool made
/// it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Delta {
    /// Typed after the last save on a resource that has been saved before.
    UnsavedEdits { resource: String },
    /// Finalized-looking state changed after its last finalization
    /// (churn-passing mutation with no subsequent save).
    UncommittedChanges { root: String },
    /// Downloaded and never attended since.
    DownloadedNotYetUsed { path: String },
    /// Typed on and never saved at all: a draft-shaped surface.
    UnfinishedDraftSurface { resource: String },
}
