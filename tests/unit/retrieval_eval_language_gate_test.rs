//! Unit tests for the canonical TSX semantic language gate (084.005-T).
//!
//! `language_of` (semantic gate) previously mapped both `.ts` and `.tsx` to
//! `typescript`, while the indexer stores the canonical `tsx` for `.tsx` files
//! (`file_node.language`, `code_graph::language_from_path`). That split meant
//! `languages = ['tsx']` included TSX functions in the *graph* path but excluded
//! every TSX function from *semantic* eval (and vice versa). The gate now
//! returns canonical `tsx` so one gate applies to both paths.
//!
//! Scenarios (plan verification, ≤3):
//! 1. `languages = ['tsx']` → TSX (`.tsx`) functions are included;
//! 2. `languages = ['typescript']` → `.ts` included, `.tsx` gated out (canonical
//!    id — documented behavior);
//! 3. empty `languages` → all gated in (opt-in unchanged).

use engram::models::Function;
use engram::models::retrieval_eval::RetrievalEvalConfig;
use engram::services::retrieval_eval::{evaluate_semantic, language_of};

/// Build a keyword-only (empty-embedding) function record with a unique
/// docstring so self-retrieval is deterministic and needs no model load.
fn make_fn(id: &str, name: &str, docstring: &str, file_path: &str) -> Function {
    Function {
        id: id.to_owned(),
        name: name.to_owned(),
        file_path: file_path.to_owned(),
        line_start: 1,
        line_end: 2,
        signature: format!("fn {name}()"),
        docstring: Some(docstring.to_owned()),
        body: String::new(),
        body_hash: String::new(),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        embedding: Vec::new(),
        summary: String::new(),
    }
}

/// One `.tsx` and one `.ts` function, each with a unique docstring.
fn ts_and_tsx_corpus() -> Vec<Function> {
    vec![
        make_fn(
            "function:tsx_widget",
            "TsxWidget",
            "tsx unique alpha widget component",
            "src/widget.tsx",
        ),
        make_fn(
            "function:ts_gadget",
            "tsGadget",
            "ts distinct bravo gadget module",
            "src/gadget.ts",
        ),
    ]
}

fn config_with_languages(langs: &[&str]) -> RetrievalEvalConfig {
    RetrievalEvalConfig {
        enabled: true,
        languages: langs.iter().map(|s| (*s).to_owned()).collect(),
        k: 5,
        sample_size: 200,
        ..RetrievalEvalConfig::default()
    }
}

// ── language_of returns the canonical identifier per extension ────────────────

#[test]
fn language_of_returns_canonical_tsx() {
    assert_eq!(
        language_of("src/widget.tsx"),
        "tsx",
        ".tsx must map to canonical `tsx`, matching the indexer's file_node.language"
    );
    assert_eq!(
        language_of("src/gadget.ts"),
        "typescript",
        ".ts must remain `typescript`"
    );
}

// ── Scenario 1: languages = ['tsx'] includes TSX functions ────────────────────

#[test]
fn tsx_gate_includes_tsx_functions() {
    let corpus = ts_and_tsx_corpus();
    let config = config_with_languages(&["tsx"]);

    let metrics = evaluate_semantic(&corpus, &config).expect("semantic eval");

    assert_eq!(
        metrics.queries, 1,
        "languages=['tsx'] must include the single .tsx function (and only it)"
    );
}

// ── Scenario 2: languages = ['typescript'] includes .ts, gates out .tsx ────────

#[test]
fn typescript_gate_excludes_tsx_by_canonical_id() {
    let corpus = ts_and_tsx_corpus();
    let config = config_with_languages(&["typescript"]);

    let metrics = evaluate_semantic(&corpus, &config).expect("semantic eval");

    assert_eq!(
        metrics.queries, 1,
        "languages=['typescript'] must include the .ts function but gate out the \
         .tsx function by its canonical `tsx` id"
    );
}

// ── Scenario 3: empty languages gates all in ──────────────────────────────────

#[test]
fn empty_languages_gates_all_in() {
    let corpus = ts_and_tsx_corpus();
    let config = config_with_languages(&[]);

    let metrics = evaluate_semantic(&corpus, &config).expect("semantic eval");

    assert_eq!(
        metrics.queries, 2,
        "empty languages must include both .ts and .tsx (opt-in gating disabled)"
    );
}
