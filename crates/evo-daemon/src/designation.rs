//! User designation channel (RFC-0011).
//!
//! The desktop shell never writes canonical state. When the user explicitly
//! marks a witnessed subject as the work to continue, the shell sends the
//! designation over a local Unix socket; the daemon — the canonical runtime
//! owner — validates it against the canonical log (RFC-0011 §4 resolution
//! rule), forwards it into the vertical runtime as a WorkDesignated signal,
//! and the runtime accepts, persists, and re-derives. The shell only reflects
//! what the daemon reports.
//!
//! The listener never writes canonical state itself: it reads the Observation
//! log to validate the subject, then hands the request to the worker thread
//! that owns persistence and derivation.

use crate::errors::DaemonError;
use crate::persistence::resolve_designated_artifact;
use evo_capture::MacOSSignal;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::SystemTime;

/// The maximum accepted designation-subject length, in bytes.
const MAX_SUBJECT_BYTES: usize = 4096;

/// The deterministic socket path for one storage root.
///
/// Unix sockets have a hard path-length limit (SUN_LEN, 104 bytes), so the
/// socket cannot live inside an arbitrarily long storage root. It lives in
/// the user's temp directory under a short, deterministic hash of the root:
/// the same root always maps to the same socket, and distinct roots map to
/// distinct sockets. Derived presentation/transport detail only — never a
/// canonical object.
pub fn designation_socket_path(storage_root: &Path) -> PathBuf {
    // The socket name is a hash of the storage root. macOS paths are
    // case-insensitive ("/…/Evo/storage" and "/…/evo/storage" are the same
    // directory) but the string hash is not: the daemon (launchd-spawned,
    // lowercase env) and the desktop (bundle-configured, capitalized) must
    // land on the SAME socket, so the hash input is case-folded — the same
    // fold the filesystem applies.
    let folded = storage_root.to_string_lossy().to_lowercase();
    let hash = fnv1a64(folded.as_bytes());
    std::env::temp_dir().join(format!("evo-desig-{hash:016x}.sock"))
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

/// The outcome of a designation request, as reported by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesignationResponse {
    /// The designation was validated and forwarded for canonical processing.
    Accepted,
    /// The designation was rejected. The reason is canonical and honest.
    Rejected(String),
}

impl DesignationResponse {
    /// A short human-readable summary for presentation.
    pub fn summary(&self) -> &str {
        match self {
            DesignationResponse::Accepted => "accepted",
            DesignationResponse::Rejected(_) => "rejected",
        }
    }
}

/// Spawns the daemon-side designation listener on `<root>/designation.sock`.
///
/// Each accepted connection is handled in a fresh thread. The listener never
/// writes canonical state: it validates the subject against the canonical log
/// and, when valid, forwards a WorkDesignated signal into the vertical
/// runtime's signal channel.
pub fn spawn_designation_listener(
    storage_root: PathBuf,
    sender: Sender<MacOSSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let socket_path = designation_socket_path(&storage_root);
        // A stale socket from a previous process must not block the rebind.
        let _ = std::fs::remove_file(&socket_path);
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("EVO-DAEMON designation listener bind failed: {err}");
                return;
            }
        };
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let sender = sender.clone();
                    let root = storage_root.clone();
                    thread::spawn(move || {
                        handle_designation_connection(stream, &root, &sender);
                    });
                }
                Err(_) => continue,
            }
        }
    })
}

/// Handles one designation connection: read the request, validate it against
/// the canonical log, forward it when valid, and reply honestly.
fn handle_designation_connection(stream: UnixStream, root: &Path, sender: &Sender<MacOSSignal>) {
    let mut stream = stream;
    let response = match read_request(&mut stream) {
        Err(reason) => DesignationResponse::Rejected(reason),
        Ok(subject) => match validate_subject(root, &subject) {
            Err(reason) => DesignationResponse::Rejected(reason),
            Ok(()) => {
                let signal = MacOSSignal::WorkDesignated {
                    subject,
                    observed_at: SystemTime::now(),
                };
                match sender.send(signal) {
                    Ok(()) => DesignationResponse::Accepted,
                    Err(_) => DesignationResponse::Rejected(
                        "the capture runtime is shutting down".to_string(),
                    ),
                }
            }
        },
    };
    let _ = stream.write_all(response_line(&response).as_bytes());
    let _ = stream.flush();
}

/// Reads a length-prefixed designation subject from the connection.
fn read_request(stream: &mut UnixStream) -> Result<String, String> {
    use std::io::Read;

    let mut reader = BufReader::new(&mut *stream);
    let mut length_line = String::new();
    reader
        .read_line(&mut length_line)
        .map_err(|err| format!("could not read the designation request: {err}"))?;
    let length: usize = length_line
        .trim()
        .parse()
        .map_err(|_| "the designation request is malformed".to_string())?;
    if length == 0 || length > MAX_SUBJECT_BYTES {
        return Err("the designation subject is empty or too long".to_string());
    }
    let mut subject = vec![0u8; length];
    reader
        .read_exact(&mut subject)
        .map_err(|err| format!("could not read the designation subject: {err}"))?;
    String::from_utf8(subject).map_err(|_| "the designation subject is not valid text".to_string())
}

/// Validates a designation subject against the canonical Observation log
/// (RFC-0011 §4): the subject must resolve to exactly one canonical Artifact
/// established by a content Observation. No fuzzy matching, no guessing.
fn validate_subject(root: &Path, subject: &str) -> Result<(), String> {
    if subject.trim().is_empty() {
        return Err("the designation subject must not be empty".to_string());
    }
    match resolve_designated_artifact(root, subject) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(
            "Evo has not witnessed this subject as a resource, so it cannot be marked as the work to continue (no canonical object resolves to it)."
                .to_string(),
        ),
        Err(err) => Err(format!("could not validate the designation: {err}")),
    }
}

/// The wire response for one designation request.
fn response_line(response: &DesignationResponse) -> String {
    match response {
        DesignationResponse::Accepted => "accepted\n".to_string(),
        DesignationResponse::Rejected(reason) => format!("rejected:{reason}\n"),
    }
}

/// Desktop-side client: sends one explicit designation request to the daemon
/// and returns the daemon's honest response.
pub fn submit_designation(
    root: &Path,
    subject: &str,
) -> Result<DesignationResponse, DaemonError> {
    let socket_path = designation_socket_path(root);
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        DaemonError::Designation(format!(
            "the capture worker is not running or not reachable ({}): {err}",
            socket_path.display()
        ))
    })?;
    stream
        .write_all(format!("{}\n", subject.len()).as_bytes())
        .map_err(designation_io)?;
    stream.write_all(subject.as_bytes()).map_err(designation_io)?;
    stream.flush().map_err(designation_io)?;
    let mut response = String::new();
    BufReader::new(&mut stream)
        .read_line(&mut response)
        .map_err(designation_io)?;
    let response = response.trim_end();
    if let Some(reason) = response.strip_prefix("rejected:") {
        Ok(DesignationResponse::Rejected(reason.to_string()))
    } else if response == "accepted" {
        Ok(DesignationResponse::Accepted)
    } else {
        Err(DaemonError::Designation(format!(
            "unexpected designation response from the daemon: {response:?}"
        )))
    }
}

fn designation_io(err: std::io::Error) -> DaemonError {
    DaemonError::Designation(format!("designation channel error: {err}"))
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
        std::env::temp_dir().join(format!("evo-daemon-designation-{label}-{nanos}"))
    }

    #[test]
    fn socket_path_is_short_and_root_deterministic() {
        let root = unique_root("path");
        let first = designation_socket_path(&root);
        let second = designation_socket_path(&root);
        assert_eq!(first, second, "same root maps to the same socket");
        assert!(
            first.as_os_str().to_string_lossy().len() < 104,
            "socket path must stay under SUN_LEN, was {}",
            first.display()
        );
        let other = designation_socket_path(&unique_root("path"));
        assert_ne!(first, other, "distinct roots map to distinct sockets");
    }

    fn observation_with_subject(schema: ObservationSchema, subject: &str) -> Observation {
        let fact_name = schema.canonical_fact_name().unwrap();
        Observation::new(
            ObservationId::new(),
            schema,
            Provenance::new(
                ObservationSource::new("designation-test").unwrap(),
                UNIX_EPOCH,
                HashMap::new(),
            ),
            Evidence::new(vec![ObservedFact::new(fact_name, FactValue::Text(subject.into())).unwrap()]),
        )
    }

    #[test]
    fn request_round_trips_through_the_socket() {
        let root = unique_root("socket");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_designation_listener(root.clone(), sender);

        // Wait for the socket to exist before connecting.
        let socket = designation_socket_path(&root);
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let response = submit_designation(&root, "/tmp/plan.md").expect("submission should work");
        // No content observation exists yet, so the subject cannot resolve to
        // a canonical Artifact: the daemon rejects it honestly.
        assert!(matches!(response, DesignationResponse::Rejected(_)));
        assert!(receiver.try_recv().is_err(), "rejected request must not be forwarded");

        // Once a content observation witnesses the subject, the designation
        // is accepted and forwarded.
        {
            let _guard = Storage::with_thread_root(root.clone());
            crate::persistence::persist_observation(&observation_with_subject(
                ObservationSchema::file_saved_v1(),
                "/tmp/plan.md",
            ))
            .expect("content observation persists");
        }

        let response = submit_designation(&root, "/tmp/plan.md").expect("submission should work");
        assert_eq!(response, DesignationResponse::Accepted);
        let signal = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("accepted designation must be forwarded to the runtime");
        let MacOSSignal::WorkDesignated { subject, .. } = signal else {
            panic!("forwarded signal must be WorkDesignated");
        };
        assert_eq!(subject, "/tmp/plan.md");
    }

    #[test]
    fn resolution_rule_is_deterministic_and_subject_exact() {
        let root = unique_root("resolve");
        {
            let _guard = Storage::with_thread_root(root.clone());
            crate::persistence::persist_observation(&observation_with_subject(
                ObservationSchema::file_saved_v1(),
                "/tmp/plan.md",
            ))
            .expect("content observation persists");
        }

        let resolved = resolve_designated_artifact(&root, "/tmp/plan.md").expect("resolution");
        assert!(resolved.is_some(), "witnessed subject resolves to its Artifact");

        // A subject that was never witnessed does not resolve — no fuzzy
        // matching, no guessing.
        let missing = resolve_designated_artifact(&root, "/tmp/other.md").expect("resolution");
        assert!(missing.is_none());

        // A subject witnessed by a WorkDesignated observation alone never
        // establishes an Artifact (RFC-0011 §4).
        {
            let _guard = Storage::with_thread_root(root.clone());
            crate::persistence::persist_observation(&observation_with_subject(
                ObservationSchema::work_designated_v1(),
                "/tmp/lonely.md",
            ))
            .expect("designation observation persists");
        }
        let lonely = resolve_designated_artifact(&root, "/tmp/lonely.md").expect("resolution");
        assert!(lonely.is_none());
    }
}
