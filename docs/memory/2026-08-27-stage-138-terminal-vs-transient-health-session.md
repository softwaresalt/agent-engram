---
title: Stage session — re-scope 137.006-T into 138-F and unblock 130-S closure
date: 2026-08-27
type: session-memory
doc_type: memory
agent: stage
worktree: .worktrees/ship-137-late-readiness-proxy-recovery-20260826
branch: chore/130-s-post-merge-closure
feature: 138-F
shipment: 131-S
unblocked: 130-S
---

# Stage session — 2026-08-27

> [!NOTE]
> **No RCA restated.** Root cause for the late-readiness sticky-proxy defect
> lives in
> `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md`.
> Prior verification-shipment evidence is in
> `docs/exec-plans/2026-08-26-137-late-readiness-proxy-recovery-verification-plan.md`,
> `docs/closure/130-S-2026-08-27-post-merge-closure.md`, and
> `docs/closure/130-S-2026-08-27-runtime-verification.md`. This memory records
> only the staging decisions.

## Operating Mode

* **P-012 degraded CLI mode declared.** backlogit MCP was not exposed to this
  session. The registry (`.autoharness/backlog-registry.yaml`) declares
  `cli.binary: backlogit` and per-operation `cli_command` fallbacks, so every
  backlog mutation used backlogit CLI **v1.10.1** scoped to this worktree. No
  ad-hoc filesystem editing of artifact IDs or frontmatter was used.
* **Engram discovery unavailable.** `engram daemon-status` failed with
  `daemon unavailable: Daemon failed to reach Ready state within 30000ms` — an
  in-the-wild instance of the very failure class this feature addresses. Engram
  MCP surface was not exposed either, so source grounding fell through to the
  third tier (`Select-String` / file reads), as the fallback ladder permits only
  after both prior tiers are unavailable.
* `INDEX_SYNC_OK` at session start and end.

## Problem Solved

`130-S` had the exact manifest `137-F`, `137.001-T`..`137.005-T`, all terminal.
It could not be shipped because `137.006-T` — a queued Copilot-review follow-up
from PR #364 that was **never an exact manifest member** — was a child of
covering feature `137-F`. backlogit expands the covering-feature relationship,
so the queued child made the expansion non-terminal and blocked
`backlogit shipment ship 130-S`.

The wrong fixes (rejected): force-override the ship gate; falsely mark
`137.006-T` done; implement the fix inside the verification-only `130-S` scope;
delete and recreate the task, losing its history.

The right fix: re-scope the follow-up out from under `137-F` into its own
independently owned release unit.

## What Was Done

1. Created `138-F` as an independently owned reliability feature.
2. **`backlogit adopt 137.006-T --parent 138-F`** — native, safe re-parent.
   Returned `{ new_id: 138.001-T, origin_feature: 137-F, was_orphan: false }`.
   No clone, no duplicate, history preserved, `origin_feature` retained in
   `custom_fields`.
3. Authored the implementation plan, ran mandatory hardening (elevated blast
   radius: error taxonomy + MCP wire contract + shim public API), then the
   adversarial review gate (`138.001-R`, approved with changes, 2 cycles).
4. Harvested 7 tasks with a harness-first dependency DAG.
5. Created **one** queued shipment `131-S`. Not claimed.

## Key Technical Finding (drives the whole design)

`fetch_health` constructs `IpcError::ReceiveFailed` for three genuinely
**terminal** cases — daemon `_health` error object, missing `result`, undecodable
payload — using the *same variant* `ipc_client::send_request` yields for
genuinely **transient** transport failures. Downstream re-classification by
matching on the returned `EngramError` is therefore provably impossible. The
discriminant must be preserved **at construction**, which is why the fix is a
result-preserving return type and not a `matches!` filter at the call sites.

`ensure_daemon_running_inner` already classifies `VersionMismatch` correctly on
the *pre*-deadline path; only the *post*-deadline recovery path lost it. The fix
restores symmetry rather than inventing semantics.

## Decisions Recorded

| Decision | Rationale |
|---|---|
| Add `ShimFailureClass::ProtocolIncompatible` (exit 14, wire 15005) rather than reuse | `readiness_timeout` misreports a protocol mismatch; `transport_failure` misattributes a *successful* transport round trip. Additive; no existing value changes. |
| Record-reader tolerance for an unknown `failure_class` is a **veto-capable** check on `138.003-T` | If an older binary hard-fails on the new string, D2 rolls back to `TransportFailure` + sub-field and the plan returns to review. |
| `retry_after_ms` must be **key-absent** for terminal, not `null`/`0` | Agents branch on key presence; a value is a fail-open signal. |
| Over-terminalization is the dominant risk, not the bug | A false `Terminal` permanently kills a healthy warm-up session — worse than the reported defect. Rule: `Terminal` requires a received response that *proves* incompatibility; silence/timeout/reset are always `Transient`. `R1`/`R2`/`R3` are the guards. |
| Probe seam is a default-installed indirection, not `#[cfg(test)]` | `#[cfg(test)]` branching inside `forwarding_endpoint` would mean the tested path is not the shipped path. |
| Concurrency proven by `Barrier` + `tokio::time::pause`, never `sleep` | A sleep-seeded test may serialize and still pass — it proves nothing and flakes in CI. |
| No fail-open escape hatch / feature flag | An env-gated bypass of a fail-closed path re-opens the exact hole being closed. Rollback is by revert; every layer is independently revertible. |
| No `131-S` → `130-S` shipment block edge | `130-S` is mid-closure; adding an edge risked mutating it. Ordering is documented instead. |

## Verified Outcomes

* `137-F` hierarchy is now `137-F` (active) + 6 terminal children; **zero**
  queued/active/blocked descendants. `130-S` closes without `--force`.
* `130-S` manifest re-read and byte-identical: `137-F`, `137.001-T`..`137.005-T`.
  Its rollup dropped from 6 unsized members to 5 — the only change, and it is a
  computed-on-read projection, not persisted state.
* `backlogit doctor`: 43 findings, all pre-existing `archived_from_self_ref` on
  legacy archives (031–062 range). Zero orphans, zero duplicates, nothing
  touching 130/131/137/138.
* `git status` confirms zero source/test/config files modified.

## Next Steps (Ship)

1. Commit and push the Stage artifacts on `chore/130-s-post-merge-closure`.
2. Complete `137-F` and close `130-S` — no force override required.
3. **Do not claim `131-S` / `138.002-T` yet.** PR #365's Copilot review
   surfaced 12+ plan/task-feasibility defects against the `138-F` plan and
   its harvested tasks (harness/seam compile-ordering, a concurrency-test
   barrier deadlock, a `std::time::Instant` vs. `tokio::time` clock-seam
   gap, an unverifiable compatibility-veto consumer, a missing
   terminal-record write path, over-broad JSON-RPC-error classification,
   `138.002-T`'s task-granularity/scope violation, IPC-framing assumptions
   in the hardening doc, and `expected`/`actual` fields required on
   terminal outcomes that cannot supply them). See the consolidated list in
   `docs/memory/2026-08-27/130-s-ship-session-post-merge-closure-memory.md`
   (Session 2, PR #365 review-fix cycles section). Stage must revise the
   plan/hardening and re-run `138.001-R` (or record a formal addendum)
   before `138.002-T` is claimed — otherwise an agent following only this
   memory could start a known-unexecutable plan.
