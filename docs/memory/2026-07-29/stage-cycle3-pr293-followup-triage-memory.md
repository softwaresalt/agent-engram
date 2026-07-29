# Stage session memory — Cycle 3: PR #293 post-merge follow-up triage (2026-07-29)

## Context
Burn-through cycle. Fresh 5-entry stash of PR #293 review follow-ups (094-S/101-F
+ 093-S/102-F merged & shipped). Full pipeline: triage → deliberation/spike where
warranted → impl-plan → plan-harden (risk-triggered) → plan-review gate → harvest
→ queued shipment(s).

Step 0: `ALL_TOOLS_OK` (backlogit 1.7.0 + registry CLI fallbacks). Index synced
(783 artifacts start; 78x end). Confirmed Ship closed 094-S + 093-S (closure memos
present). No source/build/PR/branch work (Stage role boundary). `start.ps1`
untouched. Planning artifacts left UNCOMMITTED (Orchestrator commits).

## Triage outcomes (5 entries)

| Stash | Kind/prio | Disposition | Artifacts |
|---|---|---|---|
| 685FAA80 | task/med | ADVANCED → 103-F (grouped w/ 92EE75BB) | 103.001-T; shipment **096-S** |
| 92EE75BB | task/low | ADVANCED → 103-F (grouped w/ 685FAA80) | 103.002-T; shipment **096-S** |
| BE366218 | bug/med | ADVANCED → 104-F | 104.001-T/002-T; shipment **095-S** |
| D2416925 | task/low | RESOLVED IN-CYCLE (Stage-domain) | convention doc + provenance comments; archived |
| 99AFF44B | task/low | DEFERRED → deliberation 017-D (+spike shape) | 017-D; stash stays active |

## Grouping / width decisions
- **685FAA80 + 92EE75BB → one feature (103-F).** Both harden the SAME surface:
  the 101-F forced-index / `--revalidate-code-graph` certify path, gating the
  `code_graph_extraction_generation` marker advance. Same width (code-graph
  reconciliation in `code_graph.rs` + `cozo_queries.rs`). Two tasks, chained
  (U2 file-set reconcile → then U1 orphan sweep in the same pass; task dep
  103.002-T blocks-after 103.001-T to serialize same-file edits).
- **BE366218 → separate feature (104-F).** Daemon lifecycle / sync-queue state
  machine (`lifecycle.rs` + `write.rs`) — different width from 103-F. Not merged.
  `related_to` 015-D (5765BAAB daemon-index spike; different defect, same width).
- **99AFF44B kept out of any shipment** — major dependency bump, unbounded blast
  radius (see below).

## Key technical grounding (verified in-tree this cycle)
- **685FAA80:** no global orphan-GC exists for `calls_edge` — `rm_orphan_edges`
  (`cozo_queries.rs:5973`) is lineage-only; `count_dangling_calls_edges` (3576)
  only counts. Design: new `retract_dangling_calls_edges` (retract rows where
  `not has_def[from] or not has_def[to]`), wired into BOTH certify blocks
  (`index_workspace` ~1900; `--revalidate` sync ~2923) before
  `set_code_graph_extraction_generation`. Retraction-only, no live-endpoint edge
  removed (no recall loss), fail-closed.
- **92EE75BB:** the forced-index route (`index_workspace`) walks only discovered
  files; the indexed-vs-current file-set reconciliation (Phase 1 deletion,
  ~2156, `handle_deleted_file`) lives ONLY in the sync path. Newly-EXCLUDED
  still-on-disk file never self-heals (deletion phase only fires for on-disk
  deletions) → stale same-file edge falsely certified. Fix: reconcile prior
  indexed set on the forced/revalidate route before marker advance, reusing
  `handle_deleted_file`. Gated `force || !any_hash_skipped` so a partial index
  never evicts.
- **BE366218:** `drain_pending_sync` (`lifecycle.rs:389`) single-shot; cancel
  path (~248) + DB-connect-fail path (~261) call `finish_indexing()` WITHOUT
  draining → companion bits (`pending_sync_revalidate` /
  `pending_sync_backfill_python`, set in `write.rs` ~283-289) leak sticky into a
  later routine sync (spurious heavy revalidate/backfill); re-arm during drain
  (~433) relies on an unspecified "next caller" → stall. Fix (locked): CLEAR-all
  atomically on cancel/DB-fail; bounded LOOP-drain on normal completion; preserve
  `write.rs` publish ordering. RED (104.001-T) → GREEN (104.002-T).
- **99AFF44B:** cozo pinned `0.7` (`Cargo.toml:31`). lz4_flex RUSTSEC-2026-0041
  via non-optional `swapvec` — no in-range fix; needs cozo 0.8+ major bump
  (storage-sqlite/graph-algo + thousands of lines CozoScript) — unbounded, can't
  honestly scope to ≤2h. Already accepted-with-rationale (102-F triage;
  cozo-internal disk-spill, trusted round-trip, non-blocking CI). DEFER: Option C
  now → Option B (upstream fix) opportunistically → Option A only after a
  Ship/runtime spike that verifies the bump even re-pins lz4_flex and scopes the
  API delta. Not scheduled.

## D2416925 — resolved in-cycle (Stage-domain)
`backlogit stash archive` has NO `--reason` flag (confirmed) → harvest link is
dropped on archival. Resolved by: (1) convention doc
`docs/decisions/2026-07-29-stage-harvest-provenance-convention.md` (append a
harvest-provenance comment to the promoted artifact BEFORE archiving; record the
mapping in session memory); (2) retroactive provenance comments on 101-F
(existing) and 102-F (added this cycle for F97D51DF); (3) applied the convention
prospectively (103-F/104-F carry harvest comments). No Ship shipment — pure Stage
bookkeeping.

## Deliverables
- Queued shipment **095-S** — 104-F daemon pending-sync drain hardening
  (104.001-T RED, 104.002-T GREEN). Medium.
- Queued shipment **096-S** — 103-F forced-index certify-path completeness
  (103.001-T orphan sweep, 103.002-T file-set reconcile). Medium.
- Deliberation **017-D** — cozo 0.8+ bump feasibility (deferred).
- Deferred (stash active, deliberation-linked): 015-D (5765BAAB), 016-D
  (B94772CB), 017-D (99AFF44B).
- Archived: 685FAA80, 92EE75BB, BE366218 (harvested), D2416925 (resolved).

## Recommended Ship execution order
1. **095-S (104-F) FIRST** — live daemon correctness bug: companion-state leak
   can trigger spurious heavy revalidate/backfill passes on unrelated syncs +
   drain stall. Highest operational impact.
2. **096-S (103-F)** — certify-path completeness/hygiene follow-up to the
   freshly-merged 101-F marker.
- **Independent** (disjoint file sets: `lifecycle.rs`/`write.rs` vs
  `code_graph.rs`/`cozo_queries.rs`) — no inter-shipment dependency; order is a
  soft preference. Within each shipment tasks are dependency-chained (RED→GREEN;
  reconcile→sweep).
- Next investigative step remains the **015-D** daemon-index hands-on spike
  (needs a live daemon repro env) — adjacent to 104-F.

## Notes
- All Cycle-3 docs + backlog markdown are in the WORKING TREE only — NOT
  committed (Orchestrator commits before Ship branches).
- `start.ps1` not touched.
