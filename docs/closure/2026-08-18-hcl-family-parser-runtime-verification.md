---
title: "HCL family parser final scoped runtime verification"
date: 2026-08-18
doc_type: runtime-verification
mode: auto
surface: "cli/background-job"
verdict: PASS
shipment_id: "117-S"
feature_id: "121-F"
task_id: "121.015-T"
branch: "feat/117-s-shared-hcl-parser"
evaluated_implementation_head: "10e7533ffccba0a432d67a3d3ec522f4e3c0e58b"
evaluated_tree: "f99e02c565d2d5f97ff8448a95bb2b5974e3e51e"
binary_sha256: "5048626D26A7EB35CA90142F3264C4E3FA96D5A143513680BEB5F7CA1A4EB77D"
---

## Verdict

**PASS.** The exact current-branch binary and daemon exercised the final
scoped HCL implementation on Windows. Cold startup indexed lowercase `.hcl`,
`.tf`, and `.tfvars` through the default supported-language configuration.
Canonical identity, structural symbols, hint-only persistence, two explicit
sync cycles, live create and modify routing, restart persistence, containment
controls, replacement-race safety, and rollback reconciliation matched the
release contract.

No runtime discrepancy was found. No source, test, Cargo, backlog, stash, plan,
or PR state was changed.

## Scope and authority

This run follows the
[post-boundary Stage triage](../decisions/2026-08-18-117-s-post-boundary-commit-triage.md)
and supersedes the stale implementation evidence in the
[2026-08-16 closure](2026-08-16-hcl-family-parser-operational-closure.md).
The implementation authority is:

| Field | Exact value |
|---|---|
| Worktree | `C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618` |
| Branch | `feat/117-s-shared-hcl-parser` |
| Evaluated implementation HEAD | `10e7533ffccba0a432d67a3d3ec522f4e3c0e58b` |
| Evaluated tree | `f99e02c565d2d5f97ff8448a95bb2b5974e3e51e` |
| Binary version | `engram 0.2.0+g10e7533f` |
| Runtime binary SHA-256 | `5048626D26A7EB35CA90142F3264C4E3FA96D5A143513680BEB5F7CA1A4EB77D` |

The branch and tree checks were clean before runtime work. Runtime fixtures are
ignored, retained evidence rather than tracked product changes.

## Strict-safety record

| Field | Value |
|---|---|
| ProposedAction summary | Exercise the exact branch binary in retained isolated workspaces and stop only daemons started by this run |
| Targets | The retained fixture roots, exact branch binary, and this report |
| Change kind | Local fixture creation and edits, local daemon lifecycle, read-only graph queries, deterministic tests, and documentation |
| ActionRisk | Moderate |
| Approval required | Yes; supplied explicitly in the operator request |
| Rollback | Retain fixture state, stop every owned daemon by exact PID, and leave source and shared state unchanged |
| ActionResult | Applied |

No directory or file was deleted as cleanup. No process was stopped by name.
No history rewrite, push, database drop, or external mutation ran.

## Environment prechecks

| Check | Expected | Observed |
|---|---|---|
| Operating system | Windows with symlink capability | Windows 11 Enterprise `10.0.26200`, build `26200`, 64-bit |
| Rust | Installed stable toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)` |
| Cargo | Matching Cargo | `cargo 1.97.0 (c980f4866 2026-06-30)` |
| Branch | Shipment branch | Exact match |
| Implementation HEAD | Supplied immutable SHA | Exact match |
| Implementation tree | Supplied immutable tree | Exact match |
| Tracked status | Clean | Clean |
| Fixture path | New | `False` before creation |

The first cold-start attempt exited `2` before creating a process because the
new fixture was not yet a Git root:

```text
Error: cannot compute IPC endpoint: Path '...\117-s-final-runtime-20260818-101417'
is not a Git repository root
```

The contained fixture was initialized with `git init`, then the unchanged
cold-start command passed. This was a fixture prerequisite correction, not an
implementation retry or product discrepancy.

Agent-intercom and repository Engram MCP surfaces were not exposed to this
execution. Remote visibility and indexed repository lookup were degraded.
The runtime itself used the exact repository binary and its local daemon.

## Build and binary evidence

Command:

```powershell
cargo build --locked --manifest-path "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\Cargo.toml" --target-dir "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target"
```

Observed: exit `0`; `Finished dev profile` after 1 minute 28 seconds. The
runtime binary was re-hashed after the focused test builds and before closure;
its SHA-256 and version are recorded above.

## Retained fixtures

Primary runtime workspace:

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417
```

Outside-workspace link targets:

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417-outside
```

Rollback rehearsal:

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417-rollback-workspace
```

The primary config intentionally omitted `supported_languages` to exercise
the default HCL enablement:

```toml
idle_timeout_minutes = 0
debounce_ms = 150

[code_graph]
max_file_size_bytes = 512
parse_concurrency = 1
```

Initial lowercase sources were `infra/main.tf`, `infra/service.hcl`, and
`env/values.tfvars`. Controls included an ignored `.tf`, a 701-byte `.hcl`,
three uppercase aliases, and external directory and file link targets.

## Cold daemon startup and default indexing

Command:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" daemon-status --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417" --format json
```

Observed: exit `0`; PID `7420`; workspace identity
`62829467404a943b05974d09ef229b35129e2a14bfb31835d998a899bd3d3578`;
binary, PID, identity, pipe, registry, offline-scan, and session checks green.
The only yellow check was expected empty startup telemetry.

Cold `workspace-status` returned:

```json
{
  "classes": 6,
  "code_files": 3,
  "edges": 6,
  "functions": 0,
  "interfaces": 0
}
```

The six exact structural symbols were:

```text
env/values.tfvars hcl.attribute.region
env/values.tfvars hcl.attribute.replicas
infra/main.tf hcl.block.data.aws_ami.ubuntu
infra/main.tf hcl.block.resource.aws_instance.web
infra/service.hcl hcl.block.locals
infra/service.hcl hcl.block.service.api
```

Forced daemon index command:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" index --force --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417" --timeout 60 --format json
```

Exact result counts:

```json
{
  "classes_indexed": 6,
  "edges_created": 12,
  "errors": [],
  "files_parsed": 3,
  "files_skipped": 1,
  "oversized_files_skipped": 1
}
```

The twelve extraction edges were six `Defines` edges plus six normalized
traversal-reference rows.

## Canonical identity, structure, and persistence boundary

`map-code hcl.block.resource.aws_instance.web --depth 2 --max-nodes 20`
returned the exact HCL class as the root, its `infra/main.tf` file node, and
two file-to-class `defines` edges. No fallback was used and the result was not
truncated.

The real embedded-backend security target ran with:

```powershell
cargo test --locked --manifest-path "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\Cargo.toml" --target-dir "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target" --test hcl_security_test
```

Observed: exit `0`; six passed, zero failed. This includes
`hcl_references_stay_hint_only_while_sql_references_still_resolve`, which
proves:

* Stored HCL language is canonical `hcl`
* HCL references persist as file self-loops with `target_hint`
* HCL bypasses workspace-global name binding
* A colliding SQL reference still resolves normally

The same target also passed static links, ignored and outside aliases,
oversize limits, malformed/deep input bounds, and no-side-effect syntax
controls.

## Explicit edit and sync cycles

PID `7420` was stopped by exact PID before direct synchronization.

### Cycle 1

`infra/main.tf` changed `aws_instance.web` to `aws_instance.web_v2` and
added `var.instance_count`.

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" sync --direct --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417" --timeout 60 --format json
```

Observed:

```json
{
  "edges_created": 7,
  "errors": [],
  "files_added": 3,
  "files_deleted": 6,
  "files_modified": 0,
  "files_unchanged": 0,
  "oversized_files_skipped": 1,
  "symbols_reembedded": 6
}
```

The one-time registry classification reflects switching from daemon startup
state to the direct-sync snapshot. The graph remained three code files and
six symbols after restart; only `web_v2` remained.

### Cycle 2

`infra/service.hcl` changed `service.api` to `service.api_v2` and added
`var.region`. The same direct command returned:

```json
{
  "edges_created": 3,
  "errors": [],
  "files_added": 0,
  "files_deleted": 0,
  "files_modified": 1,
  "files_unchanged": 2,
  "oversized_files_skipped": 1,
  "symbols_reembedded": 1,
  "symbols_reused": 1
}
```

Restart PID `25964` retained the same workspace identity and exact counts of
three files, six symbols, and six `Defines` edges. `web_v2` and `api_v2`
each existed once; the prior names were absent.

## Live create and modify routing

While PID `25964` was running, `infra/live-created.tf` was created. After a
five-second bounded wait, a file-scoped symbol query returned exactly:

```text
hcl.block.resource.null_resource.live_created
```

The file was modified without an explicit index or sync command. After three
seconds, the same query returned exactly:

```text
hcl.block.resource.null_resource.live_modified
```

The old symbol was absent.

A second watcher-only proof avoided a graph CLI call between mutation and
inspection. While PID `26808` was idle, `infra/watch-only.tf` was created and
then modified. Direct reads of the retained
`.engram/code-graph/master/nodes.jsonl` observed:

```text
hcl.block.resource.null_resource.watch_created
hcl.block.resource.null_resource.watch_modified
stale watch_created count: 0
```

The create appeared within five seconds and the replacement within three
seconds. This proves background daemon persistence rather than a query-time
explicit sync. Final CLI projection returned only `watch_modified`.

## Malformed, resource, ignore, and case controls

| Scenario | Expected | Observed |
|---|---|---|
| Malformed `malformed.tf` | Bounded parse with no fabricated symbol | Zero symbols; final daemon health green |
| Oversized `oversized.hcl` | Skip before parse at 512 bytes | Zero symbols; `oversized_files_skipped=1` |
| Gitignored `ignored.tf` | Excluded | Zero symbols |
| `uppercase.HCL` | Case-sensitive exclusion | Zero symbols |
| `uppercase.TF` | Case-sensitive exclusion | Zero symbols |
| `uppercase.TFVARS` | Case-sensitive exclusion | Zero symbols |
| External directory target | Never persist | Zero linked symbols |
| External final-file target | Never persist | Zero linked symbols |

Two final forced indexes returned identical semantic counts:

```json
{
  "classes_indexed": 8,
  "edges_created": 18,
  "errors": [],
  "files_parsed": 6,
  "files_skipped": 1,
  "oversized_files_skipped": 1
}
```

Final `workspace-status` returned six code files, eight structural symbols,
and eight `Defines` edges. The 18 extraction edges were eight `Defines` and
ten hint-only reference rows.

## Windows static symlink and reparse containment

PowerShell confirmed both retained controls were real `SymbolicLink` reparse
entries:

```text
linked-directory -> ...\117-s-final-runtime-20260818-101417-outside\directory-target
linked-final.tf -> ...\117-s-final-runtime-20260818-101417-outside\linked-final.tf
```

The forced index returned no error, did not traverse the directory link, and
did not open the final-file link. File-scoped queries for
`linked-directory/linked.tf` and `linked-final.tf` each returned zero symbols.
The external sentinel namespace was absent.

## Deterministic replacement safety

Command:

```powershell
cargo test --locked --manifest-path "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\Cargo.toml" --target-dir "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target" --lib services::code_graph::source_race_tests
```

Observed: exit `0`; 29 passed, zero failed, zero ignored. Critical Windows
coverage included:

* `full_index_rejects_file_replaced_by_external_link_after_discovery`
* `cold_sync_rejects_ancestor_replaced_by_external_link_after_discovery`
* `explicit_sync_rejection_retains_last_known_good_graph`
* `sync_rejects_file_replaced_by_internal_link_after_discovery`
* `sync_rejects_ancestor_replaced_by_internal_link_after_discovery`
* `regular_file_controls_remain_indexable`
* `capability_directory_enumeration_classifies_without_following_links`

These deterministic barriers replace the final file or ancestor after
discovery and before the scoped code-graph reader opens or publishes it. They
prove external bytes never reach file, function, or class bodies and prove
last-known-good preservation where required.

Two discovery-root replacement tests are intentionally Unix-only and were
compiled out on Windows:

```text
final_workspace_replacement_cannot_redirect_discovery_or_deletion
workspace_ancestor_replacement_cannot_redirect_discovery_or_deletion
```

They remain retained CI evidence for Unix rename and discovery-root behavior.
They are not represented as Windows runtime passes.

## Restart persistence and stable identity

The primary daemon identities and counts remained stable across PIDs `7420`,
`25964`, `26264`, `26808`, and final restart PID `23768`.

| Property | Initial | Final restart |
|---|---|---|
| Workspace identity | `62829467404a943b05974d09ef229b35129e2a14bfb31835d998a899bd3d3578` | Same |
| Database path | `.engram\cozo\master\engram.db` | Same |
| Code files | 3 | 6 |
| Symbols | 6 | 8 |
| Defines edges | 6 | 8 |
| Stale files | `false` | `false` |

The final restart preserved all expected edited and live-routed symbols
without duplicates or stale names.

## Configuration exclusion and rollback reconciliation

The primary fixture was copied only after its daemons were stopped. PowerShell
materialized the two static links in the rollback copy, so its pre-mitigation
offline scan reported seven HCL files and nine symbols rather than the
primary snapshot's five and seven. This strengthens the reconciliation check:
all copied HCL state still had to be removed.

The isolated config was changed to:

```toml
[code_graph]
supported_languages = ["rust"]
max_file_size_bytes = 512
parse_concurrency = 1
```

Forced reconciliation command:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" index --force --direct --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-final-runtime-20260818-101417-rollback-workspace" --timeout 60 --format json
```

Observed:

```json
{
  "classes_indexed": 0,
  "edges_created": 0,
  "errors": [],
  "files_parsed": 0,
  "files_reconciled": 3,
  "oversized_files_skipped": 0
}
```

Restart PID `15968` then returned exactly zero code files, zero classes, zero
functions, zero interfaces, and zero edges. A canonical HCL query found no
symbol. The rollback/configuration exclusion procedure therefore has a
deterministic zero-state completion gate.

## Process ledger

| PID | Purpose | Stop evidence |
|---:|---|---|
| `7420` | Cold startup and baseline index | Exact `Stop-Process -Id 7420`; exact lookup absent |
| `25964` | Sync restart, live route, malformed and guard checks | Exact stop; exact lookup absent |
| `26264` | Persistence restart | Exact stop; exact lookup absent |
| `32344` | Rollback-copy baseline | Exact stop; exact lookup absent |
| `15968` | Zero-state rollback restart | Exact stop; exact lookup absent |
| `26808` | Watcher-only persistence proof and final forced indexes | Exact stop; exact lookup absent |
| `23768` | Final identity and count restart | Exact stop; exact lookup absent |

The final combined exact-PID query returned no process. Expected PowerShell
exit `1` means no requested PID remained.

## Runtime handoff

* Verification verdict: **PASS**
* Runtime surfaces: exact CLI binary, daemon, IPC, direct sync, watcher,
  embedded graph persistence, Windows links, and isolated rollback
* Healthy baseline: final forced index `6/8/18`, workspace status `6/8/8`,
  `errors=[]`, one oversize skip, stable identity
* Rollback completion gate: `0` HCL files, symbols, and edges
* Accepted containment residuals: hardlinks, mount points, and in-place
  mutation only
* ActionResult: **applied**
* Next action: operational closure, then pending current-HEAD and merge-commit
  gates
