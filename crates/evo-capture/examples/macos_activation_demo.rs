
#[cfg(target_os = "macos")]
mod platform_ref {
    #![allow(unused_imports, dead_code)]
    use evo_capture::{CaptureEngine, MacOSEventSource, MacOSAdapter, MacOSSignal};
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::ObservationSource;

    use std::cell::RefCell;
    use std::rc::Rc;
}

#[cfg(target_os = "macos")]
use platform_ref::*;

#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn CFRunLoopRun();
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = MacOSAdapter::new(ObservationSource::new("macos_event_source")?);
    let schema = ObservationSchema::window_focus_gained_v1();
    let engine = Rc::new(RefCell::new(CaptureEngine::new()));
    let adapter = Rc::new(adapter);
    let schema = Rc::new(schema);

    let _source = MacOSEventSource::new(
        evo_capture::desktop_shell_pid(),
        {
        let engine = Rc::clone(&engine);
        let adapter = Rc::clone(&adapter);
        let schema = Rc::clone(&schema);

        move |signal: MacOSSignal| {
            let raw_event = match adapter.normalize(signal) {
                Ok(raw_event) => raw_event,
                Err(err) => {
                    eprintln!("failed to normalize macOS signal: {err}");
                    return;
                }
            };

            if let Some(raw_event) = raw_event {
                match engine.borrow_mut().ingest(raw_event, &schema) {
                    Ok(observation) => {
                        println!(
                            "observation captured: {} @ {:?}",
                            observation.id(),
                            observation.provenance().observed_at()
                        );
                    }
                    Err(err) => {
                        eprintln!("observation acceptance failed: {err}");
                    }
                }
            }
        }
    })?;

    println!("macOS activation source running. Switch applications to produce observations.");
    println!("Press Ctrl-C to stop.");

    #[cfg(target_os = "macos")]
    unsafe {
        CFRunLoopRun();
    }

    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("macOS demo is only available on macOS.");
    }

    Ok(())
}

/// Off-macOS this target does not exist: it exercises macOS APIs directly.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("macos_activation_demo is a macOS diagnostic target; nothing to do on this platform.");
}
