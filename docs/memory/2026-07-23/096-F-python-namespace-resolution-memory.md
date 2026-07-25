---
title: 096-F Python module-namespace-qualified call resolution — Stage staging + plan-hardening
type: staging-memory
date: 2026-07-23
feature: 096-F
shipment: 091-S
tasks: [096.001-T, 096.002-T, 096.003-T, 096.004-T, 096.005-T, 096.006-T, 096.007-T, 096.008-T, 096.009-T, 096.010-T]
pr: 285
status: queued
harvest_source: FE8B3B2D
follow_ups: [FF7DE872]
---

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

## Addendum — PR #285 plan-review CYCLE 5 (operator: Option A = fix Y1+Y2+Y3, then hard-stop)

Cycle-5 review returned three P1 findings; **Y2 and Y3 are self-inflicted contradictions
introduced by cycle-4's X4 and X1.** No scope change; **task count stays 10; DAG unchanged
and acyclic** (Y3 placed in T5b, which already depends on T1 — **no new dependency edge**).

* **Y1 — function-local import fail-closed ordering (`096.009-T`/T2b).** `def g(): f(); from
  x import f` — `f` is function-local for the whole body, so the call before the import
  raises `UnboundLocalError`; a scope-only lookup would wrongly emit `x.f`. T2b now tracks
  binding AND call positions: calls before a function-local import fail closed; only calls
  after resolve; uncertain control flow fails closed. Before/after + uncertain-order cases
  added (consolidated to 4 scenarios).
* **Y2 — order-aware rebind guard (`096.007-T`/T5c + plan T5c).** X4's blunt "re-bound
  anywhere" set contradicted X2/X3 last-binding-wins (would drop `def parse; from bar import
  parse; parse()`, which must resolve to `bar.parse`). T5c is now **order-aware**: only a
  rebind AFTER the import invalidates it; a def/class/del/match-capture or any rebind BEFORE
  the import does not. Added the import-after-def resolves case to scenario (a);
  cross-referenced T5b<->T5c.
* **Y3 — legacy fallback preserves recall (`096.006-T`/T5b + `096.005-T`/T5a + plan).** T5a's
  `python_bare` stamp excludes bare calls from the legacy name-only matcher; when T1 rejects a
  src/-root or namespace layout (no module context), T5b had no context -> the call dropped to
  NO edge, regressing a unique bare call that resolves today via the legacy matcher. T5b now
  falls back to the existing legacy name-only unique-match when there is no module path (recall
  preserved, no false module edge); canonical resolution applies only when a module path
  exists. Placed in T5b (already deps T1) -> **no new DAG edge**; legacy pass untouched. Added
  the no-context recall test to T5b (consolidated fail-closed vectors to stay at 4); T5a gets a
  cross-ref note only.

Reconciliation principle (crisp, for the incoming multi-persona adversarial review):
**last-binding-wins, order-aware; when ambiguous/uncertain, fail closed.** T5b resolves to the
last effective binding and is guard-agnostic; T5c wraps T5b and fails closed only on a rebind
AFTER the import; T2b supplies the order model incl. the function-local UnboundLocalError rule.
T5a/T5b/T5c do not contradict each other.

Files changed: plan doc + tasks `096.005-T`, `096.006-T`, `096.007-T`, `096.009-T` + feature
`096-F` (Acceptance/goals consistency) + this memory. Verified: DAG acyclic (T5b independent of
T5c; T5a deps unchanged = {096.004-T}), 10 tasks, shipment 091-S = 11 items. Gate remains PASS.

## Addendum — PR #285 cycle 6 (adversarial reconciliation / structural revision)

A **3-model adversarial review** (Opus 4.8 + GPT-5.6-Sol + Gemini 3.1 Pro, source-verified
against merged main) returned **🔴 NOT CONVERGED** with **2 gate-blocking P0s** plus verified
P1s and Copilot cycle-6 threads that all 5 Copilot cycles + the X/Y fixes missed. Operator chose
**Option A = fix substantively, then push and STOP**. This cycle is a **structural revision**
(not a consistency refinement): the root defect is that last-binding-wins was adjudicated in
T5b/T5c while the layer BELOW them — the in-file direct-edge path at `code_graph.rs:900-902` —
mints a false `M.callee` edge and renders T5b's local-def branches unreachable.

* **F2 (P0, root defect) — `096.005-T`/T5a + plan §Design-decision/§T5a/Risks.** Removed the
  "in-file bare calls stay direct — unchanged (900-903)" claim; made the in-file `(Some,Some)`
  decision import/shadow-aware (a same-file callee that is ALSO a module import routes to
  `python_bare` staging, not a direct edge). **T5a owns the `code_graph.rs:896-908` change on
  both arms.** **Added the ONE new DAG edge this cycle: T2→T5a** (T5a must consult T2's
  `ImportBindings` at staging time). `096.005-T` frontmatter deps `{096.004-T}` → `{096.002-T,
  096.004-T}`.
* **F1 (P0, contradiction) — `096.007-T`/T5c + plan §T5c.** Re-anchored the order-aware guard on
  the WINNING binding T5b resolved (import OR def), failing closed only on rebinds AFTER the
  winner; `from bar import parse; def parse(); parse()` → `M.parse` now SURVIVES (def-after-import;
  cycle-5 Y2 only covered def-before-import).
* **F3 (P1) — `096.006-T`/T5b + plan §T5b/Risks.** Legacy name-only fallback fires whenever T5b
  derives NO canonical target (not only T1==None) — covers provable-namespace-but-unbound
  (star/re-export/relative), which `qualifier_kind.is_empty()` filtering (cozo 2222-2227) would
  otherwise drop.
* **F9 (P1) — `096.006-T`/T5b.** Chose option (a): expose a public read-only language-scoped
  name→IDs helper in `cozo_queries.rs`; deleted the "No cozo_queries.rs change" claim (T5b now 3
  files). Option (b) rejected (can't serve the F3 case).
* **F4 (P1) — `096.002-T`/T2 + T5c.** Module-scope `from N import *` recorded as a positioned
  order-aware invalidator; star-after-winner fails closed.
* **F5 (P1) — `096.009-T`/T2b.** Modeled the lexical closure chain (honoring global/nonlocal) or
  fail closed on ambiguous enclosing bind; folded F8 (pre-import poison/tombstone) + F14
  (positions for all bindings).
* **C6-1 — `096.003-T`/T3.** Package-topology changes (add/remove `__init__.py`) reindex/
  invalidate descendants past the content-hash skip (1252-1263); both transitions tested.
* **C6-4/5 (+F6/F7) — `096-F` DoD + `096.008-T`/T6 + plan §Monitoring/Risks.** Replaced the
  report-only 1.000-precision claim with a manifest-backed target-identity gate + manual audit +
  recall parity; `get_retrieval_eval_report` kept as a dangling-edge tripwire only.
* **F18/F19.** Reconciled the DAG edge-list (added T3→T6 to the main list) and qualified the
  feature/spike "Option B" vs plan "Option A" naming collision at `096-F:21`.
* **Declined/flagged:** F16 (split T5b) declined — advisory/LOW + operator minimal-DAG directive
  (F3+F9 stay in T5b, 3 files, ≤4 scenarios). F17 (`import a.b` root binding) treated as a
  defensive-test note — reviewer states likely already fail-closed. Both reported to operator.

**One coherent contract:** T5a routes shadowed in-file calls to staging (F2) → T5b adjudicates
last-binding-wins and falls back on no-target (F3/F9) → T5c fails closed only on rebinds after the
winning binding (F1). **DAG: exactly one new edge T2→T5a**; task count stays 10; shipment 091-S
stays 11 items; acyclic (verify: T5a←{096.002-T,096.004-T}). Files changed: plan doc + tasks
`096.002-T`, `096.003-T`, `096.005-T`, `096.006-T`, `096.007-T`, `096.008-T`, `096.009-T` +
feature `096-F` + this memory (moved to `docs/memory/2026-07-23/` with YAML frontmatter, C6-2).
This is plan-review-fix **cycle 6 (operator-directed; Option A — structural revision, hard-stop
after push)**.

## Addendum — plan-review-fix cycle 7 (FINAL consolidated pass; prove-or-fail-closed generalization)

A focused 3-model adversarial re-check confirmed cycle-6's **F1/F2/F3/F9/F5 all PASS**, then
triangulated **one remaining root defect** + bounded adjacent items. Root: cycle-6's F2 keyed
T5a's routing predicate on **named module-level imports ONLY**, so it did not compose with the
other shadow classes. Operator chose **Option A = fix substantively, then push and STOP** (LAST
structural pass). All fixes fold into one **prove-or-fail-closed invariant**.

* **Z1+Z2+C7-2 (unified — generalize T5a) — `096.005-T` + plan §Resolution-rule invariant +
  §Design-decision + §T5a + Requirements-Trace + Risks.** T5a's `(Some,Some)` in-file predicate
  now routes a same-file `def` to `python_bare` staging whenever its name is **shadow-contested at
  ANY modeled scope** — named import, positioned star-after-def (Z1), non-import module rebind
  (Z2), OR scoped function-local/enclosing import (C7-2). Expressed as an abstraction
  ("shadow-contested at any modeled scope"), not axis-by-axis, so future axes are covered by
  construction (also closes the F4 remainder).
* **C7-1 (call-position / time axis) — `096-F` rule + `096.006-T`/T5b + `096.007-T`/T5c + plan
  §Resolution-rule/Requirements-Trace/Risks.** "Last-binding-wins" is now **call-site-effective**:
  resolve to the last provable binding PRECEDING the call in execution order (module source order);
  a module-level call before a later import binds the earlier def (`def parse(); parse(); from bar
  import parse` → M.parse); T5c's invalidation window is strictly between the winner and the call;
  a function-body call contested by a later module rebind FAILS CLOSED.
* **C7-3 (upgrade-gate partial-failure) — `096.010-T`/T7 + plan §T7 + Risks.** The
  `PYTHON_CANONICAL_EXTRACTION_VERSION` marker persists ONLY on a fully-successful pass; any
  per-file failure (`SyncResult.errors` non-empty, code_graph.rs:1215-1221) keeps the OLD marker so
  migration retries next sync (no stale-forever hash-skip). Partial-failure regression test added.
* **C7-4 (spike prose) — spike doc.** Added a labeled **SUPERSEDED** note to the "no changes to
  cozo_queries.rs" conclusion + change-surface table row (T5b/F9 adds a public read-only name→IDs
  helper; still zero schema change). History preserved.
* **Z3 (096-F resolution-rule prose) — `096-F`.** Qualified the star/re-export/unbound "→ DROP" to
  "DROP (no canonical module-qualified edge; a unique legacy name-only edge is preserved per F3)";
  reconciled with the F3 recall goal.

**Whole-contract composition re-verified** (plan §Resolution rule composition-trace table): def-only,
import-only, def-before-import, import-before-def, def+star-after-def, def+non-import-rebind,
def+function-local-import, provable-namespace-unbound, call-before-later-rebind — each yields exactly
one unambiguous outcome owned by exactly one layer; everything unprovable fails closed. **DAG: exactly
one new edge T2b→T5a** (T5a consults T2b's scoped-binding signals); acyclic (T2b←{T2} only, already
upstream of T5b/T5c — no back-edge). New T5a deps: {096.002-T, 096.004-T, 096.009-T}. **Task count
stays 10; shipment 091-S stays 11 items**; all new vectors consolidated into existing ≤4-scenario
tasks — no new task, no scope creep. **No findings judged false positives.** Files changed: plan doc +
spike doc + tasks `096.005-T`, `096.006-T`, `096.007-T`, `096.010-T` + feature `096-F` + this memory.
This is plan-review-fix **cycle 7 (operator-directed; Option A — FINAL consolidated pass, hard-stop
after push)**.

## Cycle 8 addendum — narrow closure of the non-import-rebind axis (C6, Option B)

The cycle-7 FINAL pass was validated (C7-1/C7-3/C7-4/Z3 PASS; **8 of 9 composition vectors** close);
a targeted 3-model re-check @ `689da02a` found **one** remaining failing vector — **C6: `def` +
non-import module/scope rebind** (`def parse(); parse = factory(); parse()`; also
`class`/`del`/`match`-case/`for`/`with … as`/`except … as`/walrus/parameter targets). **Root cause:**
cycle-7's "shadow-contested at any modeled **scope**" abstraction closed over SCOPES but **not over
binding FORMS**, and the non-import-rebind axis has **no producer** (T2/`096.002-T` and T2b/`096.009-T`
walk only import nodes), so that case still hit the in-file `(Some,Some)` arm
(`code_graph.rs:900-902` full-index / ~1635-1636 sync) and minted a false direct `M.parse` edge that
never reached T5c's (correct) rebind guard. Operator authorized **Option B** — minimal, no new
producer/task/DAG edge — because **T5a already holds the caller file's tree-sitter AST** at the
consumer site.

* **Invariant reframed over FORMS *and* SCOPES — plan §Resolution-rule invariant.** Prove-the-negative
  default: *keep a same-file `def` on the direct-edge path ONLY when it is provably the sole binding
  across every modeled scope AND with no non-import rebind form present; otherwise route to
  `python_bare` staging.* Closure is now over both the scope axis and the binding-form axis, so neither
  a new scope nor a new syntactic rebind form can silently reopen a false-edge path. Repaired the
  "future axes covered by construction" wording to mean form-closure too.
* **Shared rebind-form set (RFS) — single source of truth — plan §Design decision.** Defined the
  rebind-FORM set **once**; consumed **identically** by T5a's in-file routing scan (order-agnostic
  "any RFS form present → stage") and T5c's order-aware invalidation scan (same forms, winner→call
  window). The shared artifact is the syntactic form set, not the order logic — so `096.005-T`/T5a and
  `096.007-T`/T5c can never diverge on which forms count.
* **T5a runs the shared in-file rebind scan — `096.005-T` + plan §T5a + §Design-decision + Risks +
  Requirements-Trace.** T5a's `(Some,Some)` routing now routes to staging whenever the callee name is
  contested by EITHER a producer-backed import signal (T2/T2b) OR any RFS non-import rebind form it
  detects with its OWN tree-sitter scan (the same one T5c runs), on **both** arms (896-908 +
  1573-1643). Added routing vector `def f; f = factory(); f()` → **STAGED, no direct `M.f`** (also
  covers `class`/`del`/`match`-case) and the def+non-import-rebind → DROP row to the Requirements-Trace.
* **Composition-trace row C6 — plan.** Now **DROP** (owning layer = T5a routing (shared RFS scan) →
  T5c invalidation). Re-verified C1–C5, C7, C8, C9 unchanged.
* **T5c references the shared RFS — `096.007-T`.** No behavior change (T5c already scanned these
  forms); it now names the shared RFS definition as its source so it cannot drift from T5a.

**No DAG change, no new task:** the non-import rebind forms have no producer and T5a scans them itself,
so **no new producer, task, or DAG edge** was required. Edge list unchanged (T5a←{T2,T2b,T4};
T5b←{T1,T3,T5a,T2b}; T5c←{T5b,T2b}; T2b←{T2}; T3←{T1}; T6←{T3,T5b,T5c,T7}; T7←{T3,T5b}; acyclic).
**Task count stays 10; shipment 091-S stays 11 items.** **C6 judged a genuine defect (3/3 unanimous,
source-verified) — no false positives.** Files changed: plan doc + tasks `096.005-T`, `096.007-T` +
this memory. This is plan-review-fix **cycle 8 (operator-directed; Option B — narrow closure,
hard-stop after push)**.

**Folded into the same cycle-8 commit — two adjacent internal-consistency threads (Copilot
cycle-8 @ `689da02a`):**

* **C8-1 (duplicate same-name imports) — Option B (narrowed rule).** The frozen C7-1 call-site
  last-binding rule appeared to promise ordering across `from a import f; from b import f; f()`,
  contradicting T2's fail-closed-on-duplicate (M1). Reconciled (cheapest, strictly fail-closed):
  ordering disambiguates `def`-vs-**single**-import + rebinds only; **≥2 competing imports of a
  name → T2 no binding (M1) → T5b no target → fail closed** (F3 legacy fallback also drops,
  non-unique). No regression vs main; ordered-import-history is a v1 non-goal. Edited: plan
  §Resolution-rule (new C8-1 blockquote) + §T5b (new fail-closed sub-bullet); `096.002-T` (M1
  now cites the two-from-import example) + `096.006-T` (fail-closed vector folded, no 5th
  scenario). **T2 + T5b + acceptance + plan now agree.**
* **C8-2 (spike DROP rows vs F3 legacy edge) — spike doc.** Added two labeled "⚠️ Refined
  (C7-4/F3)" notes (under the resolution-rule list and the Fail-closed matrix) qualifying every
  `DROP` as "no canonical module-qualified edge; a unique legacy name-only edge may remain
  (non-unique still fails closed)". Spike history preserved; now matches plan + `096-F`.

Both fold into the same commit as the C6 closure (one push / one re-check). No new task, no new
DAG edge, no new code behavior. Task count stays 10; shipment 091-S stays 11 items. Files added
this fold: `096.002-T`, `096.006-T`, spike doc. **No findings judged false positives.**

## Cycle 9 — final spec-polish (C9-1..C9-4 + nits A/B, planning-only)

An independent 3-model adversarial re-check @ `1d4f9caf` returned **CONVERGED** (C6 genuinely
closed, all 9 vectors compose, no P0/P1/P2, no false-edge hole). Four Copilot cycle-9 LOW/P3
threads + two re-check nits folded into ONE commit atop `1d4f9caf`. **None create a false edge.**

* **C9-1 (RFS self-contest / over-routing).** `def name` is in the RFS, so "any RFS form → stage"
  would match the def's OWN binding for `def f; f()`, making the direct fast path unreachable dead
  code (no false edge). Fix: T5a's scan **excludes the matched `def` under consideration** (candidate
  target, not self-competitor) and counts only OTHER bindings applicable to the caller's lexical
  scope; direct fast path taken iff the matched def is provably the sole applicable binding. Edited:
  plan §Design-decision (RFS + F2 self-exclusion) + §T5a + `096.005-T`.
* **C9-2 + nit A (the "T5a already holds the AST" claim was FALSE).** `parse_source` returns
  `ParseResult{symbols,edges}` only (`parsing.rs:247-252`) — no tree-sitter `Tree` retained; the
  file `source: String` IS in scope (cloned ~`code_graph.rs:609`/`1290`). Fix: T5a **re-parses the
  in-scope source in-memory**, computing the Rebind-Form-Set map **once per file, cached** (not per
  call). Load-bearing conclusion stays TRUE: no new producer/DB field/task/DAG edge. All "AST" seam
  wording purged from §T5a / `096.005-T` / Risks / cycle-8 addendum.
* **C9-3 (C7-1 module-level call ordering cannot run).** The extractor emits `Calls` only from
  function bodies (`python.rs:44-79,233-277`); a module-level `parse()` is never staged. Fix:
  C7-1 narrowed to **function-body calls only** — resolve to a stable module/enclosing binding, else
  fail closed; **module-level (top-level) call ordering is a v1 non-goal** (neither regressed nor
  handled). The extractable outcome (fail-closed / no false edge) is unchanged; only unreachable
  prose corrected. Edited: §Resolution-rule (C7-1 blockquote + bare-name rule), §T5b, §T5c,
  composition-trace C9, Requirements-Trace, Risks, Monitoring corpus, `096.006-T`.
* **C9-4 (F3 fallback must not leak an edge for competing bindings).** Fix: T5b's outcome carries a
  **typed no-target reason**; legacy name-only fallback allowed ONLY for
  `{NoModuleContext, UnsupportedImportForm}`; NEVER for
  `{CompetingBindings, Shadowed, DuplicateSameNameImport}` (fully fail-closed, no edge). C8-1
  duplicate-import → `DuplicateSameNameImport` → no fallback (genuinely complete). Edited: §T5b,
  Requirements-Trace, `096.006-T`.
* **Nit B (annotated assignment).** RFS "assignment" now explicitly enumerates **annotated
  assignment** (`name: T = …`) alongside plain/augmented. No behavior change. Edited: §Design-decision
  RFS + `096.005-T`.

**No DAG change, no new task:** C9-2's honest resolution is a self-serve in-memory re-parse.
`096.005-T` deps stay {`096.002-T`,`096.004-T`,`096.009-T`}; `096.006-T` deps unchanged. Edge list
unchanged (T5a←{T2,T2b,T4}; T5b←{T1,T3,T5a,T2b}; T5c←{T5b,T2b}; T2b←{T2}; T3←{T1};
T6←{T3,T5b,T5c,T7}; T7←{T3,T5b}; acyclic). **Task count stays 10; shipment 091-S stays 11 items.**
All 9 composition vectors re-verified — C9-1/C9-4 tighten only HOW a vector is decided, not its
result; C9-3 corrects unreachable prose. Files changed: plan doc + `096.005-T` + `096.006-T` + this
memory. **No findings judged false positives.** This is plan-review-fix **cycle 9 (operator-directed;
final spec-polish, hard-stop after push)**.
