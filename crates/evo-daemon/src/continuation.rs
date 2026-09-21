//! User continuation-surface channel (RFC-0013, OBS-CONTINUATION-SURFACE).
//!
//! The desktop shell never writes canonical state. When the user explicitly
//! declares that a set of already-witnessed subjects currently constitutes
//! the continuation of their work, the shell sends the set over a local Unix
//! socket; the daemon — the canonical runtime owner — validates that every
//! subject is witnessed in the canonical Observation log, canonicalizes the
//! set (deduplicated, ascending canonical order), forwards it into the
//! vertical runtime as a ContinuationSurface signal, and the runtime accepts,
//! persists, and re-derives. The shell only reflects what the daemon reports.
//!
//! This is the RFC-0011 designation pattern applied to a multi-resource
//! continuation declaration: the truth condition is the performance of the
//! declaration act through a trusted capture origin (`user_continuation`),
//! exactly as WorkDesignated's truth condition is the performance of the
//! designation act and WorkGrouped's is the performance of the grouping act.

use crate::errors::DaemonError;
use crate::persistence::resolve_designated_artifact;
use evo_capture::MacOSSignal;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::SystemTime;

/// The maximum accepted subject length, in bytes.
const MAX_SUBJECT_BYTES: usize = 4096;

/// The maximum accepted surface size (number of declared subjects). A surface
/// is a bounded declaration; an unbounded declaration is malformed.
const MAX_SURFACE_SUBJECTS: usize = 64;

/// The deterministic socket path for one storage root (same derivation rule
/// as the designation and grouping sockets, distinct prefix).
pub fn continuation_surface_socket_path(storage_root: &Path) -> PathBuf {
    // The socket name is a hash of the storage root. macOS paths are
    // case-insensitive ("/…/Evo/storage" and "/…/evo/storage" are the same
    // directory) but the string hash is not: the daemon (launchd-spawned,
    // lowercase env) and the desktop (bundle-configured, capitalized) must
    // land on the SAME socket, so the hash input is case-folded — the same
    // fold the filesystem applies.
    let folded = storage_root.to_string_lossy().to_lowercase();
    let hash = fnv1a64(folded.as_bytes());
    std::env::temp_dir().join(format!("evo-continuation-{hash:016x}.sock"))
}

/// FNV-1a 64-bit hash (deterministic, local-only, no dependencies).
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x00000100000001B3);
    }
    hash
}

/// The outcome of a continuation-surface request, as reported by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuationResponse {
    /// The declaration was validated and forwarded for canonical processing.
    Accepted,
    /// The declaration was rejected. The reason is canonical and honest.
    Rejected(String),
}

/// Spawns the daemon-side continuation-surface listener on the deterministic
/// socket path for one storage root.
///
/// Each accepted connection is handled in a fresh thread. The listener never
/// writes canonical state: it validates every subject against the canonical
/// log and, when valid, forwards a ContinuationSurface signal into the
/// vertical runtime's signal channel.
pub fn spawn_continuation_listener(
    storage_root: PathBuf,
    sender: Sender<MacOSSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let socket_path = continuation_surface_socket_path(&storage_root);
        let _ = std::fs::remove_file(&socket_path);
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("EVO-DAEMON continuation listener bind failed: {err}");
                return;
            }
        };
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let sender = sender.clone();
                    let root = storage_root.clone();
                    thread::spawn(move || {
                        handle_continuation_connection(stream, &root, &sender);
                    });
                }
                Err(_) => continue,
            }
        }
    })
}

/// Handles one continuation connection: read the declared set, validate every
/// subject against the canonical log, canonicalize (deduplicate, ascending
/// order), forward when valid, reply honestly.
fn handle_continuation_connection(stream: UnixStream, root: &Path, sender: &Sender<MacOSSignal>) {
    let mut stream = stream;
    let response = match read_request(&mut stream) {
        Err(reason) => ContinuationResponse::Rejected(reason),
        Ok(subjects) => match validate_surface(root, &subjects) {
            Err(reason) => ContinuationResponse::Rejected(reason),
            Ok(()) => {
                // Canonical surface rule (RFC-0013): deduplicated, ascending
                // canonical order. The lexicographically smallest subject is
                // the Required Subject.
                let mut subjects = subjects;
                subjects.sort();
                subjects.dedup();
                let signal = MacOSSignal::ContinuationSurface {
                    subjects,
                    observed_at: SystemTime::now(),
                };
                match sender.send(signal) {
                    Ok(()) => ContinuationResponse::Accepted,
                    Err(_) => ContinuationResponse::Rejected(
                        "the capture runtime is shutting down".to_string(),
                    ),
                }
            }
        },
    };
    let _ = stream.write_all(response_line(&response).as_bytes());
    let _ = stream.flush();
}

/// Reads a count-prefixed list of subjects from the connection.
fn read_request(stream: &mut UnixStream) -> Result<Vec<String>, String> {
    use std::io::Read;

    let mut reader = BufReader::new(&mut *stream);
    let mut count_line = String::new();
    reader
        .read_line(&mut count_line)
        .map_err(|err| format!("could not read the continuation request: {err}"))?;
    let count: usize = count_line
        .trim()
        .parse()
        .map_err(|_| "the continuation request is malformed".to_string())?;
    if count < 2 {
        return Err(
            "a continuation surface must name at least two distinct subjects; \
             a single-subject continuation declaration belongs to the designation \
             contract (RFC-0011)"
                .to_string(),
        );
    }
    if count > MAX_SURFACE_SUBJECTS {
        return Err("the continuation surface is unreasonably large".to_string());
    }

    let mut subjects = Vec::with_capacity(count);
    for _ in 0..count {
        let mut len_line = String::new();
        reader
            .read_line(&mut len_line)
            .map_err(|err| format!("could not read the continuation request: {err}"))?;
        let len: usize = len_line
            .trim()
            .parse()
            .map_err(|_| "the continuation request is malformed".to_string())?;
        if len == 0 || len > MAX_SUBJECT_BYTES {
            return Err("a continuation surface subject is empty or too long".to_string());
        }
        let mut bytes = vec![0u8; len];
        reader
            .read_exact(&mut bytes)
            .map_err(|err| format!("could not read a continuation surface subject: {err}"))?;
        let subject = String::from_utf8(bytes)
            .map_err(|_| "a continuation surface subject is not valid text".to_string())?;
        if subjects.contains(&subject) {
            return Err("a continuation surface must not name a subject twice".to_string());
        }
        subjects.push(subject);
    }
    Ok(subjects)
}

/// Validates a declared surface against the canonical Observation log
/// (RFC-0013 Declaration Semantics): every subject must resolve to a
/// canonical Artifact established by a content Observation. No fuzzy
/// matching, no guessing; a declaration referencing any unwitnessed subject
/// is rejected before any canonical Observation is created (no partial
/// declaration ever exists).
fn validate_surface(root: &Path, subjects: &[String]) -> Result<(), String> {
    for subject in subjects {
        if subject.trim().is_empty() {
            return Err("continuation surface subjects must not be empty".to_string());
        }
        match resolve_designated_artifact(root, subject) {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(format!(
                    "Evo has not witnessed the subject {subject:?} as a resource, so it cannot be \
                     declared in a continuation surface (no canonical object resolves to it)"
                ))
            }
            Err(err) => return Err(format!("could not validate a surface subject: {err}")),
        }
    }
    Ok(())
}

/// The wire response for one continuation request.
fn response_line(response: &ContinuationResponse) -> String {
    match response {
        ContinuationResponse::Accepted => "accepted\n".to_string(),
        ContinuationResponse::Rejected(reason) => format!("rejected:{reason}\n"),
    }
}

/// Desktop-side client: sends one explicit continuation-surface declaration to
/// the daemon and returns the daemon's honest response.
pub fn submit_continuation_surface(
    root: &Path,
    subjects: &[String],
) -> Result<ContinuationResponse, DaemonError> {
    let socket_path = continuation_surface_socket_path(root);
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        DaemonError::Designation(format!(
            "the capture worker is not running or not reachable ({}): {err}",
            socket_path.display()
        ))
    })?;
    stream
        .write_all(format!("{}\n", subjects.len()).as_bytes())
        .map_err(continuation_io)?;
    for subject in subjects {
        stream
            .write_all(format!("{}\n", subject.len()).as_bytes())
            .map_err(continuation_io)?;
        stream.write_all(subject.as_bytes()).map_err(continuation_io)?;
    }
    stream.flush().map_err(continuation_io)?;
    let mut response = String::new();
    BufReader::new(&mut stream)
        .read_line(&mut response)
        .map_err(continuation_io)?;
    let response = response.trim_end();
    if let Some(reason) = response.strip_prefix("rejected:") {
        Ok(ContinuationResponse::Rejected(reason.to_string()))
    } else if response == "accepted" {
        Ok(ContinuationResponse::Accepted)
    } else {
        Err(DaemonError::Designation(format!(
            "unexpected continuation response from the daemon: {response:?}"
        )))
    }
}

fn continuation_io(err: std::io::Error) -> DaemonError {
    DaemonError::Designation(format!("continuation channel error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use evo_observation::evidence::{Evidence, FactValue, ObservedFact};
    use evo_observation::observation::Observation;
    use evo_observation::observation_id::ObservationId;
    use evo_observation::observation_schema::ObservationSchema;
    use evo_observation::provenance::{ObservationSource, Provenance};
    use evo_storage::Storage;

    use std::collections::HashMap;
    use std::sync::mpsc::channel;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-daemon-continuation-{label}-{nanos}"))
    }

    fn observation_with_subject(schema: ObservationSchema, subject: &str) -> Observation {
        let fact_name = schema.canonical_fact_name().unwrap();
        Observation::new(
            ObservationId::new(),
            schema,
            Provenance::new(
                ObservationSource::new("continuation-test").unwrap(),
                UNIX_EPOCH,
                HashMap::new(),
            ),
            Evidence::new(vec![ObservedFact::new(fact_name, FactValue::Text(subject.into())).unwrap()]),
        )
    }

    fn witness(root: &Path, subjects: &[&str]) {
        let _guard = Storage::with_thread_root(root.to_path_buf());
        for subject in subjects {
            crate::persistence::persist_observation(&observation_with_subject(
                ObservationSchema::file_saved_v1(),
                subject,
            ))
            .expect("content observation persists");
        }
    }

    #[test]
    fn socket_path_is_short_and_root_deterministic() {
        let root = unique_root("path");
        let first = continuation_surface_socket_path(&root);
        let second = continuation_surface_socket_path(&root);
        assert_eq!(first, second, "same root maps to the same socket");
        assert!(
            first.as_os_str().to_string_lossy().len() < 104,
            "socket path must stay under SUN_LEN, was {}",
            first.display()
        );
        assert_ne!(
            first,
            crate::designation::designation_socket_path(&root),
            "continuation and designation sockets are distinct"
        );
        assert_ne!(
            first,
            crate::grouping::grouping_socket_path(&root),
            "continuation and grouping sockets are distinct"
        );
    }

    #[test]
    fn declaration_round_trips_through_the_socket() {
        let root = unique_root("socket");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_continuation_listener(root.clone(), sender);

        let socket = continuation_surface_socket_path(&root);
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        // No content observations exist yet: every subject is unwitnessed, so
        // the declaration is rejected honestly.
        let response = submit_continuation_surface(
            &root,
            &["/tmp/a.md".to_string(), "/tmp/b.md".to_string()],
        )
        .expect("submission should work");
        assert!(matches!(response, ContinuationResponse::Rejected(_)));
        assert!(
            receiver.try_recv().is_err(),
            "rejected request must not be forwarded"
        );

        // Witness both subjects.
        witness(&root, &["/tmp/a.md", "/tmp/b.md"]);

        // The declaration is accepted and forwarded with canonical ordering
        // (ascending; the smallest subject is the Required Subject).
        let response = submit_continuation_surface(
            &root,
            &["/tmp/b.md".to_string(), "/tmp/a.md".to_string()],
        )
        .expect("submission should work");
        assert_eq!(response, ContinuationResponse::Accepted);
        let signal = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("accepted declaration must be forwarded to the runtime");
        let MacOSSignal::ContinuationSurface { subjects, .. } = signal else {
            panic!("forwarded signal must be ContinuationSurface");
        };
        assert_eq!(subjects, vec!["/tmp/a.md".to_string(), "/tmp/b.md".to_string()]);
    }

    #[test]
    fn fewer_than_two_subjects_is_rejected() {
        let root = unique_root("single");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_continuation_listener(root.clone(), sender);

        let socket = continuation_surface_socket_path(&root);
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        witness(&root, &["/tmp/a.md"]);

        // One subject: rejected — a single-subject continuation declaration
        // belongs to the designation contract (RFC-0011).
        let response = submit_continuation_surface(
            &root,
            &["/tmp/a.md".to_string()],
        )
        .expect("submission should work");
        assert!(matches!(response, ContinuationResponse::Rejected(_)));
        assert!(receiver.try_recv().is_err(), "no partial declaration ever exists");
    }

    #[test]
    fn unwitnessed_subject_rejects_without_partial_state() {
        let root = unique_root("unwitnessed");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_continuation_listener(root.clone(), sender);

        let socket = continuation_surface_socket_path(&root);
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        // Witness only the first subject: the declaration is rejected.
        witness(&root, &["/tmp/a.md"]);
        let response = submit_continuation_surface(
            &root,
            &["/tmp/a.md".to_string(), "/tmp/missing.md".to_string()],
        )
        .expect("submission should work");
        assert!(matches!(response, ContinuationResponse::Rejected(_)));
        assert!(receiver.try_recv().is_err(), "no partial declaration ever exists");
    }

    #[test]
    fn duplicate_subjects_are_rejected() {
        let root = unique_root("duplicate");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_continuation_listener(root.clone(), sender);

        let socket = continuation_surface_socket_path(&root);
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        witness(&root, &["/tmp/a.md"]);

        let response = submit_continuation_surface(
            &root,
            &["/tmp/a.md".to_string(), "/tmp/a.md".to_string()],
        )
        .expect("submission should work");
        assert!(matches!(response, ContinuationResponse::Rejected(_)));
        assert!(receiver.try_recv().is_err(), "no partial declaration ever exists");
    }
}
