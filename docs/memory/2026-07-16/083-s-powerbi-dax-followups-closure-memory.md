---
type: session-memory
date: 2026-07-16
agent: ship
session: "2c95481b - 083-S PowerBI/DAX followups closure"
topic: "083-S PowerBI/DAX PR#246 review-deferred followups merged; post-merge closure"
---

# Session memory - 083-S PowerBI/DAX followups closure

## Outcome

Shipment **083-S** (feature `087-F`, PowerBI/DAX PR#246 review-deferred followups) merged to `main`
as merge commit **e0f8e440** via **PR #257**. The shipment hardens DAX linting and PowerBI impact
docs and, most substantially, centralizes source traversal into a symlink-cycle-safe helper shared by
the PowerBI, PBIP, notebook, and backlog indexers, with matching symlink-safe deletion sweeps.
Closure artifacts and backlog archival were produced on branch `chore/083-closure` in the `ship-083`
worktree off the merged `main` (`e0f8e44`).

## Tasks completed

* `087.001-T` - DAX lint results carry an index-version fingerprint so stale lint summaries are
  detectable.
* `087.002-T` - DAX `--` line comments are tokenized so references and division inside comments no
  longer emit findings.
* `087.003-T` - `impact_analysis` PowerBI behavior documented.
* `087.004-T` - `collect_recursive` made symlink-cycle-safe: directory symlinks are followed only
  when their canonical target stays under the workspace root, and canonical directory visits are
  tracked to prevent cycles and alias duplication.

## Files modified in the shipped feature

Feature and review-cycle work touched these primary surfaces (all merged via PR #257):

* `src/services/source_traversal.rs` - centralized traversal + shared
  `is_regular_file_in_workspace` helper (`symlink_metadata` no-follow, `is_file`, canonicalize
  within root); `collect_recursive` now warns on unreadable-directory `read_dir` failure.
* `src/services/powerbi_indexer.rs` - `workspace_relative_path` validator gained `has_root()` guard;
  deletion sweep uses the shared regular-file-in-workspace helper.
* `src/services/notebook_indexer.rs` - same `has_root()` guard and symlink-safe sweep.
* `src/services/backlog_indexer.rs` - `compute_deleted_paths` rewritten as `filter_map` guarded by a
  relative-only `workspace_relative_path` validator, threaded with `workspace_root`; backlog nodes
  store relative `file_path`, so relative-only is the correct contract.
* `tests/unit/backlog_indexer_test.rs` - sweep tests moved to relative paths; new `S-BI-10`
  escape-path skip test.
* `tests/integration/powerbi_search_ingestion_test.rs`, `tests/integration/dax_lint_tier2_test.rs`,
  `tests/unit/dax_lint_test.rs` - deconflicted duplicate test-ID doc comments.

Closure work touched only documentation and backlog state:

* `docs/closure/2026-07-16-083-s-powerbi-dax-followups-closure.md`
* `docs/memory/2026-07-16/083-s-powerbi-dax-followups-closure-memory.md`
* `.backlogit/archive/083-S.md`, `.backlogit/archive/087-F.md`,
  `.backlogit/archive/087.001-T.md`..`.backlogit/archive/087.004-T.md`
* `.backlogit/queue/087.005-T.md`, `.backlogit/queue/087.006-T.md` (kept queued)

## Key decisions

* **Collector-versus-sweep symmetry is the core invariant.** 087.004-T made collectors symlink-safe;
  Copilot cycle 1 correctly flagged that the deletion sweeps still followed symlinks. The fix mirrors
  collector semantics exactly in every sweep, so a symlinked-out file is neither indexed nor retained.
* **Sweeps fail closed and never probe outside the workspace.** Absolute, root-relative (`\foo`
  under Windows semantics), `..`, and drive-prefix paths are rejected before any filesystem probe.
* **Two architectural findings deferred, not rushed.** Cycle-3 F1 (stale alias-backed records need
  collected-set reconciliation) and F3 (per-record TMDL write exposes the final hash before
  completion) are genuine but change deletion and persistence semantics; landing either safely needs
  broader verification than a late review cycle allows and was judged unsafe to rush AFK. Filed as
  `087.005-T` and `087.006-T`, both queued.
* **F3 confirms the pre-PR P2 deferral.** The TMDL atomicity concern is the same TOCTOU the pre-PR
  adversarial review deferred as P2; Copilot independently surfaced it, so the deferral is validated
  rather than scope creep.

## Adversarial and Copilot cycle summary

One pre-PR cross-model adversarial review (rust `gpt-5.6-sol`, security `gemini-3.1-pro`, scope
`gpt-5.6-terra`, follow-up `gemini-3.5-flash`) fixed three P1s and deferred one P2. Four Copilot
cycles followed:

* Cycle 1 (4 findings): collector-versus-sweep symlink asymmetry across all four sweeps -> fixed in
  `5154f39` via the shared `is_regular_file_in_workspace` helper + backlog `workspace_root` threading.
* Cycle 2 (6 findings): `has_root()` gap in PowerBI/notebook validators; backlog relative-only
  rewrite + `S-BI-10` test; three duplicate test-ID deconflicts -> fixed in `f2fc3f0`.
* Cycle 3 (3 substantive): F2 lost unreadable-directory warning -> fixed in `71d5b5c`; F1 -> deferred
  `087.005-T`; F3 -> deferred `087.006-T`. Copilot re-flagged F2/F3 dups -> resolved with fix/defer
  rationale.
* Cycle 4: clean pass (18/18 changed files, no new comments) at HEAD `4325273`.

Merge-gate evidence before PR #257 merged: Copilot review bound to HEAD `4325273`, Copilot
de-requested, 0 unresolved review threads, `mergeable_state == clean`, and the `build` check green.

## Verification state

* Formatting, clippy with `-D warnings -D clippy::pedantic`, and the affected test binaries
  (`unit_backlog_indexer`, `integration_powerbi_search_ingestion`,
  `integration_pbip_search_ingestion`, notebook and DAX lib tests) were green.
* Symlink sweep assertions actually executed on the local Windows environment (symlink-creation
  privilege held), exercising the collector-versus-sweep symmetry.
* CI `build` was green at HEAD `4325273`.

## Deferred follow-ups

* `087.005-T` - reconcile deletion sweeps against the collected-path set so a directory-symlink alias
  cannot leave a stale, alias-backed record; queued.
* `087.006-T` - persist PowerBI TMDL content records atomically (or gate hash-skip on a completion
  marker) so a partial write cannot permanently skip a missing summary; queued.

These do not block the shipped feature. They are stale-record and durability improvements, not
known false-edge or data-exposure paths.

## Known external flakes and open operator items

* The HF embeddings backfill CI flake is environment-not-code: model download/cache timing can trip
  the fixed deadline; unrelated to this shipment.
* PR #248 (081-S) remains open for the operator. Its head branch is the operator's active
  `feat/088-rec1-call-resolution`; Option C supersedes its approach, but closing an operator's
  active-branch PR while AFK is not sound. Flag for operator.

## Next steps

1. Commit closure doc + memory + backlog archival, push `chore/083-closure`, open the closure PR.
2. Request Copilot review, drive the four-point merge gate, merge (docs-only, usually clean).
3. Queue order after closure: **085-S** (090-F CLI<->MCP parity) -> **086-S** (092-F writer
   atomicity), one active shipment at a time per P-001.
