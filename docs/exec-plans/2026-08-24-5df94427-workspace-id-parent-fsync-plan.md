---
title: "Fsync the Unix workspace identity parent with explicit failure precedence"
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: reviewed-ready
source: docs/decisions/2026-08-24-workspace-id-parent-fsync-decision.md
source_stash_id: "5DF94427"
---

# Fsync the Unix workspace identity parent with explicit failure precedence

## Problem Frame

The staged identity file is content-synced before hard-link or checked-rename publication, but `.engram` itself is not synced after the final directory entry is created and staging cleanup completes. A Unix crash can lose `.workspace-id` and remint identity. The safe `reopen_dir`/`into_std_file`/`sync_all` route is Unix-only: cap-std Windows directory handles lack the write access required by `FlushFileBuffers`.

The caller currently converts `AlreadyExists` into a successful handle-derived winner read. Therefore, syncing only the new-publish path or preserving `AlreadyExists` ahead of sync failure would still report success without the promised durability barrier. The plan must define exact precedence for all publication outcomes.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| Capability-safe Unix directory fsync | U2 uses safe `Dir::reopen_dir` plus `into_std_file().sync_all()` behind `cfg(unix)`. |
| New-file ordering | U1 requires content sync, publish, cleanup, parent sync, then handle-derived read/return. |
| Existing-winner ordering | U1 requires `AlreadyExists`, cleanup, parent sync, then winner read/return. |
| Error precedence | U1/U2 make parent-sync failure override both successful outcomes; unrelated publication failures retain their original error. |
| Safe failure injection | U1 uses a private safe seam or wrapper; no unsafe, raw handle, ambient reopen, or permission mutation. |
| Windows safety | U1/U2 prove Windows never calls the write-required directory flush and preserve its residual. |
| Preserve publication semantics | Hard-link-first and checked-rename fallback remain unchanged. |

## Implementation Units

### U1 — RED: table-driven publish protocol and precedence

Add a colocated deterministic platform-gated protocol seam and test matrix in `src/db/workspace.rs` before production behavior changes. The Unix matrix must cover:

1. New-file publication: `file-sync -> publish -> cleanup -> parent-sync -> read/return`; injected parent-sync failure returns the sync-derived error instead of the minted UUID.
2. Existing winner: `file-sync -> AlreadyExists -> cleanup -> parent-sync -> winner read/return`; injected parent-sync failure overrides a valid winner-read result.
3. Other publication failure: preserve the existing publication-derived error and do not mask it with parent sync.

A Windows-gated assertion requires no parent-sync event. Use a private safe closure, trait, or wrapper to inject sync success/failure deterministically; no unsafe code, raw handles, ambient path, permission mutation, sleep, or false power-loss claim. Current Unix code must fail because the parent-sync event and precedence are absent. One test module and table-driven matrix, target 100 minutes.

### U2 — GREEN: safe barrier and exact error ordering

Behind `cfg(unix)`, add a private safe helper that reopens/duplicates the retained directory capability, converts only the duplicate to `std::fs::File`, and calls `sync_all()`. Keep the original retained `CapRoot` available for handle-derived reads.

After `publish_new_child_file` completes cleanup:

- `Ok(())`: parent-sync first; on sync error return the durability error; otherwise perform the handle-derived read and return.
- `AlreadyExists`: parent-sync first; on sync error return the durability error; otherwise read and return the existing winner.
- Any other publication error: preserve the current publication-derived error and do not replace it with sync failure.

Do not invoke this helper on Windows. Do not change hard-link-first behavior, checked-rename fallback, no-clobber semantics, or read authority. No ambient path, unsafe, raw handle, dependency change, or broad publish rewrite. One production file, fewer than four functions, target 90 minutes.

### U3 — platform verification and closure

Run targeted RED/GREEN evidence, concurrent cold-start identity coverage, primary/worktree bind, Windows/Linux CI, and a Unix filesystem check confirming directory sync succeeds. Verify the table-driven failure injection, including new-file and `AlreadyExists` precedence, and confirm unrelated publication errors remain unchanged. Confirm Windows first bind remains green with no parent-sync attempt and record the durability residual. Verification only, target 90 minutes.

## Dependency Graph

`U1 -> U2 -> U3`. Independent from `1CB366DB` and `7B15B447`; excluded from their shipment width. No feature, tasks, or shipment may be harvested while adversarial review remains blocked.

## Decisions and Rationale

- Sync after staging cleanup so final link creation and temporary unlink share one directory durability barrier.
- Attempt the barrier on `Ok` and `AlreadyExists` because each can otherwise produce a successful UUID result.
- Give parent-sync error precedence over both success outcomes; durability cannot be claimed after a failed barrier.
- Preserve unrelated publication failures because they already identify the operation that failed and no success is being reported.
- Reopen a duplicate so original retained authority remains available for winner read-back.
- Use a deterministic safe seam; normal unit tests cannot honestly simulate power loss.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Windows `FlushFileBuffers` requires write access absent from the cap-std directory handle | Compile and invoke the helper only on Unix; preserve the Windows residual. |
| `AlreadyExists` winner path bypasses durability | Parent-sync before winner read/return; sync error overrides winner success. |
| Sync error masks unrelated publication failure | Sync only after `Ok` or `AlreadyExists`; preserve every other publication-derived error. |
| Consuming the retained `Dir` loses read authority | Reopen/duplicate first and consume only the duplicate. |
| Tests claim crash proof | Assert protocol events and injected errors only; make no power-loss reproduction claim. |
| Added fsync affects startup latency | Measure cold start against the 121-S absolute budget. |

## Plan Hardening Signals

- Public API/schema/contract: absent.
- Security-sensitive behavior: present adjacent to persisted workspace identity.
- Migration/destructive action: absent.
- External integration/checkpoint: absent.
- High runtime/rollback risk: present; every Unix cold first bind publishes identity.

Requires plan hardening: yes

## Runtime Verification and Closure

Success signals: persisted ID survives restart, concurrent starters agree, Unix directory sync succeeds on both new-file and existing-winner paths, injected sync error blocks both success outcomes, unrelated publication errors are preserved, Windows first bind never attempts the barrier, and no ambient access appears. Roll back U2 on legitimate bind failure, precedence mismatch, or startup latency above the 121-S budget. Owner: Ship. Observation window: 48 hours after a future reviewed shipment. Windows crash durability remains open.

## Plan Hardening

Hardening rerun: **required and satisfied for standard review**.

Reinforcing context: `.github/instructions/strict-safety.instructions.md`, `.github/instructions/adversarial-review.instructions.md`, the retained-capability compound learning, current `workspace_id_from_metadata`, and PR #363 thread `discussion_r3848530354`.

| ProposedAction | targets | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|
| Add Unix parent-directory durability barrier with explicit success-path precedence | `src/db/workspace.rs` | high | revert U2; restore documented durability residual | preferred before implementation | planned |
| Add safe deterministic sync-failure seam for RED tests | colocated test-only workspace code | moderate | remove seam with U1 | no | planned |

Protected invariants: atomic complete-content publication, no-clobber behavior, checked fallback, retained capability authority, parent-sync failure overrides `Ok` and `AlreadyExists` success, unrelated publication errors retain precedence, no false crash claim, and no write-required flush through Windows read-only directory handles.

## Plan Review

Gate: **PASS (standard review only)**. Standard plan review was rerun after the operator-authorized remediation pass. Hardening is required and present. Personas applied locally: constitution, Rust/API, architecture, scope boundary, test strategy, security lens, operational readiness, and learnings.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| A1 | P1 | Consuming the only `Dir` could lose authority needed for read-back. | Resolved: reopen a duplicate and consume only the duplicate. |
| T1 | P1 | A restart test does not prove crash durability. | Resolved: deterministic protocol order and explicit claim limits. |
| S1 | P1 | Sync before cleanup does not durably remove staging residue. | Resolved: cleanup precedes parent sync. |
| W1 | P1 | Windows directory `sync_all()` can fail because cap-std opens the handle read-only. | Resolved: barrier and ordering tests are Unix-only; Windows residual remains explicit. |
| E1 | P1 | Ambiguous precedence could let `AlreadyExists` winner read report success after parent-sync failure. | Resolved: sync error overrides `Ok` and `AlreadyExists`; other publication failures retain their original error. |
| E2 | P2 | Failure injection might depend on unsafe or host permissions. | Resolved: private safe seam/wrapper only; no unsafe, raw handle, ambient reopen, or permission mutation. |

No unresolved standard-review P0/P1 findings remain. This operator-authorized remediation pass follows the prior three-cycle stop without weakening the gate.

## Adversarial Multi-Model Review — Cycle 5 Final

Gate: **PASS WITH LOW ADVISORY**. The valid three-model four-plan review found no HIGH, MEDIUM, P0, or P1 issue for this plan. Its single LOW API-feasibility advisory is explicit: confirm the exact safe public `Dir::reopen_dir`/`into_std_file` signatures against pinned cap-std 4.0.2 before GREEN; fail closed rather than using unsafe, raw handles, or ambient reopen. The final bounded rerun reconfirmed this unchanged pass and separate release width. Review-fix cycles: 0 of 3.

Evidence: `docs/closure/2026-08-24-dark-factory-cycle5-four-plan-adversarial-review-rerun.md` and `docs/closure/2026-08-24-dark-factory-cycle5-four-plan-adversarial-review-final.md`.
