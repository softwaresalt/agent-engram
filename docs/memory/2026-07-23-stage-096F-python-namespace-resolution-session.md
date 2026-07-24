# Stage session — 096-F Python module-namespace-qualified call resolution

**Date**: 2026-07-23 · **Agent**: Stage · **Branch**: `stage/py-namespace` (off `main` `6d6c9d9c`)
**Stash**: FE8B3B2D · **Deliberation**: Option B ratified (product value operator-asserted → feasibility spike)

## Outcome

Full staging pipeline executed: **spike [GO] → impl-plan → plan-harden → plan-review [PASS] → harvest → queued shipment**. Pushed for Orchestrator to own PR + merge. STOP boundary respected (no `gh pr create`, no merge).

## Artifacts

* Spike: `docs/decisions/2026-07-23-python-namespace-canonical-resolution-spike.md` — verdict **GO / high confidence** (commit `04d78ad1`).
* Plan (+ hardening + plan-review inline): `docs/exec-plans/2026-07-23-python-namespace-canonical-resolution-plan.md` — **PASS** (commit `1e912ec9`).
* Harvest: feature **096-F** + tasks **096.001-T .. 096.008-T** (T1, T2, T3, T4, T5a, T5b, T5c, T6); queued shipment **091-S** (commit `7a779240`).

## Key findings (code-grounded, verified against current tree)

* `python.rs` already emits Calls (094-F merged); imports captured as a **flat string** (`extract_import` 141-143) — no symbol-level binding table (the gap).
* Canonical DB layer is **language-agnostic and reused with ZERO schema change**: `function_meta.canonical_path` (cozo_queries 924-940), `function_ids_by_canonical_path` (1046), `canonical_paths_for_function_name` duplicate fail-closed (1026).
* `reresolve_calls_edges` (cozo_queries 2182) is language-scoped but **name-only** → drops 2+ same-name (the recall gap FE8B3B2D closes via canonical/module paths).
* Resolver `reresolve_calls_edges_with_canonical_context` (code_graph **264**, NOT cozo_queries as the stash claimed) has 2 Rust-specific seams (`rust_ctx_for_staged_file`, `canonical_target_for_staged_call`); the singleton/duplicate fail-closed core is reused.
* Populator `canonical_path_for_function` (code_graph 145-169) Rust-only; call sites 721/1446.

## Decisions

* **Option A** (mirror Rust): route Python canonical-eligible calls through provenance staging; canonical post-pass resolves them; `reresolve_calls_edges` (bare-name) untouched; `cozo_queries.rs` change-free.
* **plan-review P1 caught & resolved**: local var/param shadowing an imported module name (`import parser; parser=f(); parser.parse()`) → false edge → dedicated fail-closed guard task **T5c**. 1 P1 < 3 → no full adversarial review needed.
* Scope frozen: IN = module-level + `module.func()`; OUT = instance-method dispatch. **FF7DE872** (same-file shadowing) split out, independent, NOT subsumed (canonical index fails closed on duplicate, can't do last-wins).
* No dependency wired to 090-S/095-F (operator-ratified independent). Internal DAG: T1→T3/T5b; T2→T5b; T4→T5a→T5b→T5c; {T3,T5b,T5c}→T6.

## Next steps (for Orchestrator / Ship)

* Orchestrator: create PR for `stage/py-namespace`, merge (Stage does not).
* Ship: claim shipment **091-S** after merge; execute T1→T6 test-first; enforce ordered gates (`fmt → clippy --all-targets → cargo dev-test → cargo audit`); target-identity + fail-closed acceptance tests are the hard gate.
* Rollback triggers (from plan): any false Python→X edge; any Rust singleton/canonical regression; `__init__.py`/namespace-package def acquiring a non-empty canonical_path.

## Blockers

None. Spike = GO; plan-review = PASS; shipment queued.

## Addendum — PR #285 plan-review hardening (cycles 1–2)

Post-harvest, PR #285 plan-review hardened the impl-plan + task files. Net result: the
task set grew from the originally-harvested **8** to a final **10**.

* **Cycle 1 (M1–M5):** split out **T2b** (`096.009-T`, scope-aware binding isolation,
  from M1) and **T7** (`096.010-T`, versioned re-extraction/backfill, from M4); T1
  package-layout fail-closed (M5), T4 `self`/`cls` exclusion (M2), M3 test-target
  registration. Task count 8 → 10.
* **Cycle 2 (Q1–Q6):** Q1 extended the T5c shadow guard to imported **bare callees**
  (`096.007-T`/`096.006-T`); Q2 made T7 run the canonical resolution pass in the **same
  operation** and assert the resolved edge (`096.010-T`); **Q3+Q6 = NARROW** — T1 fails
  closed on `src/`-roots / PEP 420 namespace / `__init__.py`, source-root machinery
  removed, source-root-aware resolution = documented v1 non-goal (`096.001-T`/
  `096.003-T`); Q4/Q5 aligned the DoD + artifact count. **No new task — count stays 10.**

**Final harvest (10 tasks):** feature **096-F** + **096.001-T .. 096.010-T** =
T1, T2, T3, T4, T5a, T5b, T5c, T6 (T2/T2b/T7 numbering: 096.002-T=T2, 096.009-T=T2b,
096.010-T=T7); queued shipment **091-S** (11 items: 096-F + 10 tasks). DAG (acyclic):
T3←T1; T5a←T4; T5b←{T1,T3,T5a,T2b}; T5c←{T5b,T2b}; T6←{T3,T5b,T5c,T7}; T2b←T2;
T7←{T3,T5b}. Scope unchanged: module-level namespace resolution, fail-closed;
FF7DE872 independent; no 090-S/095-F dependency.

## Addendum — PR #285 plan-review hardening (cycle 3, the cap)

Cycle-3 review at `0279a823` surfaced three plan-consistency findings (R1–R3), all gaps
in the cycle-2 hardening. **No new task — count stays 10; DAG unchanged and acyclic.**

* **R1 — extraction version vs `content_hash` contract (`096.010-T`/T7).** T7's cycle-2
  wording folded the version into the `.py` content hash, but `retrieval_eval::is_index_stale`
  (`retrieval_eval.rs:717-718`) compares `file_node.content_hash` byte-for-byte against the
  raw source SHA. Moved the version into a **dedicated `PYTHON_CANONICAL_EXTRACTION_VERSION`
  index-state marker** (`TMDL_DAX_INDEX_VERSION` precedent, `powerbi_indexer.rs:60-81`);
  `content_hash` stays the raw SHA; added a staleness-preservation regression.
* **R2 — `ImportBindings` binding kind (`096.002-T`/T2, used by `096.006-T`/T5b).** T2 now
  records `(canonical_path, kind∈{ModuleImport, FromImportSymbol})`; T5b resolves a module
  receiver only from a `ModuleImport` and fails closed on a `FromImportSymbol` receiver
  (`from pkg import parse; parse.tokenize()` → no edge, out of scope).
* **R3 — T5b/T5c completability cycle (`096.006-T`/T5b, `096.007-T`/T5c).** Guard now lives
  **entirely in T5c**, which wraps a **guard-agnostic** T5b (T5b no longer invokes the guard);
  dependency stays one-directional T5c→T5b, so both are independently completable and the DAG
  is acyclic. Guard coverage still includes bare imports (Q1).

Verified after edits: `get_dependencies` shows T5b={001,003,005,009} (NOT 007) and
T5c={006,009} → acyclic; 10 tasks; shipment **091-S** = 11 items (096-F + 10). Cycle 3 is
the cap — if a cycle-4 review still surfaces NEW substantive gaps, accept the plan as-is
(residual → Ship execution-time considerations).

## Addendum — PR #285 plan-review CYCLE 4 (operator: Option A = fix substantively)

Cycle-4 review returned five plan/task-consistency findings (X1–X5); **X6 = PR body,
Orchestrator-owned, untouched.** No scope change; **task count stays 10; DAG unchanged and
acyclic.**

* **X1 — sync-arm staging bypasses canonical resolution (`096.005-T`/T5a).** The Calls sync
  consumer (`code_graph.rs:1573-1643`) name-only-stages bare Python calls via
  `put_staged_call` (1639), so T7 re-extraction strands them — a **third** site after Q2/R1.
  T5a now routes Python bare + module-qualified calls through provenance on **both** the
  full-index (851-908) and sync (1573-1643) arms.
* **X2/X3 — local-def-first false edge (`096.006-T`/T5b + plan resolution rule).** `def
  parse; from bar import parse; parse()` binds to `bar.parse`; local-def-first mints
  `M.parse`. Now **last-binding-wins**: a later `FromImportSymbol` rebind beats a local def
  → `N.callee`; unclear order → fail closed. Counterexample pinned in T5b.
* **X4 — shadow-guard rebind set (`096.007-T`/T5c).** Added `def`/`class`/`del`/`match`-case
  capture as name-rebinding invalidators (`import bar; class bar: ...; bar.parse()` → no
  edge).
* **X5 — monitoring contract (plan).** Added a **Monitoring & Rollback** section mirroring
  090-S A5 / 095-F Fork-A: SLI = Python module-qualified edge precision via
  `get_retrieval_eval_report`; baseline 1.000 / 0 false edges; alert = precision < 1.0;
  rollback = any confirmed false edge post-index; window = first release cycle (≈2 wks) +
  first real-package cohort; owner = code-graph parsing/resolution area.

Files changed: plan doc + tasks `096.005-T`, `096.006-T`, `096.007-T` + feature `096-F`
(rule/DoD/goals consistency). Verified again: DAG acyclic (T5b independent of T5c), 10 tasks,
shipment 091-S = 11 items. Gate remains PASS.
