# Stage session memory — 087 PowerBI durability pair → shipment 100-S

Date: 2026-07-30
Agent: Stage (dark-factory / autonomous, planning + assembly ONLY)
Branch: `100-powerbi-durability` (from `main` @ `a70395c5`; main untouched)

## Objective

Give the two 083-S / PR #257 cycle-3 deferrals (087.005-T deletion-semantics,
087.006-T durability-contract) their dedicated, well-reviewed cycle: produce a
grounded impl-plan + plan-harden + plan-review (PASS), decompose where the 2-hour
rule requires, and assemble a queued shipment for Ship to claim. No build, no PR,
no push to main.

## Tool / session state

* backlogit MCP DEGRADED ("Transport closed") → used sanctioned CLI fallback
  `C:\Tools\backlogit.exe --no-update-check ...` (~30-40s/op) + read-only python
  sqlite3 SELECT on `.backlogit\backlogit.db`. `backlogit sync` run after every
  out-of-band edit. `DEGRADED_MODE: [backlogit-mcp]`. `INDEX_SYNC_OK` throughout.
* engram daemon healthy but code-graph index is fixtures-only → every code site
  re-verified via grep + direct file read (card line numbers had drifted).
* Next shipment ID allocated cleanly = **100-S** (highest prior = 099-S active).

## Verified real code sites (main a70395c5)

* Deletion sweeps (087.005): `source_traversal.rs` `collect_files_in_workspace`
  (L15), `collect_recursive` visited-set dedup (L62-65), `is_regular_file_in_workspace`
  (L44); physical-existence-only `compute_deleted_paths` in `powerbi_indexer.rs`
  (L90) + `notebook_indexer.rs` (L97); sweeps `sweep_deleted_powerbi_files`
  (L1594) / `sweep_deleted_notebook_files` (L407). Sweeps run ONLY in the
  full-index pass (`ingestion.rs` L129-137 notebook, L152-154 powerbi);
  `reactive_sync.rs` is markdown-only and never calls them → "collected this pass"
  is authoritative in the sweep context (unless a subtree read was silently
  skipped — the fail-open hazard closed by INV-2/INV-3).
* Hash-skip poisoning (087.006): `index_powerbi_source` (L1314) builds
  `existing_hashes` from persisted content rows (L1341-1346), hash-skips at
  L1414-1420; per-record `upsert_content_record` writes final hash before all
  records complete (TMDL L1486-1487, non-TMDL L1552). `upsert_content_record` is
  single-row `:put` (`cozo_queries.rs` L3728/3741).
* Fix precedents found in-repo: completion-marker `lineage_index_state`
  (cozo L6143) + `file_hash` (L4227); atomic batch `:put`
  `upsert_backlog_content_records` (L5577); schema `CREATE_*` convention in
  `src/db/cozo_backend/schema.rs` (CREATE_CONTENT_RECORD L1051, CREATE_FILE_HASH
  L1078, CREATE_SCHEMA_META L1038 version-key mechanism).

## Decisions

* **087.006 kept as ONE task** (single-width PowerBI durability contract): add
  `powerbi_file_index_state` completion-marker relation; source the hash-skip gate
  from the marker (not content rows); write marker last; clean marker on delete.
  Chosen over atomic batch `:put` because nodes/edges are separate calls; marker
  is strictly more robust and self-populates a one-time reprocess on upgrade
  (non-destructive migration, matches 087.001 precedent).
* **087.005 DECOMPOSED** (spans 2 sweep call-sites + shared fail-closed semantics):
  * `087.005.001-ST` Unit A — shared fail-closed reconciler + completeness-aware
    collector in `source_traversal.rs` (foundation, TDD). No deps.
  * `087.005.002-ST` Unit B — wire reconciler into notebook sweep. blocks-dep A.
  * `087.005.003-ST` Unit C — wire reconciler into non-TMDL PowerBI sweep. blocks-dep A.
* No dependency between 087.005-T and 087.006-T (orthogonal functions).
* Freeze-scope: pbip + backlog sweeps share the same alias-stale bug but are OUT
  of the two-card scope → recorded as plan-review P2-1 follow-up, NOT harvested.
  090/091 low-priority tail explicitly excluded.
* 087-F parent feature is archived → referenced, NOT reopened.

## Artifacts produced

* Plan: `docs/exec-plans/2026-07-30-087-powerbi-durability-plan.md`
  (impl-plan + `## Plan Hardening` [INV-1..6, PA-1..4 ProposedAction/ActionRisk,
  RF-1..8 regression fixtures] + `## Plan Review` **GATE: PASS**, P0/P1 none,
  P2-1/P2-2 + P3-1/P3-2 recorded).
* Backlog: subtasks 087.005.001-ST / .002-ST / .003-ST created under 087.005-T;
  deps `.002→.001`, `.003→.001` (blocks). Plan reference added to 087.005-T &
  087.006-T frontmatter (out-of-band edit + sync).
* Shipment **100-S** (status `queued`): items 087.005-T, 087.005.001-ST,
  087.005.002-ST, 087.005.003-ST, 087.006-T. No covering feature (087-F archived,
  intentionally not added).

## Guardrails honored

Did NOT build/implement, run harness-architect/build-feature, create a PR, or push
main. `main` still `a70395c5`. All commits confined to branch
`100-powerbi-durability`. No P-010 (operator requested planning only).

## Next steps (for Ship)

1. Claim shipment 100-S.
2. Execute A (087.005.001-ST) first (unblocks B & C), then B/C, and 087.006-T in
   parallel. TDD harness generation is Ship's job — RF-1..8 are the failing-first
   fixtures.
3. Verify P2-2 marker-hygiene coupling (Unit C sweep must also drop
   `powerbi_file_index_state` markers for swept paths).
4. Monitor sweep `removed` and powerbi ingested/unchanged counts on first
   post-deploy re-index; rollback trigger = any deletion of a still-collected path
   or missing-summaries recall regression.
5. Consider stashing P2-1 (extend reconciler to pbip/backlog sweeps) for a later cycle.
