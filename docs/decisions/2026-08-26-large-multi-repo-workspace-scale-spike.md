---
title: "Large multi-repo workspace scale feasibility"
type: spike
date: 2026-08-26
time_box: "1h"
conclusion: "pivot"
confidence: "high"
linked_parent_work_item: null
promoted_to: ["none"]
tags:
  - "daemon"
  - "indexing"
  - "multi-repo"
  - "performance"
---

## Goal

Is the current Engram release designed and validated to index more than 5,000
files across as many as 10 repositories and then reliably answer queries?

## Success Criteria

* Identify the intended workspace and daemon isolation model
* Identify hard limits and scale-sensitive indexing behavior
* Compare documented targets with implemented benchmarks
* Explain whether nested Copilot sessions should share or duplicate daemons
* Form a recommendation grounded in current code and live process evidence

## Scope Constraints

Read-only investigation of the current source, tests, durable learnings, and
running local processes. No daemon termination, database mutation, or production
code change was performed.

## Investigation Approach

1. Inspect workspace identity, shim convergence, daemon locking, and IPC readiness
2. Trace source discovery, parsing, persistence, hydration, and query readiness
3. Review scale benchmarks and prior performance incidents
4. Compare the design with the live daemon processes on the development box
5. Assess multi-repo and nested-session behavior separately

## Findings

### What Was Discovered

#### Engram is workspace-local, not a federated multi-repo service

`canonicalize_workspace` requires the supplied root itself to contain valid Git
metadata. The daemon endpoint, lock, PID file, workspace identity, and default
data directory are all scoped to that root. The production daemon constructs
`AppState::new(1)`, so one daemon serves one active workspace.

A directory that merely contains 10 sibling repositories is therefore not one
supported Engram workspace unless that directory is also a valid Git checkout.
If a Git super-root contains nested repositories, source discovery can recurse
through their source trees while skipping hidden `.git` directories, but Engram
does not preserve those nested repositories as independent query scopes.

#### Multiple sessions are supported when they resolve the same root

Each Copilot session starts its own stdio shim. The shim computes the
workspace-specific endpoint, probes an existing daemon, and reuses it when
healthy. A workspace lock prevents two daemons from owning the same workspace.
The lifecycle code also handles simultaneous shim startup by allowing the losing
daemon child to observe the winning daemon's endpoint or PID publication.

Concurrent IPC reads are explicitly tested. Concurrent index requests are
serialized, with one caller receiving `IndexInProgress`.

Starting a nested session should therefore not cause a same-workspace connection
collision. It can still expose the same startup timeout if the shared daemon has
not become ready.

#### Different roots create different daemons and multiply resource cost

The live process tree contained four shims and two daemons:

* One daemon for `C:\Source\GitHub\autoharness`
* One daemon for `C:\Source\GitHub\engram`

Each daemon used approximately 1.2-1.3 GiB of working memory. One daemon had
accumulated sustained CPU time consistent with a long-running startup or
index-related operation. Ten simultaneously active repositories can therefore
multiply both model/database memory and background CPU cost.

The default daemon idle timeout is four hours, so that multiplication can persist
well after the initiating session becomes inactive.

#### The current indexing path has material scale constraints

The full index path:

* Performs recursive, capability-checked discovery and ignore processing
* Runs a global Rust canonicalization prepass before database publication
* Processes files in a serial `for` loop
* Spawns and immediately awaits one blocking parser task per file
* Performs many awaited database writes per file and per symbol
* Generates embeddings in batches scoped to one file

`CodeGraphConfig::parse_concurrency` exists but has no implementation use outside
the configuration model. The current full-index path is therefore not a bounded
parallel parsing pipeline.

The Rust prepass retains source snapshots under a hard aggregate 64 MiB ceiling.
A Rust-heavy workspace whose total retained source exceeds that limit rejects the
global prepass and aborts the index. With 5,000 Rust files, an average file size
near 13 KiB is enough to reach that ceiling.

#### The scale target is aspirational rather than validated

A storage decision estimated that 50,000-100,000 symbols and roughly 150,000
SQLite-backed Cozo writes could hydrate in about 30 seconds. That estimate was a
design assumption.

The planned 5,000-symbol HNSW benchmark was not implemented. Current checked-in
benchmarks cover:

* 500 sequential edge creations with a 60-second limit
* 100 symbols and 50 vector queries with a 30-second limit
* 100 in-memory keyword-search candidates

There is no integration or acceptance test that indexes 5,000 source files,
measures peak memory, waits for daemon readiness, and executes a real query.

Historical evidence is stronger than the benchmark evidence: a prior startup
auto-reindex of 1,382 files consumed more than 14 GiB and caused out-of-memory
failures. Auto-reindex is now disabled by default, which prevents accidental
startup failure but does not establish that manual indexing at 5,000 files is
safe.

#### The observed failure had separate trigger and recovery defects

The installed build was `0.2.0+g18411394-dirty`. Its shim recorded a
`readiness_timeout` after the 30-second startup budget. Two direct daemon-status
attempts returned the same readiness timeout.

The current branch database was approximately 139 MiB. The daemon reached
approximately 1.3 GiB working memory, and database files continued changing for
several minutes after process start. Startup therefore exceeded the shim's
30-second readiness attribution budget.

Direct named-pipe inspection later returned `_health.status = "ready"` from that
same daemon. A newly started stdio shim immediately returned daemon status and
semantic search results. The existing long-lived shim continued returning its
earlier `readiness_timeout`.

The primary incident root cause was therefore a sticky proxy state:

* `compute_startup_outcome` published one degraded result after the deadline
* `ShimHandler` retained that result through a Tokio watch channel
* Every later `tools/call` returned the cached error without probing the daemon
* The same session could not recover after the named-pipe daemon became ready

The startup delay triggered the defect, but it was not the reason the session
remained unusable after the daemon recovered. The historical data did not
identify which database-open phase consumed the time, so the resolution adds
separate timing fields for process-lock wait, file-lock wait, database open,
schema bootstrap, and total connection duration.

The current repository has 2,950 tracked files. The slow startup on this smaller
repository still means a 5,000-file multi-repo expectation is not defensible
without a dedicated scale test and additional performance work.

### What Was Tried and Failed

* `set_workspace` through the current MCP shim timed out after 30 seconds
* `get_daemon_status` through the shim timed out twice with the same classified
  readiness failure
* The expected 5,000-symbol HNSW benchmark file was not present

Repeated status retries through the original shim could not recover because they
read the same cached startup result. Direct named-pipe inspection and a fresh shim
were required to separate current daemon health from stale proxy state.

### Remaining Unknowns

* The exact language and symbol distribution of the proposed 5,000-file corpus
* Peak memory and end-to-end indexing duration for that specific corpus
* Whether the two observed repositories were intentionally active at the same
  time or resulted from the nested-session workflow
* Which database startup phase caused the historical delay; new phase timing
  makes future incidents attributable

## Recommendation

**Conclusion**: pivot
**Confidence**: high

Do not treat one 10-repository, 5,000-file database as a supported deployment
shape for the current version.

Use one Engram workspace and daemon per Git repository, and query repositories
individually. Avoid keeping many repositories active concurrently until daemon
memory is bounded or the embedding/database state is shared safely between
workspaces.

Before claiming 5,000-file support, add a release-mode acceptance benchmark that
indexes a representative mixed-language corpus, records peak RSS and duration,
waits for readiness, and executes representative symbol, graph, and semantic
queries. The implementation also needs bounded parallel parsing, batched
database writes, an explicit multi-repo identity model if federation is desired,
and a larger or streaming replacement for the 64 MiB Rust prepass snapshot.

Increasing `ENGRAM_READY_TIMEOUT_MS` may make a slow startup appear less broken,
but it does not address the memory, serial indexing, hard prepass limit, or
multi-daemon resource multiplication.

## Incident Resolution

Task `136.001-T` hardens the existing proxy architecture rather than replacing
it:

* Readiness expiry is represented as a recoverable intermediate state
* One background monitor probes the workspace endpoint with bounded backoff
* A request can accelerate recovery through a session-wide single-flight probe
* Failed concurrent probes share a cooldown instead of serializing repeatedly
* Agent-facing errors include `recoverable` and `retry_after_ms`
* Admission, endpoint, protocol, shutdown, and other terminal failures remain
  fail-closed
* Session teardown snapshots the state and cancels unresolved startup work
* CozoDB startup emits one structured phase-timing event

The regression harness starts an owned daemon whose readiness is deliberately
delayed beyond the shim budget. It proves that the same stdio process first
returns a retryable timeout, later forwards `get_workspace_status` over the same
named-pipe endpoint, and exits successfully after recovery. A second contract
proves client disconnect cancels unresolved startup work within the teardown
budget.

## Next Steps

* Reproduce with a controlled 5,000-file corpus and capture duration and peak RSS
* Add a query-after-index acceptance test as the release gate for scale claims
* Decide whether multi-repo means isolated per-repo daemons or a federated service
* Profile startup before changing the readiness timeout
* Add explicit daemon inventory and memory guidance for nested sessions

## References

* `src/db/workspace.rs`
* `src/daemon/lockfile.rs`
* `src/daemon/ipc_server.rs`
* `src/shim/lifecycle.rs`
* `src/services/code_graph.rs`
* `src/services/hydration.rs`
* `src/models/config.rs`
* `tests/integration/concurrent_sessions_test.rs`
* `tests/integration/cozo_benchmark_test.rs`
* `tests/integration/performance_test.rs`
* `docs/decisions/2026-04-19-cozo-storage-backend.md`
* `docs/decisions/2026-04-19-cozo-hnsw-benchmark.md`
* `docs/compound/best-practices/auto-reindex-oom-gate-2026-05-09.md`
* `docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md`
* `docs/compound/concurrency-issues/early-hydration-ready-before-heavy-io-2026-05-09.md`
