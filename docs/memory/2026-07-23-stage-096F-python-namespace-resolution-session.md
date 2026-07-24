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
