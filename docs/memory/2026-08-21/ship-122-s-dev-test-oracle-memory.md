---
type: session-memory
date: 2026-08-21
agent: ship
shipment: 122-S
feature: 126-F
batch: dark-factory-20260820-870b1aff-568b257c-c2413934-de460a88
order: 3
status: build-complete-pending-pr
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
  `cargo-devtest` shim, `.cargo/config.toml` alias `dev-test = "devtest"` + env
  tunables, `.gitattributes` (`*.sh eol=lf`).
* U7/U8 docs (commit `2d00a94f`): workflows.md section; closure record
  `docs/closure/2026-08-21-c2413934-runtime-verification.md`.
* Review fixes (commits `484aa5f2`, `7aadbb42`): dropped misleading
  `cargo-devtest.cmd` (cargo ignores `.cmd` subcommands), added
  `scripts/dev-test.ps1` Windows wrapper; oracle fails closed (exit 3) when
  `origin/main` base ref is unresolvable; `.ps1` no longer throws on native git
  stderr; docs aligned with per-platform reality.

### Design decisions

* Manifest maps source surfaces → test-target NAME globs. Non-HCL tiers
  (`contract_*`,`integration_*`,`unit_*`,`cold_*`,`helpers_*`) partition all 208
  non-HCL targets; the 6 HCL targets map to the `src/services/parsing/hcl.rs`
  leaf. Production `src/` surfaces map comprehensively (plan-review A1 broad
  mapping); narrow surfaces (tests self-cover, docs/scripts ignore) differentiate.
* Cargo aliases cannot shell out → `dev-test` uses an external subcommand.
  cargo resolves `cargo-devtest` (no ext) on Linux/macOS but NOT `.cmd` on
  Windows → Windows uses `pwsh scripts/dev-test.ps1`. Documented honestly.

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

1. Mark 126.007-T, 126.008-T done; track commits.
2. Push branch; open implementation PR.
3. Request/poll Copilot review at current HEAD; enforce 4-point merge gate.
4. Merge (merge commit only); confirm; runtime verify; operational closure.
5. shipment-reconcile (pre/post); ship 122-S; archive descendants; compound-refresh
   if guidance changed. Do NOT start 123-S.
