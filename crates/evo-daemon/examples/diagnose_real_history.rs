//! Measures this machine's real canonical Observation history against the
//! product questions, through the exact code path the daemon runs.
//!
//! [`replay_real_history`](../replay_real_history.rs) prints what Home would
//! show, and stops at the Engagement layer. This one goes all the way through
//! Restoration and prints the *ratios* — how many observed resources become
//! Workspaces, how many Workspace members Restoration would open, which
//! mechanism made each resource a member and which made it a restoration
//! target. A wall of qualitative output cannot answer "is restoration
//! selective"; a ratio can.
//!
//! Read-only. It opens the canonical storage root, decodes the Observation log,
//! derives, and prints. Nothing is written.
//!
//! ```text
//! cargo run -p evo-daemon --example diagnose_real_history
//! ```

use evo_daemon::persistence::{load_artifact_locators, load_persisted_observations};
use evo_daemon::understanding::{derive, evidence_of};
use evo_engagement::{
    ActCharacter, Engagement, EngagementParams, Reconstruction, ResourceRole, SubjectShape,
};
use evo_execution::{select_restoration, Disposition, RestorationSelection};
use evo_storage::canonical_storage_root;
use evo_workspace::projection::project_engagements;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

fn main() {
    let root = canonical_storage_root();
    let observations = match load_persisted_observations(&root) {
        Ok(observations) => observations,
        Err(error) => {
            println!("could not read the Observation log: {error:?}");
            return;
        }
    };
    if observations.is_empty() {
        println!("no observations recorded on this machine");
        return;
    }

    let params = EngagementParams::default();
    let reconstruction = Reconstruction::from_observations(&observations, params);
    let understanding = derive(evidence_of(&observations), params);
    // The canonical (schema, subject) evidence per Artifact — the exact input the
    // Execution layer classifies reopenable targets from. Read from the same log,
    // so what this report says can be opened is what the desktop would open.
    let locators = match load_artifact_locators(&root) {
        Ok(locators) => locators,
        Err(error) => {
            println!("could not read the Artifact locator evidence: {error:?}");
            return;
        }
    };

    println!("================================================================");
    println!("EVO REAL-DATA DIAGNOSIS");
    println!("root: {}", root.display());
    println!("================================================================");

    // ---- 1. raw observations -------------------------------------------
    let mut by_schema: BTreeMap<&str, usize> = BTreeMap::new();
    for observation in &observations {
        *by_schema.entry(observation.schema().name()).or_insert(0) += 1;
    }
    println!();
    println!("[1] RAW OBSERVATIONS: {}", observations.len());
    for (schema, count) in &by_schema {
        println!(
            "      {count:>6}  {schema}  ({:.1}%)",
            100.0 * *count as f64 / observations.len() as f64
        );
    }

    // ---- 2. unique resources, before and after identity ----------------
    let (acts, _) = evo_engagement::interpret(&observations);
    let witnessed: BTreeSet<&str> = acts.iter().map(|act| act.resource().subject()).collect();
    let canonical: BTreeSet<&str> = witnessed
        .iter()
        .map(|subject| reconstruction.identity().canonical(subject))
        .collect();
    println!();
    println!("[2] UNIQUE RESOURCES");
    println!(
        "      distinct witnessed subjects (one per Observation string): {}",
        witnessed.len()
    );
    println!(
        "      distinct resources after identity folding:               {}",
        canonical.len()
    );
    println!(
        "      names folded away: {} ({:.1}% of witnessed subjects were restatements)",
        witnessed.len().saturating_sub(canonical.len()),
        100.0 * witnessed.len().saturating_sub(canonical.len()) as f64 / witnessed.len() as f64
    );

    // The resources whose identity had to be recovered from several names.
    let mut fragmentation: Vec<(usize, &str)> = canonical
        .iter()
        .map(|name| (reconstruction.identity().sighting_count(name), *name))
        .filter(|(count, _)| *count > 1)
        .collect();
    fragmentation.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    println!(
        "      resources recovered from several witnessed names: {}",
        fragmentation.len()
    );
    for (count, name) in fragmentation.iter().take(8) {
        println!("        {count:>3} names -> {}", trim(name, 84));
    }

    // ---- 3-5. workspaces ------------------------------------------------
    let workspaces = understanding.workspaces();
    println!();
    println!("[3] WORKSPACES DERIVED: {}", workspaces.len());
    println!(
        "      {} resources witnessed  ->  {} workspaces   ({:.4} workspaces per resource)",
        canonical.len(),
        workspaces.len(),
        workspaces.len() as f64 / canonical.len() as f64
    );
    println!(
        "      threads kept without the claim of work: {}",
        reconstruction.set().remembered().len()
    );

    let mut sizes: BTreeMap<usize, usize> = BTreeMap::new();
    for workspace in workspaces {
        *sizes.entry(workspace.attachments().len()).or_insert(0) += 1;
    }
    println!();
    println!("[4] WORKSPACE SIZE DISTRIBUTION");
    for (size, count) in &sizes {
        println!("      {count:>4} workspace(s) with {size} member(s)");
    }

    let single = workspaces
        .iter()
        .filter(|workspace| workspace.attachments().len() <= 1)
        .count();
    println!();
    println!(
        "[5] SINGLE-MEMBER WORKSPACES: {single} of {} ({:.1}%)",
        workspaces.len(),
        100.0 * single as f64 / workspaces.len().max(1) as f64
    );

    // ---- 6-9. what kind of resource ------------------------------------
    //
    // Classified by SubjectShape and ActCharacter — Evo's own structural
    // vocabulary, derived from the frozen Observation schema. An Address is a
    // browser resource, a Surface is an application window with no address, a
    // Path is a file, an Opaque subject is repository evidence. No application,
    // domain, or file extension is named, so a resource kind that does not
    // exist yet classifies itself.
    let mut shape_of: BTreeMap<&str, (SubjectShape, BTreeSet<ActCharacter>)> = BTreeMap::new();
    for act in &acts {
        let key = reconstruction.identity().canonical(act.resource().subject());
        let entry = shape_of
            .entry(key)
            .or_insert_with(|| (act.resource().shape(), BTreeSet::new()));
        entry.1.insert(act.character());
    }
    let total_resources = shape_of.len().max(1);
    let mut shapes: BTreeMap<SubjectShape, usize> = BTreeMap::new();
    let mut characters: BTreeMap<String, usize> = BTreeMap::new();
    for (shape, chars) in shape_of.values() {
        *shapes.entry(*shape).or_insert(0) += 1;
        let mut names: Vec<String> = chars.iter().map(|c| format!("{c:?}")).collect();
        names.sort();
        *characters.entry(names.join(" + ")).or_insert(0) += 1;
    }
    println!();
    println!("[6-9] RESOURCE KIND, by witnessed structure");
    for (shape, count) in &shapes {
        let gloss = match shape {
            SubjectShape::Address => "addressable location  (browser resource)",
            SubjectShape::Surface => "named surface         (application window)",
            SubjectShape::Path => "filesystem path       (file)",
            SubjectShape::Opaque => "opaque identifier     (repository evidence)",
        };
        println!(
            "      {count:>5}  ({:>5.1}%)  {gloss}",
            100.0 * *count as f64 / total_resources as f64
        );
    }
    println!("      how the acts on each resource were witnessed:");
    for (kind, count) in &characters {
        println!(
            "      {count:>5}  ({:>5.1}%)  {kind}",
            100.0 * *count as f64 / total_resources as f64
        );
    }

    // ---- 10-12. restoration --------------------------------------------
    println!();
    println!("[10-12] RESTORATION, per workspace");
    let mut complete = 0usize;
    let mut with_next_step = 0usize;
    let mut opened_total = 0usize;
    let mut member_total = 0usize;
    let mut open_ratios: Vec<(f64, usize)> = Vec::new();

    for workspace in workspaces {
        let key = workspace.id().to_string();
        let outcome = understanding.outcomes().get(&key);
        let members = workspace.attachments().len();
        let opens = workspace
            .attachments()
            .iter()
            .filter(|attachment| attachment.opens_on_restore())
            .count();
        member_total += members;
        opened_total += opens;
        open_ratios.push((opens as f64 / members.max(1) as f64, members));

        let (state, next, chain) = match outcome {
            Some(outcome) => (
                if outcome.is_complete() {
                    "complete"
                } else {
                    "INSUFFICIENT"
                },
                outcome.next_step().is_some(),
                outcome.context_chain().artifacts().len(),
            ),
            None => ("<no outcome>", false, 0),
        };
        if state == "complete" {
            complete += 1;
        }
        if next {
            with_next_step += 1;
        }
        println!(
            "      {state:<12} next_step={next:<5} chain={chain:<3} opens {opens}/{members}  {}",
            trim(
                understanding
                    .titles()
                    .get(&key)
                    .map(String::as_str)
                    .unwrap_or("<untitled>"),
                50
            )
        );
    }

    println!();
    println!(
        "[10] workspaces with a complete restoration plan: {complete} of {} ({:.1}%)",
        workspaces.len(),
        100.0 * complete as f64 / workspaces.len().max(1) as f64
    );
    println!(
        "      workspaces with a derived next step:        {with_next_step} of {} ({:.1}%)",
        workspaces.len(),
        100.0 * with_next_step as f64 / workspaces.len().max(1) as f64
    );
    println!("[11] resources Restoration would open automatically: {opened_total}");
    println!(
        "[12] members Restoration holds back:                 {} ({:.1}% of members)",
        member_total.saturating_sub(opened_total),
        100.0 * member_total.saturating_sub(opened_total) as f64 / member_total.max(1) as f64
    );
    println!(
        "      observed resources in no workspace at all:      {}",
        canonical.len().saturating_sub(member_total)
    );
    // Does restoration size grow with workspace size, or stay bounded? The
    // product answer is "bounded" — enough to resume, not everything present.
    open_ratios.sort_by(|a, b| a.1.cmp(&b.1));
    println!("      opens-per-workspace against workspace size:");
    for (ratio, members) in &open_ratios {
        println!(
            "        {members:>3} members -> {:>2} opened  ({:.0}%)",
            (ratio * *members as f64).round() as usize,
            100.0 * ratio
        );
    }

    // ---- 13-14. the mechanism, per resource ----------------------------
    println!();
    println!("[13] WHY EACH RESOURCE IS A MEMBER — the strongest witnessed tie");
    let mut tie_kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut role_counts: BTreeMap<ResourceRole, usize> = BTreeMap::new();
    // Membership evidence, split the way the product question is asked: does this
    // member's tie rest on more than the two things having been present together,
    // and was a person ever witnessed acting on it at all. Since correspondence
    // stopped being able to *constitute* a relationship, every tie necessarily
    // carries co-presence, so "which single signal is strongest" is close to
    // tautological and these two splits are what remain informative.
    let mut corroborated = 0usize;
    let mut co_presence_only = 0usize;
    let mut never_acted_on = 0usize;
    for engagement in reconstruction.engagements() {
        let members = engagement.subjects();
        for participant in engagement.participants() {
            *role_counts.entry(participant.role()).or_insert(0) += 1;
            if participant.is_corroborated() {
                corroborated += 1;
            } else {
                co_presence_only += 1;
            }
            if !participant.person_acted() {
                never_acted_on += 1;
            }
            // The strongest relationship this member holds to any other member,
            // read back out of the graph the clustering decided from. This is
            // the mechanism that made it a member; if it is empty, membership
            // rests on nothing witnessed and that is a defect.
            let tie = members
                .iter()
                .filter(|other| other.as_str() != participant.subject())
                .filter_map(|other| {
                    reconstruction
                        .graph()
                        .evidence(participant.subject(), other)
                        .filter(|evidence| {
                            evidence.score(reconstruction.params()) > 0.0
                        })
                        .map(|evidence| {
                            (evidence.score(reconstruction.params()), evidence.strongest(reconstruction.params()))
                        })
                })
                .max_by(|a, b| a.0.total_cmp(&b.0));
            let label = match tie {
                Some((_, kind)) => format!("{kind:?}"),
                None => "NO WITNESSED TIE to any other member".to_string(),
            };
            *tie_kinds.entry(label).or_insert(0) += 1;
        }
    }
    for (kind, count) in &tie_kinds {
        println!("      {count:>5}  {kind}");
    }
    let membership_total = (corroborated + co_presence_only).max(1);
    println!("      of which, by what the tie rests on:");
    println!(
        "      {corroborated:>5}  corroborated — shares distinctive wording, a location, or was declared ({:.1}%)",
        100.0 * corroborated as f64 / membership_total as f64
    );
    println!(
        "      {co_presence_only:>5}  co-presence only — witnessed together and nothing more ({:.1}%)",
        100.0 * co_presence_only as f64 / membership_total as f64
    );
    println!(
        "      {never_acted_on:>5}  members no person was ever witnessed acting on ({:.1}%)",
        100.0 * never_acted_on as f64 / membership_total as f64
    );

    println!();
    println!("[14] WHY EACH RESOURCE IS OR IS NOT A RESTORATION TARGET — role");
    for (role, count) in &role_counts {
        println!(
            "      {count:>5}  {role:?}{}",
            if role.opens_on_restore() {
                "  <- OPENS AUTOMATICALLY"
            } else {
                ""
            }
        );
    }

    // ---- what the machine will actually open ---------------------------
    //
    // The single most important list in this report. Everything above is a
    // ratio; this is the answer to "what happens when the person comes back".
    //
    // Every member is listed, not only the opened ones, with the disposition
    // Restoration gives it and the witnessed facts the role was decided from. A
    // report that prints only what opens cannot show a *false negative* — the
    // file the work was actually about, sitting in the held-back list — and that
    // is the failure mode this product cannot afford.
    //
    // The disposition and the reason beside each member are *not* computed here.
    // They are read out of `select_restoration` — the same function the desktop
    // calls — over the same Workspace membership and the same canonical locator
    // evidence. A gate that re-derived them would be measuring a second
    // implementation instead of the product, and would keep reporting "OPEN NOW"
    // after the product had stopped agreeing.
    println!();
    println!("================================================================");
    println!("WHAT RESTORATION WOULD OPEN, per body of work");
    println!("================================================================");
    for (engagement, workspace) in project_engagements(&reconstruction, understanding.artifacts()) {
        let key = workspace.id().to_string();
        println!();
        println!("  {}", trim(engagement.title(), 72));
        println!("    {}", engagement.explanation());
        match engagement.continuation() {
            Some(participant) => println!(
                "    RESUMES IN: {}",
                trim(participant.resource().display_name(), 60)
            ),
            None => println!(
                "    NO RESUME POINT — {}",
                silence_reason(engagement, reconstruction.params())
            ),
        }
        // The canonical reason the derivation itself gives for having no Resume
        // Point, printed beside the evidence reading above so the two can be
        // compared rather than trusted separately.
        let Some(outcome) = understanding.outcomes().get(&key) else {
            println!("    <no restoration outcome derived for this body of work>");
            continue;
        };
        if let Some(missing) = outcome.resume_point_missing() {
            println!("    DERIVATION SAYS: {}", missing.reason());
        }

        // Exactly what the product does: the same membership, the same locator
        // evidence, the same selection function.
        let members = workspace.members_across_history();
        let selection = select_restoration(outcome, &members, &locators);
        let dispositions = dispositions_of(&selection);

        for participant in engagement.participants() {
            // Resolved through the same identity map the projection attached
            // members with, so a member's evidence lines up with its Artifact.
            // Two distinct ways a participant can be absent from the selection,
            // kept apart because they are different facts: the name resolves to
            // no Artifact at all, or it resolves to one the Snapshot History
            // never recorded as a member (so `members_across_history` — hence the
            // selection — never saw it). Collapsing them would report a resource
            // Evo cannot identify as though it were merely unattached.
            let artifact = understanding.artifacts().get(participant.subject());
            let entry = artifact.and_then(|artifact| dispositions.get(artifact.as_str()));
            let (label, reason) = match (artifact, entry) {
                (_, Some((label, reason))) => (*label, DispositionNote::Reason(reason)),
                (None, None) => ("UNIDENTIFIED", DispositionNote::NoArtifact),
                (Some(_), None) => ("NOT A MEMBER", DispositionNote::NotAMember),
            };
            println!(
                "      {label:<19} [{:?}] {}",
                participant.role(),
                trim(participant.resource().display_name(), 54)
            );
            println!(
                "                attention {:>5}, sittings here {}/{}, exclusive {:<5} returned {:<5} acted {:<5} corroborated {}",
                human(participant.attention()),
                participant.sittings_here(),
                participant.witnessed_sittings(),
                participant.is_exclusive(),
                participant.returned(reconstruction.params()),
                participant.person_acted(),
                participant.is_corroborated(),
            );
            match reason {
                DispositionNote::Reason(reason) => {
                    println!("                because {}", trim(reason, 96))
                }
                DispositionNote::NoArtifact => println!(
                    "                no canonical Artifact resolves this name, so the \
                     selection never saw it"
                ),
                DispositionNote::NotAMember => println!(
                    "                resolves to an Artifact the Snapshot History never \
                     recorded as a member, so the selection never saw it"
                ),
            }
        }

        // The three answers, counted. `withheld` is deliberately reported as two
        // numbers: kept to hand and kept as record are different promises, and a
        // single "held back" count is what let a reference document the person
        // read three times look identical to a window that was merely on screen.
        println!(
            "      opens now: {}  |  in the continuation but not reopenable: {}",
            selection.restore_worthy().len(),
            selection.unavailable().len(),
        );
        println!(
            "      withheld: {} available if needed, {} history only — all {} still named, none opened",
            selection.available_if_needed().count(),
            selection.history_only().count(),
            selection.withheld().len(),
        );
    }

    // ---- what was seen, grouped, and then refused -----------------------
    //
    // The complement of the report above, and the only place a *false negative*
    // is visible. These clusters cohered — the affinity graph put them together
    // — and then failed the significance test, so Evo will never offer them.
    // Real work that lands here is invisible to the product, which is the
    // failure mode no amount of workspace-quality measurement can detect.
    println!();
    println!("================================================================");
    let remembered = reconstruction.set().remembered();
    println!("SEEN AND KEPT, WITHOUT THE CLAIM OF WORK ({} threads)", remembered.len());
    println!("================================================================");
    let mut kept: Vec<&evo_engagement::Engagement> = remembered.iter().collect();
    kept.sort_by_key(|engagement| std::cmp::Reverse(engagement.participants().len()));
    for engagement in kept {
        let subjects = engagement.subjects();
        let attention: Duration = subjects
            .iter()
            .filter_map(|subject| reconstruction.ledger().get(subject))
            .map(|record| record.attention)
            .sum();
        let human_acts: usize = subjects
            .iter()
            .filter_map(|subject| reconstruction.ledger().get(subject))
            .map(|record| record.human_acts)
            .sum();
        println!();
        println!(
            "  {:?} — {} member(s), {} attention, {human_acts} human act(s), {}",
            trim(engagement.title(), 48),
            subjects.len(),
            human(attention),
            engagement.standing().phrase(),
        );
        for subject in subjects.iter().take(4) {
            let record = reconstruction.ledger().get(subject);
            println!(
                "      {} (attention {}, revisits {})",
                trim(subject, 62),
                record.map(|r| human(r.attention)).unwrap_or_default(),
                record.map(|r| r.recurrence()).unwrap_or(0),
            );
        }
        if subjects.len() > 4 {
            println!("      … and {} more", subjects.len() - 4);
        }
    }

    // Resources this history never witnessed a person at. Reported, never
    // silently dropped: the Observation log that produced them is untouched.
    let unattended = reconstruction.set().unattended();
    println!();
    println!("================================================================");
    println!("WITNESSED WITH NO PERSON IN THE RECORD ({} resources)", unattended.len());
    println!("================================================================");
    for subject in unattended.iter().take(12) {
        println!("      {}", trim(subject, 70));
    }
    if unattended.len() > 12 {
        println!("      … and {} more", unattended.len() - 12);
    }
}

/// What to print beneath a participant in place of its disposition reason.
///
/// A participant either has a canonical selection reason, or it is absent from
/// the selection for one of two distinct reasons that must not be reported as
/// the same thing.
enum DispositionNote<'a> {
    /// The canonical reason the selection attached to this member.
    Reason(&'a str),
    /// The witnessed name resolves to no canonical Artifact at all.
    NoArtifact,
    /// The name resolves to an Artifact, but no Snapshot recorded it as a
    /// member, so the selection never considered it.
    NotAMember,
}

/// Every Artifact the selection reached, mapped to the canonical word for what
/// happens to it and the canonical reason it was given.
///
/// Nothing here is a judgement made by this diagnostic. The three withheld and
/// open labels come from [`evo_execution::Disposition::label`] and the reasons
/// from the selection itself, so the report cannot drift from the product: if
/// the selection layer changes what it opens, or why, this output changes with
/// it and no edit here is required.
///
/// `unavailable` has no `Disposition` because it is not a disposition — it is a
/// member of the current continuation that Evo *would* open and honestly cannot,
/// which is a different fact from having decided not to.
fn dispositions_of(selection: &RestorationSelection) -> BTreeMap<&str, (&'static str, &str)> {
    let mut map: BTreeMap<&str, (&'static str, &str)> = BTreeMap::new();
    for resource in selection.restore_worthy() {
        map.insert(
            resource.artifact_id().as_str(),
            (Disposition::OpenNow.label(), resource.reason()),
        );
    }
    for resource in selection.unavailable() {
        map.insert(
            resource.artifact_id().as_str(),
            ("CANNOT REOPEN", resource.reason()),
        );
    }
    for member in selection.withheld() {
        map.insert(
            member.artifact_id().as_str(),
            (member.disposition().label(), member.reason()),
        );
    }
    map
}

/// Why this body of work has no resume point, read back out of the evidence.
///
/// Silence is a product feature, so an unexplained silence is a defect: the
/// person is owed the reason, and so is anyone auditing the derivation. Every
/// branch here restates a measurement.
fn silence_reason(engagement: &Engagement, params: &EngagementParams) -> String {
    let attended = engagement
        .participants()
        .iter()
        .filter(|participant| participant.attention() > Duration::ZERO)
        .count();
    if attended == 0 {
        return "no member was ever attended".to_string();
    }
    let exclusive = engagement
        .participants()
        .iter()
        .filter(|participant| participant.attention() > Duration::ZERO)
        .filter(|participant| participant.is_exclusive())
        .count();
    let returned = engagement
        .participants()
        .iter()
        .filter(|participant| participant.attention() > Duration::ZERO)
        .filter(|participant| participant.returned(params))
        .count();
    let primaries = engagement.primary().count();
    if primaries > 0 {
        return format!(
            "{primaries} place(s) the work lives, all last attended at the same moment"
        );
    }
    if returned == 0 {
        return format!(
            "no member was returned to: {attended} attended, none in {} or more of this work's sittings",
            params.min_revisits
        );
    }
    if exclusive == 0 {
        return format!(
            "no member belongs to this work alone: {attended} attended, all witnessed elsewhere too"
        );
    }
    format!(
        "{attended} attended, {exclusive} exclusive, {returned} returned — but none is both"
    )
}

fn trim(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let head: String = text.chars().take(width.saturating_sub(1)).collect();
    format!("{head}…")
}

fn human(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m");
    }
    format!("{}h {}m", minutes / 60, minutes % 60)
}
