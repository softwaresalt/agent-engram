---
title: "Ship 092-S — same-file duplicate-name fail-closed direct-edge target (100-F / FF7DE872) merge closure"
date: "2026-07-28"
type: "ship-closure-memory"
feature: "100-F"
shipment: "092-S"
pr: 291
merge_commit: "8a6c6e32507434ff80e7453b92ecf27d21992bc4"
status: "shipped"
---

# Ship 092-S — merge closure memory

## Outcome

Feature 100-F shipped via **PR #291** (merge commit
`8a6c6e32507434ff80e7453b92ecf27d21992bc4`, merge-commit strategy per P-009 —
2 parents `a3b26705` ∪ `5d2ed0a8`). Fixes the `find_function_id` first-match
wrong-target direct edge by failing closed on `>1` same-file same-name candidate
(Decision 014-D, Option A — fail-closed, language-agnostic).

## Git base decision

`main` was protected/unpushable, so the feature branch
`100-same-file-shadowing-fail-closed` was based on the local Stage planning
commit `1f640e02` (planning artifacts appeared in the PR base — expected).
`start.ps1`'s unrelated uncommitted modification was never touched or committed.

## Per-task outcomes (TDD honored)

- **100.001-T (U1 RED)** `097c77e7` — failing harness; the Rust `#[cfg]`-gated
  duplicate-`platform` fixture minted the wrong `describe -> platform` edge (RED).
- **100.002-T (U2 GREEN)** `45165e6c` — additive `UniqueFunctionId` +
  `find_unique_function_id`, guarded both direct-edge minting sites,
  `same_file_ambiguous_dropped` counter. U1 -> GREEN.
- **100.003-T (U3)** `bf0694da` + remediation `5d2ed0a8` — 4 acceptance/regression
  tests + architecture.md subsection + deliberation/plan execution-correction
  notes.

## Key finding (hard-won)

The plan's premise was wrong: **Python was already fail-closed** at ship HEAD
(096-F `is_contested` over `module_binding_counts` is Python-scoped via
`increment_python_binding`), so the live wrong edge was **Rust-only** through
`#[cfg(unix)]/#[cfg(windows)]`-gated duplicate defs (tree-sitter extracts both
branches). The plan's inline-`mod` repro is unreachable — the Rust extractor has
no `mod_item` descent. Full detail in the compound bug entry
`same-file-duplicate-name-fail-closed-resolution-defect-was-rust-cfg-gated-2026-07-28`.

## Gates + review + runtime

- fmt PASS; clippy `-D warnings -D clippy::pedantic` PASS; `cargo dev-test` 461 +
  affected integration (incl. 18 recall — no loss) PASS; `cargo audit` = 10
  pre-existing transitive advisories only (Cargo.lock byte-identical to base).
- Copilot merge gate FULLY GREEN over 2 review cycles (7 threads resolved);
  latest review `commit_id == HEAD`, Copilot removed from requested_reviewers,
  0 unresolved threads, `mergeStateStatus == CLEAN`. Re-verified immediately
  before merge.
- Runtime: same-file `describe -> plat` fail-closed; same-file
  `caller_unique -> helper` direct edge (recall). Cross-file singleton
  daemon-CLI anomaly noted as pre-existing (stash `5765BAAB`), un-regressed by
  this change (in-process `index_workspace` tests resolve it).

## Closure actions

- Shipment 092-S -> **shipped**; archived scope: 100.001-T, 100.002-T,
  100.003-T, 100-F, 092-S. Merge SHA recorded on the shipment.
- Index resynced post-archival (770 artifacts).
- 4 follow-up items stashed: `8DD29746` (versioned backfill, task/med),
  `B94772CB` (Python last-wins, feature/low), `F97D51DF` (cargo audit, task/low),
  `5765BAAB` (daemon index routing/hang, bug/med). Item #5 (planning-wording
  reconciliation) assessed moot — corrections already on `main`.
- Compound learning + closure-refresh record written.

## Process learnings for next ship

- When the target repo enforces branch protection, closure docs cannot be pushed
  to `main` directly — land them via a dedicated `chore/close-XXXs` / `docs/...`
  PR (established convention), and keep the implementation PR scoped to
  implementation only.
- The Copilot merge gate must be re-checked after **every** push and once more
  immediately before merge; resolving all review conversations is what flips
  `mergeStateStatus` BLOCKED -> CLEAN (the repo enforces conversation resolution).
- A TDD RED test is the authority over the plan's stated repro: ours corrected
  two false premises (Python already closed; inline-`mod` unreachable) before a
  line of fix was written.
