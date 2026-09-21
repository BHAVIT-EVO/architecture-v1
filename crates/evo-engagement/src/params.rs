//! Engagement parameters.
//!
//! Every threshold in the Engagement layer lives here, in one place, with the
//! semantic reason it exists. This module exists because of a specific product
//! failure: the previous Formation model gated Workspace origination on
//! "witnessed twice", which is not a semantic claim about work at all — it is
//! a counter. Anything a computer touches twice satisfies it.
//!
//! The rule this module follows: a parameter is admissible only when it
//! expresses a *limit of what Evo can honestly claim to have witnessed*, not a
//! preference about applications, professions, or content. Each field below
//! documents which limit it expresses.
//!
//! Nothing here names an application, a domain, a file type, a directory, or a
//! category of work. The parameters are pure time/evidence quantities and
//! apply identically to a lawyer's contract review, a producer's session, and
//! a compiler engineer's build.

use std::time::Duration;

/// The tunable limits of engagement inference.
///
/// All fields are public and the struct is `Copy`, so a caller may adjust any
/// limit (tests do exactly this to prove behavior is a function of evidence
/// and not of a magic number).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngagementParams {
    /// The longest gap between consecutive witnessed events across which Evo
    /// may still claim the person was *continuously present*.
    ///
    /// This is not a guess about work rhythm. It is the horizon beyond which
    /// Evo has no evidence of presence at all, so it must stop treating
    /// subsequent activity as the same sitting. Crossing it starts a new
    /// Episode.
    pub presence_horizon: Duration,

    /// The longest gap between two consecutive witnessed acts that still counts
    /// as one continuous stretch of attention.
    ///
    /// A window that holds focus while the person walks away accumulates wall
    /// clock, not attention, and Evo cannot witness the difference. So an
    /// interval is evidence of attention only if it is short enough to be one
    /// stretch of it: within this bound the whole interval is credited, and
    /// beyond it nothing is — the gap is silence, and silence is not attention.
    /// It is deliberately all-or-nothing. Crediting the bound itself for a long
    /// gap would invent attention out of the absence of evidence, which is how
    /// "left open overnight" outranks real work.
    ///
    /// It also sets the decay window for within-sitting co-presence, where it
    /// carries the same meaning: how close in time two acts must be to be one
    /// stretch of working.
    pub max_interval_attention: Duration,

    /// The minimum number of Episodes in which two resources must co-occur
    /// before cross-Episode co-occurrence is allowed to contribute evidence.
    ///
    /// Below this, a co-occurrence statistic is a coincidence of a single
    /// sitting, not a witnessed pattern of return.
    pub min_co_episodes: usize,

    /// The minimum combined affinity for two resources to be considered
    /// related at all.
    ///
    /// Edges below this are dropped rather than clustered, which is what keeps
    /// unrelated work separate.
    pub affinity_floor: f64,

    /// The minimum *average* affinity between two groups for them to merge
    /// into one body of work.
    ///
    /// Average linkage (not single linkage) is deliberate: single linkage
    /// chains, so one incidental edge between two unrelated tasks would fuse
    /// them. This is the parameter that keeps two tasks inside one repository,
    /// or two tasks in one application, distinct.
    pub cohesion_floor: f64,

    /// Within how many of a locus's *strongest* relationships another locus
    /// must fall — for **both** of them — before the two may be clustered as
    /// one body of work.
    ///
    /// A relationship reaching [`EngagementParams::affinity_floor`] is enough to
    /// remember that two resources were witnessed together; it is not enough, on
    /// its own, to claim they are the *same work*. The window everything happens
    /// beside — a desktop, a music player, a chat kept open — has a moderate,
    /// real relationship to each of the many activities it accompanies, and is
    /// the strongest relationship of none of them. Left to form edges freely,
    /// that one resource fuses every distinct thing beside it into a single blob,
    /// because connectivity is transitive and it touches everything.
    ///
    /// The limit this expresses is the crate's one principle — something that
    /// correlates with everything correlates with nothing — asked of a
    /// relationship rather than a resource: a tie may join two loci into one body
    /// of work only when it is, for each of them, among the few strongest they
    /// were witnessed to have. A source file and the search for the very code in
    /// it are each other's strongest company and stay together even when they
    /// share no words; the desktop they both happened to pass through is nobody's
    /// strongest company and stays out. It names no application, domain, file
    /// type, or category — it is a property of the witnessed relationship graph
    /// alone — and it decides only what may *found* a body of work, never what is
    /// remembered in one: a resource kept out of clustering is still admitted as
    /// a member wherever it was genuinely present, with its measured strength
    /// recorded.
    ///
    /// Two is the smallest rank that admits a chain — a locus, its strongest
    /// partner, and *that* partner's strongest partner — rather than only
    /// isolated pairs, so a body of work spread across three loci is not split
    /// for lack of a direct strongest-tie between its ends. The tests vary it to
    /// prove the separation follows the evidence rather than this number.
    pub reciprocal_rank: usize,

    /// The share of a body of work's member-pairs that must be *related to each
    /// other* before Evo will present the group as one body of work.
    ///
    /// A body of work is resources used *together*. The clustering that founds one
    /// asks whether loci are each other's strongest company; this asks the
    /// completed grouping the plainer question behind it — are its members, in the
    /// main, related to one another at all, or is each of them related only to the
    /// one resource the thread formed around?
    ///
    /// The failure it answers is the mirror of the one
    /// [`EngagementParams::reciprocal_rank`] answers in clustering. A resource
    /// witnessed beside all of a person's activity — a login screen, a music
    /// player, an inbox, a file that churns all day — is present in the sittings of
    /// everything, so the thread formed around it admits, as members, everything
    /// that was ever on screen while it was. Those members are related to *it* and
    /// to nothing else here: the grouping is a hub with spokes, not a body of work.
    /// Measured as the fraction of member-pairs whose affinity reaches
    /// [`EngagementParams::affinity_floor`], such a grouping is almost all
    /// unrelated pairs; a genuine body of work is almost all related ones.
    ///
    /// A strict majority, the same shape and the same reason as
    /// [`EngagementParams::speaks_for_work`] and
    /// [`EngagementParams::attributable_change_locality`]: more of the group's
    /// member-pairs are witnessed as related than are not. It is the crate's one
    /// principle — something beside everything is beside nothing in particular —
    /// asked of a grouping as a whole rather than of a single resource or edge, and
    /// it names no application, domain, file type, or category.
    ///
    /// Deliberately a limit on what Evo will *claim*, not on what it keeps. A
    /// grouping below this bar is not dissolved and nothing is dropped from it: it
    /// stays a complete record with every relationship intact, kept as
    /// [`crate::Standing::Remembered`] rather than presented as work. The tests
    /// vary it to prove the standing follows the evidence rather than this number.
    pub internal_cohesion: f64,

    /// The attention a body of work must have received **within a single
    /// sitting** before Evo will present it as something to resume.
    ///
    /// Deliberately a per-sitting quantity rather than a lifetime total. A
    /// lifetime total is inflated by frequency, so a habit — glancing at the
    /// same three things for a few seconds, forty separate times — accumulates
    /// more of it than an afternoon of genuine work, and Evo would present the
    /// habit as work. Asking instead "was there ever a sitting in which someone
    /// was actually working here?" cannot be gamed by repetition.
    ///
    /// This is a product judgement, stated plainly rather than dressed up: a
    /// couple of minutes of attention in one sitting is the least that could
    /// leave a person with context worth reloading. It says nothing about
    /// applications, content, or professions, and the tests vary it to prove
    /// behavior follows the evidence rather than this number.
    pub sustained_sitting_attention: Duration,

    /// The least attention, within a single sitting, that marks a resource as
    /// one a person was *working in* rather than glancing at — the floor a
    /// resource must clear before it may lead a body of work and be reopened as
    /// the place it resumes.
    ///
    /// Leadership is a sharper claim than membership. To name a resource the
    /// place work continues is to say a person was *doing something* there, not
    /// merely that it was on screen beside the work. A window brought forward for
    /// a few seconds and left is witnessed exactly like one worked in; the only
    /// thing separating them that Evo can measure is how much attention was
    /// spent, and this is where that line falls.
    ///
    /// Deliberately smaller than [`EngagementParams::sustained_sitting_attention`],
    /// because it answers a different question. That threshold asks whether a
    /// *body of work* ever had a genuine working occasion at all, and gates
    /// whether Evo will call the grouping work; this asks, of a *single resource*
    /// inside work already established, whether it was worked in enough to speak
    /// for it. A resource can fall short of a full sitting's depth and still be
    /// plainly more than a glance — the place an interrupted afternoon stopped is
    /// often exactly that — so holding leadership to the significance threshold
    /// would leave honest single-session work with nowhere to resume. Measured
    /// within one sitting rather than as a lifetime total, for the same reason as
    /// the line above: a total is inflated by repetition, and a resource glanced
    /// at forty times is still a glance.
    ///
    /// A product judgement, stated plainly: under a minute of attention in a
    /// sitting is a look, not a stint of work. It names no application, domain,
    /// file type, or category, and the tests vary it to prove behavior follows
    /// the evidence rather than this number.
    pub engaged_attention: Duration,

    /// The number of times a resource must have been *attended* before it can
    /// contribute to a body of work being presentable.
    ///
    /// This is the "return" in activity → interruption → return, and it is the
    /// structural difference between working and passing through. Something
    /// opened once and never opened again was encountered; something the person
    /// kept coming back to was being used. Two is the smallest number that
    /// distinguishes them, and no amount of duration substitutes for it — a
    /// single window left focused is not work no matter how long it sat there.
    pub min_revisits: usize,

    /// The largest number of resources Restoration will open automatically.
    ///
    /// This is a cognitive limit, not a technical one: restoration must put
    /// the person back into the work, not recreate a screenful of windows.
    pub max_primary: usize,

    /// How much of its own witnessed life a resource must have spent inside this
    /// body of work's sittings for a change Evo *cannot attribute to a person* to
    /// count as taking part in the work.
    ///
    /// This is the only judgement Evo can make about an unattributable change. A
    /// file write is witnessed identically whether a person saved a draft or a
    /// service flushed a cache, so the act itself says nothing; what separates
    /// them is whether the changes happen *because* this work is happening. A
    /// draft changes when, and only when, the person is working on it. A cache
    /// changes all day regardless.
    ///
    /// A strict majority, rather than a tuned constant: the resource's life must
    /// be more inside this work than outside it. Below that, Evo says the resource
    /// was present and claims nothing further. This is the crate's one principle —
    /// something that correlates with everything correlates with nothing — asked
    /// of a single resource's changes.
    pub attributable_change_locality: f64,

    /// How much of a resource's *total* measured relatedness must fall inside
    /// one body of work before that resource may speak for it — name it, lead
    /// it, or be reopened as it.
    ///
    /// This is [`crate::engagement::Participant::specificity`] against a bar,
    /// and it is the same shape of judgement as
    /// [`EngagementParams::attributable_change_locality`]: a strict majority,
    /// not a tuned constant. Above it, this body of work is where the resource
    /// mostly belongs. Below it, the resource belongs more to the person's other
    /// work than to this, so whatever it is, it is not what *this* work is —
    /// however much of this work's time it happened to hold.
    ///
    /// It is the crate's one principle — something that correlates with
    /// everything correlates with nothing — asked of a resource against the
    /// person's whole body of work rather than against a single pair. It names
    /// no application, domain, extension or category, and it is symmetric: the
    /// same resource, inside work that is genuinely *about* it, has all of its
    /// relatedness here and leads.
    ///
    /// Deliberately not a membership test. A resource below this bar is still a
    /// full member of the work, with its measured strength recorded — it is
    /// remembered, and offered, and simply never opened or used as a name.
    pub speaks_for_work: f64,

    /// How much stronger the best answer to a retrieval trigger must be than the
    /// runner-up before Evo will act on it rather than ask which one.
    ///
    /// A ratio of two scores on the same scale, so it carries no units and no
    /// assumption about vocabulary. Below it Evo has two candidates it cannot
    /// separate on evidence, and asking "which one?" is the honest answer;
    /// restoring the higher of two indistinguishable scores would present a
    /// coin-flip as a conclusion.
    pub retrieval_margin: f64,

    /// Relative weight of within-Episode interleaving evidence.
    ///
    /// Interleaving — moving back and forth between two resources in one
    /// sitting — is the strongest available witness that two things are part
    /// of the same activity, so it carries the most weight.
    pub weight_interleave: f64,

    /// Relative weight of cross-Episode co-occurrence evidence (normalized
    /// pointwise mutual information).
    pub weight_co_episode: f64,

    /// Relative weight of lexical evidence (distinctive shared vocabulary in
    /// the witnessed names).
    pub weight_lexical: f64,

    /// Relative weight of structural evidence (shared containing location or
    /// origin).
    pub weight_structural: f64,
}

impl Default for EngagementParams {
    fn default() -> Self {
        Self {
            // Twenty minutes: long enough that a phone call, a coffee, or a
            // meeting does not shatter one sitting into fragments; short
            // enough that "this morning" and "last night" are never merged
            // into a single claim of continuous presence.
            presence_horizon: Duration::from_secs(20 * 60),
            // Ten minutes from one interval. Sustained attention is proven by
            // returning, not by leaving something focused.
            max_interval_attention: Duration::from_secs(10 * 60),
            // Two: one shared sitting is a coincidence, two is a pattern.
            min_co_episodes: 2,
            affinity_floor: 0.20,
            cohesion_floor: 0.28,
            // Two: a locus and its single strongest partner, and that partner's
            // strongest partner — a three-locus body of work holds together, a
            // resource beside everything still founds nothing.
            reciprocal_rank: 2,
            // A strict majority: more of a body of work's member-pairs are
            // witnessed as related to each other than are not. Below it the
            // grouping is a hub with spokes — a resource beside everything and the
            // things that were on screen with it — not resources used together.
            internal_cohesion: 0.5,
            // Two minutes of attention inside one sitting.
            sustained_sitting_attention: Duration::from_secs(2 * 60),
            // One minute of attention inside one sitting: below a full working
            // occasion, above a glance. Strictly less than
            // `sustained_sitting_attention` — leading a body of work is a claim
            // about a single resource, not about whether the group is work.
            engaged_attention: Duration::from_secs(60),
            // Two: the smallest number of visits that is a return.
            min_revisits: 2,
            max_primary: 3,
            // A strict majority: the resource's witnessed life must be more
            // inside this body of work than outside it.
            attributable_change_locality: 0.5,
            // A strict majority, the same shape as the line above: more of this
            // resource's measured relatedness is here than everywhere else put
            // together.
            speaks_for_work: 0.5,
            // The runner-up must reach four fifths of the winner before Evo
            // treats them as indistinguishable and asks.
            retrieval_margin: 0.8,
            weight_interleave: 0.45,
            weight_co_episode: 0.25,
            weight_lexical: 0.20,
            weight_structural: 0.10,
        }
    }
}

impl EngagementParams {
    /// The sum of the evidence weights, used to normalize a combined affinity
    /// back into `0.0..=1.0`.
    pub fn weight_total(&self) -> f64 {
        self.weight_interleave + self.weight_co_episode + self.weight_lexical + self.weight_structural
    }
}
