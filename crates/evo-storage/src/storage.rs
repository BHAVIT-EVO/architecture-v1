//! Storage boundary.
//!
//! `Storage` defines Evo's persistence boundary.
//! The boundary is intentionally minimal and infrastructure-only.

use std::cell::RefCell;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::errors::StorageError;

/// Canonical persisted object categories defined by the Architecture.
///
/// These variants model only persistence categories. They do not model
/// business behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageObjectKind {
    /// Immutable append-only Observation evidence log.
    Observation,

    /// Persistent Artifact identity state.
    Artifact,

    /// Persistent Workspace state.
    Workspace,

    /// Persistent Workspace Attachment state.
    Attachment,

    /// Persistent immutable Workspace Snapshot state.
    Snapshot,

    /// Persistent Knowledge state.
    Knowledge,

    /// Persistent Decision log state.
    Decision,

    /// Persistent derived Restoration understanding state.
    Restoration,
}

thread_local! {
    static THREAD_ROOT_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Guard that restores the previous thread-local storage root when dropped.
pub struct StorageRootGuard {
    previous: Option<PathBuf>,
}

/// Storage service boundary.
///
/// The boundary exposes a deterministic append/read model for canonical
/// persisted object categories.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Storage;

impl Storage {
    /// Constructs a storage boundary value.
    pub fn new() -> Self {
        Self
    }

    /// Sets the storage root for the current thread until the returned guard
    /// is dropped.
    pub fn with_thread_root(root: impl Into<PathBuf>) -> StorageRootGuard {
        let previous = THREAD_ROOT_OVERRIDE.with(|slot| slot.replace(Some(root.into())));
        StorageRootGuard { previous }
    }

    /// Appends an immutable record for a canonical object category.
    ///
    /// # Parameters
    ///
    /// - `kind`: canonical persistence category.
    /// - `record`: opaque serialized bytes for one persisted record.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::EmptyRecord`] when `record` is empty.
    pub fn append(&self, kind: StorageObjectKind, record: &[u8]) -> Result<(), StorageError> {
        if record.is_empty() {
            return Err(StorageError::EmptyRecord);
        }

        let path = record_path(kind);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| StorageError::BackendNotConfigured)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|_| StorageError::BackendNotConfigured)?;
        write_record(&mut file, record).map_err(|_| StorageError::BackendNotConfigured)?;
        file.sync_data()
            .map_err(|_| StorageError::BackendNotConfigured)?;
        Ok(())
    }

    /// Reads all persisted records for a canonical object category.
    ///
    /// A torn trailing record — a crash or power loss during an append that
    /// left the final record incomplete — is recovered by truncating the log
    /// to the last complete record. The incomplete bytes were never durably
    /// written (append syncs after each record), so recovering them is crash
    /// recovery, not history rewriting; the append-only guarantee for every
    /// completed record is unchanged. Genuine mid-log corruption is still
    /// reported as an error, never silently discarded.
    ///
    /// # Parameters
    ///
    /// - `kind`: canonical persistence category.
    ///
    /// # Errors
    pub fn read_all(&self, kind: StorageObjectKind) -> Result<Vec<Vec<u8>>, StorageError> {
        let path = record_path(kind);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let bytes = fs::read(&path).map_err(|_| StorageError::BackendNotConfigured)?;
        let mut records = Vec::new();
        let mut offset = 0usize;
        let mut last_good = 0usize;

        while offset < bytes.len() {
            // Each record is `len\n<bytes>\n`. A record that cannot be parsed
            // as a complete record is either a torn tail (the final write was
            // interrupted) or genuine mid-log corruption. Only a torn tail is
            // recovered.
            let Some(line_end) = find_newline(&bytes, offset) else {
                // The file ends inside a length line: torn tail.
                break;
            };
            let parsed = parse_length(&bytes[offset..line_end]);
            let Ok(record_len) = parsed else {
                // An unparsable length line that is followed by more complete
                // records is corruption, not a torn write: our writer never
                // writes a new record after an incomplete one. If nothing
                // complete follows, the tail was interrupted mid-length-line.
                if has_complete_record_after(&bytes, line_end + 1) {
                    return Err(StorageError::BackendNotConfigured);
                }
                break;
            };
            let after_line = line_end + 1;
            if record_len == 0 {
                // The writer never emits a zero-length record. A zero-length
                // record followed by complete records is corruption; alone at
                // the tail it is unrecoverable garbage and the log ends.
                if has_complete_record_after(&bytes, after_line) {
                    return Err(StorageError::BackendNotConfigured);
                }
                break;
            }
            let record_end = after_line + record_len;
            if record_end > bytes.len() {
                // The record body extends past the end of the file: torn tail.
                break;
            }
            if record_end == bytes.len() || bytes[record_end] != b'\n' {
                // The record terminator is missing. When more complete records
                // follow, the file was corrupted in the middle (the writer
                // never appends after an incomplete record); only a bare torn
                // tail is recovered.
                if record_end < bytes.len()
                    && has_complete_record_after(&bytes, record_end + 1)
                {
                    return Err(StorageError::BackendNotConfigured);
                }
                break;
            }
            records.push(bytes[after_line..record_end].to_vec());
            offset = record_end + 1;
            last_good = offset;
        }

        if last_good < bytes.len() {
            // Recover from the torn tail so the next append starts at a clean
            // record boundary.
            let file = OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(|_| StorageError::BackendNotConfigured)?;
            file.set_len(last_good as u64)
                .map_err(|_| StorageError::BackendNotConfigured)?;
            file.sync_data().map_err(|_| StorageError::BackendNotConfigured)?;
        }

        Ok(records)
    }

    /// Reads the records of a canonical object category that were appended
    /// at or after a known byte offset, returning them plus the byte offset
    /// just past the last complete record read.
    ///
    /// This is the incremental companion to [`Storage::read_all`]: a reader
    /// that has already consumed the log up to `offset` reads only the new
    /// records, so derived state can be updated without re-parsing the full
    /// history (ARCHITECTURE §2: derived views are cached for performance;
    /// the log itself stays the authoritative canonical evidence).
    ///
    /// # Recovery semantics
    ///
    /// - A torn trailing record (a final write interrupted mid-record) stops
    ///   the read at the last complete record and returns that position; the
    ///   reader retries the same region on its next call, so a live writer
    ///   completing the record is picked up, and a reader never truncates a
    ///   log another process may be actively appending to.
    /// - Genuine mid-log corruption (an unparsable or incomplete record
    ///   followed by complete records) is reported as an error, exactly like
    ///   [`Storage::read_all`].
    /// - When the log is shorter than `offset` (e.g. the writer recovered a
    ///   torn tail at startup), the read restarts from the beginning so no
    ///   record is missed.
    ///
    /// # Parameters
    ///
    /// - `kind`: canonical persistence category.
    /// - `offset`: the byte offset of the first not-yet-read record.
    ///
    /// # Errors
    pub fn read_from(
        &self,
        kind: StorageObjectKind,
        offset: u64,
    ) -> Result<(Vec<Vec<u8>>, u64), StorageError> {
        let path = record_path(kind);
        if !path.exists() {
            return Ok((Vec::new(), offset));
        }

        let bytes = fs::read(&path).map_err(|_| StorageError::BackendNotConfigured)?;
        let mut records = Vec::new();
        let mut current = offset as usize;
        let mut last_good = offset as usize;

        if current > bytes.len() {
            // The log was truncated below our position (torn-tail recovery by
            // the writer): re-read from the start so nothing is missed.
            current = 0;
            last_good = 0;
        }

        while current < bytes.len() {
            let Some(line_end) = find_newline(&bytes, current) else {
                break;
            };
            let parsed = parse_length(&bytes[current..line_end]);
            let Ok(record_len) = parsed else {
                if has_complete_record_after(&bytes, line_end + 1) {
                    return Err(StorageError::BackendNotConfigured);
                }
                break;
            };
            let after_line = line_end + 1;
            if record_len == 0 {
                if has_complete_record_after(&bytes, after_line) {
                    return Err(StorageError::BackendNotConfigured);
                }
                break;
            }
            let record_end = after_line + record_len;
            if record_end > bytes.len() {
                break;
            }
            if record_end == bytes.len() || bytes[record_end] != b'\n' {
                if record_end < bytes.len()
                    && has_complete_record_after(&bytes, record_end + 1)
                {
                    return Err(StorageError::BackendNotConfigured);
                }
                break;
            }
            records.push(bytes[after_line..record_end].to_vec());
            current = record_end + 1;
            last_good = current;
        }

        Ok((records, last_good as u64))
    }
}

/// Finds the first newline at or after `from`, returning its byte offset.
fn find_newline(bytes: &[u8], from: usize) -> Option<usize> {
    bytes[from..].iter().position(|byte| *byte == b'\n').map(|index| from + index)
}

/// Whether a complete record (`len\n<bytes>\n`) begins exactly at `from`.
///
/// Used to distinguish a torn trailing write from genuine mid-log corruption:
/// if a complete, valid record begins immediately after a parse failure, the log
/// was corrupted in the middle and the failure must be reported.
///
/// Deliberately does *not* scan forward for a well-formed record at some later
/// offset. Every caller passes the position directly after a suspicious point,
/// because the writer never appends past an incomplete record — so a complete
/// record can only legitimately begin at that exact boundary. Hunting for
/// record-shaped bytes at arbitrary offsets would guess, and would guess in the
/// dangerous direction: a false positive reports corruption and refuses to read
/// a log that is merely torn. Checking one boundary errs toward recovery.
fn has_complete_record_after(bytes: &[u8], from: usize) -> bool {
    if from >= bytes.len() {
        return false;
    }
    let Some(line_end) = find_newline(bytes, from) else {
        return false;
    };
    let Ok(record_len) = parse_length(&bytes[from..line_end]) else {
        return false;
    };
    if record_len == 0 {
        return false;
    }
    let record_end = line_end + 1 + record_len;
    record_end < bytes.len() && bytes[record_end] == b'\n'
}

/// Parses a length-prefix line (without the trailing newline).
fn parse_length(line: &[u8]) -> Result<usize, StorageError> {
    let text = std::str::from_utf8(line).map_err(|_| StorageError::BackendNotConfigured)?;
    let trimmed = text.trim_end_matches(['\r', '\n']);
    trimmed
        .parse::<usize>()
        .map_err(|_| StorageError::BackendNotConfigured)
}

fn write_record(writer: &mut File, record: &[u8]) -> Result<(), std::io::Error> {
    writeln!(writer, "{}", record.len())?;
    writer.write_all(record)?;
    writer.write_all(b"\n")?;
    Ok(())
}

impl Drop for StorageRootGuard {
    fn drop(&mut self) {
        THREAD_ROOT_OVERRIDE.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}

fn record_path(kind: StorageObjectKind) -> PathBuf {
    storage_root().join(record_filename(kind))
}

fn storage_root() -> PathBuf {
    THREAD_ROOT_OVERRIDE
        .with(|slot| slot.borrow().clone())
        .unwrap_or_else(crate::root::canonical_storage_root)
}

fn record_filename(kind: StorageObjectKind) -> &'static str {
    match kind {
        StorageObjectKind::Observation => "observation.log",
        StorageObjectKind::Artifact => "artifact.log",
        StorageObjectKind::Workspace => "workspace.log",
        StorageObjectKind::Attachment => "attachment.log",
        StorageObjectKind::Snapshot => "snapshot.log",
        StorageObjectKind::Knowledge => "knowledge.log",
        StorageObjectKind::Decision => "decision.log",
        StorageObjectKind::Restoration => "restoration.log",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-storage-{label}-{nanos}"))
    }

    #[test]
    fn storage_new_and_default_are_equivalent() {
        let from_new = Storage::new();
        let from_default: Storage = Default::default();
        assert_eq!(from_new, from_default);
    }

    #[test]
    fn append_rejects_empty_records() {
        let _guard = Storage::with_thread_root(unique_root("append-empty"));
        let storage = Storage::new();
        let error = storage
            .append(StorageObjectKind::Observation, &[])
            .unwrap_err();
        assert_eq!(error, StorageError::EmptyRecord);
    }

    #[test]
    fn append_persists_non_empty_records() {
        let _guard = Storage::with_thread_root(unique_root("append-non-empty"));
        let storage = Storage::new();
        storage
            .append(StorageObjectKind::Observation, b"record")
            .unwrap();
    }

    #[test]
    fn read_all_returns_all_persisted_records_for_kind() {
        let _guard = Storage::with_thread_root(unique_root("read-all"));
        let storage = Storage::new();
        storage
            .append(StorageObjectKind::Workspace, b"record-a")
            .unwrap();
        storage
            .append(StorageObjectKind::Workspace, b"record-b")
            .unwrap();

        let records = storage.read_all(StorageObjectKind::Workspace).unwrap();
        assert_eq!(records, vec![b"record-a".to_vec(), b"record-b".to_vec()]);
    }

    #[test]
    fn read_all_returns_empty_vector_for_missing_kind() {
        let _guard = Storage::with_thread_root(unique_root("read-missing"));
        let storage = Storage::new();
        let records = storage.read_all(StorageObjectKind::Decision).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn all_storage_object_kinds_are_constructible_and_distinct() {
        let kinds = [
            StorageObjectKind::Observation,
            StorageObjectKind::Artifact,
            StorageObjectKind::Workspace,
            StorageObjectKind::Attachment,
            StorageObjectKind::Snapshot,
            StorageObjectKind::Knowledge,
            StorageObjectKind::Decision,
            StorageObjectKind::Restoration,
        ];

        assert_eq!(kinds.len(), 8);
        for (index, kind) in kinds.iter().enumerate() {
            assert_eq!(kind, &kinds[index]);
        }
        assert_ne!(StorageObjectKind::Observation, StorageObjectKind::Artifact);
        assert_ne!(StorageObjectKind::Workspace, StorageObjectKind::Snapshot);
        assert_ne!(StorageObjectKind::Knowledge, StorageObjectKind::Decision);
    }

    #[test]
    fn torn_record_body_is_recovered_by_truncation() {
        let root = unique_root("torn-body");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
            storage.append(StorageObjectKind::Observation, b"record-b").unwrap();
            // Simulate a crash mid-write: a length line whose body was never
            // durably written.
            let path = root.join("observation.log");
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(b"999\npart").unwrap();
        }

        let _guard = Storage::with_thread_root(root.clone());
        let storage = Storage::new();
        let records = storage.read_all(StorageObjectKind::Observation).unwrap();
        // The complete records survive; the torn tail is dropped and the log
        // is truncated so the next append starts clean.
        assert_eq!(records, vec![b"record-a".to_vec(), b"record-b".to_vec()]);
        storage.append(StorageObjectKind::Observation, b"record-c").unwrap();
        let records = storage.read_all(StorageObjectKind::Observation).unwrap();
        assert_eq!(
            records,
            vec![
                b"record-a".to_vec(),
                b"record-b".to_vec(),
                b"record-c".to_vec()
            ]
        );
    }

    #[test]
    fn torn_length_line_is_recovered() {
        let root = unique_root("torn-length");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
            let path = root.join("observation.log");
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            // The final write was interrupted inside the length line.
            file.write_all(b"12").unwrap();
        }

        let _guard = Storage::with_thread_root(root);
        let storage = Storage::new();
        let records = storage.read_all(StorageObjectKind::Observation).unwrap();
        assert_eq!(records, vec![b"record-a".to_vec()]);
    }

    #[test]
    fn mid_log_corruption_is_reported_not_discarded() {
        let root = unique_root("corruption");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
            let path = root.join("observation.log");
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            // Garbage mid-log followed by a complete record: the log is
            // corrupt in the middle, which must fail loudly — a torn tail is
            // only ever the final record.
            file.write_all(b"not-a-length\n3\nabc\n").unwrap();
        }

        let _guard = Storage::with_thread_root(root);
        let storage = Storage::new();
        assert!(storage.read_all(StorageObjectKind::Observation).is_err());
    }

    #[test]
    fn storage_records_survive_reloads_through_same_root() {
        let root = unique_root("reload");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage
                .append(StorageObjectKind::Artifact, b"artifact-a")
                .unwrap();
        }

        let _guard = Storage::with_thread_root(root);
        let storage = Storage::new();
        let records = storage.read_all(StorageObjectKind::Artifact).unwrap();
        assert_eq!(records, vec![b"artifact-a".to_vec()]);
    }

    #[test]
    fn read_from_tails_only_new_records() {
        let root = unique_root("tail-new");
        let _guard = Storage::with_thread_root(root.clone());
        let storage = Storage::new();

        storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
        storage.append(StorageObjectKind::Observation, b"record-b").unwrap();
        let (first, offset) = storage
            .read_from(StorageObjectKind::Observation, 0)
            .unwrap();
        assert_eq!(first, vec![b"record-a".to_vec(), b"record-b".to_vec()]);
        assert_eq!(offset, first_two_records_end());

        // Appending after the recorded offset must not re-read the earlier
        // records — only the new one is returned.
        storage.append(StorageObjectKind::Observation, b"record-c").unwrap();
        let (next, next_offset) = storage
            .read_from(StorageObjectKind::Observation, offset)
            .unwrap();
        assert_eq!(next, vec![b"record-c".to_vec()]);
        assert_eq!(next_offset, offset + record_bytes_len(b"record-c"));
        assert!(next_offset > offset);
    }

    #[test]
    fn read_from_restarts_when_the_log_shrinks_below_the_offset() {
        let root = unique_root("tail-shrink");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
        }
        // Simulate a writer-side torn-tail recovery that truncated the log
        // below the reader's recorded offset: the reader must restart from
        // the beginning rather than skipping the surviving records.
        let _guard = Storage::with_thread_root(root.clone());
        let storage = Storage::new();
        let (records, offset) = storage
            .read_from(StorageObjectKind::Observation, 10_000)
            .unwrap();
        assert_eq!(records, vec![b"record-a".to_vec()]);
        assert_eq!(offset, record_bytes_len(b"record-a"));
    }

    #[test]
    fn read_from_stops_at_a_torn_tail_without_truncating() {
        let root = unique_root("tail-torn");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
            let path = root.join("observation.log");
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(b"999\npart").unwrap();
        }

        let _guard = Storage::with_thread_root(root.clone());
        let storage = Storage::new();
        let (records, offset) = storage
            .read_from(StorageObjectKind::Observation, 0)
            .unwrap();
        // The complete record survives; the torn region is not advanced past.
        assert_eq!(records, vec![b"record-a".to_vec()]);
        assert_eq!(offset, record_bytes_len(b"record-a"));
        // The reader never truncates a log that a live writer may be
        // appending to; the writer's own recovery owns truncation.
        let bytes = fs::read(root.join("observation.log")).unwrap();
        assert!(bytes.len() as u64 > record_bytes_len(b"record-a"));
    }

    #[test]
    fn read_from_reports_mid_log_corruption_like_read_all() {
        let root = unique_root("tail-corruption");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
            let path = root.join("observation.log");
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            // Garbage mid-log followed by a complete record: corruption.
            file.write_all(b"not-a-length\n3\nabc\n").unwrap();
        }

        let _guard = Storage::with_thread_root(root);
        let storage = Storage::new();
        assert!(storage.read_from(StorageObjectKind::Observation, 0).is_err());
    }

    #[test]
    fn zero_length_record_followed_by_complete_record_is_corruption() {
        let root = unique_root("zero-length-corruption");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
            let path = root.join("observation.log");
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            // A zero-length record is never produced by the writer; one that
            // is followed by more records is corruption.
            file.write_all(b"0\n3\nxyz\n").unwrap();
        }

        let _guard = Storage::with_thread_root(root);
        let storage = Storage::new();
        assert!(storage.read_all(StorageObjectKind::Observation).is_err());
    }

    #[test]
    fn torn_length_line_at_end_is_still_recovered_by_read_from() {
        let root = unique_root("read-from-torn-length");
        {
            let _guard = Storage::with_thread_root(root.clone());
            let storage = Storage::new();
            storage.append(StorageObjectKind::Observation, b"record-a").unwrap();
            let path = root.join("observation.log");
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(b"12").unwrap();
        }

        let _guard = Storage::with_thread_root(root);
        let storage = Storage::new();
        let (records, offset) = storage
            .read_from(StorageObjectKind::Observation, 0)
            .unwrap();
        assert_eq!(records, vec![b"record-a".to_vec()]);
        assert_eq!(offset, record_bytes_len(b"record-a"));
    }

    /// Byte length of one `len\n<record>\n` encoding.
    fn record_bytes_len(record: &[u8]) -> u64 {
        let digits = record.len().to_string().len() as u64;
        digits + 1 + record.len() as u64 + 1
    }

    /// Byte offset just past the second complete record of the tail test.
    fn first_two_records_end() -> u64 {
        record_bytes_len(b"record-a") + record_bytes_len(b"record-b")
    }
}
