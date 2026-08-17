---
title: "HCL family parser operational closure"
doc_type: closure
date: 2026-08-16
mode: pre-merge
readiness: READY
source: "docs/exec-plans/2026-08-15-hcl-family-parser-plan.md"
shipment_id: "117-S"
feature_id: "121-F"
task_id: "121.016-T"
subtask_id: "121.016.001-ST"
branch: "feat/117-s-shared-hcl-parser"
evaluated_head: "fbe9ef0131430033aee203a40e995ecdb999eb4e"
evaluated_tree: "3fda14160d7de05fe22a813421b04355c1b501ef"
runtime_report: "docs/closure/2026-08-16-hcl-family-parser-runtime-verification.md"
runtime_verdict: PASS
implementation_review: "PASS WITH ACKNOWLEDGMENT"
---

## Readiness decision

**READY.** Shipment `117-S` feature `121-F` is operationally ready for PR
handoff under the monitoring and rollback plan below. This pre-merge result
closes authoritative U16 task `121.016-T` and exact provenance/rollback
subtask `121.016.001-ST`.

READY does not authorize a merge. GitHub CI and Copilot review for the final
branch HEAD remain pending lifecycle gates. The release is a no-go until those
gates pass for the exact commit that would be merged.

No required local pre-merge gate is unresolved. U15 runtime verification is
PASS, the reviewed implementation has no P0/P1 finding, and the source, test,
and dependency tree has not changed since the recorded full gates.

## Scope and provenance

### Branch and evaluated HEAD

| Field | Evidence |
|---|---|
| Worktree | `C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618` |
| Branch | `feat/117-s-shared-hcl-parser` |
| Evaluated implementation HEAD | `fbe9ef0131430033aee203a40e995ecdb999eb4e` |
| Evaluated tree | `3fda14160d7de05fe22a813421b04355c1b501ef` |
| HEAD subject | `chore: complete U15 runtime verification state` |
| HEAD authored | `2026-08-16T20:47:50-07:00` |
| Initial U16 status | Clean worktree; `121.016-T` and `121.016.001-ST` active |

The U15 source SHA was
`f0f632d906cfcca885662bd7ff02c39e050112f7`. The only changes from that SHA
through evaluated HEAD were the U15 report and the completed U15 backlog
state. A path-scoped Git comparison returned no changes under `src`, `tests`,
`Cargo.toml`, or `Cargo.lock`.

The closure and U16 backlog commits made after this evaluation are
documentation/backlog-only transitions. They do not extend the reviewed
runtime surface. They do change branch HEAD, so GitHub CI and Copilot review
must bind to the final post-closure HEAD.

### Exact shipment manifest

The active shipment manifest is exactly:

| Scope | IDs |
|---|---|
| Shipment | `117-S` |
| Covering feature | `121-F` |
| Tasks | `121.001-T`, `121.002-T`, `121.003-T`, `121.004-T`, `121.005-T`, `121.006-T`, `121.007-T`, `121.008-T`, `121.009-T`, `121.010-T`, `121.011-T`, `121.012-T`, `121.013-T`, `121.014-T`, `121.015-T`, `121.016-T` |
| Subtasks | `121.002.001-ST`, `121.003.001-ST`, `121.015.001-ST`, `121.016.001-ST` |

This closure operates only on `121.016-T` and `121.016.001-ST`. Feature
`121-F` and shipment `117-S` remain active for PR and merge processing. They
must not be shipped or archived during pre-merge closure.

Authoritative inputs are:

* [Final U1-U16 implementation plan](../exec-plans/2026-08-15-hcl-family-parser-plan.md)
* [HCL parser decision](../decisions/2026-08-15-hcl-family-parser-deliberation.md)
* [Grammar compatibility spike](../decisions/2026-08-15-tree-sitter-hcl-compatibility-spike.md)
* [Stage adversarial review](2026-08-15-hcl-family-parser-stage-adversarial-review.md)
* [U15 runtime verification](2026-08-16-hcl-family-parser-runtime-verification.md)

Historical U1-U10 and U1-U14 sections are non-authoritative append-only
history. Only final U1-U16 governed this release.

## Released contract

### Dependency and provenance

The approved dependency is the crates.io registry package
`tree-sitter-hcl = "=1.1.0"`. `Cargo.lock` records:

```text
name = "tree-sitter-hcl"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5a7b2cc3d7121553b84309fab9d11b3ff3d420403eef9ae50f9fd1cd9d9cf012"
dependencies = ["cc", "tree-sitter-language"]
```

The package is Apache-2.0 licensed and uses the
`tree-sitter-language 0.1` bridge. Its development dependency targets
tree-sitter 0.25.3, and the resolved workspace runtime remains one
tree-sitter 0.25 line.

The registry artifact, manifest, binding, and checksum are the reproducibility
authority. The historical upstream `v1.1.0` Git tag is not source-equivalent
to the published 2025 crate. This accepted provenance exception does not
permit a Git/path source, vendored grammar, checksum change, alternate
version, unsafe shim, or second runtime tree-sitter. Any such change blocks
release and requires a new spike, decision, plan amendment, and review.

### Language and extraction

* Lowercase `.hcl`, `.tf`, and `.tfvars` are case-sensitive file aliases for
  one canonical stored and returned language, `hcl`
* `terraform` is not a language alias
* Each allowlisted top-level block emits one structural class-kind symbol
  named `hcl.block.<header-segments>` and one `Defines` edge
* Each allowlisted top-level attribute emits
  `hcl.attribute.<key>` and one `Defines` edge
* Plain traversals normalize to dotted targets such as `var.region`,
  `module.vpc.id`, `data.aws_ami.ubuntu.id`, and `aws_vpc.main.id`
* Duplicate `(file, target)` references preserve deterministic
  first-encounter order

HCL normalized targets are persistence-only hints. Each reference persists as
the existing file self-loop plus `target_hint`; HCL bypasses workspace-global
name resolution. The implementation does not fabricate resolved targets or
extra `map_code` nodes to expose those hints.

The parser performs no Terraform/HCL evaluation, provider or module download,
schema lookup, type inference, environment expansion, network request,
subprocess execution, or cross-workspace binding. Indexed forms are syntax
only.

### Routing, parity, and containment

Startup discovery, forced/ordinary explicit sync, and created/modified live
routing consume the same canonical HCL classifier. U15 proved parity through
cold startup, two explicit-sync cycles, restart, and Windows live watcher
create/modify events.

Existing workspace containment remains authoritative. Gitignored files,
oversize files, uppercase aliases, and outside-workspace files produced no
symbols. Malformed HCL remained bounded, produced no fabricated symbol, and
did not degrade daemon health.

## U15 reproducible runtime evidence

The [U15 report](2026-08-16-hcl-family-parser-runtime-verification.md) records
PASS for the exact branch binary, named-pipe daemon, direct sync, Windows file
watcher, embedded persistence, and isolated reverted runtime.

The retained primary fixture is:

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621
```

Retained rollback evidence is under the adjacent
`117-s-runtime-20260816-201621-rollback-src`,
`117-s-runtime-20260816-201621-rollback-target`, and
`117-s-runtime-20260816-201621-rollback-workspace` paths.

Reproduction baselines are:

| Scenario | Baseline |
|---|---|
| Cold startup | 3 parsed HCL files, 6 symbols, 12 extraction edges, no errors |
| Final forced index | 5 parsed files, 7 symbols, 15 extraction edges, no errors |
| Explicit sync | Two bounded edit cycles; replacements present once; stale names absent |
| Live routing | Create visible within 5 seconds; modify visible within 3 seconds |
| Restart | Stable workspace identity and no duplicate symbols |
| Persistence guard | HCL file self-loop plus target hint; colliding SQL reference still resolves |
| Containment | Zero symbols for ignored, oversize, uppercase, and outside controls |
| Corrected rollback | 3 files reconciled; 0 HCL files, symbols, and edges |

All U15-owned daemon processes were stopped by exact PID. The retained
fixtures were not deleted.

## Ordered current-HEAD gates

These gates apply to evaluated HEAD because its source, tests, manifest, and
lockfile are identical to the U15 gated tree. The format check was repeated at
evaluated HEAD to prove no source drift.

| Order | Gate | Result |
|---:|---|---|
| 1 | `cargo fmt --all -- --check` | PASS, exit 0 at evaluated HEAD |
| 2 | `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | PASS, exit 0 |
| 3 | `cargo dev-test --locked` | PASS, exit 0 |
| 4 | `cargo audit --no-fetch --stale` | PASS, exit 0; 14 allowed pre-existing warnings and no denied vulnerability |

The closure and backlog commits require no repeat of the heavy gates because
they do not change source, tests, dependencies, or configuration. A path
diff must still remain empty before PR handoff.

## Implementation adversarial review

The implementation adversarial review evaluated exact HEAD
`fbe9ef0131430033aee203a40e995ecdb999eb4e`.

| Severity | Count |
|---|---:|
| P0 | 0 |
| P1 | 0 |
| LOW P2 | 2 |

Verdict: **PASS WITH ACKNOWLEDGMENT**.

### AR-1 persistence hint visibility

AR-1 is acknowledged and disposed as intentional contract behavior.
Normalized HCL targets are persistence-only `references_edge.target_hint`
values on file self-loops. They are not resolved graph nodes, and `map_code`
must not fabricate nodes merely to display an unresolved hint. The focused
embedded persistence test and U15 forced-index evidence prove that the hints
exist while global binding remains bypassed. No defect remains.

### AR-2 advisory follow-up

AR-2 is acknowledged as a non-blocking advisory. It demonstrates no runtime,
contract, containment, persistence, or parity defect, and it does not alter
the PASS evidence. No twenty-first task or other work item is created:
shipment scope is frozen, the finding is LOW P2, and the operator prohibited
new work items or stash entries in U16.

## Strict-safety record

| ProposedAction | Targets and change kind | ActionRisk | Approval | ActionResult |
|---|---|---|---|---|
| Integrate the exact external grammar | `Cargo.toml`, `Cargo.lock`, native grammar build graph | High | The reviewed exact pin was explicitly operator-authorized; any alternate source/version requires new approval | Applied and verified |
| Enable canonical HCL routing | Startup/default, explicit sync, live routing, graph contents | Moderate | Explicit feature scope and reviewed final U1-U16 plan | Applied and verified |
| Create pre-merge operational closure | This closure artifact and exact U16 task states | Moderate | Explicit user instruction for `121.016-T` and `121.016.001-ST` | Applied |
| Execute rollback if a trigger fires | Runtime configuration, release binary, forced reconciliation, future merge-commit revert | High | Fresh operator approval required before execution | Planned; not executed |

No U16 action deleted a file, stopped a process, rewrote history, dropped
data, changed source/configuration, or executed a rollback.

## Invariants to preserve

* Exact registry dependency and checksum remain unchanged
* One canonical `hcl` identity covers only the three lowercase aliases
* Symbols retain `hcl.block.*` and `hcl.attribute.*` namespaces
* Traversals remain syntactic, deterministic, persistence-only hints
* HCL references never enter global binding
* Startup, explicit sync, and live routing remain behaviorally equivalent
* Existing ignore, size, malformed-input, and workspace containment gates
  precede persistence
* Repeated sync/restart creates no duplicate or stale graph records
* Engram adds no unsafe code or raw grammar-handle shim

## Pre-deploy audit

| Check | Result and evidence |
|---|---|
| Migration or schema | None. Existing `code_file`, symbol, `defines_edge`, and `references_edge` records are reused |
| Feature flag or rollout gate | None. `hcl` is intentionally default-enabled; explicit supported-language configuration is the mitigation control |
| Cross-service dependency | None. This is a local daemon and embedded database change |
| Dependency | Exact crates.io `tree-sitter-hcl 1.1.0`, checksum attested, no Git/path/vendor substitution, existing bridge, no second runtime tree-sitter |
| Compatibility | Safe `LANGUAGE.into()` ABI and representative `.hcl`, `.tf`, `.tfvars` parsing passed |
| Security and containment | No workspace unsafe code; ignored, oversize, malformed, uppercase, and outside controls passed |
| Runtime | U15 startup, explicit sync, live routing, restart, persistence, and rollback rehearsal passed |
| Monitoring | Manual plan below has named queries, baselines, thresholds, window, and owner |
| Rollback | Configuration and reverted-binary ordering was rehearsed; zero-HCL reconciliation is the completion gate |

The release path is PR merge by merge commit, then normal release-binary
publication and local daemon uptake. There is no database migration, canary
service, maintenance window, or separate hosted deployment.

## Monitoring plan

No hosted dashboard exists for this local-first daemon. The release operator
must run this manual observation plan on the retained fixture and the first
real HCL workspace.

| SLI | Query or log | Baseline | Alert or rollback threshold | Owner |
|---|---|---|---|---|
| Valid HCL index errors | `engram index --force --direct --workspace <fixture> --timeout 60 --format json`; inspect `errors` | `errors=[]` | Any error for valid lowercase HCL | Release operator |
| Graph cardinality | Forced-index JSON plus `engram workspace-status --workspace <fixture> --format json` | 5 files, 7 symbols, 15 extraction edges | Unplanned drift, duplicate, or stale symbol on repeated run | Release operator |
| Daemon and IPC health | `engram daemon-status --workspace <fixture> --format json` | Overall green and IPC reachable | Any red check, crash, or unavailable IPC | Release operator |
| Live replacement latency | Create and modify a compact `.tf`; poll `engram symbols` | Create in 5 seconds; modify in 3 seconds | Expected replacement absent after 10 seconds or old and new symbols coexist | Release operator |
| Persistence boundary | Run the focused `hcl_references_stay_hint_only_while_sql_references_still_resolve` test or inspect the equivalent embedded rows | HCL self-loop with hint; SQL collision resolves normally | Any HCL global binding or missing normalized hint | Release operator |
| Containment | File-filtered symbol queries for ignored, oversize, uppercase, and outside controls | Zero symbols from every control | Any control file enters the graph | Release operator |
| Rollback reconciliation | Reverted `workspace-status` after forced reconciliation | 0 HCL files, 0 symbols, 0 edges | Any residual HCL record or `unsupported language: hcl` error | Release operator |

Healthy means all baselines hold with no valid-input error, no containment
breach, and green daemon health. Degraded means a threshold is crossed but
the daemon remains available while mitigation is applied. Rolled back means
the revert procedure completed and the zero-HCL reconciliation gate passed.
Silence is not a healthy result.

## Post-merge and release observation window

Owner: **release operator**.

Duration: **30 minutes**, beginning when the merged/released binary first
indexes the retained fixture or the first real HCL workspace, whichever
occurs first.

Required checkpoints:

1. At startup, record daemon health, index errors, and graph counts.
2. Within 10 minutes, perform one HCL create/modify cycle and verify
   replacement without duplication.
3. At 30 minutes, repeat health, forced-index counts, persistence-boundary,
   and containment queries.
4. Record the close state as `healthy`, `degraded`, or `rolled back`.

The owner must continue observing through the entire window even when the
initial check is green.

## Rollback triggers and procedures

### Triggers

Rollback or immediate mitigation begins if any of these occur:

* Valid lowercase HCL produces an index error, daemon crash, or IPC failure
* Any `.hcl`, `.tf`, or `.tfvars` alias loses canonical `hcl` identity
* An HCL traversal binds globally or lacks its normalized persistence hint
* Repeated sync/restart leaves a stale or duplicate symbol
* An ignored, oversize, uppercase, or outside-workspace control enters the
  graph
* Live create/modify replacement is absent after 10 seconds
* The exact package source/checksum changes, a second runtime tree-sitter
  appears, or an unsafe shim is introduced

### Configuration mitigation

1. Set an explicit pre-release `code_graph.supported_languages` list that
   excludes `hcl`; do not rely on the new default list.
2. Restart only the affected local workspace daemon through the normal
   operator-controlled process lifecycle.
3. Stop indexing HCL workspaces if live events still route while mitigation
   is active.
4. Record health and error output. Configuration mitigation contains new
   indexing but does not by itself prove old HCL graph records were removed.

### Merge-commit revert path

1. Obtain explicit operator approval and identify the release merge commit.
2. Create a normal revert commit with `git revert -m 1 <merge-commit>`.
   Do not reset, rebase, force-push, or rewrite history.
3. Build and publish the reverted binary through normal release controls.
4. Restore the pre-HCL binary and pre-HCL configuration together. For the
   reverted binary, remove the feature-era explicit `hcl`-only setting or
   restore the exact prior supported-language list.
5. Run forced discovery/reconciliation on the affected workspace.
6. Require 0 HCL code files, 0 HCL symbols, 0 HCL edges, no
   `unsupported language: hcl` error, and green daemon health.
7. If any HCL record remains, keep rollback blocked and request separately
   approved remediation. Do not delete the embedded database as an automatic
   cleanup step.

No revert, history operation, configuration change, or forced reconciliation
is executed by this pre-merge closure.

## PR and merge gate

Only a GitHub **merge commit** is permitted. Squash and rebase merge are
forbidden. Before merging, verify repository settings allow merge commits and
disable squash/rebase choices.

GitHub CI and Copilot review are still pending for the final current HEAD.
After the closure and U16 backlog commits are pushed, the merge gate requires:

1. Every required CI check succeeds for the final branch HEAD.
2. A Copilot review exists whose `commit_id` exactly equals that HEAD.
3. Copilot is absent from `requested_reviewers`.
4. Zero unresolved review threads remain.
5. GitHub reports `mergeable_state == clean`.

Any HEAD change after review, including a documentation or backlog-only
commit, invalidates the current-HEAD review gate. Re-run CI and obtain a new
Copilot review before merge. A transient clean mergeable state after a push
is not authorization.

## Provenance, rollback, and scope-preservation evidence

Exact subtask `121.016.001-ST` was active before closure work. Its required
registry/tag exception, artifact attestation, owner/window, rollback triggers,
clean forced-reindex reconciliation, and blocked-return handoff are recorded
above.

Commit `aa14af6ec4d47846c094feb6ea7a1b1e3a17b8dd` remains the preserved stash
commit. Its seven IDs are:

* `4D08C3D9`
* `0B729BFE`
* `60A58C8D`
* `C64FD73F`
* `B82ABA6E`
* `1328405A`
* `AA96FC45`

U16 did not edit, add, archive, or remove a stash entry. It created no work
item. Shipment `116-S`, feature `120-F`, its tasks, and any later shipment
received no mutation.

Agent-intercom and backlog MCP/sync visibility were not exposed in this
execution environment. Remote progress and index-synchronization visibility
were therefore degraded. The local backlogit CLI remained reachable and
provided the authoritative item, dependency, manifest, metadata, status, and
commit-link operations. This degraded observability does not weaken the
recorded Git diff or local task state, but it must not be represented as
remote operator awareness.

## Final handoff

* Operational readiness: **READY**
* Runtime verification: **PASS**
* Implementation review: **PASS WITH ACKNOWLEDGMENT**
* Local gates: **PASS**
* Monitoring owner/window: release operator, 30 minutes
* Rollback readiness: rehearsed configuration-plus-binary restoration and
  forced reconciliation with a zero-HCL completion gate
* Remaining gate: final-HEAD GitHub CI and Copilot review, then
  merge-commit-only merge
* Feature/shipment state: `121-F` active; `117-S` active
