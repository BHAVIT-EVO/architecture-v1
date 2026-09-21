//! Drives the real daemon vertical pipeline with fabricated canonical
//! WindowFocusGained observations, against an isolated scratch storage root.
//!
//! This is developer/verification tooling, not production code. It exercises
//! the exact production path the daemon uses once a macOS Accessibility event
//! has been captured: MacOSAdapter → CaptureEngine → Observation acceptance →
//! VerticalPipeline (Observation persistence → Artifact acceptance → derived
//! canonical index → Engagement inference → Workspace projection → Restoration
//! Derivation).
//!
//! The only stage not exercised is the raw macOS Accessibility signal source,
//! which requires TCC Accessibility permission (see the daemon's honest
//! `AccessibilityPermissionRequired` error when it is not granted).
//!
//! **The Observations below are invented.** They are not distinguishable from
//! real ones once persisted — the log format has nowhere to record that a
//! record was fabricated, deliberately — so this example refuses to start
//! unless `EVO_STORAGE_ROOT` points somewhere other than the canonical root.
//! Earlier revisions defaulted to the canonical root and wrote two invented
//! window titles into a real person's history, where they surfaced on Home as
//! work that had never happened.
//!
//! Usage:
//!   EVO_STORAGE_ROOT=$(mktemp -d) cargo run -p evo-daemon --example drive_pipeline

use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
use evo_daemon::runtime::{RestorationInputBoundary, VerticalPipeline};
use evo_observation::provenance::ObservationSource;

use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};

fn main() {
    // Refused at the storage boundary, not filtered afterwards: fabricated
    // Observations must never reach the root that holds real history.
    let storage_root = match evo_storage::fabrication_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    println!("storage root: {}", storage_root.display());

    let source = ObservationSource::new("macos_accessibility")
        .expect("hardcoded source name is non-empty");
    let adapter = MacOSAdapter::new(source);
    let mut engine = CaptureEngine::new();

    let (sender, receiver): (
        std::sync::mpsc::Sender<Result<RestorationInputBoundary, evo_daemon::DaemonError>>,
        std::sync::mpsc::Receiver<Result<RestorationInputBoundary, evo_daemon::DaemonError>>,
    ) = channel();
    let mut pipeline =
        VerticalPipeline::new(sender, storage_root.clone()).expect("pipeline builds");

    println!(
        "bodies of work derived at start: {}",
        pipeline.index().workspaces().len()
    );

    // Fabricated WindowFocusGained observations: the same window twice, then a
    // second window twice. Named so that a human reading a log can tell at a
    // glance that no person produced them.
    let subjects = [
        "EVO FIXTURE — fabricated window A",
        "EVO FIXTURE — fabricated window A",
        "EVO FIXTURE — fabricated window B",
        "EVO FIXTURE — fabricated window B",
    ];

    for (index, subject) in subjects.iter().enumerate() {
        let observed_at = SystemTime::now()
            .checked_sub(Duration::from_secs(300 - index as u64 * 7))
            .expect("representable moment");
        let signal = MacOSSignal::WindowFocusGained {
            subject: subject.to_string(),
            observed_at,
            process_identifier: None,
            owning_process_name: None,
            observed_state: None,
        };
        let raw = adapter
            .normalize(signal)
            .expect("normalization is infallible")
            .expect("WindowFocusGained normalizes to a RawEvent");
        let schema = raw.schema();
        let observation = engine
            .ingest(raw, &schema)
            .expect("observation acceptance succeeds");
        // The pipeline itself durably persists the accepted Observation
        // (IS-0001 R-7) as its first stage.
        pipeline.handle_observation(observation);
        println!("obs {} subject={:?} → accepted", index + 1, subject);
    }

    // Witnessed evidence does not announce itself act by act: a body of work is
    // a claim about a whole history, so it can only be derived once the
    // evidence has stopped arriving. This is the same call the daemon's worker
    // makes on an idle tick.
    pipeline.settle();

    let mut announced = 0usize;
    while let Ok(message) = receiver.try_recv() {
        let boundary = match message {
            Ok(boundary) => boundary,
            Err(err) => panic!("pipeline boundary failed: {err}"),
        };
        announced += 1;
        let outcome = boundary.outcome();
        let resume = outcome
            .resume_point()
            .map(|point| point.artifact_id().as_str().to_string())
            .unwrap_or_else(|| "<insufficient>".to_string());
        println!(
            "announced workspace={} snapshots={} artifacts={} resume_point={}",
            boundary.workspace().id(),
            boundary.workspace().snapshots().len(),
            boundary.workspace().attachments().len(),
            resume,
        );
    }

    if announced == 0 {
        // The honest and correct outcome for this input. Four focus sightings
        // seconds apart are someone passing through two windows; nothing here
        // is a body of work anyone could resume, and Evo says so rather than
        // presenting two windows as two Workspaces.
        println!(
            "no body of work derived: {} witnessed sightings, none amounting to work \
             someone could resume",
            subjects.len()
        );
    }

    println!(
        "done. {} now contains observation and artifact logs built from fabricated input.",
        storage_root.display()
    );
}
