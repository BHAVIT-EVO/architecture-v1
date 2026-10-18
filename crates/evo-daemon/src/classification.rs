//! Classified surfaces: the log's `OBS-SURFACE-CLASSIFIED` records, indexed by
//! subject.
//!
//! A classification is the person's own word about a surface other
//! Observations already established — "this is not work", "this is media"
//! (`OBS-SURFACE-CLASSIFIED/v1`). Evo ships no default class: silence produces
//! no classification, and every entry here carries the person's name on it.
//!
//! Two layers read this index, and they read it for the same reason:
//!
//! * the **engine** gates *founding* — a classified subject never becomes a
//!   thread anchor (`evo-threads`' `Declaration::Classified`), and
//! * this **presentation layer** gates *titling* — a classified subject never
//!   names a work, even when it rides along as a companion or a resume point.
//!
//! Neither layer ever removes the subject from a work. The non-deletion
//! guarantee holds: a class is not a quarantine.

use crate::classification_rules::IntervalClassification;
use crate::errors::DaemonError;
use evo_capture::MacOSSignal;
use evo_observation::evidence::FactValue;
use evo_observation::observation::Observation;

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::SystemTime;

/// The classified-surface index: subject → the class the person assigned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassifiedSurfaces {
    /// Subject → class. A later classification of the same subject replaces an
    /// earlier one, because it was said later — the same rule every other
    /// declaration follows.
    classes: BTreeMap<String, String>,
}

impl ClassifiedSurfaces {
    /// An index with nothing classified. Titling then behaves exactly as it did
    /// before this module existed.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builds the index over a slice of canonical Observations. Every
    /// `OBS-SURFACE-CLASSIFIED` record contributes one classified subject.
    pub fn from_observations(observations: &[Observation]) -> Self {
        let mut index = Self::empty();
        for observation in observations {
            if observation.schema().name() != "OBS-SURFACE-CLASSIFIED" {
                continue;
            }
            let subject = observation
                .evidence()
                .fact("SurfaceClassified")
                .and_then(|f| match f.value() {
                    FactValue::Text(subject) => Some(subject.clone()),
                    _ => None,
                });
            let class = observation
                .evidence()
                .fact("Class")
                .and_then(|f| match f.value() {
                    FactValue::Text(class) => Some(class.clone()),
                    _ => None,
                });
            if let (Some(subject), Some(class)) = (subject, class) {
                let subject = subject.trim().to_string();
                let class = class.trim().to_string();
                if !subject.is_empty() && !class.is_empty() {
                    index.classes.insert(subject, class);
                }
            }
        }
        index
    }

    /// Projects pure rule output onto the same in-memory index used by
    /// canonical `OBS-SURFACE-CLASSIFIED` observations.
    ///
    /// This is a derived view only. It deliberately does not claim that the
    /// results were accepted, assigned provenance, or persisted; callers that
    /// need canonical history must send each result's observation concept
    /// through the existing acceptance/persistence path first.
    pub fn from_rule_classifications(classifications: &[IntervalClassification]) -> Self {
        let mut index = Self::empty();
        for classification in classifications {
            index.record(&classification.subject, &classification.class);
        }
        index
    }

    /// Records one classification: the person called `subject` `class`. A later
    /// record for the same subject replaces an earlier one, because it was said
    /// later. Blank values carry nothing and are ignored.
    pub fn record(&mut self, subject: &str, class: &str) {
        let subject = subject.trim();
        let class = class.trim();
        if subject.is_empty() || class.is_empty() {
            return;
        }
        self.classes.insert(subject.to_string(), class.to_string());
    }

    /// Whether the person classified this subject.
    pub fn is_classified(&self, subject: &str) -> bool {
        self.classes.contains_key(subject)
    }

    /// The class the person assigned to this subject, if any.
    pub fn class_of(&self, subject: &str) -> Option<&str> {
        self.classes.get(subject).map(|class| class.as_str())
    }

    /// Every classified subject.
    pub fn subjects(&self) -> impl Iterator<Item = &String> {
        self.classes.keys()
    }

    /// How many subjects the person has classified.
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// Whether the person has classified nothing.
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }
}

// ── Authoring channel (the writer this record needs) ─────────────────────────

/// The maximum accepted subject or class length, in bytes.
const MAX_FIELD_BYTES: usize = 4096;

/// The deterministic socket path for one storage root (same derivation rule as
/// the naming socket, distinct prefix).
pub fn classification_socket_path(storage_root: &Path) -> PathBuf {
    // Case-folded for the same reason the other channels are: macOS paths are
    // case-insensitive, so the launchd-spawned daemon and the bundle-configured
    // desktop must land on the SAME socket.
    let folded = storage_root.to_string_lossy().to_lowercase();
    let hash = fnv1a64(folded.as_bytes());
    std::env::temp_dir().join(format!("evo-class-{hash:016x}.sock"))
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

/// The outcome of a classification request, as reported by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassificationResponse {
    /// The declaration was validated and forwarded for canonical processing.
    Accepted,
    /// The declaration was rejected. The reason is canonical and honest.
    Rejected(String),
}

impl ClassificationResponse {
    /// A short human-readable summary for presentation.
    pub fn summary(&self) -> &str {
        match self {
            ClassificationResponse::Accepted => "accepted",
            ClassificationResponse::Rejected(_) => "rejected",
        }
    }
}

/// Spawns the daemon-side classification listener on the deterministic socket
/// path.
///
/// The listener never writes canonical state: it validates the declaration's
/// shape and, when valid, forwards a `SurfaceClassified` signal into the
/// vertical runtime's signal channel. A classification is the person's own word
/// about a surface (RFC-0014 R11), so witnessing is not a precondition for
/// speaking — exactly as naming and grouping accept on format alone.
pub fn spawn_classification_listener(
    storage_root: PathBuf,
    sender: Sender<MacOSSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let socket_path = classification_socket_path(&storage_root);
        // A stale socket from a previous process must not block the rebind.
        let _ = std::fs::remove_file(&socket_path);
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("EVO-DAEMON classification listener bind failed: {err}");
                return;
            }
        };
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let sender = sender.clone();
                    thread::spawn(move || {
                        handle_classification_connection(stream, &sender);
                    });
                }
                Err(_) => continue,
            }
        }
    })
}

/// Handles one classification connection: read the pair, validate the shape,
/// forward when valid, reply honestly.
fn handle_classification_connection(stream: UnixStream, sender: &Sender<MacOSSignal>) {
    let mut stream = stream;
    let response = match read_request(&mut stream) {
        Err(reason) => ClassificationResponse::Rejected(reason),
        Ok((subject, class)) => {
            // A class that is really the subject re-observed carries nothing
            // the declaration can express; refuse it here, at the boundary, so
            // the shell reports the honest outcome immediately.
            if class.trim() == subject.trim() {
                ClassificationResponse::Rejected(
                    "a class must say something the surface does not already say".to_string(),
                )
            } else {
                match sender.send(MacOSSignal::SurfaceClassified {
                    subject,
                    class,
                    observed_at: SystemTime::now(),
                }) {
                    Ok(()) => ClassificationResponse::Accepted,
                    Err(_) => ClassificationResponse::Rejected(
                        "the capture runtime is shutting down".to_string(),
                    ),
                }
            }
        }
    };
    let _ = stream.write_all(response_line(&response).as_bytes());
    let _ = stream.flush();
}

/// Reads a length-prefixed (subject, class) pair from the connection.
///
/// ONE buffered reader spans both fields deliberately: a fresh `BufReader` per
/// field would let the first read buffer (and swallow) bytes of the second,
/// deadlocking the exchange — the same wire contract every other channel uses.
fn read_request(stream: &mut UnixStream) -> Result<(String, String), String> {
    let mut reader = BufReader::new(&mut *stream);
    let subject = read_field(&mut reader, "subject")?;
    let class = read_field(&mut reader, "class")?;
    Ok((subject, class))
}

fn read_field(reader: &mut BufReader<&mut UnixStream>, label: &str) -> Result<String, String> {
    use std::io::Read;

    let mut length_line = String::new();
    reader
        .read_line(&mut length_line)
        .map_err(|err| format!("could not read the classification request: {err}"))?;
    let length: usize = length_line
        .trim()
        .parse()
        .map_err(|_| "the classification request is malformed".to_string())?;
    if length == 0 || length > MAX_FIELD_BYTES {
        return Err(format!("the classification {label} is empty or too long"));
    }
    let mut bytes = vec![0u8; length];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| format!("could not read the classification {label}: {err}"))?;
    let value = String::from_utf8(bytes)
        .map_err(|_| format!("the classification {label} is not valid text"))?;
    if value.trim().is_empty() {
        return Err(format!("the classification {label} must not be empty"));
    }
    Ok(value)
}

/// The wire response for one classification request.
fn response_line(response: &ClassificationResponse) -> String {
    match response {
        ClassificationResponse::Accepted => "accepted\n".to_string(),
        ClassificationResponse::Rejected(reason) => format!("rejected:{reason}\n"),
    }
}

/// Desktop-side client: sends one explicit classification to the daemon and
/// returns the daemon's honest response.
pub fn submit_classification(
    root: &Path,
    subject: &str,
    class: &str,
) -> Result<ClassificationResponse, DaemonError> {
    let socket_path = classification_socket_path(root);
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        DaemonError::Designation(format!(
            "the capture worker is not running or not reachable ({}): {err}",
            socket_path.display()
        ))
    })?;
    stream
        .write_all(format!("{}\n", subject.len()).as_bytes())
        .map_err(classification_io)?;
    stream.write_all(subject.as_bytes()).map_err(classification_io)?;
    stream
        .write_all(format!("{}\n", class.len()).as_bytes())
        .map_err(classification_io)?;
    stream.write_all(class.as_bytes()).map_err(classification_io)?;
    stream.flush().map_err(classification_io)?;
    let mut response = String::new();
    BufReader::new(&mut stream)
        .read_line(&mut response)
        .map_err(classification_io)?;
    let response = response.trim_end();
    if let Some(reason) = response.strip_prefix("rejected:") {
        Ok(ClassificationResponse::Rejected(reason.to_string()))
    } else if response == "accepted" {
        Ok(ClassificationResponse::Accepted)
    } else {
        Err(DaemonError::Designation(format!(
            "unexpected classification response from the daemon: {response:?}"
        )))
    }
}

fn classification_io(err: std::io::Error) -> DaemonError {
    DaemonError::Designation(format!("classification channel error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_observation::accept::accept;
    use evo_observation::candidate::CandidateObservation;
    use evo_observation::observation_language::ObservationConcept;
    use evo_observation::provenance::{ObservationSource, Provenance};
    use std::collections::HashMap;
    use std::time::{Duration, UNIX_EPOCH};

    fn observation(at_secs: u64, concept: &ObservationConcept) -> Observation {
        let schema = concept.schema();
        let provenance = Provenance::new(
            ObservationSource::new("macos_adapter").unwrap(),
            UNIX_EPOCH + Duration::from_secs(at_secs),
            HashMap::new(),
        );
        let candidate = CandidateObservation::new(schema.clone(), provenance, concept.evidence());
        accept(candidate, &schema).expect("valid concept")
    }

    #[test]
    fn an_empty_log_classifies_nothing() {
        let index = ClassifiedSurfaces::from_observations(&[]);
        assert!(index.is_empty());
        assert!(!index.is_classified("https://www.youtube.com/watch?v=x"));
    }

    #[test]
    fn a_classification_is_indexed_by_subject() {
        let observations = vec![
            observation(
                10,
                &ObservationConcept::SurfaceClassified {
                    subject: "https://www.youtube.com/watch?v=abc".into(),
                    class: "NotWork".into(),
                },
            ),
            observation(
                20,
                &ObservationConcept::SurfaceNamed {
                    subject: "https://example.com/field-guide".into(),
                    name: "A Field Guide".into(),
                },
            ),
        ];
        let index = ClassifiedSurfaces::from_observations(&observations);
        assert_eq!(index.len(), 1);
        assert!(index.is_classified("https://www.youtube.com/watch?v=abc"));
        assert_eq!(
            index.class_of("https://www.youtube.com/watch?v=abc"),
            Some("NotWork")
        );
        // A naming is not a classification.
        assert!(!index.is_classified("https://example.com/field-guide"));
    }

    #[test]
    fn socket_path_is_short_and_root_deterministic() {
        let root = std::env::temp_dir().join("evo-daemon-classification-path");
        let first = classification_socket_path(&root);
        let second = classification_socket_path(&root);
        assert_eq!(first, second, "same root maps to the same socket");
        assert!(
            first.as_os_str().to_string_lossy().len() < 104,
            "socket path must stay under SUN_LEN, was {}",
            first.display()
        );
        assert_ne!(
            first,
            crate::naming::naming_socket_path(&root),
            "classification and naming sockets are distinct"
        );
        assert_ne!(
            first,
            crate::grouping::grouping_socket_path(&root),
            "classification and grouping sockets are distinct"
        );
    }

    #[test]
    fn request_round_trips_through_the_socket() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("evo-daemon-classification-socket-{nanos}"));
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_classification_listener(root.clone(), sender);

        let socket = classification_socket_path(&root);
        crate::socket_test_support::wait_for_listener(&socket);

        let response = submit_classification(
            &root,
            "https://www.youtube.com/watch?v=EsW2BoFAUs",
            "NotWork",
        )
        .expect("submission should work");
        assert_eq!(response, ClassificationResponse::Accepted);
        let signal = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("accepted classification must be forwarded to the runtime");
        match signal {
            MacOSSignal::SurfaceClassified { subject, class, .. } => {
                assert_eq!(subject, "https://www.youtube.com/watch?v=EsW2BoFAUs");
                assert_eq!(class, "NotWork");
            }
            other => panic!("forwarded signal must be SurfaceClassified, got {other:?}"),
        }
    }

    #[test]
    fn empty_and_self_referential_classes_are_refused_at_the_channel() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("evo-daemon-classification-reject-{nanos}"));
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_classification_listener(root.clone(), sender);

        let socket = classification_socket_path(&root);
        crate::socket_test_support::wait_for_listener(&socket);

        let response = submit_classification(&root, "https://x.example/a", "   ")
            .expect("submission works");
        assert!(
            matches!(response, ClassificationResponse::Rejected(_)),
            "a blank class carries nothing the concept can express"
        );

        let response = submit_classification(&root, "https://x.example/a", "https://x.example/a")
            .expect("submission works");
        assert!(
            matches!(response, ClassificationResponse::Rejected(_)),
            "a class that is really the subject re-observed carries nothing"
        );

        assert!(
            receiver.recv_timeout(Duration::from_millis(50)).is_err(),
            "no rejected declaration is ever forwarded"
        );
    }

    #[test]
    fn a_later_classification_replaces_an_earlier_one() {
        let observations = vec![
            observation(
                10,
                &ObservationConcept::SurfaceClassified {
                    subject: "https://x.example/a".into(),
                    class: "NotWork".into(),
                },
            ),
            observation(
                20,
                &ObservationConcept::SurfaceClassified {
                    subject: "https://x.example/a".into(),
                    class: "Media".into(),
                },
            ),
        ];
        let index = ClassifiedSurfaces::from_observations(&observations);
        assert_eq!(index.len(), 1);
        assert_eq!(index.class_of("https://x.example/a"), Some("Media"));
    }

    #[test]
    fn rule_output_projects_into_the_same_derived_index_without_persistence() {
        let classifications = [
            IntervalClassification {
                interval_id: 1,
                subject: "https://x.example/a".into(),
                class: "NotWork".into(),
                rule_id: "rule-a".into(),
                source: crate::classification_rules::ClassificationSource::Direct,
            },
            IntervalClassification {
                interval_id: 2,
                subject: "https://x.example/a".into(),
                class: "Media".into(),
                rule_id: "rule-a".into(),
                source: crate::classification_rules::ClassificationSource::Absorbed,
            },
        ];
        let index = ClassifiedSurfaces::from_rule_classifications(&classifications);
        assert_eq!(index.class_of("https://x.example/a"), Some("Media"));
        assert_eq!(index.len(), 1);
    }
}
