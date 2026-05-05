---
title: "Operational Closure — 040-F: SQLITE_BUSY Mutable-Script Retry Metrics & MCP Tool"
date: 2026-05-04
mode: post-merge
feature: 040-F
shipment: 023-S
pr: 78
branch: feat/040-F-sqlite-busy-retry-metrics
merge_sha: 38ae7e0
status: CLOSED
---

## Summary

Adds process-global telemetry for SQLITE_BUSY retries in the mutable CozoDB write path.
Two `AtomicU64` statics (`MUTABLE_RETRY_COUNT`, `MUTABLE_LAST_RETRY_EPOCH_MS`) are
incremented on each SQLITE_BUSY retry in `run_script_busy_retry_mutable`. A new read-only
MCP tool `get_mutable_script_retry_metrics` exposes a snapshot `{ retry_count, last_retry_at }`
without requiring workspace binding.

**Commits on branch:**

| SHA | Message |
|---|---|
| `f94878e` | test(build): scaffold harness for 040-F sqlite busy retry metrics |
| `9adf98e` | feat(db): implement SQLITE_BUSY mutable-script retry metrics (040-F) |
| `d38c41a` | docs(memory): session memory for 040-F sqlite busy retry metrics |
| `fe4685d` | fix(db): address copilot review comments on 040-F |
| `4973afb` | docs(closure): pre-merge closure artifact for 040-F |
| `9d6f154` | fix(db): second-round copilot review fixes |
| `19bf56f` | fix(build): redundant closure clippy CI fix |
| `03761d6` | fix(test): module doc and monotonicity assertion improvements |
| `abcd33e` | fix(docs): catalog scope and sentinel invariant wording |
| `793ea76` | fix(test): rename test fn; strengthen u64 assertion; mark tasks done |

**Merge commit:** `38ae7e0` on `main`

## CI Status

- CI/build: **GREEN** — all checks passing on `793ea76` (latest commit before merge)
- Pre-existing failures: `contract_shim_lifecycle` (6 tests) — daemon-spawn environment issue;
  confirmed failing on `main` baseline before this branch. Not caused by 040-F.
- Merge: `38ae7e0` merged via admin override (review approval required; user authorized)

## Review Status

- Copilot review (bot): 22 total comments across 4 rounds — all **addressed and resolved**
  - Round 1 (11): stash provenance, tool naming, visibility, serialization guard, assertions
  - Round 2 (3): module docs, tautological assertion, catalog scope
  - Round 3 (4): module doc, monotonicity test, catalog scope, sentinel invariant
  - Round 4 (4): test fn name, u64 domain assertion, task status cleanup
- Rubber-duck review gate: **PASS** — no P0/P1; two P3 advisories (independent atomic sampling,
  timestamp sentinel overlap) — both accepted for lightweight telemetry use case

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy -- -D warnings -D clippy::pedantic` | PASS |
| `cargo test` (excl. pre-existing failures) | PASS |

## Invariants to Preserve

1. `MUTABLE_RETRY_COUNT` is monotonically non-decreasing across the process lifetime.
2. `MUTABLE_LAST_RETRY_EPOCH_MS == 0` is the authoritative sentinel for "no retry yet";
   the guard `epoch_ms != 0` is checked before calling `DateTime::from_timestamp_millis`
   (which would return `Some(epoch)` for `0`, not `None` — the sentinel is enforced by the
   explicit `!= 0` check, not by a chrono `None` return).
3. `TOOL_COUNT == 18` — catalog contract test `t010_05_tool_count_matches_catalog` enforces this.
4. `get_mutable_script_retry_metrics` returns successfully without a bound workspace.
5. `Ordering::Relaxed` is the correct ordering for independent monotonic telemetry counters;
   no cross-atomic invariant requires stronger ordering.
6. Pre-existing `contract_shim_lifecycle` test failures must not increase in count.

## Pre-Deploy Audit

| Check | Status | Notes |
|---|---|---|
| No new external dependencies | PASS | Only `std::sync::atomic` and existing `chrono` used |
| No schema or DB migration required | PASS | AtomicU64 statics live in process memory only |
| No new feature flags required | PASS | Tool available unconditionally (like `get_health_report`) |
| Rollback procedure documented | PASS | See below |
| TOOL_COUNT catalog test updated | PASS | `t010_05_tool_count_matches_catalog` asserts 18 |
| Dispatch table updated | PASS | `src/tools/mod.rs` routes `get_mutable_script_retry_metrics` |

## Deployment / Rollout Path

**Merge-only** — no deployment step required. This is a local-first daemon; changes take
effect when the process is next started. No canary, phased rollout, or maintenance window needed.

## Post-Deploy Checks

After merge and next daemon restart:

1. Call `get_mutable_script_retry_metrics` via any MCP client — verify it returns
   `{ retry_count: 0, last_retry_at: null }` on a fresh start.
2. Confirm `TOOL_COUNT` is consistent with the catalog by running
   `cargo test --test contract_metrics_tools`.
3. If SQLITE_BUSY retries are artificially triggered (stress test or manual), confirm
   `retry_count` increments and `last_retry_at` becomes non-null.

## Risky Action Record

No destructive or high-blast-radius actions were taken. All changes are additive:
- New statics (process memory only)
- New public function
- New MCP tool dispatch arm
- Updated catalog count

**ActionRisk**: `low` — read-only observability addition, zero external surface expansion.

## Healthy Signals

- `get_mutable_script_retry_metrics` returns HTTP 200 / MCP success with `retry_count >= 0`
- No increase in pre-existing `contract_shim_lifecycle` failure count
- `cargo test` remains fully green except the known pre-existing failures

## Failure Signals

- `get_mutable_script_retry_metrics` returns an error — would indicate dispatch registration issue
- `TOOL_COUNT` contract test fails — would indicate catalog / dispatch drift
- `retry_count` stops incrementing under SQLITE_BUSY load — would indicate the increment
  was accidentally removed from `run_script_busy_retry_mutable`

## Monitoring Plan

| Signal | Method | Threshold |
|---|---|---|
| Retry counter drift | Call `get_mutable_script_retry_metrics` on-demand | Unexpected 0 after known SQLITE_BUSY activity |
| Tool availability | Catalog presence test (`contract_retry_metrics_tool`) | Test failure |
| SQLITE_BUSY frequency | `retry_count` delta between polls | Operator-defined; no hard threshold for this release |

No persistent dashboard required — counter is queryable on-demand via MCP tool.
Alert infrastructure not applicable to this local-first daemon without OTLP.

## Rollback Trigger

If `get_mutable_script_retry_metrics` causes a panic or returns a tool-not-found error in
a live deployment, indicating a catalog/dispatch registration issue.

## Rollback Procedure

```bash
git revert --no-edit -m 1 <merge_sha>
# Then open a PR for the revert commit, following the standard merge-commit PR flow.
```

The change is fully additive; the revert removes the two statics, the function, and the
tool dispatch arm cleanly with no migration required.

## Validation Window

**48 hours** after merge. Owner: operator on call.
The counter resets on daemon restart, so watch for unexpected 0 values after restarts.

## Source Artifact Cleanup

| Item | Field | Value | Notes |
|---|---|---|---|
| 040-F | `source_stash_id` | `51B936CD` | Record for manual stash retirement; no automated retirement available in registry |
| 040-F | `source_deliberation_id` | absent from `custom_fields` | Deliberation path recorded in body: `docs/decisions/2026-05-04-sqlite-busy-retry-metrics-deliberation.md` |

## Follow-Up Items

The following advisory items surfaced during build/review (P3, non-blocking):

1. **OTLP bridge for retry counter** — Bridge `MUTABLE_RETRY_COUNT` to OpenTelemetry when
   the `otlp-export` feature is active. Deferred per deliberation decision (Option B rejected
   for now). Source: deliberation doc.
2. **Timestamp granularity** — `last_retry_at` has epoch-ms precision; clock skew or rapid
   retries within the same millisecond will produce identical timestamps. Advisory only —
   acceptable for diagnostic use.

## Readiness

**READY** — all quality gates pass, all review comments addressed, CI pending (expected to pass
based on local gate results). Pre-existing `contract_shim_lifecycle` failures are environment
issues confirmed on `main` baseline; they are not caused by this feature and are not a merge
blocker.

**Awaiting operator merge approval on PR #78.**
