---
title: "Engram content_record schema incident handoff"
date: 2026-09-05
doc_type: memory
agent: orchestrator
status: blocked
source_workspace: "softwaresalt/autoharness"
target_workspace: "softwaresalt/agent-engram"
---

# Engram `content_record` Schema Incident Handoff

## Purpose

This document is a self-contained incident packet for the Stage and Ship agents
in the `softwaresalt/agent-engram` workspace. It records a blocking Engram
failure observed while restoring an autoharness Stage checkpoint.

The primary defect is a persisted-relation schema mismatch. Three supporting
gaps were also observed:

1. health remains green while content retrieval is unusable;
2. the available maintenance commands do not expose a clear safe repair path;
3. the schema exception is not retained in durable diagnostics.

No Engram data, configuration, or binaries were modified during this
investigation.

## Executive Summary

The running Engram binary expects the persisted `content_record` relation to
contain a `chunk_id` field. The existing autoharness workspace database does not
contain that field.

Both content retrieval paths fail with:

```text
engram_code 5001: stored relation 'content_record' does not have field 'chunk_id'
```

The daemon, workspace binding, registry, file scan, code graph, and symbol
operations remain healthy. The failure is isolated to content/context
retrieval through `query_memory` and `unified_search`.

The likely cause is a missing, skipped, or incomplete forward migration for an
existing workspace database. That is a hypothesis, not a confirmed root cause.
The Engram implementation and migration history must be traced before choosing
a repair.

## Environment

| Field | Observed value |
|---|---|
| Source repository | `softwaresalt/autoharness` |
| Workspace path | `C:\Source\GitHub\autoharness` |
| Branch | `main` |
| Engram binary | `engram 0.3.0-rc.1+ge043299` |
| Engram workspace marker | `.engram/.version` contains `5.0.0` |
| Workspace database | `.engram/cozo/main/engram.db` |
| Absolute database path | `C:\Source\GitHub\autoharness\.engram\cozo\main\engram.db` |
| Registry sources | `src`, `tests`, `docs`, `.github`, `.backlogit`, `schemas` |
| Workspace freshness | `stale_files: false` |
| Last completed scan | 27,714 of 27,714 files |

The difference between binary version `0.3.0-rc.1` and workspace marker
`5.0.0` may be intentional because they may represent different version
domains. It is included because migration selection may depend on one or both
values. Do not assume that the difference itself is the defect without tracing
the version contracts.

## Blocking Defect

### Actual behavior

The two content retrieval tool families fail:

| CLI surface | MCP/tool equivalent | Result |
|---|---|---|
| `engram query-memory` | `query_memory` | Fails with code 5001 |
| `engram search` | `unified_search` | Fails with code 5001 |

The exact normalized error is:

```text
stored relation 'content_record' does not have field 'chunk_id'
```

The failure occurred through both read paths. It is therefore not explained by
one malformed query string or one command adapter.

Engram telemetry observed before this handoff recorded:

| Tool | Calls | Results |
|---|---:|---:|
| `query_memory` | 2 | 0 |
| `unified_search` | 9 | 0 |

Do not repeatedly rerun these commands merely to reproduce the failure. The
same-operation failure threshold has already been exceeded in accumulated
telemetry. Inspect the persisted relation and migration code first. If a new
bounded reproduction is necessary after a fix, run one query per affected
surface and preserve the native result.

### Expected behavior

When the current binary opens an existing workspace database, one of these
outcomes should occur:

1. Engram automatically performs an idempotent forward migration that adds or
   reconstructs the required `chunk_id` data; or
2. Engram refuses to serve affected tools and reports a specific,
   operator-actionable migration-required state.

After a successful migration or rebuild, both `query_memory` and
`unified_search` should return valid results.

### Working subsystems

The following observations isolate the failure from general daemon or indexing
availability:

| Subsystem | Observed state |
|---|---|
| Binary startup | Working |
| Daemon PID/liveness | Green |
| IPC endpoint | Green and serving requests |
| Workspace binding | Correctly bound to the autoharness root |
| Workspace registry | Loaded and valid |
| Offline change scan | No offline changes detected |
| Incremental sync | Completes successfully |
| Workspace scan | Complete; not stale |
| Code graph | Populated |
| Symbol operations | Working |
| Content/context retrieval | Broken |

`engram workspace-status` reported:

```text
code_files: 202
functions: 1025
classes: 359
interfaces: 0
edges: 21163
files_scanned: 27714
files_total: 27714
stale_files: false
```

`engram stats` reported 1,384 of 1,384 symbols embedded with 100 percent
coverage.

### Root-cause hypothesis

The strongest current hypothesis is:

1. an older Engram version created `content_record` without `chunk_id`;
2. the current retrieval implementation reads `chunk_id` unconditionally;
3. startup or sync did not migrate the existing relation;
4. the daemon declared itself healthy because health checks do not validate the
   relation shape or execute a bounded content read.

Questions the Engram investigation should answer:

1. Which release or schema generation introduced `content_record.chunk_id`?
2. What persisted relation shapes existed before and after that change?
3. Is a forward migration registered for every supported predecessor shape?
4. Is migration selection driven by the binary version, `.engram/.version`,
   database metadata, relation introspection, or some combination?
5. Can migration report success while leaving the old relation in place?
6. Can sync bypass migration because file hashes are unchanged?
7. Are context records stored only in `content_record`, or can they be
   reconstructed safely from source documents and other relations?

## Supporting Issue 1: False-Green Health

`engram daemon-status` returned `overall: green` with all checks green:

- binary version;
- PID liveness;
- workspace identity;
- pipe reachability;
- registry validity;
- offline scan;
- session resume;
- telemetry health.

At the same time, both required content retrieval APIs failed deterministically
with a stored-schema error.

The health contract therefore does not currently prove that persisted
relations satisfy the running query schema.

### Expected health behavior

A missing field required by a registered read tool should cause one of these
results:

- daemon startup fails with a migration-required diagnostic;
- the affected subsystem is marked red or degraded;
- overall health is non-green while required retrieval tools cannot execute.

A bounded relation-schema validation is preferable to repeatedly running a
semantic query as a health probe.

## Supporting Issue 2: Conflicting Embedding Metrics

Two diagnostics from the same running workspace disagree:

| Diagnostic | Symbols with embeddings | Total symbols | Coverage |
|---|---:|---:|---:|
| `engram stats` | 1,384 | 1,384 | 100% |
| `engram health` | 0 | 0 | 0% |

This may be a separate metrics-source or aggregation defect. At minimum, it
makes incident diagnosis ambiguous. The Engram agents should determine whether
the two commands intentionally measure different populations. If so, the output
must name those populations rather than presenting both as generic symbol
embedding status.

## Supporting Issue 3: No Clear Safe Repair Command

The discoverable maintenance surfaces do not clearly repair this state:

| Command | Contract relevant to this incident |
|---|---|
| `engram sync` | Incrementally synchronizes files into the code graph |
| `engram sync --full --force` | Re-parses and re-embeds discovered files, but does not document a `content_record` schema migration guarantee |
| `engram reinstall` | Reinstalls runtime artifacts while preserving existing workspace data, including the incompatible database |
| `engram migrate-down` | Only exposes the unrelated `calls-resolution` rollback target |

A normal sync completed before the incident was escalated, but content reads
continued to fail. Reinstall alone is unlikely to help because it explicitly
preserves the broken persisted data.

Engram needs a supported recovery contract that:

1. detects the incompatible relation shape;
2. creates a bounded backup or otherwise preserves recoverable state;
3. migrates or reconstructs `content_record`;
4. is idempotent and interruption-safe;
5. verifies both content query paths before reporting success;
6. explains whether source re-indexing can reconstruct all lost fields.

## Supporting Issue 4: Missing Durable Diagnostic Evidence

At inspection time:

- `.engram/diagnostics/` contained no incident artifact;
- `.engram/logs/` listed only `shim-startup-failures.jsonl`;
- health telemetry recorded zero-result calls but did not surface the schema
  exception.

A persistent relation-contract failure should produce a bounded diagnostic
record containing:

- the relation name;
- missing field;
- expected schema generation;
- observed schema generation or field list;
- binary version;
- workspace/data version;
- recommended repair command;
- whether the affected data can be rebuilt.

The record must not expose document contents or other sensitive payload data.

## Autoharness Operational Impact

The autoharness workspace has the `agent-engram` capability pack installed.
Checkpoint recovery therefore uses this sequence:

```text
restore -> prune/gate -> resume -> resolve checkpoint
```

The prune/gate step must read the restored checkpoint and bound Engram context,
while preserving:

- the active shipment/task cursor;
- the unresolved checkpoint pointer;
- recorded gate verdicts.

When Engram is installed but its bound content substrate is unavailable, the
autoharness policy fails closed. It forbids:

- file-based substitute pruning;
- resume without pruning;
- resolving the checkpoint before successful resume.

The affected checkpoint is:

| Field | Value |
|---|---|
| Filename | `checkpoint-20260904-220151.json` |
| Owner | `stage` |
| Phase | `publication-blocked` |
| Shipment | `159-S` |
| Feature | `151-F` |
| Current status | `active` |

Stage independently proved that the checkpoint is stale:

- its two recorded commits are ancestors of `origin/main`;
- PR #432 published the recorded artifacts;
- post-merge closure is present on `main`;
- shipment `159-S` is intact, queued, and unclaimed;
- the source stash was archived and was not re-harvested.

Despite that proof, the checkpoint cannot be resolved under the current policy
because Engram content cannot be read for the mandatory prune-on-restore gate.
This blocks Orchestrator from claiming `159-S`.

## Investigation Boundaries

The Engram Stage agent should frame and plan the defect before implementation.
The Engram Ship agent should implement only after the work is harvested and
queued under that repository's normal workflow.

During investigation:

- do not hand-edit `.engram/` database or registry artifacts;
- do not delete or replace the autoharness workspace database without an
  explicit, backed-up recovery plan;
- do not assume `reinstall` repairs persisted data;
- do not treat a successful code-graph sync as proof that content retrieval
  works;
- do not repeatedly invoke the already-failing query operations;
- preserve the original database as a migration regression fixture if its
  contents can be handled safely.

## Suggested Investigation Sequence

1. Locate the code that declares and queries `content_record`.
2. Identify the commit and schema generation that introduced `chunk_id`.
3. Trace startup migration registration and version-selection logic.
4. Compare the expected relation fields with a safely inspected copy of the
   affected database.
5. Determine whether existing records can derive `chunk_id` deterministically.
6. Add a pre-change fixture representing the old relation shape.
7. Write a failing regression test that opens or migrates that fixture.
8. Implement the smallest safe forward migration or rebuild path.
9. Add schema validation to daemon health/startup.
10. Reconcile the `health` and `stats` embedding metrics.
11. Add bounded diagnostic reporting for migration/schema failures.
12. Validate the repaired database with one `query-memory` call and one
    `search` call.

## Acceptance Criteria

1. Opening a pre-`chunk_id` workspace database either upgrades it safely or
   returns an explicit migration-required error before serving retrieval tools.
2. The migration or rebuild is idempotent.
3. An interrupted migration cannot silently leave a partially upgraded
   relation.
4. Existing recoverable context records are preserved, or documented rebuild
   semantics are proven complete.
5. `query-memory` succeeds after repair.
6. `search` succeeds after repair.
7. Neither command emits Engram code 5001 for the repaired database.
8. `daemon-status` is non-green when a required persisted relation is
   incompatible with the running binary.
9. `health` and `stats` report consistent embedding populations and totals, or
   clearly name intentionally different populations.
10. The operator has a documented CLI recovery path that does not require
    hand-editing tool-managed data.
11. A regression test covers an existing database created before
    `content_record.chunk_id`.
12. A bounded diagnostic artifact records schema migration failures without
    exposing indexed content.

## Completion Signal Needed by Autoharness

After Engram is repaired in the autoharness workspace, the following evidence
is sufficient to retry checkpoint recovery:

1. daemon and workspace status are healthy;
2. workspace binding points to `C:\Source\GitHub\autoharness`;
3. the workspace is not stale;
4. one bounded `query-memory` probe succeeds;
5. one bounded `search` probe succeeds;
6. the repair method and resulting schema version are recorded.

Autoharness Stage can then retry the owner-exclusive recovery, complete the
prune-on-restore gate, resolve
`checkpoint-20260904-220151.json`, and allow Orchestrator to proceed toward
claiming `159-S`.
