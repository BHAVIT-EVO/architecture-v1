//! Standalone input counter: runs the content-free event tap and writes
//! InputActivity observations to the canonical observation log.
//!
//! This binary runs alongside the existing daemon (which writes to SQLite
//! but has no input counter). It fills the one gap the old daemon cannot:
//! counting keystrokes and clicks per focused surface, content-free.
//!
//! Usage: `evo-input-counter` (runs until killed; requires Input Monitoring
//! permission, requested on first run).

use evo_capture::adapters::macos::{MacOSAdapter, MacOSSignal};
use evo_capture::macos_input_counter::{MacOSInputCounter, SubjectResolver};
use evo_capture::raw_event::RawEvent;
use evo_observation::accept::accept;
use evo_observation::observation::Observation;
use evo_observation::provenance::ObservationSource;
use evo_storage::{Storage, StorageObjectKind};

use std::sync::{Arc, Mutex};
use std::time::SystemTime;

fn main() {
    println!("EVO-INPUT-COUNTER starting");

    // Prompt for Accessibility if not granted: the counter needs it to
    // know which surface is focused (to attribute input buckets to).
    if !evo_capture::accessibility_permission_granted() {
        println!("EVO-INPUT-COUNTER requesting Accessibility permission...");
        evo_capture::prompt_for_accessibility_permission();
        println!("EVO-INPUT-COUNTER grant Accessibility in System Settings, then restart this binary.");
    }

    // The shared current-subject slot, updated by the AX focus source.
    let current_subject: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // Start the AX focus source to track what the person is on.
    let focus_subject = current_subject.clone();
    let focus_source = evo_capture::MacOSEventSource::new(
        evo_capture::desktop_shell_pid(),
        move |signal| {
            if let MacOSSignal::WindowFocusGained { subject, .. } = signal {
                if let Ok(mut slot) = focus_subject.lock() {
                    *slot = Some(subject);
                }
            }
        },
    );
    match focus_source {
        Ok(_source) => println!("EVO-INPUT-COUNTER focus source: active"),
        Err(evo_capture::MacOSEventSourceError::AccessibilityPermissionRequired) => {
            eprintln!("EVO-INPUT-COUNTER Accessibility permission required for focus tracking");
            eprintln!("Grant it in System Settings > Privacy & Security > Accessibility");
        }
        Err(err) => {
            eprintln!("EVO-INPUT-COUNTER focus source failed: {err}");
        }
    }

    // The storage for appending observations.
    let storage = Storage::new();

    // Start the input counter. Its flush buckets attribute to the focused
    // surface through the shared slot.
    let counter_subject: Arc<Mutex<Option<String>>> = current_subject.clone();
    let resolver: SubjectResolver = Box::new(move || {
        counter_subject.lock().ok().and_then(|slot| slot.clone())
    });

    let counter = MacOSInputCounter::start(
        resolver,
        Box::new(move |signal| {
            if let MacOSSignal::InputActivity {
                subject,
                keys,
                clicks,
                scrolls,
                observed_at,
            } = signal
            {
                // Normalize: only persist non-zero buckets (absence is the zero).
                if keys == 0 && clicks == 0 && scrolls == 0 {
                    return;
                }
                let Some(subject) = subject.filter(|s| !s.trim().is_empty()) else {
                    return;
                };

                // Project onto the engine's contract and persist.
                let concept = evo_observation::observation_language::ObservationConcept::InputActivity {
                    subject,
                    keys,
                    clicks,
                    scrolls,
                };
                let source = ObservationSource::new("macos_input_counter")
                    .expect("source name is valid");
                let raw_event = RawEvent::new(
                    source,
                    observed_at,
                    std::collections::HashMap::new(),
                    concept.clone(),
                );
                let schema = raw_event.schema();
                if let Ok(observation) = accept(raw_event.into_candidate_observation(), &schema) {
                    let encoded = encode_observation(&observation);
                    if let Err(err) = storage.append(StorageObjectKind::Observation, encoded.as_bytes()) {
                        eprintln!("EVO-INPUT-COUNTER append failed: {err}");
                    } else {
                        println!(
                            "EVO-INPUT-COUNTER bucket: keys={keys} clicks={clicks} scrolls={scrolls} subject={}",
                            observation.evidence().fact("InputActivity")
                                .map(|f| format!("{:?}", f.value()))
                                .unwrap_or_default()
                        );
                    }
                }
            }
        }),
    );

    match counter {
        Ok(_counter) => {
            println!("EVO-INPUT-COUNTER input counter: active (10-second buckets)");
            println!("EVO-INPUT-COUNTER running; Ctrl+C to stop");
            // Park on the main run loop; the counter's tap and timer fire here.
            #[cfg(target_os = "macos")]
            unsafe {
                unsafe extern "C" {
                    fn CFRunLoopRun();
                }
                CFRunLoopRun();
            }
        }
        Err(err) if err.is_permission() => {
            eprintln!("EVO-INPUT-COUNTER Input Monitoring permission required.");
            eprintln!("Grant it in System Settings > Privacy & Security > Input Monitoring");
            eprintln!("Then restart this binary.");
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("EVO-INPUT-COUNTER failed to start: {err}");
            std::process::exit(1);
        }
    }
}

/// Encodes an Observation in the observation.log's line-oriented format.
/// Must match evo-daemon's `encode_observation_record` byte-for-byte:
/// timestamps as `+{secs}:{nanos}`, facts with `fact_value_kind=` and the
/// correctly-typed value line. The input counter cannot depend on
/// evo-daemon (circular), so this is a careful mirror — the integration
/// tests catch drift.
fn encode_observation(observation: &Observation) -> String {
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
    let mut record = String::new();
    use std::fmt::Write;
    writeln!(&mut record, "observation-v1").unwrap();
    writeln!(&mut record, "id_hex={}", hex(observation.id().to_string().as_bytes())).unwrap();
    writeln!(&mut record, "schema_name_hex={}", hex(observation.schema().name().as_bytes())).unwrap();
    writeln!(&mut record, "schema_version={}", observation.schema().version()).unwrap();
    let source = observation.provenance().source().as_str();
    writeln!(&mut record, "source_hex={}", hex(source.as_bytes())).unwrap();
    // Timestamp: +{secs}:{nanos} — the canonical format the decoder's
    // split_once(':') expects.
    let observed_at = observation
        .provenance()
        .observed_at()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("+{}:{}", d.as_secs(), d.subsec_nanos()))
        .unwrap_or_else(|_| "+0:0".to_string());
    writeln!(&mut record, "observed_at={observed_at}").unwrap();

    // Context (sorted, matching the daemon's encoder).
    let context = observation.provenance().context();
    writeln!(&mut record, "context_count={}", context.len()).unwrap();
    let mut entries: Vec<_> = context.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (key, value) in entries {
        writeln!(&mut record, "context_key_hex={}", hex(key.as_bytes())).unwrap();
        writeln!(&mut record, "context_value_hex={}", hex(value.as_bytes())).unwrap();
    }

    // Facts: each with a kind discriminator and the correctly-typed value.
    let facts = observation.evidence().facts();
    writeln!(&mut record, "fact_count={}", facts.len()).unwrap();
    for fact in facts {
        writeln!(&mut record, "fact_name_hex={}", hex(fact.name().as_bytes())).unwrap();
        match fact.value() {
            evo_observation::evidence::FactValue::Text(text) => {
                writeln!(&mut record, "fact_value_kind=Text").unwrap();
                writeln!(&mut record, "fact_value_hex={}", hex(text.as_bytes())).unwrap();
            }
            evo_observation::evidence::FactValue::Integer(value) => {
                writeln!(&mut record, "fact_value_kind=Integer").unwrap();
                writeln!(&mut record, "fact_value_int={value}").unwrap();
            }
            evo_observation::evidence::FactValue::Boolean(value) => {
                writeln!(&mut record, "fact_value_kind=Boolean").unwrap();
                writeln!(&mut record, "fact_value_bool={value}").unwrap();
            }
            _ => {
                writeln!(&mut record, "fact_value_kind=Text").unwrap();
                writeln!(&mut record, "fact_value_hex=").unwrap();
            }
        }
    }
    record
}
