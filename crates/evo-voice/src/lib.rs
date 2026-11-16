//! evo-voice: the "Hey Evo" voice moat.
//!
//! All on-device, no network. The pipeline:
//!
//! ```text
//! mic (cpal) → wake-word spotter → utterance → transcription (whisper)
//!     → text trigger → semantic resolution (evo-retrieval) → restore
//! ```
//!
//! With the `embeddings` feature enabled, this crate provides the concrete
//! `FastEmbedEmbedder` production implementation using the all-MiniLM-L6-v2
//! model via fastembed. evo-retrieval stays pure and testable; the model lives
//! here.
//!
//! # Features
//!
//! - Default: trigger pipeline without a semantic model. Trigger requests fail
//!   honestly when no embedder is available.
//! - `embeddings`: local FastEmbed-backed semantic matching. This is opt-in and
//!   never downloads model assets at runtime.
//! - `audio`: mic capture ([`Mic`]) + transcription ([`Transcriber`]).
//!   Requires hardware; build with `--features audio`.
//!
//! # No hardcoding
//!
//! Nothing here names an application, domain, or command phrase. The embedder
//! is a general language model; what a trigger matches is learned entirely
//! from the person's own witnessed material.

#[cfg(feature = "audio")]
mod audio;
mod driver;
#[cfg(feature = "embeddings")]
mod embed;
mod narrate;
mod restore;
#[cfg(feature = "audio")]
mod transcribe;
mod trigger;
mod wake;
#[cfg(feature = "wake-model")]
mod wake_model;
mod witness;

#[cfg(feature = "audio")]
pub use audio::Mic;
pub use driver::{
    AcousticWake, TriggerTransport, UtteranceTranscriber, VoiceDriver, VoiceEvent,
    VoiceEventObserver, VoiceLoopConfig,
};
#[cfg(feature = "embeddings")]
pub use embed::{FastEmbedEmbedder, EMBEDDING_DIM};
pub use narrate::{narrate_brief, narrate_briefs, narrate_status, NameLookup};
pub use restore::{
    restore_engagement, restore_engagement_staged, targets_from_engagement, trigger_restore,
    TriggerRestoreOutcome, VoiceCandidate, VoiceOutcome, VoiceSelection,
};
#[cfg(feature = "audio")]
pub use transcribe::{Transcriber, SAMPLE_RATE};
pub use trigger::{resolve_trigger, TriggerOutcome};
pub use wake::{detect_wake, sanitize_transcript, WakeResult};
#[cfg(feature = "wake-model")]
pub use wake_model::{
    CustomWakeSpotter, DEFAULT_WAKE_THRESHOLD, MODEL_FRAME_CHUNK, MODEL_INPUT_FRAMES,
};
pub use witness::{RestoredSubjects, RESTORED_SUBJECT_TTL};
