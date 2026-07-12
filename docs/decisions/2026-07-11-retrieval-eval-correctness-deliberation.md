---
title: "Retrieval-Eval Correctness Hardening — make the eval numbers trustworthy"
type: deliberation
date: 2026-07-11
status: decided
signed_off_by: orchestrator
signed_off_on: 2026-07-11
harvested_to: 084-F
plan: docs/exec-plans/2026-07-11-retrieval-eval-correctness-plan.md
related:
  - 081-F
  - 082-F
  - 077-S
stash_ids:
  - D6F70DCC
  - 88B5FAFD
  - 54848E3D
  - D07F0919
  - 49561F22
  - 2894ACB5
  - 14B33F9F
  - 00C7F3CC
  - F137D72E
  - 4CF046A5
  - 78AA205D
  - CA401F5F
  - 635EE7C0
  - 3A280A4E
supersedes_stash:
  - 49561F22
---

# Retrieval-Eval Correctness Hardening

**Status: DECIDED — orchestrator intent is authoritative (2026-07-11).** Harvested into
feature **084-F**; plan at `docs/exec-plans/2026-07-11-retrieval-eval-correctness-plan.md`.

## Problem (evidence)

The retrieval-eval subsystem shipped in **081-F** (measurement substrate) and its first
consumer **082-F** (rec1 cross-file call-edge resolution) landed in `077-S`. But the
subsystem's **metrics are buggy**, so the numbers we use to judge engram retrieval/graph
quality are currently **misleading**. Copilot's 081-F PR review (#238) plus the two 082-F
re-reviews surfaced a cluster of correctness defects that all share one theme: the reported
number is not the number the operator thinks it is. Until the eval is trustworthy, every
future retrieval/graph improvement is gated on a broken ruler, so **correctness goes first**.

Concrete, verified defects (grounded 2026-07-11):

- `count_calls_edges` (`src/db/cozo_queries.rs:2755`) is `?[count(from)] := *calls_edge { from }`
  — whole-graph, **no language gate**, counting **distinct `(from,to)` edges**.
- `count_workspace_call_sites` (`src/tools/eval.rs:73`) parses the **working-tree** files
  (`read_to_string`, `:110`), **language-gated** to `config.languages`, counting **every parse
  occurrence**, and **silently skips** unreadable files (`:104-106`).
- `count_dangling_calls_edges` (`src/db/cozo_queries.rs:2776`) only counts edges whose `to`
  has no `function_meta` row; because the indexer only creates an edge after the callee
  resolves, this is a **dangling-only lower bound** (≈0 in practice) that cannot flag a call
  resolved to a wrong-but-existing target.
- `evaluate_semantic` (`src/services/retrieval_eval.rs:192`) gates by `language_of()`
  (path-extension → `.tsx`→`typescript`, `:55`) while the graph path gates by the stored
  `file.language` (canonical `tsx`); it also scores+clones+sorts the whole candidate corpus
  per query, and derives its corpus from an INNER JOIN that drops docstring-less symbols.
- `[retrieval_eval.thresholds]` is a public config surface that `run_retrieval_eval` never
  consults; only the regression tier reads it against a committed baseline.
- `evaluate_semantic`'s `hybrid_search` swallows embedding failures (`embed_text(query).ok()`),
  so a silent keyword-only fallback is reported as if it were true hybrid retrieval.
- The graph regression tier injects `compute_graph_metrics(10,9,0)` directly, so the real
  count path can regress while the committed baseline stays green.

## Scope boundary (frozen)

Single domain = the **retrieval_eval subsystem** only:

- `src/services/retrieval_eval.rs`
- the metrics/report + `[retrieval_eval]` config models it uses (`src/models/retrieval_eval.rs`,
  the `retrieval_eval` field on `src/models/config.rs`)
- the eval graph read-path helpers it drives: `src/tools/eval.rs`
  (`count_workspace_call_sites`) and the eval **read** queries in `src/db/cozo_queries.rs`
  (`count_calls_edges`, `count_dangling_calls_edges`)
- the eval CLI surface (`engram eval` exit code) and the eval integration/contract tests
  under `tests/`
- the eval plan doc `docs/exec-plans/2026-07-10-engram-retrieval-eval-subsystem-plan.md`

**Explicitly out of scope (do not touch in 084-F):** the indexer write path
(`src/services/code_graph.rs`), the agent-efficiency `evaluation` surface
(`src/services/evaluation.rs`), and any parser/reliability work. See "Excluded / deferred".

## Cluster decisions

### Cluster A — `resolution_recall` numerator↔denominator commensurability

`D6F70DCC` (language scope), `88B5FAFD` (unit mismatch), `54848E3D` (index/disk consistency)
are **the same theme**: numerator and denominator are measured in incompatible units and
scopes, and the `[0,1]` clamp hides it. They are grouped, **not** fixed as three divergent
patches.

**Chosen direction (eval-scoped, precision-first honesty):**

1. **Unit reconciliation** — make both sides count the **same unit: distinct `(caller,callee)`
   call relations.** Dedupe the denominator (`count_workspace_call_sites`) to distinct
   `(caller,callee)` pairs so it matches the numerator's edge unit (a `calls_edge` is keyed by
   `(from,to)`). Recall then reads as "fraction of distinct call relations that resolved."
2. **Language-scope reconciliation** — gate the **numerator** to `config.languages` by joining
   `calls_edge → function_meta → file.language`, so numerator and denominator share one gate.
3. **Index/disk consistency gate** — record the index generation the edges were persisted at
   and compare it to the inventory source at eval time; when the working tree has drifted from
   the indexed revision (or indexed files are unreadable), surface an explicit `index_stale` /
   accounting flag in the report **instead of silently clamping to `[0,1]`**. Honest reporting
   over a fabricated `1.0`.

**Deferred heavier alternative (recorded, not chosen):** persisting a full call-site inventory
(with per-edge multiplicity) at **index generation** time. This is the most precise fix for
multiplicity + staleness but it **modifies the indexer write path** (`code_graph.rs` + schema),
which is outside the frozen 084-F scope and carries whole-workspace re-index blast radius. If
the generation-gate proves insufficient in practice, promote this to its own indexing-scoped
feature. Captured as a note on task `084.003-T`.

### Cluster B — false-edge **target-correctness** (dedup: D07F0919 ⇄ 49561F22)

**Dedup decision.** `D07F0919` (081-F review C2: `false_edge_rate` only detects dangling
callees) and `49561F22` (rec1 follow-up, `rec1`-tagged: `false_edge_rate` is a lower bound,
sample-verify resolved singletons) are **the same false-edge lower-bound issue**. They are
**consolidated into one work item, `084.004-T`.** `D07F0919` is retained as the canonical
source entry; **`49561F22` is superseded/archived** as the duplicate, with the supersede
link recorded here (the 2026-07-10 callgraph plan already noted `49561F22` as "formerly
tracked as `D07F0919`", confirming they are one issue).

This directly implements the **TARGET-CORRECTNESS GATE** clarification already recorded in
`docs/decisions/2026-07-08-callgraph-cross-file-resolution-deliberation.md:25-33`:

> Recall/false-edge thresholds are necessary but **not sufficient**. Every
> `calls_resolved_singleton` edge MUST match the fixture manifest's expected target, checked
> by **exact target identity** — not merely that the target exists. `false_edge_rate` (via
> `count_dangling_calls_edges`) only detects **dangling** targets, so it is a **lower-bound**
> signal; it cannot catch a call resolved to a wrong-but-existing function.

**Chosen direction:** keep `count_dangling_calls_edges` as the labeled lower-bound aggregate,
and add a **fixture-manifest target-correctness assertion** path: retain parsed callee
provenance for `calls_resolved_singleton` edges and assert each resolved target against an
expected-target manifest by exact identity. Bounded, deterministic, testable, and in-scope.
Production-scale sampling of *all* resolved singletons beyond the fixture (the unbounded part
of `49561F22`) is explicitly the follow-on beyond this shipment and is noted on `084.004-T`.

### Cluster C — canonical language gate (`2894ACB5`)

`language_for_gate`/`language_of` maps `tsx`→`typescript` while the indexer stores canonical
`tsx`. **Decision:** return canonical `tsx` so one gate applies to both the semantic and graph
paths. Opt-in only (empty `languages` still gates all in). One-line, isolated → `084.005-T`.

### Cluster D — thresholds actually gate (`14B33F9F`)

`[retrieval_eval.thresholds]` must **do something** at runtime. **Decision:** enforce
thresholds inside `run_retrieval_eval` + report (mirroring `engram verify`), and surface a
breach as a non-zero CLI exit code. Split across width domains: service enforcement
(`084.006-T`) and CLI exit-code surfacing (`084.007-T`).

### Cluster E — retrieval-mode fidelity (`00C7F3CC`)

**Decision:** record an explicit `retrieval_mode` / `fallback` field (hybrid vs silent
keyword-only) so reports and thresholds are comparable across environments and a broken
embedding path can't masquerade as a passing hybrid run → `084.008-T` (field added by the
model task `084.001-T`).

### Cluster F — completeness, perf, memory (in-scope hardening)

- `78AA205D` semantic corpus completeness: `INNER JOIN` → `LEFT JOIN` + name-fallback so the
  denominator reflects every indexed function → `084.009-T`.
- `4CF046A5` semantic eval `O(sample×symbols×log symbols)`: top-k selection instead of full
  clone+sort → `084.010-T`.
- `CA401F5F` graph eval memory: parse in bounded batches instead of accumulating all sources
  → `084.011-T`.

### Cluster G — regression tier exercises the real path (`F137D72E`)

**Decision:** build/index a fixture workspace and assert graph metrics returned by the **real**
`run_retrieval_eval` path (not injected), so Cluster A/B fixes cannot silently regress →
`084.012-T`.

### Cluster H — plan-doc alignment (`635EE7C0`)

**Decision:** update `docs/exec-plans/2026-07-10-engram-retrieval-eval-subsystem-plan.md`
(≈`:157`, `:171`) to the delivered narrowed contract (functions-only + bare-name fallback;
dangling-only lower bound) **and** the 084-F correction (fixture target-correctness added).
Docs-only, width-isolated → `084.013-T`.

### Carry-forward — merge-gate compound learning (`3A280A4E`, operator instruction)

Folded into this shipment as a **docs/instructions** task (`084.014-T`, its own work item to
avoid width-mixing with code): create
`docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md` from the preserved
draft, append the `commit_id == current HEAD` review-completion rule to
`.github/instructions/github-pr-automation.instructions.md` §1.2, and reference it from the
pr-lifecycle merge step.

## Dedup summary (recorded)

| Group | Stash IDs | Action |
|---|---|---|
| `resolution_recall` unit/scope/index | D6F70DCC, 88B5FAFD, 54848E3D | Grouped under Cluster A → tasks `084.002-T` (unit+scope), `084.003-T` (index gate). Not three divergent patches. |
| false-edge lower bound | **D07F0919 ⇄ 49561F22** | **Consolidated into `084.004-T`.** D07F0919 canonical; **49561F22 superseded/archived** as duplicate. |

## Excluded / deferred (left active in stash)

- **`30CE5DD6`** — shared `connect_db` `SQLITE_BUSY` reliability residual (U015-FLK1). Different
  domain (DB reliability); belongs to a reliability shipment, **not** eval-correctness. Left
  active in stash per operator.
- **`2C420C96`** — `get_workspace_status` capability-discovery atomicity. This is a
  **`src/tools/lifecycle.rs` status-handler concurrency/snapshot** concern that merely *reads*
  an eval field; it does **not** fit the eval config-surface scope cleanly (it's a
  pre-existing lifecycle-handler pattern; 081-F only added one field). Folding it in would mix
  lifecycle-concurrency width into the eval-correctness shipment. **Left active in stash.**

## Open questions / risks

1. **Recall unit choice.** Deduping the denominator to distinct `(caller,callee)` changes the
   metric's meaning from "call-site recall" to "call-relation recall". This is the coherent,
   defensible unit given the edge model; documented in the report/module docs so consumers
   (082-F acceptance) read it correctly.
2. **`index_stale` semantics.** Whether a stale-index run should *refuse* to emit recall or
   emit-with-flag. Chosen: **emit with an explicit flag** (non-fatal) so opt-in eval never
   hard-fails a workflow, but the number is never silently clamped.
3. **Threshold-breach exit code.** Must not break the disabled-by-default / empty-run contract
   (empty run still exits 0). Pinned by contract test in `084.007-T`.

## Acceptance nuance carried into the plan

The plan's acceptance section reproduces the **target-correctness / lower-bound** nuance from
`2026-07-08-callgraph-cross-file-resolution-deliberation.md:25-33`: `false_edge_rate` is a
dangling-only **lower bound**; correctness requires **manifest target assertions** by exact
identity in addition to the aggregate rate.
