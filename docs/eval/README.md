# Retrieval-evaluation graduated baselines

This directory holds the **committed baseline thresholds** for the portable
retrieval + graph-recall evaluation subsystem (`retrieval_eval`, feature 081-F).
Unlike raw evaluation runs — which are tool-managed and land under
`.engram/eval/{branch}/` (gitignored) — the values here are **graduated**:
promoted deliberately into version control so they can inform cross-session
tuning and guard against metric regressions.

## What the subsystem measures

The `retrieval_eval` subsystem answers two auto-derivable questions about a
workspace index (no manual relevance labels required):

* **Semantic self-retrieval** — each indexed symbol's docstring / qualified name
  becomes a known-item query whose single expected hit is that same symbol.
  Reported as `precision@k`, `recall@k`, `MRR`, and `nDCG@k`. This is a *proxy*
  signal (it rewards name/doc recall) and is read as one signal alongside the
  graph metrics.
* **Graph resolution recall** — the tree-sitter call-site inventory
  (`extract_calls_from_body`) is the denominator; resolved `calls` edges are the
  numerator. Reported as `resolution_recall` (resolved ÷ visible call sites) and
  `false_edge_rate` (edges whose callee matches no known definition ÷ resolved).
  This is the empirical surface that gates downstream call-graph work.

## `baseline.json`

`baseline.json` is a serialized [`RetrievalEvalThresholds`] record:

| Field | Direction | Meaning |
| --- | --- | --- |
| `min_precision_at_k` | floor | minimum acceptable precision@k |
| `min_recall_at_k` | floor | minimum acceptable recall@k |
| `min_mrr` | floor | minimum acceptable mean reciprocal rank |
| `min_ndcg` | floor | minimum acceptable nDCG@k |
| `min_resolution_recall` | floor | minimum acceptable graph resolution recall |
| `max_false_edge_rate` | ceiling | maximum acceptable graph false-edge rate |

Floors (`min_*`) require *higher-is-better*; the ceiling (`max_*`) requires
*lower-is-better*. The regression tier
(`tests/integration/retrieval_eval_regression_test.rs`) loads this file and
asserts a fixture evaluation clears every threshold via
`services::retrieval_eval::check_thresholds`.

## Updating the baseline

1. Run the eval on a representative workspace: `engram eval` (or the
   `run_retrieval_eval` MCP tool). Raw runs are written under
   `.engram/eval/{branch}/`.
2. Inspect the metrics and decide whether the change is an intended, durable
   improvement (tighten a floor) or an accepted trade-off (relax a threshold).
3. Edit `baseline.json` accordingly and update this README's rationale.
4. Re-run the regression tier to confirm the fixture still passes.

Baselines are intentionally conservative: they exist to catch *regressions*, not
to encode aspirational targets. Tighten them only when a gain is reproducible.
