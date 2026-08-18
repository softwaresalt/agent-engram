---
title: "Shipment 117-S final runtime and closure memory"
date: 2026-08-18
doc_type: memory
shipment_id: "117-S"
tasks:
  - "121.015-T"
  - "121.016-T"
status: completed
evaluated_implementation_head: "10e7533ffccba0a432d67a3d3ec522f4e3c0e58b"
evaluated_tree: "f99e02c565d2d5f97ff8448a95bb2b5974e3e51e"
---

## Outcome

Final scoped runtime verification passed. Pre-merge operational closure is
**READY WITH CONDITIONS** because current-HEAD CI, current-HEAD Copilot review,
and merge-commit settings remain pending after the documentation commit.

No source, test, Cargo, backlog, stash, plan, or PR state changed.

## Files created

* `docs/closure/2026-08-18-hcl-family-parser-runtime-verification.md`
* `docs/closure/2026-08-18-hcl-family-parser-operational-closure.md`
* `docs/memory/2026-08-18/117-s-final-runtime-memory.md`

## Runtime authority

| Field | Value |
|---|---|
| Branch | `feat/117-s-shared-hcl-parser` |
| Implementation HEAD | `10e7533ffccba0a432d67a3d3ec522f4e3c0e58b` |
| Implementation tree | `f99e02c565d2d5f97ff8448a95bb2b5974e3e51e` |
| Binary version | `engram 0.2.0+g10e7533f` |
| Binary SHA-256 | `5048626D26A7EB35CA90142F3264C4E3FA96D5A143513680BEB5F7CA1A4EB77D` |
| OS | Windows 11 Enterprise build 26200, 64-bit |

## Evidence summary

* Cold default startup: 3 HCL-family files, 6 symbols, 6 Defines edges
* Cold forced index: 3 parsed, 6 symbols, 12 extraction edges, no errors
* Explicit sync cycle 1: 7 edges, no errors, `web_v2` replaced `web`
* Explicit sync cycle 2: 1 modified, 2 unchanged, 3 edges, no errors
* Post-cycle restart: stable 3 files, 6 symbols, same workspace identity
* Live route: create visible within 5 seconds; modify within 3 seconds
* Watcher-only persisted route: nodes changed without an intervening CLI graph
  request; stale name count zero
* Final repeated force baseline: 6 parsed, 8 symbols, 18 extraction edges
* Final status: 6 files, 8 symbols, 8 Defines edges
* HCL security target: 6 passed, 0 failed
* Scoped source-race unit tests: 29 passed, 0 failed
* Windows static file and directory symbolic links: zero linked symbols
* Rollback config exclusion: 0 files, 0 symbols, 0 edges after force/restart
* Audit: `h2 0.4.16`; `RUSTSEC-2026-0258` absent; 14 allowed warnings

Unix-only discovery-root tests remain compiled out on Windows and retained for
CI:

* `final_workspace_replacement_cannot_redirect_discovery_or_deletion`
* `workspace_ancestor_replacement_cannot_redirect_discovery_or_deletion`

## Review and scope decisions

* Scoped security reviews: PASS, zero P0 and zero P1
* Adversarial adjudication: HIGH-confidence PASS after `cap-std`
  open-handle evidence
* Accepted containment residuals: hardlinks, mounts, and in-place mutation only
* Stage correction removed exactly seven file-tracker, hydration, and
  retrieval-evaluation tests from `117-S`
* The seven deferred tests remain preserved under `EE8C4E35`
* Follow-up stash IDs remain `EE8C4E35`, `122F86F2`, `7F71CB40`,
  `A4E72E5D`, `80BBDFA3`, `0F833F6A`, and `4F3E2EC3`
* No follow-up was harvested or mutated

## Retained fixtures

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417-outside
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417-rollback-workspace
```

## Process cleanup

Owned PIDs were `7420`, `25964`, `26264`, `32344`, `15968`, `26808`, and
`23768`. Each was stopped by exact PID. Each exact post-stop lookup returned
no process. The final combined lookup returned none.

## Failed approach and correction

The first daemon startup exited `2` because the newly created fixture was not
a Git repository root. It created no daemon. Running contained `git init`
supplied the required workspace prerequisite; the unchanged startup then
passed.

The rollback copy materialized the two static links as regular copied content,
so its baseline became seven HCL files and nine symbols. The forced
excluded-language reconciliation still reached exact zero state, which is a
stronger rollback result.

No implementation failure or source fix occurred.

## Strict-safety result

Authorized verification and documentation actions are `ActionResult:
applied`. Merge, push, PR mutation, backlog mutation, and stash mutation were
not authorized and were not attempted.

## Next steps

1. Push only through the authorized PR lifecycle.
2. Require CI and Copilot review for the new current HEAD.
3. Recheck zero unresolved threads and clean mergeable state.
4. Verify merge-commit-only repository settings.
5. Merge only with a merge commit after operator approval.
6. Run the 30-minute post-merge observation window from operational closure.
