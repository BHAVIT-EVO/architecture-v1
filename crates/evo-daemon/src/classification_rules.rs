//! Deterministic, user-authored surface classification rules.
//!
//! This module deliberately contains no application, domain, file-extension, or
//! productivity knowledge. A rule only compares a user-provided string with a
//! structural field that capture witnessed for an interval. The caller supplies
//! the interval subject explicitly; this module never derives one from another
//! field.
//!
//! The evaluator is pure. It returns the classifications that a caller may send
//! through the canonical observation acceptance path, but it does not append to
//! the observation log or invent persistence semantics.

use evo_observation::observation_language::ObservationConcept;

/// A structural field that capture may have witnessed for an interval.
///
/// `Repo` is intentionally an explicit witnessed value. A repository is never
/// inferred from a path, URL, app, or title by this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructuralField {
    App,
    WindowTitle,
    Url,
    DocumentPath,
    Repo,
}

/// The literal comparison a user-authored matcher performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MatchOperator {
    /// The witnessed value is exactly the rule value.
    Equals,
    /// `*` matches any sequence and `?` matches one character. The matcher is
    /// deliberately a small, deterministic glob rather than a regex engine:
    /// rule values remain auditable user strings and this module adds no
    /// application/domain vocabulary.
    Wildcard,
    /// The witnessed value contains the rule value as a substring.
    Contains,
    /// The witnessed value starts with the rule value.
    Prefix,
    /// The witnessed value ends with the rule value.
    Suffix,
}

/// Short name for callers that prefer the field name used in a rule document.
pub type MatchOp = MatchOperator;

/// One user-authored structural matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationMatcher {
    pub field: StructuralField,
    pub op: MatchOperator,
    /// Compared literally. No URL parsing, case folding, or domain lookup is
    /// performed.
    pub value: String,
}

impl ClassificationMatcher {
    pub fn new(field: StructuralField, op: MatchOperator, value: impl Into<String>) -> Self {
        Self {
            field,
            op,
            value: value.into(),
        }
    }

    fn validate(&self, rule_id: &str) -> Result<(), ClassificationError> {
        if self.value.trim().is_empty() {
            return Err(ClassificationError::EmptyMatcherValue {
                rule_id: rule_id.to_string(),
            });
        }
        Ok(())
    }

    /// Tests the matcher against only the selected witnessed field.
    pub fn matches(&self, fields: &WitnessedFields) -> bool {
        let values = match self.field {
            StructuralField::App => &fields.apps,
            StructuralField::WindowTitle => &fields.window_titles,
            StructuralField::Url => &fields.urls,
            StructuralField::DocumentPath => &fields.document_paths,
            StructuralField::Repo => &fields.repos,
        };

        values.iter().any(|witnessed| match self.op {
            MatchOperator::Equals => witnessed == &self.value,
            MatchOperator::Wildcard => wildcard_matches(&self.value, witnessed),
            MatchOperator::Contains => witnessed.contains(&self.value),
            MatchOperator::Prefix => witnessed.starts_with(&self.value),
            MatchOperator::Suffix => witnessed.ends_with(&self.value),
        })
    }
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let value: Vec<char> = value.chars().collect();
    let mut states = vec![false; value.len() + 1];
    states[0] = true;
    for token in pattern {
        let mut next = vec![false; value.len() + 1];
        match token {
            '*' => {
                next[0] = states[0];
                for index in 1..=value.len() {
                    next[index] = states[index] || next[index - 1];
                }
            }
            '?' => {
                for index in 1..=value.len() {
                    next[index] = states[index - 1];
                }
            }
            literal => {
                for index in 1..=value.len() {
                    next[index] = states[index - 1] && value[index - 1] == literal;
                }
            }
        }
        states = next;
    }
    states[value.len()]
}

/// The assignment made when a rule matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationAssignment {
    /// The person's class label, carried unchanged into
    /// `OBS-SURFACE-CLASSIFIED`.
    pub class: String,
}

impl ClassificationAssignment {
    pub fn new(class: impl Into<String>) -> Self {
        Self {
            class: class.into(),
        }
    }
}

/// The only rule application mode currently supported by Phase 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleApplication {
    /// Evaluate against all supplied history on every replay.
    Retroactive,
}

/// A user-authored classification rule.
///
/// Lower `priority` values win. Equal priorities are resolved by `id`, which
/// keeps evaluation stable even when rules are supplied in different orders.
/// `absorb_max_gap` is measured in milliseconds and belongs to this rule; zero
/// disables filling across a positive temporal gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationRule {
    pub id: String,
    pub priority: u32,
    pub enabled: bool,
    pub matcher: ClassificationMatcher,
    pub assign: ClassificationAssignment,
    pub apply: RuleApplication,
    pub absorb_max_gap: u64,
}

impl ClassificationRule {
    /// Creates an enabled retroactive rule with no absorb gap.
    pub fn new(
        id: impl Into<String>,
        priority: u32,
        matcher: ClassificationMatcher,
        class: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            priority,
            enabled: true,
            matcher,
            assign: ClassificationAssignment::new(class),
            apply: RuleApplication::Retroactive,
            absorb_max_gap: 0,
        }
    }

    pub fn with_absorb_max_gap(mut self, gap_ms: u64) -> Self {
        self.absorb_max_gap = gap_ms;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    fn validate(&self) -> Result<(), ClassificationError> {
        if self.id.trim().is_empty() {
            return Err(ClassificationError::EmptyRuleId);
        }
        if self.assign.class.trim().is_empty() {
            return Err(ClassificationError::EmptyClass {
                rule_id: self.id.clone(),
            });
        }
        self.matcher.validate(&self.id)
    }
}

/// Structural values witnessed during one interval.
///
/// Multiple values are retained because an interval can contain more than one
/// URL, title, document, app, or explicitly witnessed repository. Empty
/// vectors mean that capture did not witness that field; they are not a reason
/// to guess one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WitnessedFields {
    pub apps: Vec<String>,
    pub window_titles: Vec<String>,
    pub urls: Vec<String>,
    pub document_paths: Vec<String>,
    pub repos: Vec<String>,
}

impl WitnessedFields {
    pub fn with_app(mut self, value: impl Into<String>) -> Self {
        self.apps.push(value.into());
        self
    }

    pub fn with_window_title(mut self, value: impl Into<String>) -> Self {
        self.window_titles.push(value.into());
        self
    }

    pub fn with_url(mut self, value: impl Into<String>) -> Self {
        self.urls.push(value.into());
        self
    }

    pub fn with_document_path(mut self, value: impl Into<String>) -> Self {
        self.document_paths.push(value.into());
        self
    }

    /// Adds a repository only when capture witnessed repository membership.
    pub fn with_repo(mut self, value: impl Into<String>) -> Self {
        self.repos.push(value.into());
        self
    }
}

/// One interval supplied to the pure evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationInterval {
    pub id: u64,
    pub start_ms: u64,
    pub end_ms: u64,
    /// The already-established surface that a generated classification refers
    /// to. It is not derived from the matcher field.
    pub subject: String,
    pub fields: WitnessedFields,
}

impl ClassificationInterval {
    pub fn new(
        id: u64,
        start_ms: u64,
        end_ms: u64,
        subject: impl Into<String>,
        fields: WitnessedFields,
    ) -> Self {
        Self {
            id,
            start_ms,
            end_ms,
            subject: subject.into(),
            fields,
        }
    }
}

/// Why an interval received a classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationSource {
    Direct,
    Absorbed,
}

/// A classification derived for one interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervalClassification {
    pub interval_id: u64,
    pub subject: String,
    pub class: String,
    pub rule_id: String,
    pub source: ClassificationSource,
}

impl IntervalClassification {
    /// Converts the derived result to the existing canonical concept. This is
    /// intentionally only a concept conversion: provenance, acceptance, ID
    /// assignment, and persistence remain the caller's responsibility.
    pub fn observation_concept(&self) -> ObservationConcept {
        ObservationConcept::SurfaceClassified {
            subject: self.subject.clone(),
            class: self.class.clone(),
        }
    }
}

/// Validation and input errors for the pure rules evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassificationError {
    EmptyRuleId,
    DuplicateRuleId {
        rule_id: String,
    },
    EmptyMatcherValue {
        rule_id: String,
    },
    EmptyClass {
        rule_id: String,
    },
    InvalidIntervalBounds {
        interval_id: u64,
        start_ms: u64,
        end_ms: u64,
    },
    EmptyIntervalSubject {
        interval_id: u64,
    },
}

impl std::fmt::Display for ClassificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassificationError::EmptyRuleId => write!(f, "classification rule id is empty"),
            ClassificationError::DuplicateRuleId { rule_id } => {
                write!(f, "classification rule id is duplicated: {rule_id}")
            }
            ClassificationError::EmptyMatcherValue { rule_id } => {
                write!(
                    f,
                    "classification rule {rule_id} has an empty matcher value"
                )
            }
            ClassificationError::EmptyClass { rule_id } => {
                write!(f, "classification rule {rule_id} has an empty class")
            }
            ClassificationError::InvalidIntervalBounds {
                interval_id,
                start_ms,
                end_ms,
            } => write!(
                f,
                "classification interval {interval_id} ends before it starts ({start_ms} > {end_ms})"
            ),
            ClassificationError::EmptyIntervalSubject { interval_id } => {
                write!(f, "classification interval {interval_id} has no subject")
            }
        }
    }
}

impl std::error::Error for ClassificationError {}

/// An ordered collection of user-authored rules.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassificationRules {
    rules: Vec<ClassificationRule>,
}

impl ClassificationRules {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new(rules: Vec<ClassificationRule>) -> Result<Self, ClassificationError> {
        let mut set = Self::empty();
        for rule in rules {
            set.push(rule)?;
        }
        Ok(set)
    }

    pub fn push(&mut self, rule: ClassificationRule) -> Result<(), ClassificationError> {
        rule.validate()?;
        if self.rules.iter().any(|existing| existing.id == rule.id) {
            return Err(ClassificationError::DuplicateRuleId { rule_id: rule.id });
        }
        self.rules.push(rule);
        Ok(())
    }

    pub fn rules(&self) -> &[ClassificationRule] {
        &self.rules
    }

    /// Evaluates all intervals in deterministic chronological order.
    ///
    /// An unclassified run is filled only when its immediate direct
    /// classifications are produced by the same rule, carry the same class,
    /// and the elapsed span from the left interval's end to the right
    /// interval's start is within that rule's `absorb_max_gap`.
    pub fn evaluate(
        &self,
        intervals: &[ClassificationInterval],
    ) -> Result<Vec<IntervalClassification>, ClassificationError> {
        for interval in intervals {
            if interval.end_ms < interval.start_ms {
                return Err(ClassificationError::InvalidIntervalBounds {
                    interval_id: interval.id,
                    start_ms: interval.start_ms,
                    end_ms: interval.end_ms,
                });
            }
            if interval.subject.trim().is_empty() {
                return Err(ClassificationError::EmptyIntervalSubject {
                    interval_id: interval.id,
                });
            }
        }

        let mut ordered: Vec<usize> = (0..intervals.len()).collect();
        ordered.sort_by_key(|index| {
            let interval = &intervals[*index];
            (interval.start_ms, interval.end_ms, interval.id, *index)
        });

        let mut rules: Vec<&ClassificationRule> = self.rules.iter().filter(|r| r.enabled).collect();
        rules.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));

        let mut direct: Vec<Option<DirectClassification>> = Vec::with_capacity(ordered.len());
        for index in &ordered {
            let interval = &intervals[*index];
            direct.push(
                rules
                    .iter()
                    .find(|rule| rule.matcher.matches(&interval.fields))
                    .map(|rule| DirectClassification {
                        rule_id: rule.id.clone(),
                        class: rule.assign.class.clone(),
                        absorb_max_gap: rule.absorb_max_gap,
                    }),
            );
        }

        let mut resolved: Vec<Option<IntervalClassification>> = vec![None; ordered.len()];
        for (position, direct_match) in direct.iter().enumerate() {
            if let Some(direct_match) = direct_match {
                let interval = &intervals[ordered[position]];
                resolved[position] = Some(IntervalClassification {
                    interval_id: interval.id,
                    subject: interval.subject.clone(),
                    class: direct_match.class.clone(),
                    rule_id: direct_match.rule_id.clone(),
                    source: ClassificationSource::Direct,
                });
            }
        }

        let mut run_start = 0;
        while run_start < ordered.len() {
            if direct[run_start].is_some() {
                run_start += 1;
                continue;
            }

            let mut run_end = run_start;
            while run_end < ordered.len() && direct[run_end].is_none() {
                run_end += 1;
            }

            let fill = if run_start > 0 && run_end < ordered.len() {
                let left = direct[run_start - 1].as_ref();
                let right = direct[run_end].as_ref();
                match (left, right) {
                    (Some(left), Some(right))
                        if left.rule_id == right.rule_id && left.class == right.class =>
                    {
                        let left_interval = &intervals[ordered[run_start - 1]];
                        let right_interval = &intervals[ordered[run_end]];
                        let gap = right_interval.start_ms.saturating_sub(left_interval.end_ms);
                        gap <= left.absorb_max_gap
                    }
                    _ => false,
                }
            } else {
                false
            };

            if fill {
                let boundary = direct[run_start - 1]
                    .as_ref()
                    .expect("left boundary exists");
                for position in run_start..run_end {
                    let interval = &intervals[ordered[position]];
                    resolved[position] = Some(IntervalClassification {
                        interval_id: interval.id,
                        subject: interval.subject.clone(),
                        class: boundary.class.clone(),
                        rule_id: boundary.rule_id.clone(),
                        source: ClassificationSource::Absorbed,
                    });
                }
            }
            run_start = run_end;
        }

        Ok(resolved.into_iter().flatten().collect())
    }
}

/// A repeated explicit dismissal is a suggestion, not an automatic rule.
/// Keeping it as a proposal preserves user control while making repeated
/// corrections cheaper to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DismissalRuleProposal {
    pub subject: String,
    pub occurrences: usize,
    pub rule: ClassificationRule,
}

/// Turns repeated user dismissals into an auditable exact-match rule proposal.
/// The structural field is selected only from the subject's witnessed shape;
/// no application, domain, or file-type vocabulary is consulted.
pub fn propose_repeated_dismissal_rule(
    dismissed_subjects: impl IntoIterator<Item = impl AsRef<str>>,
    min_occurrences: usize,
) -> Option<DismissalRuleProposal> {
    if min_occurrences == 0 {
        return None;
    }
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for subject in dismissed_subjects {
        let subject = subject.as_ref().trim();
        if !subject.is_empty() {
            *counts.entry(subject.to_string()).or_default() += 1;
        }
    }
    let (subject, occurrences) = counts
        .into_iter()
        .filter(|(_, occurrences)| *occurrences >= min_occurrences)
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))?;
    let field = if subject.starts_with("http://") || subject.starts_with("https://") {
        StructuralField::Url
    } else if subject.starts_with("file://") || subject.starts_with('/') {
        StructuralField::DocumentPath
    } else if subject.starts_with("window:") {
        StructuralField::WindowTitle
    } else if subject.starts_with("app:") {
        StructuralField::App
    } else {
        return None;
    };
    let rule = ClassificationRule::new(
        format!("dismissal:{occurrences}:{subject}"),
        100,
        ClassificationMatcher::new(field, MatchOperator::Equals, subject.clone()),
        "NotWork",
    );
    Some(DismissalRuleProposal {
        subject,
        occurrences,
        rule,
    })
}

#[derive(Debug, Clone)]
struct DirectClassification {
    rule_id: String,
    class: String,
    absorb_max_gap: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(
        id: u64,
        start_ms: u64,
        end_ms: u64,
        subject: &str,
        fields: WitnessedFields,
    ) -> ClassificationInterval {
        ClassificationInterval::new(id, start_ms, end_ms, subject, fields)
    }

    #[test]
    fn repeated_dismissals_propose_but_do_not_apply_an_exact_structural_rule() {
        let proposal = propose_repeated_dismissal_rule(
            [
                "https://example.test/watch/1",
                "https://example.test/watch/1",
                "https://example.test/watch/2",
            ],
            2,
        )
        .expect("repeated dismissal should produce a proposal");
        assert_eq!(proposal.subject, "https://example.test/watch/1");
        assert_eq!(proposal.occurrences, 2);
        assert_eq!(proposal.rule.matcher.field, StructuralField::Url);
        assert_eq!(proposal.rule.assign.class, "NotWork");
        assert_eq!(proposal.rule.matcher.op, MatchOperator::Equals);
    }

    fn rule(
        id: &str,
        priority: u32,
        field: StructuralField,
        op: MatchOperator,
        value: &str,
        class: &str,
    ) -> ClassificationRule {
        ClassificationRule::new(
            id,
            priority,
            ClassificationMatcher::new(field, op, value),
            class,
        )
    }

    #[test]
    fn matching_is_literal_and_uses_only_the_selected_witnessed_field() {
        let rules = ClassificationRules::new(vec![rule(
            "user-app-rule",
            10,
            StructuralField::App,
            MatchOperator::Equals,
            "Editor A",
            "user-label",
        )])
        .unwrap();

        let matching = interval(
            1,
            100,
            200,
            "/witnessed/surface/one",
            WitnessedFields::default()
                .with_app("Editor A")
                .with_url("https://arbitrary.example/not-special"),
        );
        let non_matching = interval(
            2,
            300,
            400,
            "/witnessed/surface/two",
            WitnessedFields::default()
                .with_app("Editor B")
                .with_url("https://arbitrary.example/not-special"),
        );

        let results = rules.evaluate(&[non_matching, matching]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].interval_id, 1);
        assert_eq!(results[0].source, ClassificationSource::Direct);
        assert_eq!(
            results[0].observation_concept(),
            ObservationConcept::SurfaceClassified {
                subject: "/witnessed/surface/one".into(),
                class: "user-label".into(),
            }
        );
    }

    #[test]
    fn priority_then_id_makes_overlapping_rules_deterministic() {
        let rules = ClassificationRules::new(vec![
            rule(
                "z-later",
                20,
                StructuralField::WindowTitle,
                MatchOperator::Contains,
                "draft",
                "later",
            ),
            rule(
                "a-first",
                10,
                StructuralField::WindowTitle,
                MatchOperator::Contains,
                "draft",
                "first",
            ),
        ])
        .unwrap();
        let input = interval(
            1,
            0,
            10,
            "surface",
            WitnessedFields::default().with_window_title("draft — arbitrary"),
        );
        assert_eq!(rules.evaluate(&[input]).unwrap()[0].class, "first");

        let equal_priority = ClassificationRules::new(vec![
            rule(
                "z-rule",
                10,
                StructuralField::App,
                MatchOperator::Equals,
                "same",
                "z",
            ),
            rule(
                "a-rule",
                10,
                StructuralField::App,
                MatchOperator::Equals,
                "same",
                "a",
            ),
        ])
        .unwrap();
        let input = interval(
            2,
            20,
            30,
            "surface",
            WitnessedFields::default().with_app("same"),
        );
        assert_eq!(equal_priority.evaluate(&[input]).unwrap()[0].class, "a");
    }

    #[test]
    fn a_rule_can_absorb_an_unmatched_interval_between_two_agreements() {
        let rules = ClassificationRules::new(vec![
            rule(
                "coding-rule",
                1,
                StructuralField::App,
                MatchOperator::Equals,
                "Editor",
                "focused",
            )
            .with_absorb_max_gap(90_000),
        ])
        .unwrap();
        let intervals = vec![
            interval(
                1,
                0,
                100_000,
                "editor-one",
                WitnessedFields::default().with_app("Editor"),
            ),
            interval(
                2,
                100_000,
                190_000,
                "unmatched-detour",
                WitnessedFields::default().with_app("Other"),
            ),
            interval(
                3,
                190_000,
                300_000,
                "editor-two",
                WitnessedFields::default().with_app("Editor"),
            ),
        ];

        let results = rules.evaluate(&intervals).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[1].source, ClassificationSource::Absorbed);
        assert_eq!(results[1].class, "focused");
        assert_eq!(results[1].rule_id, "coding-rule");

        let bounded = ClassificationRules::new(vec![
            rule(
                "coding-rule",
                1,
                StructuralField::App,
                MatchOperator::Equals,
                "Editor",
                "focused",
            )
            .with_absorb_max_gap(89_999),
        ])
        .unwrap();
        assert_eq!(bounded.evaluate(&intervals).unwrap().len(), 2);
    }

    #[test]
    fn disagreement_does_not_fill_the_gap() {
        let rules = ClassificationRules::new(vec![
            rule(
                "left",
                1,
                StructuralField::App,
                MatchOperator::Equals,
                "Left",
                "left-class",
            )
            .with_absorb_max_gap(100_000),
            rule(
                "right",
                2,
                StructuralField::App,
                MatchOperator::Equals,
                "Right",
                "right-class",
            )
            .with_absorb_max_gap(100_000),
        ])
        .unwrap();
        let intervals = vec![
            interval(
                1,
                0,
                100,
                "left",
                WitnessedFields::default().with_app("Left"),
            ),
            interval(
                2,
                100,
                200,
                "gap",
                WitnessedFields::default().with_app("Other"),
            ),
            interval(
                3,
                200,
                300,
                "right",
                WitnessedFields::default().with_app("Right"),
            ),
        ];

        let results = rules.evaluate(&intervals).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(
            results
                .iter()
                .map(|result| result.interval_id)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
    }

    #[test]
    fn repo_is_never_inferred_from_a_document_path() {
        let rules = ClassificationRules::new(vec![rule(
            "repo-rule",
            1,
            StructuralField::Repo,
            MatchOperator::Equals,
            "/repo",
            "repo-class",
        )])
        .unwrap();
        let without_witness = interval(
            1,
            0,
            1,
            "/repo/src/main.rs",
            WitnessedFields::default().with_document_path("/repo/src/main.rs"),
        );
        assert!(rules.evaluate(&[without_witness]).unwrap().is_empty());

        let with_witness = interval(
            2,
            2,
            3,
            "/repo/src/main.rs",
            WitnessedFields::default()
                .with_document_path("/repo/src/main.rs")
                .with_repo("/repo"),
        );
        assert_eq!(
            rules.evaluate(&[with_witness]).unwrap()[0].class,
            "repo-class"
        );
    }

    #[test]
    fn invalid_rules_and_intervals_are_rejected_before_evaluation() {
        let empty_value = rule(
            "bad",
            1,
            StructuralField::Url,
            MatchOperator::Equals,
            " ",
            "class",
        );
        assert_eq!(
            ClassificationRules::new(vec![empty_value]),
            Err(ClassificationError::EmptyMatcherValue {
                rule_id: "bad".into()
            })
        );

        let rules = ClassificationRules::new(vec![rule(
            "ok",
            1,
            StructuralField::Url,
            MatchOperator::Equals,
            "x",
            "class",
        )])
        .unwrap();
        let invalid = interval(4, 10, 9, "surface", WitnessedFields::default());
        assert_eq!(
            rules.evaluate(&[invalid]),
            Err(ClassificationError::InvalidIntervalBounds {
                interval_id: 4,
                start_ms: 10,
                end_ms: 9,
            })
        );
    }

    #[test]
    fn duplicate_rule_ids_are_not_ambiguous() {
        let first = rule(
            "same",
            1,
            StructuralField::App,
            MatchOperator::Equals,
            "A",
            "one",
        );
        let second = rule(
            "same",
            2,
            StructuralField::App,
            MatchOperator::Equals,
            "B",
            "two",
        );
        assert_eq!(
            ClassificationRules::new(vec![first, second]),
            Err(ClassificationError::DuplicateRuleId {
                rule_id: "same".into()
            })
        );
    }

    #[test]
    fn disabled_rules_are_silence() {
        let rules = ClassificationRules::new(vec![
            rule(
                "disabled",
                1,
                StructuralField::App,
                MatchOperator::Equals,
                "Editor",
                "ignored",
            )
            .disabled(),
        ])
        .unwrap();
        let input = interval(
            1,
            0,
            1,
            "surface",
            WitnessedFields::default().with_app("Editor"),
        );
        assert!(rules.evaluate(&[input]).unwrap().is_empty());
    }

    #[test]
    fn empty_rule_set_is_silence() {
        let rules = ClassificationRules::empty();
        let input = interval(
            1,
            0,
            1,
            "surface",
            WitnessedFields::default().with_url("https://any.example"),
        );
        assert!(rules.evaluate(&[input]).unwrap().is_empty());
    }

    #[test]
    fn all_interval_ordering_is_replayable() {
        let rules = ClassificationRules::new(vec![rule(
            "url",
            1,
            StructuralField::Url,
            MatchOperator::Suffix,
            "/task",
            "task",
        )])
        .unwrap();
        let later = interval(
            2,
            200,
            300,
            "later",
            WitnessedFields::default().with_url("opaque/task"),
        );
        let earlier = interval(
            1,
            0,
            100,
            "earlier",
            WitnessedFields::default().with_url("opaque/task"),
        );
        let forward = rules.evaluate(&[earlier.clone(), later.clone()]).unwrap();
        let reverse = rules.evaluate(&[later, earlier]).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(
            forward
                .iter()
                .map(|result| result.interval_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }
}
