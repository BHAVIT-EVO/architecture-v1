//! User feedback channel for work dismissal and member lifecycle state.
//!
//! The desktop shell sends explicit corrections over a local Unix socket. The
//! daemon validates every referenced subject against the canonical observation
//! log, forwards a platform signal to the worker-owned runtime, and replies
//! with the actual acceptance result. This channel never writes canonical state
//! from the listener thread and never accepts a derived thread id as evidence.

use crate::errors::DaemonError;
use crate::persistence::resolve_designated_artifact;
use evo_capture::MacOSSignal;
use evo_threads::MemberState;

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::SystemTime;

const MAX_FIELD_BYTES: usize = 4096;

/// Deterministic local socket for explicit work feedback.
pub fn feedback_socket_path(storage_root: &Path) -> PathBuf {
    let folded = storage_root.to_string_lossy().to_lowercase();
    std::env::temp_dir().join(format!("evo-feed-{:016x}.sock", fnv1a64(folded.as_bytes())))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x00000100000001B3);
    }
    hash
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackResponse {
    Accepted,
    Rejected(String),
}

impl FeedbackResponse {
    pub fn summary(&self) -> &str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected(_) => "rejected",
        }
    }
}

/// Starts the daemon-owned feedback listener. The listener only validates and
/// forwards; the worker remains the sole owner of persistence and derivation.
pub fn spawn_feedback_listener(
    storage_root: PathBuf,
    sender: Sender<MacOSSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let socket_path = feedback_socket_path(&storage_root);
        let _ = std::fs::remove_file(&socket_path);
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("EVO-DAEMON feedback listener bind failed: {err}");
                return;
            }
        };
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let sender = sender.clone();
            let root = storage_root.clone();
            thread::spawn(move || handle_connection(stream, &root, &sender));
        }
    })
}

fn handle_connection(mut stream: UnixStream, root: &Path, sender: &Sender<MacOSSignal>) {
    let response = match read_request(&mut stream) {
        Err(reason) => FeedbackResponse::Rejected(reason),
        Ok(Request::Dismiss { work }) => match validate_subject(root, &work) {
            Ok(()) => sender
                .send(MacOSSignal::WorkDismissed {
                    work,
                    observed_at: SystemTime::now(),
                })
                .map(|_| FeedbackResponse::Accepted)
                .unwrap_or_else(|_| {
                    FeedbackResponse::Rejected("the capture runtime is shutting down".into())
                }),
            Err(reason) => FeedbackResponse::Rejected(reason),
        },
        Ok(Request::MemberState {
            work,
            member,
            state,
        }) => {
            let state = match MemberState::parse(&state) {
                Some(state) => state,
                None => {
                    return write_response(
                        &mut stream,
                        FeedbackResponse::Rejected(
                            "member state must be Active, Completed, Superseded, or Optional"
                                .into(),
                        ),
                    )
                }
            };
            match (
                validate_subject(root, &work),
                validate_subject(root, &member),
            ) {
                (Ok(()), Ok(())) => sender
                    .send(MacOSSignal::MemberState {
                        work,
                        member,
                        state: state.label().to_string(),
                        observed_at: SystemTime::now(),
                    })
                    .map(|_| FeedbackResponse::Accepted)
                    .unwrap_or_else(|_| {
                        FeedbackResponse::Rejected("the capture runtime is shutting down".into())
                    }),
                (Err(reason), _) | (_, Err(reason)) => FeedbackResponse::Rejected(reason),
            }
        }
    };
    write_response(&mut stream, response);
}

enum Request {
    Dismiss {
        work: String,
    },
    MemberState {
        work: String,
        member: String,
        state: String,
    },
}

fn read_request(stream: &mut UnixStream) -> Result<Request, String> {
    let mut reader = BufReader::new(&mut *stream);
    let mut kind = String::new();
    reader
        .read_line(&mut kind)
        .map_err(|err| format!("could not read feedback kind: {err}"))?;
    match kind.trim() {
        "dismiss" => Ok(Request::Dismiss {
            work: read_field(&mut reader, "work")?,
        }),
        "member-state" => Ok(Request::MemberState {
            work: read_field(&mut reader, "work")?,
            member: read_field(&mut reader, "member")?,
            state: read_field(&mut reader, "state")?,
        }),
        _ => Err("feedback request kind is unknown".into()),
    }
}

fn read_field(reader: &mut BufReader<&mut UnixStream>, label: &str) -> Result<String, String> {
    let mut length_line = String::new();
    reader
        .read_line(&mut length_line)
        .map_err(|err| format!("could not read feedback {label} length: {err}"))?;
    let length: usize = length_line
        .trim()
        .parse()
        .map_err(|_| format!("feedback {label} length is malformed"))?;
    if length == 0 || length > MAX_FIELD_BYTES {
        return Err(format!("feedback {label} is empty or too long"));
    }
    let mut bytes = vec![0u8; length];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| format!("could not read feedback {label}: {err}"))?;
    let mut delimiter = [0u8; 1];
    reader
        .read_exact(&mut delimiter)
        .map_err(|err| format!("could not read feedback {label} delimiter: {err}"))?;
    if delimiter[0] != b'\n' {
        return Err(format!("feedback {label} is not newline-delimited"));
    }
    String::from_utf8(bytes).map_err(|_| format!("feedback {label} is not valid text"))
}

fn validate_subject(root: &Path, subject: &str) -> Result<(), String> {
    if subject.trim().is_empty() {
        return Err("feedback subject must not be empty".into());
    }
    match resolve_designated_artifact(root, subject) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(format!(
            "Evo has not witnessed `{subject}` as one unambiguous canonical resource"
        )),
        Err(err) => Err(format!("could not validate feedback subject: {err}")),
    }
}

fn write_response(stream: &mut UnixStream, response: FeedbackResponse) {
    let line = match response {
        FeedbackResponse::Accepted => "accepted\n".to_string(),
        FeedbackResponse::Rejected(reason) => format!("rejected:{reason}\n"),
    };
    let _ = stream.write_all(line.as_bytes());
    let _ = stream.flush();
}

pub fn submit_dismissal(root: &Path, work: &str) -> Result<FeedbackResponse, DaemonError> {
    submit(root, "dismiss", &[work])
}

pub fn submit_member_state(
    root: &Path,
    work: &str,
    member: &str,
    state: MemberState,
) -> Result<FeedbackResponse, DaemonError> {
    submit(root, "member-state", &[work, member, state.label()])
}

fn submit(root: &Path, kind: &str, fields: &[&str]) -> Result<FeedbackResponse, DaemonError> {
    let socket = feedback_socket_path(root);
    let mut stream = UnixStream::connect(&socket).map_err(|err| {
        DaemonError::Feedback(format!(
            "the feedback worker is not running or not reachable ({}): {err}",
            socket.display()
        ))
    })?;
    stream
        .write_all(format!("{kind}\n").as_bytes())
        .map_err(feedback_io)?;
    for field in fields {
        if field.is_empty() || field.len() > MAX_FIELD_BYTES {
            return Ok(FeedbackResponse::Rejected(
                "feedback field is empty or too long".into(),
            ));
        }
        stream
            .write_all(format!("{}\n", field.len()).as_bytes())
            .and_then(|_| stream.write_all(field.as_bytes()))
            .and_then(|_| stream.write_all(b"\n"))
            .map_err(feedback_io)?;
    }
    stream.flush().map_err(feedback_io)?;
    let mut response = String::new();
    BufReader::new(&mut stream)
        .read_line(&mut response)
        .map_err(feedback_io)?;
    let response = response.trim_end();
    if response == "accepted" {
        Ok(FeedbackResponse::Accepted)
    } else if let Some(reason) = response.strip_prefix("rejected:") {
        Ok(FeedbackResponse::Rejected(reason.to_string()))
    } else {
        Err(DaemonError::Feedback(format!(
            "unexpected feedback response from daemon: {response:?}"
        )))
    }
}

fn feedback_io(err: std::io::Error) -> DaemonError {
    DaemonError::Feedback(format!("feedback channel error: {err}"))
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
    use std::time::{Duration, UNIX_EPOCH};

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-daemon-feedback-{label}-{nanos}"))
    }

    fn observation_with_subject(subject: &str) -> Observation {
        let schema = ObservationSchema::file_saved_v1();
        Observation::new(
            ObservationId::new(),
            schema.clone(),
            Provenance::new(
                ObservationSource::new("feedback-test").unwrap(),
                UNIX_EPOCH,
                HashMap::new(),
            ),
            Evidence::new(vec![ObservedFact::new(
                schema.canonical_fact_name().unwrap(),
                FactValue::Text(subject.into()),
            )
            .unwrap()]),
        )
    }

    #[test]
    fn socket_path_is_short_and_root_specific() {
        let root = unique_root("path");
        let socket = feedback_socket_path(&root);
        assert_eq!(socket, feedback_socket_path(&root));
        assert!(socket.as_os_str().to_string_lossy().len() < 104);
        assert_ne!(socket, feedback_socket_path(&unique_root("other")));
    }

    #[test]
    fn validated_feedback_round_trips_and_invalid_state_is_rejected() {
        let root = unique_root("socket");
        {
            let _guard = Storage::with_thread_root(root.clone());
            for subject in ["/tmp/work.md", "/tmp/member.md"] {
                crate::persistence::persist_observation(&observation_with_subject(subject))
                    .expect("content observation persists");
            }
        }
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_feedback_listener(root.clone(), sender);
        crate::socket_test_support::wait_for_listener(&feedback_socket_path(&root));

        assert_eq!(
            submit_member_state(
                &root,
                "/tmp/work.md",
                "/tmp/member.md",
                MemberState::Completed,
            )
            .unwrap(),
            FeedbackResponse::Accepted
        );
        let signal = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("accepted state must be forwarded");
        assert!(matches!(
            signal,
            MacOSSignal::MemberState { work, member, state, .. }
                if work == "/tmp/work.md"
                    && member == "/tmp/member.md"
                    && state == "Completed"
        ));

        let rejected = submit(
            &root,
            "member-state",
            &["/tmp/work.md", "/tmp/member.md", "unknown"],
        )
        .unwrap();
        assert!(matches!(rejected, FeedbackResponse::Rejected(_)));
        assert!(receiver.try_recv().is_err());
    }
}
