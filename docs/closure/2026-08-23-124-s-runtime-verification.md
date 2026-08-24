---
title: "124-S runtime verification — Copilot pre-initialize server/discover compatibility"
doc_type: closure
date: 2026-08-23
shipment_id: "124-S"
feature_id: "130-F"
pr: 359
merge_commit: 8f9904a0a55516582e101d7b75b9457adaf9a0be
status: done
engram_status: degraded
---

## Scope

Post-merge runtime verification for shipment 124-S, executed in the isolated
worktree `.worktrees/post-merge-124-s-closure-20260823` on branch
`post-merge/124-s-closure` at merge commit `8f9904a0…`.

## Merged-Tree Equivalence (primary evidence)

The strongest available runtime evidence is that CI ran against the exact tree
that landed on `main`:

```text
git diff --stat 189a90e08c64c5693fd3a8f6c8967106f02721b5 8f9904a0a55516582e101d7b75b9457adaf9a0be
# (empty)
```

The merge commit's tree is byte-identical to the CI-verified PR HEAD tree,
because the merge's first parent `06813e3b` was still the tip of `main` at
merge time (no intervening commits). Therefore the green CI run at
`189a90e0` — `build` pass (5m8s) and `start-launcher-windows` pass (2m13s) —
applies unchanged to `main`. A redundant full rebuild would verify the same
tree and was deliberately not run.

## Merged Code Presence

| Check | Result |
|---|---|
| `src/shim/preinit_compat.rs` | present at closure HEAD (327 lines added by `6b7e6076`) |
| `src/shim/transport.rs` | compat wiring present (24 lines changed) |
| `ENGRAM_MCP_PREINIT_COMPAT` kill switch | resolves in `preinit_compat.rs` (lines 38, 50) and `transport.rs` (line 242) |

Feature-branch commits `e25cf538` (RED contract test), `6b7e6076`
(implementation), `314652c3` (end-to-end probe-then-handshake integration),
and `3d90d5a7` (compat window + rollback runbook docs) are all reachable from
`8f9904a0`.

## Engram Daemon Diagnostics — DEGRADED

Per the worktree-safe Engram diagnostics protocol, one bounded daemon-health
probe was attempted from the closure worktree:

| Field | Value |
|---|---|
| Worktree | `C:/Source/GitHub/engram/.worktrees/post-merge-124-s-closure-20260823` |
| Branch | `post-merge/124-s-closure` |
| Command | `engram daemon-status --timeout 30` |
| Binary | `C:\Tools\engram.exe` |
| Elapsed before bounding | >150 s with no output |
| Outcome | **DEGRADED** — probe bounded and abandoned, not retried |

The probe produced no output well beyond its own 30-second timeout argument.
Consistent with the documented protocol, Engram was declared degraded and
closure continued using the direct-read fallback (Git plumbing, `gh` API, and
backlogit CLI). No daemon PID was killed, no runtime state was removed, and no
workspace rebind was attempted — daemon lifecycle is not owned by Ship.

### Corroborating evidence for spike `002-SP`

This degraded observation independently corroborates the queued spike
`002-SP` ("Profile Cozo cold start: 135 MB database takes ~7.5 minutes to
reach ready"). A `--timeout 30` client argument did not bound the observed
wait, which suggests the timeout governs a post-readiness request phase rather
than the expensive Cozo open/schema-bootstrap path. That is a useful narrowing
hint for whoever claims `002-SP`.

This evidence is recorded here rather than folded into 124-S. The Cozo
cold-start defect remains a separate, unshipped work item; `002-SP` was not
modified, claimed, or archived by this closure.

## Verdict

**PASS (with Engram degraded).** The shipped compatibility change is present
on `main` and covered by green CI against a byte-identical tree. The degraded
Engram daemon probe reflects the pre-existing, separately-tracked Cozo
cold-start defect and is not a regression introduced by 124-S — the merged
change explicitly does not alter daemon readiness timeouts.
