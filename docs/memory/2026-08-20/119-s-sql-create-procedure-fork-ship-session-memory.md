# Session Memory: Ship 119-S — SQL CREATE PROCEDURE via Approved Immutable Grammar Fork

**Date**: 2026-08-20
**Agent**: Ship
**Shipment**: 119-S (feature 123-F, tasks 123.001-T-123.006-T, 11 subtasks)
**Branch**: `feat/123-f-sql-create-procedure-grammar-fork`
**Worktree**: `C:\Source\GitHub\engram\.worktrees\ship-119-s-sql-create-procedure-fork-20260820`
**Planning HEAD**: `403c3b14775bd2290ee235458939288d2f78b1cf`
**PR**: <https://github.com/softwaresalt/agent-engram/pull/346>

## Outcome

All build/task work for 119-S is complete, reviewed, and pushed. PR #346 is
open and ready for merge **except** for one pre-existing, out-of-scope CI
failure (see Blocker below). Explicit operator merge approval is the
remaining gate; this session halted rather than expand scope to fix an
unrelated repo-wide issue.

## Work Completed

* 123.001-T (RED): added/rewrote `tests/unit/parsing_test.rs` contracts -
  grammar ABI probe, exactly-one-Function+Defines-edge assertion, malformed
  SQL graceful-degradation test. Observed RED against crates.io `0.3.11`.
* 123.002-T: pinned `tree-sitter-sequel` to the approved fork
  (`git = "https://github.com/softwaresalt/tree-sitter-sql"`,
  `rev = "50837582b5ba15c7acff3be7bf585a1082d90528"`), regenerated
  `Cargo.lock`. Discovered during GREEN verification that the fork's
  `procedure_body` grammar rejects a `;` immediately before `END` (single
  unterminated statement) - corrected the test fixture accordingly.
* 123.003-T: wrote
  `docs/decisions/2026-08-20-tree-sitter-sequel-compatibility-fork-provenance.md`
  (EG-1 evidence packet, 6 artifact hashes, license, ABI, rollback,
  90-day staleness review due 2026-11-18).
* 123.004-T: removed the stale 0.3.11 ERROR-node limitation doc in
  `src/services/parsing/sql.rs`; no new extraction arm added (pre-existing
  `"create_function" | "create_procedure"` match arm is the implementation).
* 123.005-T: ran all 4 ordered quality gates locally - `cargo fmt --all --
  check` (1 fix applied), `cargo clippy --all-targets -D warnings -D
  clippy::pedantic` (0 warnings), `cargo dev-test` (653 passed), `cargo
  audit` (0 new advisories).
* 123.006-T: wrote
  `docs/closure/2026-08-20-sql-create-procedure-compatibility-fork-closure.md`
  - runtime verification (later upgraded to a genuine `--release` binary
  after Copilot review flagged the debug-build claim), monitoring plan,
  pre-deploy audit, 72h observation window, rollback triggers.
* All 11 subtasks under the 6 tasks were discovered (via Copilot review,
  not by me initially) still `queued` after I completed their parent tasks.
  Verified each maps 1:1 to already-delivered work and moved all to
  `done`/archived with commit tracking, avoiding the
  `backlogit shipment ship` child-expansion hazard documented in
  `docs/compound/workflow-issues/backlogit-ship-blocked-child-expansion-2026-04-26.md`.

## Review Cycle

* Direct code-review agent pass: PASS, no P0/P1.
* 3 rounds of GitHub Copilot review (auto-triggered on push,
  `copilot_code_review.review_on_push: true` ruleset). All 9 first-round +
  1 second-round findings were legitimate and fixed:
  * release-equivalent binary claim corrected (rebuilt with `--release`,
    re-verified identical symbol results)
  * sql.rs doc comment over-generalized the T-SQL bare-fixture separator
    behavior - scoped correctly
  * stale Stage checkpoint (`checkpoint-20260820-080810.json`) resolved via
    `backlogit checkpoint resolve`
  * 5 stale `.backlogit/archive/*.md` cross-references broken by archival
    moves - corrected
  * closure doc's CI platform-matrix claim corrected (Ubuntu-only full CI;
    Windows PR CI is launcher-smoke-only; macOS full build is
    release-tag-only) - documented as an accepted residual gap
  * 11 orphaned subtasks (above)
* All threads resolved (0 unresolved) at final HEAD `19d0d478`. Copilot
  review landed at each pushed HEAD (current-HEAD gate satisfied each time).

## Blocker: Pre-Existing, Out-of-Scope CI Failure

`ci.yml`'s `build` job fails deterministically (reproduced 3x across 3
separate pushes) with 78 `clippy::unused_async_trait_impl` errors in
`src/db/cozo_queries.rs`, `src/server/state.rs`, and
`src/shim/transport.rs` - **none of which this PR's diff touches**
(confirmed via `git diff --stat` against planning HEAD: only `Cargo.toml`,
`Cargo.lock`, `src/services/parsing/sql.rs`, `tests/unit/parsing_test.rs`,
new `docs/`, and `.backlogit/` state changed).

Root cause: `ci.yml` uses `dtolnay/rust-toolchain@stable` (floating), which
resolved to a newer Rust/clippy (1.98.0, introducing this lint) than the
locally cached/pinned toolchain (1.97.0) used for local verification. This
is a repo-wide toolchain-drift issue that would fail identically on any PR
right now - it is not caused by, and cannot be responsibly fixed within,
123-F/119-S's frozen scope ("no unrelated SQL grammar work, fork
administration, dependency version invention, or broad fixes").

`start-launcher-windows` failed once (flaky wall-clock timing test,
`launcher_fails_open_to_copilot_within_one_prewarm_budget`) and passed
cleanly on retry - confirmed transient, not a regression.

`main` has no branch protection; the repo ruleset requires 0 approving
reviews, resolved review threads (satisfied), `merge`-only merge method (no
squash/rebase - matches constitution Principle XI), and does not list a
required status check. `mergeable: MERGEABLE`, `mergeStateStatus: UNSTABLE`
(reflects the failing `build` check, not a hard block).

**Recommendation**: triage and fix the clippy/toolchain-drift issue as a
separate, scoped chore (pin `rust-toolchain.toml`/CI toolchain or fix the 78
lint sites) independent of 119-S. This PR's own diff is fully clippy-clean
under the previously-passing toolchain and does not need further changes
once that separate issue is resolved.

## Next Steps

1. Operator decides: (a) approve merge now given `build` is not a required
   check and the failure is confirmed unrelated, or (b) wait for the
   toolchain-drift chore to land first, or (c) explicitly waive.
2. On explicit merge approval: use merge commit only (repo ruleset already
   enforces `merge`-only). Then run the Merge Confirmation Gate, shipment
   reconciliation (`backlogit shipment ship 119-S`), 72h observation per
   the closure doc, compound-refresh, and compact-context.
3. Do not touch `src/db/cozo_queries.rs`, `src/server/state.rs`, or
   `src/shim/transport.rs` as part of 119-S - that work belongs to a
   separate CI/toolchain chore.

## Files Changed (final diff vs. planning HEAD)

`Cargo.toml`, `Cargo.lock`, `src/services/parsing/sql.rs`,
`tests/unit/parsing_test.rs`,
`docs/decisions/2026-08-20-tree-sitter-sequel-compatibility-fork-provenance.md`
(new), `docs/closure/2026-08-20-sql-create-procedure-compatibility-fork-closure.md`
(new), plus `.backlogit/` state transitions (119-S, 123-F, 6 tasks, 11
subtasks archived) and a resolved Stage checkpoint.
