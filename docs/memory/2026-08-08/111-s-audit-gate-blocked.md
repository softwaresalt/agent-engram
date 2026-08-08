---
type: ship-checkpoint
timestamp: 2026-08-08T02:11:00-07:00
shipment: 111-S
branch: feat/111-s-index-coordinator-observability-reliability
status: resumed
blocker: none
---

# Shipment 111-S audit-gate checkpoint

## Completed implementation

- 117.001-T: direct Sync now atomically acknowledges retirement and claims its
  reissued work before any waiter notification.
- 117.004-T: metrics branch control uses an awaited channel send and writer
  acknowledgment; ordinary usage events remain non-blocking and droppable.
- 117.002-T: added the no-prior canonical-snapshot invalid-UTF-8 retry
  regression, including clean publication and the following hash-skip control.
- 117.003-T: function, class, interface, and edge counts now use the bounded
  immutable SQLITE_BUSY retry path; S072 uses test-owned storage.

## Hidden all-target failure

Workspace capture in `logs/111-s-cargo-test-all-targets.log` revealed:

```text
integration_smoke::s072_workspace_status_reports_code_graph_counts
fixture must index at least two functions, got 0
```

The failure was pre-existing environmental contamination, not caused by U1:
ambient `ENGRAM_DATA_DIR=C:\Source\GitHub\engram\.engram` redirected S072's
disposable fixture into the live workspace database. S072 now binds an isolated
workspace-local data directory. The focused test and full all-target suite pass.

## Verification

- `cargo fmt --all -- --check`: PASS
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: PASS
- `cargo test --all-targets`: PASS
- Focused U1, U2, U3, and U4 regressions: PASS
- `cargo audit`: ACCEPTED BASELINE — one vulnerability,
  `RUSTSEC-2026-0041`, plus the same 13 allowed informational warnings

The vulnerable dependency chain is:

```text
engram -> cozo 0.7.6 -> swapvec 0.3.0 -> lz4_flex 0.10.0
```

`swapvec` requires `lz4_flex ^0.10.0`, while patched versions begin at 0.11.6.
Authoritative deliberation `017-D` and
`docs/decisions/2026-07-28-cargo-audit-advisory-triage.md` accept this exact
upstream-pinned advisory with low-exposure rationale and non-blocking CI.
Shipment 110-S used the same command-line ignore without adding repository
policy. The current audit is an exact baseline match: one vulnerability, the
same dependency chain, and 13 unchanged allowed warnings. Neither `Cargo.toml`
nor `Cargo.lock` differs from `origin/main`, so shipment 111-S does not worsen
the dependency source or advisory surface. Follow-up stash: `27F691AE`.

## Knowledge and policy

- Compound learning:
  `docs/compound/workflow-issues/dynamic-diagnostic-escalation-2026-08-08.md`
- Formal circuit-breaker policy follow-up stash: `241B503F`

## Resume steps

1. Run the structured review gate and bounded fixes.
2. Complete commits, push, PR review, merge-commit verification, runtime
   verification, shipment reconciliation, and closure.
3. Keep 112-S through 114-S queued.
