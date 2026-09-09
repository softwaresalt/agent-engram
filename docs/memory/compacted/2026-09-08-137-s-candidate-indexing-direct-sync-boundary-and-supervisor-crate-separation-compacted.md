---
title: "Compacted memory — 137-S: Candidate indexing, direct-sync boundary and supervisor crate separation"
description: "Dense consolidated summary of the full 137-S session lifecycle (dark-factory halt → preflight → implementation → merge → post-merge closure), replacing 9 verbose checkpoints"
---

## Release unit

Shipment `137-S`, covering feature `142-F` (roster: 59 units). Branch
`feat/137-s-candidate-indexing-direct-sync-boundary-and-supervisor-crate-separation`.
PR [#388](https://github.com/softwaresalt/agent-engram/pull/388). Merge
commit `ef0135bf05aba5d7329a7306eb78bb9b18e02f69` (merge commit strategy,
P-009). Manifest: `142.015-T`, `142.016-T`, `142.020-T`, `142.021-T`,
`142.022-T`, `142.027-T` — all `done`, all archived. Shipment record
archived (`archived_status: done`) via manual safe-close.

## Timeline and key decisions

0. **Harness generation (P-002/P-004)**: replaced inert placeholder harnesses
   for F10, F11, F13, F14, F15 with real RED tests; added the F12
   supervisor-crate RED scaffold (`crates/engram-indexer/src/lib.rs`, its
   crate-local integration test, and the `engram` path dependency) plus the
   minimal compile-only production seams (`index_sealed_target` stub,
   `INSTALLED_BINARIES` stub) needed for RED compilation. All five
   implementable harnesses failed RED as expected before implementation
   began; F13/F15 passed immediately (structural-only contracts already
   true at scaffold time).
1. **Dark-factory pre-claim halt**: an earlier dark-factory activation
   scoped to `137-S` halted before claiming — the main worktree was not
   clean (unrelated `136-S` closure-session artifacts present). No branch,
   commit, PR, or backlog transition occurred at that halt; freeze-scope was
   preserved and the operator was asked to reconcile the worktree first.
2. **Implementation (6/6 manifest tasks, all completed within budget)**:
   * `142.015-T` (F10) — `src/services/code_graph.rs` gained
     `index_sealed_target`, accepting only a sealed `IndexTarget` for both
     `LegacyDirect` and `Candidate` kinds; no raw-path indexing entry point
     remains. Required 1 review-fix cycle: restricted raw-path indexing
     wrappers to crate-private visibility, disabled deletion-authoritative
     sibling eviction for single-file `LegacyDirect` scope, added
     active-generation-immutability coverage. A raw-path visibility hack
     from the first pass was reverted by Ship after re-scoping against the
     F10/F11 plan boundary.
   * `142.016-T` (F11) — `src/cli/direct.rs` now resolves daemon mode via
     the existing `daemon::ipc_server::resolve_daemon_mode` (reused, not
     re-derived) before direct sync/index work, and refuses with the
     stable `DirectSyncRefused`/F38 error in `ReadServer` mode;
     `src/cli/output.rs` gained a structured JSON-RPC error emitter
     preserving `error.name`. `Managed` mode unchanged.
   * `142.020-T` (F12) — `crates/engram-indexer` gained real `lib.rs`
     (`run_for_target`/`run_for_target_with_options`, delegating directly
     to `code_graph::index_sealed_target`, no reimplemented logic) and
     `main.rs` (minimal env-driven entrypoint sealing a `LegacyDirect`
     target). Added `tokio` (current-thread, minimal features) as the only
     new dependency.
   * `142.021-T` (F13) — workspace-boundary contract test only; zero
     production changes (the boundary was already true from F12/F12a).
   * `142.022-T` (F14) — `.github/workflows/release.yml` build matrix
     additively builds/stages/archives/uploads `engram-indexer` into a
     wholly separate `staging-indexer-*` path; existing agent archive
     steps untouched (46 insertions, 0 deletions).
   * `142.027-T` (F15) — `src/installer/mod.rs` gained a minimal, honest
     `INSTALLED_BINARIES` regression-guard constant (the module has no real
     binary-copy logic to guard beyond this); `scripts/install.{ps1,sh}`
     confirmed already agent-only, left untouched.
3. **Review**: 6 per-task `code-review` passes (5 first-pass READY, 1 after
   the fix cycle above). 1 final consolidated full-branch review:
   `READY_WITH_FOLLOWUPS` — single P2 finding (no cross-process write
   coordination yet between the new `engram-indexer` supervisor and
   existing direct-sync/daemon writers), correctly out of scope per P-021
   C1 (deferred to plan units F16-F18), captured to stash `AF5CE07E`.
4. **Gates before merge**: CI green (build + `start-launcher-windows`
   SUCCESS), P-018 copilot-review gate SATISFIED for the exact reviewed
   HEAD `653e973e201883790a9862e6ff54ea84efbb902d`, repo merge-strategy
   settings confirm merge-commit-only (P-009).
   `merge_approval_pre_authorized=false` — halted for explicit operator
   approval.
5. **Operator merge approval**: *"PR 388: Merge approved"* (scoped exactly
   to PR #388 at the stated reviewed HEAD; admin fallback not
   pre-authorized).
6. **Merge**: `gh pr merge 388 --merge` — merge commit
   `ef0135bf05aba5d7329a7306eb78bb9b18e02f69`. `MERGE_CONFIRMED` via
   `gh pr view` + `git merge-base --is-ancestor`.
7. **Post-merge closure (2026-09-08)**:
   * Pre-mode reconciliation: all 6 manifest items `pre-archived`, no
     orphans → `PROCEED`.
   * **`backlogit shipment ship` not attempted at all** — on the strength
     of the identical, already-twice-documented non-termination defect
     against the shared `142-F` covering feature (see
     `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`,
     now updated with a second-occurrence addendum for `137-S`). Manual
     safe-close applied directly: hand-authored `.backlogit/archive/137-S.md`,
     removed the queue file, `backlogit sync` clean. Verified `142-F`
     byte-for-byte unchanged (SHA-256 confirmed identical pre/post).
   * Post-mode reconciliation: all archive files present, no deletions
     (P-007 clean) → `PROCEED`.
   * Post-merge closure branch
     `post-merge/137-s-candidate-indexing-direct-sync-boundary-and-supervisor-crate-separation`
     created from fresh `main`.
   * Runtime verification: 15/15 targeted tests across all six manifest
     tasks GREEN, plus the new `engram-indexer` crate's own boundary test
     GREEN, plus full `cargo dev-test` 688/689 GREEN (build/fmt/clippy all
     clean). The one exception —
     `archive_verifier_runs_the_unpacked_native_binary` — is a
     long-documented, pre-existing, confirmed-unrelated Windows stdout-
     truncation flake (fourth+ occurrence; stashed as `7D47F30B`, ambiguous
     against four prior entries). `cli-daemon-status` live probe BLOCKED
     on a bounded 30s budget — consistent with the already-documented
     per-branch Cozo first-index cold-start cost (135-S precedent), not a
     regression. Verdict: `PASS WITH FOLLOW-UP`; releasability `READY WITH
     CONDITIONS`.
   * `docs/ARCHITECTURE.md` updated: the `crates/engram-indexer/` module
     table row (previously claiming an empty stub) now reflects the real
     F12 supervisor entry points, and a new paragraph documents F10's
     sealed-`IndexTarget` acceptance and F11's `ReadServer` refusal path.
   * `compound-refresh`: one entry updated (the non-terminating
     `shipment ship` doc, second-occurrence addendum); two entries kept
     unchanged after review (validator-manifest drift, done-status repair
     doc — neither applicable).
   * Source artifact cleanup: checked `source_stash_id` /
     `source_deliberation_id` on all 8 shipped-scope items (6 tasks + 142-F
     + 137-S) — **none present anywhere**; nothing retired.

## Stash follow-ups referenced or captured (all `requires_deliberation: true`, none blocking)

| ID | Priority | Summary |
|---|---|---|
| `AF5CE07E` | (Stage to reprioritize) | Pre-merge review follow-up: no cross-process write coordination yet between supervisor and daemon writers. Referenced only, not triaged/implemented. |
| `7D47F30B` | medium | **Post-merge, newly captured**: `archive_verifier_runs_the_unpacked_native_binary` Windows stdout-truncation flake, confirmed unrelated, ambiguous against 4 prior entries (`58B33C45`, `4EE241DC`, `3067BC32`, `0443D844`). |
| `DA0AF326` | low | (cited, not re-captured) Validator-manifest command drift — re-confirmed identical this session. |

## Outcome

137-S fully closed: shipment archived, all tasks archived, covering feature
untouched, PR #388 merged, closure evidence recorded in
`docs/closure/2026-09-08-137-s-{operational-closure,runtime-verification,compound-refresh}.md`.
Post-merge closure PR still required for the backlog-archival branch
(`post-merge/137-s-...`) — awaits its own explicit operator approval,
separate from PR #388's approval. `compact-context` (this file) invoked as
the mandatory P-020 step.

## Superseded/compacted originals (see `docs/archive/memory/2026-09-08/`)

* `red-harnesses-f10-f15-memory.md`
* `137-s-dark-factory-preclaim-halt-memory.md`
* `137-s-pre-pr-session-memory.md`
* `142-016-t-direct-sync-read-server-memory.md`
* `142.015-t-sealed-index-target-memory.md`
* `142.020-t-supervisor-crate-foundation-memory.md`
* `142.021-t-workspace-boundary-contract-memory.md`
* `142.022-t-supervisor-release-artifact-memory.md`
* `142.027-t-installer-exclusion-memory.md`
