//! User naming channel (RFC-0014, OBS-WORK-NAMED).
//!
//! The desktop shell never writes canonical state. When the person names a
//! body of work — the word that answers "what do I call this work" — the
//! shell sends the name over a local Unix socket; the daemon — the canonical
//! runtime owner — validates the shape of the declaration, forwards it into
//! the vertical runtime as a WorkNamed signal, and the runtime accepts,
//! persists, and re-derives. The shell only reflects what the daemon reports.
//!
//! This is the RFC-0011 designation pattern applied to naming: the truth
//! condition is the performance of the declaration act through a trusted
//! capture origin (`user_naming`), exactly as WorkDesignated's truth
//! condition is the performance of the designation act. A name is the
//! person's word about their own work (Law IX); no derivation may overrule
//! it, and no witnessing is a precondition for speaking.

use crate::errors::DaemonError;
use evo_capture::MacOSSignal;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::SystemTime;

/// The maximum accepted name length, in bytes.
const MAX_NAME_BYTES: usize = 4096;

/// The deterministic socket path for one storage root (same derivation rule
/// as the designation socket, distinct prefix).
pub fn naming_socket_path(storage_root: &Path) -> PathBuf {
    // The socket name is a hash of the storage root. macOS paths are
    // case-insensitive ("/…/Evo/storage" and "/…/evo/storage" are the same
    // directory) but the string hash is not: the daemon (launchd-spawned,
    // lowercase env) and the desktop (bundle-configured, capitalized) must
    // land on the SAME socket, so the hash input is case-folded — the same
    // fold the filesystem applies.
    let folded = storage_root.to_string_lossy().to_lowercase();
    let hash = fnv1a64(folded.as_bytes());
    std::env::temp_dir().join(format!("evo-name-{hash:016x}.sock"))
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

/// The outcome of a naming request, as reported by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingResponse {
    /// The declaration was validated and forwarded for canonical processing.
    Accepted,
    /// The declaration was rejected. The reason is canonical and honest.
    Rejected(String),
}

impl NamingResponse {
    /// A short human-readable summary for presentation.
    pub fn summary(&self) -> &str {
        match self {
            NamingResponse::Accepted => "accepted",
            NamingResponse::Rejected(_) => "rejected",
        }
    }
}

/// Spawns the daemon-side naming listener on the deterministic socket path.
///
/// Each accepted connection is handled in a fresh thread. The listener never
/// writes canonical state: it validates the declaration's shape and, when
/// valid, forwards a WorkNamed signal into the vertical runtime's signal
/// channel.
pub fn spawn_naming_listener(storage_root: PathBuf, sender: Sender<MacOSSignal>) -> JoinHandle<()> {
    thread::spawn(move || {
        let socket_path = naming_socket_path(&storage_root);
        // A stale socket from a previous process must not block the rebind.
        let _ = std::fs::remove_file(&socket_path);
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("EVO-DAEMON naming listener bind failed: {err}");
                return;
            }
        };
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let sender = sender.clone();
                    thread::spawn(move || {
                        handle_naming_connection(stream, &sender);
                    });
                }
                Err(_) => continue,
            }
        }
    })
}

/// Handles one naming connection: read the pair, validate the shape, forward
/// when valid, reply honestly.
fn handle_naming_connection(stream: UnixStream, sender: &Sender<MacOSSignal>) {
    let mut stream = stream;
    let response = match read_request(&mut stream) {
        Err(reason) => NamingResponse::Rejected(reason),
        Ok((subject, name)) => {
            // A name that is really the subject re-observed carries nothing
            // the declaration can express; refuse it here, at the boundary,
            // so the shell reports the honest outcome immediately.
            if name.trim() == subject.trim() {
                NamingResponse::Rejected(
                    "a name must say something the subject does not already say".to_string(),
                )
            } else {
                match sender.send(MacOSSignal::WorkNamed {
                    subject,
                    name,
                    observed_at: SystemTime::now(),
                }) {
                    Ok(()) => NamingResponse::Accepted,
                    Err(_) => {
                        NamingResponse::Rejected("the capture runtime is shutting down".to_string())
                    }
                }
            }
        }
    };
    let _ = stream.write_all(response_line(&response).as_bytes());
    let _ = stream.flush();
}

/// Reads a length-prefixed (subject, name) pair from the connection.
///
/// ONE buffered reader spans both fields deliberately: a fresh `BufReader`
/// per field would let the first read buffer (and swallow) bytes of the
/// second field, deadlocking the exchange — the same wire contract the
/// grouping and designation channels use.
fn read_request(stream: &mut UnixStream) -> Result<(String, String), String> {
    let mut reader = BufReader::new(&mut *stream);
    let first = read_field(&mut reader, "subject")?;
    let second = read_field(&mut reader, "name")?;
    Ok((first, second))
}

fn read_field(reader: &mut BufReader<&mut UnixStream>, label: &str) -> Result<String, String> {
    use std::io::Read;

    let mut length_line = String::new();
    reader
        .read_line(&mut length_line)
        .map_err(|err| format!("could not read the naming request: {err}"))?;
    let length: usize = length_line
        .trim()
        .parse()
        .map_err(|_| "the naming request is malformed".to_string())?;
    if length == 0 || length > MAX_NAME_BYTES {
        return Err(format!("the naming {label} is empty or too long"));
    }
    let mut bytes = vec![0u8; length];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| format!("could not read the naming {label}: {err}"))?;
    let value =
        String::from_utf8(bytes).map_err(|_| format!("the naming {label} is not valid text"))?;
    if value.trim().is_empty() {
        return Err(format!("the naming {label} must not be empty"));
    }
    Ok(value)
}

/// The wire response for one naming request.
fn response_line(response: &NamingResponse) -> String {
    match response {
        NamingResponse::Accepted => "accepted\n".to_string(),
        NamingResponse::Rejected(reason) => format!("rejected:{reason}\n"),
    }
}

/// Desktop-side client: sends one explicit naming declaration to the daemon
/// and returns the daemon's honest response.
pub fn submit_naming(
    root: &Path,
    subject: &str,
    name: &str,
) -> Result<NamingResponse, DaemonError> {
    let socket_path = naming_socket_path(root);
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        DaemonError::Designation(format!(
            "the capture worker is not running or not reachable ({}): {err}",
            socket_path.display()
        ))
    })?;
    stream
        .write_all(format!("{}\n", subject.len()).as_bytes())
        .map_err(naming_io)?;
    stream.write_all(subject.as_bytes()).map_err(naming_io)?;
    stream
        .write_all(format!("{}\n", name.len()).as_bytes())
        .map_err(naming_io)?;
    stream.write_all(name.as_bytes()).map_err(naming_io)?;
    stream.flush().map_err(naming_io)?;
    let mut response = String::new();
    BufReader::new(&mut stream)
        .read_line(&mut response)
        .map_err(naming_io)?;
    let response = response.trim_end();
    if let Some(reason) = response.strip_prefix("rejected:") {
        Ok(NamingResponse::Rejected(reason.to_string()))
    } else if response == "accepted" {
        Ok(NamingResponse::Accepted)
    } else {
        Err(DaemonError::Designation(format!(
            "unexpected naming response from the daemon: {response:?}"
        )))
    }
}

fn naming_io(err: std::io::Error) -> DaemonError {
    DaemonError::Designation(format!("naming channel error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::mpsc::channel;
    use std::time::Duration;

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-daemon-naming-{label}-{nanos}"))
    }

    #[test]
    fn socket_path_is_short_and_root_deterministic() {
        let root = unique_root("path");
        let first = naming_socket_path(&root);
        let second = naming_socket_path(&root);
        assert_eq!(first, second, "same root maps to the same socket");
        assert!(
            first.as_os_str().to_string_lossy().len() < 104,
            "socket path must stay under SUN_LEN, was {}",
            first.display()
        );
        assert_ne!(
            first,
            crate::designation::designation_socket_path(&root),
            "naming and designation sockets are distinct"
        );
        assert_ne!(
            first,
            crate::grouping::grouping_socket_path(&root),
            "naming and grouping sockets are distinct"
        );
    }

    #[test]
    fn request_round_trips_through_the_socket() {
        let root = unique_root("socket");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_naming_listener(root.clone(), sender);

        let socket = naming_socket_path(&root);
        crate::socket_test_support::wait_for_listener(&socket);

        let response = submit_naming(&root, "workspace-abc", "The Pitch Deck")
            .expect("submission should work");
        assert_eq!(response, NamingResponse::Accepted);
        let signal = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("accepted naming must be forwarded to the runtime");
        let MacOSSignal::WorkNamed { subject, name, .. } = signal else {
            panic!("forwarded signal must be WorkNamed");
        };
        assert_eq!(subject, "workspace-abc");
        assert_eq!(name, "The Pitch Deck");
    }

    #[test]
    fn empty_and_self_referential_names_are_refused_at_the_channel() {
        let root = unique_root("reject");
        let (sender, receiver) = channel::<MacOSSignal>();
        let _listener = spawn_naming_listener(root.clone(), sender);

        let socket = naming_socket_path(&root);
        crate::socket_test_support::wait_for_listener(&socket);

        let response = submit_naming(&root, "workspace-abc", "   ").expect("submission works");
        assert!(
            matches!(response, NamingResponse::Rejected(_)),
            "a blank name carries nothing the concept can express"
        );

        let response = submit_naming(&root, "workspace-abc", "workspace-abc").expect("works");
        assert!(
            matches!(response, NamingResponse::Rejected(_)),
            "a name that is really the subject re-observed carries nothing"
        );

        assert!(
            receiver.try_recv().is_err(),
            "no rejected declaration is ever forwarded"
        );
    }
}
