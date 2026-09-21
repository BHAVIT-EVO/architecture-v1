//! evo-retrieval — resolving a trigger phrase to a body of work.
//!
//! # What this crate does
//!
//! Given a trigger — a phrase a person typed or spoke to get back to something —
//! and a [`Reconstruction`](evo_engagement::Reconstruction) of their history,
//! retrieval decides which reconstructed thread the phrase means. It scores the
//! trigger against every thread's own vocabulary, weighting distinctive words
//! (by inverse document frequency across threads) and members that genuinely
//! belong to the thread (by specificity), and returns one of three honest
//! answers.
//!
//! # Public surface
//!
//! - [`Retrieval`] — the boundary value; holds no state.
//! - [`Resolution`] — [`Resolved`](Resolution::Resolved),
//!   [`Ambiguous`](Resolution::Ambiguous), or [`NotFound`](Resolution::NotFound).
//!
//! # Contract
//!
//! - Score a trigger against *every* reconstructed thread, not only the bodies
//!   of work Home presents — so a resource witnessed once and never returned to
//!   is still retrievable by name.
//! - Resolve to a single thread only when it clearly outscores the runner-up
//!   (by [`EngagementParams::retrieval_margin`](evo_engagement::EngagementParams::retrieval_margin));
//!   otherwise return the contenders as `Ambiguous` and let the caller ask which.
//! - Be a pure, deterministic function of the trigger and the reconstruction:
//!   no clock, no randomness, identical on every replay.
//!
//! # Relationship to RFC-0007
//!
//! RFC-0007 fixed the boundary but deliberately left the algorithm open. This
//! crate implements the algorithm WORK-MODEL §3 specifies. It resolves to a
//! *thread* rather than to a `WorkspaceId`, because the answer to "the thing I
//! read once last Tuesday" is a remembered thread that was never promoted to a
//! Workspace; mapping a resolved thread to a committed Workspace, when one
//! exists, is the daemon's job.
//!
//! # No hardcoded categories
//!
//! Nothing here names an application, domain, file type, profession, or category
//! of work. What a trigger matches is learned entirely from the tokens of the
//! person's own witnessed subjects and the specificity the reconstruction
//! measured. See [`retrieval`] for the scoring.

mod retrieval;

pub use retrieval::{Resolution, Retrieval, RetrievalIndex, WorkResolution};
