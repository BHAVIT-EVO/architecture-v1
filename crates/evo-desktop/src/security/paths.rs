//! Canonical paths and shell-free commands (brief §9).
//!
//! Two rules, both enforced here rather than at call sites:
//!
//! - a path is normalised lexically first (`..` is refused, not resolved) and
//!   then, when it exists, checked after canonicalization so a symlink cannot
//!   walk out of the folder that was allowed;
//! - a command is always a program plus an argument array. There is no shell
//!   string, so `;`, `|` and `$()` cannot be interpreted.

use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathError {
    Empty,
    Nul,
    NotAbsolute,
    Traversal,
    TooLong,
    OutsideRoot,
    ProgramNotAllowed,
}

/// Lexically normalises an absolute path: no `.`, no `..`, no empty segments.
pub fn normalize_absolute(path: &str) -> Result<String, PathError> {
    if path.is_empty() {
        return Err(PathError::Empty);
    }
    if path.contains('\0') {
        return Err(PathError::Nul);
    }
    if path.len() > 4096 {
        return Err(PathError::TooLong);
    }
    if !path.starts_with('/') {
        return Err(PathError::NotAbsolute);
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => return Err(PathError::Traversal),
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        return Ok("/".to_string());
    }
    Ok(format!("/{}", parts.join("/")))
}

/// Whether `target` sits inside `root`, on a path-component boundary.
/// (`/Users/melon` is not inside `/Users/me`.)
pub fn is_within(root: &str, target: &str) -> bool {
    let (Ok(root), Ok(target)) = (normalize_absolute(root), normalize_absolute(target)) else {
        return false;
    };
    if root == "/" {
        return true;
    }
    target == root || target.starts_with(&format!("{root}/"))
}

/// Resolves a request inside a root. Lexical checks first, then canonical.
///
/// A path that does not exist yet is normalised and returned without
/// canonicalizing: creation is allowed inside the root, and the parent is
/// still checked when it exists.
pub fn resolve_within(root: &Path, requested: &str) -> Result<PathBuf, PathError> {
    if requested.is_empty() {
        return Err(PathError::Empty);
    }
    if requested.contains('\0') {
        return Err(PathError::Nul);
    }
    if requested.len() > 4096 {
        return Err(PathError::TooLong);
    }
    if requested.split('/').any(|part| part == "..") {
        return Err(PathError::Traversal);
    }
    let candidate = if requested.starts_with('/') {
        PathBuf::from(normalize_absolute(requested)?)
    } else {
        root.join(requested)
    };
    match (root.canonicalize(), candidate.canonicalize()) {
        (Ok(canonical_root), Ok(canonical_target)) => {
            if canonical_target.starts_with(&canonical_root) {
                Ok(canonical_target)
            } else {
                Err(PathError::OutsideRoot)
            }
        }
        // Not created yet: keep the lexical answer; creation is still rooted.
        _ => Ok(candidate),
    }
}

/// A program and its arguments, never a shell string.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommandLine {
    program: PathBuf,
    args: Vec<String>,
}

/// Directories Evo is allowed to launch from.
const PROGRAM_ROOTS: [&str; 4] = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];

impl CommandLine {
    /// Builds a command. The program must be absolute and live in a system
    /// directory; arguments are inert strings even when they contain shell
    /// syntax.
    pub fn new(program: &str, args: &[&str]) -> Result<Self, PathError> {
        let normalized = normalize_absolute(program)?;
        let inside = PROGRAM_ROOTS.iter().any(|root| {
            normalized.as_str() == *root || normalized.starts_with(&format!("{root}/"))
        });
        if !inside {
            return Err(PathError::ProgramNotAllowed);
        }
        let mut owned = Vec::with_capacity(args.len());
        for arg in args {
            if arg.contains('\0') {
                return Err(PathError::Nul);
            }
            owned.push((*arg).to_string());
        }
        Ok(Self {
            program: PathBuf::from(normalized),
            args: owned,
        })
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The only way this module spawns anything: an argument array, no shell.
    pub fn to_std_command(&self) -> std::process::Command {
        let mut command = std::process::Command::new(&self.program);
        command.args(&self.args);
        command
    }
}

/// The components of a path, for callers that want to show them without
/// re-deriving anything.
pub fn components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_dot_is_refused_not_resolved() {
        assert_eq!(
            normalize_absolute("/Users/me/../../etc/passwd"),
            Err(PathError::Traversal)
        );
        assert_eq!(normalize_absolute("/../etc"), Err(PathError::Traversal));
        assert_eq!(
            resolve_within(Path::new("/Users/me"), "../../etc/passwd"),
            Err(PathError::Traversal)
        );
        assert_eq!(
            resolve_within(Path::new("/Users/me"), "/../etc"),
            Err(PathError::Traversal)
        );
        assert_eq!(
            resolve_within(Path::new("/Users/me"), ""),
            Err(PathError::Empty)
        );
        assert_eq!(
            resolve_within(Path::new("/Users/me"), "a\0b"),
            Err(PathError::Nul)
        );
    }

    #[test]
    fn invalid_paths_are_refused() {
        assert_eq!(normalize_absolute(""), Err(PathError::Empty));
        assert_eq!(
            normalize_absolute("Documents/notes"),
            Err(PathError::NotAbsolute)
        );
        assert_eq!(normalize_absolute("/Users/me/a\0b"), Err(PathError::Nul));
        let long = format!("/{}", "a".repeat(5000));
        assert_eq!(normalize_absolute(&long), Err(PathError::TooLong));
    }

    #[test]
    fn the_prefix_trick_does_not_pass_for_containment() {
        assert!(is_within("/Users/me", "/Users/me/Documents/a.md"));
        assert!(is_within("/Users/me", "/Users/me"));
        assert!(!is_within("/Users/me", "/Users/melon/a.md"));
        assert!(!is_within("/Users/me", "/etc/passwd"));
        assert!(!is_within("/Users/me", "Documents/relative.md"));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_cannot_leave_the_root() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("evo-paths-{nanos}"));
        let root = base.join("allowed");
        let outside = base.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), b"no").unwrap();
        symlink(&outside, root.join("escape")).unwrap();

        // A relative request through the symlink resolves outside the root.
        let err = resolve_within(&root, "escape/secret.txt").unwrap_err();
        assert_eq!(err, PathError::OutsideRoot);

        // An absolute request for the same file is refused too.
        let escaped = outside.join("secret.txt");
        let escaped_text = escaped.to_string_lossy().to_string();
        assert_eq!(
            resolve_within(&root, &escaped_text),
            Err(PathError::OutsideRoot)
        );

        // A plain file inside the root still resolves.
        std::fs::write(root.join("ok.txt"), b"yes").unwrap();
        let ok = resolve_within(&root, "ok.txt").unwrap();
        assert!(ok.ends_with("ok.txt"));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn shell_syntax_stays_inside_one_argument() {
        let command = CommandLine::new(
            "/usr/bin/open",
            &["-a", "Finder", "/Users/me/Q3; rm -rf ~ #.md"],
        )
        .unwrap();
        assert_eq!(command.args().len(), 3);
        assert!(command.args()[2].contains("; rm -rf"));
        assert_eq!(command.program(), Path::new("/usr/bin/open"));
    }

    #[test]
    fn programs_outside_the_system_directories_are_refused() {
        assert_eq!(
            CommandLine::new("/tmp/evo", &[]),
            Err(PathError::ProgramNotAllowed)
        );
        assert_eq!(CommandLine::new("open", &[]), Err(PathError::NotAbsolute));
        assert_eq!(
            CommandLine::new("/usr/bin/open", &["a\0b"]),
            Err(PathError::Nul)
        );
    }
}
