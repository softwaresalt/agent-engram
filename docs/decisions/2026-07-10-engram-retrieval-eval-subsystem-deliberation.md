---
title: "Portable in-product retrieval + graph-recall eval subsystem — design deliberation"
type: deliberation
date: 2026-07-10
status: decided
signed_off_by: operator
signed_off_on: 2026-07-10
harvested_to: 081-F
plan: docs/exec-plans/2026-07-10-engram-retrieval-eval-subsystem-plan.md
related: [rec1-calledges, B791DE7B, 064-F, 079-F, 081-F]
---

# Portable in-product retrieval + graph-recall eval subsystem

**Status: DECIDED — operator signed off in full on 2026-07-10.** Option B (auto-derivable
ground truth) and the full resolved-design table are accepted; all four open questions
answered (auto-derivable primary + optional labeled augmentation; `.engram/eval/` raw +
`docs/eval/` graduated; disabled by default; expose `enabled` via `get_workspace_status`/
manifest). Harvested into feature **081-F**; plan at
`docs/exec-plans/2026-07-10-engram-retrieval-eval-subsystem-plan.md`. This introduces a new engram subsystem
that measures retrieval and graph-recall quality *inside any indexed workspace*,
so autoharness can run it across languages (Go, Python, Rust, …) and feed results
back for tuning. It is the measurement substrate that gates `rec1-calledges`
(graph call-edge recall) and unblocks stash `B791DE7B` (search-ranking balance).

## Problem (evidence)

Engram has strong *usage/perf* telemetry but **no retrieval-quality measurement**:

- `get_evaluation_report` / `report eval` measures **agent efficiency** (token
  ratios, error bursts, tool-hammering) — not precision/recall of search results
  (`src/services/evaluation.rs:1`).
- The only retrieval test, `tests/integration/relevance_test.rs`, is precision@5
  against a **synthetic 10-query corpus** — a ranking-regression guard, not a
  measurement against a real codebase.
- **No graph-recall eval exists at all** — `map_code`/`impact_analysis` return
  zero call edges for many idiomatic functions (see the 2026-07-08 assessment),
  and we cannot currently quantify that gap or validate a fix.

## Operator requirements (2026-07-10)

1. **In-product / deployable** — the eval capability ships *with* engram and runs
   inside a target workspace, not as a fixture bound to this repo.
2. **Language-agnostic** — works on Go, Python, Rust, and other indexed languages.
3. **Harness-runnable** — autoharness can invoke it to emit evaluation output.
4. **Feedback loop** — emitted results feed back into this workspace as input for
   tuning engram.
5. **Config-gated** — a workspace-level enable/disable mechanism that autoharness
   reads to decide whether to run evals on a given server.

## Grounding (verified 2026-07-10)

- **Config surface already exists (committed).** `.gitignore` excepts
  `!.engram/config.toml`; `src/models/config.rs` reads it into `WorkspaceConfig`
  with a section-per-subsystem pattern (`[batch]`, `[code_graph]`, `[metrics]`,
  `[policy]`, `[evaluation]`), each `#[serde(default)]`. A new `[retrieval_eval]`
  section is a clean extension, not a new file.
- **Naming collision to avoid.** `[evaluation]` / `EvaluationConfig` /
  `get_evaluation_report` already mean *agent efficiency*. The new subsystem must
  use distinct `retrieval_eval` naming.
- **Graph ground-truth denominator already produced.** `extract_calls_from_body`
  (`src/services/parsing/rust.rs`, and peer language extractors) inventories
  syntactic call sites per file. This is an auto-derivable denominator for a
  resolution-recall metric on any language, with no external oracle.
- **Delivery precedent.** 064-F `engram verify` established the pattern where
  engram owns a CLI + output/exit contract and autoharness owns invocation +
  config consumption. CLI report commands are thin wrappers over MCP tools
  (`src/cli/commands/report.rs`).

## Crux design option — ground truth for arbitrary codebases

The hard problem: you cannot hand-label a golden set for every workspace/language.

### A. Hand-labeled per-repo golden sets
Curate query→expected-symbol labels per repo.
- Pro: highest fidelity.
- Con: **does not scale** to arbitrary codebases; violates requirement (2). Rejected
  as the primary mechanism (kept as optional augmentation).

### B. Auto-derivable ground truth (recommended)
No manual labeling; derived from the code itself, so it runs anywhere:
- **Semantic — known-item / self-retrieval.** Use each symbol's docstring /
  leading comment (and/or its fully-qualified name) as a query; the expected top
  result is that symbol. Yields precision@k, recall@k, MRR, nDCG automatically
  across languages.
- **Graph — parser-derived call-site inventory.** Ground-truth denominator = the
  syntactic call sites tree-sitter already extracts. Metrics = **resolution
  recall** (fraction of visible call sites that produced a resolved edge) and
  **false-edge rate** (edges to names with no matching def / ambiguous). Directly
  measures the rec1-calledges recall/precision tradeoff on any workspace.
- Optional: a workspace may supply a hand-labeled set to augment auto ground truth.
- Pro: language-portable, zero manual labeling, measures exactly the gaps we care
  about; false-edge rate makes the rec1 precision policy an empirical decision.
- Con: self-retrieval rewards embedding recall of names/docs (a proxy for, not a
  perfect measure of, semantic relevance); mitigate by reporting it as one signal
  alongside optional labeled sets.

### C. External oracle (LSP / compiler per language)
Use a real resolver as ground truth for call edges.
- Pro: true recall.
- Con: heavy per-language integration, toolchain dependencies in-workspace; out of
  scope for a portable heuristic eval. Rejected.

## Resolved design (pending sign-off)

| Component | Design |
|---|---|
| **Ground truth** | Option B: self-retrieval (semantic) + parser call-site inventory (graph); optional labeled augmentation. |
| **Metrics** | Semantic: precision@k, recall@k, MRR, nDCG. Graph: resolution recall + false-edge rate. |
| **Config** | New `[retrieval_eval]` section in committed `.engram/config.toml` → `RetrievalEvalConfig { enabled: bool, languages, k, sample_size, thresholds }`, `#[serde(default)]`, disabled by default. Exposed via `get_workspace_status`/`manifest` so autoharness discovers it without file-parsing. |
| **Delivery / contract** | `engram eval` CLI subcommand + MCP tool `run_retrieval_eval` (compute) / `get_retrieval_eval_report` (last report). Structured JSON to stdout + a well-known path. Engram owns CLI+output contract; autoharness owns invocation. |
| **Feedback loop** | Per-run JSON under `.engram/eval/{branch}/…` (tool-managed state); notable baselines graduate to `docs/eval/` (committed) for cross-session tuning input; a report tool queries history. |
| **Naming** | `retrieval_eval` everywhere; agent-efficiency `evaluation`/`get_evaluation_report` untouched. |

## Recommendation

Adopt Option B and the resolved design. Open questions for the operator:

1. **Ground-truth methodology**: confirm auto-derivable (self-retrieval + parser
   call-site inventory) as primary, hand-labeled as optional augmentation.
2. **Feedback destination**: is `.engram/eval/` (raw) + `docs/eval/` (graduated
   baselines) the right split, or should graduated baselines live under
   `docs/closure/`?
3. **Default state**: eval disabled by default (opt-in per workspace) — confirm.
4. **Autoharness discovery**: expose `enabled` via `get_workspace_status`/manifest
   (recommended) vs. autoharness reading `.engram/config.toml` directly.

## Blast radius / why not autonomous

New public CLI + MCP contract that autoharness depends on; new committed config
schema section; language-agnostic ground-truth methodology is a product decision
(self-retrieval fidelity vs. hand-labeled cost). Requires operator sign-off before
harvest. Additive to indexing — no change to existing search/graph behavior.

## Proposed decomposition (post sign-off)

Feature with dependency-linked shipment slices (each ≤3 files / ≤5 fns / ≤4 tests):

- **Slice 1 — config + report model + JSON contract**: `[retrieval_eval]` section,
  `RetrievalEvalConfig`, `RetrievalEvalReport` model, empty-state CLI/MCP wiring.
- **Slice 2 — semantic self-retrieval eval**: derive known-item queries from
  symbols; compute precision@k/recall@k/MRR/nDCG; report.
- **Slice 3 — graph resolution-recall eval**: parser call-site denominator vs.
  resolved edges; resolution recall + false-edge rate.
- **Slice 4 — autoharness integration + feedback**: status/manifest exposure,
  `.engram/eval/` persistence, `docs/eval/` graduation, regression test tier.

`rec1-calledges` depends on Slices 1+3 (its acceptance gate): recall must rise and
false-edge rate stay within the operator-chosen bound.
