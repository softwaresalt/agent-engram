---
title: "Fsync the workspace identity parent directory through capability APIs"
type: implementation-plan
date: 2026-08-24
status: blocked-review
source: docs/decisions/2026-08-24-workspace-id-parent-fsync-decision.md
source_stash_id: "5DF94427"
---

# Fsync the workspace identity parent directory through capability APIs

## Problem Frame

The staged identity file is content-synced before `hard_link`/rename publication, but `.engram` itself is not synced after the final directory entry is created and staging cleanup completes. A Unix crash can lose `.workspace-id` and remint identity.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| Capability-safe directory fsync | U2 uses safe `Dir::reopen_dir` plus `into_std_file().sync_all()`. |
| Correct ordering | U1 RED requires content sync, publish, cleanup, parent sync, then success. |
| Error honesty | U1/U2 require parent-sync failure to prevent success. |
| Preserve publication semantics | Hard-link-first and checked-rename fallback remain unchanged. |

## Implementation Units

### U1 — RED: publish protocol ordering

Add a colocated deterministic protocol hook and tests in `src/db/workspace.rs`. One success scenario must record `file-sync -> publish -> cleanup -> parent-sync -> return`; one injected parent-sync failure must not report success. Current code must fail because no parent-sync event exists. Test-only seam, two scenarios, target 90 minutes.

### U2 — GREEN: safe parent-directory sync

Add a private safe helper that duplicates/reopens the retained directory capability, converts the duplicate to `std::fs::File`, and calls `sync_all()`. Invoke it after final-name publication and staging cleanup before returning success. Preserve the publication error when publication failed; do not silently report durability after sync failure. No ambient path, raw handle, unsafe, dependency change, or broad publish rewrite. One file, fewer than four functions, target 90 minutes.

### U3 — Platform verification and closure

Run targeted RED/GREEN evidence, concurrent cold-start identity coverage, primary/worktree bind, Windows/Linux CI, and a Unix filesystem check confirming directory sync succeeds. Record Windows behavior rather than assuming parity. Verification only, target 90 minutes.

## Dependency Graph

`U1 -> U2 -> U3`. Independent from 1CB366DB/7B15B447 and excluded from their shipment width.

## Decisions and Rationale

- Sync after staging cleanup so both final link creation and temp unlink share one directory durability barrier.
- Use a reopened duplicate so the original retained `CapRoot` remains available for winner read-back.
- Test protocol ordering through a deterministic seam; a normal unit test cannot honestly simulate power loss.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Directory `sync_all` platform behavior differs | Unix is mandatory; record and disposition Windows explicitly. |
| Sync failure masks publish race semantics | Preserve `AlreadyExists`/publish errors; only success requires durability. |
| Added fsync affects startup latency | Measure cold start and use the 121-S absolute budget. |

## Plan Hardening Signals

- Public API/schema/contract: absent.
- Security-sensitive behavior: present adjacent to persisted workspace identity.
- Migration/destructive action: absent.
- External integration/checkpoint: absent.
- High runtime/rollback risk: present; every cold first bind publishes identity.

Requires plan hardening: yes

## Runtime Verification and Closure

Success signal: persisted ID survives restart and concurrent starters agree; Unix directory sync succeeds; no ambient access. Roll back U2 on legitimate bind failure or startup latency above budget. Owner: Ship; 48-hour observation for identity/key stability.

## Plan Hardening

| ProposedAction | targets | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|
| Add parent-directory durability barrier | `src/db/workspace.rs` | high | revert U2; restore known durability residual | preferred | planned |

Protected invariants: atomic complete-content publication, no-clobber on hard-link filesystems, checked fallback, same retained authority, no false crash test claim.

## Plan Review

Gate: **PASS (standard review only)**. Hardening required and present.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| A1 | P1 | Consuming the only `Dir` could lose authority needed for read-back. | Resolved: reopen/duplicate first, consume only the duplicate. |
| T1 | P1 | A simple restart test does not prove crash durability. | Resolved: protocol-order RED plus explicit operational claim limits. |
| S1 | P1 | Sync before cleanup does not durably remove staging residue. | Resolved: cleanup precedes the parent sync. |

No unresolved standard-review P0/P1 finding remains. Review-fix cycles: 1 of 3.

## Adversarial Multi-Model Review

Gate: **BLOCKED**. This changes security-relevant workspace identity publication and the requested cross-model reviewer surface is unavailable. No feature, tasks, stash archival, or shipment is allowed until genuine multi-model review clears the plan.
