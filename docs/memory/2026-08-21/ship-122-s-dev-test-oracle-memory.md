---
type: session-memory
date: 2026-08-21
agent: ship
shipment: 122-S
feature: 126-F
batch: dark-factory-20260820-870b1aff-568b257c-c2413934-de460a88
order: 3
status: shipped
---

## Session: Ship 122-S — canonical dev-test coverage oracle (C2413934)

### Worktree / branch

* Worktree: `C:\Source\GitHub\engram\.worktrees\ship-122-s-dev-test-oracle-20260821`
* Branch: `feat/122-s-dev-test-coverage-oracle` from clean origin/main `430225c1`
* Root `C:\Source\GitHub\engram` is dirty — never touched.

### Fail-closed preflight (PASS)

* No shipment has status `active`. 120-S, 121-S archived+shipped, merge-backed in
  origin/main (PR #349, #353). 122-S queued, unique lowest order 3 in batch;
  operator_predecessors == [120-S,121-S]. 123-S queued, order 4 — untouched.

### Tools

* backlogit CLI 1.10.0, engram CLI, gh 2.81.0, cargo/rustc 1.97.0 (pinned).
* backlogit index synced against the worktree `.backlogit`. Engram CLI not used
  (targeted direct reads sufficed); MCP never used per instruction.

### Work completed (U1–U8)

* U1/U2 RED harnesses (commit `d5824837`): `tests/unit/dev_test_coverage_oracle_test.rs`
  (4 scenarios), `tests/unit/dev_test_hcl_scope_guard_test.rs` (2 scenarios);
  registered two `[[test]]` targets in Cargo.toml. RED observed.
* U3/U4/U5/U6 GREEN (commit `4f3c2649`): `.cargo/test-coverage-manifest.toml`,
  `scripts/test-coverage-oracle.{ps1,sh}` (report/select/completeness/run modes),
  `.cargo/config.toml` bounded-concurrency env tunables, `.gitattributes`
  (`*.sh eol=lf`).
* U7/U8 docs (commit `2d00a94f`): workflows.md section; closure record
  `docs/closure/2026-08-21-c2413934-runtime-verification.md`.
* Review fixes (multiple cycles): fail-closed on indeterminate diff (unresolved
  base ref or missing merge base); feature-aware run mode (passes each target's
  `required-features`); include untracked files; portable batched run (no
  `wait -n`); strict HCL byte-identity guard; `--lib` coverage.
* **dev-test design (final):** `cargo dev-test = "test --all-targets"` — a
  native, zero-setup, cross-platform cargo alias that runs every target under
  default features including `--lib`. An earlier attempt made `dev-test` an
  external subcommand (`cargo-devtest` / `dev-test.ps1`) delegating to the oracle
  runner; Copilot review correctly flagged that this broke the pervasive
  `cargo dev-test` contract (constitution, workspace-profile, skills) and could
  not resolve on Windows. Those shims were removed. The coverage oracle is now
  the measurable audit (`--mode report`, `omitted == 0`), drift check
  (`--mode completeness`), and an OPTIONAL change-scoped bounded fast runner
  (`--mode run`), not the alias mechanism.

### Design decisions

* Manifest maps source surfaces → test-target NAME globs. Non-HCL tiers
  (`contract_*`,`integration_*`,`unit_*`,`cold_*`,`helpers_*`) partition all 208
  non-HCL targets; the 6 HCL targets map to the `src/services/parsing/hcl.rs`
  leaf. Production `src/` surfaces map comprehensively (plan-review A1 broad
  mapping); narrow surfaces (tests self-cover, docs/scripts ignore) differentiate.

### Evidence (U8)

* Representative non-HCL diff `src/db/workspace.rs`: omitted BEFORE (legacy 6-HCL
  selection) = 208; AFTER (change-scoped) = 0.
* Bounded run: cap 8, dry-run planned peak 8 for the 208-target diff; real run of
  a 2-target diff observed peak 2 (<= cap), 0 failed.
* HCL `[[test]]` blocks byte-identical (guard green). full-test/ci unchanged.
* Completeness: 214 targets, 13 src modules, 0 unmapped.

### Gates

* `cargo fmt --all -- --check` PASS. `cargo clippy --no-default-features
  --features cozo-backend,embeddings --all-targets -- -D warnings -D
  clippy::pedantic` PASS. Harnesses 6/6 green (Windows ps1). `.sh` parity
  validated in WSL bash across all modes.
* Note: CI uses `--no-default-features --features cozo-backend,embeddings`, NOT
  `--all-features`. A pre-existing `otlp-export` compile error in
  `src/server/observability.rs` (opentelemetry-otlp 0.26) is out of scope and
  not on the CI path.

### Next steps

Closure complete — see below.

## Closure (2026-08-22)

* **Merged**: PR #355 merged by merge commit `5d5bc0bd020c0af340d28d01ef60272e2410a3ed`
  at 2026-08-22T08:38:17Z; confirmed ancestor of origin/main (now
  `5d5bc0bd`). Merge strategy: merge commit (P-009 compliant; squash/rebase
  disabled).
* **Implementation commits**: `d5824837` (RED), `4f3c2649` (GREEN),
  `2d00a94f` (docs), review fixes `484aa5f2`/`7aadbb42`/`bbcadf78`/`1f0caebd`/
  `1926b67e`/`15deaefb`, memory `38eb9044`, backlog `ee0b004e`.
* **CI**: build + start-launcher-windows green on final HEAD.
* **Copilot review**: 5 review cycles. Cycles 1-3 fixed inline (feature-aware
  runs, untracked files, portable batching, strict guard, fail-closed diff,
  `--lib`, the native-alias pivot). Cycles 4-5 (past the 3-cycle limit) deferred
  to follow-up feature **128-F** (oracle audit-precision hardening: report
  default selected-set, extra build surfaces, order-independent TOML parsing,
  rollback-trigger wording). All threads resolved; 0 unresolved at merge.
* **Shipment**: 122-S status=shipped, commit=`5d5bc0bd`. All 9 manifest items
  (126-F + 126.001-008-T) done/archived.
* **123-S**: unchanged — status=queued, order=4.
* **Compound learning**: `docs/compound/2026-08-22-cargo-dev-test-alias-must-stay-native.md`.

