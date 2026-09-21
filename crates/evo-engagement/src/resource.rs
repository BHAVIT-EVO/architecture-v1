//! Witnessed resources and the character of the act that witnessed them.
//!
//! A `Resource` is the thing an Observation was about: a file path, a URL, a
//! window, a commit. Evo never classifies resources by application, vendor,
//! domain, file type, or profession. It classifies them by *how the act was
//! witnessed*, which is a property of the frozen Observation schema and
//! therefore a canonical fact rather than an opinion.

use std::collections::BTreeSet;

/// How Evo came to witness an act involving a resource.
///
/// This is the only classification Evo makes about an act, and it is derived
/// from the frozen Observation schema — never from the content, the
/// application, or the location of the resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActCharacter {
    /// The person directed their attention somewhere. Bringing a window
    /// forward or navigating to an address is an act only a person performs;
    /// it is direct evidence of attention.
    Attentional,
    /// Something changed, and Evo cannot tell from the act alone whether a
    /// person or the machine caused it. A file write is the canonical case: a
    /// person saving a draft and a background service flushing a cache are
    /// witnessed identically.
    ///
    /// Incidental acts never establish engagement on their own. They become
    /// evidence only when they are *selectively* associated with attention
    /// (see [`crate::affinity`]) — which is what separates a document the
    /// person is writing from a cache the operating system rewrites all day.
    Incidental,
    /// The person recorded a deliberate, durable act. Committing is the
    /// canonical case: it cannot happen incidentally.
    Deliberate,
}

impl ActCharacter {
    /// The character of an act witnessed under the given frozen schema name.
    ///
    /// Unknown schemas are treated as [`ActCharacter::Incidental`]: the
    /// conservative choice, since an unrecognized act has not been shown to
    /// require a person. This is what lets a new resource type be added with
    /// no application-specific code — it participates immediately, and earns
    /// attentional standing only through evidence.
    pub fn of_schema(schema_name: &str) -> Self {
        match schema_name {
            "OBS-WINDOW-FOCUS-GAINED" | "OBS-URL-NAVIGATED" => ActCharacter::Attentional,
            "OBS-COMMIT-MADE" => ActCharacter::Deliberate,
            _ => ActCharacter::Incidental,
        }
    }

    /// Whether this act is direct evidence that a person was present and
    /// acting.
    pub fn is_human_evidence(self) -> bool {
        matches!(self, ActCharacter::Attentional | ActCharacter::Deliberate)
    }
}

/// The shape of a witnessed subject, used only to derive structural
/// relationships and to phrase honest explanations.
///
/// This is generic string structure — a path has parents, an address has an
/// origin — and carries no knowledge of any particular application or site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubjectShape {
    /// A filesystem path.
    Path,
    /// An addressable location.
    Address,
    /// A named surface with no addressable location (e.g. a window title).
    Surface,
    /// An opaque identifier (e.g. a commit hash).
    Opaque,
}

/// One witnessed resource, identified by its canonical subject string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Resource {
    subject: String,
    shape: SubjectShape,
    /// A container witnessed directly rather than implied by the subject
    /// string — for example a repository the resource was observed to belong
    /// to. Treated exactly like a parent directory, never as a stronger claim.
    container: Option<String>,
}

impl Resource {
    /// Builds a resource from a canonical subject and the frozen schema that
    /// witnessed it.
    pub fn new(subject: impl Into<String>, schema_name: &str) -> Self {
        let subject = subject.into();
        let shape = shape_of(&subject, schema_name);
        Self {
            subject,
            shape,
            container: None,
        }
    }

    /// Builds a resource whose container was witnessed directly.
    ///
    /// This is how a declared membership — a repository, a project folder, a
    /// case file — enters the model: as *location*, no more. Because location
    /// alone can never reach the relatedness floor, membership never fuses
    /// distinct tasks that happen to live in the same place.
    pub fn with_container(
        subject: impl Into<String>,
        schema_name: &str,
        container: impl Into<String>,
    ) -> Self {
        let mut resource = Self::new(subject, schema_name);
        resource.container = Some(container.into());
        resource
    }

    /// Returns this resource placed inside a directly witnessed container.
    pub fn in_container(mut self, container: impl Into<String>) -> Self {
        self.container = Some(container.into());
        self
    }

    /// The same resource under its canonical name.
    ///
    /// Shape and witnessed container are carried across unchanged: the shape is
    /// a fact about the frozen schema that witnessed the act, not about the
    /// spelling of the subject, so recovering identity must not be able to
    /// change it. See [`crate::identity`] for where canonical names come from.
    pub fn renamed(&self, subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            shape: self.shape,
            container: self.container.clone(),
        }
    }

    /// The canonical witnessed subject.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The structural shape of the subject.
    pub fn shape(&self) -> SubjectShape {
        self.shape
    }

    /// The structural container this resource sits in, when its shape has
    /// one: a directly witnessed container if there is one, otherwise the
    /// parent directory of a path or the origin of an address.
    ///
    /// Surfaces deliberately have **no** implied structural container. A window
    /// title commonly ends with the name of the application that owns it, and
    /// grouping by that would make "everything in one browser" a single body
    /// of work — precisely the failure this layer exists to prevent. A surface
    /// can still have a *witnessed* container, since that is a fact rather than
    /// an inference from its name.
    pub fn container(&self) -> Option<String> {
        if let Some(container) = &self.container {
            return Some(container.clone());
        }
        match self.shape {
            SubjectShape::Path => {
                let trimmed = self.subject.trim_end_matches('/');
                trimmed.rfind('/').map(|at| trimmed[..at].to_string())
            }
            SubjectShape::Address => origin_of(&self.subject),
            SubjectShape::Surface | SubjectShape::Opaque => None,
        }
    }

    /// The distinctive words in this subject, lowercased.
    ///
    /// Used for lexical affinity. Purely mechanical: split on non-alphanumeric
    /// boundaries and on camel-case humps, drop very short fragments. No
    /// vocabulary, stop-word list, or language assumption is embedded, so this
    /// behaves the same for any language that uses the Latin alphabet and
    /// degrades to "no lexical evidence" for those that do not — never to a
    /// wrong answer.
    pub fn tokens(&self) -> BTreeSet<String> {
        tokenize(&self.subject)
    }

    /// A short human-readable name for this resource, for explanations.
    pub fn display_name(&self) -> &str {
        match self.shape {
            SubjectShape::Path => self
                .subject
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(&self.subject),
            // An address without its scheme or its query. Both are machinery: a
            // person recognises `meet.google.com/kxc-vads-gdp`, not the same
            // string wrapped in `https://` and trailed by four hundred characters
            // of tracking parameters. Nothing is added — this is a slice of the
            // witnessed subject — so the name stays something Evo actually saw.
            SubjectShape::Address => {
                let after_scheme = self
                    .subject
                    .split_once("://")
                    .map_or(self.subject.as_str(), |(_, rest)| rest);
                let readable = after_scheme
                    .split(['?', '#'])
                    .next()
                    .unwrap_or(after_scheme)
                    .trim_end_matches('/');
                if readable.is_empty() {
                    &self.subject
                } else {
                    readable
                }
            }
            _ => &self.subject,
        }
    }
}

fn shape_of(subject: &str, schema_name: &str) -> SubjectShape {
    match schema_name {
        "OBS-FILE-SAVED" => SubjectShape::Path,
        "OBS-URL-NAVIGATED" => SubjectShape::Address,
        "OBS-WINDOW-FOCUS-GAINED" => SubjectShape::Surface,
        "OBS-COMMIT-MADE" => SubjectShape::Opaque,
        // An unrecognized schema is shaped by the subject itself, so new
        // resource types participate without new code.
        _ => {
            if subject.starts_with('/') {
                SubjectShape::Path
            } else if subject.contains("://") {
                SubjectShape::Address
            } else {
                SubjectShape::Surface
            }
        }
    }
}

/// The origin of an address: everything up to the end of the authority.
fn origin_of(subject: &str) -> Option<String> {
    let after_scheme = subject.split("://").nth(1)?;
    let authority = after_scheme.split('/').next()?;
    if authority.is_empty() {
        return None;
    }
    Some(authority.to_string())
}

/// Tokenizes an arbitrary string the same way member subjects are, so a
/// retrieval trigger is split by exactly the rules its candidate vocabulary was.
///
/// Retrieval scores a phrase against the witnessed names of a thread's members
/// ([`Resource::tokens`]); if the two used different splitting rules, a trigger
/// could fail to match a token it plainly shares. Exposing the one tokenizer is
/// what keeps them honest. It embeds no vocabulary or language assumption — see
/// [`Resource::tokens`].
pub fn tokenize_subject(subject: &str) -> BTreeSet<String> {
    tokenize(subject)
}

/// Splits a subject into distinctive lowercase word fragments.
fn tokenize(subject: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut current = String::new();
    let mut previous_was_lower = false;

    let flush = |current: &mut String, tokens: &mut BTreeSet<String>| {
        if current.chars().count() >= 3 {
            tokens.insert(current.to_lowercase());
        }
        current.clear();
    };

    for ch in subject.chars() {
        if ch.is_alphanumeric() {
            // Split camel-case humps so `assignmentReport` yields both words.
            if ch.is_uppercase() && previous_was_lower {
                flush(&mut current, &mut tokens);
            }
            previous_was_lower = ch.is_lowercase();
            current.push(ch);
        } else {
            previous_was_lower = false;
            flush(&mut current, &mut tokens);
        }
    }
    flush(&mut current, &mut tokens);
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_character_follows_the_frozen_schema_not_the_application() {
        assert_eq!(
            ActCharacter::of_schema("OBS-WINDOW-FOCUS-GAINED"),
            ActCharacter::Attentional
        );
        assert_eq!(
            ActCharacter::of_schema("OBS-URL-NAVIGATED"),
            ActCharacter::Attentional
        );
        assert_eq!(
            ActCharacter::of_schema("OBS-COMMIT-MADE"),
            ActCharacter::Deliberate
        );
        assert_eq!(
            ActCharacter::of_schema("OBS-FILE-SAVED"),
            ActCharacter::Incidental
        );
    }

    /// A resource type Evo has never seen must participate without new code,
    /// and must do so conservatively.
    #[test]
    fn unknown_schemas_participate_conservatively() {
        assert_eq!(
            ActCharacter::of_schema("OBS-SPREADSHEET-CELL-EDITED"),
            ActCharacter::Incidental
        );
        let resource = Resource::new("/Users/x/model.step", "OBS-CAD-PART-SAVED");
        assert_eq!(resource.shape(), SubjectShape::Path);
        assert_eq!(resource.container().as_deref(), Some("/Users/x"));
    }

    #[test]
    fn paths_have_their_parent_directory_as_container() {
        let resource = Resource::new("/Users/x/work/notes.md", "OBS-FILE-SAVED");
        assert_eq!(resource.container().as_deref(), Some("/Users/x/work"));
        assert_eq!(resource.display_name(), "notes.md");
    }

    #[test]
    fn addresses_have_their_origin_as_container() {
        let resource = Resource::new("https://example.org/a/b?c=d", "OBS-URL-NAVIGATED");
        assert_eq!(resource.container().as_deref(), Some("example.org"));
    }

    /// The regression that keeps "all windows of one application" from
    /// becoming one body of work.
    #[test]
    fn surfaces_have_no_structural_container() {
        let resource = Resource::new("Some Document — SomeApp", "OBS-WINDOW-FOCUS-GAINED");
        assert_eq!(resource.container(), None);
    }

    #[test]
    fn a_witnessed_container_overrides_the_implied_one() {
        let resource = Resource::with_container(
            "/Users/x/repo/deep/nested/file.rs",
            "OBS-FILE-SAVED",
            "/Users/x/repo",
        );
        assert_eq!(resource.container().as_deref(), Some("/Users/x/repo"));
    }

    #[test]
    fn tokenization_is_mechanical_and_splits_camel_case() {
        let resource = Resource::new("/Users/x/InternshipAssignment_final.pdf", "OBS-FILE-SAVED");
        let tokens = resource.tokens();
        assert!(tokens.contains("internship"));
        assert!(tokens.contains("assignment"));
        assert!(tokens.contains("final"));
        assert!(tokens.contains("pdf"));
    }

    #[test]
    fn very_short_fragments_are_not_tokens() {
        let resource = Resource::new("a/b/cd/efg", "OBS-FILE-SAVED");
        let tokens = resource.tokens();
        assert!(!tokens.contains("a"));
        assert!(!tokens.contains("cd"));
        assert!(tokens.contains("efg"));
    }
}
