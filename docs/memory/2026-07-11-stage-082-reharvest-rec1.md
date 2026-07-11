# Stage session — 2026-07-11 — 082-F re-harvest (rec1-calledges) after Copilot #239 block

**Agent:** Stage · **Repo:** softwaresalt/agent-engram · **Branch:** `chore/082-reharvest-rec1` (off `main` @ 550fbb2)
**Trigger:** PR #239 (backlog closure for 077-S/081-F + 082-F/078-S harvest) merged to main (merge commit 550fbb2).
Copilot review of #239 blocked shipment **078-S** because three 082-F tasks exceeded granularity limits
(single-width, ≤3 files, ≤4 scenarios) and the executable task bodies under-specified the full
lifecycle/storage contract that lived only in the authoritative plan. 078-S was `blocked`.
Tracked by stash **CC5D369E** (high, deliberation).

**Scope:** planning + backlog re-harvest ONLY. No code, no cargo, no PR. MCP unavailable → CLI `C:\Tools\backlogit.exe`.
Authoritative source: `docs/exec-plans/2026-07-10-callgraph-cross-file-resolution-plan.md` (§5 decomposition,
§6 Constitution Check, §7 risk table + rollback, §8 SUPERSEDED block). Plan file NOT edited (already expanded in #239).

## What changed this session

### Over-limit tasks split into single-width items
- **082.002-T** rescoped → **STAGING CAPTURE**: `staged_call { caller_id, callee_name, source_file => created_at }`
  relation + record-unresolved on the index (`code_graph.rs:466-475`) and sync (`:1070-1077`) paths.
  Direct in-file edges unchanged (provenance added by 082.003-T). 4 scenarios. Files: schema.rs + code_graph.rs +
  cozo_queries.rs + test. Deps: [082.001-T].
- **082.009-T** NEW → **STAGING LIFECYCLE**: clear-before-reindex, file-deletion cleanup, and RETRACT stale
  `calls_resolved_singleton` edges while old symbol ids still exist (reindex/delete paths delete function metadata
  but NOT call edges — code_graph.rs:294-300 / :1181-1205). Tests changed/deleted caller+callee after the post-pass.
  Deps: [082.002-T, 082.003-T, 082.008-T].
- **082.003-T** rescoped → **PROVENANCE STORAGE (DB layer)**: `calls_edge.resolution` column migration (schema.rs,
  default existing rows to `direct`), persist `direct` on create_calls_edge (cozo_queries.rs:1206-1214), and a
  `count_calls_edges_by_resolution` + enumerate read query returning RAW rows (not CodeEdge). 4 scenarios.
  Deps: [082.002-T]. Labels width:indexing→width:schema. **Re-split per Copilot #240 finding 1** — the model half
  (CodeEdge.resolution + dehydration + proptest) moved out to the new task 082.011-T to keep this ≤3 files.
- **082.011-T** NEW (Copilot #240 finding 1) → **MODEL/SERIALIZATION**: add `CodeEdge.resolution: Option<String>`
  (code_edge.rs:36, serde skip-if-none + default) + update the two CodeEdge struct literals in dehydration.rs (:595,
  :603) + the proptest_models.rs prop_map literal (:323). 3 scenarios; width:models. Deps: [082.003-T]. Note: rustc
  will also require `resolution: None` on the 3 pre-existing generic edge-read CodeEdge constructors in cozo_queries.rs
  (mechanical, co-located with 082.003-T's cozo_queries.rs surface).
- **082.008-T** NEW → **POST-PASS resolution**: `reresolve_calls_edges` modeled on `reresolve_references_edges`
  (:1357), unambiguous-name-only, tags exact `calls_resolved_singleton`; invoked in FULL/`--force` index path only
  (alongside :543), skipped in incremental sync (:1152). Deps: **[082.003-T, 082.011-T]** (needs both DB column and
  model field).
- **082.010-T** NEW → **ROLLBACK down-migration**: retract calls_resolved_singleton edges + drop/ignore the
  resolution column before/during a reverting reindex (plan §7). Deps: [082.003-T, 082.008-T]. **Scenarios reordered
  per Copilot #240 finding 2** to 3, so count_calls_edges_by_resolution asserts zero BEFORE the column is dropped,
  then old-schema-writer round-trip, then idempotent rerun.

### 082.004-T reduced 5 → 4 scenarios
Folded the standalone "calls_resolved_singleton edges counted" assertion into the expected-edges manifest-match
scenario (target-correctness + count in one). Kept: (1) post>pre resolution_recall; (2) aggregate false_edge_rate ≤
threshold (LOWER-BOUND signal only, per follow-up D07F0919 — `count_dangling_calls_edges` cozo_queries.rs:2418 cannot
detect mis-resolution); (3) every calls_resolved_singleton edge matches the fixture expected-edges manifest AND count
== manifest size; (4) ambiguous names contribute no edge. Deps repointed [082.003-T,081.001-T,081.005-T] →
**[082.008-T, 082.009-T, 081.001-T, 081.005-T]**.

### Contract propagation (Copilot J-w/J_M finding)
Full `staged_call` lifecycle + `resolution` migration + read-query contract propagated INTO the executable task
BODIES (not just the plan). Canonical stored provenance string is EXACTLY `calls_resolved_singleton` everywhere;
`direct` for in-file edges. Go extractor path cited as `src/services/parsing/go_lang.rs` (NOT go.rs).

### Fan-out tasks rewired
082.005-T (Python), 082.006-T (TypeScript), 082.007-T (Go) dependency 082.003-T → **082.008-T** (post-pass).
Still deferred; NOT in 078-S; 082-F stays active for a follow-on fan-out shipment.

## Final 082-F dependency DAG (verified acyclic via item_deps query)
```
082-F         depends_on 081-F
082.001-T     (method-call capture; no intra-082 dep)
082.002-T  ─▶ 082.001-T
082.003-T  ─▶ 082.002-T
082.011-T  ─▶ 082.003-T
082.008-T  ─▶ 082.003-T, 082.011-T
082.009-T  ─▶ 082.002-T, 082.003-T, 082.008-T
082.010-T  ─▶ 082.003-T, 082.008-T
082.004-T  ─▶ 082.008-T, 082.009-T, 081.001-T, 081.005-T
082.005-T  ─▶ 082.008-T   (deferred)
082.006-T  ─▶ 082.008-T   (deferred)
082.007-T  ─▶ 082.008-T   (deferred)
```
Clean forward chain 001→002→003→011→008→{009,010,004,005,006,007}. No cycles. Cross-feature acceptance gate preserved
(082.004-T still depends on eval S1=081.001-T + S3=081.005-T, both already shipped/archived on main).

## 078-S manifest (rewired, now QUEUED)
`custom_fields.items` = **[082.001-T, 082.002-T, 082.003-T, 082.011-T, 082.008-T, 082.009-T, 082.010-T, 082.004-T]**
(execution order; 082.011-T inserted per Copilot #240). status `blocked` → `queued`. Body rewritten to a "re-harvest
complete" note. 082-F NOT in the manifest and NOT archived by this shipment (deferred fan-out children remain queued
under it).

## Stash
- **CC5D369E** archived with `reason:harvested`, `harvested_artifact_id:082-F` (split tasks 082.008/009/010/011-T +
  rescoped 002/003/004-T). Remaining active stash entries are all 081-F Copilot follow-ups (D6F70DCC, 88B5FAFD,
  F137D72E, D07F0919, 54848E3D, 4CF046A5, 78AA205D, CA401F5F, 2894ACB5, 14B33F9F, 635EE7C0, 00C7F3CC, 2C420C96,
  30CE5DD6) plus B0E2B374 (DAX) / 30F372C8 (CLI-MCP parity) — NOT touched this session.

## Protected files (left untouched, unstaged)
`.github/agents/auto-mergeinstall.agent.md` and `.github/agents/auto-tune.agent.md` — the two intentional deletions.
Committed with explicit per-file `git add` only.

## Next steps (Orchestrator / Ship)
- The eval-subsystem gate (SHIP-1 / 077-S / 081-F, including 081.001-T + 081.005-T) is **ALREADY SHIPPED and archived
  on main** (in `.backlogit/archive/`); 078-S's cross-feature dependency is satisfied. Do NOT defer 078-S waiting on
  that gate — it is claimable as soon as this re-harvest branch merges.
- Ship: route **078-S (082-F rec1)** directly once merged; SHIP-1 (081-F eval subsystem) is already delivered.
- Follow-on: a separate shipment for the deferred fan-out (082.005/006/007-T) once slice-1 lands; only then archive 082-F.
- No PR opened this session (Orchestrator to gate).

## Copilot review #240 follow-up (this session)
Fixed the 5 findings on the same branch: (1) split 082.003-T into DB-layer (082.003-T) + model/serialization
(082.011-T NEW), wired 011→003, 008→{003,011}, and inserted 011 into 078-S; (2) reordered 082.010-T rollback
scenarios so the provenance count runs before the column drop (now 3 scenarios); (3) reworded 078-S body — eval gate
already shipped, not an unmet prerequisite; (4) same staleness fix in this doc's next-steps; (5) aligned
`.backlogit/archive/stash.jsonl` CC5D369E to `reason:harvested` + `harvested_artifact_id:082-F`.
