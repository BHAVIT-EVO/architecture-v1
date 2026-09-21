//! Live macOS FSEvents producer for `FileSaved` and `CommitMade`.
//!
//! This source witnesses two platform realities through the filesystem
//! journal (FSEvents) rather than Accessibility:
//!
//! - A regular file under the user's home directory being modified is the
//!   platform's witness of a save: the file's content was written. The
//!   Collector boundary (IS-0020 §10) preserves that directly observed fact
//!   as a `FileSaved` signal whose subject is the witnessed path.
//! - A git repository's `.git/logs/HEAD` reflog being appended is the
//!   platform's witness of a HEAD transition. The reflog entry itself names
//!   the new commit; only entries whose activity message begins with
//!   "commit" produce a `CommitMade` signal (checkouts, resets, and rebases
//!   are different witnessed facts and are never mislabeled as commits).
//!
//! This source never interprets, ranks, or decides importance. Events that
//! are not a witnessed file write or commit produce no signal. The watched
//! root is the user's home directory; hidden paths and the home `Library`
//! area are excluded from `FileSaved` (capture scope to bound volume, not a
//! semantic filter — the semantics of every emitted fact are unchanged).

use crate::adapters::macos::MacOSSignal;
use crate::macos_event_source::MacOSEventSourceError;

use std::ffi::c_char;
use std::os::raw::c_void;
use std::path::{Component, Path};
use std::ptr;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::SystemTime;

// FSEventStreamEventFlags (stable, platform-independent bit values).
const FLAG_ITEM_CREATED: u32 = 0x0000_0100;
const FLAG_ITEM_MODIFIED: u32 = 0x0000_1000;
const FLAG_ITEM_IS_FILE: u32 = 0x0001_0000;
const FLAG_ITEM_IS_DIR: u32 = 0x0002_0000;
const FLAG_ITEM_IS_SYMLINK: u32 = 0x0004_0000;

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    type CFTypeRef = *const c_void;
    type CFArrayRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFRunLoopRef = *const c_void;
    type FSEventStreamRef = *const c_void;

    const CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;

    // FSEventStreamCreateFlags
    const CREATE_USE_CF_TYPES: u32 = 0x0000_0001;
    const CREATE_NO_DEFER: u32 = 0x0000_0002;
    const CREATE_FILE_EVENTS: u32 = 0x0000_0010;

    const EVENT_ID_SINCE_NOW: u64 = u64::MAX;

    #[repr(C)]
    struct FSEventStreamContext {
        version: isize,
        info: *mut c_void,
        retain: *const c_void,
        release: *const c_void,
        copy_description: *const c_void,
    }

    type FSEventStreamCallback = extern "C" fn(
        stream_ref: FSEventStreamRef,
        info: *mut c_void,
        num_events: usize,
        event_paths: *const c_void,
        event_flags: *const u32,
        event_ids: *const u64,
    );

    /// The callback holder lives for the stream's lifetime on the watcher
    /// thread and carries the capture scope plus the signal callback.
    struct CallbackHolder {
        home: String,
        callback: Box<dyn FnMut(MacOSSignal)>,
    }

    #[link(name = "CoreServices", kind = "framework")]
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn FSEventStreamCreate(
            allocator: CFTypeRef,
            callback: FSEventStreamCallback,
            context: *mut FSEventStreamContext,
            paths_to_watch: CFArrayRef,
            since_when: u64,
            latency: f64,
            flags: u32,
        ) -> FSEventStreamRef;
        fn FSEventStreamScheduleWithRunLoop(
            stream: FSEventStreamRef,
            run_loop: CFRunLoopRef,
            mode: CFStringRef,
        );
        fn FSEventStreamStart(stream: FSEventStreamRef) -> i8;
        fn FSEventStreamInvalidate(stream: FSEventStreamRef);
        fn FSEventStreamRelease(stream: FSEventStreamRef);
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopRun();
        fn CFRunLoopStop(run_loop: CFRunLoopRef);
        fn CFArrayCreate(
            allocator: CFTypeRef,
            values: *const *const c_void,
            num_values: isize,
            callbacks: *const c_void,
        ) -> CFArrayRef;
        fn CFArrayGetCount(array: CFArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CFArrayRef, index: isize) -> *const c_void;
        fn CFStringCreateWithCString(
            alloc: CFTypeRef,
            c_str: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        fn CFStringGetLength(value: CFStringRef) -> isize;
        fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
        fn CFStringGetCString(
            the_string: CFStringRef,
            buffer: *mut c_char,
            buffer_size: isize,
            encoding: u32,
        ) -> i8;
        fn CFRelease(cf: CFTypeRef);
        fn CFRetain(cf: CFTypeRef) -> CFTypeRef;

        static kCFRunLoopDefaultMode: CFStringRef;
    }

    /// A `CFRunLoopRef` transported between threads (raw pointers are not
    /// `Send`; the run loop object itself is retained and thread-safe for
    /// `CFRunLoopStop`).
    struct RunLoop(usize);

    // SAFETY: the run loop reference is retained by the sender before
    // transmission and released by the receiver on drop; `CFRunLoopStop`
    // may be invoked from another thread on a run loop object.
    unsafe impl Send for RunLoop {}

    /// Keepalive for the live FSEvents capture thread.
    pub struct FSEventsWatcher {
        run_loop: usize,
        handle: Option<JoinHandle<()>>,
    }

    impl FSEventsWatcher {
        /// Starts a background FSEvents stream over the user's home directory.
        ///
        /// The callback receives `FileSaved` and `CommitMade` signals. The
        /// stream requires no Accessibility permission; it witnesses the
        /// filesystem journal directly.
        pub fn start<F>(callback: F) -> Result<Self, MacOSEventSourceError>
        where
            F: FnMut(MacOSSignal) + Send + 'static,
        {
            let home = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .ok_or(MacOSEventSourceError::RuntimeSetupFailed("HOME lookup"))?;
            let home_str = home.to_string_lossy().into_owned();

            let (run_loop_sender, run_loop_receiver) = mpsc::channel::<RunLoop>();
            let handle = thread::spawn(move || {
                unsafe {
                    let run_loop = CFRunLoopGetCurrent();
                    let _ = run_loop_sender.send(RunLoop(CFRetain(run_loop) as usize));

                    // The holder lives on this thread for the whole stream
                    // lifetime; the stream context points at it.
                    let mut holder = CallbackHolder {
                        home: home_str,
                        callback: Box::new(callback),
                    };
                    let mut context = FSEventStreamContext {
                        version: 0,
                        info: &mut holder as *mut CallbackHolder as *mut c_void,
                        retain: ptr::null(),
                        release: ptr::null(),
                        copy_description: ptr::null(),
                    };

                    let (paths, path_string) = create_paths_array(&holder.home);
                    if paths.is_null() {
                        return;
                    }
                    let stream = FSEventStreamCreate(
                        ptr::null(),
                        stream_callback,
                        &mut context,
                        paths,
                        EVENT_ID_SINCE_NOW,
                        0.5,
                        CREATE_USE_CF_TYPES | CREATE_NO_DEFER | CREATE_FILE_EVENTS,
                    );
                    CFRelease(paths);
                    // The array may not retain its values (NULL callbacks), so
                    // the path string must stay alive until after the stream
                    // has consumed the array.
                    CFRelease(path_string);
                    if stream.is_null() {
                        return;
                    }

                    FSEventStreamScheduleWithRunLoop(stream, run_loop, kCFRunLoopDefaultMode);
                    FSEventStreamStart(stream);
                    CFRunLoopRun();
                    FSEventStreamInvalidate(stream);
                    FSEventStreamRelease(stream);
                }
            });

            // Wait for the thread to publish the retained run loop so the
            // watcher can stop it on drop. The thread sends it before any
            // fallible step, so this cannot block indefinitely.
            let run_loop = run_loop_receiver
                .recv()
                .map_err(|_| MacOSEventSourceError::RuntimeSetupFailed("FSEvents thread"))?
                .0;
            Ok(Self {
                run_loop,
                handle: Some(handle),
            })
        }
    }

    impl Drop for FSEventsWatcher {
        fn drop(&mut self) {
            unsafe {
                if self.run_loop != 0 {
                    // CFRunLoopStop only breaks a run loop that is already
                    // running. `start` publishes the run loop before its
                    // thread enters CFRunLoopRun, so a stop that lands in
                    // that window is lost and a single call would leave
                    // join() waiting forever. Keep stopping until the
                    // thread is out; whenever it enters the loop, the next
                    // call (within a few milliseconds) breaks it.
                    if let Some(handle) = self.handle.take() {
                        while !handle.is_finished() {
                            CFRunLoopStop(self.run_loop as CFRunLoopRef);
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        let _ = handle.join();
                    }
                    CFRelease(self.run_loop as CFTypeRef);
                    self.run_loop = 0;
                }
            }
        }
    }

    extern "C" fn stream_callback(
        _stream: FSEventStreamRef,
        info: *mut c_void,
        num_events: usize,
        event_paths: *const c_void,
        event_flags: *const u32,
        _event_ids: *const u64,
    ) {
        if info.is_null() {
            return;
        }
        let holder = unsafe { &mut *(info as *mut CallbackHolder) };
        let paths = event_paths as CFArrayRef;
        if paths.is_null() {
            return;
        }

        unsafe {
            let count = CFArrayGetCount(paths);
            for index in 0..count {
                if index as usize >= num_events {
                    break;
                }
                let path_value = CFArrayGetValueAtIndex(paths, index) as CFStringRef;
                let flags = *event_flags.add(index as usize);
                let Some(path) = cfstring_to_string(path_value) else {
                    continue;
                };
                emit_event_signals(
                    classify_event_path(&path, flags, &holder.home),
                    &path,
                    &mut holder.callback,
                );
            }
        }
    }

    /// Emits the ordered canonical signals for one witnessed FSEvents event.
    ///
    /// RFC-0012: a file save or commit inside a repository witnesses the
    /// repository-membership fact alongside the content fact. The membership
    /// witness MUST precede the content Observation: Workspace Formation
    /// decides Workspace assignment from persisted canonical evidence, so a
    /// resource whose membership arrives only after its own save would be
    /// assigned to a solo Workspace before the co-membership exists. Both
    /// facts are witnessed at the same instant; only the emission order
    /// differs.
    pub(crate) fn emit_event_signals(
        event: FsSignal,
        witnessed_path: &str,
        callback: &mut dyn FnMut(MacOSSignal),
    ) {
        match event {
            FsSignal::FileSaved { path } => {
                let observed_at = SystemTime::now();
                // This is WITNESSING: the git/filesystem resolution runs only
                // at capture time; Formation consumes the persisted canonical
                // Observation.
                if let Some(repository) = crate::repo_identity::
                    repository_identity_for_path(std::path::Path::new(&path))
                {
                    (callback)(MacOSSignal::RepositoryMembership {
                        member: path.clone(),
                        repository,
                        observed_at,
                    });
                }
                (callback)(MacOSSignal::FileSaved {
                    subject: path,
                    observed_at,
                });
            }
            FsSignal::CommitMade { hash } => {
                let observed_at = SystemTime::now();
                // A witnessed commit (reflog append) is a member of the
                // repository whose reflog witnessed it. The reflog path
                // identifies the repository; resolve its identity at capture
                // time. The membership witness precedes the commit
                // Observation for the same reason as above.
                if let Some(repository) = crate::repo_identity::
                    repository_identity_from_reflog_path(std::path::Path::new(witnessed_path))
                {
                    (callback)(MacOSSignal::RepositoryMembership {
                        member: hash.clone(),
                        repository,
                        observed_at,
                    });
                }
                (callback)(MacOSSignal::CommitMade {
                    subject: hash,
                    observed_at,
                });
            }
            FsSignal::None => {}
        }
    }

    /// Creates the paths array for the stream. Returns the array and the path
    /// string it references; the caller must release both. The string is kept
    /// alive by the caller because `CFArrayCreate` with NULL callbacks does
    /// not guarantee retention of its values.
    unsafe fn create_paths_array(path: &str) -> (CFArrayRef, CFStringRef) {
        unsafe {
            // CFStringCreateWithCString requires a NUL-terminated C string;
            // a Rust `&str` is not terminated, so an explicit CString is
            // required (reading past the buffer would corrupt the CFString).
            let c_path = match std::ffi::CString::new(path) {
                Ok(value) => value,
                Err(_) => return (ptr::null(), ptr::null()),
            };
            let cf_string = CFStringCreateWithCString(
                ptr::null(),
                c_path.as_ptr() as *const c_char,
                CFSTRING_ENCODING_UTF8,
            );
            if cf_string.is_null() {
                return (ptr::null(), ptr::null());
            }
            let mut values = [cf_string as *const c_void];
            let array = CFArrayCreate(ptr::null(), values.as_mut_ptr(), 1, ptr::null());
            (array, cf_string)
        }
    }

    unsafe fn cfstring_to_string(value: CFStringRef) -> Option<String> {
        unsafe {
            if value.is_null() {
                return None;
            }
            let length = CFStringGetLength(value);
            if length <= 0 {
                return Some(String::new());
            }
            let max_size = CFStringGetMaximumSizeForEncoding(length, CFSTRING_ENCODING_UTF8) + 1;
            if max_size <= 0 {
                return None;
            }
            let mut buffer = vec![0u8; max_size as usize];
            let ok = CFStringGetCString(
                value,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len() as isize,
                CFSTRING_ENCODING_UTF8,
            );
            if ok == 0 {
                return None;
            }
            let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
            String::from_utf8(buffer[..end].to_vec()).ok()
        }
    }
}

/// The classification of one witnessed FSEvents file path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsSignal {
    /// A regular, non-hidden file under the home directory was modified.
    FileSaved { path: String },
    /// A git reflog records a genuine commit; the witnessed commit hash.
    CommitMade { hash: String },
    /// The event is not a witnessed save or commit; no signal.
    None,
}

/// Classifies one witnessed FSEvents path/flag pair into a canonical signal.
///
/// Pure and deterministic: the same path and flags always classify the same
/// way. This is the exact rule the live stream callback applies. `home` is
/// the user's home directory prefix used to scope `FileSaved`.
pub fn classify_event_path(path: &str, flags: u32, home: &str) -> FsSignal {
    let is_modified = flags & FLAG_ITEM_MODIFIED != 0;
    let is_created = flags & FLAG_ITEM_CREATED != 0;
    let is_file = flags & FLAG_ITEM_IS_FILE != 0;
    let is_dir = flags & FLAG_ITEM_IS_DIR != 0;
    let is_symlink = flags & FLAG_ITEM_IS_SYMLINK != 0;

    if is_modified && is_head_reflog_path(path) {
        // A HEAD reflog append is the platform witness of a HEAD
        // transition. The commit hash and activity are read from the
        // witnessed file itself, never guessed.
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Some(line) = text.lines().next_back() {
                if let Some(hash) = commit_hash_from_reflog_line(line) {
                    return FsSignal::CommitMade { hash };
                }
            }
        }
        return FsSignal::None;
    }

    if (is_modified || is_created)
        && is_file
        && !is_dir
        && !is_symlink
        && is_user_visible_path(path, home)
    {
        return FsSignal::FileSaved {
            path: path.to_string(),
        };
    }

    FsSignal::None
}

/// Whether a path is under the user's home directory and not hidden.
///
/// This is capture scope (which files the stream considers), not a semantic
/// judgment about importance: every path that passes is a genuine witnessed
/// write whose meaning is unchanged. Hidden paths and the home `Library`
/// area are excluded to bound the volume of events the stream emits.
pub fn is_user_visible_path(path: &str, home: &str) -> bool {
    let path = Path::new(path);
    let home = Path::new(home);
    if !path.starts_with(home) {
        return false;
    }
    let rest = path.strip_prefix(home).unwrap_or(path);
    for component in rest.components() {
        if let Component::Normal(name) = component {
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                return false;
            }
            if name == "Library" {
                return false;
            }
        }
    }
    true
}

/// Whether a witnessed path is a git HEAD reflog — the file whose append is
/// the platform witness of a HEAD transition.
///
/// This covers both the main working tree (`<repo>/.git/logs/HEAD`) and
/// linked worktrees, whose HEAD logs live under the repository's git dir at
/// `<repo>/.git/worktrees/<name>/logs/HEAD`. A commit made in a worktree
/// appends to the worktree's own HEAD log, so missing this path would drop
/// genuine CommitMade evidence.
pub fn is_head_reflog_path(path: &str) -> bool {
    path.ends_with("/.git/logs/HEAD")
        || (path.contains("/.git/worktrees/") && path.ends_with("/logs/HEAD"))
}

/// Extracts the new commit hash from one git reflog line, but only when the
/// reflog activity is a genuine commit (not a checkout, reset, or rebase).
///
/// Reflog line format (from `logs/HEAD`):
/// `<old> <new> <committer> <unixtime> <tz> <activity message>`
pub fn commit_hash_from_reflog_line(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }
    let new_hash = parts.get(1)?;
    if new_hash.len() != 40 || !new_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    // The timezone token is a `+/-HHMM` field that separates the committer
    // header from the activity message.
    let mut tz_index = None;
    for (index, part) in parts.iter().enumerate().skip(4) {
        if part.len() == 5
            && (part.starts_with('+') || part.starts_with('-'))
            && part[1..].chars().all(|c| c.is_ascii_digit())
        {
            tz_index = Some(index);
            break;
        }
    }
    let message = tz_index
        .map(|index| parts[index + 1..].join(" "))
        .unwrap_or_default();

    if message.trim_start().starts_with("commit") {
        Some(new_hash.to_string())
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::*;

    pub struct FSEventsWatcher;

    impl FSEventsWatcher {
        pub fn start<F>(_callback: F) -> Result<Self, MacOSEventSourceError>
        where
            F: FnMut(MacOSSignal) + Send + 'static,
        {
            Err(MacOSEventSourceError::UnsupportedPlatform)
        }
    }
}

pub use imp::FSEventsWatcher;

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/Users/alice";
    const MODIFIED_FILE: u32 = FLAG_ITEM_MODIFIED | FLAG_ITEM_IS_FILE;

    #[test]
    fn modified_visible_file_classifies_as_file_saved() {
        let signal = classify_event_path("/Users/alice/Documents/report.md", MODIFIED_FILE, HOME);
        assert_eq!(
            signal,
            FsSignal::FileSaved {
                path: "/Users/alice/Documents/report.md".into()
            }
        );
    }

    #[test]
    fn created_visible_file_classifies_as_file_saved() {
        let signal = classify_event_path(
            "/Users/alice/Desktop/notes.txt",
            FLAG_ITEM_CREATED | FLAG_ITEM_IS_FILE,
            HOME,
        );
        assert!(matches!(signal, FsSignal::FileSaved { .. }));
    }

    #[test]
    fn paths_outside_home_produce_no_file_signal() {
        assert_eq!(
            classify_event_path("/tmp/scratch.md", MODIFIED_FILE, HOME),
            FsSignal::None
        );
    }

    #[test]
    fn hidden_paths_produce_no_file_signal() {
        assert_eq!(
            classify_event_path("/Users/alice/.zshrc", MODIFIED_FILE, HOME),
            FsSignal::None
        );
        assert_eq!(
            classify_event_path("/Users/alice/proj/.git/config", MODIFIED_FILE, HOME),
            FsSignal::None
        );
    }

    #[test]
    fn home_library_paths_produce_no_file_signal() {
        assert_eq!(
            classify_event_path(
                "/Users/alice/Library/Caches/app.db",
                MODIFIED_FILE,
                HOME
            ),
            FsSignal::None
        );
    }

    #[test]
    fn non_library_directories_still_capture() {
        assert_eq!(
            classify_event_path("/Users/alice/Work/notes.txt", MODIFIED_FILE, HOME),
            FsSignal::FileSaved {
                path: "/Users/alice/Work/notes.txt".into()
            }
        );
    }

    #[test]
    fn directories_and_symlinks_are_not_file_saves() {
        assert_eq!(
            classify_event_path(
                "/Users/alice/Documents",
                FLAG_ITEM_MODIFIED | FLAG_ITEM_IS_DIR,
                HOME
            ),
            FsSignal::None
        );
        assert_eq!(
            classify_event_path(
                "/Users/alice/link",
                FLAG_ITEM_MODIFIED | FLAG_ITEM_IS_SYMLINK,
                HOME
            ),
            FsSignal::None
        );
    }

    #[test]
    fn commit_reflog_line_yields_commit_hash() {
        let line = "0000000000000000000000000000000000000000 abcdef0123456789abcdef0123456789abcdef01 Alice Example <alice@example.com> 1700000000 +0000\tcommit: add feature";
        assert_eq!(
            commit_hash_from_reflog_line(line).as_deref(),
            Some("abcdef0123456789abcdef0123456789abcdef01")
        );
    }

    #[test]
    fn checkout_reflog_line_is_not_a_commit() {
        let line = "abcdef0123456789abcdef0123456789abcdef01 0123456789abcdef0123456789abcdef01234567 Alice Example <alice@example.com> 1700000001 +0000\tcheckout: moving from main to feature";
        assert_eq!(commit_hash_from_reflog_line(line), None);
    }

    #[test]
    fn amend_and_merge_commits_are_still_commits() {
        let amend = "abcdef0123456789abcdef0123456789abcdef01 1111111111111111111111111111111111111111 Alice <a@b.c> 1700000002 +0000\tcommit (amend): tweak";
        assert!(commit_hash_from_reflog_line(amend).is_some());
        let merge = "abcdef0123456789abcdef0123456789abcdef01 2222222222222222222222222222222222222222 Alice <a@b.c> 1700000003 +0000\tcommit (merge): merge branch";
        assert!(commit_hash_from_reflog_line(merge).is_some());
    }

    #[test]
    fn head_reflog_paths_cover_main_and_worktrees() {
        assert!(is_head_reflog_path("/Users/alice/proj/.git/logs/HEAD"));
        assert!(is_head_reflog_path("/Users/alice/proj/.git/worktrees/feature/logs/HEAD"));
        assert!(!is_head_reflog_path("/Users/alice/proj/.git/logs/refs/heads/main"));
        assert!(!is_head_reflog_path("/Users/alice/proj/src/main.rs"));
        assert!(!is_head_reflog_path("/Users/alice/proj/.git/worktrees/feature/HEAD"));
    }

    #[test]
    fn worktree_commit_append_is_classified_as_commit_made() {
        // A real worktree HEAD reflog file, appended by a genuine commit.
        let dir = std::env::temp_dir().join(format!(
            "evo-wt-reflog-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // The witnessed path is the worktree's HEAD reflog under the
        // repository's git dir, exactly as FSEvents reports it.
        let path = dir.join(".git/worktrees/feature/logs/HEAD");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "0000000000000000000000000000000000000000 abcdef0123456789abcdef0123456789abcdef01 Alice <a@b.c> 1700000000 +0000\tcommit: worktree work\n",
        )
        .unwrap();
        let signal = classify_event_path(
            path.to_str().unwrap(),
            FLAG_ITEM_MODIFIED | FLAG_ITEM_IS_FILE,
            HOME,
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            signal,
            FsSignal::CommitMade {
                hash: "abcdef0123456789abcdef0123456789abcdef01".to_string()
            }
        );
    }

    #[test]
    fn malformed_reflog_lines_yield_nothing() {
        assert_eq!(commit_hash_from_reflog_line(""), None);
        assert_eq!(commit_hash_from_reflog_line("short"), None);
        assert_eq!(
            commit_hash_from_reflog_line("a b c d e +0000 commit: x"),
            None
        );
    }

    #[test]
    fn reflog_parse_with_spaces_in_committer_name() {
        let line = "0000000000000000000000000000000000000000 3333333333333333333333333333333333333333 Jane Q. Public <jane@example.com> 1700000004 -0700\tcommit: add docs";
        assert_eq!(
            commit_hash_from_reflog_line(line).as_deref(),
            Some("3333333333333333333333333333333333333333")
        );
    }

    // Workspace Formation consumes persisted canonical evidence, so the
    // capture boundary must witness repository membership BEFORE the content
    // signal: a file whose membership arrives only after its own save would be
    // assigned to a solo Workspace before the co-membership exists. Both facts
    // are witnessed at the same instant; only the emission order differs.
    #[cfg(target_os = "macos")]
    #[test]
    fn membership_witness_precedes_content_signal() {
        use crate::adapters::macos::MacOSSignal;

        // A real repository — the same git resolution Formation relies on.
        let dir = std::env::temp_dir().join(format!(
            "evo-fsevents-order-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let status = std::process::Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .current_dir(&dir)
            .status()
            .expect("git init runs");
        assert!(status.success(), "git init");
        let file = dir.join("plan.md");
        std::fs::write(&file, "x\n").unwrap();

        // A file inside the repository: membership first, then the save.
        let mut signals: Vec<MacOSSignal> = Vec::new();
        imp::emit_event_signals(
            FsSignal::FileSaved {
                path: file.to_string_lossy().into_owned(),
            },
            &file.to_string_lossy(),
            &mut |signal| signals.push(signal),
        );
        assert_eq!(signals.len(), 2, "membership + save: {signals:?}");
        assert!(
            matches!(&signals[0], MacOSSignal::RepositoryMembership { .. }),
            "membership must precede the save: {signals:?}"
        );
        assert!(matches!(&signals[1], MacOSSignal::FileSaved { .. }));

        // A commit in the repository: membership first, then the commit.
        let mut commit_signals: Vec<MacOSSignal> = Vec::new();
        imp::emit_event_signals(
            FsSignal::CommitMade {
                hash: "abcdef0123456789abcdef0123456789abcdef01".to_string(),
            },
            &dir.join(".git/logs/HEAD").to_string_lossy(),
            &mut |signal| commit_signals.push(signal),
        );
        assert_eq!(commit_signals.len(), 2, "membership + commit: {commit_signals:?}");
        assert!(
            matches!(&commit_signals[0], MacOSSignal::RepositoryMembership { .. }),
            "membership must precede the commit: {commit_signals:?}"
        );
        assert!(matches!(&commit_signals[1], MacOSSignal::CommitMade { .. }));

        // A file outside any repository: only the save — no fabricated
        // membership (IS-0020 §19: failure to represent is silence).
        let outside = std::env::temp_dir().join(format!(
            "evo-fsevents-outside-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&outside).unwrap();
        let plain = outside.join("notes.txt");
        std::fs::write(&plain, "x").unwrap();
        let mut plain_signals: Vec<MacOSSignal> = Vec::new();
        imp::emit_event_signals(
            FsSignal::FileSaved {
                path: plain.to_string_lossy().into_owned(),
            },
            &plain.to_string_lossy(),
            &mut |signal| plain_signals.push(signal),
        );
        assert_eq!(plain_signals.len(), 1, "no membership for a non-repo file");
        assert!(matches!(&plain_signals[0], MacOSSignal::FileSaved { .. }));

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
