# Ship Session Checkpoint — 135-S Preflight Complete, Halted at Destructive-Action Gate

**Date**: 2026-09-05
**Shipment**: 135-S — "Retire HTTP and SSE transport surfaces"
**Branch**: `feat/135-s-retire-http-and-sse-transport-surfaces`
**Mode**: P-017 dark-factory, `merge_approval_pre_authorized=false`,
`admin_fallback_pre_authorized=false`. Run scope: 135-S → 142-S (this
invocation owns only 135-S).

## Completed this session

1. Tool availability gate: backlogit 1.10.1 OK, gh OK, autoharness gate CLI OK,
   cargo OK. `ALL_TOOLS_OK`.
2. `backlogit sync` → `INDEX_SYNC_OK` (1307 artifacts indexed).
3. Verified no other active shipment (`backlogit shipment list --status active` → `[]`).
4. Verified 135-S manifest: `142.023-T, 142.024-T, 142.025-T, 142.026-T`, all
   `status: queued` (no `SHIPMENT_STATE_INCONSISTENT`).
5. Verified dependency 133-S is `archived` / `archived_status: done`.
6. `autoharness gate pipeline-topology --phase pre_claim --shipment 135-S` →
   exit 0, all checks passed, `BRANCH_CREATE_ELIGIBLE`.
7. **Operator-approved carry-forward**: worktree was dirty on `main` with exactly
   the operator-authorized carry-forward set (checkpoint quarantine/archival +
   dispositions, `.backlogit/stash.jsonl` incident 0011C2DC, incident handoff
   design doc). Per explicit operator direction, deviated from the literal
   "halt on dirty worktree" pre-branch check: created the branch directly from
   the dirty tree (`git checkout -b` carries working-tree changes forward
   without ever committing to `main`), then committed the carry-forward set as
   the **first commit** on the new branch (`2d2c1324`). `main` was never
   touched. Working tree is now clean and ready for 135-S implementation.
8. Re-ran `pipeline-topology --phase pre_claim` (immediately before claim) →
   exit 0, `BRANCH_OK`.
9. `backlogit shipment claim 135-S` → `status: active`.
10. `pipeline-topology --phase post_claim` (Unit A GLOBAL verify) → exit 0,
    `active_shipment_ids: ["135-S"]` (sole active). CLI re-read confirms
    `status: active`. `CLAIM_VERIFY_OK`.
11. Manual intake reconciliation (shipment-reconcile equivalent, mode:pre,
    expected_status:queued): all 4 manifest items `matched` in queue with
    `status: queued`; no orphans found declaring 135-S; recommendation
    `PROCEED`.
12. Re-read constitution Principles I, II, IV. Baseline `cargo check
    --all-targets` → GREEN (Finished in 1m55s) on branch HEAD before any
    135-S edits.
13. **Harness/implementation scoping research (non-destructive, read-only)**:
    - Confirmed no task in the manifest carries `harness-ready`.
    - `tests/contract/supported_transport_surface_test.rs` already exists
      (registered in Cargo.toml as `contract_supported_transport_surface` by
      F00/133-S) but is an inert placeholder (`fn placeholder_registered() {}`).
      This is the intended shared RED/GREEN harness for 142.023-T + 142.024-T.
    - Confirmed `axum`, `tower`, `tower-http`, `tokio-stream` are used
      **only** inside `src/server/{mcp,router,sse}.rs` plus two other test
      files (see below); `sysinfo` is used widely elsewhere and must NOT be
      removed.
    - Confirmed **all** `#[cfg(feature = "legacy-sse")]` sites in the repo:
      `src/server/mod.rs` (×3, deleted with 142.023-T),
      `tests/integration/connection_test.rs` (whole file, deleted with
      142.023-T), **plus two sites not listed in either task's "owned files"**:
      `tests/contract/lifecycle_test.rs::contract_rate_limiting_rejects_excess_connections`
      and `tests/integration/benchmark_test.rs::t097_cold_start_under_200ms`.
      Both import `engram::server::router` / `axum`/`tower` directly.
    - **Scope determination (P-021 C1/C3)**: removing the `legacy-sse` feature
      (142.024-T's first acceptance criterion) makes any remaining
      `#[cfg(feature = "legacy-sse")]` an `unexpected_cfgs` lint hit, which
      becomes a hard error under the workspace's `cargo lint` gate
      (`clippy --all-targets --all-features -D warnings -D clippy::pedantic`).
      Both tasks' own verification steps require `cargo lint`/`cargo ci`
      **all-features** GREEN. Removing these two now-dead, feature-gated test
      functions is therefore a **same-contract-surface completion of the
      already-authorized legacy-sse retirement**, not a scope expansion — it
      is required to satisfy 142.024-T's own stated acceptance criteria. This
      will be called out explicitly in the task completion note and commit
      message rather than silently expanded into.
    - Confirmed no other workspace crate (`crates/engram-indexer`,
      `crates/powerbi-tmdl-parser`) depends on axum/tower/tower-http/tokio-stream.
    - Confirmed `src/installer/mod.rs`'s HTTP-endpoint claim is actually
      rendered by `src/installer/templates.rs` (`copilot_instructions` /
      `claude_instructions`, hard-coded `http://127.0.0.1:{port}/mcp`).
      Fixing 142.025-T's acceptance criterion ("`src/installer/mod.rs` no
      longer advertises HTTP ports or hooks") requires touching this template
      content too (mod.rs only calls it); this is treated as in-scope,
      minimal, doc-content-only collateral of the same task, not a new
      contract surface. No CLI flag / `InstallOptions.port` / `DEFAULT_PORT`
      removal is planned (that would be a broader behavior change the task
      explicitly disclaims: "No behavior change beyond removing the retired
      claims").
    - Confirmed both target ADRs (`docs/adrs/0016-legacy-sse-feature-gate.md`,
      `docs/adrs/0003-sliding-window-rate-limiter.md`) exist and are currently
      `Status: Accepted` (142.026-T scope).

## HALTED — Destructive-Action Approval Gate (NON-NEGOTIABLE)

`strict-safety` capability pack is enabled
(`.autoharness/config.yaml: strict_safety.require_approval_for: [destructive]`)
and 142.023-T's own acceptance criteria states: *"Operator approval is
recorded before execution."* Constitution Principle VII and
`safety-modes` Step 5 require explicit operator approval before any
`ActionRisk: destructive` action proceeds — this is a hard gate, not a
judgment call, and is independent of the later P-014 merge-approval gate.

**ProposedAction**

| Field | Value |
|---|---|
| summary | Delete 3 retired HTTP/SSE server modules + their test/registration, remove the `legacy-sse` feature and its 4 now-unused deps, remove 2 orphaned feature-gated test functions that reference the deleted code, rewrite installer hook-doc claims and `docs/architecture.md`, mark 2 ADRs superseded |
| targets | `src/server/mcp.rs`, `src/server/router.rs`, `src/server/sse.rs` (delete); `src/server/mod.rs` (edit); `tests/integration/connection_test.rs` (delete); `tests/contract/lifecycle_test.rs`, `tests/integration/benchmark_test.rs` (remove 1 function each); root `Cargo.toml` (remove 1 `[[test]]` block, `legacy-sse` feature, `axum`/`tower`/`tower-http`/`tokio-stream` deps); `tests/contract/supported_transport_surface_test.rs` (write real assertions); `src/installer/mod.rs`, `src/installer/templates.rs`, `docs/architecture.md` (edit); `docs/adrs/0016-*.md`, `docs/adrs/0003-*.md` (edit) |
| change_kind | deletion + config/dependency change + documentation correction |
| rollback | All work is on `feat/135-s-...` branch, never touches `main`; fully revertible via `git revert`/`git checkout` prior to merge; no runtime/production data touched |
| approval_required | **yes** |

**ActionRisk**: `destructive` (irreversible source deletion; feature/dependency removal)
**ActionResult**: `blocked` — awaiting explicit operator approval

I am halting here, before any file is deleted or modified, and requesting
your explicit approval to proceed with the 135-S implementation as scoped
above.
