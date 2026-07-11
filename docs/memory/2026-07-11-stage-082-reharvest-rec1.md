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
  default existing rows to `direct`), persist `direct` on the 2-arg create_calls_edge (cozo_queries.rs:1206-1214),
  add a resolution-accepting writer `create_calls_edge_with_resolution(from,to,resolution)` (reused by post-pass 008 +
  rehydrate 012 without breaking the ~30 existing 2-arg callers), and a `count_calls_edges_by_resolution` + enumerate
  read query returning RAW rows (not CodeEdge). 4 scenarios. Deps: [082.002-T]. Labels width:indexing→width:schema.
  **Re-split per Copilot #240 finding 1** — the model half moved out to 082.011-T to keep this ≤3 files.
- **082.011-T** NEW (Copilot #240 finding 1; expanded cycle-3) → **MODEL + JSONL EXPORT**: add
  `CodeEdge.resolution: Option<String>` (code_edge.rs:36) + thread it through the export path — add `resolution` to the
  intermediate `EdgeLine` struct (dehydration.rs:230) and copy it in the `serialize_edges_jsonl` mapping (:390) so
  provenance is not dropped on export — + the two CodeEdge test literals (:595,:603) + proptest_models.rs (:323) +
  `resolution: None` on the 3 generic edge-read CodeEdge constructors in cozo_queries.rs. 3 scenarios; width:models.
  Deps: [082.003-T]. (Files: code_edge.rs + dehydration.rs + cozo_queries.rs = 3 source + proptest test.)
- **082.012-T** NEW (Copilot #240 cycle-3 finding 1) → **REHYDRATE PROVENANCE**: add `resolution` to `ParsedEdge`
  (hydration.rs:296) and carry it through the calls upsert (:459-463) via the `create_calls_edge_with_resolution`
  writer, so provenance survives a daemon restart/rehydrate instead of being reset to `direct`. 3 scenarios;
  width:hydration. Deps: **[082.003-T, 082.011-T]**. (Files: hydration.rs + rehydrate test.)
- **082.008-T** NEW → **POST-PASS resolution**: `reresolve_calls_edges` modeled on `reresolve_references_edges`
  (:1357), unambiguous-name-only, tags exact `calls_resolved_singleton`; invoked in FULL/`--force` index path only
  (alongside :543), skipped in incremental sync (:1152). Deps: **[082.003-T, 082.011-T]** (needs both DB column and
  model field).
- **082.010-T** NEW → **ROLLBACK down-migration LOGIC**: retract calls_resolved_singleton edges + drop/ignore the
  resolution column before/during a reverting reindex (plan §7), exposed as the named `rollback_calls_resolution`
  entry point. Deps: [082.003-T, 082.008-T]. **Scenarios reordered per Copilot #240 finding 2** to 3 (count asserts
  zero BEFORE the column is dropped, then old-schema-writer round-trip, then idempotent rerun).
- **082.013-T** NEW (Copilot #240 cycle-3 finding 2) → **ROLLBACK CLI TRIGGER**: operator-invocable maintenance
  subcommand (cli/commands/migrate.rs + mod.rs + engram.rs) that calls `rollback_calls_resolution`, so the rollback is
  operational not dead code. 3 scenarios; width:cli. Deps: **[082.010-T]**. (Files: migrate.rs + mod.rs + engram.rs =
  3 source + CLI test.)

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
082.012-T  ─▶ 082.003-T, 082.011-T
082.008-T  ─▶ 082.003-T, 082.011-T
082.009-T  ─▶ 082.002-T, 082.003-T, 082.008-T
082.010-T  ─▶ 082.003-T, 082.008-T
082.013-T  ─▶ 082.010-T
082.004-T  ─▶ 082.008-T, 082.009-T, 081.001-T, 081.005-T
082.005-T  ─▶ 082.008-T   (deferred)
082.006-T  ─▶ 082.008-T   (deferred)
082.007-T  ─▶ 082.008-T   (deferred)
```
Clean forward chain 001→002→003→011→{012,008}→009→{010→013}→004. No cycles. Cross-feature acceptance gate preserved
(082.004-T still depends on eval S1=081.001-T + S3=081.005-T, both already shipped/archived on main).

## 078-S manifest (rewired, now QUEUED)
`custom_fields.items` = **[082.001-T, 082.002-T, 082.003-T, 082.011-T, 082.012-T, 082.008-T, 082.009-T, 082.010-T, 082.013-T, 082.004-T]**
(execution order; 082.011-T + 082.012-T + 082.013-T inserted per Copilot #240 cycles 2-3). status `blocked` → `queued`.
Body rewritten to a "re-harvest complete" note. 082-F NOT in the manifest and NOT archived by this shipment (deferred
fan-out children remain queued under it).

## Stash
- **CC5D369E** archived with `reason:harvested`, `harvested_artifact_id:082-F` (split tasks
  082.008/009/010/011/012/013-T + rescoped 002/003/004-T). Remaining active stash entries are all 081-F Copilot
  follow-ups (D6F70DCC, 88B5FAFD, F137D72E, D07F0919, 54848E3D, 4CF046A5, 78AA205D, CA401F5F, 2894ACB5, 14B33F9F,
  635EE7C0, 00C7F3CC, 2C420C96, 30CE5DD6) plus B0E2B374 (DAX) / 30F372C8 (CLI-MCP parity) — NOT touched this session.

## Protected files (pre-existing deletions, intentionally left unstaged)
`.github/agents/auto-mergeinstall.agent.md` and `.github/agents/auto-tune.agent.md` were deleted in a
prior session and are intentionally kept as UNCOMMITTED deletions in the working tree (status `D`).
This branch does NOT modify, restore, or commit them — they remain unstaged across every commit here.
All backlog commits used explicit per-file `git add` (never `git add -A`) to preserve that state.

## Next steps (Orchestrator / Ship)
- The eval-subsystem gate (SHIP-1 / 077-S / 081-F, including 081.001-T + 081.005-T) is **ALREADY SHIPPED and archived
  on main** (in `.backlogit/archive/`); 078-S's cross-feature dependency is satisfied. Do NOT defer 078-S waiting on
  that gate — it is claimable as soon as this re-harvest branch merges.
- Ship: route **078-S (082-F rec1)** directly once merged; SHIP-1 (081-F eval subsystem) is already delivered.
- Follow-on: a separate shipment for the deferred fan-out (082.005/006/007-T) once slice-1 lands; only then archive 082-F.
- No PR opened this session (Orchestrator to gate).

## Copilot review #240 follow-up (this session)
Cycle 2 — fixed 5 findings: (1) split 082.003-T into DB-layer (082.003-T) + model/serialization
(082.011-T NEW), wired 011→003, 008→{003,011}, and inserted 011 into 078-S; (2) reordered 082.010-T rollback
scenarios so the provenance count runs before the column drop (now 3 scenarios); (3) reworded 078-S body — eval gate
already shipped, not an unmet prerequisite; (4) same staleness fix in this doc's next-steps; (5) aligned
`.backlogit/archive/stash.jsonl` CC5D369E to `reason:harvested` + `harvested_artifact_id:082-F`.

Cycle 3 — fixed 2 substantive decomposition defects + 1 staleness: (1) **provenance end-to-end** — 082.011-T's
export claim was unsatisfiable (only edited CodeEdge test literals; the serialized `EdgeLine` struct + `serialize_edges_jsonl`
mapping dropped `resolution`, and `hydration.rs` dropped it on restart). Expanded 082.011-T to own the full export
path within its ≤3-source cap (EdgeLine :230 + serialize mapping :390, all in dehydration.rs) and created **082.012-T**
(rehydrate: ParsedEdge :296 + calls upsert :459-463 via the new `create_calls_edge_with_resolution` writer added to
082.003-T; deps [003,011]); inserted 012 into 078-S after 011. (2) **rollback trigger** — 082.010-T had no invocation
path; kept it as reusable logic exposing `rollback_calls_resolution` and split the operator trigger into **082.013-T**
(maintenance CLI subcommand `engram migrate-down calls-resolution`; deps [010]); inserted 013 into 078-S after 010.
(3) updated 082.005-T handoff summary to the full current manifest. DAG re-verified acyclic; every construction site
kept buildable-per-commit (2-arg create_calls_edge signature unchanged, ~30 callers untouched).
