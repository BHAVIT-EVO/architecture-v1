//! Resource identity: telling *what a thing is* apart from *what state it was
//! in* when it was witnessed.
//!
//! # The rung of the ladder this fills
//!
//! Every Observation carries a subject string, and Evo used to treat each
//! distinct string as a distinct resource. That collapses two levels of the
//! semantic ladder — OBSERVATION and RESOURCE — into one, and a real day makes
//! the consequences obvious. On the machine this was written against, 1245
//! Observations produced 490 "resources", among them:
//!
//! ```text
//! walkthrough.md.pdf – Page 1 of 27     one document, ten resources
//! walkthrough.md.pdf – Page 6 of 27
//! …
//! Job assignment - Claude - High memory usage - 1.4 GB - Google Chrome – BHAVIT
//! Job assignment - Claude - High memory usage - 1.3 GB - Google Chrome – BHAVIT
//! Job assignment - Claude – Audio playing - Google Chrome – BHAVIT
//!                                       one window, four resources
//! https://google.com/search?q=X&mstk=A…  one search, six resources
//! https://google.com/search?q=X&mstk=B…
//! ```
//!
//! None of that is work being done in ten places. It is one thing, seen ten
//! times, wearing whatever the window server or the address bar happened to be
//! saying at that moment. Grouping cannot repair this downstream: a document
//! split into ten fragments is ten participants that all deserve to be one, and
//! the attention that proves someone was working there is divided ten ways.
//! Identity has to be settled before anything is measured.
//!
//! # How identity is recovered
//!
//! A witnessed name is split — using punctuation only, never knowledge of any
//! application, site, or file type — into a **stem** and the **attributes**
//! appended to it:
//!
//! ```text
//! "Job assignment - Claude - High memory usage - 1.4 GB - Google Chrome – BHAVIT"
//!  stem: "Job assignment"
//!  attributes: "Claude", "High memory usage", "1.4 GB", "Google Chrome", "BHAVIT"
//!
//! "https://google.com/search?q=X&mstk=A…"
//!  stem: "https://google.com/search"
//!  attributes: "q=X", "mstk=A…"
//! ```
//!
//! Three rules then decide which attributes carry identity. All are learned from
//! the person's own history — nothing is listed in advance — and all are one
//! principle counted three different ways: *a part that does not distinguish this
//! thing from other things is not part of what this thing is.*
//!
//! 1. **Decoration.** An attribute appearing under more than one stem is not
//!    what makes this thing itself. A browser's name, a window-server status, an
//!    account, a version-control state — these attach to everything, so they
//!    separate nothing. No threshold: a second stem is disqualifying.
//!
//! 2. **Readout.** Attributes under one stem are grouped into *fields*, and a
//!    field witnessed holding two or more different values is a readout: it
//!    reports what the sighting happened to show, so no one of its values
//!    describes the thing. What counts as "the same field" is the one place the
//!    two witnessed forms differ, because they carry different evidence:
//!
//!    - A **name** has no field labels, so the only thing that can prove two
//!      values are readings of one field is that they *differ only in their
//!      digits*: `1.2 GB` and `1.5 GB`, `Page 3 of 27` and `Page 12 of 27`,
//!      `41% loaded` and `78% loaded`. Two values that differ in their letters
//!      are two different fields holding one value each, and nothing is dropped.
//!      This is what keeps `Reader - contract.pdf` and `Reader - lease.pdf`
//!      apart, and `Meet - kxc-vads-gdp` apart from `Meet - xae-jgzs-xsx`, no
//!      matter how many siblings each has.
//!    - An **address** labels its own fields, so the field is given and the
//!      question is whether any reading ever came back. A value witnessed at one
//!      key in more than one sighting is something the person asked for; a key
//!      whose every value was seen exactly once is something the far end stamped.
//!      This is what keeps forty different searches at one address apart while
//!      still folding the per-request stamps that decorate each of them.
//!
//!    Rule 2 is deliberately *not* expressed in terms of which position an
//!    attribute occupies. Positions shift: the same window drops "Audio playing"
//!    and everything after it slides left, which is enough to make a byte count
//!    and an application name look like the same slot.
//!
//! 3. **Subsumption.** After rules 1 and 2, if Evo witnessed this same stem
//!    named with a *strict subset* of the attributes this sighting carries, then
//!    it has seen the thing named without them, and they are state rather than
//!    identity. `Job assignment - Claude – Audio playing` folds into
//!    `Job assignment - Claude` because the latter was really witnessed;
//!    `Meet - kxc-vads-gdp` does not fold into anything, because a bare `Meet`
//!    never was. This rule can only ever fold a name onto another name that was
//!    actually seen, so it cannot invent a merge out of an absence.
//!
//! # What this deliberately does not do
//!
//! It never merges two things that share no stem, and it never invents a name: a
//! canonical name is always a substring structure of names really witnessed. It
//! contains no application, domain, extension, or category, and it treats a
//! window title, a URL, a path and a resource type invented next year the same
//! way.
//!
//! Where the names alone cannot settle the question, it keeps the attribute
//! rather than dropping it — a slightly longer name costs a little clarity,
//! whereas a wrongly dropped attribute merges two different things, and only one
//! of those two mistakes is recoverable.
//!
//! The honest failure modes, stated rather than hidden:
//!
//! - A readout that lives inside the **stem** rather than after a separator —
//!   `Inbox (2,886)`, `Inbox (2,884)` — is out of reach, because every rule here
//!   works on attributes and those names have none in common. Folding stems that
//!   differ only in digits would also merge `report-2024.pdf` with
//!   `report-2025.pdf`, which is the unrecoverable direction, so it is not done.
//! - Two naming systems for one thing — the address
//!   `https://meet.google.com/kxc-vads-gdp` and the window title
//!   `Meet - kxc-vads-gdp` — share no stem and cannot be merged from names
//!   alone. The evidence that would settle it is the address of the focused
//!   window, which the Observation schema has room for and the capture layer does
//!   not yet record; see `docs/rfcs/RFC-0014-engagement-contract.md` §Identity.
//! - A document reached only through a reader that leads with the reader's own
//!   name, where the document part differs only in digits — `Volume 1`,
//!   `Volume 2` — folds into one resource. That is indistinguishable from one
//!   reader showing successive volumes.

use crate::declarations::Declarations;
use crate::episode::Act;

use std::collections::{BTreeMap, BTreeSet};
use std::time::SystemTime;

/// Punctuation that separates a name from things appended to it.
///
/// Format-level only. These are the runs used to append status, application, and
/// account decorations to window titles across desktop environments; the list
/// says nothing about which applications exist or what they are for.
const SEPARATORS: [&str; 5] = [" - ", " — ", " – ", " | ", " · "];

/// Marks a field name as belonging to an address, whose fields are labelled by
/// the format itself. Not a printable character, so it cannot collide with a
/// field name learned from a real name.
const ADDRESSED_FIELD: &str = "\u{1}";

/// Stands for a run of digits in a field name. Not a printable character, so a
/// title containing `#` or a digit cannot be mistaken for a skeleton.
const DIGITS: char = '\u{2}';

/// Which field of its stem an attribute is a reading of.
///
/// An address labels its own fields, so the label is the key of the `key=value`
/// pair (or `#` for a fragment) — structure defined by the address format, not
/// knowledge of any site. A name has no labels, so the only field two values can
/// be shown to share is the one they occupy when their digits are set aside:
/// `1.2 GB` and `1.5 GB` are one field, `contract.pdf` and `lease.pdf` are two.
fn field_of(parts: &Parts, attribute: &Attribute) -> String {
    if !parts.addressed {
        return skeleton(&attribute.value);
    }
    let label = match attribute.separator {
        "#" => "#",
        _ => attribute
            .value
            .split_once('=')
            .map_or(attribute.value.as_str(), |(key, _)| key),
    };
    format!("{ADDRESSED_FIELD}{label}")
}

/// A value with every run of digits replaced by one placeholder.
///
/// Two values with the same skeleton differ only in their digits — which is why
/// equal skeletons are evidence of one varying field, and why a value with no
/// digits is always a field of its own.
fn skeleton(value: &str) -> String {
    let mut shape = String::with_capacity(value.len());
    let mut in_digits = false;
    for character in value.chars() {
        if character.is_ascii_digit() {
            if !in_digits {
                shape.push(DIGITS);
                in_digits = true;
            }
        } else {
            in_digits = false;
            shape.push(character);
        }
    }
    shape
}

/// One attribute of a witnessed name, with enough context to put the name back
/// together after some attributes have been dropped.
#[derive(Debug, Clone)]
struct Attribute {
    value: String,
    /// The punctuation that introduced this attribute in the original name.
    separator: &'static str,
}

/// A witnessed name split into the part that might be identity and the parts
/// that might be state.
#[derive(Debug, Clone)]
struct Parts {
    stem: String,
    attributes: Vec<Attribute>,
    /// Addresses reassemble with `?`/`&`/`#`; names reassemble with the
    /// punctuation they were written with.
    addressed: bool,
}

/// The canonical name of every witnessed subject, learned from the corpus.
///
/// Built once per reconstruction and consulted everywhere a subject is used, so
/// that attention, relatedness, roles, and titles all speak about the same
/// things.
#[derive(Debug, Clone, Default)]
pub struct IdentityIndex {
    canonical: BTreeMap<String, String>,
    founding: BTreeMap<String, String>,
}

impl IdentityIndex {
    /// Learns identity from every subject a history witnessed.
    ///
    /// A pure function of the *set* of subjects: the input is collected into an
    /// ordered set before anything is counted, so the result cannot depend on
    /// the order acts arrived in or on how often each name recurred.
    pub fn learn<I, S>(subjects: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let distinct: BTreeSet<String> = subjects
            .into_iter()
            .map(|subject| subject.as_ref().to_string())
            .collect();

        let parsed: BTreeMap<String, Parts> = distinct
            .iter()
            .map(|subject| (subject.clone(), parse(subject)))
            .collect();

        // Rule 1's evidence: which stems each attribute was seen under.
        let mut stems_of_value: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        // Rule 2's evidence: for each (stem, field), which names carried each
        // value the field was witnessed holding.
        let mut field_readings: BTreeMap<(&str, String), BTreeMap<&str, BTreeSet<&str>>> =
            BTreeMap::new();

        for (subject, parts) in &parsed {
            for attribute in &parts.attributes {
                stems_of_value
                    .entry(attribute.value.as_str())
                    .or_default()
                    .insert(parts.stem.as_str());
                field_readings
                    .entry((parts.stem.as_str(), field_of(parts, attribute)))
                    .or_default()
                    .entry(attribute.value.as_str())
                    .or_default()
                    .insert(subject.as_str());
            }
        }

        // Rule 2. A field is a readout when it was witnessed holding more than
        // one value, and — for an address, whose fields are labelled and whose
        // values may be things the person asked for — when no value of it ever
        // came back a second time.
        let readouts: BTreeSet<(&str, String)> = field_readings
            .iter()
            .filter(|((_, field), readings)| {
                readings.len() > 1
                    && (!field.starts_with(ADDRESSED_FIELD)
                        || readings.values().all(|names| names.len() == 1))
            })
            .map(|(key, _)| (key.0, key.1.clone()))
            .collect();

        // Rules 1 and 2, applied to every witnessed name.
        let surviving: BTreeMap<&String, Vec<&Attribute>> = parsed
            .iter()
            .map(|(subject, parts)| {
                let kept: Vec<&Attribute> = parts
                    .attributes
                    .iter()
                    .filter(|attribute| {
                        let decoration = stems_of_value
                            .get(attribute.value.as_str())
                            .is_some_and(|stems| stems.len() > 1);
                        let readout =
                            readouts.contains(&(parts.stem.as_str(), field_of(parts, attribute)));
                        !decoration && !readout
                    })
                    .collect();
                (subject, kept)
            })
            .collect();

        // Rule 3. Where Evo witnessed this same stem named with strictly fewer
        // attributes, that shorter name is what the thing is; the extra ones were
        // state. The target is the smallest such witnessed name, so a chain of
        // increasingly-decorated sightings resolves in one pass, and the tie-break
        // is the reassembled name so the outcome cannot depend on iteration order.
        let mut canonical = BTreeMap::new();
        for (subject, parts) in &parsed {
            let mine: BTreeSet<&str> = surviving[subject]
                .iter()
                .map(|attribute| attribute.value.as_str())
                .collect();

            let mut target: Option<(usize, String)> = None;
            for (other, other_parts) in &parsed {
                if other == subject || other_parts.stem != parts.stem {
                    continue;
                }
                let theirs: BTreeSet<&str> = surviving[other]
                    .iter()
                    .map(|attribute| attribute.value.as_str())
                    .collect();
                if theirs.len() >= mine.len() || !theirs.is_subset(&mine) {
                    continue;
                }
                let name = reassemble(other_parts, &surviving[other]);
                match &target {
                    Some(best) if *best <= (theirs.len(), name.clone()) => {}
                    _ => target = Some((theirs.len(), name)),
                }
            }

            let name = match target {
                Some((_, witnessed)) => witnessed,
                None => reassemble(parts, &surviving[subject]),
            };
            // Never trade a real name for an empty one.
            let name = if name.trim().is_empty() {
                subject.clone()
            } else {
                name
            };
            canonical.insert(subject.clone(), name);
        }

        Self {
            canonical,
            founding: BTreeMap::new(),
        }
    }

    /// The canonical name of a witnessed subject, or the subject itself when it
    /// was never witnessed.
    pub fn canonical<'a>(&'a self, subject: &'a str) -> &'a str {
        self.canonical
            .get(subject)
            .map(String::as_str)
            .unwrap_or(subject)
    }

    /// The witnessed names that resolved to this canonical one, in canonical
    /// order.
    ///
    /// This is the evidence for an identity claim. A resource standing for eight
    /// witnessed names is a conclusion Evo drew, and an explanation that cannot
    /// show its working is not explainable.
    pub fn witnessed_as<'a>(&'a self, canonical: &'a str) -> impl Iterator<Item = &'a String> {
        self.canonical
            .iter()
            .filter(move |(_, name)| name.as_str() == canonical)
            .map(|(witnessed, _)| witnessed)
    }

    /// How many distinct witnessed names resolved to this one.
    pub fn sighting_count(&self, canonical: &str) -> usize {
        self.witnessed_as(canonical).count()
    }

    /// Rewrites acts to speak about resources rather than sightings, and records
    /// which sighting of each resource came first.
    ///
    /// Timing, ordering, and act character are untouched: only the name each act
    /// is about changes, so nothing about *when* or *how* something was
    /// witnessed is altered by learning what it was.
    ///
    /// The founding sighting is recorded here because this is the only point at
    /// which the index sees witnessed names *together with the moments they were
    /// witnessed at*. [`learn`](Self::learn) is deliberately a pure function of the
    /// set of names and has no times to work from.
    pub fn fold_acts(&mut self, acts: Vec<Act>) -> Vec<Act> {
        let mut earliest: BTreeMap<String, (SystemTime, &str)> = BTreeMap::new();
        for act in &acts {
            let witnessed = act.resource().subject();
            let sighting = (act.at(), witnessed);
            match earliest.get(self.canonical(witnessed)) {
                Some(known) if *known <= sighting => {}
                _ => {
                    earliest.insert(self.canonical(witnessed).to_string(), sighting);
                }
            }
        }
        self.founding = earliest
            .into_iter()
            .map(|(canonical, (_, witnessed))| (canonical, witnessed.to_string()))
            .collect();

        acts.into_iter()
            .map(|act| {
                let resource = act.resource();
                let folded = resource.renamed(self.canonical(resource.subject()));
                Act::new(folded, act.character(), act.at())
            })
            .collect()
    }

    /// The witnessed name this resource was *first* seen under.
    ///
    /// # Why this exists
    ///
    /// A canonical name is a conclusion, and conclusions here improve as the corpus
    /// grows: the moment a second sighting reveals that `— Editor` was decoration
    /// rather than part of a name, every name carrying it folds to something
    /// shorter. That is the identity layer working correctly, and it means a
    /// canonical name is **not** a stable key.
    ///
    /// A founding sighting is not a conclusion — it is a witnessed fact, the exact
    /// string Evo saw at the earliest moment it saw this thing at all. Appending to
    /// the history cannot change what came earliest. Anything upstairs that needs a
    /// durable handle on a resource must key on this rather than on the canonical
    /// name.
    ///
    /// Returns `None` for a resource known only from a declaration, which carries
    /// no moment and therefore founded nothing.
    pub fn founding_name<'a>(&'a self, canonical: &str) -> Option<&'a str> {
        self.founding.get(canonical).map(String::as_str)
    }

    /// Rewrites declarations onto canonical names.
    ///
    /// A person naming a resource named a *thing*, not the state its title
    /// happened to be in at that instant, so their statement must follow the
    /// thing. Without this, designating a document while it displayed page six
    /// would bind the declaration to a name nothing else resolves to, and the
    /// ground-truth override would silently stop applying.
    pub fn fold_declarations(&self, declarations: &Declarations) -> Declarations {
        let mut folded = Declarations::new();
        for subject in declarations.designations() {
            folded.designate(self.canonical(subject));
        }
        for (first, second) in declarations.groupings() {
            folded.group(self.canonical(first), self.canonical(second));
        }
        let continuation: Vec<String> = declarations
            .continuations()
            .map(|subject| self.canonical(subject).to_string())
            .collect();
        if !continuation.is_empty() {
            // The moment travels with the surface. A held surface always has
            // one, so the fallback is unreachable; it exists because the two
            // fields are only meaningful together.
            folded.continue_from(
                continuation,
                declarations
                    .continuation_witnessed_at()
                    .unwrap_or(std::time::UNIX_EPOCH),
            );
        }
        for (member, container) in declarations.containments() {
            folded.contain(self.canonical(member), self.canonical(container));
        }
        folded
    }
}

/// Splits a witnessed name into a stem and its attributes.
fn parse(subject: &str) -> Parts {
    parse_address(subject).unwrap_or_else(|| parse_name(subject))
}

/// Addresses split on their own structure: query parameters and fragment are
/// attributes of the location before them.
fn parse_address(subject: &str) -> Option<Parts> {
    if !subject.contains("://") {
        return None;
    }
    let (head, fragment) = match subject.split_once('#') {
        Some((head, fragment)) => (head, Some(fragment)),
        None => (subject, None),
    };
    let (stem, query) = match head.split_once('?') {
        Some((stem, query)) => (stem, Some(query)),
        None => (head, None),
    };

    let mut attributes = Vec::new();
    if let Some(query) = query {
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            attributes.push(Attribute {
                value: pair.to_string(),
                separator: "&",
            });
        }
    }
    if let Some(fragment) = fragment {
        attributes.push(Attribute {
            value: fragment.to_string(),
            separator: "#",
        });
    }

    Some(Parts {
        stem: stem.to_string(),
        attributes,
        addressed: true,
    })
}

/// Names split on appended-decoration punctuation, left to right. The stem is
/// everything before the first separator.
fn parse_name(subject: &str) -> Parts {
    let mut attributes = Vec::new();
    let mut stem = subject;

    if let Some((head, separator, tail)) = split_once_any(subject) {
        stem = head;
        let mut rest = tail;
        let mut pending = separator;
        loop {
            match split_once_any(rest) {
                Some((head, separator, tail)) => {
                    attributes.push(Attribute {
                        value: head.to_string(),
                        separator: pending,
                    });
                    pending = separator;
                    rest = tail;
                }
                None => {
                    attributes.push(Attribute {
                        value: rest.to_string(),
                        separator: pending,
                    });
                    break;
                }
            }
        }
    }

    Parts {
        stem: stem.to_string(),
        attributes,
        addressed: false,
    }
}

/// The earliest separator occurrence in `text`, if any.
fn split_once_any(text: &str) -> Option<(&str, &'static str, &str)> {
    let mut best: Option<(usize, &'static str)> = None;
    for separator in SEPARATORS {
        if let Some(index) = text.find(separator) {
            match best {
                Some((current, _)) if current <= index => {}
                _ => best = Some((index, separator)),
            }
        }
    }
    let (index, separator) = best?;
    Some((&text[..index], separator, &text[index + separator.len()..]))
}

/// Puts a name back together from the attributes that survived.
fn reassemble(parts: &Parts, kept: &[&Attribute]) -> String {
    let mut name = parts.stem.clone();

    if !parts.addressed {
        for attribute in kept {
            name.push_str(attribute.separator);
            name.push_str(&attribute.value);
        }
        return name;
    }

    let mut query: Vec<&str> = Vec::new();
    let mut fragment: Option<&str> = None;
    for attribute in kept {
        if attribute.separator == "#" {
            fragment = Some(&attribute.value);
        } else {
            query.push(&attribute.value);
        }
    }
    if !query.is_empty() {
        name.push('?');
        name.push_str(&query.join("&"));
    }
    if let Some(fragment) = fragment {
        name.push('#');
        name.push_str(fragment);
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::{ActCharacter, Resource};

    fn index(subjects: &[&str]) -> IdentityIndex {
        IdentityIndex::learn(subjects.iter().copied())
    }

    /// The failure that motivated this module: one window, whose title carries
    /// the memory the browser was using and whether audio was playing, is one
    /// resource — not four.
    #[test]
    fn one_window_reporting_its_state_is_one_resource() {
        let subjects = [
            "Job assignment analysis - Claude - High memory usage - 1.4 GB - Google Chrome – BHAVIT",
            "Job assignment analysis - Claude - High memory usage - 1.3 GB - Google Chrome – BHAVIT",
            "Job assignment analysis - Claude – Audio playing - Google Chrome – BHAVIT",
            "Inbox - Gmail - High memory usage - 942 MB - Google Chrome – BHAVIT",
            "Free To Play - Store - Google Chrome – BHAVIT",
        ];
        let index = index(&subjects);

        let first = index.canonical(subjects[0]);
        assert_eq!(index.canonical(subjects[1]), first);
        assert_eq!(index.canonical(subjects[2]), first);
        assert_eq!(index.sighting_count(first), 3);

        // The browser, the account, and the status are gone; what a person would
        // recognise is what remains.
        assert_eq!(first, "Job assignment analysis - Claude");

        // The other two windows were each witnessed once, so there is no
        // evidence about which of *their* parts vary. Only the decorations
        // proven by appearing under other stems come off — a byte count seen
        // once is indistinguishable from a name seen once, and Evo keeps what it
        // cannot rule out.
        assert_eq!(index.canonical(subjects[3]), "Inbox - Gmail - 942 MB");
        assert_eq!(index.canonical(subjects[4]), "Free To Play - Store");
    }

    /// A document read page by page is one document. Each page number appears in
    /// exactly one witnessed name, so no page describes the document.
    #[test]
    fn a_document_read_page_by_page_is_one_document() {
        let subjects = [
            "Prep.pdf – Page 1 of 7",
            "Prep.pdf – Page 3 of 7",
            "Prep.pdf – Page 6 of 7",
            "Prep.pdf – Page 7 of 7",
        ];
        let index = index(&subjects);
        for subject in subjects {
            assert_eq!(index.canonical(subject), "Prep.pdf");
        }
        assert_eq!(index.sighting_count("Prep.pdf"), 4);
    }

    /// Per-request stamps in an address identify the request, not the page. What
    /// was actually asked for survives, because it was the same every time the
    /// person came back to it.
    #[test]
    fn per_sighting_address_stamps_are_dropped_and_the_query_survives() {
        let subjects = [
            "https://example.test/search?q=strip&ei=aaa&sxsrf=111",
            "https://example.test/search?q=strip&ei=bbb&sxsrf=222",
            "https://example.test/search?q=strip&ei=ccc&sxsrf=333",
        ];
        let index = index(&subjects);
        for subject in subjects {
            assert_eq!(
                index.canonical(subject),
                "https://example.test/search?q=strip"
            );
        }
    }

    /// Two different searches at one address are two things. Counting names
    /// rather than positions is what makes this work: each query survives
    /// because it was witnessed more than once, however many other searches
    /// share the address.
    #[test]
    fn two_searches_at_one_address_stay_two_things() {
        let subjects = [
            "https://example.test/search?q=lease+termination&ei=aa",
            "https://example.test/search?q=lease+termination&ei=bb",
            "https://example.test/search?q=stamp+duty&ei=cc",
            "https://example.test/search?q=stamp+duty&ei=dd",
        ];
        let index = index(&subjects);
        assert_eq!(
            index.canonical(subjects[0]),
            "https://example.test/search?q=lease+termination"
        );
        assert_eq!(index.canonical(subjects[1]), index.canonical(subjects[0]));
        assert_eq!(
            index.canonical(subjects[2]),
            "https://example.test/search?q=stamp+duty"
        );
        assert_ne!(index.canonical(subjects[0]), index.canonical(subjects[2]));
    }

    /// Two meetings that share an application prefix are two meetings. Where the
    /// evidence cannot settle whether an attribute is a name or a state, it is
    /// kept — the mistake that merges two real things is the unrecoverable one.
    #[test]
    fn two_things_sharing_a_prefix_stay_two_things() {
        let subjects = [
            "Meet - kxc-vads-gdp – BHAVIT",
            "Meet - xae-jgzs-xsx – BHAVIT",
        ];
        let index = index(&subjects);
        assert_ne!(index.canonical(subjects[0]), index.canonical(subjects[1]));
        assert!(index.canonical(subjects[0]).contains("kxc-vads-gdp"));
        assert!(index.canonical(subjects[1]).contains("xae-jgzs-xsx"));
    }

    /// A name witnessed once tells us nothing about which of its parts vary, so
    /// nothing is dropped. Evo does not guess from a single sighting.
    #[test]
    fn a_name_seen_once_is_left_exactly_as_witnessed() {
        let subject = "Quarterly Numbers - Ledger - 4 unsaved changes";
        let index = index(&[subject]);
        assert_eq!(index.canonical(subject), subject);
    }

    /// **Adversarial: accidental merging.** Three documents behind one reader's
    /// generic prefix are three documents, however many of them there are. Their
    /// names differ in their *letters*, so they are three fields holding one
    /// value each — there is no witnessed field that varies, and no shorter name
    /// was ever seen.
    ///
    /// This is the failure the previous rule had, and the reason identity had to
    /// stop counting how many names an attribute appeared in: a document opened
    /// once was indistinguishable from a page number.
    #[test]
    fn two_documents_behind_one_prefix_stay_two_documents() {
        let subjects = [
            "Reader - contract.pdf",
            "Reader - lease.pdf",
            "Reader - deed.pdf",
        ];
        let three = index(&subjects);
        assert_eq!(three.canonical(subjects[0]), "Reader - contract.pdf");
        assert_ne!(three.canonical(subjects[1]), three.canonical(subjects[2]));
        assert_eq!(three.sighting_count("Reader - contract.pdf"), 1);

        // And adding a fourth does not tip it over: there is no threshold to
        // cross, because nothing about these names says a field varies.
        let wider = index(&[
            "Reader - contract.pdf",
            "Reader - lease.pdf",
            "Reader - deed.pdf",
            "Reader - invoice.pdf",
            "Reader - warranty.pdf",
        ]);
        assert_eq!(wider.canonical(subjects[0]), "Reader - contract.pdf");
    }

    /// **Adversarial: accidental merging.** Many meetings behind one application
    /// prefix stay many meetings. Meeting codes differ in their letters, so no
    /// field is shown to vary; and `Meet` alone was never witnessed, so rule 3
    /// has nothing to fold onto.
    #[test]
    fn many_meetings_sharing_a_prefix_stay_many_meetings() {
        let subjects = [
            "Meet - kxc-vads-gdp",
            "Meet - xae-jgzs-xsx",
            "Meet - qrb-mnop-tuv",
            "Meet - zzy-aabb-ccd",
        ];
        let index = index(&subjects);
        let canonical: BTreeSet<&str> = subjects
            .iter()
            .map(|subject| index.canonical(subject))
            .collect();
        assert_eq!(canonical.len(), 4, "four meetings, four resources");
    }

    /// **Adversarial: accidental merging.** Forty searches at one address are
    /// forty things. A query that came back a second time is something the
    /// person asked for; the per-request stamps beside it, each seen exactly
    /// once, are not.
    ///
    /// Measured on this machine's real history: one address carried 41 distinct
    /// searches, and the rule this replaced folded five of them into a single
    /// resource named after nothing but the stamps they had in common.
    #[test]
    fn many_searches_at_one_address_stay_many_searches() {
        let mut subjects: Vec<String> = Vec::new();
        for index in 0..8 {
            // Each query asked for twice, each stamp fresh — the shape real
            // search history has.
            subjects.push(format!(
                "https://example.test/search?q=topic{index}&ei=first{index}&sourceid=chrome"
            ));
            subjects.push(format!(
                "https://example.test/search?q=topic{index}&ei=second{index}&sourceid=chrome"
            ));
        }
        let borrowed: Vec<&str> = subjects.iter().map(String::as_str).collect();
        let index = IdentityIndex::learn(borrowed.iter().copied());

        let canonical: BTreeSet<&str> = borrowed
            .iter()
            .map(|subject| index.canonical(subject))
            .collect();
        assert_eq!(canonical.len(), 8, "eight questions asked, eight resources");
        assert_eq!(
            index.canonical(&subjects[0]),
            "https://example.test/search?q=topic0&sourceid=chrome",
            "the stamp is gone, the question and the stable parameter remain"
        );
    }

    /// **Adversarial: fragmentation.** The real seven-name window from this
    /// machine's history, verbatim. The byte count repeats — 1.2 GB was
    /// witnessed twice — so a rule that only drops values seen exactly once
    /// leaves five resources where there is one thing. Measured: it did.
    #[test]
    fn a_repeated_readout_still_does_not_split_a_window() {
        let subjects = [
            "Job assignment analysis and deadline - Claude - High memory usage - 1.2 GB - Google Chrome – BHAVIT",
            "Job assignment analysis and deadline - Claude - High memory usage - 1.3 GB - Google Chrome – BHAVIT",
            "Job assignment analysis and deadline - Claude - High memory usage - 1.4 GB - Google Chrome – BHAVIT",
            "Job assignment analysis and deadline - Claude - High memory usage - 1.5 GB - Google Chrome – BHAVIT",
            "Job assignment analysis and deadline - Claude - High memory usage - 1.6 GB - Google Chrome – BHAVIT",
            "Job assignment analysis and deadline - Claude – Audio playing - High memory usage - 1.2 GB - Google Chrome – BHAVIT",
            "Job assignment analysis and deadline - Claude – Audio playing - High memory usage - 1.5 GB - Google Chrome – BHAVIT",
            // Another window, so "High memory usage" is provably decoration.
            "Automation Engineering Intern Application – Bhavit Saini - Gmail - High memory usage - 848 MB - Google Chrome – BHAVIT",
        ];
        let index = index(&subjects);
        let first = index.canonical(subjects[0]);
        for subject in &subjects[..7] {
            assert_eq!(index.canonical(subject), first);
        }
        assert_eq!(first, "Job assignment analysis and deadline - Claude");
        assert_eq!(index.sighting_count(first), 7);
    }

    /// **Adversarial: fragmentation.** A page readout that repeats is still a
    /// page readout, and the thirteen-name document from real history is one
    /// document.
    #[test]
    fn a_document_read_page_by_page_with_repeats_is_one_document() {
        let mut subjects: Vec<String> = Vec::new();
        for page in [3, 5, 6, 7, 8, 9, 12, 15, 16, 20, 21, 22, 23] {
            subjects.push(format!("walkthrough.md.pdf – Page {page} of 27"));
        }
        subjects.push("walkthrough.md.pdf – Page 5 of 27".to_string());
        let borrowed: Vec<&str> = subjects.iter().map(String::as_str).collect();
        let index = IdentityIndex::learn(borrowed);
        for subject in &subjects {
            assert_eq!(index.canonical(subject), "walkthrough.md.pdf");
        }
    }

    /// **Adversarial: fragmentation.** State that carries no digits — whether
    /// audio is playing, whether a microphone is live — is caught by rule 3 and
    /// only by rule 3: Evo witnessed the same stem named without it.
    #[test]
    fn state_without_digits_folds_onto_the_name_witnessed_without_it() {
        let subjects = [
            "quarterly model.xlsx — Sheet 1",
            "quarterly model.xlsx — Sheet 1 — Recalculating",
        ];
        let index = index(&subjects);
        assert_eq!(
            index.canonical(subjects[1]),
            "quarterly model.xlsx — Sheet 1"
        );
        assert_eq!(index.sighting_count("quarterly model.xlsx — Sheet 1"), 2);
    }

    /// Rule 3 folds only onto names really witnessed, so an absence can never
    /// produce a merge: the shorter name has to exist.
    #[test]
    fn subsumption_needs_the_shorter_name_to_have_been_seen() {
        let never_bare = index(&[
            "Session - alpha - Recording",
            "Session - beta - Recording",
        ]);
        assert_ne!(
            never_bare.canonical("Session - alpha - Recording"),
            never_bare.canonical("Session - beta - Recording")
        );

        let bare_seen = index(&[
            "Session - alpha",
            "Session - alpha - Recording",
            "Session - beta",
        ]);
        assert_eq!(
            bare_seen.canonical("Session - alpha - Recording"),
            "Session - alpha"
        );
        assert_ne!(
            bare_seen.canonical("Session - alpha"),
            bare_seen.canonical("Session - beta")
        );
    }

    /// Dropping a segment shifts every later segment left. Nothing here may
    /// depend on position, so a name missing its middle must still resolve with
    /// the rest.
    #[test]
    fn a_missing_segment_does_not_disturb_the_others() {
        let subjects = [
            "Ledger - Sheet - 1 unsaved change - Numbers",
            "Ledger - Sheet - 2 unsaved changes - Numbers",
            "Ledger - Sheet - Numbers",
            "Budget - Sheet - Numbers",
        ];
        let index = index(&subjects);
        let first = index.canonical(subjects[0]);
        assert_eq!(index.canonical(subjects[1]), first);
        assert_eq!(index.canonical(subjects[2]), first);
        assert_eq!(first, "Ledger");
        assert_eq!(index.canonical(subjects[3]), "Budget");
    }

    /// Learning cannot depend on the order names arrived in.
    #[test]
    fn identity_is_independent_of_input_order() {
        let forward = [
            "Report.pdf – Page 1 of 3",
            "Report.pdf – Page 2 of 3",
            "Report.pdf – Page 3 of 3",
            "Notes - Editor - Untracked",
            "Draft - Editor - Untracked",
        ];
        let mut backward: Vec<&str> = forward.to_vec();
        backward.reverse();

        let first = index(&forward);
        let second = IdentityIndex::learn(backward);
        for subject in forward {
            assert_eq!(first.canonical(subject), second.canonical(subject));
        }
    }

    /// A path containing separator punctuation in its own name is not
    /// dismantled, because nothing in the corpus shows those parts varying.
    #[test]
    fn punctuation_inside_a_stable_name_is_not_mistaken_for_decoration() {
        let subject = "/Users/x/notes/2026 - annual review.md";
        let index = index(&[subject, "/Users/x/notes/todo.md"]);
        assert_eq!(index.canonical(subject), subject);
    }

    /// Identity is recovered from names only. No application, domain, extension,
    /// or category appears in this module, so a resource type Evo has never seen
    /// folds by the same two rules with no new code.
    #[test]
    fn an_unfamiliar_resource_type_folds_by_the_same_rules() {
        let subjects = [
            "housing-block.step — Assembly — 41% loaded",
            "housing-block.step — Assembly — 78% loaded",
            "housing-block.step — Assembly — 100% loaded",
            "bracket.step — Assembly — 100% loaded",
        ];
        let index = index(&subjects);
        let first = index.canonical(subjects[0]);
        assert_eq!(index.canonical(subjects[1]), first);
        assert_eq!(index.canonical(subjects[2]), first);
        assert_eq!(first, "housing-block.step");
        assert_eq!(index.canonical(subjects[3]), "bracket.step");
    }

    /// A statement the person made must follow the thing it was about, not the
    /// title the thing wore at that instant.
    #[test]
    fn a_statement_follows_the_resource_it_named() {
        let subjects = [
            "Deposition.pdf – Page 2 of 9",
            "Deposition.pdf – Page 5 of 9",
            "Deposition.pdf – Page 9 of 9",
        ];
        let index = index(&subjects);

        let mut stated = Declarations::new();
        stated.designate(subjects[1]);
        stated.group(subjects[0], "/Users/x/case/notes.md");

        let folded = index.fold_declarations(&stated);
        assert!(folded.is_designated("Deposition.pdf"));
        assert!(folded.is_grouped("Deposition.pdf", "/Users/x/case/notes.md"));
    }

    fn act(subject: &str, secs: u64) -> Act {
        Act::new(
            Resource::new(subject, "OBS-WINDOW-FOCUS-GAINED"),
            ActCharacter::Attentional,
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs),
        )
    }

    /// A founding sighting is the earliest witnessed name, not the earliest
    /// canonical one.
    #[test]
    fn a_founding_sighting_is_the_name_first_witnessed() {
        let mut index = index(&[
            "Deposition.pdf – Page 5 of 9",
            "Deposition.pdf – Page 2 of 9",
            "Deposition.pdf – Page 9 of 9",
        ]);
        let _ = index.fold_acts(vec![
            act("Deposition.pdf – Page 5 of 9", 900),
            act("Deposition.pdf – Page 2 of 9", 100),
            act("Deposition.pdf – Page 9 of 9", 1500),
        ]);

        assert_eq!(index.canonical("Deposition.pdf – Page 2 of 9"), "Deposition.pdf");
        assert_eq!(
            index.founding_name("Deposition.pdf"),
            Some("Deposition.pdf – Page 2 of 9"),
            "the earliest sighting founds the resource, whatever order acts arrive in"
        );
    }

    /// The property upper layers depend on, and the reason [`IdentityIndex::founding_name`]
    /// exists at all.
    ///
    /// A canonical name is a conclusion drawn from the corpus, so growing the
    /// corpus can change it: the second `— Editor` proves the first was
    /// decoration. Anything keyed on the canonical name moves when that happens.
    /// The founding sighting does not, because what came earliest in an
    /// append-only history is settled once it is true.
    #[test]
    fn a_founding_sighting_survives_the_canonical_name_changing() {
        let alone = ["thesis chapter three — Editor"];
        let mut before = index(&alone);
        let _ = before.fold_acts(vec![act(alone[0], 0)]);

        let together = ["thesis chapter three — Editor", "thesis appendix draft — Editor"];
        let mut after = index(&together);
        let _ = after.fold_acts(vec![act(together[0], 0), act(together[1], 600)]);

        // The conclusion improved, exactly as it should.
        assert_eq!(before.canonical(alone[0]), "thesis chapter three — Editor");
        assert_eq!(after.canonical(alone[0]), "thesis chapter three");
        assert_ne!(before.canonical(alone[0]), after.canonical(alone[0]));

        // The witnessed fact did not.
        assert_eq!(
            before.founding_name(before.canonical(alone[0])),
            after.founding_name(after.canonical(alone[0]))
        );
        assert_eq!(
            after.founding_name("thesis chapter three"),
            Some("thesis chapter three — Editor")
        );
    }

    /// A resource named only in a declaration was never witnessed, so it founded
    /// nothing — and saying so is better than inventing a sighting for it.
    #[test]
    fn a_resource_known_only_from_a_statement_founds_nothing() {
        let mut index = index(&["/Users/x/case/notes.md"]);
        let _ = index.fold_acts(vec![]);
        assert_eq!(index.founding_name("/Users/x/case/notes.md"), None);
    }
}
