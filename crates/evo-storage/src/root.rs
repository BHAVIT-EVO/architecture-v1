//! Canonical storage-root resolution and safe copy-forward migration.
//!
//! Evo's canonical persistent state must survive restart, reboot, and temp
//! cleanup, so the canonical root is the application's persistent Application
//! Support location — never the temp directory (BE-TRACE-0001 §3.4). Builds
//! before this move kept state in `{temp}/evo-storage`; when such a root
//! exists, it is migrated forward safely and never deleted.

use crate::errors::StorageError;

use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The one canonical storage root shared by the desktop shell and the daemon
/// runtime.
///
/// `EVO_STORAGE_ROOT` overrides the location for developer/testing purposes
/// only; otherwise the root is the application's persistent Application
/// Support location — never the temp directory, which cannot survive
/// restart, reboot, or temp cleanup.
pub fn canonical_storage_root() -> PathBuf {
    std::env::var_os("EVO_STORAGE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(default_storage_root)
}

/// The default persistent root: the application's Application Support
/// directory (`~/Library/Application Support/Evo/storage`), the same
/// directory family the first-run marker already uses. Falls back to the
/// legacy temp location only when no home directory exists at all (which is
/// not a persistent location, and is documented as such).
pub fn default_storage_root() -> PathBuf {
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Evo")
            .join("storage"),
        None => legacy_storage_root(),
    }
}

/// The root used by builds before canonical state moved out of the temp
/// directory. Never written by this build; preserved as the migration
/// source only.
pub fn legacy_storage_root() -> PathBuf {
    std::env::temp_dir().join("evo-storage")
}

/// What storage-root preparation did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// No legacy root existed; nothing was migrated.
    None,
    /// Canonical state already existed; the legacy root was left untouched.
    AlreadyPresent,
    /// The legacy root's files were copied forward and verified.
    Migrated { files: usize },
}

/// Prepares the canonical storage root for use, migrating a populated legacy
/// temp-dir root forward when one exists and the destination does not.
///
/// Safe copy-forward (BE-TRACE-0001 §3.4):
///
/// - the legacy root is never deleted, modified, or written;
/// - every copied file is verified (re-read at the destination, byte length
///   compared) inside a staging directory, and the destination only becomes
///   populated by an atomic rename of the fully verified copy;
/// - an already-populated canonical root is never merged or overwritten;
/// - the operation is idempotent: repeated startup never duplicates or
///   corrupts records;
/// - a failure removes only the staging directory, preserves the source, and
///   reports honestly — nothing is installed half-verified.
///
/// An explicit `EVO_STORAGE_ROOT` override skips migration: the developer
/// pointed at a root deliberately, so no legacy temp data is touched.
pub fn prepare_storage_root() -> Result<PathBuf, StorageError> {
    let canonical = canonical_storage_root();
    if std::env::var_os("EVO_STORAGE_ROOT").is_some() {
        return Ok(canonical);
    }
    migrate_legacy_root(&legacy_storage_root(), &canonical)?;
    Ok(canonical)
}

/// Migrates one legacy root forward into `canonical`, when needed. Pure
/// function of its inputs, so tests exercise it without touching real
/// directories. `canonical` must sit on the same filesystem as its parent
/// for the verified copy to be renamed into place atomically.
pub fn migrate_legacy_root(
    legacy: &Path,
    canonical: &Path,
) -> Result<MigrationOutcome, StorageError> {
    if !legacy.is_dir() {
        return Ok(MigrationOutcome::None);
    }
    if destination_populated(canonical) {
        return Ok(MigrationOutcome::AlreadyPresent);
    }
    let parent = canonical.parent().ok_or_else(|| StorageError::Migration {
        source: "canonical storage root has no parent directory".to_string(),
    })?;
    fs::create_dir_all(parent)
        .map_err(|err| migration_error("create the canonical root's parent", &err))?;
    // Unique per canonical root, so concurrent migrations of different roots
    // (parallel tests, or a desktop and a standalone daemon) never share
    // scratch space.
    let staging = parent.join(format!(
        ".evo-migrating-{}",
        canonical.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()
    ));
    // A stale staging directory from a crashed run is this crate's own
    // scratch space, never the user's data; discard it and start clean.
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|err| migration_error("create the staging directory", &err))?;
    match copy_tree(legacy, &staging) {
        Ok(files) => {
            // Atomic hand-off: the destination becomes populated only by
            // renaming the fully verified copy into place.
            fs::rename(&staging, canonical)
                .map_err(|err| migration_error("install the migrated root", &err))?;
            Ok(MigrationOutcome::Migrated { files })
        }
        Err(err) => {
            // Nothing was installed; the source is untouched. Remove only
            // the staging scratch and report honestly.
            let _ = fs::remove_dir_all(&staging);
            Err(err)
        }
    }
}

/// Whether a directory already holds any entry.
fn destination_populated(path: &Path) -> bool {
    match fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_some(),
        // Unreadable or absent: treated as empty so migration may proceed or
        // fail with an honest error rather than guessing.
        Err(_) => false,
    }
}

/// Recursively copies a directory tree, verifying every copied file by
/// re-reading it at the destination and comparing byte length to the source.
/// Returns the number of files copied.
fn copy_tree(from: &Path, to: &Path) -> Result<usize, StorageError> {
    fs::create_dir_all(to)
        .map_err(|err| migration_error(&format!("create {}", to.display()), &err))?;
    let mut files = 0usize;
    let entries = fs::read_dir(from)
        .map_err(|err| migration_error(&format!("read {}", from.display()), &err))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| migration_error(&format!("read {}", from.display()), &err))?;
        let source_path = entry.path();
        let dest_path = to.join(entry.file_name());
        if source_path.is_dir() {
            files += copy_tree(&source_path, &dest_path)?;
        } else if source_path.is_file() {
            copy_file_verified(&source_path, &dest_path)?;
            files += 1;
        } else {
            // A dangling symlink or other unreadable entry: fail honestly
            // rather than silently skipping part of the source.
            return Err(StorageError::Migration {
                source: format!(
                    "unsupported entry in migration source: {}",
                    source_path.display()
                ),
            });
        }
    }
    Ok(files)
}

/// Copies one file and verifies the copy is readable and complete: the
/// destination is re-read and its byte length compared to the source.
fn copy_file_verified(from: &Path, to: &Path) -> Result<(), StorageError> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| migration_error(&format!("create {}", parent.display()), &err))?;
    }
    fs::copy(from, to)
        .map_err(|err| migration_error(&format!("copy {}", from.display()), &err))?;
    let source_len = fs::metadata(from)
        .map_err(|err| migration_error(&format!("read metadata of {}", from.display()), &err))?
        .len();
    let copied_len = fs::metadata(to)
        .map_err(|err| migration_error(&format!("read metadata of {}", to.display()), &err))?
        .len();
    if source_len != copied_len {
        return Err(StorageError::Migration {
            source: format!(
                "verified copy length mismatch for {} ({} != {})",
                from.display(),
                source_len,
                copied_len
            ),
        });
    }
    // Re-read the destination so an unreadable copy is caught here, at
    // migration time, rather than at first use.
    let mut buffer = Vec::new();
    fs::File::open(to)
        .and_then(|mut file| file.read_to_end(&mut buffer))
        .map_err(|err| migration_error(&format!("verify readability of {}", to.display()), &err))?;
    if buffer.len() as u64 != source_len {
        return Err(StorageError::Migration {
            source: format!(
                "verified copy is unreadable or incomplete for {}",
                from.display()
            ),
        });
    }
    Ok(())
}

fn migration_error(action: &str, err: &std::io::Error) -> StorageError {
    StorageError::Migration {
        source: format!("{action}: {err}"),
    }
}

/// A storage root that fabricated Observations may safely be written to.
///
/// Developer tooling, demonstrations, and scenario harnesses invent
/// Observations. Those Observations travel the *same* acceptance path as real
/// ones and are byte-for-byte indistinguishable once persisted — no flag
/// survives, because the log format deliberately has nowhere to put one. So the
/// isolation cannot be a filter applied after the fact; it has to be a refusal
/// to write in the first place.
///
/// This function is that refusal. It returns a root only when
/// `EVO_STORAGE_ROOT` names somewhere other than the canonical location, and
/// errors otherwise. Any tool that fabricates Observations must obtain its root
/// here rather than from [`canonical_storage_root`].
///
/// It exists because the rule was previously only a convention, and the
/// convention failed: an example that fabricated two window titles resolved
/// [`canonical_storage_root`] by default and wrote them into a real person's
/// history, where they became a Workspace on the Home surface indistinguishable
/// from their real work.
///
/// # Errors
///
/// Returns [`StorageError::FabricationAgainstCanonicalRoot`] when no override is
/// set, or when the override resolves to the canonical root.
pub fn fabrication_root() -> Result<PathBuf, StorageError> {
    decide_fabrication_root(
        std::env::var_os("EVO_STORAGE_ROOT").as_deref(),
        &default_storage_root(),
    )
}

/// The decision behind [`fabrication_root`], taken as a pure function of its
/// inputs so tests can prove the refusal without mutating process environment
/// shared with every other test in the binary.
fn decide_fabrication_root(
    override_root: Option<&OsStr>,
    canonical: &Path,
) -> Result<PathBuf, StorageError> {
    let Some(override_root) = override_root else {
        return Err(StorageError::FabricationAgainstCanonicalRoot);
    };
    if override_root.is_empty() {
        return Err(StorageError::FabricationAgainstCanonicalRoot);
    }
    let requested = PathBuf::from(override_root);
    if same_location(&requested, canonical) {
        return Err(StorageError::FabricationAgainstCanonicalRoot);
    }
    Ok(requested)
}

/// Whether two paths name the same location, comparing resolved paths when both
/// exist and lexical paths otherwise (so a not-yet-created scratch directory is
/// still comparable).
fn same_location(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-root-{label}-{nanos}"))
    }

    fn write(root: &Path, relative: &str, content: &[u8]) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn read(root: &Path, relative: &str) -> Vec<u8> {
        std::fs::read(root.join(relative)).unwrap()
    }

    fn clear_dir(root: &Path) {
        if root.exists() {
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    // No legacy root exists: nothing to migrate, and no root is fabricated.
    #[test]
    fn no_legacy_root_means_nothing_to_migrate() {
        let legacy = unique_dir("no-legacy");
        let canonical = unique_dir("no-legacy-canon");
        clear_dir(&legacy);
        clear_dir(&canonical);
        let outcome = migrate_legacy_root(&legacy, &canonical).unwrap();
        assert_eq!(outcome, MigrationOutcome::None);
        assert!(!canonical.exists());
        clear_dir(&legacy);
        clear_dir(&canonical);
    }

    // An empty legacy root migrates to an empty canonical root without error.
    #[test]
    fn empty_legacy_root_migrates_cleanly() {
        let legacy = unique_dir("empty-legacy");
        let canonical = unique_dir("empty-canon");
        clear_dir(&legacy);
        clear_dir(&canonical);
        std::fs::create_dir_all(&legacy).unwrap();
        let outcome = migrate_legacy_root(&legacy, &canonical).unwrap();
        assert_eq!(outcome, MigrationOutcome::Migrated { files: 0 });
        assert!(canonical.is_dir());
        // The legacy root is preserved, not deleted.
        assert!(legacy.is_dir());
        clear_dir(&legacy);
        clear_dir(&canonical);
    }

    // A populated legacy root is copied forward byte-for-byte, nested
    // directories included, and the source is preserved untouched.
    #[test]
    fn populated_legacy_root_is_copied_and_verified() {
        let legacy = unique_dir("pop-legacy");
        let canonical = unique_dir("pop-canon");
        clear_dir(&legacy);
        clear_dir(&canonical);
        let observation = b"EVO-OBSERVATION-SNAPSHOT v1\nsecond record\n";
        let workspace = b"EVO-WORKSPACE\n";
        write(&legacy, "observation.log", observation);
        write(&legacy, "workspace.log", workspace);
        write(&legacy, "nested/deeper/artifact.log", b"artifact-record");
        let outcome = migrate_legacy_root(&legacy, &canonical).unwrap();
        assert_eq!(outcome, MigrationOutcome::Migrated { files: 3 });
        // Observation content is preserved exactly.
        assert_eq!(read(&canonical, "observation.log"), observation);
        assert_eq!(read(&canonical, "workspace.log"), workspace);
        assert_eq!(
            read(&canonical, "nested/deeper/artifact.log"),
            b"artifact-record"
        );
        // The source is never deleted or modified.
        assert_eq!(read(&legacy, "observation.log"), observation);
        clear_dir(&legacy);
        clear_dir(&canonical);
    }

    // An already-populated destination is never merged or overwritten: the
    // destination's records win, and the legacy root is left untouched.
    #[test]
    fn an_already_populated_destination_is_left_untouched() {
        let legacy = unique_dir("dest-legacy");
        let canonical = unique_dir("dest-canon");
        clear_dir(&legacy);
        clear_dir(&canonical);
        write(&legacy, "observation.log", b"legacy-record");
        write(&canonical, "observation.log", b"canonical-record");
        let outcome = migrate_legacy_root(&legacy, &canonical).unwrap();
        assert_eq!(outcome, MigrationOutcome::AlreadyPresent);
        assert_eq!(read(&canonical, "observation.log"), b"canonical-record");
        assert_eq!(read(&legacy, "observation.log"), b"legacy-record");
        clear_dir(&legacy);
        clear_dir(&canonical);
    }

    // Repeated startup is idempotent: the second run sees canonical state
    // already present and never duplicates or corrupts records.
    #[test]
    fn migration_is_idempotent_across_repeated_startup() {
        let legacy = unique_dir("idem-legacy");
        let canonical = unique_dir("idem-canon");
        clear_dir(&legacy);
        clear_dir(&canonical);
        write(&legacy, "observation.log", b"record-one\nrecord-two\n");
        assert_eq!(
            migrate_legacy_root(&legacy, &canonical).unwrap(),
            MigrationOutcome::Migrated { files: 1 }
        );
        assert_eq!(
            migrate_legacy_root(&legacy, &canonical).unwrap(),
            MigrationOutcome::AlreadyPresent
        );
        // Exactly the original records, exactly once.
        assert_eq!(read(&canonical, "observation.log"), b"record-one\nrecord-two\n");
        clear_dir(&legacy);
        clear_dir(&canonical);
    }

    // A corrupt/incomplete migration source fails honestly: the source is
    // preserved, nothing is installed at the destination, and no staging
    // scratch is left behind. A dangling symlink is an entry that can
    // neither be read nor copied.
    #[cfg(unix)]
    #[test]
    fn a_failed_migration_preserves_the_source_and_installs_nothing() {
        use std::os::unix::fs::symlink;

        let legacy = unique_dir("corrupt-legacy");
        let canonical = unique_dir("corrupt-canon");
        clear_dir(&legacy);
        clear_dir(&canonical);
        write(&legacy, "observation.log", b"good-record");
        // A dangling symlink: not a file, not a directory — unreadable.
        symlink("/nonexistent/target", legacy.join("broken.log")).unwrap();
        let err = migrate_legacy_root(&legacy, &canonical).unwrap_err();
        assert!(matches!(err, StorageError::Migration { .. }));
        assert!(err.to_string().contains("unsupported entry"));
        // The source is preserved exactly, including the good record.
        assert_eq!(read(&legacy, "observation.log"), b"good-record");
        // Nothing was installed at the destination, and the staging scratch
        // was removed. Scoped to *this* migration's staging directory: the
        // parent is the shared temp directory, where a concurrently running
        // test's staging directory may legitimately exist mid-flight.
        assert!(!canonical.exists());
        let staging = canonical.parent().unwrap().join(format!(
            ".evo-migrating-{}",
            canonical.file_name().unwrap().to_string_lossy()
        ));
        assert!(
            !staging.exists(),
            "staging scratch left behind: {}",
            staging.display()
        );
        clear_dir(&legacy);
        clear_dir(&canonical);
    }

    // prepare_storage_root resolves to the persistent Application Support
    // root, never the temp directory, when no override is set.
    #[test]
    fn default_root_is_the_persistent_application_support_location() {
        let root = default_storage_root();
        assert!(
            !root.starts_with(std::env::temp_dir()),
            "default root must not live under the temp directory"
        );
        assert!(root.to_string_lossy().contains("Application Support"));
        assert!(root.ends_with("storage"));
    }

    // Tooling that fabricates Observations gets nowhere to write unless it was
    // pointed somewhere deliberately. Silence is not consent: an unset override
    // means the tool would land on the person's real history.
    #[test]
    fn fabrication_is_refused_when_no_scratch_root_was_chosen() {
        let canonical = unique_dir("fab-canon");
        assert_eq!(
            decide_fabrication_root(None, &canonical),
            Err(StorageError::FabricationAgainstCanonicalRoot)
        );
        assert_eq!(
            decide_fabrication_root(Some(OsStr::new("")), &canonical),
            Err(StorageError::FabricationAgainstCanonicalRoot)
        );
    }

    // Naming the canonical root explicitly is refused too. The guard is about
    // where the writes land, not about whether someone typed a variable.
    #[test]
    fn fabrication_is_refused_against_the_canonical_root_itself() {
        let canonical = unique_dir("fab-explicit");
        clear_dir(&canonical);
        assert_eq!(
            decide_fabrication_root(Some(canonical.as_os_str()), &canonical),
            Err(StorageError::FabricationAgainstCanonicalRoot)
        );
        // And when the directory exists, so the two paths resolve rather than
        // merely comparing as strings.
        std::fs::create_dir_all(&canonical).unwrap();
        let indirect = canonical.join("..").join(canonical.file_name().unwrap());
        assert_eq!(
            decide_fabrication_root(Some(indirect.as_os_str()), &canonical),
            Err(StorageError::FabricationAgainstCanonicalRoot),
            "a path that resolves to the canonical root is the canonical root"
        );
        clear_dir(&canonical);
    }

    // A genuinely separate scratch root is allowed, including one that does not
    // exist yet — the harness creates it.
    #[test]
    fn fabrication_is_allowed_against_a_separate_scratch_root() {
        let canonical = unique_dir("fab-real");
        let scratch = unique_dir("fab-scratch");
        assert_eq!(
            decide_fabrication_root(Some(scratch.as_os_str()), &canonical),
            Ok(scratch.clone())
        );
        assert!(!scratch.exists(), "the guard must not create anything");
    }
}
