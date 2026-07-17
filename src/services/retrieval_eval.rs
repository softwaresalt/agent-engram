//! Retrieval-evaluation compute (081-F).
//!
//! Semantic self-retrieval: each indexed function's docstring (falling back to
//! its name) becomes a known-item query whose single expected hit is that same
//! function. Running those queries through the hybrid ranker
//! ([`crate::services::search::hybrid_rank_of`]) and recording the rank of the
//! source function yields precision@k, recall@k, MRR and nDCG@k.
//!
//! The semantic corpus is scoped to **functions** in this baseline. Extending
//! the candidate/query abstraction to other indexed symbol kinds (classes,
//! interfaces) is tracked follow-up work.
//!
//! Ground truth is auto-derived (no manual labels): the query and its expected
//! answer both come from the same indexed symbol, so the corpus is entirely
//! self-describing. Self-retrieval is a *proxy* signal — it rewards name/doc
//! recall — and is therefore reported as one signal alongside the graph
//! resolution metrics rather than as a standalone quality score.
//!
//! This module is intentionally distinct from the agent-efficiency
//! [`crate::services::evaluation`] surface. The two subsystems measure different
//! things (`retrieval_eval` vs `evaluation`) and must not be conflated.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;

use sha2::{Digest, Sha256};

use crate::errors::{ConfigError, EngramError, SystemError};
use crate::models::CodeFile;
use crate::models::Function;
use crate::models::retrieval_eval::{
    GraphMetrics, RetrievalEvalConfig, RetrievalEvalReport, RetrievalEvalThresholds, RetrievalMode,
    SemanticMetrics,
};
use crate::services::code_graph;
use crate::services::parsing::{ExtractedEdge, Language, canonical, parse_source};
use crate::services::search::{SearchCandidate, hybrid_rank_of};

/// Maximum bytes retained from a derived known-item query.
///
/// Well under [`crate::services::embedding::MAX_QUERY_CHARS`] so a long
/// docstring never trips the query-length guard.
const MAX_QUERY_BYTES: usize = 1900;

/// Map a file path's extension to a coarse language identifier.
///
/// Used to gate the semantic corpus by [`RetrievalEvalConfig::languages`].
/// Delegates to [`crate::services::code_graph::language_from_path`] — the single
/// canonical mapping the indexer uses to populate `file_node.language` — so the
/// semantic gate and the graph gate share exactly one vocabulary with no drift
/// (e.g. `.tsx`→`tsx`, `.h++`→`cpp`, `.sql`→`sql`, `.md`→`markdown`). This is the
/// generalization of the TSX fix (084.005-T): any extension the indexer
/// recognizes gates identically in both paths. Unrecognized extensions fall
/// through to the raw extension, and a path with no extension is `"unknown"`.
#[must_use]
pub fn language_of(path: &str) -> String {
    crate::services::code_graph::language_from_path(std::path::Path::new(path))
}

/// Trim and byte-bound a raw query string on a UTF-8 char boundary.
fn bound_query(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() <= MAX_QUERY_BYTES {
        return trimmed.to_owned();
    }
    let mut end = MAX_QUERY_BYTES;
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    trimmed[..end].to_owned()
}

/// Derive a known-item query for a function symbol.
///
/// Prefers the first non-empty line of the docstring (natural-language intent),
/// falling back to the function name when no docstring is present. The result is
/// trimmed and length-bounded.
///
/// Matching is by function id (not by name), so a non-unique fallback name only
/// makes the ranking harder (a fair recall signal); it never resolves to the
/// wrong symbol.
#[must_use]
pub fn derive_query(function: &Function) -> String {
    let raw = match function.docstring.as_deref() {
        Some(doc) if !doc.trim().is_empty() => doc
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("")
            .to_owned(),
        _ => function.name.clone(),
    };
    bound_query(&raw)
}

/// Build a search candidate from a function record.
///
/// Content combines the symbol name, signature and docstring so a known-item
/// query derived from any of those fields can retrieve the symbol. The
/// embedding is included only when present, so an un-embedded corpus stays
/// keyword-only (no model load).
fn func_to_candidate(function: &Function) -> SearchCandidate {
    let content = format!(
        "{} {} {}",
        function.name,
        function.signature,
        function.docstring.as_deref().unwrap_or("")
    );
    SearchCandidate {
        id: function.id.clone(),
        source_type: "function".to_owned(),
        content,
        embedding: if function.embedding.is_empty() {
            None
        } else {
            Some(function.embedding.clone())
        },
        title: Some(function.name.clone()),
        file_path: Some(function.file_path.clone()),
        line_range: None,
        record_kind: Some("function".to_owned()),
        heading_path: Vec::new(),
        fallback_reason: None,
        lint_summary: None,
        suggestions: Vec::new(),
    }
}

/// Compute semantic self-retrieval metrics from per-query ranks.
///
/// `ranks[i]` is the 1-based rank of query `i`'s known item within the top `k`
/// results, or `None` when the item did not appear. Each query has exactly one
/// relevant document, so the metrics reduce to their single-relevant forms:
///
/// * `recall_at_k` = hits ÷ queries;
/// * `precision_at_k` = mean of `1/k` over hits;
/// * `mrr` = mean of `1/rank` over hits;
/// * `ndcg` = mean of `1/log2(rank+1)` over hits (ideal DCG is `1`).
///
/// Returns a zeroed [`SemanticMetrics`] (with `queries = 0`) for empty input.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_semantic_metrics(ranks: &[Option<usize>], k: usize) -> SemanticMetrics {
    let queries = ranks.len();
    if queries == 0 {
        return SemanticMetrics::default();
    }
    let n = queries as f64;
    let k_f = if k == 0 { 1.0 } else { k as f64 };

    let mut hits = 0.0_f64;
    let mut precision = 0.0_f64;
    let mut mrr = 0.0_f64;
    let mut ndcg = 0.0_f64;
    for rank in ranks.iter().flatten() {
        let r = *rank as f64;
        hits += 1.0;
        precision += 1.0 / k_f;
        mrr += 1.0 / r;
        ndcg += 1.0 / (r + 1.0).log2();
    }

    SemanticMetrics {
        precision_at_k: precision / n,
        recall_at_k: hits / n,
        mrr: mrr / n,
        ndcg: ndcg / n,
        queries,
        // Mode is a property of *how* the corpus was searched (embeddings
        // present or not), which this rank-only aggregate cannot observe.
        // `evaluate_semantic` records the effective mode (084.008-T).
        retrieval_mode: RetrievalMode::Unknown,
    }
}

/// Resolve the retrieval mode a semantic run actually exercised (084.008-T).
///
/// Mirrors [`hybrid_search`]'s embedding-path condition exactly: the embedding
/// (KNN) component contributes only when the corpus carries at least one vector
/// **and** the query-embedding model can embed a query. When the corpus carries
/// vectors but the query cannot be embedded, `hybrid_search` swallows the
/// failure (`embed_text(query).ok()` → `None`) and silently scores keyword-only;
/// this records that as [`RetrievalMode::KeywordOnly`] rather than masking a
/// broken embedding path as a passing hybrid run (00C7F3CC). An empty corpus
/// retrieves nothing, so its mode stays [`RetrievalMode::Unknown`].
#[must_use]
fn resolve_retrieval_mode(
    corpus_empty: bool,
    corpus_has_vectors: bool,
    query_embeds: bool,
) -> RetrievalMode {
    if corpus_empty {
        RetrievalMode::Unknown
    } else if corpus_has_vectors && query_embeds {
        RetrievalMode::Hybrid
    } else {
        RetrievalMode::KeywordOnly
    }
}

/// Evaluate semantic self-retrieval over a function corpus.
///
/// The corpus is first gated by [`RetrievalEvalConfig::languages`] (an empty
/// list disables gating). A search candidate is built per surviving function; a
/// known-item query is derived per sampled function (up to
/// [`RetrievalEvalConfig::sample_size`]); [`hybrid_search`] ranks the corpus and
/// the rank of the source symbol is recorded. Ranks aggregate into
/// [`SemanticMetrics`]. Functions with an empty derived query are skipped.
///
/// # Errors
/// Returns an error if [`hybrid_search`] rejects a derived query (e.g. it
/// exceeds the query-length budget).
pub fn evaluate_semantic(
    functions: &[Function],
    config: &RetrievalEvalConfig,
) -> Result<SemanticMetrics, EngramError> {
    let k = config.k.max(1);

    let selected: Vec<&Function> = {
        let mut v: Vec<&Function> = functions
            .iter()
            .filter(|f| {
                if config.languages.is_empty() {
                    return true;
                }
                let lang_id = language_of(&f.file_path);
                config
                    .languages
                    .iter()
                    .any(|lang| lang.eq_ignore_ascii_case(&lang_id))
            })
            .collect();
        // Deterministic order (stable source identity) so the same unchanged
        // workspace yields the same sample and the same metrics regardless of
        // database row order — this report is used as a regression gate.
        v.sort_by(|a, b| {
            (a.file_path.as_str(), a.line_start, a.name.as_str()).cmp(&(
                b.file_path.as_str(),
                b.line_start,
                b.name.as_str(),
            ))
        });
        v
    };

    let candidates: Vec<SearchCandidate> = selected.iter().map(|f| func_to_candidate(f)).collect();

    let mut ranks: Vec<Option<usize>> = Vec::new();
    // Track whether EVERY ranked query actually exercised the embedding path.
    // The reported retrieval mode is derived from this aggregate over the real
    // ranking calls (084.008-T fidelity), not a separate constant probe: a mode
    // computed from the executed rankings can never contradict the scores that
    // produced the metrics. Starts `true` (vacuous) and is cleared by the first
    // keyword-only fallback; only meaningful once at least one query ranked.
    let mut all_queries_embedded = true;
    for function in selected.iter().take(config.sample_size) {
        let query = derive_query(function);
        if query.is_empty() {
            continue;
        }
        // Bounded known-item probe: the rank of the source symbol within the
        // top-k, computed without cloning or sorting the full candidate set
        // (084.010-T). Identical to `hybrid_search(&query, &candidates, k)` then
        // locating `function.id`, but O(1) extra space per query.
        let outcome = hybrid_rank_of(&query, &candidates, &function.id, k)?;
        ranks.push(outcome.rank);
        all_queries_embedded &= outcome.embedded;
    }

    let mut metrics = compute_semantic_metrics(&ranks, k);
    // Record the retrieval mode actually exercised so a keyword-only fallback
    // (un-embedded corpus, or vectors present but the query embedding failing on
    // one or more actual queries) is never masked as hybrid (00C7F3CC /
    // 084.008-T). Mode is only meaningful when at least one known-item query was
    // ranked; when none ran (empty corpus, or `sample_size == 0` so the loop
    // never executed) report `Unknown` rather than a mode no query influenced.
    // The mode reflects the embedding path the executed rankings truly took —
    // `all_queries_embedded` aggregates each `hybrid_rank_of`'s own result — so
    // a per-query embedding failure downgrades the report to the honest
    // KeywordOnly fallback instead of a speculative probe that never ran.
    metrics.retrieval_mode = if metrics.queries == 0 {
        RetrievalMode::Unknown
    } else {
        let corpus_has_vectors = candidates.iter().any(|c| c.embedding.is_some());
        resolve_retrieval_mode(
            candidates.is_empty(),
            corpus_has_vectors,
            all_queries_embedded,
        )
    };
    Ok(metrics)
}

/// Count identifier / path call sites in a source string.
///
/// This is the graph metric *denominator*: the parser call-site inventory
/// (`ExtractedEdge::Calls`) discovered by [`parse_source`], restricted to the
/// calls the indexer can actually resolve. Free-function and path calls,
/// path-qualified calls (`a::b()`), and known-receiver `self.method()` calls
/// are all counted: canonical resolution (Option C Unit B) stages and resolves
/// qualified and `self`-receiver calls, so they are resolvable and belong in
/// the denominator. Only method calls with an *arbitrary* receiver (`x.foo()`,
/// where the receiver is not `self`) are excluded — their receiver type is
/// unknown at parse time, so name-only resolution cannot reach them. This
/// mirrors the inclusion rule in the body: `!is_method || is_qualified ||
/// raw_qualifier == "self"`. Blocklisted helpers (`clone`, `unwrap`, …) are
/// excluded by the parser. A parse failure yields `0`.
///
/// Counts **distinct `(caller, callee, raw_qualifier, qualifier_kind)` call-site
/// identities**, not raw call occurrences (084.002-T / 88B5FAFD; 091.012-T). The
/// numerator is a count of `calls_edge` rows keyed by `(from, to)` node IDs, and
/// canonical resolution (Option C Unit B) can produce **several distinct edges
/// that share a bare callee name**: `crate::a::build()` and `crate::b::build()`
/// resolve to two different targets, and `self::foo()` (module) vs `self.foo()`
/// (method) resolve to two more. Keying the denominator on the same
/// qualifier-aware identity the `staged_call` key uses keeps `resolution_recall`
/// a ratio of commensurable units: a repeated call to the identical target still
/// contributes one unit (no spurious deflation), while two same-named calls to
/// *different* targets contribute two — so recall no longer over-reports a
/// perfect `1.0` when only one of them actually resolved.
///
/// This source-only helper has no access to resolved graph IDs, so it preserves
/// the fail-closed syntactic denominator. Runtime eval uses
/// [`scan_call_site_inventory_with_resolution`] to collapse only spellings proven
/// to share the same resolved `(caller_id, target_id)` numerator edge.
#[must_use]
pub fn count_call_sites(source: &str, language: Language) -> usize {
    parse_source(source, language).map_or(0, |result| {
        let mut relations: std::collections::HashSet<(&str, &str, &str, &str)> =
            std::collections::HashSet::new();
        for edge in &result.edges {
            if let ExtractedEdge::Calls {
                caller,
                callee,
                is_method,
                is_qualified,
                raw_qualifier,
                qualifier_kind,
                ..
            } = edge
            {
                if !*is_method || *is_qualified || raw_qualifier == "self" {
                    relations.insert((
                        caller.as_str(),
                        callee.as_str(),
                        raw_qualifier.as_str(),
                        qualifier_kind.as_str(),
                    ));
                }
            }
        }
        relations.len()
    })
}

/// Resolved graph identity data used to make the graph-recall denominator
/// commensurable with the `(from, to)` numerator.
#[derive(Debug, Clone, Default)]
pub struct CallSiteResolutionContext {
    /// Indexed functions in the evaluated workspace.
    pub functions: Vec<Function>,
    /// Resolved `calls_edge` pairs keyed by `(caller_id, callee_id)`.
    pub resolved_edges: HashSet<(String, String)>,
    /// Non-empty canonical function identities keyed to matching function IDs.
    pub canonical_index: HashMap<String, Vec<String>>,
    /// Complete index-time canonical workspace context persisted with the graph edge set.
    pub canonical_workspace: Option<canonical::CanonicalWorkspace>,
}

impl CallSiteResolutionContext {
    /// Build a resolution context from the same indexed graph rows used by the
    /// resolution-recall numerator.
    #[must_use]
    pub fn new(
        functions: Vec<Function>,
        resolved_edges: HashSet<(String, String)>,
        canonical_index: HashMap<String, Vec<String>>,
        canonical_workspace: Option<canonical::CanonicalWorkspace>,
    ) -> Self {
        Self {
            functions,
            resolved_edges,
            canonical_index,
            canonical_workspace,
        }
    }
}

#[derive(Debug, Clone)]
struct ResolutionLookup {
    function_ids_by_file_and_name: HashMap<(String, String), Vec<String>>,
    function_ids_by_name: HashMap<String, Vec<String>>,
    canonical_path_by_id: HashMap<String, String>,
    resolved_edges: HashSet<(String, String)>,
    canonical_index: HashMap<String, Vec<String>>,
}

impl ResolutionLookup {
    fn from_context(context: &CallSiteResolutionContext) -> Option<Self> {
        if context.functions.is_empty() || context.resolved_edges.is_empty() {
            return None;
        }

        let mut function_ids_by_file_and_name: HashMap<(String, String), Vec<String>> =
            HashMap::new();
        let mut function_ids_by_name: HashMap<String, Vec<String>> = HashMap::new();
        for function in &context.functions {
            function_ids_by_file_and_name
                .entry((function.file_path.clone(), function.name.clone()))
                .or_default()
                .push(function.id.clone());
            function_ids_by_name
                .entry(function.name.clone())
                .or_default()
                .push(function.id.clone());
        }

        let mut canonical_path_by_id = HashMap::new();
        for (canonical_path, ids) in &context.canonical_index {
            if ids.len() == 1 {
                canonical_path_by_id.insert(ids[0].clone(), canonical_path.clone());
            }
        }

        Some(Self {
            function_ids_by_file_and_name,
            function_ids_by_name,
            canonical_path_by_id,
            resolved_edges: context.resolved_edges.clone(),
            canonical_index: context.canonical_index.clone(),
        })
    }

    fn unique_caller_id(&self, source_file: &str, caller: &str) -> Option<&str> {
        let ids = self
            .function_ids_by_file_and_name
            .get(&(source_file.to_owned(), caller.to_owned()))?;
        if ids.len() == 1 {
            Some(ids[0].as_str())
        } else {
            None
        }
    }

    fn edge_target_if_present<'a>(&self, caller_id: &str, target_id: &'a str) -> Option<&'a str> {
        if self
            .resolved_edges
            .contains(&(caller_id.to_owned(), target_id.to_owned()))
        {
            Some(target_id)
        } else {
            None
        }
    }

    fn bare_target_id(&self, source_file: &str, caller_id: &str, callee: &str) -> Option<&str> {
        // Bare-call collapse must mirror the production resolver's proof path,
        // not merely observe a same-named edge. A same-named qualified edge can
        // coexist with an unresolved ambiguous bare call; collapsing that would
        // overstate recall. Only collapse when the in-file direct target or the
        // workspace-global singleton target is unique and that exact `(from,to)`
        // edge exists in the numerator identity space.
        if let Some(ids) = self
            .function_ids_by_file_and_name
            .get(&(source_file.to_owned(), callee.to_owned()))
        {
            if ids.len() == 1 {
                let target_id = ids[0].as_str();
                if self.edge_target_if_present(caller_id, target_id).is_some() {
                    return Some(target_id);
                }
            }
        }

        let ids = self.function_ids_by_name.get(callee)?;
        if ids.len() == 1 {
            self.edge_target_if_present(caller_id, ids[0].as_str())
        } else {
            None
        }
    }

    fn canonical_target_id(
        &self,
        caller_id: &str,
        target: &canonical::CanonicalId,
    ) -> Option<&str> {
        let target = target.clone().into_string();
        let ids = self.canonical_index.get(&target)?;
        if ids.len() != 1 {
            return None;
        }
        let target_id = ids[0].as_str();
        if self
            .resolved_edges
            .contains(&(caller_id.to_owned(), target_id.to_owned()))
        {
            Some(target_id)
        } else {
            None
        }
    }

    fn enclosing_canonical_type(&self, caller_id: &str) -> Option<&str> {
        self.canonical_path_by_id
            .get(caller_id)?
            .rsplit_once("::")
            .map(|(ty, _)| ty)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CallSiteDenominatorKey {
    Resolved {
        caller_id: String,
        target_id: String,
    },
    Syntax {
        source_file: String,
        caller: String,
        callee: String,
        raw_qualifier: String,
        qualifier_kind: String,
    },
}

#[derive(Debug, Clone, Copy)]
struct ParsedCallSite<'a> {
    source_file: &'a str,
    caller: &'a str,
    callee: &'a str,
    is_method: bool,
    is_qualified: bool,
    raw_qualifier: &'a str,
    qualifier_kind: &'a str,
}

fn visible_call_site(site: ParsedCallSite<'_>) -> bool {
    !site.is_method || site.is_qualified || site.raw_qualifier == "self"
}

fn rust_canonical_ctx(
    crates: &canonical::WorkspaceCrates,
    lang: Language,
    rel_path: &str,
    source: &str,
) -> Option<(canonical::ModulePath, canonical::UseGraph)> {
    if lang != Language::Rust {
        return None;
    }
    let module = canonical::module_path_for_file(crates, rel_path)?;
    Some((module, canonical::extract_use_graph(source)))
}

fn canonical_target_for_call(
    lookup: &ResolutionLookup,
    workspace: &canonical::CanonicalWorkspace,
    module: &canonical::ModulePath,
    use_graph: &canonical::UseGraph,
    caller_id: &str,
    site: ParsedCallSite<'_>,
) -> Option<String> {
    if use_graph.has_nested_use() || use_graph.has_non_default_mod_mapping() {
        return None;
    }

    let ctx = canonical::ResolveContext {
        module,
        crates: &workspace.crates,
        use_graph,
    };
    let target = match site.qualifier_kind {
        "module" | "type" => canonical::resolve_qualifier(
            &ctx,
            None,
            &canonical::Qualifier::Path(site.raw_qualifier.to_owned()),
            &[site.callee],
        ),
        "self" if site.raw_qualifier == "Self" => {
            let enclosing = lookup.enclosing_canonical_type(caller_id)?;
            canonical::resolve_qualifier(
                &ctx,
                Some(enclosing),
                &canonical::Qualifier::SelfType,
                &[site.callee],
            )
        }
        "method" if site.raw_qualifier == "self" => {
            let enclosing = lookup.enclosing_canonical_type(caller_id)?;
            canonical::resolve_qualifier(
                &ctx,
                Some(enclosing),
                &canonical::Qualifier::SelfType,
                &[site.callee],
            )
        }
        _ => None,
    }?;

    let target_path = target.clone().into_string();
    if code_graph::is_under_unsafe_module_prefix(&target_path, &workspace.unsafe_prefixes) {
        None
    } else {
        lookup
            .canonical_target_id(caller_id, &target)
            .map(str::to_owned)
    }
}

fn resolved_target_id_for_call(
    lookup: &ResolutionLookup,
    canonical_ctx: Option<&(canonical::ModulePath, canonical::UseGraph)>,
    workspace: Option<&canonical::CanonicalWorkspace>,
    site: ParsedCallSite<'_>,
) -> Option<String> {
    let caller_id = lookup.unique_caller_id(site.source_file, site.caller)?;
    if !site.is_method && !site.is_qualified {
        return lookup
            .bare_target_id(site.source_file, caller_id, site.callee)
            .map(str::to_owned);
    }

    let (Some((module, use_graph)), Some(workspace)) = (canonical_ctx, workspace) else {
        return None;
    };
    canonical_target_for_call(lookup, workspace, module, use_graph, caller_id, site)
}

fn syntax_key(site: ParsedCallSite<'_>) -> CallSiteDenominatorKey {
    CallSiteDenominatorKey::Syntax {
        source_file: site.source_file.to_owned(),
        caller: site.caller.to_owned(),
        callee: site.callee.to_owned(),
        raw_qualifier: site.raw_qualifier.to_owned(),
        qualifier_kind: site.qualifier_kind.to_owned(),
    }
}

fn count_call_sites_resolution_aware(
    source_file: &str,
    source: &str,
    language: Language,
    lookup: &ResolutionLookup,
    workspace: Option<&canonical::CanonicalWorkspace>,
) -> usize {
    parse_source(source, language).map_or(0, |result| {
        let canonical_ctx =
            workspace.and_then(|w| rust_canonical_ctx(&w.crates, language, source_file, source));
        let mut relations: HashSet<CallSiteDenominatorKey> = HashSet::new();
        for edge in &result.edges {
            if let ExtractedEdge::Calls {
                caller,
                callee,
                is_method,
                is_qualified,
                raw_qualifier,
                qualifier_kind,
                ..
            } = edge
            {
                let site = ParsedCallSite {
                    source_file,
                    caller,
                    callee,
                    is_method: *is_method,
                    is_qualified: *is_qualified,
                    raw_qualifier,
                    qualifier_kind,
                };
                if visible_call_site(site) {
                    if let Some(target_id) =
                        resolved_target_id_for_call(lookup, canonical_ctx.as_ref(), workspace, site)
                    {
                        if let Some(caller_id) = lookup.unique_caller_id(source_file, caller) {
                            relations.insert(CallSiteDenominatorKey::Resolved {
                                caller_id: caller_id.to_owned(),
                                target_id,
                            });
                            continue;
                        }
                    }
                    relations.insert(syntax_key(site));
                }
            }
        }
        relations.len()
    })
}

/// SHA-256 hex digest of a source string.
///
/// Computed identically to the code-graph indexer's `sha256_hex` (same bytes,
/// same lowercase-hex encoding) so an eval-time re-read can be compared
/// byte-for-byte against the `content_hash` recorded in `file_node` at index
/// time (084.003-T). Must stay in lockstep with the indexer's hashing.
#[must_use]
pub fn source_content_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// True when an indexed file's freshly-read `source` no longer matches the
/// `recorded_hash` captured at index time — the working tree has drifted from
/// the indexed revision (084.003-T).
///
/// The graph recall numerator is read from the indexed edges while the
/// denominator is re-parsed from disk each run; a drift means the two describe
/// different revisions, so recall is surfaced with an honest `index_stale`
/// signal instead of silently relying on the `[0, 1]` clamp to hide the
/// inconsistency. An empty `recorded_hash` (legacy `file_node` row with no
/// stored hash) disables the check for that file rather than reporting a false
/// positive.
#[must_use]
pub(crate) fn is_index_stale(source: &str, recorded_hash: &str) -> bool {
    !recorded_hash.is_empty() && source_content_hash(source) != recorded_hash
}

/// Outcome of scanning the indexed source inventory for the graph denominator.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallSiteInventory {
    /// Distinct-relation call-site count (the resolution-recall denominator).
    pub call_sites: usize,
    /// True when any indexed file's on-disk content no longer matches the hash
    /// recorded at index time — the working tree has drifted from the indexed
    /// revision, so the numerator (indexed edges) and this freshly-read
    /// denominator describe different revisions. Surfaced instead of being
    /// hidden by the recall `[0, 1]` clamp (084.003-T).
    pub index_stale: bool,
    /// Count of indexed files that could not be read this run (missing /
    /// unreadable) and were therefore excluded from the denominator scan —
    /// accounted, not silently dropped, so a shrunken denominator is visible.
    pub unreadable_files: usize,
}

/// Number of indexed source files read + parsed per batch during the denominator
/// scan (084.011-T / CA401F5F).
///
/// The scan holds at most this many file sources in memory at once, then parses
/// the batch off-runtime and drops it before reading the next — so peak memory is
/// bounded by one batch rather than by the entire indexed corpus. The batch size
/// does not affect the counts (parsing is per-file and the totals sum).
const CALL_SITE_SCAN_BATCH_FILES: usize = 64;

/// Sum the distinct call-site relations across one batch of `(source_file,
/// language, source, recorded_hash)` tuples and report whether any source has
/// drifted from its recorded hash (084.011-T).
///
/// Pure and synchronous so it can run off the async runtime one batch at a time.
/// An unparseable language contributes nothing; the stale flag OR-accumulates so
/// a single drifted file in any batch surfaces the working-tree drift signal.
fn accumulate_call_sites(
    batch: &[(String, String, String, String)],
    resolution: Option<&ResolutionLookup>,
    canonical_workspace: Option<&canonical::CanonicalWorkspace>,
) -> (usize, bool) {
    let mut total: usize = 0;
    let mut stale = false;
    for (source_file, lang, source, recorded_hash) in batch {
        let file_stale = is_index_stale(source, recorded_hash);
        let file_fresh_for_resolution = !recorded_hash.is_empty() && !file_stale;

        // Working-tree drift: the freshly-read content no longer matches the hash
        // recorded at index time, so this denominator and the indexed numerator
        // describe different revisions.
        if file_stale {
            stale = true;
        }
        if let Ok(language) = Language::try_from(lang.as_str()) {
            total += if file_fresh_for_resolution {
                resolution.map_or_else(
                    || count_call_sites(source, language),
                    |lookup| {
                        count_call_sites_resolution_aware(
                            source_file,
                            source,
                            language,
                            lookup,
                            canonical_workspace,
                        )
                    },
                )
            } else {
                // Per-file canonical inputs (`ModulePath` + `UseGraph`) are
                // reparsed from the caller source. Only use them when the file's
                // index-time hash is known and still matches; stale or legacy
                // unknown-hash files fall back to syntax-only counting so a live
                // alias/import drift cannot collapse an index-time miss.
                count_call_sites(source, language)
            };
        }
    }
    (total, stale)
}

fn language_gated(file: &CodeFile, languages: &[String]) -> bool {
    languages.is_empty()
        || languages
            .iter()
            .any(|lang| lang.eq_ignore_ascii_case(&file.language))
}

/// Scan the indexed source inventory for the graph-metric denominator and its
/// index-consistency signals (084.003-T).
///
/// For each indexed `file` whose language passes the `languages` gate
/// (case-insensitive; empty ⇒ all), the file is re-read from disk under
/// `workspace_path` and parsed to count distinct `(caller, callee)` call
/// relations (the denominator). While scanning it surfaces two honest signals:
/// * `index_stale` — a re-read file's content no longer matches the
///   `content_hash` recorded at index time (working-tree drift);
/// * `unreadable_files` — indexed files that could not be resolved/read this run
///   (missing / unreadable), counted rather than silently dropped so a shrunken
///   denominator — and therefore an unreliable recall — stays visible.
///
/// A path that resolves outside `workspace_path` (e.g. an in-workspace symlink
/// repointed outside the root after indexing) stays blocked by the traversal
/// guard and **is** accounted as unreadable — exactly like an indexed file that
/// no longer resolves — so a shrunken denominator stays visible instead of
/// silently inflating recall. A read/resolve failure of an indexed file is
/// likewise accounted.
///
/// Files are read and parsed in bounded batches of
/// [`CALL_SITE_SCAN_BATCH_FILES`] so peak memory stays bounded by one batch
/// rather than the entire indexed corpus (084.011-T). Batching does not change
/// the counts: parsing is per-file and both the call-site total and the
/// `index_stale` OR aggregate across batches.
///
/// # Errors
/// Returns a system error if the workspace root cannot be canonicalized, or if
/// an off-runtime parse task panics — a failed read must not be silently
/// reported as an empty inventory.
pub async fn scan_call_site_inventory(
    workspace_path: &Path,
    files: &[CodeFile],
    languages: &[String],
) -> Result<CallSiteInventory, EngramError> {
    scan_call_site_inventory_inner(workspace_path, files, languages, None).await
}

/// Scan the indexed source inventory using resolved graph identity to collapse
/// only call spellings proven to share the same `(caller_id, target_id)` edge.
///
/// The reconciliation is intentionally fail-closed: resolved spellings use the
/// exact post-resolution edge identity counted by the numerator, while any call
/// site that cannot be mapped to an existing edge remains keyed by its
/// qualifier-aware syntax. That can under-report when proof is unavailable, but
/// it cannot merge a genuinely missed distinct target and over-state recall.
///
/// # Errors
/// Returns a system error if the workspace root cannot be canonicalized, or if
/// an off-runtime parse task panics.
pub async fn scan_call_site_inventory_with_resolution(
    workspace_path: &Path,
    files: &[CodeFile],
    languages: &[String],
    resolution: &CallSiteResolutionContext,
) -> Result<CallSiteInventory, EngramError> {
    scan_call_site_inventory_inner(workspace_path, files, languages, Some(resolution)).await
}

async fn scan_call_site_inventory_inner(
    workspace_path: &Path,
    files: &[CodeFile],
    languages: &[String],
    resolution: Option<&CallSiteResolutionContext>,
) -> Result<CallSiteInventory, EngramError> {
    // Canonical workspace root for the containment check below. The bound
    // workspace is canonicalized at `set_workspace` time, so this is expected
    // to succeed; a failure is surfaced rather than silently skewing metrics.
    let ws_root = tokio::fs::canonicalize(workspace_path).await.map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("cannot resolve workspace root for eval containment check: {e}"),
        })
    })?;

    // Without a persisted full index-time canonical workspace snapshot,
    // resolution-aware collapse is disabled entirely. Syntax-only counting is
    // the safe upgrade/drift fallback: it can under-report but cannot hide a
    // qualified miss by recomputing a smaller live-disk context than the one
    // that produced the stored edges.
    let canonical_workspace = resolution.and_then(|context| context.canonical_workspace.clone());
    let resolution_lookup = if canonical_workspace.is_some() {
        resolution.and_then(ResolutionLookup::from_context)
    } else {
        None
    };
    let canonical_workspace = resolution_lookup.as_ref().and(canonical_workspace);

    // Running totals accumulated one bounded batch at a time so the whole corpus
    // of source text is never resident simultaneously (084.011-T).
    let mut call_sites: usize = 0;
    let mut index_stale = false;
    // Indexed files that could not be read this run. Accounted (084.003-T) so a
    // denominator computed over fewer files than were indexed — and therefore an
    // unreliable recall — is visible rather than silently masked.
    let mut unreadable_files: usize = 0;
    // Current batch: (source_file, language, source, recorded_hash) for readable,
    // in-scope indexed files. Bounded to CALL_SITE_SCAN_BATCH_FILES entries.
    let mut batch: Vec<(String, String, String, String)> =
        Vec::with_capacity(CALL_SITE_SCAN_BATCH_FILES);

    for file in files {
        if !language_gated(file, languages) {
            continue;
        }
        let full = workspace_path.join(&file.path);
        // Workspace-isolation invariant (no traversal): resolve the target
        // (following symlinks and `..`) and require it to stay under the
        // canonical workspace root before reading.
        let Ok(canon) = tokio::fs::canonicalize(&full).await else {
            // An indexed file that no longer resolves (deleted / renamed since
            // index time) is an unreadable indexed file, not a silent skip.
            unreadable_files += 1;
            continue;
        };
        if !canon.starts_with(&ws_root) {
            // A path escaping the workspace (e.g. an in-workspace symlink
            // repointed outside the root after indexing) stays blocked — but it
            // is still an indexed file now excluded from the denominator, so
            // account it as unreadable (084.003-T), exactly like a file that no
            // longer resolves. Silently skipping it would shrink the denominator
            // while the persisted `calls` edges remain in the numerator,
            // inflating resolution_recall with no consistency signal.
            unreadable_files += 1;
            continue;
        }
        match tokio::fs::read_to_string(&canon).await {
            Ok(source) => batch.push((
                file.path.clone(),
                file.language.clone(),
                source,
                file.content_hash.clone(),
            )),
            Err(_) => unreadable_files += 1,
        }

        if batch.len() >= CALL_SITE_SCAN_BATCH_FILES {
            let (count, stale) = parse_call_site_batch(
                std::mem::take(&mut batch),
                resolution_lookup.clone(),
                canonical_workspace.clone(),
            )
            .await?;
            call_sites += count;
            index_stale |= stale;
            batch.reserve(CALL_SITE_SCAN_BATCH_FILES);
        }
    }

    // Flush the final partial batch (may be empty for an all-gated-out corpus).
    if !batch.is_empty() {
        let (count, stale) =
            parse_call_site_batch(batch, resolution_lookup, canonical_workspace).await?;
        call_sites += count;
        index_stale |= stale;
    }

    Ok(CallSiteInventory {
        call_sites,
        index_stale,
        unreadable_files,
    })
}

/// Parse one batch of source tuples off the async runtime (parsing + hash
/// verification are CPU-bound), returning the batch's call-site total and stale
/// flag. Takes ownership of the batch so it is dropped once parsed (084.011-T).
///
/// # Errors
/// Returns a system error if the off-runtime parse task panics.
async fn parse_call_site_batch(
    batch: Vec<(String, String, String, String)>,
    resolution: Option<ResolutionLookup>,
    canonical_workspace: Option<canonical::CanonicalWorkspace>,
) -> Result<(usize, bool), EngramError> {
    tokio::task::spawn_blocking(move || {
        accumulate_call_sites(&batch, resolution.as_ref(), canonical_workspace.as_ref())
    })
    .await
    .map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("retrieval eval call-site parse task failed: {e}"),
        })
    })
}

/// Compute graph resolution metrics from raw counts.
///
/// * `resolution_recall` = resolved ÷ visible call sites, clamped to `[0, 1]`
///   (`0.0` when there are no call sites);
/// * `false_edge_rate` = false edges ÷ resolved (`0.0` when nothing resolved).
///
/// `resolution_recall` is clamped because `resolved` (distinct edges in the
/// graph) and `call_sites` (raw parser occurrences re-counted from disk) are
/// gathered independently and could momentarily disagree.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_graph_metrics(call_sites: usize, resolved: u64, false_edges: u64) -> GraphMetrics {
    let resolution_recall = if call_sites == 0 {
        0.0
    } else {
        (resolved as f64 / call_sites as f64).clamp(0.0, 1.0)
    };
    let false_edge_rate = if resolved == 0 {
        0.0
    } else {
        (false_edges as f64 / resolved as f64).clamp(0.0, 1.0)
    };
    GraphMetrics {
        resolution_recall,
        false_edge_rate,
        call_sites,
        resolved,
        false_edges,
        // Staleness / accounting (084.003-T) and target-correctness (084.004-T)
        // are populated by the callers that have the required inputs (indexed
        // generation, unreadable-file count, expected-target manifest). This
        // raw-count constructor leaves them at their honest defaults.
        index_stale: false,
        unreadable_files: 0,
        target_correct: 0,
        target_mismatch: 0,
    }
}

/// Outcome of comparing produced resolved `calls` edges (singleton AND canonical)
/// against a ground-truth expected-target manifest by EXACT identity (084.004-T).
///
/// Unlike `false_edge_rate` — a DANGLING-only lower bound that is blind to
/// mis-resolution to an existing-but-wrong function
/// (`2026-07-08-callgraph-cross-file-resolution-deliberation.md:25-33`) — these
/// counts are measured against a supplied manifest and therefore also catch
/// wrong-but-existing targets. Produced only on the fixture / regression path
/// (production runs have no manifest); maps onto [`GraphMetrics::target_correct`]
/// and [`GraphMetrics::target_mismatch`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TargetCorrectness {
    /// Produced resolved edges present in the manifest (correct callee identity).
    pub target_correct: u64,
    /// Produced resolved edges absent from the manifest (wrong-but-existing or
    /// dangling) — the gap the dangling-only `false_edge_rate` cannot see.
    pub target_mismatch: u64,
}

/// Compare produced resolved `calls` edges against an expected-target manifest
/// by EXACT `(caller_id, callee_id)` identity (084.004-T).
///
/// The check is resolution-CLASS-AGNOSTIC: it scores whatever produced edges it
/// is given by identity alone. The eval harness MUST therefore feed it BOTH
/// `calls_resolved_singleton` AND `calls_resolved_canonical` edges — otherwise a
/// wrong-but-existing CANONICAL edge would silently escape the gate (M4).
///
/// `target_correct` counts produced edges present in the manifest;
/// `target_mismatch` counts produced edges ABSENT from it — i.e. resolved to a
/// wrong-but-existing function, or dangling. This closes the correctness gap
/// left by `false_edge_rate`, which counts dangling callees only and therefore
/// cannot observe mis-resolution to an existing-but-incorrect definition
/// (`2026-07-08-callgraph-cross-file-resolution-deliberation.md:25-33`). It does
/// NOT measure recall (expected edges the resolver failed to produce); that
/// remains `resolution_recall`'s and the dangling aggregate's concern.
#[must_use]
pub fn evaluate_target_correctness(
    produced_edges: &[(String, String)],
    expected_manifest: &HashSet<(String, String)>,
) -> TargetCorrectness {
    let mut target_correct = 0u64;
    let mut target_mismatch = 0u64;
    for edge in produced_edges {
        if expected_manifest.contains(edge) {
            target_correct += 1;
        } else {
            target_mismatch += 1;
        }
    }
    TargetCorrectness {
        target_correct,
        target_mismatch,
    }
}

// ── Persistence (081.006-T) ──────────────────────────────────────────────

/// Wrap a filesystem error as an [`EngramError`] with contextual detail.
fn io_err(context: &str, error: &std::io::Error) -> EngramError {
    EngramError::System(SystemError::DatabaseError {
        reason: format!("retrieval eval {context}: {error}"),
    })
}

/// Sanitize a branch name for use as a single filesystem path component.
///
/// Mirrors the branch sanitization used for the code-graph DB subdirectory so a
/// branch like `feat/foo` maps to `feat_foo`.
fn branch_dir_component(branch: &str) -> String {
    branch.replace(['/', '\\', ':'], "_")
}

/// Directory holding persisted retrieval-eval runs for a branch:
/// `{engram_dir}/eval/{branch}`.
#[must_use]
pub fn eval_dir(engram_dir: &Path, branch: &str) -> PathBuf {
    engram_dir.join("eval").join(branch_dir_component(branch))
}

/// Persist a retrieval-eval run as JSON under `{engram_dir}/eval/{branch}/`.
///
/// The filename is a high-resolution timestamp so runs sort chronologically and
/// [`latest_report`] can pick the newest. Returns the written file path.
///
/// # Errors
/// Returns a system error if the directory cannot be created or the report
/// cannot be serialized or written.
pub async fn persist_report(
    engram_dir: &Path,
    report: &RetrievalEvalReport,
) -> Result<PathBuf, EngramError> {
    let dir = eval_dir(engram_dir, &report.branch);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| io_err("create run directory", &e))?;

    let stamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros());
    let path = dir.join(format!("{stamp}.json"));

    let json = serde_json::to_string_pretty(report).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to serialize retrieval eval report: {e}"),
        })
    })?;

    // Atomic write: stream to a temp file in the same directory, fsync, then
    // rename into place. A crash or a concurrent [`latest_report`] read never
    // observes a torn / partially written `.json` run file (the reader also
    // skips non-`.json` entries, so the temp file is invisible to it).
    let tmp = path.with_extension("tmp");
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| io_err("create temp run file", &e))?;
    file.write_all(json.as_bytes())
        .await
        .map_err(|e| io_err("write temp run file", &e))?;
    file.sync_all()
        .await
        .map_err(|e| io_err("sync temp run file", &e))?;
    drop(file);
    tokio::fs::rename(&tmp, &path)
        .await
        .map_err(|e| io_err("finalize run file", &e))?;
    Ok(path)
}

/// Read the most recent persisted retrieval-eval run for a branch.
///
/// Returns `None` when no run has been persisted. The newest run is the file
/// with the greatest timestamp stem.
///
/// # Errors
/// Returns a system error if the directory listing or a file read fails, or if
/// the newest run cannot be deserialized.
pub async fn latest_report(
    engram_dir: &Path,
    branch: &str,
) -> Result<Option<RetrievalEvalReport>, EngramError> {
    let dir = eval_dir(engram_dir, branch);
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err("open run directory", &e)),
    };

    let mut best: Option<(i128, PathBuf)> = None;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| io_err("read run directory", &e))?
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let stamp = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.parse::<i128>().ok())
            .unwrap_or(0);
        if best.as_ref().is_none_or(|(current, _)| stamp >= *current) {
            best = Some((stamp, path));
        }
    }

    let Some((_, path)) = best else {
        return Ok(None);
    };
    let contents = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| io_err("read run file", &e))?;
    let report = serde_json::from_str(&contents).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to deserialize retrieval eval report: {e}"),
        })
    })?;
    Ok(Some(report))
}

// ── Threshold comparison (081.007-T) ─────────────────────────────────────

/// Outcome of comparing a report's metrics against baseline thresholds.
///
/// `Default` is intentionally not derived: a zero-valued default would be
/// `{ passed: false, breaches: [] }`, which contradicts the invariant that
/// `passed` is `true` exactly when `breaches` is empty. Construct via
/// [`check_thresholds`] so `passed` and `breaches` stay consistent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThresholdCheck {
    /// Whether every configured threshold was satisfied.
    pub passed: bool,
    /// Human-readable descriptions of each breached threshold (empty on pass).
    pub breaches: Vec<String>,
}

/// Record a breach when `actual` falls below a `min_*` floor (higher is better).
fn check_floor(name: &str, actual: f64, floor: f64, breaches: &mut Vec<String>) {
    if actual < floor {
        breaches.push(format!("{name} {actual:.4} below floor {floor:.4}"));
    }
}

/// Record a breach when `actual` exceeds a `max_*` ceiling (lower is better).
fn check_ceiling(name: &str, actual: f64, ceiling: f64, breaches: &mut Vec<String>) {
    if actual > ceiling {
        breaches.push(format!("{name} {actual:.4} above ceiling {ceiling:.4}"));
    }
}

/// Validate that every configured threshold is a finite number before it gates
/// a run.
///
/// TOML and JSON both accept non-finite floats (`nan`, `inf`). Every `<`/`>`
/// comparison in [`check_thresholds`] is false for `NaN`, so a malformed floor
/// or ceiling would silently report `thresholds_breached = false` and disable
/// the runtime gate (084.006-T). Reject non-finite thresholds with a
/// configuration error so a bad config value fails the run loudly rather than
/// defeating the gate quietly.
///
/// # Errors
/// Returns [`ConfigError::InvalidValue`] (as [`EngramError`]) when any
/// threshold is not finite.
pub fn validate_thresholds(thresholds: &RetrievalEvalThresholds) -> Result<(), EngramError> {
    for (key, value) in [
        (
            "retrieval_eval.thresholds.min_precision_at_k",
            thresholds.min_precision_at_k,
        ),
        (
            "retrieval_eval.thresholds.min_recall_at_k",
            thresholds.min_recall_at_k,
        ),
        ("retrieval_eval.thresholds.min_mrr", thresholds.min_mrr),
        ("retrieval_eval.thresholds.min_ndcg", thresholds.min_ndcg),
        (
            "retrieval_eval.thresholds.min_resolution_recall",
            thresholds.min_resolution_recall,
        ),
        (
            "retrieval_eval.thresholds.max_false_edge_rate",
            thresholds.max_false_edge_rate,
        ),
    ] {
        if !value.is_finite() {
            return Err(ConfigError::InvalidValue {
                key: key.to_owned(),
                reason: format!("threshold must be a finite number, got {value}"),
            }
            .into());
        }
    }
    Ok(())
}

/// Compare a report's metrics against baseline thresholds.
///
/// Semantic metrics and graph `resolution_recall` have `min_*` floors (higher
/// is better); `false_edge_rate` has a `max_*` ceiling (lower is better). The
/// returned [`ThresholdCheck`] lists every breach; `passed` is `true` only when
/// no threshold is violated. Used by the regression tier to guard against
/// metric regressions on a fixture corpus.
///
/// Each metric family is gated **independently** on whether it actually
/// measured anything (084.006-T): semantic floors apply only when at least one
/// known-item query ran (`semantic.queries > 0`), and graph thresholds apply
/// only when at least one call site was visible (`graph.call_sites > 0`). An
/// unmeasured family reports its metrics as `0.0`, so comparing that default
/// against a configured floor would otherwise flag a false breach for a family
/// the run never exercised (e.g. a semantic-only workspace with no call sites
/// must not breach `min_resolution_recall`, and vice versa).
#[must_use]
pub fn check_thresholds(
    report: &RetrievalEvalReport,
    thresholds: &RetrievalEvalThresholds,
) -> ThresholdCheck {
    let mut breaches = Vec::new();
    let semantic = &report.semantic;
    let graph = &report.graph;

    // Semantic floors gate only on a run that ranked at least one known-item
    // query; an unmeasured family (queries == 0) defaults to 0.0 metrics and
    // must not breach a floor it never had a chance to satisfy.
    if semantic.queries > 0 {
        check_floor(
            "precision_at_k",
            semantic.precision_at_k,
            thresholds.min_precision_at_k,
            &mut breaches,
        );
        check_floor(
            "recall_at_k",
            semantic.recall_at_k,
            thresholds.min_recall_at_k,
            &mut breaches,
        );
        check_floor("mrr", semantic.mrr, thresholds.min_mrr, &mut breaches);
        check_floor("ndcg", semantic.ndcg, thresholds.min_ndcg, &mut breaches);
    }

    // Graph thresholds gate independently on a visible call-site inventory; an
    // unmeasured graph (call_sites == 0) defaults to 0.0 metrics and must not
    // breach `min_resolution_recall` (nor spuriously pass/fail `false_edge_rate`).
    if graph.call_sites > 0 {
        check_floor(
            "resolution_recall",
            graph.resolution_recall,
            thresholds.min_resolution_recall,
            &mut breaches,
        );
        check_ceiling(
            "false_edge_rate",
            graph.false_edge_rate,
            thresholds.max_false_edge_rate,
            &mut breaches,
        );
    }
    if graph.target_correct + graph.target_mismatch > 0 && graph.target_mismatch > 0 {
        breaches.push(format!(
            "target_correctness below 1.0: {} mismatched resolved edges",
            graph.target_mismatch
        ));
    }

    ThresholdCheck {
        passed: breaches.is_empty(),
        breaches,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_of_maps_known_extensions() {
        assert_eq!(language_of("src/lib.rs"), "rust");
        assert_eq!(language_of("pkg/main.go"), "go");
        assert_eq!(language_of("app/index.ts"), "typescript");
        assert_eq!(language_of("noext"), "unknown");
    }

    #[test]
    fn language_of_matches_indexer_canonical_mapping() {
        // The semantic gate must resolve every extension to the SAME identifier
        // the indexer stores (`code_graph::language_from_path`), so a configured
        // language includes a file in BOTH the semantic and graph gates. This
        // generalizes the TSX fix (084.005-T) to every supported extension:
        // `.h++`→cpp, `.sql`→sql, `.md`→markdown, `.rb`→`rb`, and an unknown
        // extension falls through to the raw ext rather than a lossy `unknown`.
        for path in [
            "a.rs",
            "a.py",
            "a.js",
            "a.jsx",
            "a.ts",
            "a.tsx",
            "a.go",
            "a.cs",
            "a.c",
            "a.h",
            "a.cpp",
            "a.cc",
            "a.cxx",
            "a.hpp",
            "a.hh",
            "a.hxx",
            "a.h++",
            "a.swift",
            "a.sql",
            "a.kt",
            "a.kts",
            "a.md",
            "a.rb",
            "a.java",
            "a.unknownext",
            "noext",
        ] {
            assert_eq!(
                language_of(path),
                crate::services::code_graph::language_from_path(std::path::Path::new(path)),
                "language_of must match the indexer canonical mapping for {path}"
            );
        }
        // Explicit spot-checks for the extensions the reviewer named, so the
        // concrete expected identifiers are documented, not only the equivalence.
        assert_eq!(language_of("src/vec.h++"), "cpp");
        assert_eq!(language_of("db/schema.sql"), "sql");
        assert_eq!(language_of("docs/readme.md"), "markdown");
    }

    // ── 084.008-T: retrieval-mode resolution ─────────────────────────────────

    #[test]
    fn resolve_mode_empty_corpus_is_unknown() {
        // An empty corpus retrieves nothing; the vector/model flags are moot.
        assert_eq!(
            resolve_retrieval_mode(true, false, false),
            RetrievalMode::Unknown
        );
        assert_eq!(
            resolve_retrieval_mode(true, true, true),
            RetrievalMode::Unknown
        );
    }

    #[test]
    fn resolve_mode_un_embedded_corpus_is_keyword_only() {
        // No candidate carries a vector — the embedding path is never exercised.
        assert_eq!(
            resolve_retrieval_mode(false, false, false),
            RetrievalMode::KeywordOnly
        );
    }

    #[test]
    fn resolve_mode_vectors_but_unusable_model_is_keyword_only() {
        // The 00C7F3CC masquerade: the corpus carries vectors but the query
        // cannot be embedded, so hybrid_search degrades to keyword-only scoring.
        // This must be recorded honestly, not masked as hybrid.
        assert_eq!(
            resolve_retrieval_mode(false, true, false),
            RetrievalMode::KeywordOnly
        );
    }

    #[test]
    fn resolve_mode_vectors_and_usable_model_is_hybrid() {
        assert_eq!(
            resolve_retrieval_mode(false, true, true),
            RetrievalMode::Hybrid
        );
    }

    #[test]
    fn evaluate_semantic_reports_unknown_mode_when_no_query_runs() {
        // A vector-bearing corpus with `sample_size == 0`: the ranking loop
        // never executes, so `queries == 0`. The reported mode must be `Unknown`
        // — a post-hoc probe (which would otherwise report Hybrid/KeywordOnly)
        // never ran a query and must not masquerade as an exercised retrieval
        // mode (084.008-T honesty / Thread-1 correction).
        let corpus = vec![Function {
            id: "function:probe".to_owned(),
            name: "probe".to_owned(),
            file_path: "src/probe.rs".to_owned(),
            line_start: 1,
            line_end: 2,
            signature: "fn probe()".to_owned(),
            docstring: Some("probe docstring".to_owned()),
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: "explicit_code".to_owned(),
            embedding: vec![0.1_f32; 8],
            summary: String::new(),
        }];
        let config = RetrievalEvalConfig {
            enabled: true,
            languages: Vec::new(),
            k: 5,
            sample_size: 0,
            ..RetrievalEvalConfig::default()
        };
        let metrics = evaluate_semantic(&corpus, &config).expect("semantic eval");
        assert_eq!(metrics.queries, 0, "no query should have run");
        assert_eq!(
            metrics.retrieval_mode,
            RetrievalMode::Unknown,
            "a run that ranked nothing must report Unknown, not a probed mode"
        );
    }

    #[test]
    fn derive_query_prefers_docstring_first_line() {
        let f = Function {
            id: "function:x".to_owned(),
            name: "do_thing".to_owned(),
            file_path: "src/x.rs".to_owned(),
            line_start: 1,
            line_end: 2,
            signature: "fn do_thing()".to_owned(),
            docstring: Some("First line.\nSecond line.".to_owned()),
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: "explicit_code".to_owned(),
            embedding: Vec::new(),
            summary: String::new(),
        };
        assert_eq!(derive_query(&f), "First line.");
    }

    #[test]
    fn derive_query_falls_back_to_name() {
        let f = Function {
            id: "function:y".to_owned(),
            name: "helper".to_owned(),
            file_path: "src/y.rs".to_owned(),
            line_start: 1,
            line_end: 2,
            signature: "fn helper()".to_owned(),
            docstring: None,
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: "explicit_code".to_owned(),
            embedding: Vec::new(),
            summary: String::new(),
        };
        assert_eq!(derive_query(&f), "helper");
    }

    #[test]
    fn compute_metrics_perfect_when_all_rank_one() {
        let ranks = [Some(1_usize), Some(1_usize)];
        let m = compute_semantic_metrics(&ranks, 10);
        assert!((m.recall_at_k - 1.0).abs() < 1e-9);
        assert!((m.mrr - 1.0).abs() < 1e-9);
        assert!((m.ndcg - 1.0).abs() < 1e-9);
        assert_eq!(m.queries, 2);
    }

    // ── 084.011-T (CA401F5F): bounded-batch call-site accumulation ───────────

    #[test]
    fn accumulate_call_sites_sums_batch_and_ors_stale() {
        let src = "fn caller() { helper(); }\nfn helper() {}\n";
        let hash = source_content_hash(src);

        // Two clean Rust sources → one (caller,helper) relation each, not stale.
        let clean = vec![
            (
                "src/a.rs".to_owned(),
                "rust".to_owned(),
                src.to_owned(),
                hash.clone(),
            ),
            (
                "src/b.rs".to_owned(),
                "rust".to_owned(),
                src.to_owned(),
                hash.clone(),
            ),
        ];
        assert_eq!(
            accumulate_call_sites(&clean, None, None),
            (2, false),
            "a batch's counts sum and a matching hash is not stale"
        );

        // A source whose recorded hash diverges from its content flags stale, and
        // the stale flag OR-accumulates across the batch.
        let drifted = vec![(
            "src/a.rs".to_owned(),
            "rust".to_owned(),
            src.to_owned(),
            "stale-hash".to_owned(),
        )];
        let (count, stale) = accumulate_call_sites(&drifted, None, None);
        assert_eq!(count, 1, "the single relation still counts");
        assert!(
            stale,
            "content diverging from the recorded hash flags stale"
        );

        // An empty batch is the additive identity (zero, not stale) — the path a
        // final partial-batch flush of size zero must take without double count.
        let empty: Vec<(String, String, String, String)> = Vec::new();
        assert_eq!(accumulate_call_sites(&empty, None, None), (0, false));

        // An unparseable language contributes no call sites (and cannot be stale
        // via a parse it never performs; an empty recorded hash disables the check).
        let unknown = vec![(
            "src/a.unknown".to_owned(),
            "unknownlang".to_owned(),
            src.to_owned(),
            String::new(),
        )];
        assert_eq!(accumulate_call_sites(&unknown, None, None), (0, false));
    }

    // ── 084.006-T (14B33F9F): threshold input validation ─────────────────────

    #[test]
    fn validate_thresholds_rejects_non_finite_values() {
        // A finite (default) threshold set validates.
        assert!(validate_thresholds(&RetrievalEvalThresholds::default()).is_ok());

        // NaN/±inf floors and ceilings must be rejected: every `<`/`>` comparison
        // against NaN is false, so an un-rejected non-finite bound would silently
        // disable the gate (`thresholds_breached = false`) — the exact hazard
        // 084.006-T guards against. Reject with a configuration error instead.
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let floor = RetrievalEvalThresholds {
                min_recall_at_k: bad,
                ..RetrievalEvalThresholds::default()
            };
            assert!(
                validate_thresholds(&floor).is_err(),
                "non-finite floor {bad} must be rejected"
            );

            let ceiling = RetrievalEvalThresholds {
                max_false_edge_rate: bad,
                ..RetrievalEvalThresholds::default()
            };
            assert!(
                validate_thresholds(&ceiling).is_err(),
                "non-finite ceiling {bad} must be rejected"
            );
        }
    }

    #[test]
    fn check_thresholds_gates_each_family_on_measurement() {
        // A family that measured nothing reports 0.0 metrics; comparing those
        // defaults against a configured floor must NOT flag a false breach for a
        // family the run never exercised (084.006-T / Thread-X). Semantic floors
        // gate on `queries > 0`, graph thresholds on `call_sites > 0`.
        let thresholds = RetrievalEvalThresholds {
            min_precision_at_k: 0.5,
            min_recall_at_k: 0.5,
            min_mrr: 0.5,
            min_ndcg: 0.5,
            min_resolution_recall: 0.5,
            max_false_edge_rate: 0.1,
        };

        // Graph measured, semantic unmeasured (queries == 0): only graph gates.
        let mut graph_only = RetrievalEvalReport::empty(true, "b".to_owned());
        graph_only.graph = GraphMetrics {
            call_sites: 4,
            resolution_recall: 0.75,
            false_edge_rate: 0.0,
            ..GraphMetrics::default()
        };
        let graph_check = check_thresholds(&graph_only, &thresholds);
        assert!(
            graph_check.passed,
            "unmeasured semantic family must not breach; breaches={:?}",
            graph_check.breaches
        );

        // Semantic measured, graph unmeasured (call_sites == 0): only semantic gates.
        let mut semantic_only = RetrievalEvalReport::empty(true, "b".to_owned());
        semantic_only.semantic = SemanticMetrics {
            precision_at_k: 0.9,
            recall_at_k: 0.9,
            mrr: 0.9,
            ndcg: 0.9,
            queries: 3,
            retrieval_mode: RetrievalMode::Hybrid,
        };
        let semantic_check = check_thresholds(&semantic_only, &thresholds);
        assert!(
            semantic_check.passed,
            "unmeasured graph family must not breach; breaches={:?}",
            semantic_check.breaches
        );

        // A measured family genuinely below its floor still breaches.
        let mut bad_graph = RetrievalEvalReport::empty(true, "b".to_owned());
        bad_graph.graph = GraphMetrics {
            call_sites: 4,
            resolution_recall: 0.10,
            false_edge_rate: 0.0,
            ..GraphMetrics::default()
        };
        let bad_check = check_thresholds(&bad_graph, &thresholds);
        assert!(
            !bad_check.passed,
            "a measured family below its floor must still breach"
        );
    }

    #[test]
    fn evaluate_semantic_reports_keyword_only_for_unembedded_corpus() {
        // A corpus whose candidates carry no vectors can only run keyword
        // ranking; the reported mode must be the KeywordOnly fallback derived
        // from the actual `hybrid_rank_of` calls (084.008-T / Thread-Y), never
        // masked as Hybrid by a separate probe.
        let mk = |name: &str| Function {
            id: format!("function:{name}"),
            name: name.to_owned(),
            file_path: format!("src/{name}.rs"),
            line_start: 1,
            line_end: 2,
            signature: format!("fn {name}()"),
            docstring: Some(format!("{name} docstring text")),
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: "explicit_code".to_owned(),
            embedding: Vec::new(),
            summary: String::new(),
        };
        let corpus = vec![mk("alpha"), mk("beta")];
        let config = RetrievalEvalConfig {
            enabled: true,
            languages: Vec::new(),
            k: 5,
            sample_size: 2,
            ..RetrievalEvalConfig::default()
        };
        let metrics = evaluate_semantic(&corpus, &config).expect("semantic eval");
        assert!(metrics.queries > 0, "queries must have run");
        assert_eq!(
            metrics.retrieval_mode,
            RetrievalMode::KeywordOnly,
            "an un-embedded corpus must report the keyword-only fallback"
        );
    }
}
