//! Repository identity resolution for RepositoryMembership witnessing
//! (RFC-0012).
//!
//! The git/filesystem collector resolves, at capture time, the canonical
//! repository identity a witnessed file or commit belongs to. This is
//! WITNESSING: the resolution runs only while the platform event is being
//! captured, and the resulting identity is persisted as canonical evidence.
//! Workspace Formation never calls into this module and never inspects live
//! filesystem or git state — it consumes only the persisted canonical
//! Observations (IS-0012 WF-1).
//!
//! # Repository identity
//!
//! The identity of one repository is its git directory (git-common-dir) as a
//! canonical text value:
//!
//! - a main working tree resolves to `<repo>/.git`;
//! - a linked worktree's gitdir is `<repo>/.git/worktrees/<name>`, whose
//!   common dir is `<repo>/.git` — so all worktrees of one repository share
//!   one identity;
//! - a submodule is its own repository (its gitdir is its own
//!   `<super>/.git/modules/<name>` or `<submodule>/.git`), so submodules
//!   resolve to their own distinct identity;
//! - a repository relocation changes the identity (a new git dir), which is
//!   an ordinary identity-revision event; historical Observations remain
//!   unchanged (RFC-0012 §Repository Relocation).
//!
//! # Failure behavior
//!
//! When no repository can be resolved truthfully, the collector emits no
//! RepositoryMembership observation (IS-0020 §19: failure to represent is
//! silence, never fabrication).

use std::path::{Component, Path, PathBuf};

/// Resolves the canonical repository identity for a witnessed file path, or
/// `None` when the path is not inside a repository.
///
/// Walks from the file's parent directory upward to the nearest ancestor that
/// contains a `.git` entry (the innermost repository wins, so a file inside a
/// submodule resolves to the submodule's repository).
pub fn repository_identity_for_path(path: &Path) -> Option<String> {
    let absolute = std::path::absolute(path).ok()?;
    let mut directory = absolute.parent()?.to_path_buf();
    loop {
        let git_entry = directory.join(".git");
        if git_entry.is_dir() {
            return common_dir_for_git_dir(&git_entry);
        }
        if git_entry.is_file() {
            let git_dir = git_dir_from_file(&git_entry, &directory)?;
            return common_dir_for_git_dir(&git_dir);
        }
        if !directory.pop() {
            return None;
        }
    }
}

/// Resolves the canonical repository identity from a witnessed git reflog
/// path (for example `<repo>/.git/logs/HEAD` or
/// `<repo>/.git/worktrees/<name>/logs/HEAD`), or `None` when the path is not
/// a reflog of a resolvable repository.
///
/// The reflog path is the platform witness of a HEAD transition; stripping
/// the `/logs/HEAD` suffix yields the gitdir, from which the common dir is
/// derived.
pub fn repository_identity_from_reflog_path(path: &Path) -> Option<String> {
    let reflog = "logs/HEAD";
    let absolute = std::path::absolute(path).ok()?;
    let text = absolute.to_string_lossy();
    let git_dir = text.strip_suffix(reflog)?;
    if !git_dir.ends_with('/') {
        return None;
    }
    common_dir_for_git_dir(Path::new(git_dir.trim_end_matches('/')))
}

/// Derives the common git directory of one gitdir.
///
/// For a normal repository the gitdir is its own common dir. For a linked
/// worktree the gitdir is `<repo>/.git/worktrees/<name>`; its `commondir`
/// file (or the canonical `worktrees` layout) resolves to `<repo>/.git`.
fn common_dir_for_git_dir(git_dir: &Path) -> Option<String> {
    if !git_dir.is_dir() {
        return None;
    }
    // Linked worktrees carry a `commondir` file whose content is the common
    // directory path relative to the gitdir.
    let commondir_file = git_dir.join("commondir");
    if commondir_file.is_file() {
        if let Ok(content) = std::fs::read_to_string(&commondir_file) {
            let target = content.trim();
            if !target.is_empty() {
                let resolved = if Path::new(target).is_absolute() {
                    PathBuf::from(target)
                } else {
                    git_dir.join(target)
                };
                let common = std::path::absolute(resolved).ok()?;
                return Some(canonical_identity(&common));
            }
        }
    }
    // Fall back to the canonical worktree layout: the common dir is the
    // gitdir with any trailing `worktrees/<name>` suffix removed.
    let mut components: Vec<Component> = Vec::new();
    for component in git_dir.components() {
        if component.as_os_str() == "worktrees" {
            break;
        }
        components.push(component);
    }
    let common = components.iter().collect::<PathBuf>();
    if common.as_os_str().is_empty() {
        return None;
    }
    Some(canonical_identity(&common))
}

/// The canonical identity text for one git directory path.
///
/// Resolves filesystem symlinks (macOS maps `/var` → `/private/var`, and git
/// itself stores real paths) so that the same repository reached through
/// different path spellings produces one identity. Lexically normalizes `.`
/// and `..` components afterwards.
fn canonical_identity(path: &Path) -> String {
    match std::fs::canonicalize(path) {
        Ok(canonical) => normalized_path(&canonical),
        Err(_) => normalized_path(path),
    }
}

/// Parses a `.git` file (worktree or submodule pointer) into the absolute
/// gitdir it references, resolved relative to `containing_dir`.
fn git_dir_from_file(git_entry: &Path, containing_dir: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(git_entry).ok()?;
    let line = content.trim();
    let target = line.strip_prefix("gitdir:")?.trim();
    let resolved = if Path::new(target).is_absolute() {
        PathBuf::from(target)
    } else {
        containing_dir.join(target)
    };
    std::path::absolute(resolved).ok()
}

/// Normalizes a path to a canonical absolute form without resolving
/// symlinks: lexical normalization of `.` and `..` components.
fn normalized_path(path: &Path) -> String {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("evo-repo-identity-{label}-{nanos}"))
    }

    fn git(args: &[&str], cwd: &Path) -> std::process::Output {
        std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("git runs")
    }

    fn init_repo(dir: &Path) {
        std::fs::create_dir_all(dir).expect("create dir");
        git(&["init", "-b", "main"], dir);
        git(&["config", "user.email", "test@example.com"], dir);
        git(&["config", "user.name", "Evo Test"], dir);
    }

    /// Commits a placeholder file so the repository has an initial commit
    /// (required before `git submodule add`).
    fn initial_commit(dir: &Path) {
        std::fs::write(dir.join("README.md"), "seed").expect("write");
        git(&["add", "."], dir);
        git(&["commit", "-m", "seed"], dir);
    }

    #[test]
    fn file_outside_any_repository_resolves_to_none() {
        let dir = unique("outside");
        std::fs::create_dir_all(dir.join("sub")).expect("create dir");
        let file = dir.join("sub").join("notes.md");
        std::fs::write(&file, "x").expect("write");
        assert_eq!(repository_identity_for_path(&file), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_inside_main_repository_resolves_to_the_git_dir() {
        let dir = unique("main");
        init_repo(&dir);
        let file = dir.join("src").join("main.rs");
        std::fs::create_dir_all(file.parent().unwrap()).expect("create dir");
        std::fs::write(&file, "fn main() {}").expect("write");
        let identity = repository_identity_for_path(&file).expect("resolves");
        assert_eq!(identity, canonical_identity(&dir.join(".git")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn linked_worktree_resolves_to_the_same_identity_as_the_main_worktree() {
        let main = unique("worktree-main");
        init_repo(&main);
        let main_identity = repository_identity_for_path(&main.join("file.txt"))
            .expect("main resolves");
        std::fs::write(main.join("file.txt"), "x").expect("write");

        let worktree = unique("worktree-linked");
        let status = git(&["worktree", "add", worktree.to_str().unwrap(), "-b", "feature"], &main);
        assert!(status.status.success(), "git worktree add");

        let worktree_identity = repository_identity_for_path(&worktree.join("file.txt"))
            .expect("worktree resolves");
        assert_eq!(
            worktree_identity, main_identity,
            "linked worktrees unify to one repository identity"
        );
        let _ = git(&["worktree", "remove", "--force", worktree.to_str().unwrap()], &main);
        let _ = std::fs::remove_dir_all(&main);
    }

    #[test]
    fn detached_head_still_resolves_membership() {
        let dir = unique("detached");
        init_repo(&dir);
        std::fs::write(dir.join("a.txt"), "a").expect("write");
        git(&["add", "."], &dir);
        git(&["commit", "-m", "one"], &dir);
        std::fs::write(dir.join("b.txt"), "b").expect("write");
        git(&["add", "."], &dir);
        git(&["commit", "-m", "two"], &dir);
        // Detach HEAD at the first commit.
        let first = git(&["rev-parse", "HEAD~1"], &dir);
        let hash = String::from_utf8_lossy(&first.stdout).trim().to_string();
        git(&["checkout", "--detach", &hash], &dir);
        std::fs::write(dir.join("detached.txt"), "d").expect("write");
        git(&["add", "."], &dir);
        git(&["commit", "-m", "detached"], &dir);
        // The reflog append for a detached commit is still a HEAD reflog.
        let reflog = dir.join(".git/logs/HEAD");
        let identity = repository_identity_from_reflog_path(&reflog).expect("reflog resolves");
        assert_eq!(identity, canonical_identity(&dir.join(".git")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn submodule_resolves_to_its_own_identity() {
        let super_dir = unique("super");
        let sub_dir = unique("sub");
        init_repo(&super_dir);
        initial_commit(&super_dir);
        init_repo(&sub_dir);
        std::fs::write(sub_dir.join("sub.txt"), "s").expect("write");
        git(&["add", "."], &sub_dir);
        git(&["commit", "-m", "sub"], &sub_dir);
        let status = git(
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                sub_dir.to_str().unwrap(),
                "vendor/sub",
            ],
            &super_dir,
        );
        assert!(status.status.success(), "git submodule add");
        git(&["commit", "-am", "add submodule"], &super_dir);

        let inside_sub = super_dir.join("vendor/sub/sub.txt");
        let sub_identity =
            repository_identity_for_path(&inside_sub).expect("submodule file resolves");
        let super_identity = repository_identity_for_path(&super_dir.join("top.txt"))
            .expect("superproject file resolves");
        assert_ne!(
            sub_identity, super_identity,
            "submodules are separate repositories with separate identities"
        );
        // The submodule identity equals its own git dir (resolved through the
        // `.git` file pointer) and is not the superproject's.
        assert_eq!(
            sub_identity,
            canonical_identity(&super_dir.join(".git/modules/vendor/sub"))
        );
        let _ = std::fs::remove_dir_all(&super_dir);
        let _ = std::fs::remove_dir_all(&sub_dir);
    }

    #[test]
    fn reflog_identity_matches_file_identity_for_the_same_repository() {
        let dir = unique("reflog");
        init_repo(&dir);
        let file = dir.join("x.txt");
        std::fs::write(&file, "x").expect("write");
        git(&["add", "."], &dir);
        git(&["commit", "-m", "first"], &dir);
        let from_file = repository_identity_for_path(&file).expect("file identity");
        let from_reflog =
            repository_identity_from_reflog_path(&dir.join(".git/logs/HEAD")).expect("reflog identity");
        assert_eq!(from_file, from_reflog);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_reflog_path_resolves_to_none() {
        assert_eq!(
            repository_identity_from_reflog_path(Path::new("/tmp/not-a-reflog")),
            None
        );
    }
}
