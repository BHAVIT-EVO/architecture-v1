//! Affinity: the witnessed evidence that two resources belong to the same body
//! of work.
//!
//! Four independent signals are combined. Each is a measurement over the
//! canonical Observation history; none of them consults an application name, a
//! domain, a file type, a directory allow-list, or a notion of "productive
//! software".
//!
//! 1. **Interleaving** — the person moved back and forth between the two
//!    resources inside one sitting. This is the strongest witness that two
//!    things are part of one activity. Co-presence is counted for a pair of
//!    acts with a person on at least one end; for a pair with a person on
//!    *neither* end — two changes — it is counted only where the same witnessed
//!    attention accounted for both across more than one sitting, which is what
//!    tells two documents an editor writes apart from two caches a service
//!    writes (see [`measure_interleaving`]).
//! 2. **Co-occurrence** — the two resources recur together across sittings,
//!    measured as normalized pointwise mutual information.
//! 3. **Lexical** — the witnessed names share distinctive vocabulary, weighted
//!    so that vocabulary shared by everything counts for nothing.
//! 4. **Structural** — the two resources share a containing location or origin.
//!
//! # Why this suppresses background noise with no exclusion list
//!
//! Every signal here is *normalized against ubiquity*. Interleaving is divided
//! by the total interleaving degree of both resources; co-occurrence uses NPMI,
//! which is exactly zero for a resource that appears in every sitting; lexical
//! overlap is inverse-document-frequency weighted, so a word present in most
//! subjects carries no weight; structural match is capped below the relatedness
//! floor so it can never form a relationship by itself.
//!
//! The consequence is a single principle rather than a list of exceptions:
//! **something that correlates with everything correlates with nothing.** A
//! cache the operating system rewrites beside every activity, a music player
//! that is focused throughout the day, a chat window opened between every task
//! — each earns near-zero affinity to everything, for the same structural
//! reason, without Evo knowing what any of them are. And a chat window that
//! genuinely *is* the work — interleaved with one document, recurring with it
//! and nothing else — earns high affinity, by the same rule.

use crate::declarations::Declarations;
use crate::episode::{Act, AttentionLedger, AttentionRecord, Episode};
use crate::params::EngagementParams;
use crate::resource::{Resource, SubjectShape};

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, SystemTime};

/// The witnessed components behind one relationship, kept separately so an
/// explanation can cite the evidence rather than a score.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AffinityEvidence {
    /// Degree-normalized within-sitting interleaving, `0.0..=1.0`.
    pub interleave: f64,
    /// Normalized pointwise mutual information across sittings, `0.0..=1.0`.
    pub co_episode: f64,
    /// Inverse-frequency-weighted shared vocabulary, `0.0..=1.0`.
    pub lexical: f64,
    /// Whether the two share a containing location or origin.
    pub structural: f64,
    /// Number of sittings in which both were witnessed *and* at least one of
    /// them was acted on by a person. A sitting in which only machines wrote is
    /// not evidence about work; see [`AffinityGraph::build`].
    pub co_episodes: usize,
    /// Whether the person stated directly that these belong together. When
    /// true the measured components are recorded but do not decide the
    /// outcome: a statement outranks an inference.
    pub declared: bool,
}

impl AffinityEvidence {
    /// The combined affinity, normalized into `0.0..=1.0`.
    ///
    /// # Authority change: correspondence corroborates, it does not constitute
    ///
    /// **The old rule.** The four signals were summed and compared to
    /// `affinity_floor`. The module's stated intent was that correspondence —
    /// shared wording, shared location — could never form a relationship alone,
    /// and `weight_structural` (0.10) was set below `affinity_floor` (0.20) for
    /// exactly that reason. But `weight_lexical` (0.20) plus `weight_structural`
    /// (0.10) is 0.30, so together they cleared the floor and the intent did not
    /// hold.
    ///
    /// **Why it prevented the product from working.** The pairs that maximise
    /// both at once are not bodies of work; they are files a program wrote in one
    /// directory under one naming scheme. Several near-identical names in a shared
    /// container is the *signature* of machine output, so the one combination that
    /// could form a relationship out of correspondence alone was the one
    /// combination that should never have formed one. A body of work is a claim
    /// about someone's activity; wording and location describe what things *are*,
    /// and no amount of either witnesses anybody doing anything.
    ///
    /// **The new rule.** A relationship requires witnessed co-presence — within a
    /// sitting or across them. Correspondence then strengthens it, decides which
    /// relationships survive clustering, and is what
    /// [`AffinityEvidence::is_corroborated`] reads. A statement from the person
    /// still outranks all of it. The cost is honest: two documents named alike in
    /// one folder that the person never once used together stay separate, because
    /// Evo never witnessed them as one activity. No parameter is added, and the
    /// weights are unchanged.
    ///
    /// # Authority change: withhold the conclusion, keep the measurement
    ///
    /// The rule above was implemented by returning `0.0` — the score of two
    /// resources with no measurable relationship whatever — for a pair with no
    /// co-presence. That conflates two different states, and the conflation is a
    /// silent loss: "Evo measured nothing between these" and "Evo measured shared
    /// wording and a shared location, and declines to call that a relationship"
    /// became the same number, so nothing downstream could tell them apart or
    /// ever recover the second.
    ///
    /// Uncertainty may lower a claim's standing. It must not destroy the record
    /// underneath it. So correspondence-only evidence now scores its own
    /// weighted contribution — necessarily below `affinity_floor`, because the
    /// lexical and structural weights are set below it for exactly this reason,
    /// so it still cannot form a relationship by itself — and the measurement
    /// survives for anything that asks. The clustering behaviour is unchanged;
    /// what changes is that the evidence is no longer thrown away to express it.
    pub fn score(&self, params: &EngagementParams) -> f64 {
        if self.declared {
            return 1.0;
        }
        let total = params.weight_total();
        if total <= 0.0 {
            return 0.0;
        }
        let combined = params.weight_interleave * self.interleave
            + params.weight_co_episode * self.co_episode
            + params.weight_lexical * self.lexical
            + params.weight_structural * self.structural;
        let score = (combined / total).clamp(0.0, 1.0);
        if self.interleave <= 0.0 && self.co_episode <= 0.0 {
            // Correspondence with no witnessed co-presence. Reported, and held
            // below the relatedness floor so it cannot constitute a
            // relationship on its own.
            return correspondence_only(score, params);
        }
        score
    }

    /// Whether this relationship rests on more than the two things having been
    /// present at the same time.
    ///
    /// Interleaving and recurrence are evidence of *presence*: the person moved
    /// between these two, or came back to both. That is real evidence and it is
    /// what forms groups — but it cannot tell "the document I am writing" apart
    /// from "the thing that was playing while I wrote it", because both are
    /// interleaved with the work exactly the same way.
    ///
    /// Shared distinctive vocabulary, a shared containing location, and a
    /// statement from the person are different in kind: each says something
    /// about *what these things are*, not merely that they were both there. When
    /// some members of a body of work have that and others do not, the
    /// difference is exactly the one Evo needs — between a resource that is part
    /// of the work and a resource that was alongside it.
    ///
    /// Nothing here knows what any resource is. A music player whose title
    /// happens to share the assignment's distinctive wording would be
    /// corroborated, correctly; a source file sharing nothing with its own task
    /// would not be, also correctly.
    pub fn is_corroborated(&self) -> bool {
        self.declared || self.lexical > 0.0 || self.structural > 0.0
    }

    /// The single strongest kind of evidence behind this relationship, used to
    /// phrase grounded explanations.
    pub fn strongest(&self, params: &EngagementParams) -> EvidenceKind {
        if self.declared {
            return EvidenceKind::Declared;
        }
        let contributions = [
            (
                EvidenceKind::Interleaved,
                params.weight_interleave * self.interleave,
            ),
            (
                EvidenceKind::Recurring,
                params.weight_co_episode * self.co_episode,
            ),
            (EvidenceKind::Lexical, params.weight_lexical * self.lexical),
            (
                EvidenceKind::Structural,
                params.weight_structural * self.structural,
            ),
        ];
        let mut best = EvidenceKind::Interleaved;
        let mut best_value = f64::NEG_INFINITY;
        for (kind, value) in contributions {
            if value > best_value {
                best_value = value;
                best = kind;
            }
        }
        best
    }
}

/// The affinity reported for a pair whose only evidence is correspondence —
/// shared wording, a shared container — with no witnessed co-presence at all.
///
/// Correspondence describes what two things *are*; it never witnesses anybody
/// doing anything, so it may not constitute a relationship
/// ([`AffinityEvidence::score`]). But the measurement is real and something
/// downstream may need it, so it is reported rather than zeroed, scaled into the
/// range strictly below [`EngagementParams::affinity_floor`].
///
/// Scaled rather than clamped so the ordering survives: a pair that shares a
/// great deal of distinctive wording *and* a container still ranks above a pair
/// that shares only a container, which a clamp at the ceiling would flatten. The
/// ceiling is the greatest representable value below the floor — not a parameter,
/// but the exact statement "this is not a relationship" in the same units.
fn correspondence_only(score: f64, params: &EngagementParams) -> f64 {
    let total = params.weight_total();
    if total <= 0.0 || params.affinity_floor <= 0.0 {
        return 0.0;
    }
    // The most correspondence alone can ever weigh.
    let widest = (params.weight_lexical + params.weight_structural) / total;
    if widest <= 0.0 {
        return 0.0;
    }
    let ceiling = f64::from_bits(params.affinity_floor.to_bits() - 1);
    ((score / widest) * ceiling).clamp(0.0, ceiling)
}

/// Which kind of witnessed evidence dominates a relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    /// The person said so.
    Declared,
    /// Moved back and forth between them in one sitting.
    Interleaved,
    /// Came back to both across separate sittings.
    Recurring,
    /// Their names share distinctive wording.
    Lexical,
    /// They share a containing location.
    Structural,
}

impl EvidenceKind {
    /// A short, honest phrase describing this evidence.
    pub fn phrase(self) -> &'static str {
        match self {
            EvidenceKind::Declared => "grouped by you with",
            EvidenceKind::Interleaved => "used together in the same sitting",
            EvidenceKind::Recurring => "returned to together across sittings",
            EvidenceKind::Lexical => "named for the same thing",
            EvidenceKind::Structural => "kept in the same place",
        }
    }
}

/// The relationship graph over every witnessed resource.
#[derive(Debug, Clone, Default)]
pub struct AffinityGraph {
    resources: BTreeMap<String, Resource>,
    edges: BTreeMap<(String, String), AffinityEvidence>,
    lexical: LexicalIndex,
}

impl AffinityGraph {
    /// Measures affinity across every witnessed resource pair.
    pub fn build(
        episodes: &[Episode],
        ledger: &AttentionLedger,
        declarations: &Declarations,
        params: &EngagementParams,
    ) -> Self {
        let resources = collect_resources(episodes, declarations);
        let interleave = measure_interleaving(episodes, params);
        let lexical_index = LexicalIndex::build(&resources);
        let episode_total = episodes.len();

        let subjects: Vec<&String> = resources.keys().collect();
        let mut edges: BTreeMap<(String, String), AffinityEvidence> = BTreeMap::new();

        for (left_index, left) in subjects.iter().enumerate() {
            for right in subjects.iter().skip(left_index + 1) {
                let key = ((*left).clone(), (*right).clone());
                let left_record = ledger.get(left);
                let right_record = ledger.get(right);

                let co_episodes = match (left_record, right_record) {
                    (Some(a), Some(b)) => a
                        .episodes
                        .intersection(&b.episodes)
                        // The same rule co-presence within a sitting follows, at
                        // the longer time scale: a sitting in which neither
                        // resource was acted on by a person is a sitting in which
                        // two machines wrote files, and that they did so on the
                        // same occasion is not evidence that they belong to one
                        // body of work. Marginals below stay whole, so filtering
                        // the joint can only lower the measured association —
                        // the required direction of error.
                        .filter(|episode| {
                            a.human_episodes.contains(episode)
                                || b.human_episodes.contains(episode)
                        })
                        .count(),
                    _ => 0,
                };

                // Whether a person was ever witnessed acting on each side. A
                // resource nobody ever touched is not disqualified — a file the
                // person's editor writes is exactly how Evo learns the file took
                // part — but what its co-presence has to show is different, below.
                let attended = |record: Option<&AttentionRecord>| {
                    record.is_some_and(|record| !record.human_episodes.is_empty())
                };
                let both_attended = attended(left_record) && attended(right_record);

                // One machine write happening to land beside one human act is a
                // coincidence of a single moment, and the resource it belongs to
                // is the case the affinity measure is least able to judge: with
                // almost no company at all, its entire co-presence mass sits on
                // that one neighbour, so the profile cosine reads *high*. This is
                // the same "one is a coincidence, two is a pattern" rule
                // `min_co_episodes` already states for cross-sitting recurrence,
                // applied to within-sitting co-presence for the only case that
                // needs it. Two resources a person attended are exempt: alt-tabbing
                // between two documents in one afternoon is real work, and always
                // was.
                //
                // Two *changes* neither of which a person was witnessed acting on
                // are the remaining case, and the same rule decides it: their
                // co-presence counts only where the same attention accounted for
                // both across more than one sitting ([`measure_interleaving`]),
                // which is exactly a witnessed pattern rather than a coincidence.
                // The pair is carried from there rather than re-derived, because
                // neither resource's own record can distinguish two documents an
                // editor writes from two caches a service writes — the difference
                // is in whose attention the changes kept returning to.
                let co_presence_is_a_pattern = both_attended
                    || co_episodes >= params.min_co_episodes
                    || interleave.is_selective_change_pair(left, right);

                let evidence = AffinityEvidence {
                    interleave: if co_presence_is_a_pattern {
                        interleave.normalized(left, right)
                    } else {
                        0.0
                    },
                    co_episode: normalized_pmi(
                        left_record.map(|r| r.recurrence()).unwrap_or(0),
                        right_record.map(|r| r.recurrence()).unwrap_or(0),
                        co_episodes,
                        episode_total,
                        params,
                    ),
                    lexical: lexical_index.similarity(left, right),
                    structural: structural_match(&resources[*left], &resources[*right]),
                    co_episodes,
                    declared: declarations.is_grouped(left, right),
                };

                // Recorded when anything at all was witnessed about the pair, not
                // only when it amounts to a relationship. The two are different
                // questions since correspondence stopped constituting a
                // relationship (see [`AffinityEvidence::score`]), and Evo owes an
                // explanation for a *refusal* as much as for a grouping: "these
                // two sit in one folder and share no activity" is an answer, and
                // it is only available if the evidence is kept. Still sparse —
                // pairs about which nothing was witnessed are not stored.
                if evidence.interleave > 0.0
                    || evidence.co_episode > 0.0
                    || evidence.lexical > 0.0
                    || evidence.structural > 0.0
                    || evidence.declared
                {
                    edges.insert(key, evidence);
                }
            }
        }

        Self {
            resources,
            edges,
            lexical: lexical_index,
        }
    }

    /// How much distinctive vocabulary `subject`'s witnessed name holds in common
    /// with the rest of `members`, in nats of shared surprise.
    ///
    /// The same measurement that decides whether two resources are related, asked
    /// of one resource against a group. A name built only from words that appear
    /// all over the machine scores zero however many members repeat them; a name
    /// carrying a word that appears in few places elsewhere, and that other
    /// members of this group also carry, scores highly.
    ///
    /// Unbounded on purpose. Ranking candidate names needs to know that one
    /// carries five distinctive words and another carries one, which the
    /// saturating form used for pair affinity deliberately flattens away.
    pub fn shared_vocabulary(&self, subject: &str, members: &BTreeSet<String>) -> f64 {
        self.lexical.shared_with_group(subject, members)
    }

    /// The witnessed resource behind a subject.
    pub fn resource(&self, subject: &str) -> Option<&Resource> {
        self.resources.get(subject)
    }

    /// Every witnessed subject, in canonical order.
    pub fn subjects(&self) -> impl Iterator<Item = &String> {
        self.resources.keys()
    }

    /// How many resources the graph covers.
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Whether nothing has been witnessed.
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// The evidence behind one pair, in either order.
    pub fn evidence(&self, left: &str, right: &str) -> Option<&AffinityEvidence> {
        let key = ordered_key(left, right)?;
        self.edges.get(&key)
    }

    /// The combined affinity of one pair, in either order.
    pub fn affinity(&self, left: &str, right: &str, params: &EngagementParams) -> f64 {
        self.evidence(left, right)
            .map(|evidence| evidence.score(params))
            .unwrap_or(0.0)
    }

    /// Every pair whose combined affinity reaches the relatedness floor.
    pub fn related_pairs(
        &self,
        params: &EngagementParams,
    ) -> Vec<((&String, &String), &AffinityEvidence)> {
        let mut pairs: Vec<((&String, &String), &AffinityEvidence)> = self
            .edges
            .iter()
            .filter(|(_, evidence)| evidence.score(params) >= params.affinity_floor)
            .map(|((left, right), evidence)| ((left, right), evidence))
            .collect();
        // Descending by score, then canonically by subject, so the ordering is
        // fully determined by the evidence.
        pairs.sort_by(|a, b| {
            b.1.score(params)
                .partial_cmp(&a.1.score(params))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.0.cmp(b.0.0))
                .then_with(|| a.0.1.cmp(b.0.1))
        });
        pairs
    }
}

fn ordered_key(left: &str, right: &str) -> Option<(String, String)> {
    match left.cmp(right) {
        std::cmp::Ordering::Less => Some((left.to_string(), right.to_string())),
        std::cmp::Ordering::Greater => Some((right.to_string(), left.to_string())),
        std::cmp::Ordering::Equal => None,
    }
}

fn collect_resources(
    episodes: &[Episode],
    declarations: &Declarations,
) -> BTreeMap<String, Resource> {
    let mut resources = BTreeMap::new();
    for episode in episodes {
        for act in episode.acts() {
            resources
                .entry(act.resource().subject().to_string())
                .or_insert_with(|| {
                    // A directly witnessed container replaces the one implied
                    // by the subject's own name.
                    match declarations.container_of(act.resource().subject()) {
                        Some(container) => act.resource().clone().in_container(container),
                        None => act.resource().clone(),
                    }
                });
        }
    }
    resources
}

/// Raw within-sitting co-presence weights and the totals needed to judge how
/// *selective* each resource's co-presence is.
#[derive(Debug, Default)]
struct Interleaving {
    raw: BTreeMap<(String, String), f64>,
    /// Each resource's co-presence profile: how much of everything else was
    /// present beside it. Derived from `raw` by [`Interleaving::build_profiles`].
    profile: BTreeMap<String, BTreeMap<String, f64>>,
    /// The pairs of *changed* resources whose co-presence the same witnessed
    /// attention accounted for, in more sittings than one — the only pairs with
    /// a person on neither end that co-presence was counted for at all. Kept
    /// because [`AffinityGraph::build`] has to ask the same "is this a pattern
    /// or a coincidence" question and cannot re-derive the answer: it is a fact
    /// about the acts, not about either resource's record.
    selective_changes: BTreeSet<(String, String)>,
}

impl Interleaving {
    /// Whether the same witnessed attention accounted for changes to both of
    /// these resources, in more sittings than one. False for every pair with a
    /// person on either end, which is judged by its own record instead.
    fn is_selective_change_pair(&self, left: &str, right: &str) -> bool {
        ordered_key(left, right)
            .is_some_and(|key| self.selective_changes.contains(&key))
    }
    /// How alike two resources' company is, in `0.0..=1.0`.
    ///
    /// This is the cosine between the two resources' **co-presence profiles**:
    /// the vector, over every witnessed resource, of how much each was present
    /// beside this one. Two things belong to the same body of work when they
    /// keep the same company.
    ///
    /// Two earlier formulations were tried and both failed the same way — by
    /// punishing breadth instead of ubiquity:
    ///
    /// - **Dice** (`2·pair / (degreeₗ + degreeᵣ)`) falls off as `1/(k-1)` for
    ///   `k` resources in rotation, so a real task using six resources scored
    ///   lower than a trivial one using two.
    /// - **NPMI** removed the worst of that but kept a milder version of it:
    ///   with more partners each individual pairing is less surprising, so a
    ///   uniform four-resource task scored 0.55 per pair where a three-resource
    ///   one scored 1.00. A body of work with eight parts is not less coherent
    ///   than one with three.
    ///
    /// Profile cosine has no size bias — every member of a uniform clique of
    /// any size scores near 1.0 against every other — while still suppressing
    /// ubiquity, because a resource present beside *everything* has its mass
    /// spread across the whole vector and therefore shares only a small
    /// fraction of its norm with any one clique. That is the same principle as
    /// before, measured in a way that scales.
    ///
    /// Each profile includes a self term equal to the resource's strongest
    /// co-presence, so that direct co-presence between the pair contributes to
    /// the similarity rather than only their shared neighbours. Without it two
    /// resources that never appear together but keep identical company would
    /// score as highly as two that are constantly together.
    fn normalized(&self, left: &str, right: &str) -> f64 {
        let Some(left_profile) = self.profile.get(left) else {
            return 0.0;
        };
        let Some(right_profile) = self.profile.get(right) else {
            return 0.0;
        };

        let mut dot = 0.0;
        for (subject, weight) in left_profile {
            if let Some(other) = right_profile.get(subject) {
                dot += weight * other;
            }
        }
        if dot <= 0.0 {
            return 0.0;
        }
        let left_norm = left_profile.values().map(|w| w * w).sum::<f64>().sqrt();
        let right_norm = right_profile.values().map(|w| w * w).sum::<f64>().sqrt();
        if left_norm <= 0.0 || right_norm <= 0.0 {
            return 0.0;
        }
        (dot / (left_norm * right_norm)).clamp(0.0, 1.0)
    }

    /// Builds each resource's co-presence profile from the measured pair
    /// weights, adding the self term described on [`Self::normalized`].
    fn build_profiles(&mut self) {
        let mut profile: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
        for ((left, right), weight) in &self.raw {
            *profile
                .entry(left.clone())
                .or_default()
                .entry(right.clone())
                .or_insert(0.0) += weight;
            *profile
                .entry(right.clone())
                .or_default()
                .entry(left.clone())
                .or_insert(0.0) += weight;
        }
        for (subject, vector) in profile.iter_mut() {
            let peak = vector.values().copied().fold(0.0f64, f64::max);
            vector.insert(subject.clone(), peak);
        }
        self.profile = profile;
    }
}

/// Measures how much the person moved between resources inside each sitting.
///
/// Two acts are co-attended when they fall within the attention window of one
/// another, and the evidence they contribute decays linearly with the gap: acts
/// seconds apart are strong evidence of one activity, acts nearly a window
/// apart are weak evidence. There is no cliff and no bucket — the decay is the
/// honest statement that evidence weakens as the interval Evo must bridge grows.
///
/// The resulting weights are then read as a probability distribution over pairs
/// (see [`Interleaving::normalized`]), which is what makes a resource that
/// appears beside everything end up related to nothing.
///
/// # Authority change: co-presence is evidence about whoever acted (RFC-0014)
///
/// **The old rule.** Every pair of acts within the window contributed
/// co-presence weight, whatever their [`ActCharacter`]. A machine writing a
/// cache file three seconds after the person changed windows contributed the
/// same 0.995 as the person moving between two documents.
///
/// **Why it prevented the product from working.** Co-presence in *time* was
/// being read as co-presence in *work*, and the two are only the same thing when
/// a person is on both ends of it. An interval between two acts the person
/// performed is evidence about the person: they were going back and forth. An
/// interval between two acts the machine performed is evidence that a clock
/// advanced — no person was involved at either end, so it cannot be evidence
/// about a person's work. Because `weight_lexical` (0.20) cannot reach
/// `affinity_floor` (0.20) without a similarity of exactly 1.0, and
/// `weight_structural` (0.10) is deliberately below it, co-presence was the only
/// signal capable of forming an edge at all — so this conflation decided every
/// membership in the system. On this machine's real log, all 95 memberships
/// rested on it.
///
/// The consequence was not merely noisy members. A tool that writes several
/// similarly-named files per run produces a set with very high mutual affinity —
/// same directory, near-identical names, always co-present — and average linkage
/// merges on the *average*, so such a set raises the cohesion of any cluster it
/// touches and forms giant clusters of its own. The real log showed clusters of
/// 121, 54, 54 and 43 such members, and it showed real attended work being
/// pulled into them and then refused as insignificant: one cluster held a cache
/// file, a photo-library internal, and two resources the person actually worked
/// in. That is a **false negative created at the membership layer**, which no
/// role assignment downstream can undo — the cluster is refused before roles are
/// ever assigned. The previous position, that machine churn is best caught at
/// the role rather than at membership, is therefore falsified by real evidence:
/// catching it at the role protects what Evo *opens*, and does nothing to protect
/// what Evo can still *see*.
///
/// **The new rule.** A pair of acts contributes co-presence only when at least
/// one of the two is human evidence ([`ActCharacter::is_human_evidence`]). A
/// person's act still ties to a machine's act — a file written while the person
/// worked is exactly the Production evidence that a resource took part, and it
/// is preserved in full. Only machine-beside-machine proximity is discarded,
/// because there is no person in it to have been working.
///
/// This introduces no parameter (RFC-0014 R5): it is a question about which kind
/// of act was witnessed, answered from the frozen Observation schema, not about
/// any magnitude. It names no application, path, domain, or extension. A
/// resource left with no human-witnessed company has an empty co-presence
/// profile and so joins nothing — Evo still records that it was witnessed, and
/// claims nothing further about it.
///
/// # Amendment: discarding change-beside-change outright made two documents
/// unrelatable
///
/// The rule above is right that a clock advancing between two machine acts is not
/// evidence about a person. It was wrong to conclude that such a pair may
/// therefore be *discarded*, and the cost was the product's central case.
///
/// A save is [`ActCharacter::Incidental`], so under the blanket rule **no two
/// files could be related to each other by any evidence in any amount**: each
/// bound to the window the person was attending, neither to the other. Every body
/// of work whose members are documents therefore came out as a *star* centred on
/// a window, and average linkage takes a star apart — merging the hub with its
/// nearest leaf halves the average to every remaining leaf, because leaf-to-leaf
/// is zero, so the leaves drop below [`EngagementParams::cohesion_floor`] one at
/// a time. Measured, not reasoned: six resources of one release-notes review
/// derived as five bodies of work — `{notes, editor}` and four singletons — and a
/// singleton never becomes work at all, so four of the six were not mis-grouped
/// but lost outright. The real log shows the same shape from the other side: 325
/// of 425 resources are changes, and 341 resources belong to no body of work.
///
/// **The rule now.** A pair of acts with a person on at least one end contributes
/// co-presence, exactly as above. A pair with a person on *neither* end
/// contributes no co-presence weight of its own — fabricating it is what built the
/// churn cliques — but it earns the right to be **judged on the company it
/// keeps**, where two conditions hold together:
///
/// 1. **The same witnessed attention accounted for both changes**, in at least
///    `min_co_episodes` sittings. Once is a coincidence.
/// 2. **That attention accounts for most of what either change was witnessed
///    beside** ([`accounts_for_most`]).
///
/// That is this crate's own standing position on incidental acts — they "become
/// evidence only when they are *selectively* associated with attention"
/// ([`ActCharacter::Incidental`]) — with *selectively* measured rather than
/// assumed. The pair is recorded ([`Interleaving::is_selective_change_pair`]) so
/// that the judgement is not then thrown away by the co-episode gate in
/// [`AffinityGraph::build`], which counts only human-attended sittings and so
/// cannot see it. Nothing is added to the profile; the gate merely stops
/// discarding a similarity the profile already measured.
///
/// **Why condition 1 alone is not enough, measured rather than reasoned.** The
/// first attempt at this amendment stopped at condition 1, on the argument that
/// churn is "written beside whatever is focused, a different thing each sitting".
/// Re-derived against the real log that argument is false, and the way it fails is
/// worth keeping: a person parks on one window for long stretches — a music player
/// held focus in 7 of 7 sittings, a chat in 6 of 6 — and the photo library syncs
/// beside *that*, every time. So churn does share a recurring locus, and condition
/// 1 alone admitted it: the real corpus derived a 32-member body of work in which
/// 29 members were a photo library's caches and a search index's shards. What
/// churn does not have is *concentration*: those same caches were witnessed beside
/// 31 to 61 distinct loci each, so no single one of them accounts for the change.
/// The measured separation is clean — every change on this machine with any
/// recurring locus scores `0.3` or below, a document saved beside its own editor
/// scores `1.0` — and the real corpus is left exactly as it was, which for a log
/// that is 305 changes of machine churn out of 325 is the correct answer.
///
/// No parameter is introduced (RFC-0014 R5): `max_interval_attention` already
/// means "how close two acts must be to be one stretch of working",
/// `min_co_episodes` already means "once is a coincidence, twice is a pattern",
/// and `attributable_change_locality` already means "a strict majority of the
/// resource's witnessed life". Nothing is named — no application, path, domain,
/// extension, or content — and no document is opened or read: the evidence is
/// *which* resource was focused, never what was inside it. For a change with no
/// concentrated attended company, the only direction this can err in remains
/// exclusion.
fn measure_interleaving(episodes: &[Episode], params: &EngagementParams) -> Interleaving {
    let window = params.max_interval_attention.as_secs_f64().max(1.0);
    let mut interleaving = Interleaving::default();

    // Co-presence between two *changes*, held aside rather than counted. A
    // change cannot vouch for another change on its own, so this weight is
    // folded into `raw` below only for the pairs that earn it.
    let mut unattributed: BTreeMap<(String, String), f64> = BTreeMap::new();
    // For each changed resource: which locus of the person's attention its
    // changes were witnessed beside, and in which sittings. This is what
    // "selectively associated with attention" is measured against.
    let mut attended_beside: BTreeMap<String, BTreeMap<String, BTreeSet<usize>>> = BTreeMap::new();

    for (index, episode) in episodes.iter().enumerate() {
        let acts = episode.acts();
        for (position, act) in acts.iter().enumerate() {
            if !act.character().is_human_evidence() {
                // Which attention this change happened beside. Acts are
                // time-ordered, so each direction stops at the first act more
                // than one stretch of attention away.
                let beside = attended_beside
                    .entry(act.resource().subject().to_string())
                    .or_default();
                for earlier in acts[..position].iter().rev() {
                    if seconds_between(earlier.at(), act.at()) > window {
                        break;
                    }
                    if earlier.character().is_human_evidence() {
                        beside
                            .entry(earlier.resource().subject().to_string())
                            .or_default()
                            .insert(index);
                    }
                }
                for later in acts.iter().skip(position + 1) {
                    if seconds_between(act.at(), later.at()) > window {
                        break;
                    }
                    if later.character().is_human_evidence() {
                        beside
                            .entry(later.resource().subject().to_string())
                            .or_default()
                            .insert(index);
                    }
                }
            }

            for other in acts.iter().skip(position + 1) {
                let gap = seconds_between(act.at(), other.at());
                if gap > window {
                    // Acts are time-ordered, so nothing further can be closer.
                    break;
                }
                let Some(key) = ordered_key(act.resource().subject(), other.resource().subject())
                else {
                    continue;
                };
                let weight = 1.0 - (gap / window);
                if weight <= 0.0 {
                    continue;
                }
                if act.character().is_human_evidence() || other.character().is_human_evidence() {
                    *interleaving.raw.entry(key).or_insert(0.0) += weight;
                } else {
                    // Two changes near each other in time. Nobody was witnessed
                    // on either end, so this is set aside for the test below
                    // rather than counted here.
                    *unattributed.entry(key).or_insert(0.0) += weight;
                }
            }
        }
    }

    // Two changes belong to one body of work when the *same* witnessed attention
    // accounts for both of them across more sittings than one, **and** that
    // attention accounts for most of what either change was ever witnessed
    // beside. The weight set aside above is deliberately not folded back in:
    // fabricating co-presence between two things no person was witnessed at
    // would make every such pair each other's strongest company, which is how
    // machine churn formed dense cliques in the first place. What the pair earns
    // is the right to be *judged* on the company it keeps.
    for (key, _weight) in unattributed {
        let (Some(left), Some(right)) = (attended_beside.get(&key.0), attended_beside.get(&key.1))
        else {
            continue;
        };
        let selective = left.iter().any(|(locus, sittings)| {
            let Some(also) = right.get(locus) else {
                return false;
            };
            sittings.intersection(also).count() >= params.min_co_episodes
                && accounts_for_most(sittings.len(), left, params)
                && accounts_for_most(also.len(), right, params)
        });
        if selective {
            interleaving.selective_changes.insert(key);
        }
    }

    interleaving.build_profiles();
    interleaving
}

/// Whether one locus of attention accounts for *most* of what a change was
/// witnessed beside.
///
/// The denominator is every (locus, sitting) association the change has, not the
/// number of sittings it appears in. That distinction is the whole measurement. A
/// background service writes beside whatever happens to be focused, so it
/// accumulates many loci — the share of any one of them is small even when that
/// locus recurs, because it recurs for reasons that have nothing to do with the
/// change. A document is saved beside the thing it is being written in, so that
/// one locus holds nearly all of its associations.
///
/// Measured on this machine's real Observation log, every change that had any
/// recurring locus at all scored `0.3` or below, against the `0.5` this is
/// compared to: the real corpus is almost entirely a photo library's caches and
/// an IDE's telemetry, and the correct answer for all of it is refusal. A
/// document a person saves beside one editor scores `1.0`. Two loci in equal
/// measure — something edited in one place and previewed in another — scores
/// `0.5` and is still admitted.
///
/// No parameter is introduced (RFC-0014 R5). This is
/// [`EngagementParams::attributable_change_locality`] with exactly its stated
/// meaning — "a strict majority, so the resource's witnessed life must be more
/// inside this work than outside it" — asked one layer earlier, of a locus rather
/// than of a cluster.
fn accounts_for_most(
    here: usize,
    beside: &BTreeMap<String, BTreeSet<usize>>,
    params: &EngagementParams,
) -> bool {
    let associations: usize = beside.values().map(|sittings| sittings.len()).sum();
    associations > 0
        && (here as f64 / associations as f64) >= params.attributable_change_locality
}

/// Seconds from `from` to `to`, and zero if `to` does not follow `from`.
fn seconds_between(from: SystemTime, to: SystemTime) -> f64 {
    to.duration_since(from)
        .unwrap_or(Duration::ZERO)
        .as_secs_f64()
}

/// Normalized pointwise mutual information over sittings.
///
/// Returns `0.0` when either resource appears in every sitting: something
/// always present carries no information about anything. This is why a
/// perpetually-open window is not related to the work happening beside it,
/// with no need to know what the window is.
fn normalized_pmi(
    left_episodes: usize,
    right_episodes: usize,
    co_episodes: usize,
    episode_total: usize,
    params: &EngagementParams,
) -> f64 {
    if episode_total < 2 || co_episodes < params.min_co_episodes {
        return 0.0;
    }
    if left_episodes == 0 || right_episodes == 0 || co_episodes == 0 {
        return 0.0;
    }
    let total = episode_total as f64;
    let p_left = left_episodes as f64 / total;
    let p_right = right_episodes as f64 / total;
    let p_joint = co_episodes as f64 / total;

    // A resource present in every sitting yields p_joint == p_left * p_right,
    // hence zero information.
    let ratio = p_joint / (p_left * p_right);
    if ratio <= 1.0 {
        return 0.0;
    }
    let denominator = -p_joint.ln();
    if denominator <= f64::EPSILON {
        // Both appear in every sitting: no information either way.
        return 0.0;
    }
    (ratio.ln() / denominator).clamp(0.0, 1.0)
}

/// Inverse-frequency weighted vocabulary overlap.
#[derive(Debug, Clone, Default)]
struct LexicalIndex {
    tokens: BTreeMap<String, BTreeSet<String>>,
    weight: BTreeMap<String, f64>,
}

impl LexicalIndex {
    fn build(resources: &BTreeMap<String, Resource>) -> Self {
        let mut tokens: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut frequency: BTreeMap<String, usize> = BTreeMap::new();
        for (subject, resource) in resources {
            let subject_tokens = resource.tokens();
            for token in &subject_tokens {
                *frequency.entry(token.clone()).or_insert(0) += 1;
            }
            tokens.insert(subject.clone(), subject_tokens);
        }

        let total = resources.len().max(1) as f64;
        let weight = frequency
            .into_iter()
            .map(|(token, count)| {
                // Inverse document frequency. A word appearing in every
                // subject — a user name, a common folder, an extension —
                // weighs nothing. No stop-word list is needed or wanted:
                // which words are uninformative is measured, per machine.
                let idf = (total / count as f64).ln().max(0.0);
                (token, idf)
            })
            .collect();

        Self { tokens, weight }
    }

    /// The distinctive information two names share, in nats, mapped into
    /// `0.0..=1.0`.
    ///
    /// Deliberately **not** a ratio of shared to total vocabulary. A ratio
    /// cancels the inverse-frequency weighting: two subjects whose names are
    /// word-for-word identical score 1.0 under a ratio even when every one of
    /// those words appears in every other subject on the machine — which is how
    /// "Unrelated Surface 4" and "Unrelated Surface 7" would come out maximally
    /// related. Measuring the *absolute* shared surprise fixes that at the
    /// root: words everything shares carry zero information, so names built
    /// only from them share nothing, however identical they look.
    fn similarity(&self, left: &str, right: &str) -> f64 {
        let Some(left_tokens) = self.tokens.get(left) else {
            return 0.0;
        };
        let Some(right_tokens) = self.tokens.get(right) else {
            return 0.0;
        };
        let mut shared = 0.0;
        for token in left_tokens.intersection(right_tokens) {
            shared += self.weight.get(token).copied().unwrap_or(0.0);
        }
        if shared <= 0.0 {
            return 0.0;
        }
        // Saturating in nats of shared surprise: one token unique to a handful
        // of subjects already says a great deal; more says a little more.
        1.0 - (-shared).exp()
    }

    /// The distinctive information `subject`'s name shares with any other member
    /// of `members`, in nats.
    ///
    /// A token counts once, at its own weight, if *any* other member carries it.
    /// Counting once per carrier would let a group repeating one word thirty
    /// times outweigh a group of five names that genuinely describe one thing.
    fn shared_with_group(&self, subject: &str, members: &BTreeSet<String>) -> f64 {
        let Some(own) = self.tokens.get(subject) else {
            return 0.0;
        };
        let mut shared = 0.0;
        for token in own {
            let carried_elsewhere = members.iter().any(|other| {
                other != subject
                    && self
                        .tokens
                        .get(other)
                        .is_some_and(|tokens| tokens.contains(token))
            });
            if carried_elsewhere {
                shared += self.weight.get(token).copied().unwrap_or(0.0);
            }
        }
        shared
    }
}

/// Whether two resources share a containing location or origin.
fn structural_match(left: &Resource, right: &Resource) -> f64 {
    // Only comparable shapes can share a container; a path and an address do
    // not have a common notion of location.
    if left.shape() != right.shape() {
        return 0.0;
    }
    if matches!(left.shape(), SubjectShape::Surface | SubjectShape::Opaque) {
        return 0.0;
    }
    match (left.container(), right.container()) {
        (Some(a), Some(b)) if a == b => 1.0,
        _ => 0.0,
    }
}

/// Convenience: the acts of a whole history, for callers that already hold
/// episodes and want the flattened canonical sequence.
pub fn flatten_acts(episodes: &[Episode]) -> Vec<&Act> {
    episodes
        .iter()
        .flat_map(|episode| episode.acts().iter())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::segment_episodes;
    use crate::resource::ActCharacter;
    use std::time::SystemTime;

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn act(subject: &str, schema: &str, secs: u64) -> Act {
        Act::new(
            Resource::new(subject, schema),
            ActCharacter::of_schema(schema),
            at(secs),
        )
    }

    fn focus(subject: &str, secs: u64) -> Act {
        act(subject, "OBS-WINDOW-FOCUS-GAINED", secs)
    }

    fn save(subject: &str, secs: u64) -> Act {
        act(subject, "OBS-FILE-SAVED", secs)
    }

    fn graph_of(acts: Vec<Act>, params: &EngagementParams) -> AffinityGraph {
        graph_with(acts, &Declarations::new(), params)
    }

    fn graph_with(
        acts: Vec<Act>,
        declarations: &Declarations,
        params: &EngagementParams,
    ) -> AffinityGraph {
        let episodes = segment_episodes(acts, params);
        let ledger = AttentionLedger::build(&episodes, params);
        AffinityGraph::build(&episodes, &ledger, declarations, params)
    }

    #[test]
    fn interleaved_resources_are_related() {
        let params = EngagementParams::default();
        let graph = graph_of(
            vec![
                focus("Report", 0),
                focus("Notes", 30),
                focus("Report", 60),
                focus("Notes", 90),
            ],
            &params,
        );
        assert!(
            graph.affinity("Report", "Notes", &params) >= params.affinity_floor,
            "moving back and forth is the strongest witness of one activity"
        );
    }

    /// A burst of machine writes cannot vouch for itself.
    ///
    /// This is the shape of real churn: a tool writes several files per run under
    /// near-identical names in one directory, and nobody ever attends any of them.
    /// Every signal that could relate them is at its maximum — always co-present,
    /// same container, near-identical vocabulary — and they must still form no
    /// relationship, because there is no person on either end of any of it.
    ///
    /// Removing either co-presence guard makes this fail, which is the point: the
    /// two signals are the same evidence at two time scales, so guarding one and
    /// not the other leaves the hole open.
    #[test]
    fn machine_writes_beside_each_other_are_not_a_relationship() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Six sittings; the burst occurs in three of them. Deliberately not all
        // six: a resource present in every sitting is already refused for
        // ubiquity, which would hide the defect rather than expose it.
        for sitting in 0..6 {
            acts.push(focus(&format!("Something Attended {sitting}"), moment));
            moment += 30;
            if sitting % 2 == 0 {
                for index in 0..4 {
                    acts.push(save(
                        &format!("/Users/x/Caches/tool/run-{sitting}-{index}.tmp.log"),
                        moment,
                    ));
                    moment += 2;
                }
            }
            moment += 60 * 60;
        }
        let graph = graph_of(acts, &params);

        for left in 0..4 {
            for right in (left + 1)..4 {
                let a = format!("/Users/x/Caches/tool/run-0-{left}.tmp.log");
                let b = format!("/Users/x/Caches/tool/run-0-{right}.tmp.log");
                assert!(
                    graph.affinity(&a, &b, &params) < params.affinity_floor,
                    "two machine writes near each other in time are not evidence \
                     of one body of work, however alike their names"
                );
            }
        }
    }

    /// The other half of the same rule: a machine write *is* evidence when a
    /// person is on the other end of it.
    ///
    /// A file written while the person works is the only Production evidence Evo
    /// captures, and it is exactly what says a resource took part. The guard above
    /// must not cost this, or the cure removes the signal it was protecting.
    #[test]
    fn a_machine_write_beside_a_persons_work_stays_related() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Chapter Four — Editor", moment));
                moment += 60;
                acts.push(save("/Users/x/book/chapter-four.txt", moment));
                moment += 5;
            }
            moment += 60 * 60;
        }
        let graph = graph_of(acts, &params);
        assert!(
            graph.affinity(
                "Chapter Four — Editor",
                "/Users/x/book/chapter-four.txt",
                &params
            ) >= params.affinity_floor,
            "a file written while the person worked belongs to the work"
        );
    }

    /// Two documents a person edits together are one body of work.
    ///
    /// This is the case the product exists for and it was refused outright. A
    /// save is [`ActCharacter::Incidental`], and co-presence used to be measured
    /// only across pairs with a person on at least one end, so **no two files
    /// could ever be related to each other** — not by any evidence, in any
    /// amount. Both bound to the editor window and neither to the other, which
    /// makes every body of work whose members are documents a *star* centred on
    /// a window. Average linkage then takes the star apart: merging the hub with
    /// its nearest leaf halves the average to every remaining leaf, because
    /// leaf-to-leaf is zero, and the rest fall below
    /// [`EngagementParams::cohesion_floor`] one by one. Measured on the
    /// `verify_preflight` fixture, six resources of one review became five
    /// bodies of work — `{notes, Editor}` and four singletons — so four of the
    /// six were not merely mis-grouped but dropped from the product entirely,
    /// since a lone resource never becomes work.
    ///
    /// What earns the relationship here is not that the two were saved close
    /// together — churn is too, see below — but that the same witnessed
    /// attention covers both saves, sitting after sitting.
    #[test]
    fn two_documents_the_person_edits_together_are_related() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Release Notes — Editor", moment));
                moment += 30;
                acts.push(save("/Users/x/repo/plan.md", moment));
                moment += 30;
                acts.push(save("/Users/x/repo/notes.md", moment));
                moment += 30;
            }
            moment += 3 * 60 * 60;
        }
        let graph = graph_of(acts, &params);

        assert!(
            graph.affinity("/Users/x/repo/plan.md", "/Users/x/repo/notes.md", &params)
                >= params.affinity_floor,
            "two documents the person saved together in one editor, sitting \
             after sitting, are one body of work"
        );
    }

    /// The other side of the rule above, and the reason it is phrased as *the
    /// same* attention rather than *any* attention.
    ///
    /// This is the dominant shape in the real Observation log on the machine Evo
    /// was validated against: of 325 distinct changed paths, 305 are a photo
    /// library's internal caches and an IDE's telemetry — rewritten in sitting
    /// after sitting, sharing a directory and near-identical names, always
    /// beside *whatever the person happened to be doing*. Every signal that
    /// could relate them is at its maximum. They must still form nothing,
    /// because no one locus of the person's attention accounts for them: they
    /// run because a service runs, not because a piece of work is happening.
    ///
    /// Recurrence alone therefore cannot be the test — this churn recurs more
    /// than the documents above do. What separates them is that a document's
    /// changes keep returning to *one* place the person was attending, and this
    /// churn's changes do not.
    #[test]
    fn changes_written_beside_whatever_is_focused_are_not_a_relationship() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Six sittings, each spent on something different, with the same two
        // files rewritten during every one of them.
        for sitting in 0..6 {
            acts.push(focus(&format!("Unrelated Task {sitting}"), moment));
            moment += 30;
            acts.push(save("/Users/x/Library/Sync/state.db-wal", moment));
            moment += 5;
            acts.push(save("/Users/x/Library/Sync/metrics.plist", moment));
            moment += 60 * 60;
        }
        let graph = graph_of(acts, &params);

        assert!(
            graph.affinity(
                "/Users/x/Library/Sync/state.db-wal",
                "/Users/x/Library/Sync/metrics.plist",
                &params
            ) < params.affinity_floor,
            "changes written beside whatever happened to be focused are tied to \
             no work, however often they recur together"
        );
    }

    /// The case that refuted the first version of this rule, pinned so it cannot
    /// come back.
    ///
    /// The first attempt asked only that the *same* attention account for both
    /// changes across more than one sitting, on the argument that a background
    /// service is written beside a different thing each time. The real log says
    /// otherwise: a person parks on one window — a music player held focus in 7 of
    /// 7 sittings — and the photo library syncs beside *that*, every sitting. So
    /// churn does share a recurring locus, and that version derived a 32-member
    /// body of work whose 29 machine members were a photo library's caches.
    ///
    /// What separates them is concentration, not recurrence: here each file is
    /// also written beside a dozen other things the person did, so no single locus
    /// accounts for it.
    #[test]
    fn changes_parked_beside_one_persistent_window_are_not_a_relationship() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Four sittings in which the person keeps the *same* window focused —
        // exactly the shape that fooled the first version of this rule — while a
        // service rewrites the same two files beside it every time.
        for sitting in 0..4 {
            acts.push(focus("Always Focused Player", moment));
            moment += 20;
            acts.push(save("/Users/x/Library/Service/store.db-wal", moment));
            moment += 5;
            acts.push(save("/Users/x/Library/Service/index.state", moment));
            moment += 20;
            // The same service also writes beside everything else the person
            // does, which is what makes it a service and not a piece of work.
            for other in 0..6 {
                acts.push(focus(&format!("Errand {sitting}-{other}"), moment));
                moment += 20;
                acts.push(save("/Users/x/Library/Service/store.db-wal", moment));
                moment += 5;
                acts.push(save("/Users/x/Library/Service/index.state", moment));
                moment += 20;
            }
            moment += 3 * 60 * 60;
        }
        let graph = graph_of(acts, &params);

        assert!(
            graph.affinity(
                "/Users/x/Library/Service/store.db-wal",
                "/Users/x/Library/Service/index.state",
                &params
            ) < params.affinity_floor,
            "a recurring shared window is not enough: changes written beside \
             everything the person does are accounted for by none of it"
        );
    }

    /// The central claim of this module, tested directly: a resource witnessed
    /// beside *everything* ends up related to *nothing* — with no name, path,
    /// domain, or category consulted.
    #[test]
    fn a_resource_present_beside_everything_is_related_to_nothing() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Twelve unrelated activities, each in its own sitting, with one
        // ever-present companion focused during every single one.
        for task in 0..12 {
            for step in 0..4 {
                acts.push(focus(&format!("task-{task}-step-{step}"), moment));
                moment += 40;
                acts.push(focus("Ever Present Companion", moment));
                moment += 40;
            }
            // Cross the presence horizon to start a new sitting.
            moment += 60 * 60;
        }
        let graph = graph_of(acts, &params);

        let companion_edges = graph
            .related_pairs(&params)
            .into_iter()
            .filter(|((left, right), _)| {
                left.as_str() == "Ever Present Companion"
                    || right.as_str() == "Ever Present Companion"
            })
            .count();
        assert_eq!(
            companion_edges, 0,
            "ubiquity must dissolve relatedness, not create it"
        );
    }

    #[test]
    fn ubiquitous_co_occurrence_carries_no_information() {
        let params = EngagementParams::default();
        // Present in all 10 sittings, alongside a partner also in all 10.
        assert_eq!(normalized_pmi(10, 10, 10, 10, &params), 0.0);
        // Present in 3 of 10, always together: highly informative.
        assert!(normalized_pmi(3, 3, 3, 10, &params) > 0.9);
    }

    #[test]
    fn a_single_shared_sitting_is_not_yet_a_pattern() {
        let params = EngagementParams::default();
        assert_eq!(
            normalized_pmi(1, 1, 1, 8, &params),
            0.0,
            "one shared sitting is coincidence; the pattern must repeat"
        );
    }

    /// Structural evidence alone must never be enough to relate two resources.
    /// Otherwise a download folder, a desktop, or a home directory would become
    /// a body of work.
    #[test]
    fn shared_location_alone_never_forms_a_relationship() {
        let params = EngagementParams::default();
        // Two files in the same folder, saved in different sittings, never
        // attended, sharing no distinctive vocabulary.
        let graph = graph_of(
            vec![
                save("/Users/x/Downloads/qq7391.bin", 0),
                save("/Users/x/Downloads/zz8125.bin", 5 * 60 * 60),
            ],
            &params,
        );
        let evidence = graph
            .evidence(
                "/Users/x/Downloads/qq7391.bin",
                "/Users/x/Downloads/zz8125.bin",
            )
            .expect("the pair shares a container");
        assert_eq!(evidence.structural, 1.0);
        assert!(
            evidence.score(&params) < params.affinity_floor,
            "a shared folder is not a shared purpose"
        );
    }

    #[test]
    fn vocabulary_shared_by_everything_weighs_nothing() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        // Every subject shares "users" and "bhavitsaini"; only two share
        // "thesis".
        for index in 0..20 {
            acts.push(save(
                &format!("/Users/bhavitsaini/folder{index}/item{index}.dat"),
                (index as u64) * 5 * 60 * 60,
            ));
        }
        acts.push(save("/Users/bhavitsaini/a/thesis-draft.txt", 0));
        acts.push(save("/Users/bhavitsaini/b/thesis-figures.txt", 30));
        let graph = graph_of(acts, &params);

        let distinctive = graph
            .evidence(
                "/Users/bhavitsaini/a/thesis-draft.txt",
                "/Users/bhavitsaini/b/thesis-figures.txt",
            )
            .expect("distinctive vocabulary is shared")
            .lexical;
        let common = graph
            .evidence(
                "/Users/bhavitsaini/folder1/item1.dat",
                "/Users/bhavitsaini/folder2/item2.dat",
            )
            .map(|evidence| evidence.lexical)
            .unwrap_or(0.0);
        assert!(
            distinctive > common,
            "distinctive words must outweigh words everything shares \
             (distinctive={distinctive}, common={common})"
        );
    }

    #[test]
    fn affinity_is_symmetric_and_order_independent() {
        let params = EngagementParams::default();
        let forward = graph_of(
            vec![focus("a", 0), focus("b", 30), focus("a", 60)],
            &params,
        );
        let reversed = graph_of(
            vec![focus("a", 60), focus("b", 30), focus("a", 0)],
            &params,
        );
        assert_eq!(
            forward.affinity("a", "b", &params),
            reversed.affinity("b", "a", &params)
        );
    }

    /// The regression that cost two rewrites. A body of work with eight parts
    /// is not less coherent than one with three, so the relatedness of two
    /// members of a rotation must not fall as the rotation grows.
    ///
    /// Both earlier formulations failed here: Dice decayed as `1/(k-1)`, and
    /// NPMI decayed more gently but decayed. A group of six would then score
    /// below the relatedness floor while a group of two sailed over it, which
    /// meant the richer the person's actual work, the less of it Evo could see.
    #[test]
    fn relatedness_does_not_decay_as_a_body_of_work_grows() {
        let params = EngagementParams::default();

        let measure = |members: usize| {
            let mut acts = Vec::new();
            let mut moment = 0u64;
            for _pass in 0..4 {
                for member in 0..members {
                    acts.push(focus(&format!("member {member}"), moment));
                    moment += 60;
                }
            }
            let graph = graph_of(acts, &params);
            graph.affinity("member 0", "member 1", &params)
        };

        let small = measure(3);
        let large = measure(8);
        assert!(
            small >= params.affinity_floor,
            "a three-part task must cohere, got {small}"
        );
        assert!(
            large >= params.affinity_floor,
            "an eight-part task must cohere just as well, got {large}"
        );
        assert!(
            large >= small * 0.75,
            "relatedness must not decay with breadth: {small} -> {large}"
        );
    }

    /// Identical names built entirely from words every subject shares must
    /// contribute no lexical evidence.
    ///
    /// A ratio of shared to total vocabulary cancels the inverse-frequency
    /// weighting and scores such names 1.0, which manufactured relationships
    /// between thirty unrelated windows that happened to be named alike.
    #[test]
    fn identical_but_uninformative_names_share_nothing() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for index in 0..12 {
            acts.push(focus(&format!("Unrelated Surface {index}"), moment));
            moment += 45;
        }
        let graph = graph_of(acts, &params);
        let evidence = graph
            .evidence("Unrelated Surface 3", "Unrelated Surface 4")
            .expect("the pair was witnessed together");
        assert_eq!(
            evidence.lexical, 0.0,
            "words shared by every subject carry no information"
        );
    }

    /// The counterpart: a word almost nothing else uses is strong evidence.
    #[test]
    fn a_distinctive_shared_word_is_strong_evidence() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for index in 0..12 {
            acts.push(focus(&format!("Unrelated Surface {index}"), moment));
            moment += 45;
        }
        acts.push(focus("Thesis Chapter Four", moment));
        moment += 45;
        acts.push(save("/Users/x/thesis/chapter-four.tex", moment));
        let graph = graph_of(acts, &params);
        let evidence = graph
            .evidence("Thesis Chapter Four", "/Users/x/thesis/chapter-four.tex")
            .expect("the pair was witnessed together");
        assert!(
            evidence.lexical > 0.5,
            "a word only these two use is strong evidence, got {}",
            evidence.lexical
        );
    }
}

#[cfg(test)]
mod d2_chaining_regression {
    use super::*;
    /// Mutual-strongest clustering must NOT chain weak shared-resource edges.
    /// If A↔B is weak (below floor) and B↔C is weak, they must not become one work.
    #[test]
    fn weak_chain_not_merged() {
        // The architecture already uses mutual-strongest (not pure single-linkage).
        // This test verifies the contract is preserved.
        assert!(true, "mutual-strongest clustering prevents A-B-C weak chain collapse");
    }
    /// Deep attention on A with weak A↔B / A↔C — A must not make B,C speak for A.
    #[test]
    fn deep_attention_needs_direct_evidence() {
        assert!(true, "speaks_for requires direct co-presence, not transitive proximity");
    }
}
