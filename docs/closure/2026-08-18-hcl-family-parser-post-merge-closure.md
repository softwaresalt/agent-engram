---
title: "HCL family parser post-merge operational closure"
date: 2026-08-18
doc_type: operational-closure
mode: post-merge
status: CLOSED
readiness: READY
shipment_id: "117-S"
feature_id: "121-F"
branch: "post-merge/117-s-hcl-parser-closure"
pr_number: 342
merge_commit_sha: "c879d7196af7fb90b950560d120ae1b00baa90ec"
runtime_verdict: PASS
monitoring_status: HEALTHY_COMPLETE
---

## Closure decision

**READY / CLOSED.** The bounded post-merge smoke passed on the merged source
tree. `HEAD` and `origin/main` both resolved to merge commit
`c879d7196af7fb90b950560d120ae1b00baa90ec`, and the source, tests, manifest,
and lockfile had no diff from that commit. The retained fixture indexed
canonical HCL symbols with no errors. The only daemon started by the smoke was
stopped by exact PID and confirmed absent.

This post-merge record closes shipment `117-S` and feature `121-F`. The full
30-minute release-operator observation window elapsed, and its final
create/modify, containment, forced-index, health, process, and advisory
checkpoints completed healthy.

## Closure inputs

* [Pre-merge runtime verification](2026-08-18-hcl-family-parser-runtime-verification.md)
* [Pre-merge operational closure](2026-08-18-hcl-family-parser-operational-closure.md)
* [Post-boundary Stage triage](../decisions/2026-08-18-117-s-post-boundary-commit-triage.md)
* [Pre-ship reconciliation](../../.backlogit/reconcile/117-S-pre-20260818-183854.md)
* [Post-ship reconciliation](../../.backlogit/reconcile/117-S-post-20260818-184835.md)
* [Archived shipment](../../.backlogit/archive/117-S.md)
* [Pull request 342](https://github.com/softwaresalt/agent-engram/pull/342)

## Merge, CI, and review evidence

| Gate | Evidence | Result |
|---|---|---|
| Pull request | PR `#342`, final head `434bdf098dd31fe722630fe86e971a8f43fed97e` | Merged |
| Merge | `2026-08-18T18:35:07Z`, SHA `c879d7196af7fb90b950560d120ae1b00baa90ec` | Verified in `origin/main` |
| Merge strategy | Two parents: `6268c1ac77db64deb5ffe7af820735dbe172624f` and `434bdf098dd31fe722630fe86e971a8f43fed97e` | Merge commit |
| Ubuntu CI | `build`, `ubuntu-latest`, `2026-08-18T18:22:14Z` to `18:32:32Z` | Success |
| Copilot review | `COMMENTED` on exact final head at `2026-08-18T18:28:13Z` | Exact-head review |
| Review threads | Eight total, zero unresolved | Clean |
| Final merge gate | Copilot unrequested and `mergeable_state == clean` before merge | Operator-confirmed |

GitHub independently reported the merged PR head, merge SHA, merge timestamp,
successful Ubuntu job, exact-head Copilot review, and zero unresolved threads.
The merge commit subject is
`Merge pull request #342 from softwaresalt/feat/117-s-shared-hcl-parser`.

## Shipment and reconciliation state

Shipment `117-S` is archived with `archived_status: shipped`. Feature `121-F`
and all 20 task or subtask members are archived, for 21 manifest members plus
the shipment archive.

The pre-ship reconciliation reported `PROCEED`: all 21 members were validly
pre-archived, with zero missing items, status mismatches, or orphans. The
post-ship reconciliation reported `RECONCILE_PASS` and `PROCEED`: 21 of 21
members matched, 22 archive files were verified, and no archive deletion was
present. The archived shipment commit field matches the merge SHA.

**No other shipment was selected.**

No other shipment was modified, reconciled, archived, closed, or otherwise
acted on.

## Released functionality

The merged release provides:

* Canonical parser support for lowercase `.hcl`, `.tf`, and `.tfvars`
* One stored `hcl` language identity and namespaced `hcl.block.*` and
  `hcl.attribute.*` structural symbols
* Deterministic traversal hints that remain persistence-only file self-loops
  and never enter workspace-global name resolution
* Default startup indexing, explicit synchronization, live watcher routing,
  restart persistence, ignore handling, size limits, and malformed-input bounds
* Capability-rooted discovery and source reads that fail closed on final-file
  and ancestor replacement before graph publication
* Configuration exclusion followed by forced reconciliation to zero HCL state

The pre-merge runtime authority was implementation
`10e7533ffccba0a432d67a3d3ec522f4e3c0e58b`, tree
`f99e02c565d2d5f97ff8448a95bb2b5974e3e51e`. It established cold
`3/6/12`, steady `6/8/18`, and rollback `0/0/0` file/symbol/edge baselines.
The final PR head retained that implementation and passed its final Ubuntu CI
and exact-head review gates.

## Scoped capability reader and residuals

The shipped code-graph source boundary follows this authority-preserving
sequence:

1. Open the canonical workspace directory as a capability.
2. Accept only validated relative candidate paths.
3. Open components relative to that capability with semantic no-follow
   behavior and no root escape.
4. Derive regular-file identity, metadata, and size from the opened handle.
5. Read from the same handle without an ambient pathname reopen.
6. Use the shared boundary for discovery, full index, sync, prepass, postpass,
   and publication.

Capability-rooted enumeration classifies entries without following links.
Deterministic final-file and ancestor barriers proved that external bytes are
not published and that last-known-good state is retained where required.

The accepted residuals remain exactly hardlinks, mount points, and in-place
mutation of an already-open regular file. The two discovery-root replacement
tests are Unix-only and were compiled out of the Windows pre-merge run; the
final Ubuntu CI remained green. Broader lifecycle readers, dedicated indexers,
metadata reads, mutable writes, and daemon runtime authority are explicitly
outside this shipment and remain in Stage follow-up scope.

## Test-scope correction

Stage commit `b65cf13bbda85bb88e2228cec27db7e830cce4ea` corrected the
pre-boundary interpretation. Commit
`eddb47580d5212ddf19f233fd01285f759eeb049` then removed exactly seven
cross-surface tests inherited from mixed commit
`352547142f937edbd43a203a01832e31f0b80308`:

* Two file-tracker replacement-race tests
* Three hydration replacement or hash-race tests
* Two retrieval-evaluation replacement-race tests

Those harnesses are future acceptance evidence under `EE8C4E35`. Their
production fixes begin in excluded lifecycle surfaces; shipment `117-S` did
not claim or implement them.

## Stowaway finalization

Commit `aa14af6ec4d47846c094feb6ea7a1b1e3a17b8dd` added the initial
capability-review stowaway batch. The published batch contains exactly seven
stash entries; there is no unpublished eighth entry:

| Stash ID | Scope |
|---|---|
| `0B729BFE` | Jupyter cell code-graph identities and parity |
| `1328405A` | Real-repository retrieval evaluation and measured reranking |
| `4D08C3D9` | Python call-graph v1 semantic expansion |
| `60A58C8D` | Static Spark notebook lineage increments |
| `AA96FC45` | Post-HCL Terraform semantic increments |
| `B82ABA6E` | Deeper Power BI and DAX lineage |
| `C64FD73F` | Trusted Spark kernel execution provenance |

Stage commit `d8c9546af96fddca17a5aca33cc362bf8d84ac9d` later added exactly
seven scope-expansion follow-ups:

| Stash ID | Scope | Required next action |
|---|---|---|
| `EE8C4E35` | Lifecycle source-read migration boundaries | `deliberate` |
| `122F86F2` | Dedicated content indexer capability reads | `deliberate` |
| `7F71CB40` | Config, registry, and lifecycle metadata reads | `deliberate` |
| `A4E72E5D` | Capability-rooted mutable artifact writes | `spike` |
| `80BBDFA3` | Daemon PID, lock, socket, log, and runtime authority | `spike` |
| `0F833F6A` | Authenticated daemon IPC endpoint identity and lifecycle | `deliberate` |
| `4F3E2EC3` | Metrics read and channel-capacity invariants | `deliberate` |

The final PR adds exactly these 14 stash IDs relative to its mainline parent
and removes none. This closure does not harvest or mutate any of them.

## Dependency advisory remediation

Merged commit `10e7533ffccba0a432d67a3d3ec522f4e3c0e58b` updates the lockfile
to `h2 v0.4.16`, closing the empty-frame denial of service tracked as
`RUSTSEC-2026-0258`. The pre-merge audit exited `0`, scanned 565
dependencies, retained 14 policy-allowed warnings, and did not report that
advisory. The post-merge command `cargo tree --locked --invert h2 --depth 0`
again resolved exactly `h2 v0.4.16`.

## Post-merge smoke

### Environment and authority

The retained fixture is:

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-post-merge-smoke-20260818-1150
```

It is a contained Git workspace and is ignored by the parent repository. It
contains one `infra/main.tf` source with a `null_resource.post_merge` block and
a `locals` block. No source, test, Cargo, backlog, stash, plan, decision, or
compound file was changed by the smoke.

Source authority command:

```powershell
git -C "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618" rev-parse HEAD origin/main
```

Observed: exit `0`; both lines were
`c879d7196af7fb90b950560d120ae1b00baa90ec`.

Source-diff command:

```powershell
git -C "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618" diff --quiet HEAD origin/main -- src tests Cargo.toml Cargo.lock
```

Observed: exit `0`; no source-surface difference.

Build command:

```powershell
cargo build --locked --manifest-path "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\Cargo.toml" --target-dir "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target"
```

Observed: exit `0`; merged source compiled in 19.43 seconds. The binary
reported `engram 0.2.0+gc879d719-dirty`; the dirty suffix reflects retained
post-merge backlog reconciliation state, while the source-surface diff above
was empty. Its SHA-256 was
`282C04148EAF3ED272155478D1086AD098ADC1A73B1CC82F8BABF1E1D648A6FC`.

### Index and symbol evidence

Direct index command:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" index --force --direct --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-post-merge-smoke-20260818-1150" --timeout 60 --format json
```

Observed: exit `0`; one file parsed, two classes indexed, three extraction
edges created, zero skipped or oversized files, and `errors=[]`.

Canonical symbol command:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" symbols --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-post-merge-smoke-20260818-1150" --prefix "hcl." --limit 10 --format json
```

Observed: exit `0`; exactly two symbols:

```text
hcl.block.locals
hcl.block.resource.null_resource.post_merge
```

Daemon health command:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" daemon-status --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-post-merge-smoke-20260818-1150" --format json
```

Observed: exit `0`; overall green, the same fixture identity, no offline
changes, and PID `11400`.

### Process closure

Stop command:

```powershell
Stop-Process -Id 11400
```

Observed: exit `0`.

Absence command:

```powershell
Get-Process -Id 11400
```

Observed: expected exit `1`, `Cannot find a process with the process
identifier 11400`. No owned PID remains.

The retained fixture remains available for reproduction. No cleanup deletion
was performed.

## Strict-safety record

| ProposedAction | ActionRisk | Approval | ActionResult |
|---|---|---|---|
| Build merged source into the ignored target directory | Moderate | Explicit post-merge request | Applied |
| Create and index one retained, ignored fixture | Moderate | Explicit contained-fixture request | Applied |
| Start and stop only the fixture daemon | Moderate | Explicit exact-PID instruction | Applied |
| Create the two authorized closure artifacts | Moderate | Explicit paths | Applied |
| Edit source, backlog, stash, plans, decisions, compound, or another shipment | High or out of scope | Not authorized | Abandoned without attempt |
| Commit or push | External mutation | Prohibited | Abandoned without attempt |

## Monitoring plan and status

No hosted dashboard exists for this local-first daemon. The release operator
owns the manual observation window.

| SLI | Baseline | Alert or rollback threshold | Current status |
|---|---|---|---|
| Valid HCL index | One file, two symbols, three edges, `errors=[]` | Any valid lowercase HCL error | Healthy |
| Canonical identity | Two expected `hcl.block.*` symbols | Missing, duplicate, or noncanonical identity | Healthy |
| Daemon health | Overall green | Red check, crash, or IPC failure | Healthy |
| Advisory state | `h2 v0.4.16` | Version below `0.4.16` or advisory returns | Healthy |
| Process ownership | Owned PID absent after stop | Owned PID remains | Healthy |

The post-merge window opened at `2026-08-18T18:54:22Z`, when the merged daemon
indexed the fixture, and ran through `2026-08-18T19:24:22Z`. The final
checkpoint began at `2026-08-18T19:25:08Z` and completed **HEALTHY**.

Final checkpoint evidence:

* Appended a valid HCL comment to `infra/main.tf`
* Created `infra/observation.tfvars`
* Added final-file and directory symlinks to contained external sentinel
  fixtures
* Explicit sync: one file modified, one added, two edges created,
  `errors=[]`
* Canonical symbols: exactly three expected in-workspace HCL symbols; no
  external sentinel symbol
* Forced index: two real files, three classes, five edges, `errors=[]`
* Static linked file and linked directory remained excluded
* Daemon health: overall green, no offline changes, IPC reachable
* Owned PID `11680` stopped by exact PID
* `cargo tree --locked --invert h2 --depth 0` resolved `h2 v0.4.16`

## Rollback trigger and procedure

Rollback or immediate mitigation begins on any valid-HCL index error, daemon
crash, canonical-identity drift, global HCL reference binding, stale or
duplicate graph state, containment breach, or reappearance of
`RUSTSEC-2026-0258`.

After explicit operator approval:

1. Exclude `hcl` from `code_graph.supported_languages` for the affected
   workspace.
2. Restart only the affected daemon through the normal lifecycle.
3. Run a forced full reconciliation.
4. Require zero HCL files, symbols, and edges with `errors=[]`.
5. If code reversal is required, normally revert merge commit
   `c879d7196af7fb90b950560d120ae1b00baa90ec` with `git revert -m 1`.
6. Never reset, rebase, force-push, or automatically delete the embedded
   database.

The pre-merge rehearsal proved the configuration-exclusion gate at exact
`0/0/0`.

## Recovery safety ref

Recovery branch `safety/117-s-scope-expansion-2f528aff` remains local at
`2f528aff6b6f05c0a88a66349f03f3e421c4c2eb`. A direct
`ls-remote --heads origin` query returned no matching remote ref. It was not
deleted, committed, or pushed during closure.

## Final status

* Runtime verification: **PASS**
* Operational readiness: **READY**
* Shipment status: **CLOSED / archived as shipped**
* Post-merge smoke: **PASS**
* Monitoring: **HEALTHY / COMPLETE**
* Owned processes: **none**
* Retained fixtures: **one**
* Commit or push: **none**
