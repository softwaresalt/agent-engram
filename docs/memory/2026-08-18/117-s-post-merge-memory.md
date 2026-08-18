---
title: "Shipment 117-S post-merge closure memory"
date: 2026-08-18
doc_type: memory
shipment_id: "117-S"
feature_id: "121-F"
mode: post-merge
status: completed
verdict: READY_CLOSED
merge_commit_sha: "c879d7196af7fb90b950560d120ae1b00baa90ec"
---

## Outcome

Shipment `117-S` and feature `121-F` are **READY / CLOSED**. PR `#342` merged
at `2026-08-18T18:35:07Z` through two-parent merge commit
`c879d7196af7fb90b950560d120ae1b00baa90ec`, which is the exact local
`HEAD` and `origin/main`.

The bounded post-merge HCL smoke passed. No source, test, Cargo, backlog,
stash, plan, decision, compound, or other-shipment file was modified by this
session. No commit or push occurred.

## Files created

* `docs/closure/2026-08-18-hcl-family-parser-post-merge-closure.md`
* `docs/memory/2026-08-18/117-s-post-merge-memory.md`

## Merge and archive evidence

* Final PR head:
  `434bdf098dd31fe722630fe86e971a8f43fed97e`
* Final Ubuntu `build` job: success on `ubuntu-latest`
* Exact-head Copilot review: `COMMENTED` at `2026-08-18T18:28:13Z`
* Review threads: eight total, zero unresolved
* Final supplied gate: Copilot unrequested and mergeable clean
* Shipment archive: shipped, commit field matches merge SHA
* Manifest archives: 21 of 21 matched
* Reconciliation: pre `PROCEED`; post `RECONCILE_PASS` and `PROCEED`
* Archive verification: 22 files, zero archive deletions

**No other shipment was selected.**

No other shipment was modified, reconciled, archived, closed, or otherwise
acted on.

## Released behavior

The release adds canonical lowercase `.hcl`, `.tf`, and `.tfvars` parsing,
namespaced structural symbols, hint-only traversal persistence, startup and
live routing, and capability-rooted no-follow discovery and reads. The scoped
reader keeps one capability and opened handles authoritative across relative
open, metadata, and read; it does not validate then reopen by ambient path.

Accepted containment residuals remain hardlinks, mount points, and in-place
mutation of an already-open file. Broader lifecycle and runtime capability
work remains outside `117-S`.

## Scope and stash continuity

Stage correction `b65cf13b` and test-scope commit `eddb4758` removed seven
file-tracker, hydration, and retrieval-evaluation harnesses inherited from
mixed commit `35254714`. They remain future acceptance evidence under
`EE8C4E35`.

Initial stowaway commit `aa14af6e` added exactly seven entries, with no
unpublished eighth:

```text
0B729BFE 1328405A 4D08C3D9 60A58C8D AA96FC45 B82ABA6E C64FD73F
```

Stage scope commit `d8c9546a` added exactly seven more:

```text
EE8C4E35 122F86F2 7F71CB40 A4E72E5D 80BBDFA3 0F833F6A 4F3E2EC3
```

The PR therefore added all 14 IDs and removed none. None was harvested or
mutated during closure.

## Advisory state

Merged commit `10e7533f` resolves `h2 v0.4.16` and closes
`RUSTSEC-2026-0258`. Pre-merge audit passed with that advisory absent and 14
policy-allowed warnings. Post-merge `cargo tree --locked --invert h2 --depth
0` again reported `h2 v0.4.16`.

## Smoke evidence

Retained fixture:

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-post-merge-smoke-20260818-1150
```

Evidence:

* `HEAD == origin/main == c879d7196af7fb90b950560d120ae1b00baa90ec`
* Source-surface diff from `origin/main`: empty
* Locked merged build: exit `0`, 19.43 seconds
* Binary: `engram 0.2.0+gc879d719-dirty`
* Binary SHA-256:
  `282C04148EAF3ED272155478D1086AD098ADC1A73B1CC82F8BABF1E1D648A6FC`
* Direct index: one file, two classes, three edges, `errors=[]`
* Symbols: `hcl.block.locals` and
  `hcl.block.resource.null_resource.post_merge`
* Daemon health: overall green
* Owned PID: `11400`, stopped by exact PID and confirmed absent

The first symbol help probe used the obsolete subcommand `list-symbols`,
exited `2`, and suggested `symbols`. The corrected command passed. This was a
CLI syntax correction; it did not change source or invalidate the smoke.

## Monitoring and rollback

The release-operator window is `2026-08-18T18:54:22Z` through
`2026-08-18T19:24:22Z`. Initial status is **HEALTHY / ACTIVE**. Rollback is
triggered by valid-HCL errors, daemon or IPC failure, canonical-identity drift,
stale graph state, containment failure, or advisory recurrence.

After approval, exclude `hcl`, restart only the affected daemon, force
reconciliation, and require exact `0/0/0` HCL files/symbols/edges with no
errors. Revert the merge normally with `git revert -m 1` only if code reversal
is required. Never reset, rebase, force-push, or delete the database
automatically.

## Safety and handoff

Recovery ref `safety/117-s-scope-expansion-2f528aff` remains local and
unpushed at `2f528aff6b6f05c0a88a66349f03f3e421c4c2eb`. The retained fixture
remains ignored for reproduction. No owned process remains.

Next step: the release operator completes the active monitoring window and
records its final `healthy`, `degraded`, or `rolled back` outcome.
