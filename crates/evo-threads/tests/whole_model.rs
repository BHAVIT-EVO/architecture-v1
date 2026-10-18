//! Synthetic pipeline integration: scripted events through assignment,
//! deterministic-vector retrieval, scoping, and restore-bundle selection.
//! This checks wiring and evidence handling, not real observation accuracy,
//! natural-language model quality, or actual platform restoration.

use evo_semantic::{EmbedError, Embedder};
use evo_threads::retrieval::{retrieve, scope_restore_set, thread_document, Retrieval};
use evo_threads::{Engine, Event, Interaction, Origin};
use std::collections::HashMap;

fn t(min: u64) -> u64 {
    min * 60_000
}

/// A test double for the embedding model: the test declares the semantic
/// relationships explicitly — query Q is "about" document i — and the double
/// returns near-parallel vectors for about-pairs and near-orthogonal vectors
/// otherwise. This pins the pipeline wiring (log → threads → retrieve →
/// scope) while real-model behavior (paraphrase, synonyms) is covered where
/// the actual model runs.
struct FixtureEmbedder {
    vectors: HashMap<String, Vec<f32>>,
}

impl FixtureEmbedder {
    fn new(documents: &[String], about: &[(&str, usize)]) -> Self {
        let dims = documents.len().max(1);
        let mut vectors = HashMap::new();
        for (index, document) in documents.iter().enumerate() {
            let mut vector = vec![0.0f32; dims];
            vector[index] = 1.0;
            vectors.insert(document.clone(), vector);
        }
        for (query, target) in about {
            let mut vector = vec![0.05f32; dims];
            vector[*target] = 0.95;
            vectors.insert(query.to_string(), vector);
        }
        Self { vectors }
    }
}

impl Embedder for FixtureEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.vectors
            .get(text)
            .cloned()
            .ok_or_else(|| EmbedError::Refused(text.into()))
    }
}

/// The thread whose anchors witness `fragment`, as an index into `threads`.
fn thread_about(threads: &[evo_threads::ThreadView], fragment: &str) -> usize {
    threads
        .iter()
        .position(|thread| {
            thread
                .anchors
                .iter()
                .any(|anchor| anchor.resource.contains(fragment))
        })
        .unwrap_or_else(|| panic!("a thread about {fragment:?} must exist"))
}

fn ev(ts: u64, origin: Origin, i: Interaction, r: &str) -> Event {
    Event {
        timestamp: ts,
        origin,
        interaction: i,
        resource: r.into(),
        weight: None,
        detail: None,
    }
}

fn typing(ts: u64, r: &str, w: u64) -> Event {
    Event {
        timestamp: ts,
        origin: Origin::Person,
        interaction: Interaction::Typed,
        resource: r.into(),
        weight: Some(w),
        detail: None,
    }
}

/// A full day of three works sharing a browser, with the exact pattern from
/// the founder's use cases: presentation + chat research + downloaded assets,
/// interleaved with a coding session and a leisure stretch.
fn full_day() -> Vec<Event> {
    let mut log = Vec::new();
    let base = 1_789_000_000_000u64; // a day
    let day = |m: u64| base + t(m);

    // --- The presentation work (morning) ---
    // Downloaded images for the deck (a production anchor).
    log.push(Event {
        timestamp: day(5),
        origin: Origin::Person,
        interaction: Interaction::Downloaded,
        resource: "file:///Users/p/Downloads/hero-image.png".into(),
        weight: None,
        detail: None,
    });
    // Opened the presentation app and worked in it.
    log.push(ev(
        day(10),
        Origin::Person,
        Interaction::Focused,
        "file:///Users/p/Slides/presentation.key",
    ));
    log.push(typing(
        day(11),
        "file:///Users/p/Slides/presentation.key",
        400,
    ));
    log.push(ev(
        day(15),
        Origin::Person,
        Interaction::Saved,
        "file:///Users/p/Slides/presentation.key",
    ));
    log.push(typing(
        day(16),
        "file:///Users/p/Slides/presentation.key",
        300,
    ));
    log.push(ev(
        day(20),
        Origin::Person,
        Interaction::Saved,
        "file:///Users/p/Slides/presentation.key",
    ));

    // A chat surface receives actual typing during the presentation sitting.
    // Typing is production evidence even when no save is observed; the fixture
    // does not establish the semantic purpose of those keystrokes.
    log.push(ev(
        day(25),
        Origin::Person,
        Interaction::Visited,
        "https://chatgpt.com/c/presentation-research",
    ));
    log.push(typing(
        day(26),
        "https://chatgpt.com/c/presentation-research",
        200,
    ));
    log.push(ev(
        day(35),
        Origin::Person,
        Interaction::Focused,
        "https://chatgpt.com/c/presentation-research",
    ));

    // Google search tabs (finished supporting work — must NOT reopen).
    // Early in the sitting: the research is done, the person moved to
    // assembly, and the searches were left 40+ minutes before the end.
    log.push(ev(
        day(15),
        Origin::Person,
        Interaction::Visited,
        "https://www.google.com/search?q=presentation+design+tips",
    ));
    log.push(ev(
        day(18),
        Origin::Person,
        Interaction::Visited,
        "https://www.google.com/search?q=slide+layouts",
    ));
    log.push(ev(
        day(20),
        Origin::Person,
        Interaction::Focused,
        "file:///Users/p/Slides/presentation.key",
    ));
    // The assembly continues well past the searches.
    log.push(typing(
        day(30),
        "file:///Users/p/Slides/presentation.key",
        200,
    ));
    log.push(ev(
        day(35),
        Origin::Person,
        Interaction::Saved,
        "file:///Users/p/Slides/presentation.key",
    ));
    log.push(typing(
        day(50),
        "file:///Users/p/Slides/presentation.key",
        200,
    ));
    log.push(ev(
        day(55),
        Origin::Person,
        Interaction::Saved,
        "file:///Users/p/Slides/presentation.key",
    ));

    // --- The coding session (afternoon, same browser) ---
    log.push(ev(
        day(80),
        Origin::Person,
        Interaction::Focused,
        "file:///Users/p/evo/src/main.rs",
    ));
    log.push(typing(day(81), "file:///Users/p/evo/src/main.rs", 500));
    log.push(ev(
        day(85),
        Origin::Person,
        Interaction::Saved,
        "file:///Users/p/evo/src/main.rs",
    ));
    log.push(typing(day(86), "file:///Users/p/evo/src/main.rs", 400));
    log.push(ev(
        day(90),
        Origin::Person,
        Interaction::Saved,
        "file:///Users/p/evo/src/main.rs",
    ));
    // Browser docs consulted mid-flow.
    log.push(ev(
        day(92),
        Origin::Person,
        Interaction::Visited,
        "https://docs.example.com/rust/threads",
    ));
    log.push(ev(
        day(100),
        Origin::Person,
        Interaction::Focused,
        "file:///Users/p/evo/src/main.rs",
    ));
    log.push(typing(day(101), "file:///Users/p/evo/src/main.rs", 300));
    log.push(ev(
        day(105),
        Origin::Person,
        Interaction::Saved,
        "file:///Users/p/evo/src/main.rs",
    ));

    // --- Leisure (must never become a work) ---
    log.push(Event {
        timestamp: day(200),
        origin: Origin::Person,
        interaction: Interaction::Focused,
        resource: "https://youtube.com/watch?v=funny".into(),
        weight: Some(300_000),
        detail: None,
    });
    log.push(Event {
        timestamp: day(210),
        origin: Origin::Person,
        interaction: Interaction::Focused,
        resource: "https://youtube.com/watch?v=funny".into(),
        weight: Some(300_000),
        detail: None,
    });
    log.push(Event {
        timestamp: day(220),
        origin: Origin::Person,
        interaction: Interaction::Focused,
        resource: "https://youtube.com/watch?v=funny".into(),
        weight: Some(300_000),
        detail: None,
    });

    // --- The AI chat session (evening — the Z AI example) ---
    log.push(ev(
        day(300),
        Origin::Person,
        Interaction::Visited,
        "https://zai.app/chat/evo-architecture",
    ));
    log.push(typing(
        day(301),
        "https://zai.app/chat/evo-architecture",
        500,
    ));
    log.push(ev(
        day(310),
        Origin::Person,
        Interaction::Focused,
        "https://zai.app/chat/evo-architecture",
    ));
    log.push(typing(
        day(311),
        "https://zai.app/chat/evo-architecture",
        400,
    ));

    // End of day: a brief messaging reflex (the WhatsApp trap).
    log.push(typing(day(400), "app:messaging", 30));

    log
}

#[test]
fn whole_model_presentation_work() {
    let log = full_day();
    let mut engine = Engine::replay(&log);
    engine.settle(log.last().unwrap().timestamp + 86_400_000);

    let threads = engine.threads();

    // Find the presentation thread by name.
    let documents: Vec<String> = threads.iter().map(thread_document).collect();
    let embedder = FixtureEmbedder::new(
        &documents,
        &[("presentation", thread_about(&threads, "presentation"))],
    );
    let retrieval =
        retrieve("presentation", &threads, &embedder).expect("the fixture embeds everything");
    let thread_id = match &retrieval {
        Retrieval::Matched { thread, .. } => *thread,
        other => panic!("'presentation' must match a thread, got {other:?}"),
    };

    let bundle = engine.resume_bundle(thread_id);

    // The resume point is the presentation itself, not the messaging reflex.
    assert!(
        bundle.resume_point.contains("presentation.key"),
        "resume point must be the presentation, got: {}",
        bundle.resume_point
    );

    // The restore set includes the presentation and the download the person
    // needs for it — and does NOT include the finished Google searches.
    assert!(
        bundle
            .restore_set
            .iter()
            .any(|r| r.contains("presentation.key")),
        "the presentation must be in the restore set: {:?}",
        bundle.restore_set
    );
    assert!(
        bundle.restore_set.iter().any(|r| r.contains("hero-image")),
        "the downloaded image must be in the restore set: {:?}",
        bundle.restore_set
    );
    assert!(
        !bundle
            .restore_set
            .iter()
            .any(|r| r.contains("google.com/search")),
        "finished Google searches must NOT be in the restore set: {:?}",
        bundle.restore_set
    );
    assert!(
        !bundle.restore_set.iter().any(|r| r.contains("youtube")),
        "leisure must never enter a work's restore set: {:?}",
        bundle.restore_set
    );
}

#[test]
fn whole_model_coding_work() {
    let log = full_day();
    let mut engine = Engine::replay(&log);
    engine.settle(log.last().unwrap().timestamp + 86_400_000);
    let threads = engine.threads();

    let documents: Vec<String> = threads.iter().map(thread_document).collect();
    let embedder = FixtureEmbedder::new(
        &documents,
        &[("evo main", thread_about(&threads, "main.rs"))],
    );
    let retrieval =
        retrieve("evo main", &threads, &embedder).expect("the fixture embeds everything");
    let thread_id = match &retrieval {
        Retrieval::Matched { thread, .. } => *thread,
        other => panic!("'evo main' must match a thread, got {other:?}"),
    };

    let bundle = engine.resume_bundle(thread_id);
    assert!(
        bundle.resume_point.contains("main.rs"),
        "resume point must be the source file, got: {}",
        bundle.resume_point
    );
    assert!(
        !bundle
            .restore_set
            .iter()
            .any(|r| r.contains("presentation")),
        "the coding work's restore set must not include the presentation: {:?}",
        bundle.restore_set
    );
}

#[test]
fn whole_model_scoped_restore_just_the_chat() {
    let log = full_day();
    let mut engine = Engine::replay(&log);
    engine.settle(log.last().unwrap().timestamp + 86_400_000);
    let threads = engine.threads();

    // "just the zai chat" — scoped to one surface within a work.
    let zai_thread = thread_about(&threads, "zai.app");
    let bundle = engine.resume_bundle(threads[zai_thread].id);
    let chat_at = bundle
        .restore_set
        .iter()
        .position(|resource| resource.contains("zai.app"))
        .expect("the chat surface must be in the bundle to scope to it");
    let embedder = FixtureEmbedder::new(&bundle.restore_set, &[("just the zai chat", chat_at)]);
    let scoped = scope_restore_set(&bundle, "just the zai chat", &embedder)
        .expect("the fixture embeds everything");
    // The scoped set must contain only the chat surface.
    assert!(
        scoped
            .iter()
            .all(|r| r.contains("zai") || r.contains("chat")),
        "scoped restore must only include the chat surface, got: {scoped:?}"
    );
    assert!(
        !scoped.is_empty(),
        "if the chat is in the bundle, scoping to it must find it"
    );
}

#[test]
fn whole_model_leisure_never_becomes_work() {
    let log = full_day();
    let mut engine = Engine::replay(&log);
    engine.settle(log.last().unwrap().timestamp + 86_400_000);
    let threads = engine.threads();

    for thread in &threads {
        for anchor in &thread.anchors {
            assert!(
                !anchor.resource.contains("youtube"),
                "leisure must never be an anchor of any work: {}",
                anchor.resource
            );
        }
    }
}

#[test]
fn whole_model_messaging_never_hijacks_resume() {
    let log = full_day();
    let mut engine = Engine::replay(&log);
    engine.settle(log.last().unwrap().timestamp + 86_400_000);
    let threads = engine.threads();

    for thread in &threads {
        let bundle = engine.resume_bundle(thread.id);
        assert_ne!(
            bundle.resume_point, "app:messaging",
            "a messaging reflex must never be any work's resume point"
        );
    }
}

#[test]
fn whole_model_two_works_sharing_a_browser_stay_separate() {
    let log = full_day();
    let mut engine = Engine::replay(&log);
    engine.settle(log.last().unwrap().timestamp + 86_400_000);
    let threads = engine.threads();

    // There must be at least two distinct works (presentation and coding),
    // and they must not have merged.
    let presentation = threads.iter().find(|t| {
        t.anchors
            .iter()
            .any(|a| a.resource.contains("presentation"))
    });
    let coding = threads
        .iter()
        .find(|t| t.anchors.iter().any(|a| a.resource.contains("main.rs")));
    let presentation = presentation.expect("the presentation work must exist");
    let coding = coding.expect("the separately produced source-file work must exist");
    assert_ne!(
        presentation.id, coding.id,
        "independent works must not fuse"
    );
}
