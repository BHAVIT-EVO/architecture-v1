//! `evo-agent` — the headless, always-on Evo voice agent.
//!
//! "Hey Evo" works whether or not the Evo window is open: this binary
//! owns the same pieces `evo-desktop` runs behind its window — the mic
//! pipeline (wake spotter + whisper), the notch surface, and the
//! daemon (attached through the same single-instance edge the desktop
//! shell uses) — but renders no window at all. The notch IS the UI:
//! wake → expand → live transcript → outcome card or the swipeable
//! candidate carousel → tap Restore → the daemon restores the work.
//!
//! Everything is reused from the desktop crate's own modules; no
//! voice logic is duplicated here. Installed as a login LaunchAgent by
//! the sibling `evo-agent-install` binary.
//!
//! ```text
//! Mic → VoiceDriver (evo-voice) ─┬─ observer fan-out → notch surface
//!                                └─ events → this loop (logging)
//! Notch chips/drag → SurfaceAction → CHOOSE over the daemon socket →
//!     outcome fed back into the notch as a Voice event.
//! ```

use std::sync::mpsc;
use std::time::{Duration, Instant};

#[cfg(not(target_os = "macos"))]
fn main() {
    if let Err(err) = run() {
        eprintln!("EVO-AGENT: {err}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "macos")]
fn main() {
    if let Err(err) = run_macos() {
        eprintln!("EVO-AGENT: {err}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "macos")]
fn run_macos() -> Result<(), String> {
    // AppKit owns the process' main thread. Keep an autorelease pool alive
    // around the event loop, rather than putting NSApplication on the
    // supervisor thread where the notch's main-queue hops cannot be serviced.
    let _pool = unsafe { appkit::AutoreleasePool::new()? };
    let application = unsafe { appkit::shared_application()? };
    unsafe { appkit::set_accessory_activation_policy(application)? };

    let (error_tx, error_rx) = mpsc::channel::<String>();
    let _supervisor = std::thread::Builder::new()
        .name("evo-agent-supervisor".to_string())
        .spawn(move || {
            let message = match std::panic::catch_unwind(run) {
                Ok(Ok(())) => "supervisor exited unexpectedly".to_string(),
                Ok(Err(err)) => format!("supervisor failed: {err}"),
                Err(_) => "supervisor panicked".to_string(),
            };
            if error_tx.send(message).is_err() {
                eprintln!("EVO-AGENT: could not report supervisor termination");
            }
            // The event loop is the main thread's owner. Let it unwind after
            // the supervisor has reported its error instead of leaving a
            // failed agent looking alive forever.
            appkit::terminate_async();
        })
        .map_err(|err| format!("could not start supervisor thread: {err}"))?;

    unsafe { appkit::run(application) };

    match error_rx.try_recv() {
        Ok(message) => Err(message),
        Err(mpsc::TryRecvError::Empty) => {
            Err("NSApplication event loop exited unexpectedly".to_string())
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            Err("supervisor exited without reporting an error".to_string())
        }
    }
}

#[cfg(target_os = "macos")]
mod appkit {
    use std::os::raw::{c_char, c_void};
    use std::ptr;

    type Id = *mut c_void;
    type Sel = *mut c_void;
    type Bool = i8;

    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn objc_msgSend();
        fn sel_registerName(name: *const c_char) -> Sel;

        // `dispatch_get_main_queue()` is a C macro. The singleton is the same
        // public libdispatch entry point used by notch_window::objc::on_main.
        static _dispatch_main_q: c_void;
        fn dispatch_async_f(
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
    }

    pub struct AutoreleasePool(Id);

    impl AutoreleasePool {
        pub unsafe fn new() -> Result<Self, String> {
            let allocated: Id =
                unsafe { msg_send0(class(b"NSAutoreleasePool\0"), selector(b"alloc\0")) };
            if allocated.is_null() {
                return Err("NSAutoreleasePool allocation failed".to_string());
            }
            let pool: Id = unsafe { msg_send0(allocated, selector(b"init\0")) };
            if pool.is_null() {
                return Err("NSAutoreleasePool initialization failed".to_string());
            }
            Ok(Self(pool))
        }
    }

    impl Drop for AutoreleasePool {
        fn drop(&mut self) {
            unsafe {
                msg_send0::<()>(self.0, selector(b"drain\0"));
            }
        }
    }

    pub unsafe fn shared_application() -> Result<Id, String> {
        let application: Id =
            unsafe { msg_send0(class(b"NSApplication\0"), selector(b"sharedApplication\0")) };
        if application.is_null() {
            Err("NSApplication sharedApplication returned nil".to_string())
        } else {
            Ok(application)
        }
    }

    pub unsafe fn set_accessory_activation_policy(application: Id) -> Result<(), String> {
        // NSApplicationActivationPolicyAccessory = 1. This keeps the agent
        // present for AppKit without putting an icon in the Dock.
        let accepted = unsafe {
            msg_send1::<Bool, isize>(application, selector(b"setActivationPolicy:\0"), 1)
        };
        if accepted == 0 {
            Err("NSApplication could not adopt accessory activation policy".to_string())
        } else {
            Ok(())
        }
    }

    pub unsafe fn run(application: Id) {
        unsafe { msg_send0::<()>(application, selector(b"run\0")) };
    }

    pub fn terminate_async() {
        unsafe {
            let queue = &_dispatch_main_q as *const c_void;
            dispatch_async_f(queue, ptr::null_mut(), terminate_application);
        }
    }

    extern "C" fn terminate_application(_: *mut c_void) {
        unsafe {
            let application: Id =
                msg_send0(class(b"NSApplication\0"), selector(b"sharedApplication\0"));
            if !application.is_null() {
                msg_send1::<(), Id>(application, selector(b"terminate:\0"), ptr::null_mut());
            }
        }
    }

    unsafe fn msg_send0<R>(receiver: Id, selector: Sel) -> R {
        let function: extern "C" fn(Id, Sel) -> R =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        function(receiver, selector)
    }

    unsafe fn msg_send1<R, A>(receiver: Id, selector: Sel, argument: A) -> R {
        let function: extern "C" fn(Id, Sel, A) -> R =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        function(receiver, selector, argument)
    }

    unsafe fn class(name: &'static [u8]) -> Id {
        unsafe { objc_getClass(name.as_ptr() as *const c_char) }
    }

    unsafe fn selector(name: &'static [u8]) -> Sel {
        unsafe { sel_registerName(name.as_ptr() as *const c_char) }
    }
}

fn works_to_browse_entries(
    entries: Vec<evo_desktop::voice::WorkEntry>,
) -> Vec<evo_desktop::notch::BrowseEntry> {
    entries
        .into_iter()
        .map(|entry| {
            let evo_desktop::voice::WorkEntry {
                id,
                status,
                episodes,
                title,
                resume_reason,
            } = entry;
            let title = if title.is_empty() {
                resume_reason.clone()
            } else {
                title
            };
            evo_desktop::notch::BrowseEntry {
                id,
                title,
                subtitle: format!("{status} · {episodes} sittings · {resume_reason}"),
            }
        })
        .collect()
}

/// Open a selected work through the daemon, then gather the windows that
/// appeared and place them into one native tiled workspace. The daemon keeps
/// ownership of restore semantics; this is only the post-open desktop layout.
fn restore_and_arrange(
    open: impl FnOnce() -> Result<evo_voice::VoiceOutcome, String>,
) -> evo_voice::VoiceOutcome {
    let mut surface = evo_desktop::pod_surface::MacPodSurface::new();
    let before = evo_desktop::unified_workspace::snapshot(&mut surface, std::process::id() as i32);
    let outcome = match open() {
        Ok(outcome) => outcome,
        Err(reason) => return evo_voice::VoiceOutcome::Failed { reason },
    };
    // App launches and browser tabs are asynchronous. Give the OS a short
    // settling window before the accessibility inventory is tiled.
    std::thread::sleep(std::time::Duration::from_millis(700));
    let _ = evo_desktop::unified_workspace::arrange_new_windows(
        &mut surface,
        &before,
        std::process::id() as i32,
    );
    outcome
}

fn run() -> Result<(), String> {
    let storage_root =
        evo_storage::prepare_storage_root().map_err(|e| format!("storage unavailable: {e}"))?;

    let daemon_path = evo_desktop::daemon::daemon_binary_path().ok_or_else(|| {
        "daemon worker missing beside evo-agent; reinstall the agent with evo-agent-install"
            .to_string()
    })?;

    // The daemon: the same single-instance edge the desktop shell uses.
    // A healthy daemon (desktop-owned or orphaned-but-alive) is attached,
    // never replaced; a stale one is replaced. Its child (if we spawned
    // it) dies with us via DaemonManager's Drop.
    let mut daemon = evo_desktop::daemon::DaemonManager::new(storage_root.clone());
    match daemon.poll() {
        evo_desktop::daemon::DaemonStatus::Unavailable
        | evo_desktop::daemon::DaemonStatus::Failed(_) => {
            eprintln!(
                "EVO-AGENT: daemon is not healthy yet (worker {}); supervision will retry",
                daemon_path.display()
            );
        }
        _ => {}
    }

    // The notch is an optional presentation surface. The background agent
    // must still own the microphone and daemon when the machine has no notch,
    // the surface is disabled, or AppKit cannot create it in this session.
    // Voice is the product path; the notch is only its observer.
    let notch = evo_desktop::notch_window::spawn();
    if notch.is_none() {
        eprintln!(
            "EVO-AGENT: notch surface unavailable; continuing headless so voice works with the app closed"
        );
    }

    // The voice thread: mic → wake → transcribe → daemon socket. The
    // notch already rides the driver's observer fan-out inside
    // spawn_voice_thread; this channel keeps the event stream alive and
    // lets the agent log honest outcomes.
    let (tx, rx) = mpsc::channel();
    let mut voice = evo_desktop::voice::spawn_voice_thread(
        storage_root.clone(),
        tx.clone(),
        notch.as_ref().map(|surface| surface.input.clone()),
    );
    let mut retry_voice_at = Instant::now();
    let mut retry_daemon_at = Instant::now();

    eprintln!(
        "EVO-AGENT: running — say \"hey evo\". Notch decision: {:?}. Ctrl-C to quit.",
        notch.as_ref().map(|surface| surface.decision)
    );

    let client = evo_desktop::voice::VoiceSocketClient::new(storage_root.clone());
    let mut last_was_browse = false;
    loop {
        // Drain voice events so the channel never backs up (and log the
        // honest outcomes; the notch shows them too).
        while let Ok(event) = rx.try_recv() {
            match &event {
                evo_voice::VoiceEvent::Outcome(outcome) => {
                    if matches!(outcome, evo_voice::VoiceOutcome::NoCommand)
                        && !last_was_browse
                        && let Some(notch) = notch.as_ref()
                    {
                        match client.works() {
                            Ok(entries) if !entries.is_empty() => {
                                let browse = works_to_browse_entries(entries);
                                eprintln!(
                                    "EVO-AGENT: no command → browsing {} works",
                                    browse.len()
                                );
                                let _ = notch
                                    .input
                                    .send(evo_desktop::notch_window::SurfaceInput::Works(browse));
                                last_was_browse = true;
                            }
                            Ok(_) => {}
                            Err(reason) => {
                                eprintln!("EVO-AGENT: works listing failed: {reason}");
                            }
                        }
                    }
                    eprintln!("EVO-AGENT: outcome {outcome:?}");
                }
                evo_voice::VoiceEvent::HeardCommand { command } => {
                    eprintln!("EVO-AGENT: heard {command:?}");
                }
                _ => {}
            }
        }
        // The notch's chips and drags: Restore → CHOOSE over the daemon
        // socket; the honest outcome is fed back so the notch shows it.
        if let Some(notch) = notch.as_ref() {
            match notch.actions.try_recv() {
                Ok(evo_desktop::notch_window::SurfaceAction::Choose(work_id)) => {
                    let outcome = restore_and_arrange(|| client.choose(&work_id));
                    eprintln!("EVO-AGENT: chose {work_id} → {outcome:?}");
                    let _ = notch
                        .input
                        .send(evo_desktop::notch_window::SurfaceInput::Voice(
                            evo_voice::VoiceEvent::Outcome(outcome),
                        ));
                    last_was_browse = false;
                }
                Ok(evo_desktop::notch_window::SurfaceAction::ChooseCandidate(candidate)) => {
                    let outcome = restore_and_arrange(|| client.choose_candidate(&candidate));
                    eprintln!(
                        "EVO-AGENT: chose candidate {} → {outcome:?}",
                        candidate.title
                    );
                    let _ = notch
                        .input
                        .send(evo_desktop::notch_window::SurfaceInput::Voice(
                            evo_voice::VoiceEvent::Outcome(outcome),
                        ));
                    last_was_browse = false;
                }
                Ok(evo_desktop::notch_window::SurfaceAction::Browse) => match client.works() {
                    Ok(entries) if !entries.is_empty() => {
                        let browse = works_to_browse_entries(entries);
                        eprintln!("EVO-AGENT: browse → {} works", browse.len());
                        let _ = notch
                            .input
                            .send(evo_desktop::notch_window::SurfaceInput::Works(browse));
                        last_was_browse = true;
                    }
                    Ok(_) => {
                        let outcome = evo_voice::VoiceOutcome::Brief {
                            count: 0,
                            narration:
                                "No work witnessed yet — start something and I'll keep track."
                                    .into(),
                        };
                        eprintln!("EVO-AGENT: browse → {outcome:?}");
                        let _ = notch
                            .input
                            .send(evo_desktop::notch_window::SurfaceInput::Voice(
                                evo_voice::VoiceEvent::Outcome(outcome),
                            ));
                        last_was_browse = false;
                    }
                    Err(reason) => {
                        let outcome = evo_voice::VoiceOutcome::Failed { reason };
                        eprintln!("EVO-AGENT: browse → {outcome:?}");
                        let _ = notch
                            .input
                            .send(evo_desktop::notch_window::SurfaceInput::Voice(
                                evo_voice::VoiceEvent::Outcome(outcome),
                            ));
                        last_was_browse = false;
                    }
                },
                Ok(evo_desktop::notch_window::SurfaceAction::ContinueRecent) => {
                    match client.works() {
                        Ok(entries) if !entries.is_empty() => {
                            let work_id = entries[0].id.clone();
                            let outcome = restore_and_arrange(|| client.choose(&work_id));
                            eprintln!("EVO-AGENT: continued {work_id} → {outcome:?}");
                            let _ =
                                notch
                                    .input
                                    .send(evo_desktop::notch_window::SurfaceInput::Voice(
                                        evo_voice::VoiceEvent::Outcome(outcome),
                                    ));
                            last_was_browse = false;
                        }
                        Ok(_) => {
                            let outcome = evo_voice::VoiceOutcome::Brief {
                                count: 0,
                                narration:
                                    "No work witnessed yet — start something and I'll keep track."
                                        .into(),
                            };
                            eprintln!("EVO-AGENT: continue-recent → {outcome:?}");
                            let _ =
                                notch
                                    .input
                                    .send(evo_desktop::notch_window::SurfaceInput::Voice(
                                        evo_voice::VoiceEvent::Outcome(outcome),
                                    ));
                            last_was_browse = false;
                        }
                        Err(reason) => {
                            let outcome = evo_voice::VoiceOutcome::Failed { reason };
                            eprintln!("EVO-AGENT: continue-recent → {outcome:?}");
                            let _ =
                                notch
                                    .input
                                    .send(evo_desktop::notch_window::SurfaceInput::Voice(
                                        evo_voice::VoiceEvent::Outcome(outcome),
                                    ));
                            last_was_browse = false;
                        }
                    }
                }
                Ok(evo_desktop::notch_window::SurfaceAction::OpenMain) => {
                    // Headless: there is no window to open. The restore is
                    // the product; the rack lives wherever the person runs
                    // evo-desktop.
                    eprintln!(
                        "EVO-AGENT: open-rack requested (agent mode has no window; launch evo-desktop)"
                    );
                }
                // Dismiss and carousel steps: the surface handles its own
                // state; nothing for the agent to do.
                Ok(
                    evo_desktop::notch_window::SurfaceAction::Dismiss
                    | evo_desktop::notch_window::SurfaceAction::ChoiceStep(_),
                ) => {}
                Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => {}
            }
        }
        // Heartbeat: re-check the daemon child; the loop stays cheap so
        // chip clicks feel immediate. A dead child is restarted here rather
        // than leaving the voice thread connected to a permanently refused
        // socket. Permission failures are reported but are not blindly
        // restarted: granting Accessibility is a user action.
        let daemon_status = daemon.poll();
        if matches!(
            daemon_status,
            evo_desktop::daemon::DaemonStatus::Unavailable
                | evo_desktop::daemon::DaemonStatus::Failed(_)
        ) && retry_daemon_at <= Instant::now()
        {
            eprintln!("EVO-AGENT: recovering daemon after {:?}", daemon_status);
            daemon.restart();
            retry_daemon_at = Instant::now() + Duration::from_secs(3);
        }

        // The voice thread exits cleanly when model loading, microphone access,
        // or a driver dependency is unavailable. Reap it and retry with a
        // small backoff so a permission grant or transient device failure can
        // recover without a tight restart storm.
        if voice.is_finished() && retry_voice_at <= Instant::now() {
            if voice.join().is_err() {
                eprintln!("EVO-AGENT: voice thread panicked; restarting");
            } else {
                eprintln!("EVO-AGENT: voice thread exited; restarting");
            }
            voice = evo_desktop::voice::spawn_voice_thread(
                storage_root.clone(),
                tx.clone(),
                notch.as_ref().map(|surface| surface.input.clone()),
            );
            retry_voice_at = Instant::now() + Duration::from_secs(3);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
