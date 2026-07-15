# Stage triage & release plan — 12 stash entries (2026-07-14)

Agent: **Stage** (autonomous, DARK_MODE). Operator AFK, autonomous judgment
authorized. Role boundary honored: **no production/test code changes, no builds,
no PRs, no Ship work.** All mutations are backlog + planning artifacts only.

Backlogit: v1.6.0 (MCP + CLI). Engram daemon: green, workspace bound, model
loaded. Index synced (656 items) at session start and end.

## Outcome summary

- **12/12** active stash entries consumed and **archived** (not deleted) to
  `.backlogit/archive/stash.jsonl`. Active stash is now empty.
- Created **5 features** (086-F..090-F), **20 tasks**, **1 deliberation**
  (013-D), **1 cross-cutting adversarial plan-review gate** (088.001-R = PASS),
  and **5 queued shipments** (081-S..085-S).
- **2** items intentionally **blocked/deferred** and excluded from all shipments
  (088.001-T perf-spike, 090.004-T post-audit).
- No destructive actions. No shipment contains mutually conflicting work.

## Tool / state gate (Step 0)

| Check | Result |
|---|---|
| `.autoharness/backlog-registry.yaml` | present; MCP `backlogit` + CLI fallback declared |
| backlogit MCP | `TOOL_OK` (get_version 1.6.0) |
| backlogit / engram CLI | `TOOL_OK` (`C:\Tools\backlogit.exe`, `C:\Tools\engram.exe`) |
| engram daemon | green; workspace `C:\Source\GitHub\engram` bound; model `bge-small-en-v1.5` |
| Index sync | `INDEX_SYNC_OK` (656) start; re-synced at end |
| Overall | `ALL_TOOLS_OK` |

## Triage dispositions (all 12)

| Stash | Pri | Kind | Verified current-state (evidence) | Disposition | Item | Shipment |
|---|---|---|---|---|---|---|
| 2323C72A | high | feature | method/qualified calls marked (is_method/is_qualified) but not promoted to calls_edge | harvest → feature + 4 tasks (design-gated) | 088-F | 081-S |
| E1A9ED33 | med | feature | no `staged_call` in any JSONL export/dehydration path | harvest → feature + 3 tasks | 089-F | 084-S |
| 8506BC68 | med | task | `migrate_calls_edge_resolution`/`rollback_…` call `run_script` directly (schema.rs L296-298/L333-335) | harvest → task | 086.001-T | 082-S |
| 30CE5DD6 | med | task | `connect_db_open_lock` mutex exists but no bounded reopen-retry (mod.rs L110/L187) | harvest → task (PLAN-HARDEN) | 086.002-T | 082-S |
| 5C1EDA41 | med | task | DaemonLock workspace-rooted; direct.rs L88-90 admits shared-dir cross-workspace race | harvest → task (fail-closed decision) | 086.003-T | 082-S |
| 2C420C96 | low | task | lifecycle.rs L515 reads retrieval_eval_enabled separately; no snapshot_dispatch_context | harvest → task | 086.004-T | 082-S |
| 9F001621 | med | task | `compute_dirty_model_scopes` exists, no `TMDL_DAX_INDEX_VERSION` anywhere | harvest → task | 087.001-T | 083-S |
| 0C98C5F1 | med | task | dax.rs L12-13 only ignores `//` and `/* */`, not `--` | harvest → task | 087.002-T | 083-S |
| 874F8112 | med | task | tools_catalog.rs L137 summary code-only despite powerbi_node_id param (L231) | harvest → task (docs) | 087.003-T | 083-S |
| B832AC66 | med | task | collect_recursive L118 `is_dir()` follows symlinks (cycle recursion) | harvest → task | 087.004-T | 083-S |
| 30F372C8 | med | feature | gap list STALE — lint-dax CLI + query-graph neighborhood now exist | harvest → feature + 3 tasks (+1 blocked) | 090-F | 085-S |
| 2C949608 | low | feature | post-pass runs full/force only, not incremental sync | harvest → **blocked** task (perf spike) | 088.001-T | none |

All entries verified individually against current `main` via engram/grep; **none
already-completed**. No duplicates against existing backlog.

## Release units (features → shipments), recommended execution order

1. **081-S** (HIGH) ← 088-F rec1 recall recovery. Design 013-D. **Release gate:
   088.005-T eval** (recall↑, precision not↓). Runtime-affecting.
2. **082-S** (MED) ← 086-F runtime reliability & concurrency (DB open retry,
   migration retry-routing, fail-closed migrate-down guard, status atomicity).
   Runtime-affecting. 086.002-T flagged **plan-harden**.
3. **083-S** (MED) ← 087-F PowerBI/DAX PR#246 followups (index-version re-index,
   `--` comments, impact docs, symlink-safe traversal). Runtime-affecting subset.
4. **084-S** (MED) ← 089-F durable staged_call via JSONL. Runtime-affecting
   (additive format). Soft-couples into 081-S.
5. **085-S** (MED) ← 090-F CLI↔MCP parity audit/doc/guard.

Soft coupling: **084-S informs 081-S** (durable staging strengthens resolver
revalidation). No hard dependency; if Ship batches the rec1 subsystem, prefer
084-S before 081-S. Cross-shipment ordering is otherwise by priority.

## Material technical decisions (adversarially reviewed)

1. **Interim SQLITE_BUSY mitigations (086.001-T, 086.002-T) vs blocked 041-F.**
   041-F removes SQLITE_BUSY mitigations once cozo≥0.8 (unreleased). Decision:
   ship interim fixes now (real Windows reliability bugs), record `informs` links
   to 041.002-T so its future removal is complete. Not an in-shipment conflict.
2. **086.003-T fail-closed vs full exclusivity lock.** Chosen: **fail-closed** —
   reject destructive migrate-down on a shared/external resolved data dir with a
   clear error (non-destructive, minimal). Deferred alternative: DB-path-keyed
   exclusivity lock held by all writers (larger design), open only if a real
   shared-data-dir multi-daemon use case emerges.
3. **088-F resolver = qualified-name exact, singleton-only promotion** (013-D
   Option A) over receiver-type inference (B) / overload resolution (C).
   Preserves the no-false-edge invariant; eval-gated (088.005-T). B/C deferred.
4. **087.004-T = symlink_metadata classify + canonical visited-set + containment**
   (NOT blind no-follow), so legitimate in-workspace symlinked sources are kept
   while cycles/escapes are impossible.
5. **30F372C8 stale-gap → audit-first.** Only audit (090.001-T), mapping doc,
   drift-guard (090.002-T), doc-parity (090.003-T) harvested; functional
   gap-closing kept **blocked** (090.004-T) pending audit findings.

## Adversarial plan-review gate → 088.001-R (PASS)

Single-agent review (multi-model consensus not available this session — F9;
Ship review + Copilot PR review provide downstream cross-opinion). No P0. All
HIGH-confidence P0/P1 remediated pre-harvest.

| # | Sev/Conf | Finding | Disposition |
|---|---|---|---|
| F1 | P1/HIGH | 088-F precision regression (false singleton edges) | REMEDIATED: singleton-only + eval gate 088.005-T blocks release |
| F2 | P1/HIGH | interim SQLITE_BUSY mitigations vs 041-F removal | REMEDIATED: `informs` links to 041.002-T; 041-F out of scope |
| F3 | P1/MED | 086.003 fail-closed changes behavior | ACCEPTED: refusal is non-destructive; needs clear operator error |
| F4 | P2/MED | 087.004 naive no-follow drops legit symlinks | REMEDIATED: classify + visited-set + containment |
| F5 | P2/HIGH | 090-F gap list stale | REMEDIATED: audit-first; 090.004 blocked |
| F6 | P2/MED | 087-F groups 4 heterogeneous followups | ACCEPTED: single-width, independent files, themed batch |
| F7 | P2/MED | 087.001 one-time re-index latency | ACCEPTED: monitor duration; rollback=revert version bump |
| F8 | P1/HIGH | 086.002 retry on shared DB-open path | REMEDIATED: plan-harden (bounded+backoff+classify+give-up err) |
| F9 | P3/MED | no multi-model consensus | DEFERRED: downstream review provides consensus |
| F10 | P2/MED | 089-F JSONL format change compat | REMEDIATED: rehydrate tolerates legacy JSONL |
| F11 | P3/LOW | session ~20 tasks (soft stop) | ACCEPTED: bounded deliberate authoring, no execution loop |

## Constitution checks

- **II Test-First:** every executable task leads with a failing test/fixture in
  its acceptance criteria; eval-gate 088.005-T is explicit.
- **I Safety-First Rust:** tasks require `forbid(unsafe_code)`, no unwrap/expect,
  `Result<T, EngramError>` — restated in task acceptance/impl notes.
- **III Workspace Isolation:** 086.003-T + 087.004-T *strengthen* containment.
- **V Destructive Approval:** no destructive action taken; 086.003-T makes a
  destructive migration fail-closed rather than racy.
- **Task Granularity:** each task single-width / one concern / ≤~2h; template vs
  DB vs docs vs test widths kept isolated.

## Release-observability (runtime-affecting units)

| Unit | Monitor | Rollback trigger |
|---|---|---|
| 081-S (088-F) | calls_resolved_singleton edge counts; eval recall/precision | precision regression → revert resolver via calls_edge resolution rollback |
| 082-S (086-F) | daemon startup success rate; SQLITE_BUSY incidence | startup stall / masked error → revert per-task |
| 083-S (087.001/087.004) | one-time re-index duration; collection time | re-index blowup → revert TMDL version bump; traversal regression → revert |
| 084-S (089-F) | export/rehydrate round-trip on upgrade | rehydrate corruption → skip staged_call rehydrate (legacy path) |

## Deferred / blocked (with reasons)

- **088.001-T** (blocked): incremental-sync post-pass — needs a **perf spike**
  (benchmark scoped post-pass vs full-index) that Stage cannot run under the
  no-build boundary. Unblock: run spike; re-harvest if within latency budget.
- **090.004-T** (blocked): functional parity gap-closing — scope is **audit-
  derived** (090.001-T). Re-harvest into ≤2h tasks after the audit; do not use
  the stale 2026-07-05 list.
- Related pre-existing blocked upstream (untouched): 025-S / 041-F (cozo≥0.8),
  033.005-T (tree-sitter-sequel), 082.005/6/7-T (peer-language fan-out).

## Backlog ID map

- Features: 086-F reliability, 087-F DAX/PBI, 088-F recall, 089-F durability,
  090-F parity.
- Tasks: 086.001-004-T; 087.001-004-T; 088.001-005-T (088.001 blocked);
  089.001-003-T; 090.001-004-T (090.004 blocked).
- Deliberation 013-D; Review gate 088.001-R; Shipments 081-S..085-S.
- Links: 086.001-T & 086.002-T `informs` 041.002-T; 089-F `informs` 088-F;
  013-D `informs` 088-F; 088.001-R `related_to` 086/087/089/090-F.

## Ship handoff

All five shipments are **queued and claimable**. Working tree carries only Stage
artifacts (docs + `.backlogit/`); `start.ps1` was pre-existing and untouched.
See `docs/memory/2026-07-14/stage-12-stash-triage-memory.md`.
