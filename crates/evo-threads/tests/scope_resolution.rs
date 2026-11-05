//! Public scope contract, with deterministic vectors and no model/runtime I/O.
use evo_semantic::{EmbedError, Embedder};
use evo_threads::retrieval::{resolve_scope, scope_restore_set, ScopeResolution};
use evo_threads::ResumeBundle;
use std::collections::BTreeMap;

struct Stub(BTreeMap<String, Vec<f32>>);

impl Embedder for Stub {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.0
            .get(text)
            .cloned()
            .ok_or_else(|| EmbedError::Refused("stub model failure".into()))
    }
}

fn bundle(resources: &[&str]) -> ResumeBundle {
    ResumeBundle {
        resume_point: resources.first().copied().unwrap_or("").into(),
        resume_reason: "scope test".into(),
        restore_set: resources
            .iter()
            .map(|resource| (*resource).into())
            .collect(),
        open_deltas: vec![],
    }
}

fn semantic_miss(bundle: &ResumeBundle, query: &str) -> Stub {
    let mut vectors: BTreeMap<String, Vec<f32>> = bundle
        .restore_set
        .iter()
        .map(|resource| (resource.clone(), vec![1.0, 0.0]))
        .collect();
    vectors.insert(query.into(), vec![0.0, 1.0]);
    Stub(vectors)
}

fn check(bundle: &ResumeBundle, query: &str, model: &dyn Embedder, expected: ScopeResolution) {
    assert_eq!(resolve_scope(bundle, query, model).unwrap(), expected);
    let executable = scope_restore_set(bundle, query, model).unwrap();
    match expected {
        ScopeResolution::Unscoped { resources } | ScopeResolution::Matched { resources } => {
            assert_eq!(executable, resources);
        }
        ScopeResolution::Ambiguous { .. } | ScopeResolution::NoMatch => {
            assert!(executable.is_empty(), "unresolved scope must not execute");
        }
    }
    assert!(executable.iter().all(|r| bundle.restore_set.contains(r)));
    if !query.trim().is_empty() {
        assert!(executable.len() <= 1, "scope must never widen");
    }
}

#[test]
fn unscoped_order_keeps_resume_point_first_and_deduplicates_first_occurrences() {
    let bundle = bundle(&[
        "file:///z-resume",
        "file:///z-resume",
        "file:///b",
        "file:///a",
        "file:///b",
        "file:///z-resume",
    ]);
    assert_eq!(bundle.resume_point, "file:///z-resume");
    let failing = Stub(BTreeMap::new());
    for query in ["", " \n\t "] {
        check(
            &bundle,
            query,
            &failing,
            ScopeResolution::Unscoped {
                resources: vec![
                    "file:///z-resume".into(),
                    "file:///b".into(),
                    "file:///a".into(),
                ],
            },
        );
    }
}

#[test]
fn raw_full_locator_preserves_trailing_spaces_before_trim_without_embedding() {
    struct MustNotEmbed;
    impl Embedder for MustNotEmbed {
        fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
            panic!("exact locator matching must not call the embedder")
        }
    }
    let resources = [
        "file:///work/report",
        "file:///work/report ",
        "file:///work/report  ",
        "/work/report",
        "/work/report ",
    ];
    let mut bundle = bundle(&resources);
    for _ in 0..resources.len() {
        for locator in resources {
            check(
                &bundle,
                locator,
                &MustNotEmbed,
                ScopeResolution::Matched {
                    resources: vec![locator.into()],
                },
            );
        }
        bundle.restore_set.rotate_left(1);
    }
}

#[test]
fn full_locator_precedes_model_and_preserves_case_sensitive_identity() {
    let bundle = bundle(&[
        "https://example.test/Report?view=A",
        "https://example.test/report?view=A",
        "https://example.test/Report?view=B",
        "https://example.test/Report?view=A",
    ]);
    let failing = Stub(BTreeMap::new());
    check(
        &bundle,
        "  https://example.test/Report?view=A  ",
        &failing,
        ScopeResolution::Matched {
            resources: vec!["https://example.test/Report?view=A".into()],
        },
    );
}

#[test]
fn semantic_match_is_not_made_ambiguous_by_duplicate_locators() {
    let bundle = bundle(&["file:///a", "file:///b", "file:///a"]);
    let model = Stub(BTreeMap::from([
        ("file:///a".into(), vec![1.0, 0.0]),
        ("file:///b".into(), vec![0.0, 1.0]),
        ("the first surface".into(), vec![0.95, 0.31]),
    ]));
    check(
        &bundle,
        "the first surface",
        &model,
        ScopeResolution::Matched {
            resources: vec!["file:///a".into()],
        },
    );
}

#[test]
fn semantic_near_ties_remain_alternatives_even_when_a_substring_favors_one() {
    let bundle = bundle(&["file:///alpha", "file:///beta", "file:///other"]);
    let model = Stub(BTreeMap::from([
        ("file:///alpha".into(), vec![1.0, 0.0]),
        ("file:///beta".into(), vec![0.95, 0.31]),
        ("file:///other".into(), vec![0.0, 1.0]),
        ("alpha".into(), vec![1.0, 0.0]),
    ]));
    check(
        &bundle,
        "alpha",
        &model,
        ScopeResolution::Ambiguous {
            candidates: vec!["file:///alpha".into(), "file:///beta".into()],
        },
    );
}

#[test]
fn misses_and_common_url_tokens_never_widen() {
    let bundle = bundle(&[
        "https://example.test/chat/alpha",
        "https://example.test/chat/beta",
        "https://example.test/chat/gamma",
    ]);
    for query in [
        "unseen-surface",
        "https",
        "https://",
        "example.test",
        "chat",
        "test",
        "/",
        "://",
        "https example.test chat",
    ] {
        check(
            &bundle,
            query,
            &semantic_miss(&bundle, query),
            ScopeResolution::NoMatch,
        );
    }
    // A scheme remains syntax even when only one resource uses it.
    let mixed = self::bundle(&["https://example.test/alpha", "file:///beta"]);
    check(
        &mixed,
        "https",
        &semantic_miss(&mixed, "https"),
        ScopeResolution::NoMatch,
    );
}

#[test]
fn substring_fallback_requires_unique_support_and_ignores_repetition() {
    let bundle = bundle(&[
        "https://example.test/alpha-extra",
        "https://example.test/beta",
        "https://example.test/gamma",
        "https://example.test/alpha-extra",
    ]);
    let query = "example.test alpha alpha";
    check(
        &bundle,
        query,
        &semantic_miss(&bundle, query),
        ScopeResolution::Matched {
            resources: vec!["https://example.test/alpha-extra".into()],
        },
    );
    let query = "alpha alpha alpha extra beta";
    check(
        &bundle,
        query,
        &semantic_miss(&bundle, query),
        ScopeResolution::Ambiguous {
            candidates: vec![
                "https://example.test/alpha-extra".into(),
                "https://example.test/beta".into(),
            ],
        },
    );
}

#[test]
fn empty_and_singleton_restore_sets_do_not_manufacture_substring_evidence() {
    let empty = bundle(&[]);
    check(
        &empty,
        "anything",
        &Stub(BTreeMap::new()),
        ScopeResolution::NoMatch,
    );
    let singleton = bundle(&["https://example.test/alpha", "https://example.test/alpha"]);
    for query in ["https", "example.test", "alpha", "missing"] {
        check(
            &singleton,
            query,
            &semantic_miss(&singleton, query),
            ScopeResolution::NoMatch,
        );
    }
}

#[test]
fn scoped_outcomes_are_independent_of_input_permutation() {
    let resources = ["file:///alpha", "file:///beta", "file:///gamma"];
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let original = bundle(&resources);
    for query in ["file:///alpha", "alpha", "alpha beta", "missing", "file"] {
        let model = semantic_miss(&original, query);
        let expected = resolve_scope(&original, query, &model).unwrap();
        for permutation in permutations {
            let permuted = bundle(&[
                resources[permutation[0]],
                resources[permutation[1]],
                resources[permutation[2]],
                resources[permutation[0]],
            ]);
            check(&permuted, query, &model, expected.clone());
        }
    }
    let model = Stub(BTreeMap::from([
        (resources[0].into(), vec![1.0, 0.0]),
        (resources[1].into(), vec![0.95, 0.31]),
        (resources[2].into(), vec![0.0, 1.0]),
        ("near tie".into(), vec![1.0, 0.0]),
        ("unique".into(), vec![0.0, 1.0]),
    ]));
    for query in ["near tie", "unique"] {
        let expected = resolve_scope(&original, query, &model).unwrap();
        for permutation in permutations {
            let permuted = bundle(&permutation.map(|i| resources[i]));
            check(&permuted, query, &model, expected.clone());
        }
    }
}

#[test]
fn model_failures_and_invalid_vectors_never_enable_substring_fallback() {
    let bundle = bundle(&["file:///alpha", "file:///beta"]);
    let missing_document = Stub(BTreeMap::new());
    let mut missing_query = semantic_miss(&bundle, "alpha");
    missing_query.0.remove("alpha");
    for model in [missing_document, missing_query] {
        assert!(matches!(
            resolve_scope(&bundle, "alpha", &model),
            Err(EmbedError::Refused(_))
        ));
        assert!(matches!(
            scope_restore_set(&bundle, "alpha", &model),
            Err(EmbedError::Refused(_))
        ));
    }
    for text in ["file:///alpha", "alpha"] {
        for vector in [
            vec![],
            vec![0.0, 0.0],
            vec![f32::NAN, 1.0],
            vec![f32::INFINITY, 1.0],
        ] {
            let mut model = semantic_miss(&bundle, "alpha");
            model.0.insert(text.into(), vector);
            assert!(matches!(
                resolve_scope(&bundle, "alpha", &model),
                Err(EmbedError::Refused(_))
            ));
            assert!(scope_restore_set(&bundle, "alpha", &model).is_err());
        }
    }
}
