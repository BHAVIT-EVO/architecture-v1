//! Measures real cosine scores of vague spoken phrases against this machine's
//! real works — read-only.
//!
//! The question this exists to answer: do natural, vague spoken phrases score
//! above the retrieval floor, and is the floor/margin pair calibrated for them?
//! It reads the canonical Observation log without writing (like a live voice
//! request), rebuilds the engine's works, composes the exact documents
//! `voice_facts` matches against, loads the installed all-MiniLM-L6-v2 model,
//! and prints, per phrase, the top scores and the current routing.
//!
//! It also measures enrichment candidates (each appended between the witnessed
//! names and the structural material, the exact placement `voice_facts` uses)
//! and prints the routing under candidate restore floors so the floor can be
//! picked from data instead of from the verbatim calibration alone.
//!
//! ```text
//! cargo run -p evo-daemon --example measure_vague_retrieval
//! ```

use evo_daemon::name_index::NameIndex;
use evo_daemon::threads::engine_from_observations;
use evo_semantic::{Embedder, cosine_similarity};
use evo_storage::{Storage, StorageObjectKind, canonical_storage_root};
use evo_threads::retrieval::{
    RETRIEVAL_FLOOR, RETRIEVAL_MARGIN, RETRIEVAL_RESTORE_FLOOR, thread_document,
    thread_document_with_surface,
};
use evo_voice::FastEmbedEmbedder;

use std::collections::BTreeMap;

/// Phrases a person might actually say, from near-verbatim to very vague, plus
/// controls that name nothing on this machine. The label says what the phrase
/// is relative to this machine's works, judged from the works' own words.
const QUERIES: &[(&str, &str)] = &[
    ("benchmark arena", "hit: names a witnessed work's words"),
    ("the codex docs page", "hit: names a witnessed URL's words"),
    ("custom email banner for investor outreach", "hit: near-verbatim witnessed title"),
    ("the overleaf paper", "hit: names a host"),
    ("the claude chat about the banner", "near-hit: host + topic; two claude works share the host"),
    ("I was working on the pitch thing", "vague: no work says 'pitch'"),
    ("that doc I had open yesterday", "vague: time + document deixis"),
    ("what I was looking at earlier", "vague: pure deixis"),
    ("you know, the thing from before", "vague: pure deixis"),
    ("the thing with the website", "vague: category only"),
    ("open whatever I was just doing", "vague: continuation intent"),
    ("my writing", "miss: no work says 'writing'"),
    ("résumé help", "miss: topic word, no matching work"),
    ("order a pizza for dinner", "control: unrelated"),
    ("quantum computing lecture", "control: unrelated"),
];

/// Candidate restore floors for sensitivity. The production composition
/// interleaves the weakest hit (~0.349) with the strongest vague phrase
/// (~0.353), so the chosen floor must sit above both; 0.40 leaves room for
/// model and history drift while every control stays far below.
const PROPOSED_FLOORS: &[f64] = &[0.30, 0.35, 0.38, 0.40, 0.45];

/// How many nearest works the below-floor fallback would list.
const NEAREST_K: usize = 5;

fn main() {
    let root = canonical_storage_root();
    println!("storage root: {}", root.display());
    let _guard = Storage::with_thread_root(root.clone());
    let (records, _complete) = Storage::new()
        .read_from(StorageObjectKind::Observation, 0)
        .expect("read-only log read");
    let observations = records
        .iter()
        .map(|record| {
            evo_daemon::persistence::decode_observation_record_pub(record).expect("canonical record")
        })
        .collect::<Vec<_>>();
    println!("observations: {}", observations.len());

    let engine = engine_from_observations(&observations);
    let names = NameIndex::from_observations(&observations);
    let briefs = engine.briefs();
    println!("works: {}", briefs.len());

    // The exact document composition `voice_facts` performs: witnessed names
    // first, then the work's own next step (the understanding layer's short
    // natural-language line), then the structural material.
    let production_document = |thread: &evo_threads::ThreadView| {
        let bundle = engine.resume_bundle(thread.id);
        let understanding =
            evo_daemon::work_understanding::understand_named(thread, &bundle, &names);
        let material =
            thread_document_with_surface(thread, &[understanding.next_step.as_str()]);
        let witnessed: Vec<_> = thread
            .anchors
            .iter()
            .map(|a| a.resource.as_str())
            .chain(thread.companions.iter().map(|(r, _)| r.as_str()))
            .filter_map(|r| names.name_of(r))
            .collect();
        if witnessed.is_empty() {
            material
        } else {
            format!("{} | {material}", witnessed.join(" | "))
        }
    };
    let documents: Vec<(u64, String)> = engine
        .threads()
        .iter()
        .map(|thread| (thread.id, production_document(thread)))
        .collect();

    // Enrichment candidates, each placed between the witnessed names and the
    // structural material. The question: does extra natural-language surface
    // help vague phrases without diluting the true hits (generic boilerplate
    // shared by all documents adds mutual mass and shrinks differences)?
    let variant = |extra: &dyn Fn(&evo_threads::ThreadView, &evo_threads::ResumeBundle) -> String| {
        engine
            .threads()
            .iter()
            .map(|thread| {
                let material = thread_document(thread);
                let bundle = engine.resume_bundle(thread.id);
                let witnessed: Vec<_> = thread
                    .anchors
                    .iter()
                    .map(|a| a.resource.as_str())
                    .chain(thread.companions.iter().map(|(r, _)| r.as_str()))
                    .filter_map(|r| names.name_of(r))
                    .collect();
                let names_part = witnessed.join(" | ");
                let extra = extra(thread, &bundle);
                let document = match (names_part.is_empty(), extra.is_empty()) {
                    (true, true) => material,
                    (true, false) => format!("{extra} | {material}"),
                    (false, true) => format!("{names_part} | {material}"),
                    (false, false) => format!("{names_part} | {extra} | {material}"),
                };
                (thread.id, document)
            })
            .collect::<Vec<_>>()
    };
    let resume_reason_variant = variant(&|_, bundle| bundle.resume_reason.clone());
    let next_step_variant = variant(&|thread, bundle| {
        evo_daemon::work_understanding::understand_named(thread, bundle, &names).next_step
    });
    // `next_step_variant` re-derives the production composition (`documents`)
    // with a different but equivalent path; the two must print identical
    // scores. Any divergence is a bug in this example, not in the daemon.
    let narrative_variant = variant(&|thread, bundle| {
        evo_daemon::work_understanding::understand_named(thread, bundle, &names).narrative
    });
    let proposed_variant = variant(&|thread, bundle| {
        let understanding = evo_daemon::work_understanding::understand_named(thread, bundle, &names);
        let mut surface = Vec::new();
        if !bundle.resume_reason.is_empty() && bundle.resume_reason != "Last seen: nothing." {
            surface.push(bundle.resume_reason.clone());
        }
        if !understanding.next_step.is_empty() {
            surface.push(understanding.next_step.clone());
        }
        if !understanding.narrative.is_empty() {
            surface.push(understanding.narrative.clone());
        }
        surface.join(" | ")
    });

    // The `+next_step*` variant re-derives the production composition by a
    // different path; the two must agree byte for byte (a divergence is a bug
    // in this example, not in the daemon).
    assert!(
        documents.iter().zip(&next_step_variant).all(|(a, b)| a.1 == b.1),
        "production and +next_step* document compositions diverged"
    );

    let titles: BTreeMap<u64, String> = engine
        .threads()
        .iter()
        .map(|thread| {
            let title = thread
                .anchors
                .first()
                .and_then(|a| names.present_name_of(&a.resource))
                .unwrap_or_else(|| format!("work {}", thread.id));
            (thread.id, title)
        })
        .collect();

    let embedder = FastEmbedEmbedder::load().expect("installed model");
    println!("model: {}", embedder.profile_name());
    println!("floor={RETRIEVAL_FLOOR} margin={RETRIEVAL_MARGIN}");
    println!();

    if std::env::args().any(|a| a == "--docs") {
        println!("=== documents (production composition, truncated) ===");
        for (id, document) in &documents {
            let shown: String = document.chars().take(360).collect();
            println!("[{id}] {shown}");
        }
        println!();
    }

    let document_vectors: Vec<(u64, Vec<f32>)> = documents
        .iter()
        .map(|(id, text)| (*id, embedder.embed_document(text).expect("embed document")))
        .collect();
    let vectors_of = |docs: &[(u64, String)]| -> Vec<(u64, Vec<f32>)> {
        docs.iter()
            .map(|(id, text)| (*id, embedder.embed_document(text).expect("embed document")))
            .collect()
    };
    let resume_vectors = vectors_of(&resume_reason_variant);
    let next_vectors = vectors_of(&next_step_variant);
    let narrative_vectors = vectors_of(&narrative_variant);
    let proposed_vectors = vectors_of(&proposed_variant);

    let rank = |vector: &[f32], vectors: &[(u64, Vec<f32>)]| -> Vec<(u64, f64)> {
        let mut scored: Vec<(u64, f64)> = vectors
            .iter()
            .map(|(id, doc)| (*id, cosine_similarity(vector, doc)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
        scored
    };
    let route = |scored: &[(u64, f64)], floor: f64| -> String {
        let Some((_, best)) = scored.first() else {
            return "NotFound(empty rack)".to_string();
        };
        if *best >= floor {
            let candidates: Vec<u64> = scored
                .iter()
                .filter(|(_, score)| *score >= RETRIEVAL_MARGIN * best)
                .map(|(id, _)| *id)
                .collect();
            if candidates.len() == 1 {
                format!("RESTORE work {} @ {best:.4}", candidates[0])
            } else {
                format!("Ambiguous {candidates:?} (all >= {:.4})", RETRIEVAL_MARGIN * best)
            }
        } else {
            let nearest: Vec<u64> = scored.iter().take(NEAREST_K).map(|(id, _)| *id).collect();
            format!("Nearest {nearest:?} (best {best:.4} < floor)")
        }
    };

    for (query, note) in QUERIES {
        println!("=== {query:?}  ({note})");
        let vector = embedder.embed_query(query).expect("embed query");
        let scored = rank(&vector, &document_vectors);
        for (id, score) in scored.iter().take(8) {
            println!(
                "  {score:.4}  work {id:<3} {}",
                titles.get(id).map(String::as_str).unwrap_or("?")
            );
        }
        for (label, vectors) in [
            ("+resume_reason", &resume_vectors),
            ("+next_step*", &next_vectors),
            ("+narrative", &narrative_vectors),
            ("+proposed", &proposed_vectors),
        ] {
            let scored = rank(&vector, vectors);
            let top3 = scored
                .iter()
                .take(3)
                .map(|(id, score)| format!("{id}:{score:.3}"))
                .collect::<Vec<_>>()
                .join(" ");
            println!("  {label:<15} {top3}");
        }
        // The daemon's actual decision over the production documents
        // (*: witnessed names | next step | structural material), with the
        // measured restore floor and the nearest-works fallback.
        println!(
            "  daemon routing (restore floor {RETRIEVAL_RESTORE_FLOOR}): {}",
            route(&scored, RETRIEVAL_RESTORE_FLOOR)
        );
        let top2 = scored
            .iter()
            .take(2)
            .map(|(id, score)| format!("{id}:{score:.4}"))
            .collect::<Vec<_>>()
            .join(" vs ");
        println!(
            "  top2 ratio: {top2} ({:.2})",
            scored.get(1).map(|(_, s)| s / scored[0].1).unwrap_or(1.0)
        );
        // Sensitivity to nearby restore floors. On the production composition
        // the bands interleave near 0.35 (a vague phrase at 0.353, the
        // weakest hit at 0.349), so 0.40 deliberately sends both to the
        // carousel; a floor inside the interleave would auto-restore on
        // weak evidence.
        for floor in PROPOSED_FLOORS {
            println!("  floor {floor:.2}: {}", route(&scored, *floor));
        }
        println!();
    }
}
