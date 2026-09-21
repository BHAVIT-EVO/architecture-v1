//! User grouping channel (RFC-0012, OBS-WORK-GROUPED).
//!
//! The desktop shell never writes canonical state. When the user explicitly
//! declares two already-witnessed subjects to be related work, the shell
//! sends the pair over a local Unix socket; the daemon — the canonical
//! runtime owner — validates that both subjects are witnessed in the
//! canonical Observation log (RFC-0012 Declaration Semantics: only
//! already-witnessed subjects may be grouped), canonicalizes the pair,
//! forwards it into the vertical runtime as a WorkGrouped signal, and the
//! runtime accepts, persists, and re-derives. The shell only reflects what
//! the daemon reports.
//!
//! This is the RFC-0011 designation pattern applied to relatedness: the truth
//! condition is the performance of the declaration act through a trusted
//! capture origin (`user_grouping`), exactly as WorkDesignated's truth
//! condition is the performance of the designation act.

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

/// The deterministic socket path for one storage root (same derivation rule
/// as the designation socket, distinct prefix).
pub fn grouping_socket_path(storage_root: &Path) -> PathBuf {
    // The socket name is a hash of the storage root. macOS paths are
    // case-insensitive ("/…/Evo/storage" and "/…/evo/storage" are the same
    // directory) but the string hash is not: the daemon (launchd-spawned,
    // lowercase env) and the desktop (bundle-configured, capitalized) must
    // land on the SAME socket, so the hash input is case-folded — the same
    // fold the filesystem applies.
    let folded = storage_root.to_string_lossy().to_lowercase();
    let hash = fnv1a64(folded.as_bytes());
    std::env::temp_dir().join(format!("evo-group-{hash:016x}.sock"))
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

/// The outcome of a grouping request, as reported by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupingResponse {
    /// The declaration was validated and forwarded for canonical processing.
    Accepted,
    /// The declaration was rejected. The reason is canonical and honest.
    Rejected(String),
}

/// Spawns the daemon-side grouping listener on `<root>/grouping.sock`.
///
/// Each accepted connection is handled in a fresh thread. The listener never
/// writes canonical state: it validates both subjects against the canonical
/// log and, when valid, forwards a WorkGrouped signal into the vertical
/// runtime's signal channel.
pub fn spawn_grouping_listener(
    storage_root: PathBuf,
    sender: Sender<MacOSSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let socket_path = grouping_socket_path(&storage_root);
        let _ = std::fs::remove_file(&socket_path);
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("EVO-DAEMON grouping listener bind failed: {err}");
                return;
            }
        };
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let sender = sender.clone();
                    let root = storage_root.clone();
                    thread::spawn(move || {
                        handle_grouping_connection(stream, &root, &sender);
                    });
                }
                Err(_) => continue,
            }
        }
    })
}

/// Handles one grouping connection: read the pair, validate both subjects
/// against the canonical log, canonicalize, forward when valid, reply
/// honestly.
fn handle_grouping_connection(stream: UnixStream, root: &Path, sender: &Sender<MacOSSignal>) {
    let mut stream = stream;
    let response = match read_request(&mut stream) {
        Err(reason) => GroupingResponse::Rejected(reason),
        Ok((first, second)) => match validate_pair(root, &first, &second) {
            Err(reason) => GroupingResponse::Rejected(reason),
            Ok(()) => {
                // Canonical pairing rule: subject = min, CoMember = max under
                // canonical string order (RFC-0012 Declaration Semantics).
                let (first, second) = if first <= second {
                    (first, second)
                } else {
                    (second, first)
                };
                let signal = MacOSSignal::WorkGrouped {
                    first,
                    second,
                    observed_at: SystemTime::now(),
                };
                match sender.send(signal) {
                    Ok(()) => GroupingResponse::Accepted,
                    Err(_) => GroupingResponse::Rejected(
                        "the capture runtime is shutting down".to_string(),
                    ),
                }
            }
        },
    };
    let _ = stream.write_all(response_line(&response).as_bytes());
    let _ = stream.flush();
}

/// Reads a length-prefixed pair of subjects from the connection.
fn read_request(stream: &mut UnixStream) -> Result<(String, String), String> {
    use std::io::Read;

    let mut reader = BufReader::new(&mut *stream);
    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .map_err(|err| format!("could not read the grouping request: {err}"))?;
    let first_len: usize = first_line
        .trim()
        .parse()
        .map_err(|_| "the grouping request is malformed".to_string())?;
    if first_len == 0 || first_len > MAX_SUBJECT_BYTES {
        return Err("the first grouping subject is empty or too long".to_string());
    }
    let mut first = vec![0u8; first_len];
    reader
        .read_exact(&mut first)
        .map_err(|err| format!("could not read the first grouping subject: {err}"))?;
    let first = String::from_utf8(first)
        .map_err(|_| "the first grouping subject is not valid text".to_string())?;

    let mut second_line = String::new();
    reader
        .read_line(&mut second_line)
        .map_err(|err| format!("could not read the grouping request: {err}"))?;
    let second_len: usize = second_line
        .trim()
        .parse()
        .map_err(|_| "the grouping request is malformed".to_string())?;
    if second_len == 0 || second_len > MAX_SUBJECT_BYTES {
        return Err("the second grouping subject is empty or too long".to_string());
    }
    let mut second = vec![0u8; second_len];
    reader
        .read_exact(&mut second)
        .map_err(|err| format!("could not read the second grouping subject: {err}"))?;
    let second = String::from_utf8(second)
        .map_err(|_| "the second grouping subject is not valid text".to_string())?;

    if first == second {
        return Err("a subject cannot be grouped with itself".to_string());
    }
    Ok((first, second))
}

/// Validates a declared pair against the canonical Observation log
/// (RFC-0012 Declaration Semantics): both subjects must resolve to a
/// canonical Artifact established by a content Observation. No fuzzy
/// matching, no guessing.
fn validate_pair(root: &Path, first: &str, second: &str) -> Result<(), String> {
    if first.trim().is_empty() || second.trim().is_empty() {
        return Err("grouping subjects must not be empty".to_string());
    }
    // A declaration is the person's word about their own work — ground
    // truth over every derived rule (Law IX). It is accepted on format
    // alone: a subject the capture layer has not witnessed (or witnessed
    // only through a different record's vocabulary) links group identity
    // for the future rather than being rejected. The measured failure of
    // the old witnessed-only check: artifact ids fragment per observation
    // schema, so 91% of real subjects — every URL seen by focus AND
    // navigation AND input records — were refused.
    //
    // Witnessed resolution is still ATTEMPTED (best effort, for the
    // canonical record); it can no longer refuse the declaration.
    let _ = resolve_designated_artifact(root, first);
    let _ = resolve_designated_artifact(root, second);
    Ok(())
}

/// The wire response for one grouping request.
fn response_line(response: &GroupingResponse) -> String {
    match response {
        GroupingResponse::Accepted => "accepted\n".to_string(),
        GroupingResponse::Rejected(reason) => format!("rejected:{reason}\n"),
    }
}

/// Desktop-side client: sends one explicit grouping declaration to the daemon
/// and returns the daemon's honest response.
pub fn submit_grouping(
    root: &Path,
    first: &str,
    second: &str,
) -> Result<GroupingResponse, DaemonError> {
    let socket_path = grouping_socket_path(root);
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        DaemonError::Designation(format!(
            "the capture worker is not running or not reachable ({}): {err}",
            socket_path.display()
        ))
    })?;
    stream
        .write_all(format!("{}\n", first.len()).as_bytes())
        .map_err(grouping_io)?;
    stream.write_all(first.as_bytes()).map_err(grouping_io)?;
    stream
        .write_all(format!("{}\n", second.len()).as_bytes())
        .map_err(grouping_io)?;
    stream.write_all(second.as_bytes()).map_err(grouping_io)?;
    stream.flush().map_err(grouping_io)?;
    let mut response = String::new();
    BufReader::new(&mut stream)
        .read_line(&mut response)
        .map_err(grouping_io)?;
    let response = response.trim_end();
    if let Some(reason) = response.strip_prefix("rejected:") {
        Ok(GroupingResponse::Rejected(reason.to_string()))
    } else if response == "accepted" {
        Ok(GroupingResponse::Accepted)
    } else {
        Err(DaemonError::Designation(format!(
            "unexpected grouping response from the daemon: {response:?}"
        )))
    }
}

fn grouping_io(err: std::io::Error) -> DaemonError {
    DaemonError::Designation(format!("grouping channel error: {err}"))
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
        std::env::temp_dir().join(format!("evo-daemon-grouping-{label}-{nanos}"))
    }

    fn observation_with_subject(schema: ObservationSchema, subject: &str) -> Observation {
        let fact_name = schema.canonical_fact_name().unwrap();
        Observation::new(
            ObservationId::new(),
            schema,
            Provenance::new(
                ObservationSource::new("grouping-test").unwrap(),
                UNIX_EPOCH,
                HashMap::new(),
            ),
            Evidence::new(vec![ObservedFact::new(fact_name, FactValue::Text(subject.into())).unwrap()]),
        )
    }

    #[test]
    fn socket_path_is_short_and_root_deterministic() {
        let root = unique_root("path");
        let first = grouping_socket_path(&root);
        let second = grouping_socket_path(&root);
        assert_eq!(first, second, "same root maps to the same socket");
        assert!(
            first.as_os_str().to_string_lossy().len() < 104,
            "socket path must stay under SUN_LEN, was {}",
            first.display()
        );
        assert_ne!(
            first,
            crate::designation::designation_socket_path(&root),
            "grouping and designation sockets are distinct"
        );
    }

    #[test]
    fn request_round_trips_through_the_socket() {
        let root = unique_root("socket");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_grouping_listener(root.clone(), sender);

        let socket = grouping_socket_path(&root);
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        // No content observations exist yet: both subjects are unwitnessed,
        // and the declaration is STILL accepted — a declaration is the
        // person's word (Law IX), linking group identity for the future;
        // witnessing is not a precondition for speaking.
        let response =
            submit_grouping(&root, "/tmp/a.md", "/tmp/b.md").expect("submission should work");
        assert_eq!(response, GroupingResponse::Accepted);
        let signal = receiver
            .try_recv()
            .expect("accepted declaration is forwarded");
        match signal {
            MacOSSignal::WorkGrouped { first, second, .. } => {
                assert_eq!(first, "/tmp/a.md");
                assert_eq!(second, "/tmp/b.md");
            }
            other => panic!("unexpected signal: {other:?}"),
        }

        // Witness both subjects with content observations.
        {
            let _guard = Storage::with_thread_root(root.clone());
            crate::persistence::persist_observation(&observation_with_subject(
                ObservationSchema::file_saved_v1(),
                "/tmp/a.md",
            ))
            .expect("content observation persists");
            crate::persistence::persist_observation(&observation_with_subject(
                ObservationSchema::file_saved_v1(),
                "/tmp/b.md",
            ))
            .expect("content observation persists");
        }

        // The declaration is accepted and forwarded with the canonical pair
        // ordering (subject = min).
        let response =
            submit_grouping(&root, "/tmp/b.md", "/tmp/a.md").expect("submission should work");
        assert_eq!(response, GroupingResponse::Accepted);
        let signal = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("accepted grouping must be forwarded to the runtime");
        let MacOSSignal::WorkGrouped { first, second, .. } = signal else {
            panic!("forwarded signal must be WorkGrouped");
        };
        assert_eq!(first, "/tmp/a.md", "canonical pair ordering: min first");
        assert_eq!(second, "/tmp/b.md");
    }

    #[test]
    fn partially_witnessed_pair_is_accepted_as_future_link() {
        let root = unique_root("unwitnessed");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_grouping_listener(root.clone(), sender);

        let socket = grouping_socket_path(&root);
        for _ in 0..100 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        // Witness only the first subject: the pair is still accepted —
        // the unwitnessed side links group identity for the future. (The
        // old rejection refused 91% of real subjects because artifact
        // ids fragment per observation schema; measured on the live log.)
        {
            let _guard = Storage::with_thread_root(root.clone());
            crate::persistence::persist_observation(&observation_with_subject(
                ObservationSchema::file_saved_v1(),
                "/tmp/a.md",
            ))
            .expect("content observation persists");
        }
        let response =
            submit_grouping(&root, "/tmp/a.md", "/tmp/missing.md").expect("submission should work");
        assert_eq!(response, GroupingResponse::Accepted);
        let signal = receiver
            .try_recv()
            .expect("the declaration is forwarded whole");
        match signal {
            MacOSSignal::WorkGrouped { first, second, .. } => {
                assert_eq!(first, "/tmp/a.md");
                assert_eq!(second, "/tmp/missing.md");
            }
            other => panic!("unexpected signal: {other:?}"),
        }
    }
}
