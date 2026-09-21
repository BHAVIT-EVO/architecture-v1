//! Deterministic, overlapping hypotheses about bodies of work.
//!
//! A hypothesis is not a partition and is not a UI Workspace. It is a stable
//! explanation of which occurrence-local activity contexts appear to express
//! the same human purpose. The same context may support several hypotheses.

use std::collections::{BTreeMap, BTreeSet};

use crate::{ActivityContext, AffinityGraph, Declarations, EngagementParams, IdentityIndex, ResourceRole};

const FNV_OFFSET_A: u64 = 0xcbf29ce484222325;
const FNV_OFFSET_B: u64 = 0x8422_2325_cbf2_9ce4;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// Stable identity of a body-of-work hypothesis, derived from its first
/// accepted nucleus rather than from a mutable title or latest participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkId(u128);

impl WorkId {
    pub fn as_u128(self) -> u128 { self.0 }

    pub(crate) fn from_nucleus(nucleus: &BTreeSet<String>) -> Self {
        work_id(nucleus)
    }
}

impl std::fmt::Display for WorkId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:032x}", self.0)
    }
}

/// A separately retained reason for or against a context-to-work claim.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkEvidence {
    SharedSelectiveSubjects { count: usize, weight: f64 },
    CorroboratedAffinity { pairs: usize, mean: f64 },
    ExplicitGrouping,
    UbiquitousOnlyBridge,
    CompetingSelectiveCore { unsupported: usize },
}

impl WorkEvidence {
    pub fn supports(&self) -> bool {
        matches!(self, Self::SharedSelectiveSubjects { .. } | Self::CorroboratedAffinity { .. } | Self::ExplicitGrouping)
    }
}

/// Graded, explainable attribution of one occurrence context to one work.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkContextLink {
    context: usize,
    relevance: f64,
    contradiction: f64,
    evidence: Vec<WorkEvidence>,
}

impl WorkContextLink {
    pub fn context(&self) -> usize { self.context }
    pub fn relevance(&self) -> f64 { self.relevance }
    pub fn contradiction(&self) -> f64 { self.contradiction }
    pub fn evidence(&self) -> &[WorkEvidence] { &self.evidence }
}

/// One non-exclusive candidate body of work.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkHypothesis {
    id: WorkId,
    nucleus: BTreeSet<String>,
    links: Vec<WorkContextLink>,
}

/// The durable, non-exclusive relationship between one artifact and one work.
///
/// Membership relevance and restoration role are deliberately independent: a
/// strongly related reference may stay closed, while a lower-breadth but
/// decisive continuation surface may open immediately.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkArtifactLink {
    work_id: WorkId,
    subject: String,
    relevance: f64,
    specificity: f64,
    role: ResourceRole,
    contexts: BTreeSet<usize>,
}

impl WorkArtifactLink {
    pub(crate) fn new(
        work_id: WorkId,
        subject: String,
        relevance: f64,
        specificity: f64,
        role: ResourceRole,
        contexts: BTreeSet<usize>,
    ) -> Self {
        Self { work_id, subject, relevance, specificity, role, contexts }
    }

    pub fn work_id(&self) -> WorkId { self.work_id }
    pub fn subject(&self) -> &str { &self.subject }
    pub fn relevance(&self) -> f64 { self.relevance }
    pub fn specificity(&self) -> f64 { self.specificity }
    pub fn role(&self) -> ResourceRole { self.role }
    pub fn contexts(&self) -> &BTreeSet<usize> { &self.contexts }
    pub fn opens_on_restore(&self) -> bool { self.role.opens_on_restore() }
}

impl WorkHypothesis {
    pub fn id(&self) -> WorkId { self.id }
    pub fn nucleus(&self) -> &BTreeSet<String> { &self.nucleus }
    pub fn links(&self) -> &[WorkContextLink] { &self.links }
    pub fn contexts(&self) -> BTreeSet<usize> {
        self.links.iter().map(WorkContextLink::context).collect()
    }
}

/// Forms hypotheses by comparing every candidate context directly with a
/// stable nucleus. No accepted context can recruit another context, so support
/// is not transitive. Maximal compatible hypotheses survive; contexts may occur
/// in more than one survivor.
pub(crate) fn infer_work_hypotheses(
    contexts: &[ActivityContext],
    graph: &AffinityGraph,
    declarations: &Declarations,
    params: &EngagementParams,
    identity: &IdentityIndex,
) -> Vec<WorkHypothesis> {
    if contexts.is_empty() { return Vec::new(); }

    let frequency = subject_frequency(contexts);
    let mut candidates = Vec::new();
    for (nucleus_index, context) in contexts.iter().enumerate() {
        if context.human_subjects().is_empty() { continue; }
        for nucleus in candidate_nuclei(context, graph, params, &frequency, contexts.len()) {
            let mut links = Vec::new();
            for (index, candidate) in contexts.iter().enumerate() {
                let link = assess_context(index, candidate, &nucleus, &frequency, contexts.len(), graph, declarations, params);
                if index == nucleus_index || (link.relevance > params.affinity_floor && link.relevance > link.contradiction) {
                    links.push(link);
                }
            }
            links.sort_by_key(WorkContextLink::context);
            candidates.push(WorkHypothesis { id: stable_work_id(&nucleus, identity), nucleus, links });
        }
    }

    // Identity, not an equal set of supporting contexts, determines whether
    // two candidates are the same work. X and Y can be interleaved inside the
    // exact same local contexts; deleting one because their context sets match
    // recreates exclusive clustering at the deduplication boundary.
    candidates.sort_by(|a, b| b.links.len().cmp(&a.links.len()).then_with(|| a.id.cmp(&b.id)));
    let mut kept: Vec<WorkHypothesis> = Vec::new();
    for candidate in candidates {
        if kept.iter().any(|existing| existing.id == candidate.id) {
            continue;
        }
        kept.push(candidate);
    }
    kept.sort_by_key(|hypothesis| hypothesis.id);
    kept
}

fn subject_frequency(contexts: &[ActivityContext]) -> BTreeMap<String, usize> {
    let mut frequency = BTreeMap::new();
    for context in contexts {
        for subject in context.human_subjects() {
            *frequency.entry(subject.clone()).or_insert(0) += 1;
        }
    }
    frequency
}

fn candidate_nuclei(
    context: &ActivityContext,
    graph: &AffinityGraph,
    params: &EngagementParams,
    frequency: &BTreeMap<String, usize>,
    total: usize,
) -> Vec<BTreeSet<String>> {
    let mut ranked: Vec<String> = context.human_subjects().iter().cloned().collect();
    ranked.sort_by_key(|subject| (frequency.get(subject).copied().unwrap_or(0), subject.clone()));
    let mut selective: BTreeSet<String> = ranked.iter()
        .filter(|subject| frequency.get(*subject).copied().unwrap_or(total) * 4 <= total)
        .cloned().collect();
    if selective.is_empty() {
        selective.extend(ranked.into_iter().take(2));
    }

    // A local context is evidence, not an identity boundary. Rapid switching
    // may place several works in one context, so derive mutually corroborating
    // cores within it. The same non-selective browser/tool remains available as
    // membership evidence but cannot bridge those cores into one WorkId.
    crate::engagement::cluster_within(&selective, graph, params)
}

#[allow(clippy::too_many_arguments)]
fn assess_context(
    index: usize,
    context: &ActivityContext,
    nucleus: &BTreeSet<String>,
    frequency: &BTreeMap<String, usize>,
    total: usize,
    graph: &AffinityGraph,
    declarations: &Declarations,
    params: &EngagementParams,
) -> WorkContextLink {
    let shared: Vec<&String> = nucleus.intersection(context.human_subjects()).collect();
    let shared_weight: f64 = shared.iter().map(|subject| {
        ((total + 1) as f64 / (frequency.get(*subject).copied().unwrap_or(total) + 1) as f64).ln()
    }).sum();
    let declared = nucleus.iter().any(|left| context.human_subjects().iter().any(|right| declarations.is_grouped(left, right)));

    let mut supported = 0usize;
    let mut support_sum = 0.0;
    let mut corroborated = 0usize;
    for subject in context.human_subjects() {
        let mut best = 0.0_f64;
        let mut is_corroborated = false;
        for core in nucleus {
            if subject == core { best = 1.0; is_corroborated = true; continue; }
            if let Some(evidence) = graph.evidence(subject, core) {
                let score = evidence.score(params);
                if score > best { best = score; is_corroborated = evidence.is_corroborated(); }
            }
        }
        if best >= params.affinity_floor {
            supported += 1;
            support_sum += best;
            corroborated += usize::from(is_corroborated);
        }
    }
    let mean = if supported == 0 { 0.0 } else { support_sum / supported as f64 };
    let ubiquitous_only = !shared.is_empty()
        && shared.iter().all(|subject| frequency.get(*subject).copied().unwrap_or(total) * 4 > total)
        && corroborated == 0;
    let unsupported = context.human_subjects().len().saturating_sub(supported);

    let mut evidence = Vec::new();
    if !shared.is_empty() { evidence.push(WorkEvidence::SharedSelectiveSubjects { count: shared.len(), weight: shared_weight }); }
    if supported > 0 { evidence.push(WorkEvidence::CorroboratedAffinity { pairs: supported, mean }); }
    if declared { evidence.push(WorkEvidence::ExplicitGrouping); }
    if ubiquitous_only { evidence.push(WorkEvidence::UbiquitousOnlyBridge); }
    if unsupported >= supported && unsupported > 0 && context.human_subjects().len() > 1 {
        evidence.push(WorkEvidence::CompetingSelectiveCore { unsupported });
    }

    // One shared surface is weak evidence; requiring two nucleus loci (or a
    // corroborated affinity) prevents ubiquitous windows from recruiting an
    // entire history through a single bridge.
    let direct = if shared.len() >= 2 {
        (shared.len() as f64 / nucleus.len().max(2) as f64).min(1.0)
    } else {
        0.0
    };
    let semantic = if corroborated >= 2 { mean } else { 0.0 };
    let relevance = if declared { 1.0 } else { direct.max(semantic) };
    let contradiction = if ubiquitous_only { 1.0 } else if unsupported >= supported && unsupported > 0 { unsupported as f64 / context.human_subjects().len() as f64 } else { 0.0 };
    WorkContextLink { context: index, relevance, contradiction, evidence }
}

fn work_id(nucleus: &BTreeSet<String>) -> WorkId {
    let mut left = hash_bytes(FNV_OFFSET_A, b"evo-work-nucleus-v1");
    let mut right = hash_bytes(FNV_OFFSET_B, b"evo-work-nucleus-v1");
    for subject in nucleus {
        left = hash_bytes(hash_bytes(left, subject.as_bytes()), &[0]);
        right = hash_bytes(hash_bytes(right, subject.as_bytes()), &[0]);
    }
    WorkId((u128::from(left) << 64) | u128::from(right))
}

/// Hashes the immutable names first witnessed for the nucleus resources.
/// Canonical names are conclusions that may improve as history grows; founding
/// names are append-only facts, so repeated contexts converge and later
/// canonicalization cannot rename the work's identity.
fn stable_work_id(nucleus: &BTreeSet<String>, identity: &IdentityIndex) -> WorkId {
    let stable: BTreeSet<String> = nucleus.iter().map(|subject| {
        identity.founding_name(subject).unwrap_or(subject).to_string()
    }).collect();
    work_id(&stable)
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes { hash ^= u64::from(*byte); hash = hash.wrapping_mul(FNV_PRIME); }
    hash
}
