---
title: "109.031-T Windows coordinator runtime verification"
date: 2026-08-03
shipment: 104-S
feature: 109-F
task: 109.031-T
branch: feat/109-single-authority-coordinator
commit: 1edcc61f9d90296e16c7132c8fdd6ef3adc8f8b9
surface: background-job
mode: manual
verdict: pass
---

# 109.031-T Windows Coordinator Runtime Verification

## Verdict

**PASS.** The narrow test-isolation repair restored the exact Windows
CI-equivalent all-target gate. The current candidate then passed the required
15-minute named-pipe observation, PID-specific restart/reconciliation, and
full-release-unit rollback restart against disposable state.

PR readiness is still blocked outside this runtime verdict: the repository
clippy gate reports production-source lint failures that cannot be changed
under `109.031-T`'s zero-production-file constraint.

## Environment Prechecks

- Windows branch and production HEAD:
  `feat/109-single-authority-coordinator` at
  `1edcc61f9d90296e16c7132c8fdd6ef3adc8f8b9`.
- Worktree was clean before validation.
- `104-S` was active, `109.031-T` was blocked, and completed dependency
  `109.030-T` was confirmed.
- Backlogit MCP was unavailable in the session tool surface; the
  registry-declared CLI fallback was used and the index synchronized.
- Engram daemon status was reachable and bound to
  `C:\Source\GitHub\engram`.
- Test state remained under the repository. Targeted and aggregate roots were
  `tmp\109031-cycle2-red` and `tmp\109031-cycle2-aggregate`.
- `TEMP`, `TMP`, and `GIT_CEILING_DIRECTORIES` pointed to those roots.
  `ENGRAM_DATA_DIR` was removed only from the aggregate Cargo child
  environment; the targeted regression deliberately retained the ambient
  value to prove fixture isolation.

## Scenarios and Evidence

### Scenario 1 — TDD isolation repair

The exact global state was inherited
`ENGRAM_DATA_DIR=C:\Source\GitHub\engram\.engram`. Production
`resolve_data_dir` therefore bound every in-process retrieval-eval fixture to
the same `cozo\main\engram.db`, despite each test owning a different temp
workspace. That shared branch database caused both cross-row corpus
contamination and rapid Windows Cozo reopen locking.

A regression assertion was added first and observed RED:

```text
left:  C:\Source\GitHub\engram\.engram
right: ...\tmp\109031-cycle2-red\.tmp...\.engram
```

The smallest test-only repair added `bind_isolated_workspace`, following the
repository's existing explicit `WorkspaceSnapshot` fixture pattern. It binds
`data_dir` to `{temp-workspace}\.engram` atomically with test config, without
mutating process environment or production code.

Targeted outcomes with the ambient variable still present:

- regression: `1 passed`;
- `integration_retrieval_eval_thresholds`: `7 passed`;
- `contract_retrieval_eval_status`: `6 passed`;
- feature-matched `cargo check --all-targets`: PASS;
- `cargo fmt --all -- --check`: PASS.

### Scenario 2 — Exact Windows all-target gate

The exact command ran once after the targeted root cause was green:

```powershell
cargo test --no-default-features --features cozo-backend,embeddings --all-targets
```

Result: **PASS**, exit `0`. TEMP/TMP and the Git ceiling were repository
contained. No assertion, test, timeout, or scheduling policy was weakened.

### Scenario 3 — Named-pipe observation and reconciliation

The current candidate ran as PID `26388` in
`tmp\109031-cycle2-runtime`. From
`2026-08-04T02:01:45.8318676Z` through
`2026-08-04T02:16:59.8654522Z`, 16 of 16 active probes reported:

- named-pipe reachability, PID liveness, and workspace identity green;
- DB path contained under the disposable workspace;
- one indexed source file, no stuck scan, and zero duplicate-daemon events.

A fixture edit was consumed by the watcher before an explicit sync, whose
unchanged-file result completed in 31 ms. PID `26388` was then stopped
explicitly. The same current binary restarted as PID `41812`, logged stale-PID
recovery for `26388`, preserved the workspace identity and graph
(`1` file, `2` functions, `3` edges), and completed a clean no-op sync.

### Scenario 4 — Full-unit rollback restart

A detached clean worktree at the complete rollback boundary
`df2803e1834728681288a2669c314dffea004307` produced a separate baseline
binary. After stopping current PID `41812`, that baseline started as PID
`35352` against the same disposable state. Named-pipe health, workspace
identity, contained DB, graph contents, and a clean sync all passed. PID
`35352` was stopped explicitly and the clean baseline worktree was removed.
No partial source, schema, or data rollback occurred.

## Ownership and Driver Invariants

The clean aggregate run is the deterministic race proof. Its matrices cover
continuous `AdmissionGuard -> OwnerPermit -> transferred OwnerPermit`
ownership, all `OwnerKind` rows, pre-acquisition cancellation, full-mask
transfer/recovery, same-binding `0b111`, distinct-binding zero carry, one
post-unlock notification, finite empty-waiter baton progress, stale-terminal
no-op, child-before-ack ordering, no successor-before-ack,
`max_active_db_drivers == 1`, and zero old work after ack.

The live Windows run is intentionally observational. It proved named-pipe
liveness, stable identity, watcher/sync progress, restart reconciliation, and
complete rollback compatibility; it does not replace the private deterministic
counters.

## Risky Action Record

- **ProposedAction:** add shared test-only workspace binding.
  **ActionRisk:** moderate. **Approval required:** yes, supplied by the
  operator. **ActionResult:** applied with zero `src/` changes.
- **ProposedAction:** bypass assertions, serialize the suite, or extend
  timeouts. **ActionRisk:** high. **ActionResult:** abandoned/prohibited.
- **ProposedAction:** run, abort, restart, and roll back only tracked disposable
  daemon PIDs. **ActionRisk:** high. **Approval required:** yes, supplied by the
  operator. **ActionResult:** applied to PIDs `26388`, `41812`, and `35352`.
- **ProposedAction:** remove the clean disposable baseline worktree.
  **ActionRisk:** destructive but contained. **Approval:** covered by the
  disposable rollback request. **ActionResult:** applied.

## Required Follow-up

The runtime gate has no remaining blocker. PR readiness remains blocked by one
production-only quality gate:

```text
cargo clippy --no-default-features --features cozo-backend,embeddings \
  --all-targets -- -D warnings -D clippy::pedantic
```

It reports nine lint findings in `src/daemon/ipc_server.rs` and
`src/tools/write.rs` (`similar_names`, `let_and_return`,
`unnecessary_semicolon`, `too_many_arguments`, `items_after_statements`, and
`single_match`). Fixing those findings requires forbidden production edits, so
`109.031-T` remains blocked under this final delegation.
