//! Full end-to-end verification of the Continue-My-Work loop on a real machine.
//!
//! Canonical evidence (real WindowFocusGained Observations whose subject is a
//! genuinely open window on this Mac) → real VerticalPipeline → derived body of
//! work → Restoration Derivation → the desktop's exact Resume path
//! (`state::run_execution` with the real `MacOSExecutor`).
//!
//! One run verifies both branches of the loop honestly:
//!
//! - the **positive** case — the window that really is open is raised
//!   (`Opened`);
//! - the **honest negative** — the companion resources in the same body of work
//!   do not exist on this machine, and Evo reports `Unavailable` for them
//!   rather than guessing something nearby.
//!
//! Nothing is launched: raising an already-open window is the only effect this
//! example has on the machine it runs on.
//!
//! The companion subjects are fabricated, so this example runs against an
//! isolated storage root of its own and never touches real history.
//!
//! This is developer/verification tooling, not product code.
//!
//! The window under verification is taken from the live window list unless one
//! is named on the command line, so this example names no application: which
//! window it exercises is whatever this machine actually has open (§17).
//!
//! Usage:
//!   cargo run -p evo-desktop --example verify_resume
//!   cargo run -p evo-desktop --example verify_resume "<open window title>"

use evo_capture::{CaptureEngine, MacOSAdapter, MacOSSignal};
use evo_daemon::runtime::{RestorationInputBoundary, VerticalPipeline};
use evo_desktop::state::{load_display_state, run_execution};
use evo_execution::{MacOSExecutor, WindowSource, macos::SystemWindowSource};
use evo_observation::provenance::ObservationSource;

use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};

/// A file and a page that do not exist on this machine, in a body of work with
/// the real open window. Named so that nothing could mistake them for a real
/// person's resources, and reserved by RFC 2606 so the URL resolves nowhere.
const COMPANION_FILE: &str = "/Users/evo-verify-resume/companion-notes.md";
const COMPANION_PAGE: &str = "https://example.invalid/evo-verify-resume";

/// The window title used when this machine shows none, so the rest of the loop
/// can still be verified. Plainly not an application, so a `Unavailable` result
/// for it is the expected outcome rather than a finding.
const NO_SUCH_WINDOW: &str = "Evo has no window by this name";

fn main() {
    let storage_root = std::env::temp_dir().join("evo-verify-resume");
    // The persistence boundary resolves its root from the environment; point
    // the whole verification run at the isolated canonical root. Safe here:
    // the variable is set once before any thread reads it.
    unsafe { std::env::set_var("EVO_STORAGE_ROOT", &storage_root) };
    // Naming an application here would put a real application's name in the
    // repository as a default (§17), and would also be a guess about what this
    // machine has open. Ask the machine instead.
    let (open_title, really_open) = match std::env::args().nth(1) {
        Some(named) => (named, true),
        None => match first_open_window() {
            Some(title) => (title, true),
            // The window list being empty means, on a real Mac, that this
            // process is not Accessibility-trusted — not that nothing is open.
            // Everything except the raise itself is still verifiable, so run it
            // and withhold only the claim that cannot be supported.
            None => (NO_SUCH_WINDOW.to_string(), false),
        },
    };
    if really_open {
        println!("open window under verification: {open_title:?}");
    } else {
        println!(
            "no window is visible to this process, so the positive branch \
             (raising a window that really is open) cannot be exercised here."
        );
        println!(
            "  window titles come from the Accessibility API, which returns \
             nothing to an untrusted process; grant Accessibility to the binary \
             running this example, or pass a title explicitly."
        );
        println!(
            "  the rest of the loop is still verified below — formation, \
             derivation, selective execution, and the honest negative."
        );
    }

    // Clean, isolated canonical root for this verification run.
    let _ = std::fs::remove_dir_all(&storage_root);
    std::fs::create_dir_all(&storage_root).expect("create storage root");

    // ── Real pipeline: canonical Observations → a derived body of work ──────
    let source = ObservationSource::new("macos_accessibility").unwrap();
    let adapter = MacOSAdapter::new(source);
    let mut engine = CaptureEngine::new();

    let (sender, receiver): (
        std::sync::mpsc::Sender<Result<RestorationInputBoundary, evo_daemon::DaemonError>>,
        std::sync::mpsc::Receiver<Result<RestorationInputBoundary, evo_daemon::DaemonError>>,
    ) = channel();
    let mut pipeline =
        VerticalPipeline::new(sender, storage_root.clone()).expect("pipeline builds");

    // A body of work: this window, a document, and a page, used with each other
    // across three sittings ending a few minutes ago. Two focus sightings of one
    // window would form nothing at all — correctly, since nobody could resume
    // "a window someone once looked at" — so the history has to be a history.
    let signals = worked_on(&open_title);
    let witnessed = signals.len();
    {
        let _guard = evo_storage::Storage::with_thread_root(&storage_root);
        for signal in signals {
            let raw = adapter
                .normalize(signal)
                .expect("adapter normalizes")
                .expect("canonical signal");
            // The adapter is the authority on which canonical schema a signal
            // translates to (BE-TRACE-0001 §3.1: window focus is v2, carrying
            // the owning-process pid in Provenance::context).
            let schema = raw.schema();
            let observation = engine
                .ingest(raw, &schema)
                .expect("observation acceptance succeeds");
            // The pipeline itself durably persists the accepted Observation
            // (IS-0001 R-7). The storage boundary reads the thread-local root,
            // so the pipeline runs under the isolated root guard.
            pipeline.handle_observation(observation);
            while receiver.try_recv().is_ok() {}
        }
        // A body of work is a claim about a whole history, so it is derived only
        // once the evidence has stopped arriving.
        pipeline.settle();
        while receiver.try_recv().is_ok() {}
    }
    println!("witnessed {witnessed} canonical observations across three sittings");

    // ── Desktop Resume path on the real derived state ───────────────────────
    let state = load_display_state(&storage_root).expect("display state loads");
    let workspace = state
        .display
        .as_ref()
        .expect("a body of work was derived from the real observations");
    println!(
        "body of work derived: {} ({} members)",
        workspace.id(),
        workspace.attachments().len()
    );
    let outcome = state.outcome.as_ref().expect("derived restoration outcome");
    println!(
        "resume point: {:?}",
        outcome
            .resume_point()
            .map(|point| point.artifact_id().as_str())
    );
    println!("locators for artifacts: {}", state.locators.len());

    let executor = MacOSExecutor::new();
    let report = run_execution(
        Some(outcome),
        state.selection.as_ref(),
        &state.locators,
        &executor,
    );
    println!("--- resume result ---");
    let mut opened_the_real_window = false;
    let mut guessed = Vec::new();
    for attempt in report.attempts() {
        let subject = state
            .subjects
            .get(attempt.artifact_id().as_str())
            .map(String::as_str)
            .unwrap_or("<unknown subject>");
        println!(
            "artifact={} subject={subject:?} status={:?}",
            attempt.artifact_id().as_str(),
            attempt.status()
        );
        let opened = matches!(attempt.status(), evo_execution::TargetStatus::Opened { .. });
        if subject == open_title {
            opened_the_real_window = opened;
        } else if opened {
            // A resource that does not exist on this machine must never come
            // back as opened: that would mean Evo found something *near* it.
            guessed.push(subject.to_string());
        }
    }
    assert!(
        guessed.is_empty(),
        "resources that do not exist on this machine were reported as opened: {guessed:?}"
    );
    // The positive branch is a claim about a window that really was open, so it
    // is only asserted when one really was found. Where none was, the same
    // sighting is instead evidence for the negative: a window Evo cannot see is
    // reported unavailable rather than guessed at.
    if really_open {
        assert!(
            opened_the_real_window,
            "the window {open_title:?} is open on this machine but Evo did not raise it"
        );
    } else {
        assert!(
            !opened_the_real_window,
            "Evo reported raising {open_title:?}, which is not a window on this machine"
        );
    }

    // ── Honest negative: a window that is not open is never guessed ─────────
    let missing = evo_execution::engine::PlatformExecutor::focus_window(
        &executor,
        "Evo does not own this window title",
    );
    println!("missing window -> {missing:?}");
    assert!(matches!(
        missing,
        evo_execution::TargetStatus::Unavailable { .. }
    ));

    // Cleanup the verification root.
    let _ = std::fs::remove_dir_all(&storage_root);
    println!(
        "VERIFY-RESUME-OK positive={}",
        if really_open {
            "verified"
        } else {
            "not exercised (no window visible to this process)"
        }
    );
}

/// A window this machine actually has open, if any is visible to this process.
///
/// The lowest title in sorted order, so two runs against an unchanged desktop
/// exercise the same window — the live list arrives in application-launch order,
/// which is not something a verification run should depend on.
fn first_open_window() -> Option<String> {
    let windows = SystemWindowSource.windows().ok()?;
    windows
        .into_iter()
        .map(|window| window.title)
        .filter(|title| !title.trim().is_empty())
        .min()
}

/// Three sittings of real work: the window under verification, a document saved
/// while it was open, and a page checked alongside it — ending five minutes ago,
/// so the most recent sitting is the one being returned to.
fn worked_on(open_title: &str) -> Vec<MacOSSignal> {
    const PASS: u64 = 300 + 30 + 120;
    const SITTING: u64 = 4 * PASS;
    const GAP: u64 = 3 * 60 * 60;
    let span = 3 * SITTING + 2 * GAP;

    let base = SystemTime::now()
        .checked_sub(Duration::from_secs(span + 300))
        .expect("representable moment");
    let at = |offset: u64| {
        base.checked_add(Duration::from_secs(offset))
            .expect("representable moment")
    };

    let mut signals = Vec::new();
    let mut offset = 0u64;
    for sitting in 0..3 {
        for _pass in 0..4 {
            signals.push(MacOSSignal::WindowFocusGained {
                subject: open_title.to_string(),
                observed_at: at(offset),
                process_identifier: None,
                owning_process_name: None,
                observed_state: None,
            });
            offset += 300;
            signals.push(MacOSSignal::FileSaved {
                subject: COMPANION_FILE.to_string(),
                observed_at: at(offset),
            });
            offset += 30;
            signals.push(MacOSSignal::URLNavigated {
                subject: COMPANION_PAGE.to_string(),
                observed_at: at(offset),
            });
            offset += 120;
        }
        if sitting < 2 {
            offset += GAP;
        }
    }
    signals
}
