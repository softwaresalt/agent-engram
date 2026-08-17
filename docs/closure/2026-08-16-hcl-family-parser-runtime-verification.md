---
title: "HCL family parser runtime verification"
doc_type: closure
source: docs/exec-plans/2026-08-15-hcl-family-parser-plan.md
date: 2026-08-16
shipment_id: "117-S"
feature_id: "121-F"
task_id: "121.015-T"
subtask_id: "121.015.001-ST"
surface: "cli/background-job"
mode: auto
branch: "feat/117-s-shared-hcl-parser"
commit: "f0f632d906cfcca885662bd7ff02c39e050112f7"
verdict: PASS
---

## Verdict

**PASS.** The exact current-branch binary indexed lowercase `.hcl`, `.tf`, and
`.tfvars` through startup, direct explicit sync, daemon live routing, and
restart paths. The observed graph retained canonical `hcl` identity,
namespaced symbols, and hint-only HCL references. Malformed, oversize,
gitignored, uppercase, and outside-workspace inputs stayed bounded and did not
create unintended symbols.

The isolated rollback rehearsal also passed after restoring the pre-HCL
configuration before forced reconciliation. No U16 work was performed.

## Scope and authority

This report executes only authoritative U15 `121.015-T` and
`121.015.001-ST` from shipment `117-S`. It follows the final U1-U16 section of
the implementation plan and the final remediation review. Historical U1-U10,
U1-U14, and U13/U14 numbering in the append-only plan history was not used.

Authoritative inputs:

* [Final implementation plan](../exec-plans/2026-08-15-hcl-family-parser-plan.md)
* [HCL parser decision](../decisions/2026-08-15-hcl-family-parser-deliberation.md)
* [Grammar compatibility spike](../decisions/2026-08-15-tree-sitter-hcl-compatibility-spike.md)
* [Final adversarial review](2026-08-16-hcl-family-parser-final-remediation-adversarial-review.md)

## Strict safety record

| Field | Value |
|---|---|
| ProposedAction summary | Exercise the exact branch binary against retained isolated fixtures and stop only daemons started by this verification |
| Targets | The retained fixture roots listed below, exact branch and rollback binaries, and the U15 report |
| Change kind | Local fixture edits, local builds, process lifecycle, read-only queries, and one backlog status transition |
| ActionRisk | Moderate |
| Rollback | Retain all fixtures; stop each owned daemon by exact PID; do not mutate production, shared state, or history |
| Approval required | No; the operator expressly authorized bounded runtime edits and exact-PID process stops |
| ActionResult | Applied |

No file or directory was deleted. No process was stopped by name. No force
push, history rewrite, database drop, or cleanup operation ran.

## Environment prechecks

| Check | Expected | Observed | Exit |
|---|---|---|---:|
| Worktree branch | `feat/117-s-shared-hcl-parser` | Exact branch, initially clean | 0 |
| Branch source SHA | Current U15 HEAD | `f0f632d906cfcca885662bd7ff02c39e050112f7` | 0 |
| Operating system | Windows runtime capable of named-pipe daemon IPC | `Windows_NT` | 0 |
| Rust compiler | Installed stable toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)` | 0 |
| Cargo | Installed Cargo matching the compiler | `cargo 1.97.0 (c980f4866 2026-06-30)` | 0 |
| Backlog state | U15 parent and exact subtask active | Both active before evidence capture | 0 |
| Existing fixture path | Absent before creation | `False` | 0 |
| Daemon IPC | Reachable after automatic spawn | Named-pipe health response received | 0 |

Agent-intercom and agent-engram MCP tools were not exposed to this execution
environment. Remote visibility and indexed repository lookup were therefore
degraded; runtime verification used only the repository binary and local CLI.

## Binaries under test

The current branch binary was built with:

```powershell
cargo build --locked --manifest-path "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\Cargo.toml" --target-dir "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target"
```

Observed result: exit `0`; `Finished dev profile` after 45.76 seconds.

| Property | Value |
|---|---|
| Binary | `C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe` |
| Version | `engram 0.2.0+gf0f632d9-dirty` |
| SHA-256 at runtime start | `1BBE73003E49CBCF6E7BABFBE2298B0E58D65BC6B83AA59FFE5475D916DFA711` |
| Source SHA | `f0f632d906cfcca885662bd7ff02c39e050112f7` |
| Dirty reason | Authorized U15 backlog status transition; no source, test, dependency, or product-doc edit |

The rollback source was exported from
`origin/main@6268c1ac77db64deb5ffe7af820735dbe172624f` with `git archive`, built
under the retained fixture, and produced SHA-256
`B66D271118DDE56AE4DD3026C1AA05F445F73189A3FD2801CCCC739ED88D3CB9`.
Its build-time version text inherited the enclosing worktree Git context, so
the archive SHA is the authoritative rollback source identity.

## Retained fixture

Primary fixture:

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621
```

Retained rollback material:

```text
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621-rollback-src
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621-rollback-target
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621-rollback-workspace
```

The primary `.engram/config.toml` set:

```toml
idle_timeout_minutes = 0
debounce_ms = 150

[code_graph]
supported_languages = ["hcl"]
max_file_size_bytes = 512
parse_concurrency = 1
```

The initial lowercase source set contained:

* `infra/main.tf` with `resource` and `data` blocks plus
  `data.aws_ami.ubuntu.id`, `var.region`, `module.vpc.id`, and
  `aws_vpc.main.id` traversals
* `infra/service.hcl` with `locals` and `service` blocks plus
  `module.vpc.id` and `local.owner` traversals
* `env/values.tfvars` with `region` and `replicas` attributes
* `ignored.tf`, excluded by `.gitignore`
* `oversized.hcl`, 716 bytes against the configured 512-byte limit
* a sibling outside-workspace control file,
  `117-s-runtime-20260816-201621-outside.tfvars`

## Cold startup and default indexing

The first command below automatically spawned the exact branch daemon:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" daemon-status --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621" --format json
```

Observed: exit `0`, PID `8004`, version `0.2.0`, workspace identity
`48a2b678358200a73f764001cdfde1e2a4d6df0fa8d4762d5f730a0c19e9dd54`,
and green binary, PID, identity, pipe, registry, offline-scan, and session
checks. The only initial yellow check was the expected empty telemetry state.

Cold `workspace-status` observed:

| Count | Expected | Observed |
|---|---:|---:|
| Code files | 3 lowercase supported files | 3 |
| Classes/symbols | 6 top-level declarations | 6 |
| Defines edges | 6 | 6 |
| Functions | 0 | 0 |
| Interfaces | 0 | 0 |

The symbol inventory was exact:

```text
env/values.tfvars    hcl.attribute.region
env/values.tfvars    hcl.attribute.replicas
infra/main.tf        hcl.block.data.aws_ami.ubuntu
infra/main.tf        hcl.block.resource.aws_instance.web
infra/service.hcl    hcl.block.locals
infra/service.hcl    hcl.block.service.api
```

A forced daemon index returned exit `0` with:

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

For this HCL-only fixture, the 12 created edges are six `Defines` rows and six
normalized traversal-reference rows. This matches the six source traversals
listed above.

## Canonical identity and reference boundary

All three lowercase extensions were stored and returned through one
`hcl.*` namespace. The exact persistence test was also run against the real
embedded graph backend:

```powershell
cargo test --locked --manifest-path "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\Cargo.toml" --target-dir "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target" --test hcl_security_test hcl_references_stay_hint_only_while_sql_references_still_resolve -- --exact
```

Observed: exit `0`; one passed, zero failed. The test reads persisted
`code_file.language` and `references_edge` rows. It proved:

* the HCL file language is exactly `hcl`
* the HCL row is `(file_id, file_id, target_hint)`
* the colliding SQL reference still resolves to its SQL class
* HCL never uses the workspace-global name resolver

The daemon forced-index count proves that traversal rows were produced during
the actual startup/index path. The focused embedded-DB query proves their
self-loop and hint-only representation because the public CLI graph projection
does not expose `references_edge.qualified_name`.

## Edit and explicit-sync cycles

PID `8004` was stopped by exact PID before the direct sync cycles.

### Cycle 1

Edit:

* renamed `hcl.block.resource.aws_instance.web` to
  `hcl.block.resource.aws_instance.web_v2`
* added `var.instance_count`

Command:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\target\debug\engram.exe" sync --direct --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621" --timeout 60 --format json
```

Observed: exit `0`, `errors=[]`, `edges_created=7`,
`oversized_files_skipped=1`, and the direct-sync registry classified the
three-file snapshot as `files_added=3`. On restart the graph remained exactly
3 files, 6 symbols, and 6 Defines edges; `web_v2` existed once and `web` was
absent.

### Cycle 2

Edit:

* renamed `hcl.block.service.api` to `hcl.block.service.api_v2`
* added `var.region`

The same explicit direct-sync command observed exit `0`, `errors=[]`,
`files_modified=1`, `files_unchanged=2`, `files_added=0`,
`edges_created=3`, and `oversized_files_skipped=1`.

The two cycles are distinct and bounded. The first establishes the direct-sync
snapshot registry after the daemon startup path; the second demonstrates the
incremental modified-file classification. Neither produced duplicate symbols.

## Restart persistence

The daemon restarted as PID `22672`. It retained the same workspace identity
and database path:

```text
workspace identity:
48a2b678358200a73f764001cdfde1e2a4d6df0fa8d4762d5f730a0c19e9dd54

database:
C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621\.engram\cozo\runtime\engram.db
```

Post-restart counts were stable at 3 code files, 6 symbols, and 6 Defines
edges. The updated inventory contained `web_v2` and `api_v2`, each exactly
once, while the superseded names were absent.

After live and guard scenarios, a second restart as PID `32552` retained the
same identity and final counts of 5 code files, 7 symbols, and 7 Defines
edges. A final health request was fully green.

## Live daemon routing

While PID `22672` was running, `infra/live-created.tf` was created with
`hcl.block.resource.null_resource.live_created`. Without an explicit sync,
the symbol appeared after a bounded five-second wait.

The same file was then modified to
`hcl.block.resource.null_resource.live_modified`. Without an explicit sync,
the new symbol replaced the old symbol after a bounded three-second wait.
The post-live graph was 4 files, 7 symbols, and 7 Defines edges.

This is actual `ReadDirectoryChangesW` daemon/watch behavior, not unit-only
evidence.

## Resource and containment guards

| Scenario | Expected | Observed | Result |
|---|---|---|---|
| Malformed lowercase `.tf` | Bounded parse, no fabricated symbols, daemon healthy | File recorded with zero symbols; graph stayed at 7 symbols; health fully green | PASS |
| Oversize `.hcl` | Skip before parse at 512 bytes | `oversized_files_skipped=1`; zero symbols | PASS |
| Gitignored `ignored.tf` | Never index | Zero symbols; forced parse count excluded it | PASS |
| Uppercase `.HCL` | Preserve documented case-sensitive behavior | Zero symbols | PASS |
| Uppercase `.TF` | Preserve documented case-sensitive behavior | Zero symbols | PASS |
| Uppercase `.TFVARS` | Preserve documented case-sensitive behavior | Zero symbols | PASS |
| Outside-workspace `.tfvars` sibling | Never enter the fixture graph | Absolute file filter returned zero symbols; forced parse count excluded it | PASS |

The lowercase live route proves the Windows watcher was available. Uppercase
files were deliberately exercised on the same watcher and skipped, matching
the final U3 case-characterization contract rather than silently widening it.

Two final forced direct indexes returned the same semantic counts:

```json
{
  "classes_indexed": 7,
  "edges_created": 15,
  "errors": [],
  "files_parsed": 5,
  "files_skipped": 1,
  "oversized_files_skipped": 1
}
```

The stable HCL-only result consists of seven `Defines` rows and eight
hint-only reference rows. The malformed file contributes one code-file record
and no symbols or references.

## Isolated rollback reconciliation

The primary fixture was copied after all daemons were stopped. Before rollback,
the copy reported 5 code files, 7 symbols, and 7 Defines edges.

The first reverted-binary force run intentionally retained the feature-era
`supported_languages=["hcl"]` setting. It failed closed:

```json
{
  "classes_indexed": 0,
  "errors": [
    {
      "error": "Source parse failed: unsupported language: hcl",
      "file": "infra/service.hcl"
    }
  ],
  "files_parsed": 0
}
```

It left only `infra/service.hcl` with two HCL symbols. This proves that binary
rollback without configuration rollback is not sufficient.

The isolated config was then restored to the pre-HCL shape by removing the
explicit `supported_languages=["hcl"]` line. No database file was deleted.
The reverted binary ran:

```powershell
& "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621-rollback-target\debug\engram.exe" index --force --direct --workspace "C:\Source\GitHub\engram\.git\ship-session-state\117-s-20260816-1618\tmp\117-s-runtime-20260816-201621-rollback-workspace" --timeout 60 --format json
```

Observed: exit `0`, `errors=[]`, `files_reconciled=3`,
`files_parsed=0`, `classes_indexed=0`, and `edges_created=0`. A reverted
daemon restart then reported 0 code files, 0 symbols, and 0 edges.

Rollback is therefore actionable only in this order:

1. Restore the pre-HCL binary and configuration together.
2. Run forced discovery/reconciliation.
3. Require zero HCL code files, symbols, and edges.
4. Block rollback and request operator-approved remediation if any HCL record
   remains or an `unsupported language: hcl` error appears.

## Process ledger

| PID | Binary/source | Purpose | Stop evidence |
|---:|---|---|---|
| 8004 | Current branch | Cold startup and baseline index | `Stop-Process -Id 8004`; subsequent exact lookup returned no process |
| 22672 | Current branch | Restart, live create/modify, malformed guard | `Stop-Process -Id 22672`; subsequent exact lookup returned no process |
| 32552 | Current branch | Final persistence restart and health | `Stop-Process -Id 32552`; subsequent exact lookup returned no process |
| 18372 | Current branch | Rollback-copy baseline count | `Stop-Process -Id 18372`; subsequent exact lookup returned no process |
| 27376 | Reverted archive | First fail-closed rollback attempt | `Stop-Process -Id 27376`; subsequent exact lookup returned no process |
| 2776 | Reverted archive | Corrected rollback restart | `Stop-Process -Id 2776`; subsequent exact lookup returned no process |

The final exact-PID process query returned no owned daemon process. Expected
PowerShell exit `1` on each confirmation means the requested PID no longer
existed.

## Command and result summary

| Operation | Exit | Key output |
|---|---:|---|
| Exact branch build | 0 | Dev binary built from `f0f632d9` |
| Cold `daemon-status` | 0 | PID 8004; workspace identity stable |
| Cold `workspace-status` | 0 | 3 files, 6 symbols, 6 Defines edges |
| Baseline forced index | 0 | 3 parsed, 6 symbols, 12 total extraction edges, 1 oversize skip |
| Explicit sync cycle 1 | 0 | No errors; updated `web_v2`; no duplicate |
| Explicit sync cycle 2 | 0 | 1 modified, 2 unchanged; updated `api_v2`; no duplicate |
| Restart status | 0 | Stable 3/6/6 counts and identity |
| Live created route | 0 | `live_created` appeared without sync |
| Live modified route | 0 | `live_modified` replaced old symbol without sync |
| Malformed health | 0 | Zero malformed symbols; daemon health green |
| Guard queries | 0 | Ignored, oversize, uppercase, and outside controls returned zero symbols |
| Final forced indexes | 0 | Both returned 5 parsed, 7 symbols, 15 extraction edges, no errors |
| Hint-only persistence target | 0 | 1 passed, 0 failed |
| Rollback force after config restore | 0 | 3 reconciled, zero HCL records |

One focused-test invocation initially exited `101` because PID `22672` still
held `target\debug\engram.exe` on Windows. The exact owned PID was stopped and
the unchanged command then passed. This was an environment sequencing issue,
not a test failure.

## Quality gates

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | PASS, exit 0 |
| Strict clippy | `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | PASS, exit 0 in 35.87 seconds |
| Required dev tests | `cargo dev-test --locked` | PASS, exit 0 for the configured library and five HCL test targets |
| Dependency audit | `cargo audit --no-fetch --stale` | PASS, exit 0; 14 allowed warnings and no denied vulnerability |
| Markdown structure | Backlogit docline lint, `engram verify`, and local structural checker | PASS, all exited 0 |

The installed `cargo-audit` did not accept `--offline` and returned exit `2`
without scanning. The supported offline-equivalent command
`cargo audit --no-fetch --stale --file <worktree>\Cargo.lock` then completed
the required audit with exit `0`.

## Monitoring handoff for U16

There is no configured remote dashboard for this local-first daemon. U16 should
carry this manual monitoring plan into operational closure:

| SLI | Baseline | Alert or rollback threshold | Observation |
|---|---|---|---|
| HCL index errors | `errors=[]` | Any HCL parse/index error on valid input | Inspect `index` or `sync` JSON |
| Stable graph cardinality | Repeated force result `5/7/15` for parsed files/symbols/extraction edges | Any unplanned count drift or duplicate symbol | Compare forced-index JSON and `workspace-status` |
| Daemon health | Overall green after activity | Any red health check or unavailable IPC | `engram daemon-status --format json` |
| Live route latency | Created within 5 seconds; modified within 3 seconds | Missing replacement after 10 seconds | Poll `engram symbols` |
| Containment | Zero symbols for ignored, oversize, uppercase, and outside controls | Any symbol from a control file | File-filtered `engram symbols` |
| Rollback reconciliation | 0 files, 0 symbols, 0 edges | Any residual HCL record or unsupported-language error | Reverted `workspace-status` after forced reconcile |

Suggested owner: release operator. Suggested post-merge observation window:
30 minutes after first real HCL workspace index, with checks at startup, after
one edit, and at window close.

Rollback trigger: valid lowercase HCL produces an error, a traversal binds
globally instead of remaining a file self-loop with a hint, live replacement
duplicates a symbol, a containment control enters the graph, or daemon health
turns red. Roll back the binary and HCL configuration together, force
reconciliation, and require a zero-HCL graph before declaring rollback
complete.

## Operational-closure handoff

* Verification verdict: PASS
* Runtime surfaces: exact CLI, named-pipe daemon, startup index, direct sync,
  Windows file watcher, embedded persistence, and reverted runtime
* Evidence: retained fixtures, exact JSON counts, symbol inventories, health
  responses, process ledger, focused persistence test, and rollback rehearsal
* Blocked prerequisites: none
* Risky action state: moderate local verification applied; no destructive
  action performed
* Follow-up: U16 should record the monitoring window, provenance exception,
  rollback ordering, and final operational disposition
