//! Phase 0 architecture audit — measures this machine's real canonical
//! Observation history against the *product contract*, not against the current
//! design's own assumptions.
//!
//! [`diagnose_real_history`](./diagnose_real_history.rs) reports what the
//! current pipeline produces. This one asks the questions the pipeline is not
//! built to answer, so that a redesign decision rests on measurement:
//!
//! 1. Can one resource participate in more than one body of work? (Measured, not
//!    assumed: count the clusters each canonical resource lands in.)
//! 2. Does this corpus actually contain *interleaved* work, and does Evo keep
//!    the strands apart? (Per sitting, how many bodies of work were live.)
//! 3. Exactly which condition denies a body of work its Resume Point.
//! 4. What is lost: groups refused as work, and resources placed in no group at
//!    all, that nonetheless carry witnessed human evidence.
//! 5. Can the work be *named* by anything a person would say?
//! 6. Residual identity fragmentation — resources that are plainly the same
//!    thing under names identity folding did not fold.
//! 7. Is a witnessed change attributable to the place it happened? (Provenance
//!    context, which is what would tie a save to the window it happened in.)
//! 8. Does this corpus contain multi-day work, and can the bodies of work the
//!    person actually returned to after a long gap be restored? (The cross-tab
//!    is the point: restorability should track "worth resuming", not "attended
//!    deeply twice".)
//!
//! Read-only. Opens the canonical storage root, decodes the Observation log,
//! derives, prints. Nothing is written, nothing is persisted, no network.
//!
//! ```text
//! cargo run -p evo-daemon --example audit_work_model
//! ```

use evo_daemon::understanding::{derive, evidence_of};
use evo_engagement::{ActCharacter, EngagementParams, Reconstruction, ResourceRole};
use evo_storage::canonical_storage_root;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

fn main() {
    let root = canonical_storage_root();
    let observations = match evo_daemon::persistence::load_persisted_observations(&root) {
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
    let set = reconstruction.set();
    let episodes = reconstruction.episodes();
    let ledger = reconstruction.ledger();

    // Bodies of work, and what was kept without that claim. Under the Thread model
    // there is no "refused" list: a group that is not work is *remembered*, with
    // all its members and their measured strength. The two lists below are Home
    // versus the rest of the record — not survivors versus the discarded.
    let work = set.work();
    let remembered = set.remembered();

    println!("================================================================");
    println!("EVO PHASE 0 — WORK MODEL AUDIT");
    println!("root: {}", root.display());
    println!("observations: {}  sittings: {}", observations.len(), episodes.len());
    println!("================================================================");

    // Every group the reconstruction produced: bodies of work, and what was
    // remembered without that claim.
    let claimed: Vec<(String, BTreeSet<String>)> = work
        .iter()
        .map(|engagement| {
            (
                engagement.title().to_string(),
                engagement
                    .participants()
                    .iter()
                    .map(|p| p.subject().to_string())
                    .collect(),
            )
        })
        .collect();
    let refused: Vec<BTreeSet<String>> = remembered
        .iter()
        .map(|engagement| {
            engagement
                .participants()
                .iter()
                .map(|p| p.subject().to_string())
                .collect()
        })
        .collect();

    // ---- 1. multi-membership -------------------------------------------
    println!();
    println!("[A] CAN ONE RESOURCE BELONG TO MORE THAN ONE BODY OF WORK?");
    let mut groups_per_subject: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, members) in &claimed {
        for member in members {
            *groups_per_subject.entry(member.as_str()).or_insert(0) += 1;
        }
    }
    let mut all_groups_per_subject = groups_per_subject.clone();
    for members in &refused {
        for member in members {
            *all_groups_per_subject.entry(member.as_str()).or_insert(0) += 1;
        }
    }
    let mut distribution: BTreeMap<usize, usize> = BTreeMap::new();
    for count in all_groups_per_subject.values() {
        *distribution.entry(*count).or_insert(0) += 1;
    }
    println!("      resources placed in a group (claimed or refused): {}", all_groups_per_subject.len());
    for (groups, resources) in &distribution {
        println!("        {resources:>4} resource(s) appear in exactly {groups} group(s)");
    }
    let shared = all_groups_per_subject.values().filter(|c| **c > 1).count();
    println!(
        "      resources shared across two or more groups: {shared}  <-- the product requires this to be possible"
    );

    // ---- 2. interleaving: is more than one body of work live at once? ---
    println!();
    println!("[B] DOES THE CORPUS INTERLEAVE, AND DOES EVO KEEP THE STRANDS APART?");
    let mut live_per_sitting: Vec<(usize, Vec<&str>)> = Vec::new();
    for (index, episode) in episodes.iter().enumerate() {
        let present = episode.subjects();
        let mut live: Vec<&str> = Vec::new();
        for (title, members) in &claimed {
            if members.iter().any(|member| present.contains(member)) {
                live.push(title.as_str());
            }
        }
        live_per_sitting.push((index, live));
    }
    let mut breadth: BTreeMap<usize, usize> = BTreeMap::new();
    for (_, live) in &live_per_sitting {
        *breadth.entry(live.len()).or_insert(0) += 1;
    }
    for (count, sittings) in &breadth {
        println!("        {sittings:>4} sitting(s) had {count} claimed body/bodies of work live in them");
    }
    let interleaved: Vec<&(usize, Vec<&str>)> =
        live_per_sitting.iter().filter(|(_, live)| live.len() > 1).collect();
    println!(
        "      sittings in which the person interleaved two or more claimed bodies of work: {}",
        interleaved.len()
    );
    for (index, live) in interleaved.iter().take(8) {
        println!("        sitting {index:>3}: {}", live.join("  |  "));
    }

    // ---- 3. what denies the Resume Point -------------------------------
    println!();
    println!("[C] WHY EACH BODY OF WORK HAS OR LACKS A RESUME POINT");
    println!(
        "      (Primary requires exclusive AND returned; exclusive requires >= {}s of attention in >= 2 sittings)",
        params.sustained_sitting_attention.as_secs()
    );
    let mut lacking = 0usize;
    let mut blocked_by_dwell_only = 0usize;
    for engagement in work {
        let has_continuation = engagement
            .participants()
            .iter()
            .any(|p| p.role() == ResourceRole::Continuation);
        if has_continuation {
            println!("      HAS RESUME POINT   {}", trim(engagement.title(), 60));
            continue;
        }
        lacking += 1;
        // Which of the two conjuncts of `exclusive` actually failed: a resource
        // must have spent a sitting's worth of attention here in at least two
        // separate sittings before exclusivity is even asked about.
        let mut best: Option<(&str, usize, usize, Duration)> = None;
        for participant in engagement.participants() {
            let record = ledger.get(participant.subject());
            let dwelling = record.map_or(0, |r| {
                r.per_episode
                    .values()
                    .filter(|attention| **attention >= params.sustained_sitting_attention)
                    .count()
            });
            let deepest = record.map_or(Duration::ZERO, |r| r.deepest_sitting());
            let sittings = participant.episodes().len();
            let better = match best {
                None => true,
                Some((_, best_dwelling, _, best_deepest)) => {
                    dwelling > best_dwelling
                        || (dwelling == best_dwelling && deepest > best_deepest)
                }
            };
            if better {
                best = Some((participant.subject(), dwelling, sittings, deepest));
            }
        }
        match best {
            Some((subject, dwelling, sittings, deepest)) => {
                if dwelling < 2 {
                    blocked_by_dwell_only += 1;
                }
                println!(
                    "      NO RESUME POINT    {}\n            best candidate: {}\n            deepest single sitting {}s; sittings with a sitting's worth of attention: {}; sittings in this work: {}\n            -> {}",
                    trim(engagement.title(), 60),
                    trim(subject, 60),
                    deepest.as_secs(),
                    dwelling,
                    sittings,
                    if dwelling < 2 {
                        "BLOCKED: fewer than two sittings of real attention, so exclusivity is never even asked"
                    } else {
                        "reached two sittings of real attention; blocked by the elsewhere clause"
                    }
                );
            }
            None => println!(
                "      NO RESUME POINT    {}\n            no member carried any measurable attention",
                trim(engagement.title(), 60)
            ),
        }
    }
    println!(
        "      bodies of work with no Resume Point: {lacking} of {}  ({} because no member reached two sittings of real attention)",
        work.len(),
        blocked_by_dwell_only
    );

    // ---- 3b. does interleaving itself destroy exclusivity? --------------
    println!();
    println!("[C2] FOR EACH BLOCKED CANDIDATE: WHERE ITS DWELLING SITTINGS FELL");
    println!("      (exclusive requires every dwelling sitting to be inside this work,");
    println!("       or to be a sitting in which nothing else held the person)");
    // Sittings in which some resource was attended or acted on — the ledger's
    // whole picture, so a dwelling sitting can be placed as inside this work,
    // shared with other attended work, or alone.
    let attended_in: BTreeMap<usize, BTreeSet<&String>> = {
        let mut map: BTreeMap<usize, BTreeSet<&String>> = BTreeMap::new();
        for subject in ledger.subjects() {
            let Some(record) = ledger.get(subject) else { continue };
            for (sitting, attention) in &record.per_episode {
                if *attention > Duration::ZERO {
                    map.entry(*sitting).or_default().insert(subject);
                }
            }
            for sitting in &record.human_episodes {
                map.entry(*sitting).or_default().insert(subject);
            }
        }
        map
    };
    let mut examined = 0usize;
    for engagement in work {
        if engagement
            .participants()
            .iter()
            .any(|p| p.role() == ResourceRole::Continuation)
        {
            continue;
        }
        let members: BTreeSet<&str> = engagement
            .participants()
            .iter()
            .map(|p| p.subject())
            .collect();
        for participant in engagement.participants() {
            let Some(record) = ledger.get(participant.subject()) else { continue };
            let dwelling: Vec<usize> = record
                .per_episode
                .iter()
                .filter(|(_, attention)| **attention >= params.sustained_sitting_attention)
                .map(|(sitting, _)| *sitting)
                .collect();
            if dwelling.len() < 2 {
                continue;
            }
            examined += 1;
            let inside: Vec<usize> = dwelling
                .iter()
                .copied()
                .filter(|sitting| participant.episodes().contains(sitting))
                .collect();
            let outside: Vec<usize> = dwelling
                .iter()
                .copied()
                .filter(|sitting| !participant.episodes().contains(sitting))
                .collect();
            let mut fatal: Vec<(usize, usize)> = Vec::new();
            for sitting in &outside {
                let others = attended_in
                    .get(sitting)
                    .map(|set| {
                        set.iter()
                            .filter(|subject| !members.contains(subject.as_str()))
                            .count()
                    })
                    .unwrap_or(0);
                if others > 0 {
                    fatal.push((*sitting, others));
                }
            }
            println!(
                "      {}\n            {}",
                trim(engagement.title(), 60),
                trim(participant.subject(), 60)
            );
            println!(
                "            dwelt in {} sitting(s): {} inside this work, {} outside",
                dwelling.len(),
                inside.len(),
                outside.len()
            );
            if fatal.is_empty() {
                println!("            nothing outside was contested — exclusivity was not denied here");
            } else {
                for (sitting, others) in &fatal {
                    println!(
                        "            sitting {sitting}: outside this work, and {others} other attended resource(s) held the person there  -> EXCLUSIVITY DENIED"
                    );
                }
            }
        }
    }
    if examined == 0 {
        println!("      NULL RESULT: no blocked candidate anywhere in this corpus reached two");
        println!("      sittings of real attention, so the elsewhere clause never fired at all.");
        println!("      Exclusivity is denied earlier, by the recurrence requirement itself.");
    }

    // ---- 3c. would any resource belong to two bodies of work? -----------
    println!();
    println!("[C3] FORCED CHOICE — RESOURCES RELATED TO MORE THAN ONE BODY OF WORK");
    println!(
        "      (measured affinity at or above the relatedness floor {:.2}, ignoring the partition)",
        params.affinity_floor
    );
    let graph = reconstruction.graph();
    let mut contested: Vec<(&String, Vec<(&str, f64)>)> = Vec::new();
    for subject in ledger.subjects() {
        let mut ties: Vec<(&str, f64)> = Vec::new();
        for (title, members) in &claimed {
            let strongest = members
                .iter()
                .filter(|member| member.as_str() != subject.as_str())
                .map(|member| graph.affinity(subject, member, &params))
                .fold(0.0_f64, f64::max);
            if strongest >= params.affinity_floor {
                ties.push((title.as_str(), strongest));
            }
        }
        if ties.len() > 1 {
            ties.sort_by(|a, b| b.1.total_cmp(&a.1));
            contested.push((subject, ties));
        }
    }
    contested.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(b.0)));
    println!(
        "      resources measurably related to two or more claimed bodies of work: {} of {}",
        contested.len(),
        ledger.subjects().count()
    );
    println!("      each was assigned to exactly one; the other relationships were discarded");
    for (subject, ties) in contested.iter().take(10) {
        println!("        {}", trim(subject, 66));
        for (title, score) in ties.iter().take(4) {
            println!("            {score:.2}  {}", trim(title, 56));
        }
    }
    // ---- 4. what is lost ------------------------------------------------
    println!();
    println!("[D] WHAT IS LOST — EVIDENCE THE PRODUCT NEVER SEES");
    let claimed_members: BTreeSet<&str> = groups_per_subject.keys().copied().collect();
    // Refused groups that nonetheless carry real human evidence.
    let mut refused_with_evidence: Vec<(usize, usize, Duration, &BTreeSet<String>)> = Vec::new();
    for members in &refused {
        if members.len() < 2 {
            continue;
        }
        let human: usize = members
            .iter()
            .map(|m| ledger.get(m).map_or(0, |r| r.human_acts))
            .sum();
        let attention: Duration = members
            .iter()
            .map(|m| ledger.get(m).map_or(Duration::ZERO, |r| r.attention))
            .sum();
        if human >= 2 {
            refused_with_evidence.push((members.len(), human, attention, members));
        }
    }
    refused_with_evidence.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.1.cmp(&a.1)));
    println!(
        "      groups refused as work despite two or more members and two or more human acts: {}",
        refused_with_evidence.len()
    );
    for (size, human, attention, members) in refused_with_evidence.iter().take(10) {
        println!(
            "        {size} member(s), {human} human act(s), {}s attention",
            attention.as_secs()
        );
        for member in members.iter().take(4) {
            let record = ledger.get(member);
            println!(
                "            {}  (attention {}s, sittings {})",
                trim(member, 62),
                record.map_or(0, |r| r.attention.as_secs()),
                record.map_or(0, |r| r.episodes.len())
            );
        }
    }
    // Resources the person demonstrably worked on that reached no claimed work.
    let mut orphans_with_attention: Vec<(&String, Duration, usize, usize)> = Vec::new();
    for subject in ledger.subjects() {
        if claimed_members.contains(subject.as_str()) {
            continue;
        }
        let Some(record) = ledger.get(subject) else { continue };
        if record.human_acts == 0 {
            continue;
        }
        orphans_with_attention.push((
            subject,
            record.attention,
            record.episodes.len(),
            record.human_acts,
        ));
    }
    orphans_with_attention.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let orphan_attention: Duration = orphans_with_attention.iter().map(|(_, a, _, _)| *a).sum();
    println!(
        "      resources a person was witnessed acting on that reached NO claimed body of work: {}",
        orphans_with_attention.len()
    );
    println!(
        "        carrying {} minutes of witnessed attention in total",
        orphan_attention.as_secs() / 60
    );
    for (subject, attention, sittings, human) in orphans_with_attention.iter().take(12) {
        println!(
            "            {}s over {sittings} sitting(s), {human} human act(s)  {}",
            attention.as_secs(),
            trim(subject, 58)
        );
    }

    // ---- 5. can the work be named? --------------------------------------
    println!();
    println!("[E] CAN THE WORK BE NAMED BY ANYTHING A PERSON WOULD SAY?");
    let mut grounded = 0usize;
    for engagement in work {
        let by_vocabulary = engagement.titled_by_shared_vocabulary();
        if by_vocabulary {
            grounded += 1;
        }
        println!(
            "      {}  {}",
            if by_vocabulary { "shared wording " } else { "one member's name" },
            trim(engagement.title(), 68)
        );
    }
    println!(
        "      titles resting on vocabulary the members actually share: {grounded} of {}",
        work.len()
    );

    // ---- 6. residual identity fragmentation -----------------------------
    println!();
    println!("[F] RESIDUAL IDENTITY FRAGMENTATION");
    println!("      canonical resources that share a long leading prefix — plainly one thing under several names");
    let canonical: Vec<&String> = ledger.subjects().collect();
    let mut families: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for subject in &canonical {
        // Purely structural: the leading run of non-digit characters, at least
        // eight long. No application, domain, or category is named.
        let stem: String = subject.chars().take_while(|c| !c.is_ascii_digit()).collect();
        if stem.chars().count() >= 8 {
            families.entry(stem).or_default().push(subject);
        }
    }
    let mut fragmented: Vec<(usize, &String, &Vec<&String>)> = families
        .iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(stem, group)| (group.len(), stem, group))
        .collect();
    fragmented.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let fragmented_total: usize = fragmented.iter().map(|(n, _, _)| *n).sum();
    println!(
        "      {} families covering {} of {} canonical resources ({:.1}%)",
        fragmented.len(),
        fragmented_total,
        canonical.len(),
        100.0 * fragmented_total as f64 / canonical.len().max(1) as f64
    );
    for (size, stem, group) in fragmented.iter().take(8) {
        println!("        {size} names under “{}”", trim(stem, 50));
        for member in group.iter().take(3) {
            println!("            {}", trim(member, 68));
        }
    }

    // ---- 7. what the evidence base can and cannot support ---------------
    println!();
    println!("[G] EVIDENCE BASE — WHAT THIS CORPUS CAN AND CANNOT ESTABLISH");
    let (acts, declarations) = evo_engagement::interpret(&observations);
    let mut by_character: BTreeMap<&str, usize> = BTreeMap::new();
    for act in &acts {
        let label = match act.character() {
            ActCharacter::Attentional => "Attentional (a person looked)",
            ActCharacter::Deliberate => "Deliberate (a person committed)",
            ActCharacter::Incidental => "Incidental (something changed)",
        };
        *by_character.entry(label).or_insert(0) += 1;
    }
    for (label, count) in &by_character {
        println!(
            "      {count:>6}  {label}  ({:.1}%)",
            100.0 * *count as f64 / acts.len() as f64
        );
    }
    println!(
        "      statements the person made about their own work: {} grouping(s), {} designation(s), {} continuation subject(s)",
        declarations.groupings().count(),
        declarations.designations().count(),
        declarations.continuations().count()
    );
    println!(
        "      workspaces the daemon would commit: {}",
        understanding.workspaces().len()
    );
    println!("      GROUND TRUTH FOR AUTO-OPEN PRECISION/RECALL: UNKNOWN");
    println!("        no labelled record of what the person considered one body of work exists in");
    println!("        this corpus, so precision and recall cannot be computed, only bounded by");
    println!("        the loss measured in [D].");

    // ---- 8. provenance: can a change be attributed to where it happened? ---
    println!();
    println!("[H] IS A WITNESSED CHANGE ATTRIBUTABLE TO THE PLACE IT HAPPENED?");
    println!("      An Observation carries Provenance::context. If it is empty, a FILE-SAVED");
    println!("      cannot be tied to the window the person was in, so 'which artifacts");
    println!("      participated' loses its strongest available link.");
    let mut with_context = 0usize;
    let mut context_keys: BTreeMap<&str, usize> = BTreeMap::new();
    for observation in &observations {
        let context = observation.provenance().context();
        if !context.is_empty() {
            with_context += 1;
            for key in context.keys() {
                *context_keys.entry(key.as_str()).or_insert(0) += 1;
            }
        }
    }
    println!(
        "      observations carrying any provenance context: {} of {} ({:.1}%)",
        with_context,
        observations.len(),
        100.0 * with_context as f64 / observations.len() as f64
    );
    if context_keys.is_empty() {
        println!("      context keys present: NONE — the attribution link is absent from the corpus");
    } else {
        for (key, count) in &context_keys {
            println!("        {key}: {count}");
        }
    }

    // ---- 9. does this corpus span days, and are there long gaps? ----------
    println!();
    println!("[I] MULTI-DAY WORK AND LONG GAPS (stress scenarios 10 and 11)");
    let mut boundaries: Vec<(usize, std::time::SystemTime, std::time::SystemTime)> = Vec::new();
    for (index, episode) in episodes.iter().enumerate() {
        if let (Some(started), Some(ended)) = (episode.started_at(), episode.ended_at()) {
            boundaries.push((index, started, ended));
        }
    }
    if let (Some(first), Some(last)) = (boundaries.first(), boundaries.last()) {
        let span = last.2.duration_since(first.1).unwrap_or(Duration::ZERO);
        println!(
            "      corpus spans {:.1} day(s) across {} sitting(s) with known bounds",
            span.as_secs_f64() / 86_400.0,
            boundaries.len()
        );
    }
    let mut long_gaps = 0usize;
    let mut widest = Duration::ZERO;
    for pair in boundaries.windows(2) {
        let gap = pair[1].1.duration_since(pair[0].2).unwrap_or(Duration::ZERO);
        if gap >= Duration::from_secs(86_400) {
            long_gaps += 1;
        }
        if gap > widest {
            widest = gap;
        }
    }
    println!(
        "      gaps between consecutive sittings of a day or more: {long_gaps}  widest gap: {:.1} day(s)",
        widest.as_secs_f64() / 86_400.0
    );
    // A body of work is multi-day if the sittings it was live in straddle a
    // day-long gap. Measured, not assumed.
    let mut multi_day = 0usize;
    let mut resumed_after_long_gap = 0usize;
    // Indexed by position, never by title: two bodies of work in this corpus
    // share the title "www.google.com/search", so a title lookup silently
    // conflates them — the very defect [E] reports.
    let mut multi_day_titles: Vec<(usize, String, bool)> = Vec::new();
    for (index_of_work, (title, members)) in claimed.iter().enumerate() {
        let mut live: Vec<usize> = Vec::new();
        for (index, episode) in episodes.iter().enumerate() {
            if episode.subjects().iter().any(|s| members.contains(s)) {
                live.push(index);
            }
        }
        if live.len() < 2 {
            multi_day_titles.push((index_of_work, title.clone(), false));
            continue;
        }
        let mut straddles = false;
        for pair in live.windows(2) {
            let earlier = boundaries.iter().find(|b| b.0 == pair[0]);
            let later = boundaries.iter().find(|b| b.0 == pair[1]);
            if let (Some(earlier), Some(later)) = (earlier, later) {
                let gap = later.1.duration_since(earlier.2).unwrap_or(Duration::ZERO);
                if gap >= Duration::from_secs(86_400) {
                    straddles = true;
                }
            }
        }
        if straddles {
            multi_day += 1;
            resumed_after_long_gap += 1;
        }
        multi_day_titles.push((index_of_work, title.clone(), straddles));
    }
    println!(
        "      claimed bodies of work live across a day-long gap: {multi_day} of {} (i.e. genuinely resumed later: {resumed_after_long_gap})",
        claimed.len()
    );
    // Cross-tab: is restorability under the *current* model the same thing as
    // multi-day recurrence? If the only restorable work is the multi-day work,
    // the model restores habits rather than tasks.
    println!("      cross-tab — has a Resume Point today vs spans a day-long gap:");
    for (index_of_work, title, straddles) in &multi_day_titles {
        let restorable = work[*index_of_work]
            .participants()
            .iter()
            .any(|p| p.role() == ResourceRole::Continuation);
        println!(
            "        resume point: {:<3}  multi-day: {:<3}  [{index_of_work}] {}",
            if restorable { "yes" } else { "no" },
            if *straddles { "yes" } else { "no" },
            trim(title, 54)
        );
    }
}

fn trim(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    let mut out: String = value.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}
