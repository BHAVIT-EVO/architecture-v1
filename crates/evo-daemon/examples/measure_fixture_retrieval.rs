//! Measures real cosine scores of spoken-style phrases against a small rack of
//! representative, fictional works — no personal history is read or written.
//!
//! The question this exists to answer: with the production document
//! composition and the production model, which phrases clear the restore bar
//! *and* lead the runner-up by the margin, which fall to the nearest-works
//! carousel, and what floor and margin separate those two outcomes? Every
//! phrase carries an expected target (or none), so the run is also a check:
//! a wrong restore, a missed restore, or a control that auto-restored exits
//! nonzero.
//!
//! The works below are fixtures, not a vocabulary: they stand in for any
//! person's own witnessed material. The documents are composed exactly as
//! `voice_facts` composes them — witnessed names first, then the work's own
//! next-step line, then the structural material (`thread_document_with_surface`)
//! — so the measured scores are scores of the production matching text.
//!
//! ```text
//! cargo run -p evo-daemon --example measure_fixture_retrieval
//! ```

use evo_semantic::{Embedder, cosine_similarity};
use evo_threads::retrieval::{
    RETRIEVAL_MARGIN, RETRIEVAL_RESTORE_FLOOR, thread_document, thread_document_with_surface,
};
use evo_threads::{Anchor, AnchorKind, Status, ThreadView};
use evo_voice::FastEmbedEmbedder;

use std::collections::BTreeMap;

/// One representative work: what the engine would carry after real use.
///
/// `names` are the witnessed display names (OBS-SURFACE-NAMED), `next_step`
/// is the understanding layer's one-liner, `narrative` its longer sentence —
/// the exact surfaces `voice_facts` and the understanding layer fold into the
/// matching document.
struct Work {
    id: u64,
    key: &'static str,
    names: &'static [&'static str],
    anchors: &'static [&'static str],
    companions: &'static [(&'static str, f64)],
    next_step: &'static str,
    narrative: &'static str,
}

/// A rack a product builder might plausibly carry: distinct domains, some
/// decoys that share a category word (investor-facing, document, paper).
const WORKS: &[Work] = &[
    Work {
        id: 1,
        key: "pitch",
        names: &["Evo seed deck"],
        anchors: &["file:///Users/fixture/Desktop/Evo/pitch/Evo-seed-deck.key"],
        companions: &[("file:///Users/fixture/Desktop/Evo/pitch/README.md", 180.0)],
        next_step: "Unsaved changes in Evo-seed-deck",
        narrative: "Observed activity for Evo seed deck across 1 recorded episode. Next: Unsaved changes in Evo-seed-deck",
    },
    Work {
        id: 2,
        key: "investor",
        names: &["Investor update – March"],
        anchors: &["https://docs.example.com/document/d/1xKQm4rt/edit"],
        companions: &[],
        next_step: "Unsaved changes in 1xKQm4rt",
        narrative: "Observed activity for Investor update – March across 1 recorded episode. Next: Unsaved changes in 1xKQm4rt",
    },
    Work {
        id: 3,
        key: "benchmark",
        names: &["Benchmark arena"],
        anchors: &["file:///Users/fixture/evo/tools/bench/arena.py"],
        companions: &[("https://arena.example.com/leaderboard", 240.0)],
        next_step: "Continue in arena",
        narrative: "Observed activity for Benchmark arena across 2 recorded episodes. Next: Continue in arena",
    },
    Work {
        id: 4,
        key: "website",
        names: &["OmniRush landing page"],
        anchors: &["https://omnirush.example/"],
        companions: &[("file:///Users/fixture/omnirush/site/index.html", 200.0)],
        next_step: "Unsaved changes in index.html",
        narrative: "Observed activity for OmniRush landing page across 1 recorded episode. Next: Unsaved changes in index.html",
    },
    Work {
        id: 5,
        key: "paper",
        names: &["Retrieval paper draft"],
        anchors: &["https://www.overleaf.com/project/6712f0aa5d13b90001234567"],
        companions: &[],
        next_step: "Continue in 6712f0aa5d13b90001234567",
        narrative: "Observed activity for Retrieval paper draft across 1 recorded episode. Next: Continue in 6712f0aa5d13b90001234567",
    },
    Work {
        id: 6,
        key: "banner",
        names: &["Custom email banner for investor outreach"],
        anchors: &["https://claude.ai/chat/9fae2b"],
        companions: &[],
        next_step: "Continue in 9fae2b",
        narrative: "Observed activity for Custom email banner for investor outreach across 1 recorded episode. Next: Continue in 9fae2b",
    },
    Work {
        id: 7,
        key: "quantum",
        names: &["Quantum L08 — entanglement"],
        anchors: &["file:///Users/fixture/school/quantum/L08-entanglement.pdf"],
        companions: &[],
        next_step: "Continue in L08-entanglement",
        narrative: "Observed activity for Quantum L08 — entanglement across 1 recorded episode. Next: Continue in L08-entanglement",
    },
    Work {
        id: 8,
        key: "wake",
        names: &["evo-voice wake.rs"],
        anchors: &["file:///Users/fixture/evo/crates/evo-voice/src/wake.rs"],
        companions: &[("file:///Users/fixture/evo/crates/evo-voice/src/driver.rs", 160.0)],
        next_step: "Uncommitted changes in evo-voice",
        narrative: "Observed activity for evo-voice wake.rs across 2 recorded episodes. Next: Uncommitted changes in evo-voice",
    },
];

/// What routing the phrase must get. A phrase restores without asking only
/// when its words actually name the work; a vague phrase without the
/// witnessed words must instead *lead the carousel* with the intended work —
/// nearest first is the routing; opening it would be the guess.
enum Expect {
    /// Clear words: clears the floor and the margin and opens this work.
    Restore(&'static str),
    /// Vague words: must not open anything, and the carousel's first entry
    /// must be this work.
    CarouselFirst(&'static str),
    /// Deixis and controls: must not open anything; no claim about order.
    Carousel,
}

struct Phrase {
    text: &'static str,
    expect: Expect,
    note: &'static str,
}

const PHRASES: &[Phrase] = &[
    Phrase { text: "the pitch thing", expect: Expect::CarouselFirst("pitch"), note: "vague noun; one pitch work" },
    Phrase { text: "that pitch deck I was still working on", expect: Expect::Restore("pitch"), note: "clear" },
    Phrase { text: "that investor update I was drafting", expect: Expect::Restore("investor"), note: "near-verbatim" },
    Phrase { text: "the thing I was putting together for the investors", expect: Expect::Carousel, note: "vague; two investor-facing works" },
    Phrase { text: "that doc I had open", expect: Expect::Carousel, note: "deixis; several doc-like works" },
    Phrase { text: "what I was looking at earlier", expect: Expect::Carousel, note: "pure deixis" },
    Phrase { text: "the benchmark thing", expect: Expect::CarouselFirst("benchmark"), note: "vague noun; unique domain word" },
    Phrase { text: "that website I was tweaking", expect: Expect::CarouselFirst("website"), note: "vague; one site work" },
    Phrase { text: "the banner chat", expect: Expect::Restore("banner"), note: "topic + surface" },
    Phrase { text: "the wake word code", expect: Expect::Restore("wake"), note: "near-verbatim" },
    Phrase { text: "that paper I was writing", expect: Expect::CarouselFirst("paper"), note: "vague; paper draft vs lecture notes" },
    Phrase { text: "the overleaf paper", expect: Expect::Restore("paper"), note: "host" },
    Phrase { text: "order a pizza for dinner", expect: Expect::Carousel, note: "control" },
    Phrase { text: "buy running shoes online", expect: Expect::Carousel, note: "control" },
];

/// Candidate floors for the sensitivity table, spanning the old match bar
/// (0.25) through the chosen restore bar (0.40) and above.
const FLOORS: &[f64] = &[0.25, 0.30, 0.35, 0.40, 0.45, 0.50];
const MARGINS: &[f64] = &[0.80, 0.85, 0.90];

/// The production document composition: witnessed names | next step |
/// structural material. Mirrors `facts_from_engine` in `evo-daemon::voice`.
fn production_document(work: &Work, surface: &str) -> String {
    let thread = thread(work);
    let material = if surface.is_empty() {
        thread_document(&thread)
    } else {
        thread_document_with_surface(&thread, &[surface])
    };
    if work.names.is_empty() {
        material
    } else {
        format!("{} | {material}", work.names.join(" | "))
    }
}

fn thread(work: &Work) -> ThreadView {
    ThreadView {
        id: work.id,
        status: Status::Active,
        episodes: Vec::new(),
        anchors: work
            .anchors
            .iter()
            .map(|resource| Anchor {
                resource: resource.to_string(),
                kind: AnchorKind::Mutation,
                strength: 300.0,
            })
            .collect(),
        companions: work
            .companions
            .iter()
            .map(|(resource, weight)| (resource.to_string(), *weight))
            .collect(),
        contested: Vec::new(),
        fork: None,
    }
}

/// The voice trigger's decision over a ranking, before transport encoding:
/// restore only when the best score clears `floor` *and* no runner-up reaches
/// `margin` of it; otherwise the nearest `k` works are the carousel and
/// nothing is executed.
fn route(scored: &[(u64, f64)], floor: f64, margin: f64, k: usize) -> Route {
    let Some((best_id, best)) = scored.first().copied() else {
        return Route::NotFound;
    };
    if best >= floor {
        let contenders: Vec<u64> = scored
            .iter()
            .filter(|(_, score)| *score >= margin * best)
            .map(|(id, _)| *id)
            .collect();
        if contenders.len() == 1 {
            Route::Restore(best_id)
        } else {
            Route::Ask {
                candidates: contenders,
                above_floor: true,
            }
        }
    } else {
        Route::Ask {
            candidates: scored.iter().take(k).map(|(id, _)| *id).collect(),
            above_floor: false,
        }
    }
}

#[derive(PartialEq)]
enum Route {
    Restore(u64),
    Ask { candidates: Vec<u64>, above_floor: bool },
    NotFound,
}

fn main() {
    let embedder = FastEmbedEmbedder::load().expect("installed model");
    println!("model: {}", embedder.profile_name());
    println!(
        "works: {}  phrases: {}  constants: floor={RETRIEVAL_RESTORE_FLOOR} margin={RETRIEVAL_MARGIN}",
        WORKS.len(),
        PHRASES.len(),
    );
    println!();

    let keys: BTreeMap<&str, u64> = WORKS.iter().map(|w| (w.key, w.id)).collect();
    let titles: BTreeMap<u64, &str> = WORKS.iter().map(|w| (w.id, w.key)).collect();
    let embed_documents = |surface: &dyn Fn(&Work) -> &'static str| -> Vec<(u64, Vec<f32>)> {
        WORKS
            .iter()
            .map(|work| {
                let document = production_document(work, surface(work));
                (work.id, embedder.embed_document(&document).expect("embed document"))
            })
            .collect()
    };
    let production = embed_documents(&|work| work.next_step);
    let structural = embed_documents(&|_| "");
    let narrative = embed_documents(&|work| work.narrative);

    let rank = |vector: &[f32], documents: &[(u64, Vec<f32>)]| -> Vec<(u64, f64)> {
        let mut scored: Vec<(u64, f64)> = documents
            .iter()
            .map(|(id, document)| (*id, cosine_similarity(vector, document)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
        scored
    };

    let mut failures = Vec::new();
    let mut summary = Vec::new();
    // (phrase, production ranking) kept for the sensitivity and ablation
    // tables so the query is embedded exactly once per run.
    let mut rankings: Vec<(&Phrase, Vec<(u64, f64)>)> = Vec::new();

    for phrase in PHRASES {
        let vector = embedder.embed_query(phrase.text).expect("embed query");
        let scored = rank(&vector, &production);
        println!("=== {:?}  ({})", phrase.text, phrase.note);
        for (id, score) in scored.iter().take(5) {
            println!(
                "  {score:.4}  {}  (work {id})",
                titles.get(id).copied().unwrap_or("?")
            );
        }
        let decision = route(&scored, RETRIEVAL_RESTORE_FLOOR, RETRIEVAL_MARGIN, 5);
        let ratio = scored.get(1).map(|(_, s)| s / scored[0].1).unwrap_or(0.0);
        println!(
            "  top-2 ratio: {ratio:.2}   decision: {}",
            describe(&decision, &titles)
        );
        let verdict = verdict_of(&phrase.expect, &decision, &keys);
        if !verdict {
            failures.push(phrase.text);
        }
        summary.push((phrase, scored.clone(), decision, ratio, verdict));
        println!(
            "  expected: {:<28} verdict: {}",
            expect_label(&phrase.expect),
            if verdict { "PASS" } else { "FAIL" },
        );
        println!();
        rankings.push((phrase, scored));
    }

    println!("=== summary (production composition) ===");
    println!(
        "{:<52}  {:<10} {:>7}  {:<10} {:>7} {:>5}  {:<44} {}",
        "phrase", "best", "score", "runner-up", "score", "ratio", "decision", "verdict"
    );
    for (phrase, scored, decision, ratio, verdict) in &summary {
        let (best_id, best) = scored[0];
        let (runner_id, runner) = scored.get(1).copied().unwrap_or((0, 0.0));
        println!(
            "{:<52}  {:<10} {:>7.4}  {:<10} {:>7.4} {:>5.2}  {:<44} {}",
            format!("{:?}", phrase.text),
            titles[&best_id],
            best,
            titles.get(&runner_id).copied().unwrap_or("?"),
            runner,
            ratio,
            describe(decision, &titles),
            if *verdict { "PASS" } else { "FAIL" },
        );
    }
    println!();

    // Sensitivity: how many phrases restore the right work, restore the wrong
    // work (misses and false restores), for each floor/margin pair. The pair
    // is chosen from this table plus the interleaving band, never from a
    // single phrase.
    println!("=== floor/margin sensitivity (production composition) ===");
    println!(
        "{:>6} {:>7}   {:>17}   {:>16}   {:>7}",
        "floor", "margin", "correct restores", "missed restores", "false"
    );
    for floor in FLOORS {
        for margin in MARGINS {
            let mut correct = 0;
            let mut missed = 0;
            let mut false_restores = 0;
            for (phrase, scored) in &rankings {
                let decision = route(scored, *floor, *margin, 5);
                match &phrase.expect {
                    Expect::Restore(key) => {
                        if matches!(decision, Route::Restore(id) if keys[*key] == id) {
                            correct += 1;
                        } else {
                            missed += 1;
                        }
                    }
                    Expect::CarouselFirst(_) | Expect::Carousel => {
                        if matches!(decision, Route::Restore(_)) {
                            false_restores += 1;
                        }
                    }
                }
            }
            let mark = if *floor == RETRIEVAL_RESTORE_FLOOR && *margin == RETRIEVAL_MARGIN {
                "  <- chosen"
            } else {
                ""
            };
            println!(
                "{floor:>6.2} {margin:>7.2}   {correct:>17}   {missed:>16}   {false_restores:>7}{mark}"
            );
        }
    }
    println!();

    // Composition ablation: what the one-line next step adds over the bare
    // structural material, and what the longer narrative does to the same
    // decisions. Enrichment must not buy vague phrases a restore.
    println!("=== composition ablation (floor {RETRIEVAL_RESTORE_FLOOR}, margin {RETRIEVAL_MARGIN}) ===");
    println!(
        "{:<44} {:>17} {:>16} {:>7} {:>9}",
        "composition", "correct restores", "missed restores", "false", "mean best"
    );
    for (label, documents) in [
        ("names | next step | material", &production),
        ("names | material", &structural),
        ("names | narrative | material", &narrative),
    ] {
        let mut correct = 0;
        let mut missed = 0;
        let mut false_restores = 0;
        let mut best_sum = 0.0;
        for phrase in PHRASES {
            let vector = embedder.embed_query(phrase.text).expect("embed query");
            let scored = rank(&vector, documents);
            best_sum += scored[0].1;
            let decision = route(&scored, RETRIEVAL_RESTORE_FLOOR, RETRIEVAL_MARGIN, 5);
            match &phrase.expect {
                Expect::Restore(key) => {
                    if matches!(decision, Route::Restore(id) if keys[*key] == id) {
                        correct += 1;
                    } else {
                        missed += 1;
                    }
                }
                Expect::CarouselFirst(_) | Expect::Carousel => {
                    if matches!(decision, Route::Restore(_)) {
                        false_restores += 1;
                    }
                }
            }
        }
        println!(
            "{label:<44} {correct:>17} {missed:>16} {false_restores:>7} {:>9.4}",
            best_sum / PHRASES.len() as f64
        );
    }
    println!();

    if failures.is_empty() {
        println!("all {} phrase verdicts PASS", PHRASES.len());
    } else {
        println!("FAILED phrase verdicts:");
        for failure in &failures {
            println!("  - {failure:?}");
        }
        std::process::exit(1);
    }
}

fn verdict_of(expect: &Expect, decision: &Route, keys: &BTreeMap<&str, u64>) -> bool {
    match expect {
        Expect::Restore(key) => matches!(decision, Route::Restore(id) if keys[key] == *id),
        Expect::CarouselFirst(key) => matches!(
            decision,
            Route::Ask { candidates, .. } if candidates.first() == Some(&keys[key])
        ),
        Expect::Carousel => matches!(decision, Route::Ask { .. }),
    }
}

fn expect_label(expect: &Expect) -> String {
    match expect {
        Expect::Restore(key) => format!("restore {key}"),
        Expect::CarouselFirst(key) => format!("carousel first: {key}"),
        Expect::Carousel => "never a restore".to_string(),
    }
}

fn describe(route: &Route, titles: &BTreeMap<u64, &str>) -> String {
    match route {
        Route::Restore(id) => format!("RESTORE {} (work {id})", titles[id]),
        Route::Ask { candidates, above_floor: true } => format!(
            "ASK (above floor, contested): [{}]",
            candidates.iter().map(|id| titles[id]).collect::<Vec<_>>().join(", ")
        ),
        Route::Ask { candidates, above_floor: false } => format!(
            "ASK (below floor, nearest): [{}]",
            candidates.iter().map(|id| titles[id]).collect::<Vec<_>>().join(", ")
        ),
        Route::NotFound => "NOTFOUND (empty rack)".to_string(),
    }
}
