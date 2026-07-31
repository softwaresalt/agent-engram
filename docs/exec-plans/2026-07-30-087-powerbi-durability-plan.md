---
title: "087 PowerBI durability pair — alias-stale deletion reconciliation + TMDL atomic content persistence"
type: impl-plan
date: 2026-07-30
cycle: "Stage cycle (post-099-S/097-F merge @ main a70395c5); 083-S / PR #257 cycle-3 deferrals given their dedicated cycle"
feature: 087-F (archived — referenced, NOT reopened)
width: "PowerBI/notebook source-indexing durability: shared source traversal + deletion sweeps (src/services/source_traversal.rs, notebook_indexer.rs, powerbi_indexer.rs) + PowerBI content-record atomicity (src/services/powerbi_indexer.rs + src/db cozo relation)"
status: "reviewed + hardened (plan-review GATE: PASS)"
harvest_targets:
  - 087.005-T   # deletion-sweep reconciliation (DECOMPOSED into 3 subtasks)
  - 087.006-T   # TMDL atomic content persistence (kept as single task)
relates_to: ["087-F", "087.001-T", "087.004-T", "083-S"]
tags:
  - powerbi
  - notebook
  - indexing
  - deletion-semantics
  - durability
  - data-loss
  - fail-closed
  - review-deferred
---

## Problem frame

Two medium tasks were deferred from the 083-S / PR #257 cycle-3 Copilot review as
"unsafe to rush during an AFK autonomous run." They are **not** unsafe to *plan* —
they are a contract/durability pair that was waiting for exactly this dedicated,
well-reviewed cycle. Both are grounded below against current `main`
(`a70395c5`) via targeted grep + direct source reads (the engram code-graph index
holds only test fixtures this session, so every code site was re-verified; card
line numbers had drifted and are corrected here).

### 087.005-T — alias-backed stale records survive the deletion sweeps (deletion-semantics)

The shared traversal `collect_files_in_workspace`
(`src/services/source_traversal.rs:15`) tracks visited **canonical** directories
in a `HashSet` and dedups the emitted file list
(`collect_recursive` visited-set gate at `source_traversal.rs:62-65`;
`files.sort(); files.dedup()` at `:33-34`). When a **directory symlink** aliases
an already-visited canonical dir, only ONE path per real directory is emitted —
the first-sorted alias wins.

The deletion sweeps, however, decide liveness **per stored record** using only a
**physical-existence** check:

* `compute_deleted_paths` (powerbi non-TMDL) at `powerbi_indexer.rs:90`
  keeps a record unless `!is_regular_file_in_workspace(candidate, canonical_root)`
  (`:110`).
* `compute_deleted_paths` (notebook) at `notebook_indexer.rs:97`, same
  `is_regular_file_in_workspace` gate at `:117`.
* `is_regular_file_in_workspace` (`source_traversal.rs:44`) returns `true` as long
  as the path is a physical regular file in-bounds.

Scenario: a notebook is indexed under path `A = z/shared.ipynb`. Later an
earlier-sorting **alias directory** `a/ -> z/` appears, so traversal now emits
only `B = a/shared.ipynb` and dedups `z/` away. The DB still holds the record
under `A`. The sweep checks `is_regular_file_in_workspace(z/shared.ipynb)` — the
physical file exists (real dir `z/`), so `A` is deemed live and **never removed**.
The index now carries BOTH the stale `A` and the fresh `B` for the same real file.

**Fix direction (from card):** reconcile stored paths against the set actually
**collected this pass** — delete records whose path was not collected — OR give
traversal a stable canonical identity. This is a **deletion-semantics change**:
the fail-closed risk is *wrongly deleting live records* when "collected this pass"
is not authoritative (partial/incremental collection).

**Authoritativeness is what makes this safe.** The sweeps run **only** in the
full-index ingestion pass: `run_ingestion` in `src/services/ingestion.rs` calls
`index_notebook_source` + `sweep_deleted_notebook_files` (`:129-137`) and
`index_powerbi_source` + `sweep_deleted_powerbi_files` (`:152-154`). The reactive
single-file path (`src/services/reactive_sync.rs`) handles **markdown only** and
never invokes these sweeps. So "collected this pass" in the sweep context is the
**complete** file set for the source — *provided the traversal did not silently
skip an unreadable subtree* (`collect_recursive` swallows read errors with a
`warn!` and continues, `source_traversal.rs:73-80`). That skip is the one way a
full pass can become non-authoritative, and it is the exact fail-open hazard this
plan closes.

### 087.006-T — partial content-record write poisons the hash-skip (durability-contract)

`index_powerbi_source` (`powerbi_indexer.rs:1314`) builds its change-detection map
`existing_hashes` **directly from persisted content rows**
(`select_content_records(...).map(|r| (r.file_path, r.content_hash))`,
`:1341-1346`) and hash-skips unchanged files (`unchanged` gate at `:1414-1420`).

For a changed TMDL file it builds `tmdl_records` (multiple `ContentRecord`s, each
carrying the file's **final** `content_hash = &hash`), upserts graph nodes/edges,
then writes each record one-by-one:
`for record in &tmdl_records { queries.upsert_content_record(record).await?; }`
(`:1486-1487`). The non-TMDL branch does the same per-summary
(`upsert_content_record(&record).await?` at `:1552`). Each
`upsert_content_record` is a **single-row** `:put content_record`
(`cozo_queries.rs:3728`, `:put` at `:3741`); nodes/edges are **separate**
`run_script` calls (`upsert_powerbi_nodes` / `upsert_powerbi_edges`).

If an early record persists but a later write fails and `?` aborts the function,
the **next run** rebuilds `existing_hashes` from the partial rows — the file_path
now maps to the **final** hash — so `unchanged == true` and the file is
**hash-skipped forever**, permanently missing the summaries that failed to write.
This is the P2 (TOCTOU/atomicity) previously deferred in the PR #246 pre-PR
adversarial review, re-surfaced in PR #257 cycle-3.

**Fix direction (from card):** persist a file's records atomically, OR write a
**completion marker** only after *every* graph + content write succeeds and gate
the hash-skip on that marker (interaction with the `existing_hashes` hash-skip).

## Requirements trace

| Source requirement | Implementation action | Unit |
|---|---|---|
| 087.005: reconcile stored paths vs collected-this-pass; delete records not collected | Add a shared, fail-closed `reconcile_deleted_paths` + completeness-aware collector in `source_traversal.rs`; union `physically-absent OR (authoritative AND not-collected)` | A |
| 087.005: never wrongly delete live records across incremental/sync | Gate not-collected deletion on an **authoritative complete** full-pass collection; incremental/reactive never calls the sweep (verified); fail-closed on partial collection and canonicalize failure | A, B, C |
| 087.005: notebook deletion sweep | Wire the reconciler into `sweep_deleted_notebook_files` | B |
| 087.005: non-TMDL PowerBI deletion sweep | Wire the reconciler into `sweep_deleted_powerbi_files` | C |
| 087.006: atomic persistence OR completion-marker gating the hash-skip | Add `powerbi_file_index_state` completion-marker relation; gate `unchanged` on marker+hash; write marker last; clean marker on delete | D |
| 087.006: interaction with `existing_hashes` hash-skip | Source the hash-skip map from the completion marker, not from content rows | D |

## Implementation Units

Each unit obeys the 2-hour rule (< 3 files, < 5 functions, < 4 test scenarios),
width isolation (single subsystem/domain), and an atomic verifiable milestone.
Execution posture is **test-first (TDD)** per constitution Principle II.

### Unit A — Shared fail-closed collected-set reconciler  →  subtask `087.005.001-T`

**Changes**

* In `src/services/source_traversal.rs`, add a **completeness-aware** collector,
  e.g. `collect_files_in_workspace_checked(dir, root, is_target) -> CollectedFiles`
  where `CollectedFiles { files: Vec<PathBuf>, complete: bool }`. `complete` is
  `false` if *any* directory read failed during traversal (the current
  `warn!`-and-continue branch, `source_traversal.rs:73-80`). Keep the existing
  `collect_files_in_workspace` as a thin wrapper (returns `.files`) so current
  callers are untouched.
* Add `reconcile_deleted_paths(stored_rel_paths, collected, workspace_root) -> Vec<String>`
  implementing the fail-closed union:
  * `physically_absent = !is_regular_file_in_workspace(root.join(P), canonical_root)`
  * `not_collected = P ∉ normalized(collected)` (normalize collected absolute
    paths to workspace-relative, `/`-separated, matching stored `file_path` form
    exactly as `index_*_source` produces it via `strip_prefix + replace('\\',"/")`)
  * `stale = physically_absent OR (collected.complete AND not_collected)`
* **Tighten the existing fail-open canonicalize hazard**: when
  `workspace_root.canonicalize()` fails, return an **empty** deletion set (today
  `compute_deleted_paths` returns *all* paths — fail-open mass-delete). The new
  reconciler MUST fail closed.

**Files:** `src/services/source_traversal.rs` (+ its `#[cfg(test)]` module).

**Tests (unit, proptest where natural):**
1. alias-superseded stored path (present on disk, but a different collected path
   canonicalizes to the same real file) → deleted when `complete == true`;
2. live stored path present in `collected` → retained;
3. `complete == false` (a subtree read failed) → only physically-absent paths
   deleted; aliased-stale retained (accept transient staleness over data-loss);
4. `workspace_root` canonicalize failure → empty deletion (no mass-delete).

**Milestone:** reconciler + completeness collector compile with failing→passing
tests; no sweep call-site changed yet.

### Unit B — Wire reconciler into the notebook deletion sweep  →  subtask `087.005.002-T`

**Changes**

* In `sweep_deleted_notebook_files` (`notebook_indexer.rs:407`), collect this
  pass via `collect_files_in_workspace_checked` over the source dir with the
  notebook target predicate, gather the source-scoped `known_paths` (already
  filtered by `r.source_path == source.path`, `:411`-region), call
  `reconcile_deleted_paths`, and delete via the existing
  `delete_content_records_by_scope(path, "notebook", &source.path)` (`:426`).
* Replace/retire the notebook-local `compute_deleted_paths` (`:97`) in favor of
  the shared reconciler (keep the workspace-relative escape guard behavior).

**Files:** `src/services/notebook_indexer.rs` (+ tests).

**Tests (integration, fixture with a directory symlink alias):**
1. alias creates a superseded record under the old path → sweep drops the stale
   record and retains the freshly-collected one;
2. a genuinely deleted notebook → still removed (physical-absence path intact);
3. a full pass with an unreadable subtree → live records under untouched subtrees
   are **retained** (fail-closed).

**Depends on:** Unit A.

### Unit C — Wire reconciler into the non-TMDL PowerBI deletion sweep  →  subtask `087.005.003-T`

**Changes**

* In `sweep_deleted_powerbi_files` (`powerbi_indexer.rs:1594`), same wiring:
  `collect_powerbi_files_in_workspace_checked` (or the shared checked collector
  with the powerbi predicate) → source-scoped `known_paths` (`:1600`-region) →
  `reconcile_deleted_paths` → `delete_content_records_by_scope(path,"powerbi",…)`
  + `delete_powerbi_nodes_by_file_path(path)` (`:1615`-region).
* Retire the powerbi-local `compute_deleted_paths` (`:90`) in favor of the shared
  reconciler.
* **Scope boundary:** the TMDL model-scope reconciliation inside
  `index_powerbi_source` (dirty-scope `prior_rel` delete loop at
  `powerbi_indexer.rs:1362-1373`) already reconciles against the collected file
  set and is **out of scope** — do not touch it.

**Files:** `src/services/powerbi_indexer.rs` (+ tests).

**Tests (integration, directory-symlink-alias fixture):**
1. alias-superseded powerbi record dropped; freshly-collected retained;
2. deleted `.pbix`/model file still removed;
3. non-authoritative pass retains untouched-subtree live records.

**Depends on:** Unit A.

### Unit D — PowerBI content-record atomicity via completion marker  →  task `087.006-T`

**Changes**

* **New relation** `powerbi_file_index_state { file_path, source_path => content_hash, completed_at }`
  declared as `CREATE_POWERBI_FILE_INDEX_STATE` in
  `src/db/cozo_backend/schema.rs` (mirrors `CREATE_FILE_HASH` at `:1078` and the
  `lineage_index_state` completion-marker pattern) and registered in the backend
  relation-bootstrap create list alongside the other `CREATE_*` relations.
* **Query methods** in `src/db/cozo_queries.rs`: `upsert_powerbi_index_state`,
  `select_powerbi_index_state(source) -> HashMap<file_path,content_hash>`, and
  `delete_powerbi_index_state_by_scope(file_path, source_path)` — routed through
  the same busy-tolerant mutable/immutable script helpers already used by
  `upsert_content_record` (`:3728`) / `select_content_records` (`:3829`) /
  `delete_content_records_by_scope` (`:4016`).
* **Gate rewire** in `index_powerbi_source` (`powerbi_indexer.rs:1314`):
  * Source the hash-skip map (`existing_hashes` used at `:1353`, `:1362`, `:1414`)
    from `select_powerbi_index_state` — i.e. a file counts as "previously
    indexed at hash H" **only if its completion marker says so**, decoupling the
    skip decision from the (possibly partial) content rows.
  * After **all** graph nodes + edges + content records for a file succeed
    (TMDL branch after `:1486-1487`; non-TMDL after `:1552`), write the marker
    LAST via `upsert_powerbi_index_state(rel_path, source.path, hash)`. If any
    prior write fails and `?` aborts, the marker is absent ⇒ next run recomputes
    `unchanged == false` ⇒ file is reprocessed (stale partial deleted + rebuilt).
    **Fail-closed:** marker absence forces safe re-work, never a skip.
  * Delete the marker wherever the file's content rows are deleted (hash-change
    delete at `:1502-1505`; dirty-scope pre-delete at `:1362-1373`) so a mid-flight
    failure cannot leave a stale marker.
* **Migration (self-populating, non-destructive):** existing indexed workspaces
  have content rows but no markers. The first post-upgrade full index sees every
  powerbi file as marker-absent ⇒ reprocesses once ⇒ writes markers. No data
  migration script and no `--force` needed. This matches the one-time-reprocess
  precedent set by 087.001-T (`TMDL_DAX_INDEX_VERSION`).

**Files:** `src/db/cozo_backend/schema.rs`, `src/db/cozo_queries.rs`,
`src/services/powerbi_indexer.rs` (+ tests). *If harness generation finds the
relation + bootstrap + gate rewire heavier than one focused session, Ship may
split the cozo-layer relation/methods from the indexer gate rewire; noted as a
caveat, not pre-decomposed — the concern stays single-width (PowerBI durability).*

**Tests:**
1. **unit/contract** — `powerbi_file_index_state` upsert/select/delete round-trips
   scoped by source;
2. **integration (partial-write → reprocess)** — simulate a mid-file write failure
   (no marker written); assert the next run does NOT hash-skip and fully
   materializes the previously-missing records;
3. **integration (steady state)** — unchanged file WITH a valid marker is
   hash-skipped (no needless reprocess); first-run-without-marker reprocesses once.

**Depends on:** none (orthogonal to Units A–C).

## Dependency Graph

```text
087.005-T (deletion-sweep reconciliation)
  ├── 087.005.001-T  Unit A  shared fail-closed reconciler        (foundation)
  ├── 087.005.002-T  Unit B  notebook sweep wiring     ── blocked_by ─▶ 001
  └── 087.005.003-T  Unit C  powerbi non-TMDL sweep    ── blocked_by ─▶ 001

087.006-T (TMDL atomic content persistence)  Unit D               (independent)
```

* Real edges only: B and C each `blocks`-depend on A (shared helper).
* No edge between 087.005-T and 087.006-T — orthogonal subsystems/functions.
* No cycles.

## Decisions and Rationale

1. **Completion marker over atomic batch `:put` for 087.006.** A batched
   `:put content_record` (precedent: `upsert_backlog_content_records`,
   `cozo_queries.rs:5577`) would make the *content rows* atomic, but nodes and
   edges are separate `run_script` calls — a failure between the content batch and
   the node/edge writes still leaves a partial graph that poisons the hash-skip.
   A completion marker written **after all three** succeed is strictly more robust
   and has direct in-repo precedent (`file_hash` `:1078`, `lineage_index_state`).
   Batch `:put` is recorded as an optional follow-up optimization (P3), not scope.
2. **Authoritative-collection gating over unconditional not-collected deletion for
   087.005.** The card's simplest reading — "delete records whose path was not
   collected" — is unsafe if the collection was partial. Gating not-collected
   deletion on `complete == true` (no swallowed read errors) preserves the fix's
   power for the alias case while making the data-loss path impossible; a
   non-authoritative pass degrades to the current physical-existence-only sweep.
3. **Shared reconciler in `source_traversal.rs` over per-collector copies.** The
   fail-closed logic must be identical across notebook and powerbi (and is reusable
   for pbip/backlog later). One tested helper prevents divergence — this is why A
   is a foundation subtask that B and C depend on.
4. **Reference, do not reopen, 087-F.** The parent feature is archived and its
   sibling deferrals (087.001-T index-version fingerprint, 087.004-T
   symlink-cycle-safe traversal — the change that *introduced* the canonical
   visited-set this plan reconciles against) are done. The two open tasks are
   harvested/decomposed in place under 087-F without reactivating it.
5. **Keep 087.006 as one task; decompose 087.005.** 087.006 is a single-width
   durability contract in one subsystem (+ its query support). 087.005 spans two
   sweep call-sites plus shared fail-closed semantics — the operator-flagged
   decomposition trigger — so it becomes A/B/C.

## Risks and Caveats

* **R1 (data-loss, 087.005):** wrongly deleting live records. *Mitigation:*
  authoritative-complete gating + fail-closed on canonicalize failure + sweeps
  never run in incremental/reactive context (verified). Regression fixtures
  encode all three fail-closed paths.
* **R2 (durability, 087.006):** permanent hash-skip of missing summaries.
  *Mitigation:* marker-gated skip, marker-last write, marker cleanup on delete;
  partial-write regression fixture proves reprocess.
* **R3 (one-time reprocess cost, 087.006):** first post-upgrade index reprocesses
  all powerbi files. *Mitigation:* bounded, observable, matches 087.001 precedent;
  monitoring + rollback below.
* **R4 (orphan markers):** deleted files could leave stale
  `powerbi_file_index_state` rows if the sweep does not clean them. *Mitigation:*
  fold marker deletion into the powerbi delete paths (Unit D) and verify the
  powerbi sweep (Unit C) also drops the marker for swept paths.
* **Out of scope (freeze-scope):** pbip and backlog deletion sweeps share the same
  alias-stale pattern but are NOT in the two cards — recorded as a P2 backlog
  follow-up, not harvested here. Do not pull in the 090/091 low-priority tail.

## Plan Hardening Signals (REQUIRED)

* public API, schema, or contract change — **PRESENT**: new cozo relation
  `powerbi_file_index_state` (schema) + change to deletion semantics and the
  hash-skip durability contract.
* security/auth/permission/compliance-sensitive behavior — **ABSENT**: no
  auth/authz, secrets, trust boundary, or external-facing surface touched.
* migration/backfill/destructive-data/irreversible step — **PRESENT**: deletion
  sweeps *delete* records (data-removing); one-time self-populating marker
  reprocess (non-destructive migration).
* external integration / operator checkpoint / external dependency — **ABSENT**:
  purely in-process indexing; no external systems.
* high runtime / rollout / rollback risk — **PRESENT**: deletion-semantics change
  with data-loss blast radius + a durability contract change on the indexing hot
  path.

Requires plan hardening: **yes**

## Runtime Verification and Closure

* **Runtime surfaces changed:** the full-index ingestion pass
  (`engram index` / `index_workspace` / daemon indexing) — deletion counts and
  powerbi reprocess counts change. No CLI/MCP tool schema change.
* **Runtime verification before absorption:**
  * 087.005 — on a workspace with a directory-symlink alias, run a full index
    twice; assert the stale alias record is gone and the live record remains;
    assert an unreadable-subtree pass deletes nothing under the untouched tree.
  * 087.006 — force a mid-file write failure (fault injection in test), re-index,
    assert full record materialization; assert steady-state unchanged files are
    still skipped.
* **Operational closure artifacts:**
  * monitoring — watch `removed` (sweep deletions) and powerbi `ingested`
    vs `unchanged` counts in the ingestion summary log
    (`"Power BI indexing complete"`, `powerbi_indexer.rs:~1560`) across the first
    post-deploy index; a spike in `removed` beyond expected alias cleanup is a
    red flag.
  * rollback trigger — if post-deploy `removed` exceeds a small expected bound
    (named metric: sweep `removed` per source per pass; threshold: any deletion of
    a path whose real file is still collected) OR any "missing summaries" recall
    regression appears, revert the sweep-reconciliation and/or marker-gating
    commits (feature is branch-isolated on `100-powerbi-durability`; rollback =
    drop the offending unit's commit).
  * owner — Ship agent during 100-S execution/closure; validation window one full
    post-deploy re-index cycle.

---

## Plan Hardening

**Hardening required:** yes — 087.005 is a deletion-semantics change with
data-loss blast radius and 087.006 is a durability-contract change on the indexing
hot path (schema + hash-skip). Both were explicitly deferred as "unsafe to rush."

**Learnings / instructions consulted:** `.github/instructions/constitution.instructions.md`
(Principles I safety-first Rust, II test-first, III workspace isolation, VI
safety modes: **freeze-scope** to the two cards + **careful** for destructive
deletes), `.github/instructions/strict-safety.instructions.md` (ProposedAction /
ActionRisk vocabulary), `docs/compound/` scanned for prior deletion-sweep and
hash-skip traps (087.004-T canonical-visited-set introduction is the upstream
change this reconciles; 087.001-T one-time-reprocess-on-version-change precedent).

### Protected invariants (fail-closed)

1. **INV-1 (no live deletion):** a stored record whose real file is still
   collected this pass MUST NOT be deleted. Enforced by `not_collected` requiring
   `complete == true` and normalized-path equality with the collected set.
2. **INV-2 (partial collection ⇒ retain):** if traversal skipped any directory
   (read error), the pass is non-authoritative and only **physically-absent**
   records may be deleted.
3. **INV-3 (canonicalize failure ⇒ no-op delete):** a failure to canonicalize the
   workspace root yields an EMPTY deletion set (closes the current fail-open
   mass-delete in `compute_deleted_paths`).
4. **INV-4 (marker-last):** the `powerbi_file_index_state` marker is written only
   after every graph + content write for the file succeeds; the hash-skip reads
   only the marker map, never partial content rows.
5. **INV-5 (marker hygiene):** deleting a file's content rows MUST delete its
   marker in the same path, so no stale marker can cause a wrongful skip.
6. **INV-6 (scope freeze):** changes are confined to notebook + non-TMDL powerbi
   sweeps, the shared reconciler, and powerbi content-atomicity; pbip/backlog
   sweeps and the TMDL dirty-scope reconciliation are untouched.

### Risky actions (ProposedAction / ActionRisk)

| ID | ProposedAction | ActionRisk | Approval / control |
|---|---|---|---|
| PA-1 | Delete content records + graph nodes for paths flagged stale by the new reconciler (Units B, C) | **High** (destructive; data-loss if reconciler is wrong) | Gate behind INV-1..3; TDD fixtures 1–4 (Unit A) + integration fail-closed tests must pass before the delete path is enabled. Careful mode. |
| PA-2 | Add `powerbi_file_index_state` relation + register in schema bootstrap (Unit D) | **Medium** (schema/contract change; new persisted relation) | Idempotent `:create`; additive (no column drops); self-populating migration. |
| PA-3 | Re-source the hash-skip gate from the marker map (Unit D) | **Medium** (durability contract on hot path) | INV-4; steady-state skip test guards against needless reprocess; first-run reprocess bounded + monitored. |
| PA-4 | Retire per-collector `compute_deleted_paths` in favor of the shared reconciler | **Low-Medium** (behavior consolidation) | Preserve workspace-relative escape guard; characterize existing behavior in tests before replacing. |

`ActionResult` for PA-1..PA-4 remains **pending** — carried into Ship's build →
review → runtime-verification → operational-closure for 100-S.

### Regression-fixture list (must exist before merge)

* `RF-1` notebook: directory-symlink alias → stale record swept, live retained.
* `RF-2` powerbi: directory-symlink alias → stale record swept, live retained.
* `RF-3` unreadable-subtree pass (non-authoritative) → zero deletions under the
  untouched tree (INV-2).
* `RF-4` canonicalize-failure → empty deletion set (INV-3).
* `RF-5` genuine file deletion → record removed (physical-absence unchanged).
* `RF-6` powerbi partial-write (fault-injected) → next run reprocesses, all
  records materialized (INV-4).
* `RF-7` powerbi steady state → unchanged+marker skipped; first-run-no-marker
  reprocessed once.
* `RF-8` powerbi file deletion → content rows AND marker removed (INV-5).

### Reinforced verification / rollback

* Verification depth and blocked-path handling captured in "Runtime Verification
  and Closure" above; each unit is test-first with the RF-* fixtures as the
  failing-first harness.
* Rollback is per-unit commit revert on the isolated `100-powerbi-durability`
  branch; markers are additive so a revert leaves only harmless orphan
  `powerbi_file_index_state` rows (never read after revert).
* Unresolved operator decision blocking safe execution: **none** — the plan is
  self-contained; the only judgment call (batch `:put` vs marker) is decided
  (marker) with the alternative recorded as an advisory follow-up.

---

## Plan Review

**Gate decision: PASS** (hardening required and satisfied; no P0/P1; P2/P3
recorded as advisory follow-ups).

Reviewed in dark-factory autonomous mode across the always-on personas
(Constitution, Rust, Scope Boundary, Learnings) plus Architecture Strategist.
Security Lens and Agent-Native Parity were evaluated and **not triggered**: no
auth/authz/secrets/trust-boundary surface and no MCP tool-schema/parity change;
a data-integrity lens was still applied given the deletion/data-loss blast radius.

**Hardening check:** the plan shows hardening signals (schema/contract change,
data-removing deletes + migration, high runtime/rollback risk) AND includes a
complete `## Plan Hardening` section with protected invariants, ProposedAction /
ActionRisk classification, and a regression-fixture list ⇒ passes the
hardening-present gate. Strict-safety classification present ⇒ no FAIL on that
axis.

### Findings

**P0:** none.
**P1:** none — the two data-loss/durability risks are closed by INV-1..5 with
failing-first regression fixtures RF-1..RF-8.

**P2 (recorded as backlog follow-up, non-blocking):**
* `P2-1` — pbip and backlog deletion sweeps share the same alias-stale
  vulnerability (`pbip_indexer.rs:112/857`, `backlog_indexer.rs:49/402`) but are
  out of the two-card freeze scope. Record a stash follow-up to extend the shared
  reconciler to them in a later cycle; **do not** harvest into 100-S.
* `P2-2` — marker hygiene coupling: Unit C's powerbi sweep must also delete the
  `powerbi_file_index_state` marker for swept paths (INV-5). Verify during build
  that Unit C and Unit D agree on marker cleanup so no orphan marker survives a
  swept deletion.

**P3 (advisory):**
* `P3-1` — optional batched `:put content_record` (per-file) to narrow the
  partial-write window further (precedent `upsert_backlog_content_records`);
  strictly an optimization on top of the marker gate.
* `P3-2` — consider a `schema_meta` version key for the TMDL/powerbi index format
  (like `PYTHON_CANONICAL_EXTRACTION_VERSION_KEY`) so future format changes can
  force a reprocess, aligning with 087.001-T; not needed for this fix.

### Runtime verification / closure readiness

Present and specific (deletion/reprocess counters, named rollback trigger, owner,
validation window). No verification or closure gaps that block harvest.

**Cycle count:** 1 review cycle, no P0/P1 to resolve → within the 3-cycle limit.
Proceed to harvest/assembly.
