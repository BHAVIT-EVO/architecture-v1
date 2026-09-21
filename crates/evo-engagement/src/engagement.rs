//! Engagements: bodies of work reconstructed from witnessed evidence.
//!
//! An **Engagement** is a set of resources the evidence says were being used
//! together for one purpose, together with the internal structure that makes it
//! resumable: which resource the work continues in, which ones participated,
//! which were merely consulted, and which were only ever present.
//!
//! This is the layer that was missing. Previously an Artifact that had been
//! witnessed twice became a Workspace directly, so Workspace identity was
//! Artifact identity and every resource the machine touched became a "body of
//! work". Engagement sits between them and answers a different question:
//! *what was going on here?*
//!
//! # Why clustering is average-linkage
//!
//! Single-linkage clustering chains: if any one resource of task A has a single
//! edge to any one resource of task B, the two tasks fuse. That is precisely
//! the failure that must not happen — two tasks in one repository, or two tasks
//! in one application, must stay distinct. Average linkage requires the two
//! groups to be related *as groups*, so one incidental bridge cannot merge
//! them.

use crate::affinity::{AffinityEvidence, AffinityGraph, EvidenceKind};
use crate::declarations::Declarations;
use crate::episode::{Act, AttentionLedger, AttentionRecord, Episode};
use crate::params::EngagementParams;
use crate::resource::Resource;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::{Duration, SystemTime};

fn absolute_seconds_between(left: SystemTime, right: SystemTime) -> f64 {
    left.duration_since(right)
        .or_else(|_| right.duration_since(left))
        .unwrap_or(Duration::ZERO)
        .as_secs_f64()
}

/// How much of a body of work Evo is prepared to claim.
///
/// Three separate claims about one body of work, deliberately not collapsed into
/// a boolean. Evo used to make one decision — present it or discard it — and
/// discarding was implemented by dropping the group into a list nothing read.
/// Measured on this machine's real history, that lost 330 groups and 341
/// resources, among them everything a person did once and did not return to.
///
/// These are ordered by how much is claimed, so `>=` reads as "at least".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Standing {
    /// Evo witnessed this and keeps the whole record of it. Findable by name,
    /// never presented as work, never opened.
    ///
    /// A thirty-second read of a document, once, is exactly this: frequency is
    /// evidence, not truth, and something used once may still be the thing the
    /// person needs back. Evo simply will not claim it was a body of work.
    Remembered,
    /// A body of work: presented, named, with its members and their evidence.
    /// Evo can say what this was and what it consisted of.
    Continuable,
    /// Continuable, and the evidence names one unambiguous place the work
    /// continues — so Evo can put the person back into it rather than merely
    /// describing it.
    Restorable,
}

impl Standing {
    /// Whether Evo will present this as a body of work at all.
    pub fn is_work(self) -> bool {
        self >= Standing::Continuable
    }

    /// Whether Evo may open anything automatically for this.
    pub fn is_restorable(self) -> bool {
        self == Standing::Restorable
    }

    /// A short, honest phrase for this standing.
    pub fn phrase(self) -> &'static str {
        match self {
            Standing::Remembered => "witnessed and kept, not claimed as work",
            Standing::Continuable => "a body of work, with no single place it resumes",
            Standing::Restorable => "a body of work Evo can put you back into",
        }
    }
}

/// How a resource participates in a body of work.
///
/// These are deliberately five distinct states rather than one boolean,
/// because "is part of this work" and "should be reopened to resume this work"
/// are different claims and conflating them is what produces twenty windows.
///
/// The five answer, in order, the five questions a Workspace has to be able to
/// tell apart: where did the work stop, where does it happen, what was used as
/// part of it, what was consulted, and what merely happened to be there.
///
/// The declaration order is the order of importance:
/// [`ResourceRole::Continuation`] sorts first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceRole {
    /// The single place the work stopped, and so where it resumes. Exactly one
    /// member of a body of work holds this role.
    ///
    /// This is [`ResourceRole::Primary`] with one further fact attached: of the
    /// places the work happens, this is the one attended most recently. Keeping
    /// it distinct is what lets Restoration name *one* Resume Point without
    /// ranking Artifacts by confidence, identifier, or attachment order — none of
    /// which is evidence about where a person left off.
    Continuation,
    /// Where the work happens. Restoration opens these, and only these.
    Primary,
    /// Actively used as part of the work, but not where the work continues.
    /// Restoration lists these; the person opens them if they want them.
    Supporting,
    /// Witnessed in passing: looked at briefly, or changed with no attention
    /// Evo could measure. Part of the work, but not somewhere the person
    /// worked.
    Reference,
    /// Around while the work was going on, and nothing more. Either Evo
    /// witnessed nothing happen here during any sitting of this work, or all it
    /// saw was a change it cannot attribute to anyone — a file the machine
    /// rewrote — tied to the work by nothing but having happened at the same
    /// time. Evo records it and says exactly that.
    Context,
}

impl ResourceRole {
    /// A short, honest phrase for this role.
    pub fn phrase(self) -> &'static str {
        match self {
            ResourceRole::Continuation => "where you left off",
            ResourceRole::Primary => "where you were working",
            ResourceRole::Supporting => "used as part of this",
            ResourceRole::Reference => "used in passing",
            ResourceRole::Context => "present, but nothing happened here",
        }
    }

    /// Whether Restoration should open this resource automatically.
    pub fn opens_on_restore(self) -> bool {
        matches!(self, ResourceRole::Continuation | ResourceRole::Primary)
    }
}

/// One resource's participation in one body of work.
#[derive(Debug, Clone, PartialEq)]
pub struct Participant {
    resource: Resource,
    role: ResourceRole,
    attention: Duration,
    recurrence: usize,
    last_seen: Option<SystemTime>,
    first_seen: Option<SystemTime>,
    /// The strongest witnessed reason this resource belongs here, and the
    /// resource it is tied to. `None` for a lone participant.
    tie: Option<(String, EvidenceKind)>,
    /// Whether any tie to another member rests on more than co-presence.
    ///
    /// See [`AffinityEvidence::is_corroborated`]. This is what separates being
    /// part of the work from having been alongside it.
    corroborated: bool,
    /// What share of this resource's own witnessed life coincided with this body
    /// of work's sittings. See [`Participant::locality`].
    locality: f64,
    /// How many sittings this resource was *ever* witnessed in, across the whole
    /// history — the denominator [`Participant::locality`] is computed over.
    ///
    /// Kept because a ratio over a single trial carries no information: a
    /// resource witnessed in exactly one sitting is local to that sitting by
    /// construction, whatever its nature. See [`Participant::locality_is_evidence`].
    witnessed_sittings: usize,
    /// Whether this resource is corroborated by a member a person was
    /// demonstrably *active* in, rather than by other members nobody attended.
    ///
    /// [`Participant::corroborated`] asks whether wording, location or a
    /// declaration ties this resource to *any* other member. That is the right
    /// question for membership and the wrong one for attributing a change,
    /// because a set of resources nobody ever attended can satisfy it entirely
    /// among themselves. See [`Participant::is_attention_corroborated`].
    attention_corroborated: bool,
    /// Whether Evo ever witnessed this resource somewhere other than this body of
    /// work. See [`Participant::is_exclusive`].
    exclusive: bool,
    /// How strongly the evidence ties this resource to the rest of the work.
    /// See [`Participant::belonging`].
    belonging: f64,
    /// The sittings *of this work* in which this resource was active.
    episodes: BTreeSet<usize>,
    /// Whether a person was witnessed acting on this resource during a sitting
    /// of *this* work. See [`Participant::person_acted`].
    person_acted: bool,
    /// How strongly the evidence ties this resource to *this* body of work
    /// specifically: its membership strength here. See
    /// [`Participant::strength`].
    strength: f64,
    /// What share of this resource's total measured relatedness, across every
    /// body of work it belongs to, falls here. See
    /// [`Participant::specificity`].
    specificity: f64,
    /// Whether this resource is one of the loci this body of work was formed
    /// around. See [`Participant::is_seed`].
    seed: bool,
    /// The most attention this resource drew in any single sitting *of this
    /// work* — its one deepest occasion of the person's attention here.
    ///
    /// Distinct from the whole-life depth [`Participant::is_exclusive`] rests on:
    /// measured strictly within this work's own sittings, because leading this
    /// work is a claim about attention spent *here*. See [`Participant::engaged`].
    deepest_sitting_here: Duration,
}

impl Participant {
    /// The witnessed resource.
    pub fn resource(&self) -> &Resource {
        &self.resource
    }

    /// The canonical subject.
    pub fn subject(&self) -> &str {
        self.resource.subject()
    }

    /// How this resource participates.
    pub fn role(&self) -> ResourceRole {
        self.role
    }

    /// Attention Evo can claim was spent here.
    pub fn attention(&self) -> Duration {
        self.attention
    }

    /// How many distinct sittings this resource was witnessed in.
    pub fn recurrence(&self) -> usize {
        self.recurrence
    }

    /// The last moment this resource was witnessed.
    pub fn last_seen(&self) -> Option<SystemTime> {
        self.last_seen
    }

    /// The first moment this resource was witnessed.
    ///
    /// The whole witnessed life of the resource, not only its life inside this
    /// body of work. Higher layers use it to name the member a body of work grew
    /// out of, and that founding member must not change identity because the work
    /// later spread into another sitting.
    pub fn first_seen(&self) -> Option<SystemTime> {
        self.first_seen
    }

    /// Whether this resource's belonging here rests on more than having been
    /// present at the same time as the rest.
    ///
    /// See [`crate::affinity::AffinityEvidence::is_corroborated`]. Reported for
    /// explanation only — deliberately *not* what decides roles. A resource can be
    /// the entire point of a task and share no wording or location with anything in
    /// it but the person's attention; and because distinctive vocabulary is
    /// measured in absolute IDF nats, a token every member shares carries no
    /// information, so a tightly-named body of work corroborates nothing at all.
    pub fn is_corroborated(&self) -> bool {
        self.corroborated
    }

    /// What fraction of this resource's witnessed life coincided with this body of
    /// work, `0.0..=1.0`.
    ///
    /// Of all the sittings this resource was ever witnessed in, the share that
    /// were sittings *of this work* — those in which more than one of its members
    /// was active. One means this resource is only ever around when this work is
    /// going on. A small value means it mostly lives elsewhere and happened to
    /// also be here.
    ///
    /// This is the crate's one principle — something that correlates with
    /// everything correlates with nothing — asked of a single resource rather than
    /// of a pair.
    pub fn locality(&self) -> f64 {
        self.locality
    }

    /// How many sittings this resource was ever witnessed in, across the whole
    /// history. This is the denominator of [`Participant::locality`].
    pub fn witnessed_sittings(&self) -> usize {
        self.witnessed_sittings
    }

    /// Whether [`Participant::locality`] carries information about this resource.
    ///
    /// Locality is a ratio: the share of a resource's witnessed sittings that
    /// were sittings of this work. A ratio computed over a single trial is not
    /// evidence. A resource witnessed in exactly one sitting has locality `1.0`
    /// by construction — not because its changes follow this work, but because
    /// there was never an occasion on which they could have been seen not to.
    ///
    /// An ephemeral file is the case that matters: a tool that writes a
    /// freshly-named log on every run produces a resource witnessed once,
    /// forever, whose locality is therefore always whole. Reading that as "this
    /// work is what changes it" promotes pure machine churn to a participant in
    /// the work, which is what this predicate exists to prevent.
    ///
    /// More than one witnessed sitting is the minimum for the ratio to be able
    /// to come out below one, so it is the minimum for the ratio to mean
    /// anything. This is a structural property of the quantity, not a tunable
    /// threshold, so it introduces no parameter (RFC-0014 R5).
    pub fn locality_is_evidence(&self) -> bool {
        self.witnessed_sittings >= 2
    }

    /// Whether wording, location or a declaration ties this resource to a member
    /// of this work that a person was demonstrably **active** in — one with
    /// measurable attention, or one Evo witnessed a human act on.
    ///
    /// This is [`Participant::is_corroborated`] with the pool of possible
    /// corroborators narrowed to those carrying human evidence, and the narrowing
    /// is the whole point. Corroboration among all members lets a group of
    /// resources nobody ever touched vouch for one another: a tool that writes
    /// several similarly-named files per run gives each of them a strong shared
    /// vocabulary with the others, so each looks well-corroborated while the set
    /// as a whole is invisible to the person. Requiring the corroborating member
    /// to be one the person was active in is the crate's stated position on
    /// incidental acts — they "become evidence only when they are *selectively*
    /// associated with attention"
    /// ([`crate::resource::ActCharacter::Incidental`]) — read strictly: with
    /// attention, not with each other.
    ///
    /// It is a question about the presence of evidence, not its magnitude, so it
    /// introduces no parameter (RFC-0014 R5), and it names no application, path,
    /// domain or extension. Where Evo measures no attention at all it is false
    /// for everything, which is the honest answer: with no attention captured
    /// there is nothing a change could be attributed to.
    pub fn is_attention_corroborated(&self) -> bool {
        self.attention_corroborated
    }

    /// How many sittings *of this body of work* this resource was active in.
    ///
    /// The numerator of [`Participant::locality`], and what
    /// [`Participant::returned`] counts. Reported so an explanation can say
    /// "witnessed in three of this work's sittings, and in no others" rather than
    /// quoting a ratio.
    pub fn sittings_here(&self) -> usize {
        self.episodes.len()
    }

    /// Whether this resource's *life* happened here and nowhere else.
    ///
    /// [`Participant::locality`] at its strongest, and asked in a way a ratio
    /// cannot ask. Every sitting in which this resource held the person was either
    /// a sitting of *this* work, or a sitting in which the person was attending
    /// nothing else at all — and required to be more than one such sitting, so the
    /// claim rests on something Evo had an occasion to see contradicted
    /// ([`Participant::locality_is_evidence`] is the same requirement for the
    /// ratio).
    ///
    /// # Why *held the person* and not merely *was witnessed*
    ///
    /// Presence and life are different claims, and reading the second off the
    /// first is a defect real history exposed immediately. A source file the
    /// person spent two long sittings working in was also left open in four later
    /// sittings, drawing sixty seconds, forty seconds, twelve seconds and nothing
    /// — an editor tab tabbed past on the way somewhere else. Counted as presence,
    /// four glimpses outvoted two hours and the file was denied leadership of the
    /// work it *was*; the search page consulted beside it led instead.
    ///
    /// So a sitting joins this resource's life only if the resource drew at least
    /// [`EngagementParams::sustained_sitting_attention`] in it — the same bar Evo
    /// already uses to say a sitting was *spent* in something rather than passed
    /// through it. Not a new threshold: the existing definition of "a sitting's
    /// worth of attention", asked of the question "where does this thing live".
    ///
    /// This cuts in both directions, which is why it is the right reading rather
    /// than a convenient one. It admits the long-worked file whose tab lingered,
    /// and it refuses a page glanced at for nineteen seconds in two different
    /// sittings — witnessed twice, exclusive by presence, and never once a place
    /// anybody worked.
    ///
    /// This is the question "is this resource what the work is *about*", asked in
    /// the only terms Evo can witness. It is not a threshold and not a discount: a
    /// resource Evo saw living a life elsewhere may well have been used here, but
    /// whatever it is, it is not what *this* work is. And it is a fact about the
    /// resource alone, so it cannot move when the body of work gains or loses a
    /// member — the failure that made restoration importance a function of
    /// workspace size.
    ///
    /// # Why a sitting with nothing else in it does not count against it
    ///
    /// A sitting of this work is one in which more than one of its members was
    /// active, which is what stops a resource that is merely *around* from
    /// inflating the work. But the last thing a person touches before they stop is
    /// very often the *only* thing they were touching — they were finishing up in
    /// one place. Counting that sitting as a life elsewhere would demote a resource
    /// precisely *because* it was the last thing used, which is backwards: it is
    /// the moment the product exists to recover. So a sitting outside this work
    /// counts against the resource only if the person was demonstrably occupied
    /// with something else in it. Being alone is the absence of an alternative, not
    /// evidence of one.
    ///
    /// Because the sittings it counts are *this* work's sittings, the same resource
    /// can be exclusive to one body of work and not to another. That is what makes
    /// the roles contextual: a music player the person spends real time in beside
    /// every kind of work is exclusive to none of them, and the same player, in a
    /// body of work about music, lived nowhere else.
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }

    /// Whether a person was witnessed *working in* this resource within this body
    /// of work — it drew more than a glance's worth of attention
    /// ([`EngagementParams::engaged_attention`]) in at least one of this work's
    /// sittings.
    ///
    /// This is the floor for leading a body of work and being reopened as the
    /// place it resumes. It is a weaker claim than [`Participant::is_exclusive`],
    /// which demands a full sitting's depth across two of them, and a different
    /// one from [`Participant::returned`], which demands the return but no
    /// particular depth. A single interrupted afternoon — one deep sitting, no
    /// second visit — is the canonical thing to resume, and it satisfies neither
    /// of those, yet it is unmistakably somewhere work was done.
    ///
    /// Measured within this work's own sittings, for the same reason
    /// [`Participant::returned`] is: a deep sitting spent on this resource for
    /// *other* work says nothing about *this* work being somewhere to carry on.
    pub fn engaged(&self, params: &EngagementParams) -> bool {
        self.deepest_sitting_here >= params.engaged_attention
    }

    /// Whether the person came *back* to this resource within this body of work:
    /// it was active in at least [`EngagementParams::min_revisits`] of this work's
    /// own sittings.
    ///
    /// This is activity → interruption → return asked of a single member. A place
    /// the person left and returned to is a place they had not finished with.
    /// Something witnessed in one unbroken sitting and never again may have been
    /// finished, abandoned, or a one-off, and Evo cannot tell which — so it does
    /// not offer it as somewhere to carry on.
    ///
    /// Counted over this work's sittings rather than over the resource's whole
    /// witnessed life: returning to a resource in the course of other work says
    /// nothing about *this* work being unfinished.
    pub fn returned(&self, params: &EngagementParams) -> bool {
        self.episodes.len() >= params.min_revisits
    }

    /// How strongly the evidence ties this resource to the rest of this body of
    /// work, `0.0..=1.0`: the mean affinity between it and every other member.
    ///
    /// This is the strength of the *membership claim*, and it is a different
    /// quantity from [`Participant::role`], which is the resource's importance
    /// *within* the work. A reference page consulted once can belong beyond
    /// doubt; a place the work continues can belong on thinner evidence than
    /// that. Keeping the two apart is what lets Evo say "this is certainly part
    /// of this work, and you do not need it reopened".
    ///
    /// The mean, not the maximum: a resource tied hard to one member and to
    /// nothing else in the group is exactly the incidental bridge that
    /// average-linkage clustering exists to refuse, and reporting its belonging
    /// as maximal would contradict the reason it was allowed in.
    pub fn belonging(&self) -> f64 {
        self.belonging
    }

    /// The sittings *of this body of work* in which this resource was active.
    ///
    /// Indices into the reconstruction's sittings. Not every sitting the
    /// resource was ever witnessed in — only those it shared with this work,
    /// which is what makes this the record of how the work itself unfolded.
    pub fn episodes(&self) -> &BTreeSet<usize> {
        &self.episodes
    }

    /// Whether a person was witnessed acting on this resource during a sitting of
    /// this body of work.
    ///
    /// True when at least one act here was direct human evidence — attention
    /// directed at it, or a deliberate durable act
    /// ([`crate::resource::ActCharacter::is_human_evidence`]). False when
    /// everything Evo saw was a change it cannot attribute: a file write is
    /// witnessed identically whether a person saved a draft or a service flushed
    /// a cache.
    ///
    /// Distinct from [`Participant::attention`], which can be zero for a resource
    /// a person plainly acted on: a commit is an instant, and a window brought
    /// forward as the last act of a sitting accrues nothing because Evo did not
    /// witness how long the person stayed.
    pub fn person_acted(&self) -> bool {
        self.person_acted
    }

    /// How strongly the evidence ties this resource to *this* body of work,
    /// `0.0..=1.0`: the strongest measured affinity between it and any of the
    /// loci this work was formed around.
    ///
    /// The maximum, not the mean, and the difference matters. A body of work has
    /// several places it happens, and a reference page consulted while working in
    /// one of them is legitimately related to that one and to nothing else in the
    /// work. Averaging would report it as weakly attached to the whole, which
    /// misdescribes what was witnessed: it was strongly attached to a part.
    /// [`Participant::belonging`] is the mean and answers a different question —
    /// how cohesive is this membership with the group as a whole.
    pub fn strength(&self) -> f64 {
        self.strength
    }

    /// What share of everything this resource is related to falls inside *this*
    /// body of work, `0.0..=1.0`.
    ///
    /// ```text
    /// specificity(R, T) = strength(R, T) / Σ over every body of work T' of strength(R, T')
    /// ```
    ///
    /// One means this resource belongs to this work and to nothing else. A small
    /// value means it is beside many of the person's bodies of work at similar
    /// strength, so its presence does not distinguish this one from the others.
    ///
    /// This is the crate's one principle — something that correlates with
    /// everything correlates with nothing — asked at the level the product
    /// actually needs it. [`Participant::locality`] asks the same question of
    /// *sittings*, which cannot separate a resource that is beside several bodies
    /// of work at once from one that is genuinely part of this one, because both
    /// are witnessed in the same sittings. Specificity divides by the person's
    /// other work rather than by the clock.
    ///
    /// # What this replaces
    ///
    /// It replaces the partition. Membership used to be exclusive, so a resource
    /// beside three bodies of work was assigned to one and its other two
    /// relationships were discarded — 152 of 425 resources on this machine's real
    /// history. Specificity is what makes overlapping membership *safe*: every
    /// relationship is kept, and the resource is prevented from speaking for a
    /// body of work it only partly belongs to, rather than being deleted from the
    /// ones it belongs to least.
    ///
    /// It names no application, domain, extension or category, and it is
    /// symmetric. A music player beside every kind of work has low specificity in
    /// each, so it names none of them and opens in none of them; the same music
    /// player, in a body of work *about* music, has all its relatedness there and
    /// leads it.
    ///
    /// A resource in exactly one body of work scores `1.0`, which is correct and
    /// carries no information — with one body of work there was never an occasion
    /// to see the resource belong elsewhere. See
    /// [`Participant::specificity_is_evidence`].
    pub fn specificity(&self) -> f64 {
        self.specificity
    }

    /// Whether [`Participant::specificity`] carries information about this
    /// resource.
    ///
    /// Specificity is a share of a resource's relatedness across the bodies of
    /// work it belongs to. A resource belonging to exactly one has a share of
    /// `1.0` by construction — not because its relatedness is concentrated here,
    /// but because there was never another claimant for it to be divided with.
    ///
    /// The same structural point as [`Participant::locality_is_evidence`], and
    /// like it, no threshold and no parameter: more than one claimant is the
    /// minimum for the ratio to be able to come out below one.
    pub fn specificity_is_evidence(&self) -> bool {
        self.specificity < 1.0
    }

    /// Whether this resource may speak for this body of work — name it, lead it,
    /// or be reopened as it.
    ///
    /// Either the evidence never saw this resource belong anywhere else, or a
    /// strict majority of everything it belongs to is here
    /// ([`EngagementParams::speaks_for_work`]).
    ///
    /// Deliberately not a membership test. A resource that fails it is still a
    /// full member with its strength recorded — remembered, listed, offered.
    /// What it may not do is stand for the work as a whole.
    pub fn speaks_for_work(&self, params: &EngagementParams) -> bool {
        speaks_for(self.specificity, params)
    }

    /// Whether this resource is one of the loci this body of work was formed
    /// around.
    ///
    /// A seed is a resource a person was witnessed working in: they came back to
    /// it across sittings, or spent a sitting's worth of attention in it once.
    /// Bodies of work are formed by clustering *these* and nothing else, which is
    /// what stops machine churn from forming or joining work at the structural
    /// level rather than being demoted after the fact.
    ///
    /// The test is about witnessed human activity only. It says nothing about
    /// what the resource is.
    pub fn is_seed(&self) -> bool {
        self.seed
    }

    /// A grounded, non-fabricated statement of why this resource is here.
    ///
    /// Every phrase is a restatement of a measurement. Nothing is inferred
    /// about content, intent, or subject matter, because Evo did not witness
    /// any of those.
    pub fn why(&self) -> String {
        match &self.tie {
            Some((other, kind)) => format!(
                "{} — {} {}",
                self.role.phrase(),
                kind.phrase(),
                short_name(other)
            ),
            None => self.role.phrase().to_string(),
        }
    }
}

/// A body of work reconstructed from evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct Engagement {
    work_id: crate::WorkId,
    context_links: Vec<crate::WorkContextLink>,
    artifact_links: Vec<crate::WorkArtifactLink>,
    participants: Vec<Participant>,
    attention: Duration,
    episodes: BTreeSet<usize>,
    last_active: Option<SystemTime>,
    first_active: Option<SystemTime>,
    title: String,
    /// The distinctive vocabulary the title shares with the rest of the work, in
    /// nats. Zero means the title is the leading participant's name and nothing
    /// stronger. See [`Engagement::titled_by_shared_vocabulary`].
    title_weight: f64,
    /// How much of this body of work Evo is prepared to claim. See
    /// [`Engagement::standing`].
    standing: Standing,
    /// The fraction of this grouping's member-pairs that are related to each
    /// other, at [`EngagementParams::affinity_floor`]. One for a body of work
    /// whose members were all used together; near zero for a hub with spokes. See
    /// [`Engagement::cohesion`] and [`EngagementParams::internal_cohesion`].
    cohesion: f64,
}

/// A short, ordered neighborhood of human activity.
///
/// Contexts are the occurrence-level unit used to attribute evidence to a
/// body of work. Artifact identity remains canonical and global, but a shared
/// terminal, document, or conversation can occur in several contexts and can
/// therefore belong to several works without importing its whole lifetime into
/// each one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityContext {
    episode: usize,
    acts: Vec<usize>,
    subjects: BTreeSet<String>,
    human_subjects: BTreeSet<String>,
    started_at: Option<SystemTime>,
    ended_at: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq)]
struct ContextGroup {
    work_id: crate::WorkId,
    contexts: BTreeSet<usize>,
    seeds: BTreeSet<String>,
    evidence: Vec<crate::WorkContextLink>,
}

pub(crate) fn build_activity_contexts(
    episodes: &[Episode],
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> Vec<ActivityContext> {
    let mut contexts = Vec::new();
    for (episode_index, episode) in episodes.iter().enumerate() {
        let mut start: Option<usize> = None;
        let mut end: Option<usize> = None;
        let mut human_subjects: BTreeSet<String> = BTreeSet::new();
        for (index, act) in episode.acts().iter().enumerate() {
            if act.character().is_human_evidence() {
                let subject = act.resource().subject();
                let contradicts_active = human_subjects.len() >= 2
                    && !human_subjects.contains(subject)
                    && human_subjects.iter().all(|active| {
                        graph.affinity(active, subject, params) < params.affinity_floor
                    });
                if contradicts_active {
                    if let (Some(start), Some(end)) = (start, end) {
                        contexts.push(make_context(episode_index, episode, start, end));
                    }
                    start = Some(index);
                    human_subjects.clear();
                }
                // A repeated locus marks the end of one local activity cycle.
                // This is occurrence-local: a shared terminal/browser/chat may
                // recur in every cycle without making the entire sitting one
                // context, while a single-surface work can repeat indefinitely
                // without being fragmented.
                if !contradicts_active && human_subjects.len() >= 2 && human_subjects.contains(subject) {
                    if let (Some(start), Some(end)) = (start, end) {
                        contexts.push(make_context(episode_index, episode, start, end));
                    }
                    start = Some(index);
                    human_subjects.clear();
                } else if start.is_none() {
                    start = Some(index);
                }
                human_subjects.insert(subject.to_string());
            }
            if start.is_some() {
                end = Some(index);
            }
        }
        if let (Some(start), Some(end)) = (start, end) {
            contexts.push(make_context(episode_index, episode, start, end));
        }
    }
    contexts
}

/// Forms candidate work bodies from occurrence-level contexts.
///
/// A context is a short run of activity, not a workspace. Contexts are merged
/// only when their evidence has a repeatable relationship: they share more than
/// a single ubiquitous surface, or a non-shared pair of resources is itself
/// strongly related. This keeps two interleaved works that happen to use the
/// same terminal/browser surface separate while allowing a work to recur with
/// different supporting artifacts. Components are deliberately built over
/// contexts, so an artifact may occur in several components and therefore have
/// overlapping membership in the resulting engagements.
fn context_seed_groups(hypotheses: &[crate::WorkHypothesis]) -> Vec<ContextGroup> {
    hypotheses
        .iter()
        .map(|hypothesis| ContextGroup {
            work_id: hypothesis.id(),
            contexts: hypothesis.contexts(),
            seeds: hypothesis.nucleus().clone(),
            evidence: hypothesis.links().to_vec(),
        })
        .collect()
}

fn make_context(episode_index: usize, episode: &Episode, start: usize, end: usize) -> ActivityContext {
    let acts: Vec<usize> = (start..=end).collect();
    let subjects = acts
        .iter()
        .filter_map(|index| episode.acts().get(*index))
        .map(|act| act.resource().subject().to_string())
        .collect();
    let human_subjects = acts
        .iter()
        .filter_map(|index| episode.acts().get(*index))
        .filter(|act| act.character().is_human_evidence())
        .map(|act| act.resource().subject().to_string())
        .collect();
    ActivityContext {
        episode: episode_index,
        started_at: acts.first().and_then(|index| episode.acts().get(*index)).map(Act::at),
        ended_at: acts.last().and_then(|index| episode.acts().get(*index)).map(Act::at),
        acts,
        subjects,
        human_subjects,
    }
}

impl ActivityContext {
    /// The sitting containing this context.
    pub fn episode(&self) -> usize { self.episode }

    /// Indices of the canonical acts in the containing Episode.
    pub fn acts(&self) -> &[usize] { &self.acts }

    /// Distinct resources observed in this context.
    pub fn subjects(&self) -> &BTreeSet<String> { &self.subjects }

    /// Resources a person was directly witnessed acting on in this context.
    pub fn human_subjects(&self) -> &BTreeSet<String> { &self.human_subjects }

    /// First witnessed moment in this context.
    pub fn started_at(&self) -> Option<SystemTime> { self.started_at }

    /// Last witnessed moment in this context.
    pub fn ended_at(&self) -> Option<SystemTime> { self.ended_at }
}

impl Engagement {
    /// Stable identity of the underlying body-of-work hypothesis.
    pub fn work_id(&self) -> crate::WorkId { self.work_id }

    /// Occurrence-level evidence attributed to this body of work.
    pub fn context_links(&self) -> &[crate::WorkContextLink] { &self.context_links }

    /// Non-exclusive artifact membership and independent restoration roles.
    pub fn artifact_links(&self) -> &[crate::WorkArtifactLink] { &self.artifact_links }

    /// Every resource in this body of work, most important first.
    pub fn participants(&self) -> &[Participant] {
        &self.participants
    }

    /// The resources Restoration should open: where the work happens, including
    /// where it left off.
    pub fn primary(&self) -> impl Iterator<Item = &Participant> {
        self.participants
            .iter()
            .filter(|participant| participant.role.opens_on_restore())
    }

    /// The resources Restoration should make available without opening.
    pub fn secondary(&self) -> impl Iterator<Item = &Participant> {
        self.participants
            .iter()
            .filter(|participant| !participant.role.opens_on_restore())
    }

    /// How much of this body of work Evo is prepared to claim.
    ///
    /// Callers must read this before presenting anything. A
    /// [`Standing::Remembered`] thread is a complete, honest record that Evo
    /// declines to call work: findable, listable, never on Home and never
    /// opened.
    pub fn standing(&self) -> Standing {
        self.standing
    }

    /// Whether this is a body of work at all, as opposed to something witnessed
    /// and kept.
    pub fn is_work(&self) -> bool {
        self.standing.is_work()
    }

    /// How much of this grouping's internal structure is relationships between its
    /// members, rather than each member's relationship to whatever founded it.
    ///
    /// The fraction of member-pairs whose affinity reaches
    /// [`EngagementParams::affinity_floor`]: one when every member was used with
    /// every other, near zero for a hub whose members are each beside the founding
    /// resource and nothing else. This is what separates a body of work from the
    /// pile of things that were on screen alongside an ever-present window; see
    /// [`EngagementParams::internal_cohesion`].
    pub fn cohesion(&self) -> f64 {
        self.cohesion
    }

    /// The loci this body of work was formed around: the places a person was
    /// witnessed working.
    ///
    /// In the order [`Engagement::participants`] uses, so the first is the
    /// most-attended locus. Empty for a thread formed out of resources no locus's
    /// evidence reached — which is exactly the case where Evo has no place to
    /// claim the work was centred.
    pub fn seeds(&self) -> impl Iterator<Item = &Participant> {
        self.participants
            .iter()
            .filter(|participant| participant.is_seed())
    }

    /// The single place the work should resume: the primary attended most
    /// recently.
    ///
    /// `None` only when no member of this body of work was ever attended, which
    /// the projection treats as having no continuation point rather than picking
    /// one arbitrarily.
    pub fn continuation(&self) -> Option<&Participant> {
        self.participants
            .iter()
            .find(|participant| participant.role == ResourceRole::Continuation)
    }

    /// Total attention across the whole body of work.
    pub fn attention(&self) -> Duration {
        self.attention
    }

    /// How many distinct sittings this work spanned.
    pub fn recurrence(&self) -> usize {
        self.episodes.len()
    }

    /// The sittings this body of work was going on in.
    ///
    /// Indices into the reconstruction's sittings, and the spine of the work's
    /// own history: activity, interruption, return. A sitting counts as this
    /// work's when more than one of its members was active in it — a member that
    /// merely happened to be around during someone else's sitting does not drag
    /// that sitting in.
    pub fn episodes(&self) -> &BTreeSet<usize> {
        &self.episodes
    }

    /// When this work was last witnessed.
    pub fn last_active(&self) -> Option<SystemTime> {
        self.last_active
    }

    /// When this work was first witnessed.
    pub fn first_active(&self) -> Option<SystemTime> {
        self.first_active
    }

    /// The canonical subjects of every participant.
    pub fn subjects(&self) -> BTreeSet<String> {
        self.participants
            .iter()
            .map(|participant| participant.subject().to_string())
            .collect()
    }

    /// The canonical subjects that define this work's identity.
    ///
    /// Only the primary resources: peripheral context comes and goes between
    /// sittings, but the places the work happens are what make it the same
    /// work when Evo restarts.
    pub fn signature(&self) -> BTreeSet<String> {
        let primary: BTreeSet<String> = self
            .primary()
            .map(|participant| participant.subject().to_string())
            .collect();
        if primary.is_empty() {
            self.subjects()
        } else {
            primary
        }
    }

    /// A short name for this body of work, taken from where the work continues.
    ///
    /// Always a name Evo actually witnessed. Never assembled from fragments,
    /// never invented, and never drawn from any knowledge of what applications or
    /// websites are — all three would be fabrication, and a title is the one place
    /// a fabrication is guaranteed to be read.
    ///
    /// Chosen by which member's name carries the most distinctive vocabulary the
    /// rest of this work also carries. That is what makes a name *about* the work
    /// rather than merely present in it, and it is why Home no longer leads with
    /// the most-attended window: an afternoon on a coursework assignment was
    /// titled "Spotify Premium" and an RFC being drafted was titled "WhatsApp",
    /// because attention was all the title had to go on.
    ///
    /// The candidates are the members Restoration would open — those where the
    /// work happens — rather than every member, because those are exactly the
    /// resources Evo has evidence were part of the work.
    ///
    /// See [`Engagement::titled_by_shared_vocabulary`] for whether the choice
    /// rested on that evidence or fell back to the leading participant.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Whether [`Engagement::title`] was chosen because that name shares
    /// distinctive vocabulary with the rest of the work.
    ///
    /// `false` means the members' names had nothing in common that Evo could
    /// measure, and the title is simply the name of where the work continues.
    /// That is a weaker claim and callers are entitled to know which they have.
    pub fn titled_by_shared_vocabulary(&self) -> bool {
        self.title_weight > 0.0
    }

    /// A grounded statement of what Evo actually witnessed here.
    pub fn explanation(&self) -> String {
        let attended = self
            .participants
            .iter()
            .filter(|participant| participant.attention > Duration::ZERO)
            .count();
        let minutes = self.attention.as_secs() / 60;
        let sittings = self.recurrence();
        format!(
            "{minutes} min of attention across {attended} resource{} in {sittings} sitting{}",
            if attended == 1 { "" } else { "s" },
            if sittings == 1 { "" } else { "s" }
        )
    }
}

/// Everything Evo reconstructed from a history: the bodies of work, and
/// everything else it witnessed and kept without claiming that much.
#[derive(Debug, Clone, Default)]
pub struct EngagementSet {
    /// Every reconstructed thread, sorted bodies of work first (see
    /// [`EngagementSet::build`]). The first `work_count` are the bodies of work;
    /// the rest are [`Standing::Remembered`]. Keeping them in one contiguous,
    /// sorted vector is what lets [`EngagementSet::work`] and
    /// [`EngagementSet::remembered`] hand back borrowed slices rather than
    /// re-filtering, and lets [`EngagementSet::engagements`] expose the whole
    /// record for retrieval without a second copy.
    engagements: Vec<Engagement>,
    /// How many leading entries of `engagements` are bodies of work. The sort in
    /// [`EngagementSet::build`] guarantees work precedes everything remembered,
    /// so this is the single split point between the two.
    work_count: usize,
    /// Resources no thread's evidence reached and which the history never
    /// witnessed a person at. See [`EngagementSet::unattended`].
    unattended: Vec<String>,
}

impl EngagementSet {
    /// Reconstructs bodies of work from a canonical history.
    ///
    /// # Authority change: membership is not a partition
    ///
    /// **The old rule.** `cluster()` ran connected components plus
    /// average-linkage agglomeration over the *whole* affinity graph and returned
    /// disjoint sets. Each set became at most one body of work, so every resource
    /// belonged to exactly one — by construction, before any evidence was
    /// consulted. Anything the significance test then refused was pushed into a
    /// list nothing read.
    ///
    /// **Why it prevented the product from working.** Measured on this machine's
    /// real Observation log (1245 observations, 425 resources, 61 sittings):
    ///
    /// - **0 of 425** resources were in more than one body of work, while
    ///   **152 of 425** had measurable affinity to two or more. Every
    ///   relationship but the strongest, per resource, was discarded to satisfy
    ///   the partition. A person holds several bodies of work at once and they
    ///   overlap; a partition cannot represent that, so the ambiguity the product
    ///   exists to preserve was destroyed before anything could reason about it.
    /// - **330** groups and **341** resources ended in the refused list, among
    ///   them 24 resources a person was witnessed acting on. A document read once
    ///   for thirty seconds did not survive as "kept but not promoted"; it left
    ///   the product.
    /// - Because the cluster was the unit, whatever held the most attention named
    ///   the work. An afternoon spent drafting an RFC was titled for a messaging
    ///   window; a coding session was titled for a search page.
    ///
    /// **The new rule.** Three separate questions, none of which forces a choice.
    ///
    /// 1. **Which resources can centre a body of work?** [`seeds`] — the ones a
    ///    person was witnessed working *in*. Clustering runs over those and
    ///    nothing else, so churn cannot form a body of work or inflate the
    ///    cohesion of one.
    /// 2. **Which resources belong to it?** Every resource whose strongest
    ///    affinity to one of those loci reaches [`EngagementParams::affinity_floor`]
    ///    — in *every* thread it reaches, with the strength recorded. No resource
    ///    is ever taken from one body of work to give to another.
    /// 3. **How much may a resource speak for it?** [`Participant::specificity`],
    ///    computed once every thread is known. This is what makes overlap safe:
    ///    a resource beside everything cannot name, lead, or reopen as any of
    ///    them, and it is still remembered in all of them.
    ///
    /// Nothing is discarded. Every group that forms becomes an `Engagement`
    /// carrying a [`Standing`], and anything with human evidence that no thread's
    /// evidence reached is grouped on its own terms rather than dropped.
    pub fn build(
        episodes: &[Episode],
        ledger: &AttentionLedger,
        graph: &AffinityGraph,
        declarations: &Declarations,
        params: &EngagementParams,
    ) -> Self {
        let contexts = build_activity_contexts(episodes, graph, params);
        let hypotheses = crate::hypothesis::infer_work_hypotheses(
            &contexts, graph, declarations, params, &crate::IdentityIndex::default(),
        );
        Self::build_with_hypotheses(
            episodes, &contexts, &hypotheses, ledger, graph, declarations, params,
        )
    }

    /// Reconstructs using an already-derived occurrence context index.
    ///
    /// Keeping this boundary explicit lets the top-level reconstruction retain
    /// the exact context evidence used for attribution, while the legacy
    /// `build` entry point remains source-compatible for callers that only have
    /// Episodes and the graph.
    pub fn build_with_contexts(
        episodes: &[Episode],
        contexts: &[ActivityContext],
        ledger: &AttentionLedger,
        graph: &AffinityGraph,
        declarations: &Declarations,
        params: &EngagementParams,
    ) -> Self {
        let hypotheses = crate::hypothesis::infer_work_hypotheses(
            contexts, graph, declarations, params, &crate::IdentityIndex::default(),
        );
        Self::build_with_hypotheses(
            episodes, contexts, &hypotheses, ledger, graph, declarations, params,
        )
    }

    /// Reconstructs Engagement projections from the authoritative hypotheses.
    pub fn build_with_hypotheses(
        episodes: &[Episode],
        contexts: &[ActivityContext],
        hypotheses: &[crate::WorkHypothesis],
        ledger: &AttentionLedger,
        graph: &AffinityGraph,
        declarations: &Declarations,
        params: &EngagementParams,
    ) -> Self {
        let mut founding = context_seed_groups(hypotheses);
        merge_declared_context_groups(&mut founding, declarations);
        if founding.is_empty() {
            // Histories with no local human context (for example old fixtures
            // containing only a deliberate act) retain the conservative legacy
            // fallback. Normal capture always takes the context-first path.
            founding = enforce_declarations(
                cluster_within(&seeds(ledger, graph, declarations, params), graph, params),
                declarations,
            )
            .into_iter()
            .map(|seeds| ContextGroup {
                work_id: crate::WorkId::from_nucleus(&seeds),
                contexts: BTreeSet::new(),
                seeds,
                evidence: Vec::new(),
            })
            .collect();
        }
        let mut threads = form_threads(founding, episodes, contexts, graph, params);

        // Nothing a person was witnessed at may end up in no thread at all. What
        // no locus's evidence reached is grouped on its own terms — that is where
        // the document read once for thirty seconds lives, and it is why refusing
        // to call something work no longer means losing it.
        let placed: BTreeSet<&String> = threads
            .iter()
            .flat_map(|thread| thread.members.keys())
            .collect();
        let mut stranded: BTreeSet<String> = BTreeSet::new();
        let mut unattended: Vec<String> = Vec::new();
        for subject in graph.subjects() {
            if placed.contains(subject) {
                continue;
            }
            if witnessed_a_person(ledger, subject) {
                stranded.insert(subject.clone());
            } else {
                unattended.push(subject.clone());
            }
        }
        for group in cluster_within(&stranded, graph, params) {
            threads.push(Thread::among(group, ledger, graph, params));
        }

        collapse_indistinguishable(&mut threads);
        measure_specificity(&mut threads);

        let mut engagements: Vec<Engagement> = threads
            .iter()
            .filter_map(|thread| assemble(thread, ledger, graph, declarations, params))
            .collect();

        // Bodies of work first, then most recently active: the thing you were
        // just doing is the thing you are most likely returning to. Standing
        // separates what Evo will present from what it merely kept; recency
        // orders within each, because a Restorable body of work from three days
        // ago is not more use than a Continuable one from an hour ago.
        engagements.sort_by(|left, right| {
            right
                .standing
                .is_work()
                .cmp(&left.standing.is_work())
                .then_with(|| right.last_active.cmp(&left.last_active))
                .then_with(|| right.attention.cmp(&left.attention))
                .then_with(|| left.title().cmp(right.title()))
        });
        let _ = episodes;

        // The sort above places every body of work before everything remembered,
        // so a single count is the split point the accessors slice at.
        let work_count = engagements
            .iter()
            .filter(|engagement| engagement.standing.is_work())
            .count();

        Self {
            engagements,
            work_count,
            unattended,
        }
    }

    /// Forms candidate work bodies from the attended loci, then associates each
    /// body with the occurrence contexts in which one of those loci was present.
    ///
    /// This boundary is intentionally non-transitive over contexts.  A context
    /// can contain several works and a shared artifact can occur in several
    /// contexts; neither fact is evidence that all contexts connected by one
    /// pairwise affinity are one body of work.  The seed graph is the narrower
    /// question: which places the person actually worked in are the same work?
    /// Everything reconstructed: bodies of work first, then what was witnessed
    /// and kept without that claim.
    ///
    /// Each carries its own [`Standing`], and callers are expected to read it.
    /// This is deliberately the whole record rather than the presentable part of
    /// it — Home shows [`EngagementSet::work`], and retrieval searches all of
    /// this, because "the thing I read once last Tuesday" is a real question and
    /// the answer to it is not a body of work.
    pub fn engagements(&self) -> &[Engagement] {
        &self.engagements
    }

    /// The bodies of work, most recently active first.
    ///
    /// This is the presentable part of the record — what Home shows. It is a
    /// borrowed prefix of [`EngagementSet::engagements`], since `build` sorts
    /// work first.
    pub fn work(&self) -> &[Engagement] {
        &self.engagements[..self.work_count]
    }

    /// What Evo witnessed and kept without claiming it was a body of work.
    ///
    /// The complement of [`EngagementSet::work`]: the suffix of
    /// [`EngagementSet::engagements`] carrying [`Standing::Remembered`].
    /// Retrievable by name, never presented on Home.
    pub fn remembered(&self) -> &[Engagement] {
        &self.engagements[self.work_count..]
    }

    /// Resources this history never witnessed a person at, and which no thread's
    /// evidence reached.
    ///
    /// Machine churn: files a tool rewrote beside someone's work, with no
    /// attention and no deliberate act ever witnessed on them. They are reported
    /// rather than silently dropped, and reported as exactly what the evidence
    /// says — witnessed, with no person in the record. The Observation log that
    /// produced them is append-only and untouched, and the resources remain in
    /// the affinity graph; what Evo declines to do is present a resource nobody
    /// ever touched as something a person might want back.
    ///
    /// This is not the old refused list. Nothing with human evidence can appear
    /// here: such resources are grouped into threads of their own and carry
    /// [`Standing::Remembered`].
    pub fn unattended(&self) -> &[String] {
        &self.unattended
    }

    /// How many bodies of work were reconstructed.
    pub fn len(&self) -> usize {
        self.work_count
    }

    /// Whether Evo has no body of work to offer.
    pub fn is_empty(&self) -> bool {
        self.work_count == 0
    }

    /// The body of work containing a subject, if any.
    ///
    /// A resource legitimately belongs to several, which is the point of the
    /// model; this returns the strongest claim on it — a body of work before
    /// something merely remembered, and the most recent among equals. See
    /// [`EngagementSet::every_containing`] for the complete answer.
    pub fn containing(&self, subject: &str) -> Option<&Engagement> {
        self.engagements.iter().find(|engagement| {
            engagement
                .participants
                .iter()
                .any(|participant| participant.subject() == subject)
        })
    }

    /// Every reconstructed thread this subject belongs to, strongest claim
    /// first.
    ///
    /// Overlap is normal and is not a defect to be resolved: an API reference
    /// consulted while doing two different things belongs to both, and Evo says
    /// so with the strength of each membership rather than picking one.
    pub fn every_containing(&self, subject: &str) -> Vec<&Engagement> {
        self.engagements
            .iter()
            .filter(|engagement| {
                engagement
                    .participants
                    .iter()
                    .any(|participant| participant.subject() == subject)
            })
            .collect()
    }
}

/// One body of work under construction: the loci it was formed around, the
/// sittings it was going on in, and every resource the evidence attaches to it.
#[derive(Debug, Clone)]
struct Thread {
    work_id: crate::WorkId,
    evidence: Vec<crate::WorkContextLink>,
    /// The attended loci this thread was formed around. Empty for a thread
    /// formed out of what no locus reached.
    seeds: BTreeSet<String>,
    /// The exact occurrence contexts attributed to this body of work.
    contexts: BTreeSet<usize>,
    /// The sittings this body of work was going on in. See [`sittings_of`].
    episodes: BTreeSet<usize>,
    /// Subject → [`Participant::strength`] in this thread.
    members: BTreeMap<String, f64>,
    /// The sittings in which each resource was actually attributed to this
    /// thread.  This is deliberately finer grained than `episodes`: one
    /// sitting may contain several interleaved bodies of work, so the whole
    /// sitting cannot be treated as evidence for every member of every thread.
    activity: BTreeMap<String, ThreadActivity>,
    /// Subject → [`Participant::specificity`]. Filled by [`measure_specificity`]
    /// once every thread is known, because it is a share across all of them.
    specificity: BTreeMap<String, f64>,
}

/// Work-specific evidence for one resource.
///
/// The global [`AttentionLedger`] answers what happened to a resource over its
/// whole life.  A body of work needs the narrower answer: what happened to that
/// resource on the occurrences attributed to this work.  Without this record a
/// shared terminal, inbox, or chat imports its attention and recency from every
/// other work into this one.
#[derive(Debug, Clone, Default)]
struct ThreadActivity {
    attention: Duration,
    per_episode: BTreeMap<usize, Duration>,
    episodes: BTreeSet<usize>,
    human_episodes: BTreeSet<usize>,
    human_acts: usize,
    contexts: BTreeSet<usize>,
    first_seen: Option<SystemTime>,
    last_seen: Option<SystemTime>,
}

impl ThreadActivity {
    fn record(
        &mut self,
        episode: usize,
        context: Option<usize>,
        act: &crate::episode::Act,
        attention: Duration,
    ) {
        self.episodes.insert(episode);
        if let Some(context) = context { self.contexts.insert(context); }
        if act.character().is_human_evidence() {
            self.human_acts += 1;
            self.human_episodes.insert(episode);
        }
        self.attention += attention;
        if attention > Duration::ZERO {
            *self.per_episode.entry(episode).or_insert(Duration::ZERO) += attention;
        }
        self.first_seen = earlier(self.first_seen, Some(act.at()));
        self.last_seen = later(self.last_seen, Some(act.at()));
    }

    fn deepest_sitting(&self) -> Duration {
        self.per_episode
            .values()
            .copied()
            .max()
            .unwrap_or(Duration::ZERO)
    }
}

impl Thread {
    /// Admits every resource the evidence attaches to one of these loci, and that
    /// the person was witnessed at while this work was going on.
    ///
    /// Strength is the **maximum** over the loci, not the mean. A body of work
    /// has several places it happens, and a reference consulted while working in
    /// one of them is legitimately related to that one and to nothing else here;
    /// a mean would report it as weakly attached to the whole, which misdescribes
    /// what was witnessed. A locus is maximally related to itself.
    ///
    /// **Authority change: admission must not chain.** Taking the maximum on its
    /// own is single linkage, which is the one thing this crate's clustering was
    /// chosen to avoid — and it reintroduced it one layer up. A window the person
    /// keeps open through every sitting can clear
    /// [`EngagementParams::cohesion_floor`] against a genuine locus and join its
    /// seed group; every unrelated thing that window ever sat beside then reaches
    /// the thread through it. Measured on the fixtures this crate already had, one
    /// afternoon of coursework acquired six unrelated errands, and two long
    /// sittings on one file acquired four later sittings of different work.
    ///
    /// Requiring the resource to have been *present while this work was going on*
    /// breaks the chain without appealing to anything but the record: a
    /// relationship that reaches a body of work only through a third party, and
    /// that was never witnessed inside it, is not evidence of taking part in it.
    fn around(
        group: ContextGroup,
        all_episodes: &[Episode],
        contexts: &[ActivityContext],
        graph: &AffinityGraph,
        params: &EngagementParams,
    ) -> Self {
        let ContextGroup {
            work_id,
            contexts: linked_contexts,
            seeds,
            evidence,
        } = group;
        let mut members: BTreeMap<String, f64> = BTreeMap::new();
        let mut activity: BTreeMap<String, ThreadActivity> = BTreeMap::new();
        let window = params.max_interval_attention.as_secs_f64().max(1.0);

        // Attribute occurrences, not whole sittings.  A sitting can contain
        // X → Y → X; the old model admitted every resource seen anywhere in
        // that sitting to both works.  Here an occurrence earns membership only
        // when it is temporally close to an occurrence of one of this thread's
        // seed loci and the pair has measured affinity.
        for (episode_index, episode) in all_episodes.iter().enumerate() {
            let episode_contexts: Vec<(usize, &ActivityContext)> = contexts
                .iter()
                .enumerate()
                .filter(|(index, context)| {
                    context.episode == episode_index
                        && (linked_contexts.is_empty() || linked_contexts.contains(index))
                })
                .collect();
            let ranges: Vec<(Option<usize>, Vec<usize>)> = if episode_contexts.is_empty() {
                // Once a sitting has local contexts, an absent seed means this
                // work was not active in that sitting. Falling back to the
                // entire sitting would reintroduce the old whole-session leak.
                if contexts.iter().any(|context| context.episode == episode_index) {
                    Vec::new()
                } else {
                    vec![(None, (0..episode.acts().len()).collect())]
                }
            } else {
                episode_contexts.iter().map(|(index, context)| (Some(*index), context.acts.clone())).collect()
            };

            for (context_index, range) in ranges {
                let seed_positions: Vec<(usize, &str)> = range
                    .iter()
                    .filter_map(|position| {
                        let act = episode.acts().get(*position)?;
                        seeds
                            .contains(act.resource().subject())
                            .then_some((*position, act.resource().subject()))
                    })
                    .collect();
                if seed_positions.is_empty() {
                    continue;
                }

                for position in &range {
                    let Some(act) = episode.acts().get(*position) else { continue };
                let subject = act.resource().subject();
                    let mut strength: f64 = if seeds.contains(subject) { 1.0 } else { 0.0 };
                    for (seed_position, seed) in &seed_positions {
                    if subject == *seed { continue; }
                    let Some(seed_act) = episode.acts().get(*seed_position) else { continue };
                    let gap = absolute_seconds_between(seed_act.at(), act.at());
                    if gap > window {
                        continue;
                    }
                    let proximity = 1.0 - gap / window;
                    let affinity = if subject == *seed {
                        1.0
                    } else {
                        graph.affinity(subject, seed, params)
                    };
                    let local_human = act.character().is_human_evidence()
                        && seed_act.character().is_human_evidence();
                    let local_support = if local_human {
                        // Direct human co-attention is occurrence evidence even
                        // when the canonical resource has no lifetime affinity
                        // to this seed. This is what lets a shared terminal,
                        // document, or conversation belong to several works;
                        // specificity later decides whether it can represent
                        // any one of them.
                        proximity
                    } else {
                        affinity * proximity
                    };
                    strength = strength.max(local_support);
                }
                if strength >= params.affinity_floor {
                    members
                        .entry(subject.to_string())
                        .and_modify(|held| *held = held.max(strength))
                        .or_insert(strength);
                    let attributed_attention = if act.character()
                        == crate::resource::ActCharacter::Attentional
                    {
                        episode
                            .acts()
                            .get(*position + 1)
                            .and_then(|next| next.at().duration_since(act.at()).ok())
                            .filter(|gap| *gap <= params.max_interval_attention)
                            .unwrap_or(Duration::ZERO)
                    } else {
                        Duration::ZERO
                    };
                    activity
                        .entry(subject.to_string())
                        .or_default()
                        .record(episode_index, context_index, act, attributed_attention);
                }
            }
            }
        }
        let episodes: BTreeSet<usize> = activity
            .values()
            .flat_map(|record| record.episodes.iter().copied())
            .collect();
        Self {
            work_id,
            evidence,
            seeds,
            contexts: linked_contexts,
            episodes,
            members,
            activity,
            specificity: BTreeMap::new(),
        }
    }

    /// A thread formed out of resources no locus's evidence reached.
    ///
    /// These have human evidence — Evo witnessed a person at them — but not
    /// enough of it to centre a body of work, and no measured relationship to
    /// anywhere that could. They are kept together as they were found, with the
    /// strength of whatever relationship they do have to each other.
    fn among(
        group: BTreeSet<String>,
        ledger: &AttentionLedger,
        graph: &AffinityGraph,
        params: &EngagementParams,
    ) -> Self {
        let mut members: BTreeMap<String, f64> = BTreeMap::new();
        for subject in &group {
            let strength = group
                .iter()
                .filter(|other| *other != subject)
                .map(|other| graph.affinity(subject, other, params))
                .fold(0.0_f64, f64::max);
            members.insert(subject.clone(), strength);
        }
        Self {
            work_id: crate::WorkId::from_nucleus(&group),
            evidence: Vec::new(),
            seeds: BTreeSet::new(),
            contexts: BTreeSet::new(),
            episodes: sittings_of(&group, ledger),
            members,
            activity: group
                .iter()
                .filter_map(|subject| {
                    ledger.get(subject).map(|record| {
                        (
                            subject.clone(),
                            ThreadActivity {
                                attention: record.attention,
                                per_episode: record.per_episode.clone(),
                                episodes: record.episodes.clone(),
                                human_episodes: record.human_episodes.clone(),
                                human_acts: record.human_acts,
                                contexts: BTreeSet::new(),
                                first_seen: record.first_seen,
                                last_seen: record.last_seen,
                            },
                        )
                    })
                })
                .collect(),
            specificity: BTreeMap::new(),
        }
    }

    /// The loci that survive being measured against the person's other work.
    ///
    /// See [`form_threads`] for why this exists. A locus whose specificity here is
    /// below [`EngagementParams::speaks_for_work`] was witnessed more beside the
    /// person's other bodies of work than beside this one, so it cannot be one of
    /// the places *this* work happens — whatever else it is.
    ///
    /// If that would leave no loci at all, the original ones are kept. A thread
    /// every one of whose loci is ubiquitous still records something that
    /// happened; dissolving it would be silent loss, and Evo has nothing better to
    /// prefer.
    fn refined_seeds(&self, params: &EngagementParams) -> BTreeSet<String> {
        let kept: BTreeSet<String> = self
            .seeds
            .iter()
            .filter(|seed| {
                self.specificity
                    .get(*seed)
                    .copied()
                    .is_some_and(|share| speaks_for(share, params))
            })
            .cloned()
            .collect();
        if kept.is_empty() {
            self.seeds.clone()
        } else {
            kept
        }
    }
}

/// The sittings a body of work was going on in: those in which **more than one**
/// of its loci was present.
///
/// **Authority change.** The rule used to be "more than one *member* was present,
/// and one of them was a locus". Membership overlaps now, so that let a resource
/// which is around during everything drag its own unrelated sittings in: the
/// sitting contained the ever-present window and one other thing, which satisfied
/// both halves. The work's attention, its recency, and every member's
/// [`Participant::locality`] were then computed over sittings the work had nothing
/// to do with — which is how a companion witnessed in four sittings of coursework
/// and six of errands measured a locality of 1.0 in the coursework.
///
/// Two of the *loci* — the places the person was witnessed working, not merely
/// things that were on screen — is a claim about the work rather than about one
/// resource's habits. It is the crate's one principle again: presence is not
/// evidence, co-presence is. A single place being open cannot distinguish "the
/// person was working on this" from "this is always open".
///
/// When no sitting has two loci in it — a body of work with one locus, or loci
/// the person never had open together — Evo falls back to every sitting its loci
/// were witnessed in. There is nothing else to appeal to, and claiming no sittings
/// at all would erase the work.
fn sittings_of(loci: &BTreeSet<String>, ledger: &AttentionLedger) -> BTreeSet<usize> {
    let mut present: BTreeMap<usize, usize> = BTreeMap::new();
    for locus in loci {
        if let Some(record) = ledger.get(locus) {
            for episode in &record.episodes {
                *present.entry(*episode).or_insert(0) += 1;
            }
        }
    }
    let together: BTreeSet<usize> = present
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(episode, _)| *episode)
        .collect();
    if together.is_empty() {
        present.into_keys().collect()
    } else {
        together
    }
}

/// Forms threads around seed groups, then re-forms them once the loci have been
/// measured against each other.
///
/// **Why two passes.** A locus is what a body of work is judged by — its sittings,
/// its identity, and whether it amounts to work at all are all read off the loci.
/// A resource that is beside all of the person's work can clear
/// [`EngagementParams::cohesion_floor`] against a genuine locus and become one,
/// and then it supplies the return and the depth that make *everything* it touches
/// look like work.
///
/// [`Participant::specificity`] is exactly the measurement that separates the two
/// cases, and it exists only once every thread is known. So: form threads, measure,
/// discard the loci that turned out to be beside everything, and form again. Two
/// passes, not a loop to a fixed point — Evo would rather state a rule with a bound
/// than iterate — and the second pass is skipped entirely when nothing was demoted,
/// which is the common case.
fn form_threads(
    founding: Vec<ContextGroup>,
    episodes: &[Episode],
    contexts: &[ActivityContext],
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> Vec<Thread> {
    let mut threads: Vec<Thread> = founding
        .iter()
        .cloned()
        .map(|group| Thread::around(group, episodes, contexts, graph, params))
        .collect();
    measure_specificity(&mut threads);

    let refined: Vec<ContextGroup> = threads
        .iter()
        .map(|thread| ContextGroup {
            work_id: thread.work_id,
            contexts: thread.contexts.clone(),
            seeds: thread.refined_seeds(params),
            evidence: thread.evidence.clone(),
        })
        .collect();
    if refined == founding {
        return threads;
    }
    refined
        .into_iter()
        .map(|seeds| Thread::around(seeds, episodes, contexts, graph, params))
        .collect()
}

fn merge_declared_context_groups(groups: &mut Vec<ContextGroup>, declarations: &Declarations) {
    for (first, second) in declarations.groupings() {
        let left = groups
            .iter()
            .position(|group| group.seeds.contains(first));
        let right = groups
            .iter()
            .position(|group| group.seeds.contains(second));
        let (Some(left), Some(right)) = (left, right) else { continue };
        if left == right { continue; }
        let absorbed = groups.remove(right);
        let survivor = if right < left { left - 1 } else { left };
        groups[survivor].contexts.extend(absorbed.contexts);
        groups[survivor].seeds.extend(absorbed.seeds);
        groups[survivor].evidence.extend(absorbed.evidence);
        groups[survivor].work_id = crate::WorkId::from_nucleus(&groups[survivor].seeds);
    }
}

/// Merges threads that no measurement can tell apart.
///
/// Two loci that stayed separate under [`EngagementParams::cohesion_floor`] can
/// still admit exactly the same resources, and then Evo has two records of one
/// body of work: two identities, two entries on Home, two restorations. Identical
/// membership is not "similar enough to merge" — it is the absence of any
/// measured difference, so keeping them apart would be asserting a distinction
/// the evidence does not contain.
fn collapse_indistinguishable(threads: &mut Vec<Thread>) {
    let mut collapsed: Vec<Thread> = Vec::new();
    for thread in threads.drain(..) {
        let twin = collapsed.iter_mut().find(|other| {
            other.work_id == thread.work_id
                && other.members.len() == thread.members.len()
                && other.members.keys().eq(thread.members.keys())
        });
        match twin {
            Some(existing) => {
                existing.seeds.extend(thread.seeds);
                existing.episodes.extend(thread.episodes);
                for (subject, strength) in thread.members {
                    if let Some(held) = existing.members.get_mut(&subject) {
                        *held = held.max(strength);
                    }
                }
            }
            None => collapsed.push(thread),
        }
    }
    *threads = collapsed;
}

/// Computes what share of each resource's observed work-local evidence falls in
/// each thread.
///
/// See [`Participant::specificity`]. This is the step that replaces the
/// partition's forced choice: instead of deleting a resource's weaker
/// relationships, Evo keeps them all and records how much each one is worth
/// relative to the others.
///
/// A global affinity only says that two canonical resources have been related at
/// some point in their lifetime. It must not be reused as the full strength of
/// every work-local claim: a file worked in for an hour in X and glanced at for
/// ten seconds while doing Y belongs to both, but it is not equally diagnostic
/// of both. Each claim is therefore weighted by the directly witnessed attention
/// in the attributed contexts, retaining a unit baseline for a genuine but
/// unmeasurable human occurrence (for example the final focused artifact before
/// the person stopped).
///
/// A resource claimed by exactly one thread scores `1.0`, and a resource with no
/// measured strength anywhere — a lone member of a thread of its own — divides
/// evenly among its claimants rather than being reported as belonging nowhere.
fn measure_specificity(threads: &mut [Thread]) {
    let mut across: BTreeMap<&String, (f64, usize)> = BTreeMap::new();
    for thread in threads.iter() {
        for (subject, strength) in &thread.members {
            let entry = across.entry(subject).or_insert((0.0, 0));
            entry.0 += work_local_claim_strength(thread, subject, *strength);
            entry.1 += 1;
        }
    }
    let across: BTreeMap<String, (f64, usize)> = across
        .into_iter()
        .map(|(subject, totals)| (subject.clone(), totals))
        .collect();

    for thread in threads.iter_mut() {
        thread.specificity = thread
            .members
            .iter()
            .map(|(subject, strength)| {
                let (total, claimants) = across
                    .get(subject)
                    .copied()
                    .unwrap_or((*strength, 1));
                let claim = work_local_claim_strength(thread, subject, *strength);
                let share = if total > 0.0 {
                    claim / total
                } else {
                    1.0 / claimants.max(1) as f64
                };
                (subject.clone(), share.clamp(0.0, 1.0))
            })
            .collect();
    }
}

/// The evidence mass behind one artifact-to-work claim.
///
/// `Thread::members` records the compatibility of the artifact with the work;
/// `ThreadActivity` records what actually happened in this work's contexts.
/// Both are required. The unit baseline means an observed human occurrence is
/// never erased merely because it ended a sitting and therefore has no measured
/// dwell interval.
fn work_local_claim_strength(thread: &Thread, subject: &str, compatibility: f64) -> f64 {
    let observed_attention = thread
        .activity
        .get(subject)
        .map(|activity| activity.attention.as_secs_f64())
        .unwrap_or(0.0);
    compatibility * (1.0 + observed_attention)
}

/// The resources that may *centre* a body of work: the ones a person was
/// witnessed working in.
///
/// Two witnessed facts, either of which is enough, and a statement by the person,
/// which overrides both:
///
/// - **They came back.** At least [`EngagementParams::min_revisits`] acts Evo
///   could attribute to a person. Return is the structural difference between
///   working somewhere and passing through it.
/// - **They stayed.** A sitting's worth of attention in one sitting
///   ([`EngagementParams::sustained_sitting_attention`]). Depth without return is
///   still a place someone worked, once.
///
/// Everything else can *join* a body of work but cannot define one. That is the
/// whole of the difference, and it is why churn is no longer a structural
/// problem: a file a tool rewrote beside real work has no attention and no act
/// Evo can attribute to a person, so it cannot found a thread, cannot inflate the
/// cohesion of one, and cannot pull attended work into a group that is then
/// refused. It is still admitted as a member wherever the evidence attaches it,
/// and reported as what it is.
///
/// Nothing here asks what the resource *is*. No application, domain, extension,
/// path or category appears in this test or anywhere beneath it — a lawyer's
/// contract, a producer's session file and a compiler engineer's source file
/// satisfy it identically, and a telemetry log fails it for the same reason in
/// every profession: nobody was ever witnessed there.
fn seeds(
    ledger: &AttentionLedger,
    graph: &AffinityGraph,
    declarations: &Declarations,
    params: &EngagementParams,
) -> BTreeSet<String> {
    graph
        .subjects()
        .filter(|subject| {
            if declarations.mentions(subject) {
                return true;
            }
            ledger.get(subject).is_some_and(|record| {
                record.human_acts >= params.min_revisits
                    || record.deepest_sitting() >= params.sustained_sitting_attention
            })
        })
        .cloned()
        .collect()
}

/// Whether the history ever witnessed a person at this resource.
///
/// Attention Evo could measure, or a deliberate act it could attribute to a
/// person. Not a judgement about importance — a single thirty-second read
/// satisfies it — only about whether there is a person in the record at all.
fn witnessed_a_person(ledger: &AttentionLedger, subject: &str) -> bool {
    ledger.get(subject).is_some_and(|record| {
        record.human_acts > 0 || record.attention > Duration::ZERO
    })
}

/// Whether a resource holding this share of its own relatedness may speak for the
/// body of work it holds it in.
///
/// One definition, used by [`Participant::speaks_for_work`] and by the
/// attribution decisions in [`assemble`] that have to be made before any
/// `Participant` exists.
fn speaks_for(specificity: f64, params: &EngagementParams) -> bool {
    specificity >= 1.0 || specificity >= params.speaks_for_work
}

/// How much of this body of work Evo is prepared to claim.
///
/// See [`Standing`]. Three claims, asked in order of how much they assert, and
/// the answer is always one of them — never "discard".
fn standing_of(
    engagement: &Engagement,
    ledger: &AttentionLedger,
    declarations: &Declarations,
    params: &EngagementParams,
) -> Standing {
    if !is_significant(engagement, ledger, declarations, params) {
        // Witnessed and kept, in full, with every relationship recorded. Evo
        // will not call it a body of work.
        return Standing::Remembered;
    }
    if engagement.continuation().is_some() {
        // A body of work with one unambiguous place it continues.
        return Standing::Restorable;
    }
    // A real body of work Evo can name and describe, with no single place it
    // resumes. Saying so is the honest answer; nominating the longest-attended
    // window present would be a coin-flip presented as a conclusion.
    Standing::Continuable
}

/// Whether a reconstructed body of work is something Evo can honestly present.
///
/// Four witnessed conditions, all necessary, and none of them about what the
/// resources *are* — no application, domain, file type, or category appears
/// here or anywhere beneath it.
///
/// 1. **Relationship.** At least two resources. A body of work is constituted
///    by things being used *together*; that is the whole content of the claim
///    "this is one piece of work". A lone resource carries no such evidence, has
///    no internal role structure to describe, and offers nothing to select from
///    when restoring — so Evo keeps it as something witnessed and says nothing
///    more.
/// 2. **Cohesion.** Those resources must, in the main, be related to one
///    *another* — not each to the single resource the thread formed around. This
///    is the same claim as the relationship condition asked of the whole group
///    rather than of a pair: a login screen or an inbox is present in the sittings
///    of everything, so the thread built around it admits everything on screen
///    beside it, and the result is a hub with spokes whose members relate to the
///    hub and to nothing else here. Measured as
///    [`Engagement::cohesion`] against [`EngagementParams::internal_cohesion`].
/// 3. **Return.** Some member was attended more than once. This is the
///    structural difference between working and passing through, and it is what
///    separates a body of work from a sweep: thirty things opened once each in
///    one afternoon produce no return, whatever their timing looks like.
/// 4. **Depth.** The group received real attention inside a single sitting. A
///    lifetime total will not do — a habit repeated forty times out-totals an
///    afternoon of work — so the question is whether there was ever *an
///    occasion* of working here.
///
/// Each alone admits something wrong. Return without depth is a habit: the same
/// three things touched for three seconds, over and over. Depth without return
/// is one window left focused while the person was elsewhere. And without the
/// relationship condition, any sufficiently-attended single window qualifies,
/// which is how a music player, a file-browser sidebar and a phone-mirroring
/// window each became a "body of work" on a real machine.
///
/// # Authority change: the conditions are asked of the work's own resources
///
/// **The old rule.** All three were asked of *any* member of the cluster.
///
/// **Why it prevented the product from working.** Under a partition that was
/// nearly harmless, because a cluster was small and every member had been
/// admitted by mutual cohesion. Membership now overlaps, so a thread legitimately
/// contains resources that are beside it rather than of it — and a messaging
/// window the person returns to all day would supply the return, and its
/// attention would supply the depth, for every body of work it was measurably
/// near. Every thread would become work, which is the failure this test exists to
/// prevent, arriving through a different door.
///
/// **The new rule.** The conditions are asked of the thread's **own** resources:
/// its loci ([`Participant::is_seed`] — places a person was witnessed working)
/// together with the members that hold a strict majority of their relatedness
/// here ([`Participant::speaks_for_work`]). A resource that is beside this work
/// and six others cannot make this work presentable, and it is still a full
/// member of all seven. Depth is still summed across those resources within a
/// single sitting, because a body of work is worked as a whole and the person
/// moving between its parts is one stretch of working.
///
/// The relationship condition has a cost, and it is worth stating rather than
/// hiding: a day spent entirely inside one document, touching nothing else, is
/// not presented as work. That case is genuinely indistinguishable — from
/// attention evidence alone — from a window that was simply left open and
/// returned to, because the two produce identical measurements. Evo declines to
/// guess, keeps the whole record as [`Standing::Remembered`], and the person can
/// always settle it by saying so: a designation makes the resource their work
/// regardless of what was measured, via the ground-truth short-circuit below.
fn is_significant(
    engagement: &Engagement,
    ledger: &AttentionLedger,
    declarations: &Declarations,
    params: &EngagementParams,
) -> bool {
    // The person's own statement is ground truth. If they said this is what
    // they are working on, Evo presents it — however little attention it
    // happened to witness, whether or not anything else was used with it, and
    // without arguing.
    if engagement
        .participants
        .iter()
        .any(|participant| declarations.mentions(participant.subject()))
    {
        return true;
    }

    let core: Vec<&Participant> = engagement
        .participants
        .iter()
        .filter(|participant| participant.is_seed() || participant.speaks_for_work(params))
        .collect();

    if core.len() < 2 {
        return false;
    }

    // The relationship condition, asked of the grouping as a whole: its members
    // must, in the main, be related to one *another*, not each to the resource the
    // thread formed around. A login screen, an inbox, a music player is present in
    // the sittings of everything, so the thread built around it admits everything
    // that was ever on screen beside it — a hub with spokes, whose member-pairs
    // are almost all unrelated. Evo keeps such a grouping in full and simply
    // declines to call it work. See [`EngagementParams::internal_cohesion`].
    if engagement.cohesion() < params.internal_cohesion {
        return false;
    }

    let returned = core.iter().any(|participant| {
        ledger
            .get(participant.subject())
            .is_some_and(|record| record.human_acts >= params.min_revisits)
    });
    if !returned {
        return false;
    }

    deepest_sitting(&core, ledger) >= params.sustained_sitting_attention
}

/// The attention these resources received in whichever single sitting they
/// received the most, summed across them.
///
/// Summing within a sitting is the right shape: a body of work is worked *as a
/// whole*, and the person moving between its parts is the same stretch of
/// working. Summing across *sittings* would not be — that is the
/// frequency-inflated total this function exists to avoid.
fn deepest_sitting(participants: &[&Participant], ledger: &AttentionLedger) -> Duration {
    let mut sittings: BTreeMap<usize, Duration> = BTreeMap::new();
    for participant in participants {
        let Some(record) = ledger.get(participant.subject()) else {
            continue;
        };
        for (episode, attention) in &record.per_episode {
            *sittings.entry(*episode).or_insert(Duration::ZERO) += *attention;
        }
    }
    sittings.into_values().max().unwrap_or(Duration::ZERO)
}

/// Groups related resources into candidate bodies of work.
/// Merges any clusters the person said belong together.
///
/// Measured affinity already treats a declaration as maximal, but clustering
/// could still separate a declared pair if the surrounding evidence pulled
/// hard the other way. This step removes that possibility: a statement by the
/// person is never overridden by a measurement, not even a confident one.
fn enforce_declarations(
    clusters: Vec<BTreeSet<String>>,
    declarations: &Declarations,
) -> Vec<BTreeSet<String>> {
    if declarations.is_empty() {
        return clusters;
    }

    let mut merged: Vec<Option<BTreeSet<String>>> = clusters.into_iter().map(Some).collect();

    for (first, second) in declarations.groupings() {
        let left = merged
            .iter()
            .position(|cluster| cluster.as_ref().is_some_and(|set| set.contains(first)));
        let right = merged
            .iter()
            .position(|cluster| cluster.as_ref().is_some_and(|set| set.contains(second)));
        match (left, right) {
            (Some(left), Some(right)) if left != right => {
                let absorbed = merged[right].take().expect("position implies presence");
                merged[left]
                    .as_mut()
                    .expect("position implies presence")
                    .extend(absorbed);
            }
            _ => {}
        }
    }

    merged.into_iter().flatten().collect()
}

/// Groups a *given set* of resources into candidate bodies of work.
///
/// Two stages. First, connected components over the *mutually-strongest* edges —
/// a tie founds a body of work only when each locus is among the other's
/// strongest few (see [`EngagementParams::reciprocal_rank`]). This is what stops
/// the one window everything happens beside from fusing every distinct activity
/// into a single component: its ties are moderate and many, the strongest of
/// none, so they are not mutual. Second, average-linkage agglomerative merging
/// inside each component, read over the *full* affinities, which can still
/// separate two distinct sub-groups that chained through a mutual edge but can
/// never rejoin what the first stage kept apart.
///
/// # Authority change: clustering runs over the loci, not over everything
///
/// **The old rule.** This ran over `graph.subjects()` — every resource Evo had
/// ever witnessed — and its disjoint output *was* the set of bodies of work.
///
/// **Why it prevented the product from working.** Two distinct failures from one
/// cause. Because everything was clustered, resources nobody ever touched shaped
/// the result: they inflated the cohesion of groups they joined and formed large
/// groups of their own, into which genuinely attended work was pulled and then
/// refused as insignificant. And because the output was disjoint, being placed in
/// one group *meant* being absent from every other, so a resource used across
/// three bodies of work had two of its three relationships deleted.
///
/// **The new rule.** Clustering answers one narrow question — *which attended
/// loci are the same body of work?* — and is given only [`seeds`] to answer it
/// with. Membership is a separate step ([`Thread::around`]) which does not
/// partition anything. Two loci that are not related as *groups* stay separate,
/// which is what keeps two tasks in one repository or one application distinct,
/// and churn can no longer influence the answer because it is not in the input.
pub(crate) fn cluster_within(
    subjects: &BTreeSet<String>,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> Vec<BTreeSet<String>> {
    let mut adjacency: BTreeMap<&String, BTreeMap<&String, f64>> = BTreeMap::new();
    for subject in subjects {
        adjacency.entry(subject).or_default();
    }
    for ((left, right), evidence) in graph.related_pairs(params) {
        if !subjects.contains(left) || !subjects.contains(right) {
            continue;
        }
        let score = evidence.score(params);
        adjacency.entry(left).or_default().insert(right, score);
        adjacency.entry(right).or_default().insert(left, score);
    }

    // Which relationships may *found* a body of work: only those that are, for
    // both loci, among their strongest. See [`EngagementParams::reciprocal_rank`].
    // A resource beside everything has a moderate tie to each thing it
    // accompanies and is the strongest tie of none; letting such ties form edges
    // freely fuses every distinct activity that shared one window into a single
    // component, because connectivity is transitive. Keeping only mutually-
    // strongest edges leaves that ever-present resource a component of its own —
    // remembered where it was present (membership is a separate step), but
    // founding nothing — while a locus and its own strongest company stay
    // together even when their tie is presence alone.
    let strongest = strongest_ties(&adjacency, params.reciprocal_rank);
    let mut reciprocal: BTreeMap<&String, BTreeSet<&String>> = BTreeMap::new();
    for subject in subjects {
        reciprocal.entry(subject).or_default();
    }
    for (subject, top) in &strongest {
        for neighbour in top {
            if strongest
                .get(neighbour)
                .is_some_and(|theirs| theirs.contains(subject))
            {
                reciprocal.entry(subject).or_default().insert(neighbour);
                reciprocal.entry(neighbour).or_default().insert(subject);
            }
        }
    }

    let mut clusters = Vec::new();
    let mut visited: BTreeSet<&String> = BTreeSet::new();

    for subject in subjects {
        if visited.contains(subject) {
            continue;
        }
        // Breadth-first over the mutually-strongest graph, in canonical order.
        let mut component: Vec<&String> = Vec::new();
        let mut queue: VecDeque<&String> = VecDeque::new();
        queue.push_back(subject);
        visited.insert(subject);
        while let Some(current) = queue.pop_front() {
            component.push(current);
            if let Some(neighbours) = reciprocal.get(current) {
                for neighbour in neighbours {
                    if visited.insert(neighbour) {
                        queue.push_back(neighbour);
                    }
                }
            }
        }
        component.sort();

        if component.len() == 1 {
            clusters.push(BTreeSet::from([component[0].clone()]));
            continue;
        }
        // Average-linkage still reads the *full* affinities within the
        // component: mutual-strongest decides which loci are one body of work,
        // then linkage may still separate two genuinely distinct sub-groups
        // that happened to chain through a mutual edge. It can only subdivide a
        // component, never join two, so no cross-activity fusion returns here.
        clusters.extend(average_linkage(&component, &adjacency, params));
    }

    clusters
}

/// For each subject, the `rank` neighbours it is most strongly related to.
///
/// Ranked by affinity, strongest first, ties broken by canonical subject order
/// so the kept set is fully determined by the evidence rather than by discovery
/// order. This is the raw material of the mutual-strongest test in
/// [`cluster_within`]: an edge founds a body of work only when each endpoint is
/// in the other's kept set.
fn strongest_ties<'g>(
    adjacency: &BTreeMap<&'g String, BTreeMap<&'g String, f64>>,
    rank: usize,
) -> BTreeMap<&'g String, BTreeSet<&'g String>> {
    let mut strongest: BTreeMap<&'g String, BTreeSet<&'g String>> = BTreeMap::new();
    for (subject, neighbours) in adjacency {
        let mut ranked: Vec<(&'g String, f64)> =
            neighbours.iter().map(|(name, score)| (*name, *score)).collect();
        ranked.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(right.0))
        });
        let kept: BTreeSet<&'g String> =
            ranked.into_iter().take(rank).map(|(name, _)| name).collect();
        strongest.insert(*subject, kept);
    }
    strongest
}

/// Average-linkage agglomerative merging within one connected component.
///
/// At each step the two groups with the highest *average* pairwise affinity
/// merge, and only while that average reaches the cohesion floor. Pairs with no
/// edge count as zero in the average, which is what stops a single bridging
/// edge from fusing two otherwise-unrelated groups.
fn average_linkage(
    component: &[&String],
    adjacency: &BTreeMap<&String, BTreeMap<&String, f64>>,
    params: &EngagementParams,
) -> Vec<BTreeSet<String>> {
    let size = component.len();
    let mut groups: Vec<Option<BTreeSet<String>>> = component
        .iter()
        .map(|subject| Some(BTreeSet::from([(*subject).clone()])))
        .collect();

    // Pairwise average affinity between groups; starts as the raw affinity.
    let mut average = vec![vec![0.0_f64; size]; size];
    for (left_index, left) in component.iter().enumerate() {
        for (right_index, right) in component.iter().enumerate() {
            if left_index == right_index {
                continue;
            }
            average[left_index][right_index] = adjacency
                .get(*left)
                .and_then(|neighbours| neighbours.get(*right))
                .copied()
                .unwrap_or(0.0);
        }
    }
    let mut counts: Vec<usize> = vec![1; size];

    loop {
        let mut best: Option<(usize, usize, f64)> = None;
        for left_index in 0..size {
            if groups[left_index].is_none() {
                continue;
            }
            for right_index in (left_index + 1)..size {
                if groups[right_index].is_none() {
                    continue;
                }
                let value = average[left_index][right_index];
                if value < params.cohesion_floor {
                    continue;
                }
                let better = match best {
                    None => true,
                    // Strictly greater only: ties resolve to the lower index,
                    // which is canonical subject order, so the result is fully
                    // determined by the evidence.
                    Some((_, _, best_value)) => value > best_value,
                };
                if better {
                    best = Some((left_index, right_index, value));
                }
            }
        }

        let Some((into, from, _)) = best else {
            break;
        };

        let absorbed = groups[from].take().expect("merge source is live");
        groups[into]
            .as_mut()
            .expect("merge target is live")
            .extend(absorbed);

        // Lance-Williams update for average linkage: the merged group's
        // affinity to any other group is the size-weighted mean of its parts'.
        let into_count = counts[into];
        let from_count = counts[from];
        let merged_count = into_count + from_count;
        for other in 0..size {
            if other == into || other == from || groups[other].is_none() {
                continue;
            }
            let merged = (into_count as f64 * average[into][other]
                + from_count as f64 * average[from][other])
                / merged_count as f64;
            average[into][other] = merged;
            average[other][into] = merged;
        }
        counts[into] = merged_count;
    }

    groups.into_iter().flatten().collect()
}

/// Builds the internal structure of one candidate body of work.
/// The fraction of a grouping's member-pairs that are related to each other, at
/// [`EngagementParams::affinity_floor`].
///
/// This is the plain question behind "is this one body of work": of all the ways
/// its members could be related to one another, how many are witnessed? A body of
/// work worked as a whole approaches one; a grouping held together only by a
/// resource every one of them happened to be beside — the star that forms around
/// an ever-present window — approaches zero, because its members relate to that
/// founding resource and not to each other.
///
/// Fewer than two members has no pair to measure and no relationship to claim, so
/// it is zero: a lone resource is never a body of work, and this reports it as
/// carrying none of the internal structure one would need.
fn internal_cohesion(
    members: &BTreeSet<String>,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> f64 {
    let mut pairs = 0usize;
    let mut related = 0usize;
    for (index, left) in members.iter().enumerate() {
        for right in members.iter().skip(index + 1) {
            pairs += 1;
            if graph.affinity(left, right, params) >= params.affinity_floor {
                related += 1;
            }
        }
    }
    if pairs == 0 {
        0.0
    } else {
        related as f64 / pairs as f64
    }
}

fn assemble(
    thread: &Thread,
    ledger: &AttentionLedger,
    graph: &AffinityGraph,
    declarations: &Declarations,
    params: &EngagementParams,
) -> Option<Engagement> {
    let members: BTreeSet<String> = thread.members.keys().cloned().collect();
    if members.is_empty() {
        return None;
    }

    // The sittings this body of work was actually going on in — decided when the
    // thread was formed, from its loci and nothing else. See [`sittings_of`].
    // Attention, recency and every member's locality are measured inside this set,
    // so a resource's activity in sittings this work was not going on in cannot
    // become this work's.
    let episodes = thread.episodes.clone();

    let mut total_attention = Duration::ZERO;
    let mut last_active: Option<SystemTime> = None;
    let mut first_active: Option<SystemTime> = None;

    for subject in &members {
        if let Some(record) = thread.activity.get(subject) {
            total_attention += record.attention;
            last_active = later(last_active, record.last_seen);
            first_active = earlier(first_active, record.first_seen);
        }
    }

    let mut participants: Vec<Participant> = Vec::new();

    // The members that are *of* this body of work rather than merely beside it:
    // ones a strict majority of whose measured relatedness is here. Membership now
    // overlaps, so "is a member" no longer answers "is part of this", and the
    // questions below — was the person elsewhere, is there activity to attribute a
    // change to — need the second answer, not the first.
    let own: BTreeSet<String> = members
        .iter()
        .filter(|subject| {
            thread
                .specificity
                .get(*subject)
                .is_some_and(|share| speaks_for(*share, params))
        })
        .cloned()
        .collect();

    // The sittings in which the person was demonstrably occupied with something
    // that is *not* part of this body of work.
    //
    // This is the only witnessed evidence that a resource has a life elsewhere,
    // and the distinction it draws is one [`Participant::locality`] cannot. A
    // resource witnessed in a sitting where this work was not going on has either
    // been somewhere else — the person was attending other things and this
    // resource was among them — or been *alone*, with nothing else attended at
    // all. The second is not evidence of a life elsewhere; it is the last thing
    // someone was looking at before they stopped, which is the single most
    // important moment this product exists to recover.
    //
    // Drawn from the whole ledger rather than from other bodies of work, because
    // the question is about the person's attention, not about Evo's grouping: a
    // resource Evo never grouped into anything still competes for the person's
    // attention. Only *attended* resources count — a file changing on its own is
    // not the person being elsewhere.
    // The members a person was demonstrably active in during this work: ones with
    // attention Evo could measure, or ones it witnessed a human act on. Only
    // these can attribute an otherwise unattributable change to this work — see
    // [`Participant::is_attention_corroborated`]. Drawn from the work's own
    // members: a person's activity in something merely beside this work is not
    // activity this work can attribute a file write to.
    let attended: BTreeSet<String> = own
        .iter()
        .filter(|subject| {
            ledger.get(subject.as_str()).is_some_and(|record| {
                attention_within(record, &episodes) > Duration::ZERO
                    || !record.human_episodes.is_disjoint(&episodes)
            })
        })
        .cloned()
        .collect();

    for subject in &members {
        let resource = graph.resource(subject)?.clone();
        let record = ledger.get(subject);
        let work_record = thread.activity.get(subject);
        let attention = work_record.map_or(Duration::ZERO, |record| record.attention);
        let locality = record.map_or(1.0, |record| {
            let total = record.episodes.len().max(1) as f64;
            let here = work_record.map_or(0, |work| work.episodes.len()) as f64;
            (here / total).clamp(0.0, 1.0)
        });
        // The sittings in which this resource *held* the person — where it drew a
        // sitting's worth of attention rather than merely being on screen. This is
        // the resource's witnessed life, and the set [`Participant::is_exclusive`]
        // asks its question of. Presence is not life: an editor tab glanced at on
        // the way past leaves a sighting, not a sitting spent somewhere.
        let dwelling: BTreeSet<usize> = record.map_or_else(BTreeSet::new, |record| {
            record
                .per_episode
                .iter()
                .filter(|(_, attention)| **attention >= params.sustained_sitting_attention)
                .map(|(sitting, _)| *sitting)
                .collect()
        });
        // The single deepest occasion of attention this resource held *inside*
        // this work. `dwelling` above asks its question of the resource's whole
        // witnessed life, because exclusivity is about where the resource lived;
        // this is confined to this work's own sittings, because leading this work
        // is a claim about attention spent here and a deep sitting elsewhere is
        // not evidence of it.
        let deepest_sitting_here = work_record
            .map(ThreadActivity::deepest_sitting)
            .unwrap_or(Duration::ZERO);
        participants.push(Participant {
            resource,
            // Provisional; assigned below once every participant is measured.
            role: ResourceRole::Context,
            attention,
            recurrence: work_record.map_or(0, |r| r.episodes.len()),
            last_seen: work_record.and_then(|r| r.last_seen),
            first_seen: work_record.and_then(|r| r.first_seen),
            tie: strongest_tie(subject, &members, graph, params),
            corroborated: is_corroborated_within(subject, &members, graph),
            locality,
            witnessed_sittings: record.map_or(0, |record| record.episodes.len()),
            attention_corroborated: is_corroborated_within(subject, &attended, graph),
            exclusive: work_record.is_some_and(|work| {
                dwelling.len() >= 2 && dwelling.is_subset(&work.episodes)
            }),
            belonging: belonging_within(subject, &members, graph, declarations, params),
            episodes: work_record
                .map_or_else(BTreeSet::new, |record| record.episodes.clone()),
            person_acted: work_record.is_some_and(|record| !record.human_episodes.is_empty()),
            strength: thread.members.get(subject).copied().unwrap_or(0.0),
            specificity: thread.specificity.get(subject).copied().unwrap_or(1.0),
            seed: thread.seeds.contains(subject),
            deepest_sitting_here,
        });
    }

    // Most attended first; ties broken canonically so the ordering — and
    // therefore which members reach the cap on primaries — is a function of the
    // evidence alone.
    //
    // Raw attention, not a share of the work's total: the two order identically
    // within one body of work, and the absolute quantity cannot change when the
    // work gains a member. Ordering by attention *discounted by locality* was what
    // this line did before; that discount belonged to the era when leadership
    // required exclusivity, and now that leadership is gated on attention-spent-here
    // and specificity it would only distort which of the eligible members the
    // primary cap falls on. Plain attention lets it fall on the ones worked in most.
    participants.sort_by(|left, right| {
        right
            .attention
            .cmp(&left.attention)
            .then_with(|| right.episodes.len().cmp(&left.episodes.len()))
            .then_with(|| right.recurrence.cmp(&left.recurrence))
            .then_with(|| left.subject().cmp(right.subject()))
    });

    assign_roles(&mut participants, declarations, params);
    let (title, title_weight) = choose_title(&participants, &members, graph, params);

    let artifact_links = participants
        .iter()
        .map(|participant| crate::WorkArtifactLink::new(
            thread.work_id,
            participant.subject().to_string(),
            participant.strength(),
            participant.specificity(),
            participant.role(),
            thread.activity
                .get(participant.subject())
                .map_or_else(BTreeSet::new, |activity| activity.contexts.clone()),
        ))
        .collect();

    let mut engagement = Engagement {
        work_id: thread.work_id,
        context_links: thread.evidence.clone(),
        artifact_links,
        participants,
        attention: total_attention,
        episodes,
        last_active,
        first_active,
        title,
        title_weight,
        standing: Standing::Remembered,
        cohesion: internal_cohesion(&members, graph, params),
    };
    engagement.standing = standing_of(&engagement, ledger, declarations, params);
    Some(engagement)
}

/// Chooses the name Home shows, and reports the evidence behind it.
///
/// See [`Engagement::title`]. Two things decide it, in this order.
///
/// **Only where the work continues may name it.** Shared vocabulary says a name is
/// *about* the same thing as the rest of the work; it does not say the name
/// matters. Letting any member compete titled a twenty-nine-minute coding session
/// `www.google.com/search`, because a search address and its results page have
/// every scrap of query machinery in common while a source file shares nothing
/// with anything. Restoration already decides which resources the work continues
/// in, and that is the set entitled to name it.
///
/// **Among those, the name carrying the most of the work's own distinctive
/// vocabulary wins.** Every primary has already earned its standing on attention,
/// so ranking them by vocabulary costs nothing and gains a great deal: it is what
/// turns "Spotify Premium" into "Job assignment analysis and deadline — Claude"
/// for an afternoon where both were open and only one was the work.
///
/// **When no name shares anything measurable, only a resource the evidence never
/// saw outside this work may name it.** This third rule exists because the second
/// one can measure nothing at all — on a real history it commonly does, since a
/// document's name and a terminal's name genuinely have no words in common — and
/// what the fallback is decides whether the whole mechanism holds or quietly
/// unwinds.
///
/// # Authority change: absent subject evidence is not licence to fall back to attention
///
/// **The old rule.** Primaries arrive ordered by leading share, so when no name
/// shared any vocabulary the order was left untouched and the title became
/// whichever resource held the largest share of this work's attention.
///
/// **Why it prevented the product from working.** Attention share answers *where
/// was the person*, which is a claim about position; a title answers *what is this
/// work*, which is a claim about subject. Substituting the first for the second
/// presents a measurement as a meaning, and the measurement is exactly the one
/// that produced the failure this rewrite exists to end. On the machine this was
/// written against it titled an afternoon of drafting RFC-0011 "‎WhatsApp": the
/// messaging app held 8 of the work's 15 minutes across four returns, three times
/// the attention the document received, and shared no vocabulary with anything —
/// so the vocabulary rule measured nothing, the guard fell away, and the old
/// behaviour reappeared underneath it. A semantic mechanism that reverts to the
/// signal it was built to overrule has not been added; it has been decorated.
///
/// **The new rule.** With no subject evidence to rank by, the name is taken from
/// the resources the evidence places *wholly inside* this work — the ones whose
/// [`Participant::locality`] is whole, meaning every sitting they were ever
/// witnessed in was a sitting of this work — highest leading share first. This is
/// not a threshold on a continuum and introduces no number: whole locality versus
/// anything less is the difference between "never seen outside this work" and
/// "seen outside this work", which is a difference in kind. It is the crate's one
/// principle — something that correlates with everything correlates with nothing —
/// applied to naming: a resource the person also uses elsewhere is not what *this*
/// work is called, however much of this work's time it happened to hold. The
/// messaging app's locality was 0.75; the document's and the project window's were
/// whole, and the document led among them.
///
/// A positive vocabulary measurement is never overridden by this rule. Locality
/// decides only the case where there was nothing to rank — it is a tie-break in
/// the absence of evidence, not a competing signal. And if no opener is wholly
/// within the work, the ranking stands as it was: a body of work Evo chose to
/// present is never left nameless (§11), and reporting
/// [`Engagement::titled_by_shared_vocabulary`] is what keeps the weaker case
/// legible instead of dressed up (§14).
///
/// # Authority change: only a resource that belongs here may name it
///
/// **The old rule.** Every opener competed, and the absent-evidence fallback
/// preferred whole [`Participant::locality`].
///
/// **Why it was not enough.** Locality is a share of *sittings*, and it cannot
/// separate a resource that is beside several bodies of work at once from one that
/// is part of this one, because both are witnessed in the same sittings. Measured
/// on the real log, a messaging window reached locality `0.75` and a music player
/// `0.86` — high enough to survive, low enough to prove nothing — and both went on
/// to name work they had nothing to do with.
///
/// **The new rule.** Only members that [`Participant::speaks_for_work`] may
/// compete: a strict majority of everything the resource is related to has to be
/// here. That is a share across the person's *other work* rather than across the
/// clock, which is the question a title actually turns on. When nothing measurable
/// is shared, the fallback ranks by specificity and then locality — both
/// witnessed, both about belonging, neither about position. A resource beside
/// everything is excluded from naming without any list of what it is, and the same
/// resource inside work genuinely about it has all of its relatedness here and
/// names it.
fn choose_title(
    participants: &[Participant],
    members: &BTreeSet<String>,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> (String, f64) {
    let usable =
        |participant: &&Participant| !participant.resource().display_name().trim().is_empty();

    // Only where the work continues may name it, and only if it belongs here.
    let openers: Vec<&Participant> = participants
        .iter()
        .filter(|participant| participant.role.opens_on_restore())
        .filter(|participant| participant.speaks_for_work(params))
        .filter(usable)
        .collect();

    let mut best: Option<(&Participant, f64)> = None;
    for participant in &openers {
        let shared = graph.shared_vocabulary(participant.subject(), members);
        if best.is_none_or(|(_, incumbent)| shared > incumbent) {
            best = Some((participant, shared));
        }
    }

    // Nothing measurable was shared, so there is no subject evidence to rank by
    // and leading share must not stand in for it. Among the openers, prefer the
    // one the evidence places most firmly inside this work.
    if best.is_some_and(|(_, shared)| shared <= 0.0) {
        let mut belongs: Option<&Participant> = None;
        for participant in &openers {
            let better = match belongs {
                None => true,
                Some(incumbent) => {
                    (participant.specificity, participant.locality)
                        > (incumbent.specificity, incumbent.locality)
                }
            };
            if better {
                belongs = Some(participant);
            }
        }
        if let Some(participant) = belongs {
            best = Some((participant, 0.0));
        }
    }

    // No opener could name it. Fall back through the members that belong here,
    // then through every participant, so a body of work Evo chose to present is
    // never nameless.
    let best = best
        .or_else(|| {
            participants
                .iter()
                .filter(|participant| participant.speaks_for_work(params))
                .find(usable)
                .map(|participant| (participant, 0.0))
        })
        .or_else(|| {
            participants
                .iter()
                .find(usable)
                .map(|participant| (participant, 0.0))
        });

    match best {
        Some((participant, shared)) => {
            (participant.resource().display_name().to_string(), shared)
        }
        None => ("Untitled work".to_string(), 0.0),
    }
}

/// Assigns each participant its role from the evidence tying it to *this* body
/// of work.
///
/// The cap on primaries is the whole point of this layer: a body of work with
/// sixteen resources still has at most a few places the work actually
/// continues, and restoring the rest would recreate a screenful of windows
/// instead of putting the person back into the work.
///
/// # Authority change: leadership is a witnessed fact about a resource, not a
/// share of a total
///
/// **The old rule.** A participant led its work when
/// `leading_share >= primary_attention_share`, where `leading_share` was the
/// resource's share of the work's total attention discounted by its locality.
///
/// **Why it prevented the product from working.** Two failures, both measured on
/// this machine's real Observation log rather than argued from principle.
///
/// 1. *Importance was decided by workspace size.* A share is a quantity divided
///    by a total that grows with every member, so the same twenty-nine minutes in
///    the same file cleared the bar in a five-member body of work and missed it in
///    a twenty-nine-member one. Measured: restoration opened 3 of 3 members in one
///    workspace and 1 of 29 in another, and the difference was arithmetic, not
///    evidence. A parameter is admissible here only if it expresses a limit of
///    what Evo can honestly claim to have witnessed
///    ([`EngagementParams`]); a share of a growing total expresses a preference
///    about how large a body of work ought to be.
/// 2. *A discount is not a disqualification.* Locality entered only as a
///    multiplier, so a resource with a life outside this work kept most of its
///    standing. Measured: a messaging window with locality `0.75` led the body of
///    work in which an RFC was being written, and a music player with locality
///    `0.86` led the one in which a job assignment was being analysed — in both
///    cases ranking above the document the work was about.
///
/// **The new rule.** Three questions, each answered by a witnessed fact about the
/// resource alone, and all three necessary:
///
/// - **Was it attended inside this work at all** — `attention > 0` measured
///   within this work's sittings. Absolute, so it cannot move when the work gains
///   a member.
/// - **Is it exclusive to this work** — [`Participant::is_exclusive`]. Evo never
///   witnessed this resource anywhere but this work: every sitting it appeared in
///   was either a sitting of this work or a sitting in which the person attended
///   nothing else at all. Not a threshold and not a discount: if Evo saw the
///   resource living a life elsewhere, then whatever it is, it is not what *this*
///   work is about. A resource witnessed in a single sitting cannot make the claim
///   at all, because Evo never had an occasion to see it contradicted.
/// - **Was it returned to** — [`Participant::returned`]. It was active in at
///   least [`EngagementParams::min_revisits`] of this work's own sittings.
///   Something the person went back to is something they had not finished with;
///   something witnessed in a single unbroken sitting and never again may have
///   been finished, abandoned, or a one-off, and Evo cannot tell which.
///
/// This is why the roles are *contextual* rather than global. Nothing here knows
/// what a music player, a messaging window or a meeting is. A music player used
/// all day across every body of work fails exclusivity in each of them — and the
/// same music player, in a body of work *about* music, is witnessed only in that
/// work's sittings and returned to across them, so it leads. The evidence is about
/// this body of work, so the answer can differ between two bodies of work
/// containing the same resource, which is what the product needs and what a
/// global rule cannot express.
///
/// It is also what makes silence reachable. A body of work every one of whose
/// members was witnessed in a single sitting has no member the person returned
/// to, so it has no primary, no continuation, and no resume point — and Evo says
/// so rather than nominating the longest-attended thing present. Measured on the
/// real log: five of nine bodies of work are in exactly that position, among them
/// a finished video call that the old rule offered as a place to carry on.
///
/// Deliberately **not** used here: whether a member's belonging is corroborated
/// by shared wording or location (`Participant::is_corroborated`). Gating
/// leadership on that was tried and was wrong — it demoted twenty-nine minutes
/// in a source file to a supporting role and let a thirty-four-second search page
/// lead the session, because the search page shared its query text with a result
/// window while the real work shared no vocabulary with anything. A resource can
/// be the entire point of a task and still have nothing in common with the rest
/// of it but the person's attention.
///
/// # Authority change: measured attention decides leadership, not participation
///
/// **The old rule.** A participant with no measured attention was
/// [`ResourceRole::Context`] — "present, but never attended. Evo will not claim
/// it was worked in."
///
/// **Why it prevented the product from working.** Attention accrues only to
/// **attentional** acts, and only across the interval to the next act in the same
/// sitting ([`AttentionLedger::build`]). A change carries no interval, so a
/// resource that is only ever *changed* has exactly zero attention however often
/// it is changed. The rule therefore reported the document a person saved four
/// times a sitting across two sittings as merely present — and
/// [`ResourceRole::Context`] is excluded from the restoration Context Chain, so the
/// file the work was about was not among the context Evo offered when resuming it.
/// That is §4's two ends collapsed into one: *merely encountered* and *interacted
/// with as part of something* were both answered "not worked in".
///
/// **The new rule.** Zero attention removes a resource from *leadership*, not
/// automatically from *participation*, and the two questions are now asked
/// separately. For a participant Evo measured no attention in:
///
/// - A person was witnessed acting on it during this work
///   ([`Participant::person_acted`]) — Evo simply could not measure how long:
///   [`ResourceRole::Reference`].
/// - Otherwise all Evo saw was a change it cannot attribute to anyone, and the
///   question becomes whether the changes happen *because* this work is
///   happening. A draft changes when, and only when, the person works on it; a
///   cache changes all day regardless. That is [`Participant::locality`], against
///   [`EngagementParams::attributable_change_locality`] — a strict majority, so
///   the resource's witnessed life must be more inside this work than outside it
///   — **and** that locality must be measurable at all
///   ([`Participant::locality_is_evidence`]), **and** the change must be
///   corroborated by a member the person was actually active in
///   ([`Participant::is_attention_corroborated`]). Above it, the work is what
///   changed the resource: [`ResourceRole::Supporting`]. Below it, or with no
///   locality evidence either way, or with nothing but other untouched changes to
///   vouch for it, Evo says the resource was present and claims nothing further:
///   [`ResourceRole::Context`], which keeps its meaning and stays reachable.
///
/// **Amendment: locality must be measurable before it can promote.** The rule
/// above originally read locality alone. Validated against a real Observation log
/// it admitted pure machine churn as [`ResourceRole::Supporting`] — several
/// `…/event-log-data/logs/FUS/<uuid>-2025.2.0.20-eap.log` files and a photo
/// library's internal caches appeared as participants in a person's work, with
/// zero attention and locality `1.00`. The reasoning was locally valid and the
/// measurement was wrong: a share computed over one sitting is `1.0` whatever the
/// resource is, so locality was reporting the absence of contrary evidence as the
/// presence of supporting evidence. It is now required to be informative before it
/// may promote, which costs nothing for a genuine draft — a resource the work keeps
/// returning to is witnessed in several sittings by definition.
///
/// **Amendment: a change must be corroborated by attention, not by other
/// changes.** The amendment above was necessary and, measured against the same
/// real log, not sufficient. It was written believing the ephemeral logs were
/// "written once under a freshly generated name and never seen again, so they are
/// witnessed in exactly one sitting". Re-derived, that was true of the photo
/// library's caches (one witnessed sitting each, correctly demoted) and **false**
/// of the JetBrains logs: each was witnessed in *two* sittings, cleared
/// `locality_is_evidence`, and stayed [`ResourceRole::Supporting`] in a person's
/// body of work with zero attention. The claim is corrected here rather than left
/// standing.
///
/// **Why it prevented the product from working.** Home's sidebar is the answer to
/// "what did this work consist of", so a telemetry log listed there as something
/// the work was *used with* is §1's failure — a resource surfaced as work on the
/// strength of having been written nearby. The reason it survived is that these
/// resources corroborate *each other*: a tool writing four similarly-named files
/// per run gives each of them a large shared vocabulary with the other three, so
/// [`Participant::is_corroborated`] is satisfied entirely within a set the person
/// never touched. Corroboration among all members is the right test for
/// membership and the wrong one for attributing a change.
///
/// **The new rule.** A change may promote to [`ResourceRole::Supporting`] only if
/// something the person was demonstrably active in corroborates it — attention, or
/// a witnessed human act, on a member sharing its wording, location or
/// declaration. Locality and attention-corroboration are independent and both
/// necessary: locality refuses churn that is *co-located* with the work but runs
/// regardless of it, and attention-corroboration refuses churn that is merely
/// *coincident* with it. Neither names an application, a path, a domain or an
/// extension, and neither adds a parameter.
///
/// This is the crate's stated position on incidental acts — they "become evidence
/// only when they are *selectively* associated with attention"
/// ([`crate::resource::ActCharacter::Incidental`]) — applied where it belongs,
/// with no path list, application list or frequency cutoff, and with the same
/// principle the rest of the layer runs on.
///
/// Machine churn takes two different forms. A resource *rewritten all day*
/// correlates with everything, so it correlates with nothing, so clustering does
/// not admit it as a member. A resource written a handful of times under names
/// never used again is the opposite case: it correlates with almost nothing,
/// which makes what little it does correlate with look distinctive, so clustering
/// may well admit it.
///
/// This function used to be the only place either form was refused. That was
/// wrong, and the real log is what showed it: churn admitted at membership does
/// not merely ride along as [`ResourceRole::Context`] — it inflates the cohesion
/// of the clusters it joins and forms very large clusters of its own, into which
/// genuinely attended work gets pulled and then refused as insignificant. A
/// cluster refused for insignificance never reaches this function at all, so no
/// role assignment can undo the damage. Churn is now also refused upstream, where
/// it originates, by measuring co-presence only across pairs of acts with a
/// person on at least one end
/// ([`crate::affinity::measure_interleaving`](../affinity/fn.measure_interleaving.html)).
///
/// The two are not redundant. Upstream refusal removes members that were never
/// tied to a person at all. The amendments here judge changes that *are* tied to
/// a person's activity but cannot be attributed to the person — a file written
/// beside real work — and decide whether the work is what changes it. Evo still
/// says such resources were present, because they were: [`ResourceRole::Context`]
/// is an honest answer and remains reachable.
///
/// Corroboration against *all* members ([`Participant::is_corroborated`]) was
/// tried here and is wrong for the same reason it is wrong for leadership:
/// distinctive shared vocabulary is measured in absolute IDF nats, so a token
/// every member shares carries no information at all. A spreadsheet, its file and
/// its terminal, all named for the same forecast, corroborate nothing — and the
/// file at the centre of the work would have been reported as merely present.
/// [`Participant::is_attention_corroborated`] is a different question: not "is
/// this member distinctive within the group" but "is there a person's activity for
/// this change to be attributed to at all".
fn assign_roles(
    participants: &mut [Participant],
    declarations: &Declarations,
    params: &EngagementParams,
) {
    let mut primaries = 0usize;
    for participant in participants.iter_mut() {
        // A resource the person named as their work, or as where they
        // continue, is primary. Measurement does not get to overrule that.
        if declarations.is_continuation(participant.subject())
            || declarations.is_designated(participant.subject())
        {
            participant.role = ResourceRole::Primary;
            primaries += 1;
            continue;
        }
        if participant.episodes.is_empty() {
            // Nothing about this resource happened while this work was going
            // on. It was around. Evo will not claim anything further.
            participant.role = ResourceRole::Context;
            continue;
        }
        if participant.attention == Duration::ZERO {
            // Something was witnessed here, but not attention Evo could measure —
            // a save and a commit are instants, and the last act of a sitting
            // accrues nothing. This is not a place to reopen. Whether it took
            // part at all is settled by the only evidence it has.
            participant.role = if participant.person_acted {
                // A person did act here; Evo simply could not measure how long.
                ResourceRole::Reference
            } else if participant.locality_is_evidence()
                && participant.locality >= params.attributable_change_locality
                && participant.is_attention_corroborated()
            {
                // Only unattributable changes — but they happen when, and mostly
                // only when, this work is going on, and they happen somewhere the
                // person was demonstrably active. That is the work changing it.
                ResourceRole::Supporting
            } else {
                // Changes that happen regardless of this work; a resource
                // witnessed in too few sittings for locality to say either way;
                // or changes with nothing but each other to vouch for them.
                // Evo says it was present and claims nothing further.
                ResourceRole::Context
            };
            continue;
        }
        if primaries < params.max_primary
            && participant.engaged(params)
            && participant.speaks_for_work(params)
            && (participant.is_exclusive()
                || (participant.locality() >= params.speaks_for_work
                    && participant.returned(params)))
        {
            // Worked in during this work — more than a glance in at least one of
            // its sittings — and a strict majority of everything it is related to
            // is here. That is a place this work lives, and one Evo can reopen.
            participant.role = ResourceRole::Primary;
            primaries += 1;
        } else if participant.is_exclusive() || participant.returned(params) {
            // Really used here — either it belongs to this work alone, or the
            // person kept coming back to it — but not both, so Evo will not
            // reopen it. Or it is used across several bodies of work, in which
            // case reopening it as *this* one would be a guess.
            participant.role = ResourceRole::Supporting;
        } else {
            // Attended during this work, with a life mostly elsewhere and no
            // return: a passing glance, kept and recoverable, opened by nothing.
            participant.role = ResourceRole::Reference;
        }
    }

    name_the_continuation(participants, params);
}

/// Promotes the primary the person attended most recently to
/// [`ResourceRole::Continuation`], when the evidence names one.
///
/// Restoration has to name exactly one Resume Point, and a body of work
/// legitimately has several places the work happens. Something has to choose, and
/// the choice belongs here rather than in the Restoration layer, because this is
/// the layer holding the evidence: when each member was last attended. Restoration
/// sees Artifacts and Attachments, and picking among those would mean ranking by
/// confidence, identifier, or attachment order — none of which says anything about
/// where a person stopped.
///
/// "Most recently attended" is a witnessed fact, not a threshold: the last thing
/// the person was looking at when the work broke off is where they left off.
///
/// # Authority change: a tie is answered with silence, not with an ordering
///
/// **The old rule.** Ties in `last_seen` broke on canonical subject, descending;
/// and if nothing had cleared the primary share, the best-placed attended
/// participant was promoted to primary first so that this function always had a
/// candidate. Every body of work therefore ended with exactly one continuation.
///
/// **Why it prevented the product from working.** Both halves manufacture
/// certainty Evo does not have, and the Restoration layer has no way to tell
/// manufactured certainty from the real thing: because exactly one member always
/// carried [`ResourceRole::Continuation`], `derive_restoration_plan` always found
/// a sole candidate, so every plan was complete and every plan carried a next
/// step. Measured on this machine's real log: 9 of 9 bodies of work complete, 9 of
/// 9 with a next step, 0 insufficient. The
/// [`crate::engagement`] layer was the reason Restoration could never be silent —
/// not the Restoration layer, whose insufficient outcomes were correct and
/// unreachable.
///
/// Which of two names sorts later is not evidence about where a person stopped. A
/// coin-flip presented as a resume point is worse than no resume point, because
/// the person cannot tell the two apart.
///
/// **The new rule.** The continuation is the primary with the *strictly* latest
/// `last_seen`. If two or more primaries share that latest moment, Evo has no
/// witnessed reason to prefer either and names no continuation; the body of work
/// keeps its primaries, and Restoration reports that it cannot determine where the
/// work resumes. A work whose primaries were never attended, or which has no
/// primaries at all, likewise keeps no continuation.
fn name_the_continuation(participants: &mut [Participant], params: &EngagementParams) {
    let latest = participants
        .iter()
        .filter(|participant| participant.role == ResourceRole::Primary)
        .filter(|participant| {
            participant.deepest_sitting_here >= params.sustained_sitting_attention
        })
        .filter_map(|participant| participant.last_seen)
        .max();

    let Some(latest) = latest else { return };

    let mut candidates = participants
        .iter()
        .enumerate()
        .filter(|(_, participant)| participant.role == ResourceRole::Primary)
        .filter(|(_, participant)| {
            participant.deepest_sitting_here >= params.sustained_sitting_attention
        })
        .filter(|(_, participant)| participant.last_seen == Some(latest));

    let Some((index, _)) = candidates.next() else {
        return;
    };
    if candidates.next().is_some() {
        // Two places the work lives, both last attended at the same witnessed
        // moment. Nothing measured says which one the work resumes in.
        return;
    }
    participants[index].role = ResourceRole::Continuation;
}

/// What share of `record`'s witnessed life coincided with this work's sittings.
///
/// See [`Participant::locality`]. A resource with no measured sittings has nothing
/// to divide, so it is reported as fully local rather than as suspiciously
/// ubiquitous.
fn locality_of(record: &AttentionRecord, episodes: &BTreeSet<usize>) -> f64 {
    if record.episodes.is_empty() {
        return 1.0;
    }
    record.episodes.intersection(episodes).count() as f64 / record.episodes.len() as f64
}

/// The attention `record` accrued during this work's sittings.
fn attention_within(record: &AttentionRecord, episodes: &BTreeSet<usize>) -> Duration {
    episodes
        .iter()
        .map(|episode| record.attention_in(*episode))
        .sum()
}

/// How strongly the evidence ties one member to the rest of its group.
///
/// See [`Participant::belonging`]. A member the person grouped by hand belongs
/// completely and is reported as such — the statement is not an average of
/// anything, and averaging it with silence would weaken a claim Evo has no
/// standing to weaken.
///
/// Pairs with no edge at all count as zero, exactly as they do in the clustering
/// that admitted this member. Anything else would report a group as more
/// cohesive than the mechanism that formed it believed it to be.
fn belonging_within(
    subject: &str,
    members: &BTreeSet<String>,
    graph: &AffinityGraph,
    declarations: &Declarations,
    params: &EngagementParams,
) -> f64 {
    if declarations.mentions(subject) {
        return 1.0;
    }
    let mut total = 0.0;
    let mut others = 0usize;
    for other in members {
        if other == subject {
            continue;
        }
        others += 1;
        if declarations.is_grouped(subject, other) {
            total += 1.0;
            continue;
        }
        if let Some(evidence) = graph.evidence(subject, other) {
            total += evidence.score(params);
        }
    }
    if others == 0 {
        // A lone member has nothing to be related to. Its belonging rests on
        // whatever put it here, which was not a relationship.
        return 0.0;
    }
    (total / others as f64).clamp(0.0, 1.0)
}

/// Whether any tie from `subject` to another member rests on more than the two
/// having been present at the same time.
fn is_corroborated_within(
    subject: &str,
    members: &BTreeSet<String>,
    graph: &AffinityGraph,
) -> bool {
    members.iter().any(|other| {
        other != subject
            && graph
                .evidence(subject, other)
                .is_some_and(AffinityEvidence::is_corroborated)
    })
}

/// The strongest witnessed reason a member belongs with the rest.
fn strongest_tie(
    subject: &str,
    members: &BTreeSet<String>,
    graph: &AffinityGraph,
    params: &EngagementParams,
) -> Option<(String, EvidenceKind)> {
    let mut best: Option<(String, EvidenceKind, f64)> = None;
    for other in members {
        if other == subject {
            continue;
        }
        let Some(evidence) = graph.evidence(subject, other) else {
            continue;
        };
        let score = evidence.score(params);
        let replace = match &best {
            None => true,
            Some((_, _, best_score)) => score > *best_score,
        };
        if replace {
            best = Some((other.clone(), evidence.strongest(params), score));
        }
    }
    best.map(|(other, kind, _)| (other, kind))
}

fn later(current: Option<SystemTime>, candidate: Option<SystemTime>) -> Option<SystemTime> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

fn earlier(current: Option<SystemTime>, candidate: Option<SystemTime>) -> Option<SystemTime> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

fn short_name(subject: &str) -> &str {
    let trimmed = subject.trim_end_matches('/');
    match trimmed.rsplit('/').next() {
        Some(name) if !name.is_empty() => name,
        _ => subject,
    }
}

/// Convenience: the evidence behind a pair, for callers assembling
/// explanations.
pub fn pair_evidence<'graph>(
    graph: &'graph AffinityGraph,
    left: &str,
    right: &str,
) -> Option<&'graph AffinityEvidence> {
    graph.evidence(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::{Act, segment_episodes};
    use crate::resource::ActCharacter;

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

    fn visit(subject: &str, secs: u64) -> Act {
        act(subject, "OBS-URL-NAVIGATED", secs)
    }

    fn reconstruct(acts: Vec<Act>, params: &EngagementParams) -> EngagementSet {
        let episodes = segment_episodes(acts, params);
        let ledger = AttentionLedger::build(&episodes, params);
        let declarations = Declarations::new();
        let graph = AffinityGraph::build(&episodes, &ledger, &declarations, params);
        EngagementSet::build(&episodes, &ledger, &graph, &declarations, params)
    }

    #[test]
    fn three_interleaved_works_keep_stable_identities_and_share_tools() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _round in 0..4 {
                for (work, unique) in [
                    ("automation x", "x webhook editor"),
                    ("automation y", "y slack editor"),
                    ("automation z", "z crm editor"),
                ] {
                    acts.push(focus(unique, moment)); moment += 120;
                    acts.push(focus("shared browser", moment)); moment += 20;
                    acts.push(focus(&format!("{work} terminal"), moment)); moment += 120;
                }
            }
            moment += 3 * 60 * 60;
        }

        let mut declared = acts;
        for index in 0..40 {
            declared.push(Act::new(
                Resource::new("atlas planning canvas", "OBS-WORK-GROUPED"),
                ActCharacter::of_schema("OBS-WORK-GROUPED"),
                at(moment + index as u64),
            ));
        }
        let set = reconstruct(declared, &params);
        let find = |subject: &str| set.engagements().iter()
            .find(|work| work.seeds().any(|seed| seed.subject() == subject))
            .expect("each unique locus establishes a remembered body of work");
        let x = find("x webhook editor");
        let y = find("y slack editor");
        let z = find("z crm editor");
        assert_ne!(x.work_id(), y.work_id());
        assert_ne!(x.work_id(), z.work_id());
        assert_ne!(y.work_id(), z.work_id());
        assert!(set.every_containing("shared browser").len() >= 2,
            "a shared tool is linked to multiple works instead of choosing or merging");
    }

    #[test]
    fn one_session_work_is_remembered_without_recurrence() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _ in 0..8 {
            acts.push(focus("estate comparison notes", moment)); moment += 180;
            acts.push(visit("https://research.example/estate-comparison", moment)); moment += 180;
        }
        let set = reconstruct(acts, &params);
        let work = set.engagements().iter()
            .find(|work| work.subjects().contains("estate comparison notes"))
            .expect("deep one-session work must remain discoverable");
        assert_eq!(work.recurrence(), 1);
        assert!(work.artifact_links().len() >= 2);
    }

    #[test]
    fn broad_membership_does_not_expand_the_auto_open_set() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for index in 0..40 {
                acts.push(focus("atlas planning canvas", moment));
                moment += 30;
                acts.push(focus(&format!("atlas planning reference {index:02}"), moment));
                moment += if index < 4 { 120 } else { 5 };
            }
            moment += 3 * 60 * 60;
        }
        let set = reconstruct(acts, &params);
        // Without an explicit declaration, generic evidence may conservatively
        // retain this broad history as several overlapping hypotheses. The
        // invariant we can require here is that memory is retained and every
        // projection keeps the immediate restoration set bounded.
        assert!(set.engagements().iter().map(|work| work.participants().len()).sum::<usize>() >= 40);
        assert!(set.engagements().iter().all(|work| work.primary().count() <= params.max_primary));
    }

    /// One coherent task spread across a document, a terminal, and browser
    /// research must converge into exactly one body of work — with nothing in
    /// the code knowing what a document, a terminal, or a browser is.
    #[test]
    fn heterogeneous_resources_of_one_task_converge() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Internship Assignment — Editor", moment));
                moment += 120;
                acts.push(save("/Users/x/work/internship/assignment.md", moment));
                moment += 20;
                acts.push(focus("internship — Terminal", moment));
                moment += 120;
                acts.push(visit("https://docs.example.org/internship/guide", moment));
                moment += 120;
            }
            moment += 60 * 60;
        }
        let set = reconstruct(acts, &params);
        assert_eq!(
            set.len(),
            1,
            "one task must be one body of work, got {:?}",
            set.engagements()
                .iter()
                .map(|e| e.subjects())
                .collect::<Vec<_>>()
        );
        let engagement = &set.engagements()[0];
        assert!(engagement.subjects().len() >= 3);
        assert!(engagement.continuation().is_some());
    }

    /// Two unrelated tasks that happen to live in the same folder must stay
    /// distinct. Structural proximity is not shared purpose.
    #[test]
    fn two_tasks_in_one_location_stay_distinct() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Task one, three sittings.
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Billing Migration — Editor", moment));
                moment += 150;
                acts.push(save("/Users/x/repo/billing/migration.rs", moment));
                moment += 20;
                acts.push(focus("billing migration — Terminal", moment));
                moment += 150;
            }
            moment += 60 * 60;
        }
        // Task two, three separate sittings, same repository.
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Onboarding Copy — Editor", moment));
                moment += 150;
                acts.push(save("/Users/x/repo/billing/onboarding.rs", moment));
                moment += 20;
                acts.push(focus("onboarding copy — Terminal", moment));
                moment += 150;
            }
            moment += 60 * 60;
        }
        let set = reconstruct(acts, &params);
        assert_eq!(
            set.len(),
            2,
            "two tasks in one place are two bodies of work, got {:?}",
            set.engagements()
                .iter()
                .map(|e| e.subjects())
                .collect::<Vec<_>>()
        );
    }

    /// Brief focus, however often repeated, is an encounter and not work.
    #[test]
    fn brief_repeated_encounters_are_not_bodies_of_work() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _visit in 0..40 {
            acts.push(focus("Some Music Player", moment));
            moment += 3;
            acts.push(focus("Some Chat Application", moment));
            moment += 3;
            acts.push(visit("https://search.example.org/?q=something", moment));
            moment += 60 * 60;
        }
        let set = reconstruct(acts, &params);
        assert!(
            set.is_empty(),
            "encounters must not become work, got {:?}",
            set.engagements()
                .iter()
                .map(|e| e.subjects())
                .collect::<Vec<_>>()
        );
    }

    /// Many unrelated resources must not produce many bodies of work.
    #[test]
    fn thirty_unrelated_resources_do_not_make_thirty_bodies_of_work() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for index in 0..30 {
            acts.push(focus(&format!("Unrelated Surface {index}"), moment));
            moment += 45;
            acts.push(save(&format!("/Users/x/misc/thing{index}.dat"), moment));
            moment += 45;
        }
        let set = reconstruct(acts, &params);
        assert!(
            set.len() <= 2,
            "unrelated encounters must not become a wall of work, got {}",
            set.len()
        );
    }

    /// Files the machine rewrote, with no person present, are never work.
    #[test]
    fn machine_change_alone_is_never_a_body_of_work() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        for step in 0..600 {
            acts.push(save("/Users/x/Pictures/Library/syncstatus.plist", step * 4));
            acts.push(save("/Users/x/Pictures/Library/metrics.plist", step * 4 + 1));
        }
        let set = reconstruct(acts, &params);
        assert!(set.is_empty(), "a machine writing files is not work");
    }

    /// Changes plus one deliberate act are not yet a body of work.
    ///
    /// This is the evidence shape a real session produces when file capture is
    /// live but attention capture is not: two files are written and a commit is
    /// recorded, and nothing witnessed a person sitting with any of it. The
    /// commit is the one human act (`ActCharacter::Deliberate`); the writes are
    /// `Incidental` and could equally be a formatter or a checkout.
    ///
    /// Return (RFC-0014 R2) asks for `min_revisits` human acts, and one act is
    /// not a return. Evo therefore declines to name this work rather than
    /// inferring a body of work from a single deliberate moment — the honest
    /// answer when the evidence is one commit long. Add attention to the same
    /// resources across sittings and it becomes work; see
    /// [`heterogeneous_resources_of_one_task_converge`].
    #[test]
    fn changes_plus_one_deliberate_act_are_not_yet_a_body_of_work() {
        let params = EngagementParams::default();
        let acts = vec![
            save("/Users/x/repo-a/plan.md", 0),
            save("/Users/x/repo-a/notes.md", 30),
            save("/Users/x/repo-a/plan.md", 60),
            act("9f1c2d3e4b5a6978", "OBS-COMMIT-MADE", 90),
        ];
        let set = reconstruct(acts, &params);
        assert!(
            set.is_empty(),
            "one commit over background writes is not resumable work, got {:?}",
            set.engagements().len()
        );
    }

    /// The measurement behind [`EngagementParams::internal_cohesion`]: the shape
    /// of a grouping, read off the witnessed graph alone, tells a body of work
    /// apart from a hub with spokes — knowing nothing about what any surface is.
    ///
    /// A body of work is resources used *together*, so its members are related to
    /// one another: nearly every pair clears [`EngagementParams::affinity_floor`],
    /// and the fraction that does approaches one. A hub with spokes — a status
    /// tray, a login screen, a music player kept present beside each separate
    /// errand of a day — is the opposite shape: every spoke is related to the hub
    /// and to none of the others, so almost every *pair* is unrelated and the
    /// fraction approaches zero. This is the crate's one principle — something
    /// beside everything is beside nothing in particular — measured over a whole
    /// grouping rather than a single resource.
    ///
    /// One history here holds both shapes, kept in separate sittings so they share
    /// no witnessed company. Measured against the same floor on the same graph, the
    /// clique sits above the default majority limit and the star far below it: the
    /// separation the limit relies on, present in the evidence itself. The end of
    /// that separation — the standing it decides — is
    /// [`tests::standing_as_work_follows_the_internal_cohesion_limit`], and it is
    /// confirmed on real history by the cohesion probe.
    #[test]
    fn a_hub_with_spokes_measures_as_mostly_unrelated() {
        let params = EngagementParams::default();

        // A genuine body of work: three surfaces worked together, interleaved
        // every sitting, so each is witnessed beside the others.
        let clique = ["atlas", "borealis", "cirrus"];
        // A hub kept present beside six errands, each errand worked only beside the
        // hub and its own private note — never beside another errand.
        let hub = "status tray";
        let spokes = [
            "errand one",
            "errand two",
            "errand three",
            "errand four",
            "errand five",
            "errand six",
        ];

        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Clique sittings: the three surfaces interleaved together.
        for _sitting in 0..3 {
            for _pass in 0..4 {
                for surface in clique {
                    acts.push(focus(surface, moment));
                    moment += 90;
                }
            }
            moment += 60 * 60;
        }
        // Star sittings: each errand lives mostly in its own rich private world —
        // its own draft, data, and log, interleaved together — with the hub only
        // touched in passing. So the hub is beside every errand, but each errand's
        // company is overwhelmingly its own: the hub is a minor shared neighbour,
        // not the thing that defines any errand. This is the real shape of a login
        // screen or a status tray, and the opposite of the clique above.
        for spoke in spokes {
            let world = [
                format!("{spoke} draft"),
                format!("{spoke} data"),
                format!("{spoke} log"),
            ];
            for _sitting in 0..2 {
                for _pass in 0..6 {
                    acts.push(focus(spoke, moment));
                    moment += 90;
                    for companion in &world {
                        acts.push(focus(companion, moment));
                        moment += 90;
                    }
                }
                // The hub, present only in passing.
                acts.push(focus(hub, moment));
                moment += 90;
                acts.push(focus(spoke, moment));
                moment += 90;
                acts.push(focus(hub, moment));
                moment += 90;
                moment += 60 * 60;
            }
        }

        let episodes = segment_episodes(acts, &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        let declarations = Declarations::new();
        let graph = AffinityGraph::build(&episodes, &ledger, &declarations, &params);

        let clique_members: BTreeSet<String> = clique.iter().map(|s| s.to_string()).collect();
        let mut star_members: BTreeSet<String> = spokes.iter().map(|s| s.to_string()).collect();
        star_members.insert(hub.to_string());

        let clique_cohesion = internal_cohesion(&clique_members, &graph, &params);
        let star_cohesion = internal_cohesion(&star_members, &graph, &params);

        assert!(
            clique_cohesion >= params.internal_cohesion,
            "a body of work whose members were used together must clear the \
             majority limit; measured {clique_cohesion:.2}",
        );
        assert!(
            star_cohesion < params.internal_cohesion,
            "a hub with spokes is mostly unrelated pairs and must fall below the \
             majority limit; measured {star_cohesion:.2}",
        );
        // The two shapes are separable by cohesion alone, and not marginally.
        assert!(
            clique_cohesion > star_cohesion,
            "clique {clique_cohesion:.2} must read as more cohesive than star {star_cohesion:.2}",
        );
    }

    /// Whether a grouping is presented as work follows its witnessed cohesion
    /// against [`EngagementParams::internal_cohesion`], not any fixed constant —
    /// and a grouping held back for want of cohesion is still kept whole.
    ///
    /// The grouping here is deliberately mid-scale: a cohering core of five
    /// surfaces worked together every sitting, each also touched beside its own
    /// private companion — an output, a scratch note, a log — that is related to
    /// it and to nothing else. The thread that forms admits all ten; its core
    /// coheres but its companions are pendants, so a majority — though not all —
    /// of its member-pairs are related. That cohesion is a property of the
    /// evidence, the same whatever the limit is set to.
    ///
    /// Set below the measured cohesion, the limit lets the grouping stand as work;
    /// set above it, the very same grouping — same members, same cohesion — is held
    /// back as [`Standing::Remembered`], findable and whole but not claimed as a
    /// body of work. The standing crosses exactly where the evidence crosses the
    /// limit, and nothing is dropped as it does. Nothing here names an application,
    /// a file type, or a category. The extreme this sits above — a login screen or
    /// music player beside everything, far below the default limit — is measured in
    /// [`tests::a_hub_with_spokes_measures_as_mostly_unrelated`].
    #[test]
    fn standing_as_work_follows_the_internal_cohesion_limit() {
        let cores = ["aurora", "basalt", "cobalt", "dune", "ember"];

        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            // The core, interleaved together — five surfaces genuinely used as one.
            for _pass in 0..4 {
                for code in cores {
                    acts.push(focus(&format!("{code} module"), moment));
                    moment += 90;
                }
            }
            // Each surface beside its own private companion, touched only when it is.
            for code in cores {
                for _pass in 0..3 {
                    acts.push(focus(&format!("{code} module"), moment));
                    moment += 120;
                    acts.push(focus(&format!("{code} companion"), moment));
                    moment += 120;
                }
            }
            moment += 60 * 60;
        }

        // The grouping's cohesion is a property of the evidence — read it once from
        // a reconstruction that presents it, then drive the limit around it.
        let find_initial = |set: &EngagementSet| -> Engagement {
            set.engagements()
                .iter()
                .find(|engagement| engagement.subjects().contains("aurora module"))
                .expect("the core must form one grouping")
                .clone()
        };

        let baseline = reconstruct(acts.clone(), &EngagementParams::default());
        let grouping = find_initial(&baseline);
        let work_id = grouping.work_id();
        let find = |set: &EngagementSet| -> Engagement {
            set.engagements()
                .iter()
                .find(|engagement| engagement.work_id() == work_id)
                .expect("the same stable work hypothesis must survive parameter changes")
                .clone()
        };
        let cohesion = grouping.cohesion();
        // A partially cohering grouping — neither a pure clique nor a pure star —
        // is what makes this a test of the limit and not of an extreme.
        assert!(
            (0.45..0.95).contains(&cohesion),
            "expected a mid-scale cohesion to sweep the limit across; got {cohesion:.2}",
        );

        // Below the measured cohesion: the grouping stands as work.
        let lenient = EngagementParams {
            internal_cohesion: cohesion - 0.05,
            ..EngagementParams::default()
        };
        let lenient_set = reconstruct(acts.clone(), &lenient);
        let under_lenient = find(&lenient_set);
        assert!(
            under_lenient.standing().is_work(),
            "with the limit below its cohesion ({cohesion:.2}), the grouping must be \
             presented as work; stood {:?}",
            under_lenient.standing(),
        );
        assert!(
            lenient_set
                .work()
                .iter()
                .any(|engagement| engagement.subjects().contains("aurora module")),
            "the grouping must appear on the work surface under the lenient limit",
        );

        // Above the measured cohesion: the very same grouping is held back.
        let strict = EngagementParams {
            internal_cohesion: cohesion + 0.05,
            ..EngagementParams::default()
        };
        let strict_set = reconstruct(acts.clone(), &strict);
        let under_strict = find(&strict_set);
        assert_eq!(
            under_strict.standing(),
            Standing::Remembered,
            "with the limit above its cohesion ({cohesion:.2}), the grouping must be \
             held back as Remembered",
        );
        assert!(
            !strict_set
                .work()
                .iter()
                .any(|engagement| engagement.work_id() == work_id),
            "the grouping must not be presented as work under the strict limit",
        );

        // No silent loss: the limit changed only what is claimed. It is the same
        // grouping either way — same members, same measured cohesion.
        assert_eq!(
            under_strict.subjects(),
            under_lenient.subjects(),
            "the members must be identical whether or not it is called work",
        );
        assert_eq!(
            under_strict.cohesion(),
            cohesion,
            "cohesion is a property of the evidence, not of the limit",
        );
    }

    #[test]
    fn roles_separate_where_work_continues_from_what_supported_it() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..5 {
                // The document holds most of the attention.
                acts.push(focus("Thesis Draft — Editor", moment));
                moment += 400;
                acts.push(save("/Users/x/thesis/draft.tex", moment));
                moment += 10;
                acts.push(visit("https://library.example.org/thesis/source", moment));
                moment += 30;
            }
            moment += 60 * 60;
        }
        let set = reconstruct(acts, &params);
        assert_eq!(set.len(), 1);
        let engagement = &set.engagements()[0];
        let primaries: Vec<&str> = engagement.primary().map(|p| p.subject()).collect();
        assert!(primaries.len() <= params.max_primary);
        assert!(
            primaries.contains(&"Thesis Draft — Editor"),
            "the place attention was spent is where the work continues"
        );
        // The file that only ever changed was never attended.
        let file = engagement
            .participants()
            .iter()
            .find(|p| p.subject() == "/Users/x/thesis/draft.tex")
            .expect("the file is part of the work");
        assert!(!file.role().opens_on_restore());
    }

    /// §4(E): the continuation point is a distinct claim from "important for
    /// resuming", and exactly one member carries it.
    #[test]
    fn exactly_one_member_is_the_continuation_point() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..5 {
                acts.push(focus("Merger Memo — Editor", moment));
                moment += 400;
                acts.push(focus("Merger filings — Reader", moment));
                moment += 380;
                acts.push(save("/Users/x/merger/memo.docx", moment));
                moment += 10;
            }
            moment += 60 * 60;
        }
        let set = reconstruct(acts, &params);
        let engagement = &set.engagements()[0];

        let continuations: Vec<&str> = engagement
            .participants()
            .iter()
            .filter(|p| p.role() == ResourceRole::Continuation)
            .map(Participant::subject)
            .collect();
        assert_eq!(
            continuations.len(),
            1,
            "Restoration needs exactly one Resume Point"
        );
        assert_eq!(
            engagement.continuation().map(Participant::subject),
            Some(continuations[0])
        );
        assert!(
            engagement.primary().count() >= 1,
            "the continuation point is still a place the work happens"
        );
    }

    /// The continuation point is where the person *stopped*, which is a witnessed
    /// fact — not the member with the most attention, and not a ranking.
    #[test]
    fn the_continuation_point_is_where_the_work_stopped() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..5 {
                // The memo holds far more attention across the whole history.
                acts.push(focus("Merger Memo — Editor", moment));
                moment += 600;
                acts.push(focus("Merger filings — Reader", moment));
                moment += 200;
            }
            moment += 60 * 60;
        }
        // …but the filings are what the person was looking at when they stopped.
        acts.push(focus("Merger filings — Reader", moment));

        let set = reconstruct(acts, &params);
        let engagement = &set.engagements()[0];
        assert_eq!(
            engagement.continuation().map(Participant::subject),
            Some("Merger filings — Reader"),
            "resuming means going back to where you left off, not to where you \
             spent the most time"
        );
    }

    #[test]
    fn reconstruction_is_independent_of_input_order() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Case File — Reader", moment));
                moment += 200;
                acts.push(save("/Users/x/cases/hendricks/brief.pdf", moment));
                moment += 20;
                acts.push(focus("hendricks brief — Notes", moment));
                moment += 200;
            }
            moment += 60 * 60;
        }
        let forward = reconstruct(acts.clone(), &params);
        acts.reverse();
        let reversed = reconstruct(acts, &params);
        assert_eq!(
            forward
                .engagements()
                .iter()
                .map(Engagement::subjects)
                .collect::<Vec<_>>(),
            reversed
                .engagements()
                .iter()
                .map(Engagement::subjects)
                .collect::<Vec<_>>()
        );
    }

    /// A resource type Evo has never seen participates with no new code.
    #[test]
    fn an_unknown_resource_type_participates_without_new_code() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Bracket Assembly — Modeller", moment));
                moment += 200;
                acts.push(act(
                    "/Users/x/parts/bracket-assembly.step",
                    "OBS-CAD-PART-SAVED",
                    moment,
                ));
                moment += 20;
                acts.push(focus("bracket assembly tolerances — Notes", moment));
                moment += 200;
            }
            moment += 60 * 60;
        }
        let set = reconstruct(acts, &params);
        assert_eq!(set.len(), 1);
        assert!(
            set.engagements()[0]
                .subjects()
                .contains("/Users/x/parts/bracket-assembly.step"),
            "an unrecognized resource type joins the work it was used in"
        );
    }

    /// Significance rests on two witnessed facts, and each must be load-bearing
    /// on its own. Depth without return is a window someone left focused;
    /// return without depth is a habit. Proven by taking away one at a time
    /// from a history that otherwise qualifies.
    #[test]
    fn both_return_and_depth_are_necessary() {
        let params = EngagementParams::default();

        // Return and depth: four minutes of attention, revisited, in one
        // sitting.
        let mut qualifying = Vec::new();
        let mut moment = 0u64;
        for _pass in 0..4 {
            qualifying.push(focus("Deposition Summary — Editor", moment));
            moment += 60;
            qualifying.push(focus("hendricks deposition — Reader", moment));
            moment += 60;
        }
        assert_eq!(
            reconstruct(qualifying, &params).len(),
            1,
            "sustained, revisited work is a body of work"
        );

        // Return removed: each resource attended exactly once, same total
        // attention.
        let depth_only = vec![
            focus("Deposition Summary — Editor", 0),
            focus("hendricks deposition — Reader", 300),
            focus("something else entirely", 600),
        ];
        assert!(
            reconstruct(depth_only, &params).is_empty(),
            "attention alone, never returned to, is not work"
        );

        // Depth removed: revisited across many sittings, seconds at a go.
        let mut return_only = Vec::new();
        let mut moment = 0u64;
        for _visit in 0..40 {
            return_only.push(focus("Deposition Summary — Editor", moment));
            moment += 2;
            return_only.push(focus("hendricks deposition — Reader", moment));
            // Beyond the presence horizon: a separate sitting each time.
            moment += 60 * 60;
        }
        assert!(
            reconstruct(return_only, &params).is_empty(),
            "returning briefly, however often, is a habit and not work"
        );
    }

    /// Depth is a per-sitting question, never a lifetime total. This is the
    /// property that keeps a frequent brief habit from out-scoring an
    /// afternoon of work: the habit's total is larger, and it still loses.
    #[test]
    fn frequency_cannot_substitute_for_a_sitting_of_work() {
        let params = EngagementParams::default();

        let mut habit = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..80 {
            habit.push(focus("A Surface", moment));
            moment += 4;
            habit.push(focus("Another Surface", moment));
            // Beyond the presence horizon: a separate sitting each time.
            moment += 60 * 60;
        }
        let habit_episodes = segment_episodes(habit.clone(), &params);
        let habit_ledger = AttentionLedger::build(&habit_episodes, &params);

        let mut work = Vec::new();
        let mut moment = 0u64;
        for _pass in 0..2 {
            work.push(focus("A Surface", moment));
            moment += 70;
            work.push(focus("Another Surface", moment));
            moment += 70;
        }
        let work_episodes = segment_episodes(work.clone(), &params);
        let work_ledger = AttentionLedger::build(&work_episodes, &params);

        assert!(
            habit_ledger.total_attention() > work_ledger.total_attention(),
            "the habit must genuinely out-total the work for this test to mean anything"
        );
        assert!(reconstruct(habit, &params).is_empty());
        assert_eq!(reconstruct(work, &params).len(), 1);
    }

    /// The thresholds are limits on what Evo claims, not hidden opinions, so
    /// lowering them must surface exactly the work they were withholding —
    /// and nothing about *which* resources are involved changes.
    #[test]
    fn what_is_presented_follows_the_stated_limits() {
        let brief = vec![
            focus("Some Surface", 0),
            focus("Another Surface", 20),
            focus("Some Surface", 40),
            focus("Another Surface", 60),
        ];

        let strict = EngagementParams::default();
        assert!(
            reconstruct(brief.clone(), &strict).is_empty(),
            "a minute of attention is below the stated limit"
        );

        let permissive = EngagementParams {
            sustained_sitting_attention: Duration::from_secs(30),
            ..EngagementParams::default()
        };
        assert_eq!(
            reconstruct(brief, &permissive).len(),
            1,
            "the same evidence, read against a lower stated limit, is presentable"
        );
    }

    /// A single resource, however much attention it received and however often
    /// the person came back to it, is not a body of work. This is the condition
    /// that keeps a music player, a file-browser window, and a phone-mirroring
    /// session off the resume surface — and it does so without any of them being
    /// named, because it is a claim about relationship rather than identity.
    ///
    /// The evidence here is deliberately generous: hours of attention across
    /// many sittings. It still fails, because nothing was ever used *with* it.
    #[test]
    fn a_lone_resource_is_an_encounter_not_a_body_of_work() {
        let params = EngagementParams::default();

        let mut alone = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..12 {
            // Two acts a sitting, five minutes apart: real, sustained,
            // repeatedly returned-to attention.
            alone.push(focus("A Surface On Its Own", moment));
            moment += 5 * 60;
            alone.push(focus("A Surface On Its Own", moment));
            moment += 60 * 60;
        }

        let episodes = segment_episodes(alone.clone(), &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        let record = ledger
            .get("A Surface On Its Own")
            .expect("the resource was witnessed");
        assert!(
            record.deepest_sitting() >= params.sustained_sitting_attention,
            "the fixture must clear the depth bar for this test to mean anything"
        );
        assert!(
            record.recurrence() >= 2 && record.human_acts >= params.min_revisits,
            "the fixture must clear the return bar for this test to mean anything"
        );

        let set = reconstruct(alone.clone(), &params);
        assert!(
            set.is_empty(),
            "a resource used with nothing else is an encounter, not work"
        );
        let remembered = set.remembered();
        assert!(
            remembered.iter().any(|engagement| {
                engagement
                    .subjects()
                    .contains("A Surface On Its Own")
            }),
            "and the whole record of it is kept, as something witnessed rather than work"
        );
        assert!(
            remembered
                .iter()
                .all(|engagement| engagement.standing() == Standing::Remembered
                    && !engagement.standing().is_restorable()),
            "which is not a claim Evo will act on: nothing here is opened automatically"
        );

        // §10: the person's judgement outranks the measurement. Saying so makes
        // it their work, with no change to the evidence.
        let episodes = segment_episodes(alone, &params);
        let ledger = AttentionLedger::build(&episodes, &params);
        let mut declarations = Declarations::new();
        declarations.designate("A Surface On Its Own");
        let graph = AffinityGraph::build(&episodes, &ledger, &declarations, &params);
        let declared = EngagementSet::build(&episodes, &ledger, &graph, &declarations, &params);
        assert_eq!(
            declared.len(),
            1,
            "a designation is ground truth and is never overruled by measurement"
        );
    }

    /// The failure that put "Spotify Premium" at the top of a real Home screen.
    ///
    /// A resource the person passes through on their way to everything
    /// accumulates more attention than any single piece of work, so ranking by
    /// attention alone hands it the lead. The evidence that separates it from the
    /// work is not inside the group: it is that most of its life was spent in
    /// sittings this work had nothing to do with.
    ///
    /// Nothing here knows what any of these resources are, and the constant is
    /// deliberately the *least* attended thing in the coursework sittings — if
    /// this passed on attention it would prove nothing.
    #[test]
    fn a_resource_that_lives_in_other_sittings_does_not_lead_the_work() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;

        // Four sittings of coursework, with the constant present throughout.
        for _sitting in 0..4 {
            for _pass in 0..4 {
                acts.push(focus("A Constant Companion", moment));
                moment += 60;
                acts.push(focus("Coursework Brief — Editor", moment));
                moment += 300;
                acts.push(save("/Users/x/coursework/brief.md", moment));
                moment += 30;
            }
            moment += 2 * 60 * 60;
        }
        // Six further sittings of unrelated work, the constant still present.
        for sitting in 0..6 {
            for _pass in 0..4 {
                acts.push(focus("A Constant Companion", moment));
                moment += 300;
                acts.push(focus(&format!("Unrelated Errand {sitting} — Window"), moment));
                moment += 60;
            }
            moment += 2 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let coursework = set
            .engagements()
            .iter()
            .find(|engagement| engagement.subjects().contains("Coursework Brief — Editor"))
            .expect("the coursework was witnessed across four sittings");

        let companion = coursework
            .participants()
            .iter()
            .find(|participant| participant.subject() == "A Constant Companion");

        if let Some(companion) = companion {
            assert!(
                companion.locality() < 1.0,
                "most of the companion's life was spent elsewhere, locality was {}",
                companion.locality()
            );
            assert_ne!(
                companion.role(),
                ResourceRole::Primary,
                "a resource that is merely always around is not where work continues"
            );
        }
        assert_ne!(
            coursework.title(),
            "A Constant Companion",
            "Home must not offer to resume the thing the person passes through"
        );
    }

    /// The other direction, which any fix for the above must not break: work
    /// really can run across many sittings, and recurring is what that looks
    /// like. What separates it from a passing-through resource is that its
    /// members recur *with each other*, so its locality stays whole.
    #[test]
    fn long_running_work_is_not_penalised_for_recurring() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..8 {
            for _pass in 0..4 {
                acts.push(save("/Users/x/thesis/chapter-three.tex", moment));
                moment += 30;
                acts.push(focus("chapter three — Editor", moment));
                moment += 300;
                acts.push(visit("https://library.example.edu/thesis/sources", moment));
                moment += 120;
            }
            moment += 3 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        assert_eq!(set.len(), 1, "eight sittings on one thing is one thing");
        let thesis = &set.engagements()[0];
        for participant in thesis.participants() {
            assert_eq!(
                participant.locality(),
                1.0,
                "{} recurs together with the rest of the work",
                participant.subject()
            );
        }
        assert!(
            thesis
                .primary()
                .any(|participant| participant.subject() == "chapter three — Editor"),
            // The editor, not the file: a save is an instant, and the attention
            // this work received was received in the window it was held open in.
            "work witnessed across eight sittings is still where the work continues"
        );
    }

    /// §14/§15: the name Home shows is the member whose witnessed name carries the
    /// most of the work's own distinctive vocabulary — never assembled, never
    /// invented, and never the merely-loudest participant.
    #[test]
    fn the_work_is_titled_for_what_its_members_share() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..5 {
                // The most-attended member shares nothing with the others.
                acts.push(focus("An Unrelated Companion", moment));
                moment += 240;
                acts.push(focus("Osteology Fieldwork — Notes", moment));
                moment += 180;
                acts.push(save("/Users/x/osteology/fieldwork.md", moment));
                moment += 30;
                acts.push(visit("https://example.org/osteology/fieldwork", moment));
                moment += 120;
            }
            moment += 2 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let work = set
            .engagements()
            .iter()
            .find(|engagement| engagement.subjects().contains("Osteology Fieldwork — Notes"))
            .expect("the fieldwork was witnessed across three sittings");

        assert!(
            work.title().to_lowercase().contains("osteology"),
            "the title must name what the work is about, got {:?}",
            work.title()
        );
        assert!(
            work.titled_by_shared_vocabulary(),
            "and must report that it rests on shared vocabulary"
        );
    }

    /// §14: when the members' names have nothing measurable in common — an
    /// ordinary shape of work — the title is simply where the work continues, and
    /// Evo says so rather than implying it found a theme.
    #[test]
    fn a_title_admits_when_nothing_was_shared() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(save("/Users/x/quarry/ledger.ods", moment));
                moment += 300;
                acts.push(focus("Untitled 3 — Sheet", moment));
                moment += 180;
            }
            moment += 2 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        assert_eq!(set.len(), 1);
        let work = &set.engagements()[0];
        assert!(
            !work.titled_by_shared_vocabulary(),
            "these names share nothing; claiming otherwise would be a fabrication"
        );
        assert!(
            !work.title().trim().is_empty(),
            "a body of work Evo presents is never nameless"
        );
    }

    /// §15/§23: the case that was still broken on the real machine after the
    /// vocabulary rule went in. When no name shares any measurable vocabulary the
    /// title must *not* fall back to whichever resource held the most attention —
    /// that is the signal the vocabulary rule exists to overrule, and reverting to
    /// it makes the whole mechanism ornamental.
    ///
    /// The shape here is the one that produced "‎WhatsApp" over an afternoon of
    /// drafting: a companion that dominates this work's attention, returns to it
    /// repeatedly, shares no words with anything — and is also witnessed in
    /// sittings that are not this work's. Nothing about what it *is* is consulted;
    /// only that the evidence saw it elsewhere.
    #[test]
    fn a_companion_seen_outside_the_work_does_not_name_it() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Four sittings of one body of work. The companion holds most of the
        // attention in every one of them and is returned to more often than the
        // document is.
        for _sitting in 0..4 {
            for _pass in 0..4 {
                acts.push(focus("Marginalia", moment));
                moment += 420;
                acts.push(focus("Cadastral Survey — Draughting", moment));
                moment += 150;
                // Deliberately shares no token with the draughting window: this
                // test is about the path taken when vocabulary measures nothing.
                acts.push(save("/Users/x/plots/9814-b.dgn", moment));
                moment += 30;
            }
            moment += 3 * 60 * 60;
        }
        // Two further sittings in which the companion is present and this work is
        // not. That is the whole of the new evidence: it was seen elsewhere.
        for sitting in 0..2 {
            for _pass in 0..4 {
                acts.push(focus("Marginalia", moment));
                moment += 300;
                acts.push(focus(&format!("Errand {sitting} — Window"), moment));
                moment += 120;
            }
            moment += 3 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let work = set
            .engagements()
            .iter()
            .find(|engagement| {
                engagement
                    .subjects()
                    .contains("Cadastral Survey — Draughting")
            })
            .expect("the survey was witnessed across four sittings");

        // The premise: the companion really does dominate this work's attention,
        // so this is not a case the old ranking would have got right by accident.
        let companion = work
            .participants()
            .iter()
            .find(|participant| participant.subject() == "Marginalia")
            .expect("the companion is a member of this work");
        assert!(
            companion.attention() > Duration::from_secs(0),
            "the companion held real attention here"
        );
        assert!(
            companion.locality() < 1.0,
            "and the evidence saw it outside this work, locality was {}",
            companion.locality()
        );
        assert!(
            !work.titled_by_shared_vocabulary(),
            "no member shares measurable vocabulary — this is the fallback path"
        );

        assert_ne!(
            work.title(),
            "Marginalia",
            "a resource the person also uses elsewhere is not what this work is called"
        );
        // And the name that is chosen belongs to the work entirely.
        let namesake = work
            .participants()
            .iter()
            .find(|participant| participant.resource().display_name() == work.title())
            .expect("the title is one of the members' witnessed names");
        assert_eq!(
            namesake.locality(),
            1.0,
            "the name came from a resource the evidence never saw outside this work"
        );
    }

    /// A file worked in deeply, then left open and tabbed past in later sittings,
    /// still leads the work it *is*.
    ///
    /// This is the exact shape real history produced, and the reason
    /// [`Participant::is_exclusive`] counts sittings the resource *held* the person
    /// rather than sittings it was witnessed in. A source file took twenty-nine
    /// minutes across two long sittings; its editor tab then stayed open through
    /// four later sittings of unrelated work and drew a minute, forty seconds,
    /// twelve seconds, nothing. Read as presence, those four glimpses outvoted the
    /// two hours: the file was denied leadership and the search page consulted
    /// beside it led the work instead — a person coming back would be handed their
    /// old search results and not the file they had been writing.
    ///
    /// Nothing here knows what a `.py` file or a search engine is. The only
    /// difference between the two resources is where the person's attention was
    /// actually spent.
    #[test]
    fn a_file_left_open_after_deep_work_remains_an_immediate_restoration_target() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;

        // Two long sittings of real work: the file and a page consulted beside it.
        for _sitting in 0..2 {
            for _pass in 0..5 {
                acts.push(focus("transform.py", moment));
                moment += 300;
                acts.push(visit("https://forum.example.org/thread/91", moment));
                moment += 90;
            }
            moment += 3 * 60 * 60;
        }
        // Four later sittings of different work, in which the file's window is
        // merely tabbed past. The person is demonstrably occupied elsewhere, so
        // these sittings are the ones that used to destroy its exclusivity.
        for sitting in 0..4 {
            for _pass in 0..4 {
                acts.push(focus(&format!("Errand {sitting} — Window"), moment));
                moment += 300;
                acts.push(focus("transform.py", moment));
                moment += 10;
            }
            moment += 3 * 60 * 60;
        }

        let set = reconstruct(acts, &params);

        // The file belongs to several threads now, and that is the model working:
        // it was witnessed in the later sittings, so those threads remember it. The
        // claim under test is about the one it *is* — the thread a majority of its
        // measured relatedness falls in. Asking for "the thread containing the file"
        // would be asking a question overlap has made ambiguous.
        let candidates = set.every_containing("transform.py");
        assert!(
            candidates.len() > 1,
            "the later sittings really did witness it, so more than one thread \
             remembers it — that is what makes this a test and not a tautology"
        );
        let work = candidates
            .iter()
            .find(|engagement| {
                engagement
                    .participants()
                    .iter()
                    .any(|participant| {
                        participant.subject() == "transform.py"
                            && participant.speaks_for_work(&params)
                    })
            })
            .expect("two long sittings of work on one file is a body of work");

        let file = work
            .participants()
            .iter()
            .find(|participant| participant.subject() == "transform.py")
            .expect("the file is a member of the work it holds the attention of");

        // The premise: the evidence really did witness it outside this work, so
        // this is not a case that passes because there was nothing to contradict.
        assert!(
            file.witnessed_sittings() > file.sittings_here(),
            "witnessed in {} sittings, only {} of them this work's",
            file.witnessed_sittings(),
            file.sittings_here()
        );
        assert!(
            file.locality() < 1.0,
            "locality was {} — the glimpses are visible to the ratio",
            file.locality()
        );

        assert!(
            file.is_exclusive(),
            "every sitting the person *spent* in this file was a sitting of this work"
        );
        assert!(
            file.role().opens_on_restore(),
            "deep work in the file remains immediate restoration context even \
             though another artifact was witnessed later"
        );
        assert_eq!(
            work.continuation().map(Participant::subject),
            Some("https://forum.example.org/thread/91"),
            "the continuation records the last witnessed artifact; importance \
             for restoration is the separate Primary claim above"
        );
    }

    /// A page glanced at in two sittings and dwelt in during neither never leads.
    ///
    /// The other direction of the same rule, and the one that keeps it from being a
    /// mere relaxation. By presence this resource looks ideal — witnessed only in
    /// this work's sittings, returned to, locality a perfect 1.0 — and a person was
    /// never once *in* it. Real history offered a nineteen-second search query as
    /// the place an afternoon of drafting should resume; leadership has to mean
    /// somewhere work was done.
    #[test]
    fn a_page_glanced_at_twice_is_never_where_work_resumes() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;

        for _sitting in 0..3 {
            for _pass in 0..5 {
                acts.push(focus("Thesis Chapter Four — Editor", moment));
                moment += 240;
                acts.push(save("/Users/x/thesis/chapter-four.tex", moment));
                moment += 20;
            }
            // Consulted for a few seconds, twice, and never returned to for longer.
            acts.push(visit("https://example.org/how-to-cite", moment));
            moment += 8;
            acts.push(focus("Thesis Chapter Four — Editor", moment));
            moment += 240;
            moment += 3 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let work = set
            .engagements()
            .iter()
            .find(|engagement| {
                engagement
                    .subjects()
                    .contains("https://example.org/how-to-cite")
            })
            .expect("the page consulted during the work belongs to it");

        let glance = work
            .participants()
            .iter()
            .find(|participant| participant.subject() == "https://example.org/how-to-cite")
            .expect("the page is a member");

        // The premise: by presence this page has every credential leadership used
        // to ask for.
        assert_eq!(
            glance.locality(),
            1.0,
            "the page was never witnessed outside this work"
        );
        assert!(
            glance.returned(&params),
            "and the person came back to it in {} of this work's sittings",
            glance.sittings_here()
        );

        assert!(
            !glance.is_exclusive(),
            "a resource nobody ever spent a sitting in has no life anywhere, here included"
        );
        assert_ne!(
            work.continuation().map(Participant::subject),
            Some("https://example.org/how-to-cite"),
            "a page read for seconds is not where an afternoon of writing resumes"
        );
        assert!(
            !glance.role().opens_on_restore(),
            "and it is not reopened, though it stays available: role was {:?}",
            glance.role()
        );
    }

    /// The relaxation the rule needs to stay honest: if *no* opener is wholly
    /// within the work, Evo still names it rather than presenting a nameless card.
    /// Locality is a preference in the absence of evidence, not a veto.
    #[test]
    fn work_whose_every_member_lives_elsewhere_is_still_named() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Two resources used together across three sittings…
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Sundial", moment));
                moment += 300;
                acts.push(focus("Astrolabe", moment));
                moment += 240;
            }
            moment += 3 * 60 * 60;
        }
        // …and each of them also witnessed apart from the other, so neither is
        // wholly inside the work they form together.
        for _pass in 0..3 {
            acts.push(focus("Sundial", moment));
            moment += 300;
            acts.push(focus("Errand — Window", moment));
            moment += 120;
        }
        moment += 3 * 60 * 60;
        for _pass in 0..3 {
            acts.push(focus("Astrolabe", moment));
            moment += 300;
            acts.push(focus("Other Errand — Window", moment));
            moment += 120;
        }

        let set = reconstruct(acts, &params);
        let work = set
            .engagements()
            .iter()
            .find(|engagement| engagement.subjects().contains("Sundial"))
            .expect("the pair was witnessed together across three sittings");
        assert!(
            !work.title().trim().is_empty() && work.title() != "Untitled work",
            "a body of work Evo presents is never nameless, got {:?}",
            work.title()
        );
    }

    /// An address is shown as the part a person reads. Nothing is added: the
    /// displayed name is a slice of the witnessed subject, so a title can always
    /// be traced back to something Evo saw.
    #[test]
    fn an_address_is_named_without_its_machinery() {
        let resource = Resource::new(
            "https://example.org/search?q=anything&sourceid=chrome&ie=UTF-8#top",
            "OBS-URL-NAVIGATED",
        );
        assert_eq!(resource.display_name(), "example.org/search");
        assert!(
            resource.subject().contains(resource.display_name()),
            "the shown name must be part of the witnessed one"
        );
    }

    /// §4: the file a person keeps saving took part in the work. It is not where
    /// the work continues — a save is an instant and carries no attention — but
    /// calling it merely present would drop the thing the work was about out of
    /// the very sidebar that says what the work consisted of.
    ///
    /// The person has a second, unrelated task here, and it is load-bearing.
    /// Promotion requires the change to be corroborated by something attended
    /// ([`Participant::is_attention_corroborated`]), and corroboration is measured
    /// in absolute IDF nats — a token every subject on the machine carries is
    /// worth zero. In a universe of three resources *every* token is carried by
    /// everything, so "quarterly" and "forecast" would weigh nothing and the file
    /// would have no evidence to promote on. A person with one three-resource task
    /// and no other history is not the case this rule is for; the companion test
    /// [`a_change_with_no_evidence_but_timing_is_reported_as_merely_present`] pins
    /// what happens there, deliberately.
    #[test]
    fn a_repeatedly_changed_resource_took_part_in_the_work() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..2 {
            for _pass in 0..4 {
                acts.push(focus("Quarterly Forecast — Spreadsheet", moment));
                moment += 90;
                acts.push(save("/Users/x/finance/quarterly-forecast.numbers", moment));
                moment += 90;
                acts.push(focus("quarterly forecast reconcile — Terminal", moment));
                moment += 90;
            }
            moment += 4 * 60 * 60;
        }
        // The rest of this person's week, sharing no vocabulary with the forecast.
        // Its only job is to make the machine's vocabulary wider than one task, so
        // that a word two subjects share is worth something.
        for _sitting in 0..2 {
            for _pass in 0..4 {
                acts.push(focus("Hendricks Deposition — Reader", moment));
                moment += 90;
                acts.push(save("/Users/x/cases/hendricks/deposition.pdf", moment));
                moment += 90;
                acts.push(focus("hendricks deposition markup — Notes", moment));
                moment += 90;
            }
            moment += 4 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let work = set
            .engagements()
            .iter()
            .find(|engagement| {
                engagement
                    .subjects()
                    .iter()
                    .any(|subject| subject.ends_with("quarterly-forecast.numbers"))
            })
            .expect("four passes across two sittings is a body of work");
        let file = work
            .participants()
            .iter()
            .find(|participant| participant.subject().ends_with("quarterly-forecast.numbers"))
            .expect("the file was saved in every pass of every sitting");

        assert_eq!(
            file.attention(),
            Duration::ZERO,
            "a save is an instant; Evo measured no dwell here and must not invent any"
        );
        assert_ne!(
            file.role(),
            ResourceRole::Context,
            "something happened to this eight times; it was not merely around"
        );
        assert!(
            !file.role().opens_on_restore(),
            "and it is still not a place to reopen — the window is"
        );
        assert!(
            work.primary()
                .any(|participant| participant.subject() == "Quarterly Forecast — Spreadsheet"),
            "the places the work happens are the ones attention was measured in"
        );
    }

    /// The deliberate trade behind [`Participant::is_attention_corroborated`],
    /// pinned so it cannot be changed by accident.
    ///
    /// A change with nothing to recommend it but timing is reported as merely
    /// present, even when it is in fact the document the work was about. This is a
    /// choice between two errors, made where Evo genuinely cannot tell the two
    /// apart: a person's document and a tool's telemetry log both have zero
    /// attention, whole locality, and high co-presence with everything attended
    /// (measured on a real log: `interleave` 0.76–0.84, `co_episode` 0.88–1.00,
    /// `lexical` 0.000, `structural` 0.000 for both). Claiming such a resource was
    /// "used as part of this" is §1's failure — machine churn presented as work.
    /// Saying it was present is a weaker claim that is true either way.
    ///
    /// What the weaker claim costs, precisely: the resource keeps its place in the
    /// Workspace's membership and in Home's member list, which render every
    /// Attachment without consulting its role, so it stays visible and reachable.
    /// It is excluded from the *restoration* Context Chain, which is asked for the
    /// minimum context needed to understand the Resume Point (CC-4). So the cost is
    /// that Evo does not offer it as part of the context when resuming — not that
    /// it disappears.
    ///
    /// The sibling test [`a_repeatedly_changed_resource_took_part_in_the_work`]
    /// shows the same file promoted as soon as the machine's vocabulary is wide
    /// enough for its name to carry information — which is the ordinary case, and
    /// the reason this trade is acceptable rather than merely defensible.
    #[test]
    fn a_change_with_no_evidence_but_timing_is_reported_as_merely_present() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // The person's entire witnessed history: one task, three resources. Every
        // token is carried by every subject, so no name can be distinctive.
        for _sitting in 0..2 {
            for _pass in 0..4 {
                acts.push(focus("Quarterly Forecast — Spreadsheet", moment));
                moment += 90;
                acts.push(save("/Users/x/finance/quarterly-forecast.numbers", moment));
                moment += 90;
                acts.push(focus("quarterly forecast reconcile — Terminal", moment));
                moment += 90;
            }
            moment += 4 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let work = &set.engagements()[0];
        let file = work
            .participants()
            .iter()
            .find(|participant| participant.subject().ends_with("quarterly-forecast.numbers"))
            .expect("it was saved in every pass; it is a member either way");

        assert!(
            !file.is_attention_corroborated(),
            "in a corpus this small no token carries information, so nothing attended \
             can corroborate the change — this is the premise of the test"
        );
        assert_eq!(
            file.role(),
            ResourceRole::Context,
            "with only timing to go on, Evo reports presence and claims nothing more"
        );
        assert!(
            !file.role().opens_on_restore(),
            "and it is certainly not somewhere to reopen"
        );
    }

    /// §4 (A) versus (B): *merely encountered* survives as its own state, and
    /// promoting changed resources does not quietly abolish it. A resource that
    /// changes whether or not this work is happening is reported as present and
    /// nothing more — no path list, no application list, no frequency cutoff,
    /// only whether its own life is mostly inside this work.
    #[test]
    fn a_change_that_happens_regardless_is_reported_as_merely_present() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        // Three sittings of the work, with the file changing throughout.
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Untitled 3 — Sheet", moment));
                moment += 300;
                acts.push(save("/var/db/opqrs/flush.bin", moment));
                moment += 30;
                acts.push(focus("sheet reconcile — Terminal", moment));
                moment += 180;
            }
            moment += 2 * 60 * 60;
        }
        // And it goes on changing when nobody is working at all: seven more
        // stretches of the same writes with nothing else happening anywhere.
        for _stretch in 0..7 {
            for _pass in 0..4 {
                acts.push(save("/var/db/opqrs/flush.bin", moment));
                moment += 120;
            }
            moment += 2 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let sheet_work = set
            .engagements()
            .iter()
            .find(|engagement| engagement.subjects().contains("Untitled 3 — Sheet"))
            .expect("four passes across three sittings is a body of work");
        let flush = sheet_work
            .participants()
            .iter()
            .find(|participant| participant.subject() == "/var/db/opqrs/flush.bin")
            .expect("it changed in every sitting of the work");

        assert!(
            !flush.person_acted(),
            "a file write is witnessed identically whoever caused it"
        );
        assert!(
            flush.locality() < params.attributable_change_locality,
            "most of its life happens where this work is not, got {}",
            flush.locality()
        );
        assert_eq!(
            flush.role(),
            ResourceRole::Context,
            "so Evo says it was present, and claims nothing further"
        );
    }

    /// A file written once, under a name never used again, is not a participant
    /// in the work that happened to be going on at the time.
    ///
    /// This is the case a real Observation log exposed. Several
    /// `…/event-log-data/logs/FUS/<uuid>-2025.2.0.20-eap.log` files reached
    /// `Supporting` in a person's work with zero attention: each is written once
    /// under a freshly generated name, so each is witnessed in exactly one
    /// sitting, so its locality is `1.0` — not because the work drives the
    /// changes, but because there was never an occasion on which it could have
    /// been seen elsewhere. Locality over a single sitting is not evidence
    /// (`Participant::locality_is_evidence`), so it may not promote.
    ///
    /// The contrast is the whole point: the draft in the same work is saved
    /// across every sitting, so its locality *is* measurable and it stays
    /// `Supporting`. Nothing here knows what a log file, a UUID, or an IDE is.
    #[test]
    fn a_file_written_once_under_a_fresh_name_is_not_a_participant() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Kiln Schedule — Notes", moment));
                moment += 300;
                // The draft: the same subject in every sitting.
                acts.push(save("/Users/x/ceramics/kiln-schedule.md", moment));
                moment += 60;
                acts.push(visit("https://example.org/ceramics/kiln-schedule", moment));
                moment += 240;
            }
            // Written once per sitting under a name that never recurs, and
            // sharing plenty of vocabulary with its own siblings.
            acts.push(save(
                &format!(
                    "/Users/x/Caches/toolvendor/event-log-data/logs/FUS/{sitting}a9b-4479-afee-671462a1f053-2025.2.0.20-eap.log"
                ),
                moment,
            ));
            moment += 2 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let work = set
            .engagements()
            .iter()
            .find(|engagement| engagement.subjects().contains("Kiln Schedule — Notes"))
            .expect("a document, a page and a terminal across three sittings is work");

        for participant in work.participants() {
            if participant.subject().contains("eap.log") {
                assert_eq!(
                    participant.witnessed_sittings(),
                    1,
                    "each ephemeral log is witnessed in exactly one sitting"
                );
                assert!(
                    !participant.locality_is_evidence(),
                    "so its locality of {} is a ratio over one trial, not evidence",
                    participant.locality()
                );
                assert_eq!(
                    participant.role(),
                    ResourceRole::Context,
                    "so it is reported as present and nothing more, not as Supporting"
                );
            }
        }

        let draft = work
            .participants()
            .iter()
            .find(|participant| participant.subject().ends_with("kiln-schedule.md"))
            .expect("the draft is a member");
        assert!(
            draft.witnessed_sittings() >= 2,
            "the draft recurs across sittings, so its locality is measurable"
        );
        assert!(
            draft.locality_is_evidence(),
            "which is what keeps the amendment from costing a genuine draft its role"
        );
        assert_ne!(
            draft.role(),
            ResourceRole::Context,
            "the file the work is about is not merely present"
        );
    }

    /// A person's act is evidence even when Evo cannot measure how long it
    /// lasted. A window brought forward as the last act of a sitting accrues no
    /// attention by design — under-claiming rather than inventing — and must
    /// still not be reported as something that merely happened to be there.
    #[test]
    fn an_act_with_no_measurable_dwell_is_still_a_persons_act() {
        let params = EngagementParams::default();
        let mut acts = Vec::new();
        let mut moment = 0u64;
        for _sitting in 0..3 {
            for _pass in 0..4 {
                acts.push(focus("Ceramics Kiln Log — Notes", moment));
                moment += 300;
                acts.push(save("/Users/x/ceramics/kiln-log.md", moment));
                moment += 60;
            }
            // The last act of every sitting: a person brought it forward, and
            // Evo did not witness how long they stayed.
            acts.push(focus("Whatever This Is", moment));
            moment += 2 * 60 * 60;
        }

        let set = reconstruct(acts, &params);
        let stray = set
            .engagements()
            .iter()
            .flat_map(|engagement| engagement.participants())
            .find(|participant| participant.subject() == "Whatever This Is");

        let stray = stray.expect("it was witnessed in every sitting of the work");
        assert_eq!(
            stray.attention(),
            Duration::ZERO,
            "Evo did not witness how long the person stayed and must not guess"
        );
        assert!(stray.person_acted(), "but a person did bring it forward");
        assert_eq!(
            stray.role(),
            ResourceRole::Reference,
            "used in passing — not something that merely happened to be there"
        );
        assert!(
            !stray.role().opens_on_restore(),
            "and still not a place to reopen"
        );
    }
}
