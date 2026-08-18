---
title: "HCL family parser final scoped operational closure"
date: 2026-08-18
doc_type: operational-closure
mode: pre-merge
readiness: READY_WITH_CONDITIONS
shipment_id: "117-S"
feature_id: "121-F"
task_id: "121.016-T"
branch: "feat/117-s-shared-hcl-parser"
evaluated_implementation_head: "10e7533ffccba0a432d67a3d3ec522f4e3c0e58b"
evaluated_tree: "f99e02c565d2d5f97ff8448a95bb2b5974e3e51e"
runtime_report: "docs/closure/2026-08-18-hcl-family-parser-runtime-verification.md"
runtime_verdict: PASS
---

## Readiness decision

**READY WITH CONDITIONS.** Shipment `117-S` passed final scoped runtime
verification at implementation HEAD
`10e7533ffccba0a432d67a3d3ec522f4e3c0e58b`, tree
`f99e02c565d2d5f97ff8448a95bb2b5974e3e51e`. The former P1
source-read replacement race is closed at the scoped code-graph reader
boundary by capability-rooted, handle-preserving opens and capability-rooted
enumeration.

The implementation is operationally ready for the merge gate. Merge is not
authorized by this artifact: the documentation commit changes branch HEAD, so
final current-HEAD CI and Copilot review must complete, and merge-commit-only
settings must be verified. This session did not push, mutate PR state, or
touch backlog or stash state.

## Closure inputs

* [Final runtime verification](2026-08-18-hcl-family-parser-runtime-verification.md)
* [Post-boundary Stage triage](../decisions/2026-08-18-117-s-post-boundary-commit-triage.md)
* [Historical P1 security review](2026-08-16-hcl-source-read-toctou-security-review.md)
* Evaluated implementation HEAD `10e7533ffccba0a432d67a3d3ec522f4e3c0e58b`
* Evaluated implementation tree `f99e02c565d2d5f97ff8448a95bb2b5974e3e51e`
* Runtime binary SHA-256
  `5048626D26A7EB35CA90142F3264C4E3FA96D5A143513680BEB5F7CA1A4EB77D`

## Final review disposition

The supplied final scoped review handoff and runtime evidence produce this
gate:

| Review | Verdict | P0 | P1 | Disposition |
|---|---|---:|---:|---|
| Scoped capability-reader security review | PASS | 0 | 0 | No blocker |
| Scoped discovery/publication security review | PASS | 0 | 0 | No blocker |
| Adversarial adjudication | HIGH-confidence PASS | 0 | 0 | Consensus after handle evidence |

The adversarial adjudication moved to **HIGH PASS** after confirming the
`cap-std` directory and file handles remain the authority across relative
open, metadata, and read. The reader does not validate a pathname and then
reopen it ambiently. Deterministic final-file and ancestor replacement tests
passed on Windows, and the capability-enumeration test proved discovery
classifies links without following them.

No P0 or P1 finding remains. The only accepted containment residuals are:

* Hardlinks
* Mount points
* In-place mutation of an already-open regular file

No other security residual is accepted by this closure.

## Stage scope correction

Stage commit `b65cf13bbda85bb88e2228cec27db7e830cce4ea` corrected the
pre-boundary interpretation. Commit
`eddb47580d5212ddf19f233fd01285f759eeb049` removed exactly seven
cross-surface tests from shipment `117-S`; their evidence remains preserved
under stash `EE8C4E35` for lifecycle source-read migration:

| Surface | Deferred test |
|---|---|
| File tracker | `final_replacement_after_collection_is_not_hashed` |
| File tracker | `ancestor_replacement_after_collection_is_not_hashed` |
| Hydration | `final_replacement_persists_degraded_function_body` |
| Hydration | `ancestor_replacement_persists_degraded_class_body` |
| Hydration | `indexed_hash_mismatch_persists_degraded_body` |
| Retrieval evaluation | `eval_final_replacement_is_unreadable_without_parsing` |
| Retrieval evaluation | `eval_ancestor_replacement_is_unreadable_without_parsing` |

Their production fixes begin in excluded post-boundary surfaces. This closure
does not claim them, reintroduce them, or widen the scoped code-graph boundary.

The Stage follow-up stash IDs remain:

| Stash ID | Scope | Next action |
|---|---|---|
| `EE8C4E35` | Lifecycle source-read migration boundaries | `deliberate` |
| `122F86F2` | Dedicated content indexer capability reads | `deliberate` |
| `7F71CB40` | Config, registry, and lifecycle metadata reads | `deliberate` |
| `A4E72E5D` | Capability-rooted mutable artifact writes | `spike` |
| `80BBDFA3` | Daemon PID, lock, socket, log, and runtime authority | `spike` |
| `0F833F6A` | Authenticated daemon IPC endpoint identity and lifecycle | `deliberate` |
| `4F3E2EC3` | Metrics read and channel capacity invariants | `deliberate` |

All remain unharvested. No stash or backlog mutation occurred.

## Dependency and advisory closure

Implementation HEAD resolves:

```text
h2 v0.4.16
```

Commit `10e7533ffccba0a432d67a3d3ec522f4e3c0e58b` patches the
empty-frame denial of service tracked as `RUSTSEC-2026-0258`.

Command:

```powershell
cargo audit --no-fetch --stale --file "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\Cargo.lock"
```

Observed: exit `0`; 565 dependencies scanned; 14 allowed warnings;
`RUSTSEC-2026-0258` absent. The allowed warnings are existing audit-policy
output and do not reopen the scoped HCL containment gate.

## Strict-safety record

| ProposedAction | ActionRisk | Approval | ActionResult |
|---|---|---|---|
| Exercise the exact branch binary and retained fixtures | Moderate | Explicit operator authorization | Applied |
| Start and stop isolated daemons | Moderate | Explicit exact-PID authorization | Applied |
| Validate capability-rooted replacement containment | High security relevance; non-destructive | Explicit scoped verification request | Applied |
| Create final runtime and closure records | Moderate | Explicitly authorized paths only | Applied |
| Merge, push, or mutate PR/backlog state | High or external | Not authorized | Blocked and not attempted |

Overall `ActionResult`: **applied** for the authorized verification and
documentation scope.

## Invariants to preserve

* Exact `tree-sitter-hcl = 1.1.0` registry provenance and checksum
* One canonical stored language, `hcl`, for lowercase `.hcl`, `.tf`, and
  `.tfvars`
* Structural symbols remain in `hcl.block.*` and `hcl.attribute.*`
* Traversals remain deterministic persistence-only hints on file self-loops
* HCL never enters workspace-global name resolution
* Startup, explicit sync, live routing, and restart remain equivalent
* Ignore, size, case, malformed-input, and link controls precede publication
* Discovery and source reads remain capability-rooted and no-follow
* Rejected final-file or ancestor replacement never publishes external bytes
* Repeated sync and restart leave no stale or duplicate symbol
* No unsafe code or ambient pathname reopen is introduced

## Pre-deploy audit

| Check | Result |
|---|---|
| Runtime verification | PASS at exact implementation HEAD/tree and binary SHA |
| Scoped security reviews | PASS; zero P0 and zero P1 |
| Adversarial review | HIGH-confidence PASS after `cap-std` handle evidence |
| Schema or migration | None; existing graph records are reused |
| Default rollout | HCL remains intentionally default-enabled |
| Mitigation control | Explicit `supported_languages` exclusion rehearsed |
| Rollback | Forced reconciliation reached zero HCL state |
| Dependency advisory | `h2 0.4.16`; `RUSTSEC-2026-0258` absent from audit |
| Cross-service dependency | None; local daemon and embedded state only |
| Stage scope | Seven cross-surface tests deferred under `EE8C4E35` |
| Monitoring | Manual plan below has baselines, thresholds, owner, and window |
| Current-HEAD gate | Pending after this documentation commit |
| Merge strategy | Merge commit required; repository setting recheck pending |

No feature flag, database migration, hosted canary, or maintenance window is
required. The release path is merge commit, normal release publication, and
local daemon uptake.

## Monitoring plan

No hosted dashboard exists for this local-first daemon. The release operator
must run the following manual plan on the retained fixture and the first real
HCL workspace.

| SLI | Query or evidence | Baseline | Alert or rollback threshold | Owner |
|---|---|---|---|---|
| Valid HCL index errors | Forced-index JSON `errors` | `[]` | Any valid lowercase HCL error | Release operator |
| Graph cardinality | Forced index | 6 files, 8 symbols, 18 extraction edges | Unplanned drift on repeat |
| Persisted graph | `workspace-status` | 6 files, 8 symbols, 8 Defines edges | Duplicate or stale symbol |
| Daemon health | `daemon-status` | Overall green after activity | Red check, crash, or IPC failure |
| Live route latency | Create/modify `.tf` and inspect persisted nodes | Create within 5 seconds; modify within 3 seconds | Replacement absent after 10 seconds |
| Hint-only boundary | Focused HCL security target | HCL self-loop and hint; SQL still resolves | HCL global binding or missing hint |
| Static containment | Linked file and directory queries | Zero linked symbols | Any linked external symbol |
| Replacement containment | Scoped deterministic race tests | 29 Windows tests pass | Any external sentinel or LKG drift |
| Rollback | Excluded-language status | 0 files, symbols, and edges | Any residual HCL state |

Healthy means all baselines hold with no valid-input error, containment breach,
or stale graph state. Silence is not a healthy result.

## Post-merge observation window

Owner: **release operator**.

Duration: **30 minutes**, beginning when the merged or released binary first
indexes the retained fixture or the first real HCL workspace.

Required checkpoints:

1. At startup, record daemon identity, health, index errors, and graph counts.
2. Within 10 minutes, perform one create and modify cycle.
3. Run static link and one focused replacement containment check.
4. At 30 minutes, repeat health, forced counts, and hint-only persistence.
5. Record `healthy`, `degraded`, or `rolled back`.

## Failure signals and rollback triggers

Rollback or immediate mitigation begins if any of these occur:

* Valid lowercase HCL produces an index error, daemon crash, or IPC failure
* Canonical `hcl` identity or the namespaced symbol contract changes
* HCL traversal hints disappear or bind globally
* Repeated sync or restart leaves a stale or duplicate symbol
* Ignored, oversize, uppercase, or linked content enters the graph
* Final-file or ancestor replacement publishes an external sentinel
* Live replacement is absent after 10 seconds
* `RUSTSEC-2026-0258` reappears or `h2` resolves below `0.4.16`

## Rollback procedure

1. Obtain explicit operator approval.
2. Exclude `hcl` from `code_graph.supported_languages` in the affected
   workspace.
3. Restart only the affected daemon through the normal lifecycle.
4. Run a forced full reconciliation.
5. Require zero HCL code files, symbols, and edges with `errors=[]`.
6. If release code must be reverted, create a normal revert of the merge
   commit with `git revert -m 1`; never reset, rebase, or force-push.
7. If any HCL record remains, keep rollback blocked and request separately
   approved remediation. Do not delete the embedded database automatically.

The isolated rehearsal proved steps 2 through 5. No production rollback or
history operation ran.

## PR and merge conditions

The following gates remain pending because this closure commit changes branch
HEAD and the operator prohibited PR interaction:

1. Required CI succeeds for the new current HEAD.
2. Copilot review exists with `commit_id` exactly equal to that HEAD.
3. Copilot is absent from `requested_reviewers`.
4. Zero unresolved review threads remain.
5. GitHub reports `mergeable_state == clean`.
6. Repository settings permit merge commits and disable squash and rebase
   merge.
7. The PR is merged only with a merge commit.

These are conditions, not failures in the evaluated implementation. No push,
review request, PR comment, thread mutation, backlog transition, or merge was
performed.

## Final handoff

* Operational readiness: **READY WITH CONDITIONS**
* Runtime verification: **PASS**
* Scoped security review: **PASS**, no P0 or P1
* Adversarial adjudication: **HIGH PASS**
* Dependency advisory: `h2 0.4.16`, `RUSTSEC-2026-0258` closed
* Accepted residuals: hardlinks, mounts, and in-place mutation only
* Monitoring owner and window: release operator, 30 minutes
* Rollback gate: forced reconciliation to zero HCL state
* ActionResult: **applied**
* Remaining work: current-HEAD CI/Copilot gate and merge-commit-only gate
