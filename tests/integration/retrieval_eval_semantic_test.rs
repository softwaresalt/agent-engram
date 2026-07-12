//! Integration tests for semantic self-retrieval eval compute (081.004-T).
//!
//! Exercises the four plan scenarios for the semantic slice:
//! 1. known-item hit@1 → MRR = 1.0 (and recall/nDCG = 1.0);
//! 2. injected miss → recall@k drops below 1.0;
//! 3. nDCG rewards correct ordering (rank 1 beats rank 3);
//! 4. empty / disabled corpus → zeroed report (no panic).

use engram::models::Function;
use engram::models::retrieval_eval::{RetrievalEvalConfig, RetrievalMode};
use engram::services::retrieval_eval::{compute_semantic_metrics, evaluate_semantic};

/// Build a synthetic function record with an empty embedding so search stays
/// keyword-only (deterministic, no model load).
fn make_fn(id: &str, name: &str, signature: &str, docstring: &str, file_path: &str) -> Function {
    Function {
        id: id.to_owned(),
        name: name.to_owned(),
        file_path: file_path.to_owned(),
        line_start: 1,
        line_end: 2,
        signature: signature.to_owned(),
        docstring: if docstring.is_empty() {
            None
        } else {
            Some(docstring.to_owned())
        },
        body: String::new(),
        body_hash: String::new(),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        embedding: Vec::new(),
        summary: String::new(),
    }
}

/// A small corpus where each function's docstring uniquely identifies itself.
fn distinct_corpus() -> Vec<Function> {
    vec![
        make_fn(
            "function:alpha",
            "alpha",
            "fn alpha()",
            "alpha unique widget assembly",
            "src/alpha.rs",
        ),
        make_fn(
            "function:bravo",
            "bravo",
            "fn bravo()",
            "bravo distinct gadget compilation",
            "src/bravo.rs",
        ),
        make_fn(
            "function:charlie",
            "charlie",
            "fn charlie()",
            "charlie separate sprocket rendering",
            "src/charlie.rs",
        ),
    ]
}

#[test]
fn known_item_hit_at_one_gives_perfect_metrics() {
    let corpus = distinct_corpus();
    let config = RetrievalEvalConfig {
        enabled: true,
        languages: vec!["rust".to_owned()],
        k: 5,
        sample_size: 200,
        ..RetrievalEvalConfig::default()
    };

    let metrics = evaluate_semantic(&corpus, &config).expect("semantic eval");

    assert_eq!(metrics.queries, 3, "all three symbols should be evaluated");
    assert!(
        (metrics.recall_at_k - 1.0).abs() < 1e-9,
        "expected recall 1.0, got {}",
        metrics.recall_at_k
    );
    assert!(
        (metrics.mrr - 1.0).abs() < 1e-9,
        "expected MRR 1.0, got {}",
        metrics.mrr
    );
    assert!(
        (metrics.ndcg - 1.0).abs() < 1e-9,
        "expected nDCG 1.0, got {}",
        metrics.ndcg
    );
}

#[test]
fn injected_miss_drops_recall() {
    // Two hits and one miss → recall = 2/3.
    let ranks = [Some(1_usize), None, Some(2_usize)];
    let metrics = compute_semantic_metrics(&ranks, 5);

    assert_eq!(metrics.queries, 3);
    assert!(
        (metrics.recall_at_k - (2.0 / 3.0)).abs() < 1e-9,
        "expected recall 2/3, got {}",
        metrics.recall_at_k
    );
    assert!(
        metrics.recall_at_k < 1.0,
        "recall must drop below 1.0 on a miss"
    );
}

#[test]
fn ndcg_rewards_better_ordering() {
    let better = compute_semantic_metrics(&[Some(1_usize)], 10);
    let worse = compute_semantic_metrics(&[Some(3_usize)], 10);

    assert!(
        better.ndcg > worse.ndcg,
        "rank-1 nDCG {} should exceed rank-3 nDCG {}",
        better.ndcg,
        worse.ndcg
    );
    assert!(
        (better.ndcg - 1.0).abs() < 1e-9,
        "rank-1 nDCG should be 1.0"
    );
}

#[test]
fn empty_corpus_yields_zero_report() {
    let config = RetrievalEvalConfig {
        enabled: true,
        ..RetrievalEvalConfig::default()
    };

    let from_eval = evaluate_semantic(&[], &config).expect("empty eval");
    assert_eq!(from_eval.queries, 0);
    assert!((from_eval.mrr - 0.0).abs() < 1e-9);
    assert!((from_eval.recall_at_k - 0.0).abs() < 1e-9);

    let from_compute = compute_semantic_metrics(&[], 10);
    assert_eq!(from_compute.queries, 0);
    assert!((from_compute.precision_at_k - 0.0).abs() < 1e-9);
    assert!((from_compute.ndcg - 0.0).abs() < 1e-9);
}

// ── 084.008-T: retrieval-mode fidelity / silent keyword-only fallback ────────
//
// `hybrid_search` swallows `embed_text(query).ok()`, so a keyword-only run —
// either an un-embedded corpus or a corpus with vectors but an unusable
// embedding model — was previously reported as if it were true hybrid
// retrieval (00C7F3CC). `evaluate_semantic` now records the mode it actually
// exercised so reports are comparable and a broken embedding path cannot
// masquerade as a passing hybrid run.

/// An enabled config over the default (`["rust"]`) language scope.
fn enabled_config() -> RetrievalEvalConfig {
    RetrievalEvalConfig {
        enabled: true,
        ..RetrievalEvalConfig::default()
    }
}

/// The [`distinct_corpus`] but with every function carrying a non-empty
/// embedding vector, so `hybrid_search` exercises the embedding (KNN) path.
fn embedded_corpus() -> Vec<Function> {
    distinct_corpus()
        .into_iter()
        .map(|mut f| {
            f.embedding = vec![0.125_f32; 384];
            f
        })
        .collect()
}

#[test]
fn un_embedded_corpus_records_keyword_only() {
    // `make_fn` builds functions with empty embeddings: no candidate carries a
    // vector, so the embedding path is never exercised.
    let metrics = evaluate_semantic(&distinct_corpus(), &enabled_config()).expect("semantic eval");
    assert_eq!(
        metrics.retrieval_mode,
        RetrievalMode::KeywordOnly,
        "an un-embedded corpus must record a keyword-only fallback, not hybrid"
    );
}

#[test]
fn embedded_corpus_records_hybrid() {
    // Vectors present + a usable query-embedding model → the hybrid path runs.
    let metrics = evaluate_semantic(&embedded_corpus(), &enabled_config()).expect("semantic eval");
    assert_eq!(
        metrics.retrieval_mode,
        RetrievalMode::Hybrid,
        "a corpus carrying vectors with a usable embedding model must record hybrid"
    );
}

#[test]
fn modes_are_distinguishable_across_runs() {
    let config = enabled_config();
    let keyword = evaluate_semantic(&distinct_corpus(), &config)
        .expect("keyword eval")
        .retrieval_mode;
    let hybrid = evaluate_semantic(&embedded_corpus(), &config)
        .expect("hybrid eval")
        .retrieval_mode;
    assert_ne!(
        keyword, hybrid,
        "keyword-only and hybrid runs must be distinguishable in the report"
    );
    assert_eq!(keyword, RetrievalMode::KeywordOnly);
    assert_eq!(hybrid, RetrievalMode::Hybrid);
}

#[test]
fn empty_corpus_records_unknown_mode() {
    // Nothing is retrieved against an empty corpus, so the mode stays unknown
    // rather than claiming a keyword-only or hybrid run.
    let metrics = evaluate_semantic(&[], &enabled_config()).expect("empty eval");
    assert_eq!(
        metrics.retrieval_mode,
        RetrievalMode::Unknown,
        "an empty corpus retrieves nothing, so the mode stays unknown"
    );
}
