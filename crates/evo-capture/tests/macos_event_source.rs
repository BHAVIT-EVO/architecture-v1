use evo_capture::{CaptureEngine, MacOSEventSource, MacOSAdapter, MacOSSignal};
use evo_observation::observation_schema::ObservationSchema;
use evo_observation::provenance::ObservationSource;

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRunLoopRunInMode(mode: *const std::ffi::c_void, seconds: f64, return_after_source_handled: bool) -> i32;
    static kCFRunLoopDefaultMode: *const std::ffi::c_void;
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "interactive integration test; switch applications while it runs"]
fn focused_window_source_can_initialize_when_accessibility_is_available() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = MacOSAdapter::new(ObservationSource::new("macos_event_source")?);
    let schema = ObservationSchema::window_focus_gained_v1();
    let engine = Rc::new(RefCell::new(CaptureEngine::new()));
    let observations = Rc::new(RefCell::new(Vec::new()));

    let _source = match MacOSEventSource::new(
        evo_capture::desktop_shell_pid(),
        {
        let engine = Rc::clone(&engine);
        let adapter = Rc::new(adapter);
        let schema = Rc::new(schema);
        let observations = Rc::clone(&observations);

        move |signal: MacOSSignal| {
            let raw_event = match adapter.normalize(signal) {
                Ok(raw_event) => raw_event,
                Err(err) => {
                    eprintln!("normalize failed: {err}");
                    return;
                }
            };

            if let Some(raw_event) = raw_event {
                match engine.borrow_mut().ingest(raw_event, &schema) {
                    Ok(observation) => observations.borrow_mut().push(observation),
                    Err(err) => eprintln!("ingest failed: {err}"),
                }
            }
        }
    }) {
        Ok(source) => source,
        Err(evo_capture::MacOSEventSourceError::AccessibilityPermissionRequired) => {
            eprintln!("skipping interactive macOS source test: Accessibility permission is required");
            return Ok(());
        }
        Err(err) => return Err(Box::new(err)),
    };

    unsafe {
        let _ = CFRunLoopRunInMode(kCFRunLoopDefaultMode, 10.0, true);
    }

    assert!(observations.borrow().is_empty());
    Ok(())
}
