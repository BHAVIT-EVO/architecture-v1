//! The permission model: scopes, actions, modes and the decision function.
//!
//! The model is deliberately small. A scope says *where* Evo may act; an
//! action says *what* it may do there; the mode says whether it asks. A
//! request that falls outside every scope is denied in every mode — least
//! privilege is not a preference.
//!
//! Persistence is a plain tab-separated text file so this module needs no
//! serialization dependency: one `mode` line, then one line per scope.
//! Malformed lines are skipped and counted, never treated as fatal.

use std::fmt;

use super::paths;

/// What Evo wants to do with a work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Open,
    Read,
    Write,
    Run,
}

impl Action {
    /// Writing and running change things; opening and reading do not.
    pub fn is_risky(self) -> bool {
        matches!(self, Action::Write | Action::Run)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Action::Open => "open",
            Action::Read => "read",
            Action::Write => "write",
            Action::Run => "run",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "open" => Some(Action::Open),
            "read" => Some(Action::Read),
            "write" => Some(Action::Write),
            "run" => Some(Action::Run),
            _ => None,
        }
    }
}

/// How cautious Evo is inside the scopes the user allowed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    AskEveryTime,
    #[default]
    AskForRisky,
    ActWithinScope,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::AskEveryTime => "ask_every_time",
            Mode::AskForRisky => "ask_for_risky",
            Mode::ActWithinScope => "act_within_scope",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "ask_every_time" => Some(Mode::AskEveryTime),
            "ask_for_risky" => Some(Mode::AskForRisky),
            "act_within_scope" => Some(Mode::ActWithinScope),
            _ => None,
        }
    }
}

/// One place Evo was allowed to act: a folder, an app, or a site.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Scope {
    Folder(String),
    App(String),
    Site(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ScopeError {
    Empty,
    NotAbsolute,
    Traversal,
    Nul,
    TooLong,
    BadApp,
    BadSite,
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            ScopeError::Empty => "a scope can't be empty",
            ScopeError::NotAbsolute => "a folder scope must be an absolute path",
            ScopeError::Traversal => "a folder scope can't contain ..",
            ScopeError::Nul => "a scope can't contain a null byte",
            ScopeError::TooLong => "a scope is longer than 1024 characters",
            ScopeError::BadApp => "an app name can't contain a path separator",
            ScopeError::BadSite => "a site must be a host name, without a scheme or slash",
        };
        f.write_str(text)
    }
}

impl Scope {
    pub fn kind(&self) -> &'static str {
        match self {
            Scope::Folder(_) => "folder",
            Scope::App(_) => "app",
            Scope::Site(_) => "site",
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Scope::Folder(v) | Scope::App(v) | Scope::Site(v) => v,
        }
    }

    /// Parses one scope line. Rejects anything that could escape its kind.
    pub fn parse(kind: &str, value: &str) -> Result<Self, ScopeError> {
        if value.is_empty() {
            return Err(ScopeError::Empty);
        }
        if value.contains('\0') {
            return Err(ScopeError::Nul);
        }
        if value.chars().count() > 1024 {
            return Err(ScopeError::TooLong);
        }
        match kind {
            "folder" => {
                if !value.starts_with('/') {
                    return Err(ScopeError::NotAbsolute);
                }
                if value.split('/').any(|part| part == "..") {
                    return Err(ScopeError::Traversal);
                }
                let normalized = paths::normalize_absolute(value).map_err(|err| match err {
                    paths::PathError::Traversal => ScopeError::Traversal,
                    paths::PathError::NotAbsolute => ScopeError::NotAbsolute,
                    paths::PathError::Nul => ScopeError::Nul,
                    paths::PathError::Empty => ScopeError::Empty,
                    paths::PathError::TooLong => ScopeError::TooLong,
                    paths::PathError::OutsideRoot | paths::PathError::ProgramNotAllowed => {
                        ScopeError::NotAbsolute
                    }
                })?;
                Ok(Scope::Folder(normalized))
            }
            "app" => {
                if value.contains('/') || value.contains('\\') {
                    return Err(ScopeError::BadApp);
                }
                if value.trim().is_empty() {
                    return Err(ScopeError::Empty);
                }
                Ok(Scope::App(value.trim().to_string()))
            }
            "site" => {
                if !is_host(value) {
                    return Err(ScopeError::BadSite);
                }
                Ok(Scope::Site(value.to_ascii_lowercase()))
            }
            _ => Err(ScopeError::Empty),
        }
    }

    /// Whether a request target falls inside this scope.
    pub fn contains(&self, target: &str) -> bool {
        match self {
            Scope::Folder(root) => paths::is_within(root, target),
            Scope::App(app) => app.eq_ignore_ascii_case(target.trim()),
            Scope::Site(site) => {
                let host = target.trim().trim_end_matches('.').to_ascii_lowercase();
                host == *site || host.ends_with(&format!(".{site}"))
            }
        }
    }
}

/// Lowercase host name: labels of letters, digits and hyphens.
fn is_host(value: &str) -> bool {
    if value.contains("://") || value.contains('/') || value.contains(' ') {
        return false;
    }
    let host = value.trim_end_matches('.');
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}

/// Why a request was refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DenyReason {
    Paused,
    OutOfScope,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    Allow,
    Ask,
    Deny(DenyReason),
}

impl Decision {
    pub fn is_allowed(self) -> bool {
        matches!(self, Decision::Allow)
    }
}

/// The world the decision is made in.
#[derive(Clone, Copy, Debug, Default)]
pub struct Context {
    /// The global kill switch. Paused denies everything, in every mode.
    pub paused: bool,
}

/// The user's choices: one mode, and the scopes they allowed.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PermissionSet {
    mode: Mode,
    scopes: Vec<Scope>,
}

impl PermissionSet {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            scopes: Vec::new(),
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Adds a scope. Returns the parse error instead of storing anything that
    /// is not a well-formed scope.
    pub fn allow(&mut self, scope: Scope) -> &mut Self {
        if !self.scopes.contains(&scope) {
            self.scopes.push(scope);
        }
        self
    }

    /// Removes a scope. Returns whether it was there.
    pub fn revoke(&mut self, kind: &str, value: &str) -> bool {
        let before = self.scopes.len();
        self.scopes
            .retain(|scope| !(scope.kind() == kind && scope.value() == value));
        self.scopes.len() != before
    }

    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    /// The decision function. Order matters: kill switch, then scope, then mode.
    pub fn decide(&self, context: Context, action: Action, target: &str) -> Decision {
        if context.paused {
            return Decision::Deny(DenyReason::Paused);
        }
        if !self.scopes.iter().any(|scope| scope.contains(target)) {
            return Decision::Deny(DenyReason::OutOfScope);
        }
        match self.mode {
            Mode::AskEveryTime => Decision::Ask,
            Mode::AskForRisky => {
                if action.is_risky() {
                    Decision::Ask
                } else {
                    Decision::Allow
                }
            }
            Mode::ActWithinScope => Decision::Allow,
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("mode\t{}\n", self.mode.as_str());
        for scope in &self.scopes {
            out.push_str(scope.kind());
            out.push('\t');
            out.push_str(&escape(scope.value()));
            out.push('\n');
        }
        out
    }

    /// Reads what it can. Returns the set and how many lines were unreadable,
    /// so the UI can say "2 lines in this file were skipped" instead of
    /// silently dropping a permission.
    pub fn from_text(text: &str) -> (Self, usize) {
        let mut set = PermissionSet::default();
        let mut skipped = 0;
        for (index, line) in text.lines().enumerate() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(2, '\t');
            let kind = parts.next().unwrap_or_default();
            let value = unescape(parts.next().unwrap_or_default());
            match kind {
                "mode" => match Mode::parse(&value) {
                    Some(mode) => set.mode = mode,
                    None => skipped += 1,
                },
                "folder" | "app" | "site" => match Scope::parse(kind, &value) {
                    Ok(scope) => {
                        set.allow(scope);
                    }
                    Err(_) => skipped += 1,
                },
                _ => {
                    if index > 0 {
                        skipped += 1;
                    }
                }
            }
        }
        (set, skipped)
    }
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(mode: Mode) -> PermissionSet {
        let mut set = PermissionSet::new(mode);
        set.allow(Scope::parse("folder", "/Users/me/Documents").unwrap());
        set.allow(Scope::parse("app", "Claude").unwrap());
        set.allow(Scope::parse("site", "claude.ai").unwrap());
        set
    }

    #[test]
    fn out_of_scope_is_denied_in_every_mode() {
        for mode in [Mode::AskEveryTime, Mode::AskForRisky, Mode::ActWithinScope] {
            let decision = set(mode).decide(Context::default(), Action::Read, "/etc/passwd");
            assert_eq!(decision, Decision::Deny(DenyReason::OutOfScope));
        }
    }

    #[test]
    fn paused_denies_even_inside_scope() {
        let decision = set(Mode::ActWithinScope).decide(
            Context { paused: true },
            Action::Open,
            "/Users/me/Documents/notes.md",
        );
        assert_eq!(decision, Decision::Deny(DenyReason::Paused));
    }

    #[test]
    fn modes_differ_only_on_risky_actions() {
        let target = "/Users/me/Documents/notes.md";
        assert_eq!(
            set(Mode::AskEveryTime).decide(Context::default(), Action::Open, target),
            Decision::Ask
        );
        assert_eq!(
            set(Mode::AskForRisky).decide(Context::default(), Action::Open, target),
            Decision::Allow
        );
        assert_eq!(
            set(Mode::AskForRisky).decide(Context::default(), Action::Write, target),
            Decision::Ask
        );
        assert_eq!(
            set(Mode::ActWithinScope).decide(Context::default(), Action::Run, target),
            Decision::Allow
        );
    }

    #[test]
    fn traversal_never_becomes_a_scope() {
        assert_eq!(
            Scope::parse("folder", "/Users/me/../etc"),
            Err(ScopeError::Traversal)
        );
        assert_eq!(
            Scope::parse("folder", "Documents"),
            Err(ScopeError::NotAbsolute)
        );
        assert_eq!(
            Scope::parse("folder", "/Users/me/Doc\0uments"),
            Err(ScopeError::Nul)
        );
        assert_eq!(Scope::parse("app", "../bin/sh"), Err(ScopeError::BadApp));
        assert_eq!(
            Scope::parse("site", "https://claude.ai"),
            Err(ScopeError::BadSite)
        );
        assert_eq!(
            Scope::parse("site", "claude.ai/login"),
            Err(ScopeError::BadSite)
        );
        assert_eq!(Scope::parse("folder", ""), Err(ScopeError::Empty));
    }

    #[test]
    fn sites_are_matched_by_host_suffix_only() {
        let site = Scope::parse("site", "claude.ai").unwrap();
        assert!(site.contains("claude.ai"));
        assert!(site.contains("CLAUDE.AI."));
        assert!(site.contains("docs.claude.ai"));
        assert!(!site.contains("notclaude.ai"));
        assert!(!site.contains("claude.ai.evil.example"));
    }

    #[test]
    fn text_round_trips_and_counts_garbage() {
        let original = set(Mode::ActWithinScope);
        let text = original.to_text();
        let (parsed, skipped) = PermissionSet::from_text(&text);
        assert_eq!(skipped, 0);
        assert_eq!(parsed, original);

        let dirty = format!("{text}folder\trelative/path\nsite\tnot a host\nnonsense\n");
        let (parsed, skipped) = PermissionSet::from_text(&dirty);
        assert_eq!(skipped, 3, "three bad lines, none of them permissions");
        assert_eq!(parsed.scopes().len(), original.scopes().len());
    }

    #[test]
    fn values_with_tabs_and_newlines_do_not_break_the_format() {
        let tricky = Scope::parse("app", "My App").unwrap();
        let text = format!(
            "mode\t{}\n{}\t{}\n",
            Mode::AskEveryTime.as_str(),
            tricky.kind(),
            "My App"
        );
        let (parsed, skipped) = PermissionSet::from_text(&text);
        assert_eq!(skipped, 0);
        assert_eq!(parsed.scopes()[0], tricky);
    }
}
