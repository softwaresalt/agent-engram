---
title: "Shipment 117-S post-boundary commit triage"
date: 2026-08-18
doc_type: decision
shipment_id: "117-S"
pull_request: "https://github.com/softwaresalt/agent-engram/pull/342"
rollback_boundary: "22df6ce50f8e89b54f2bf65a9d3917c97dbb0e54"
safety_ref: "safety/117-s-scope-expansion-2f528aff"
safety_tip: "2f528aff6b6f05c0a88a66349f03f3e421c4c2eb"
commit_count: 31
status: completed
---

## Decision

The preserved range
`22df6ce5..safety/117-s-scope-expansion-2f528aff` contains 31 commits.
Only code-graph source discovery, open, read, publication, and deterministic
replacement-race work remains eligible for near-term reintegration into
shipment `117-S`. Adjacent runtime, indexer, configuration, state, write-side,
and metrics changes remain unharvested Stage follow-ups.

This decision is a triage result, not implementation authorization. It does
not modify the active shipment manifest or PR [#342](https://github.com/softwaresalt/agent-engram/pull/342).

## Scope test

A reintegration candidate must be narrowly necessary to close the confirmed
HCL/code-graph source replacement race. Eligible surfaces are the workspace
capability reader, semantic no-follow open, code-graph discovery/open/read and
publication, source traversal/enumeration, deterministic code-graph race tests,
directly coupled dependency changes, and directly coupled HCL closure state.

Daemon IPC/runtime/socket authentication, PID and lock artifacts, endpoint
lifecycle, metrics capacity, hydration/dehydration, registry/config/state,
DAX/ingestion, dedicated indexers, general file tracking, and unrelated
lifecycle work fail this test. Ambiguity is resolved toward `follow-up` or
`mixed-extract-only`.

## Commit-by-commit triage

| SHA | Subject | Files or surfaces | Classification | Rationale | Destination stash |
|---|---|---|---|---|---|
| `d62d7cb252d7` | `fix(security): route lifecycle source reads through capability boundary` | `workspace_source`; file tracker; hydration; retrieval evaluation | `mixed-extract-only` | The capability directory-entry types and `list_directory_blocking` are a direct prerequisite for scoped source enumeration. The lifecycle call-site migrations are expansion. Extract only the `workspace_source` enumeration hunks. | `EE8C4E35` lifecycle source-read boundaries |
| `ca1c86d4925a` | `fix(security): preserve oversized source publication semantics` | `code_graph` Rust prepass and publication | `reintegration-candidate` | Preserves code-graph publication and stale-record handling when a source is oversized. This is directly within code-graph read/publication closure. | - |
| `a4141be7c134` | `fix(security): distinguish content from capability rejection` | `code_graph` prepass rejection state | `reintegration-candidate` | Distinguishes content limits from capability-boundary failures so code-graph closure remains fail-closed without misclassifying publication state. | - |
| `bb7162a8da0c` | `test(security): cover remaining capability read routes` | DAX; file tracker; hydration; ingestion; reactive sync; workspace reader | `follow-up` | Tests unrelated lifecycle and ingestion read routes rather than the confirmed code-graph replacement race. | `EE8C4E35`, `122F86F2` |
| `a2486fd4fab2` | `fix(security): capability-read hydration artifacts` | hydration; workspace reader | `follow-up` | Hydration artifact reads are an explicit out-of-scope lifecycle surface. | `EE8C4E35` |
| `aae50162c9ce` | `fix(security): capability-read reactive content routes` | DAX; ingestion; reactive sync; workspace reader | `follow-up` | Reactive and ingestion routes are adjacent pipelines, not required code-graph closure. | `EE8C4E35`, `122F86F2` |
| `eb31fed98fe1` | `fix(security): capability-read file tracker hashes` | file tracker | `follow-up` | General file tracking is explicitly outside the source replacement closure. | `EE8C4E35` |
| `a9648155452c` | `test(security): bind race barriers to capability roots` | DAX; hydration; ingestion; reactive sync; workspace reader | `follow-up` | These barriers validate out-of-scope lifecycle and ingestion routes, not deterministic code-graph discovery. | `EE8C4E35`, `122F86F2` |
| `f5dc70f08772` | `test(security): cover dedicated and lifecycle source races` | CLI verify; DB workspace; hydration; ingestion; retrieval evaluation | `follow-up` | Dedicated and lifecycle race coverage expands beyond code-graph/HCL source discovery. | `EE8C4E35`, `122F86F2` |
| `1d82f0095c6f` | `fix(security): capability-bind dedicated content indexers` | backlog; ingestion; notebook; PBIP; Power BI indexers | `follow-up` | Dedicated indexers are separate content families and must be staged independently. | `122F86F2` |
| `063ea1419b85` | `fix(security): capability-bind lifecycle metadata reads` | daemon; Cozo schema/query; DB; config; hydration; registry; retrieval; PID; lifecycle tool | `follow-up` | Configuration, registry, metadata, and lifecycle reads are broad state surfaces. The added async directory API is not used by scoped code-graph enumeration. | `7F71CB40` |
| `0170473460c8` | `fix(security): keep capability indexers lint-clean` | backlog; notebook; Power BI indexers | `follow-up` | Cleanup is coupled only to the out-of-scope dedicated indexer migration. | `122F86F2` |
| `178c4d568fb9` | `fix(security): preserve configured PID capability roots` | daemon; PID file | `follow-up` | PID capability roots are daemon runtime authority, not source-read closure. | `80BBDFA3` |
| `da257cc7bd48` | `test(security): cover capability artifact write races` | daemon lock; DB; dehydration; metrics; workspace artifacts; PID | `follow-up` | Write-side artifact races and runtime files require separate cross-platform investigation. | `A4E72E5D`, `80BBDFA3` |
| `7dc76df8edb5` | `fix(security): add capability-rooted artifact writes` | daemon lock; DB; dehydration; metrics; workspace artifacts; PID | `follow-up` | Mutable artifact writes are not needed to close a source-open/read race. | `A4E72E5D`, `80BBDFA3` |
| `b45932767e04` | `fix(security): capability-bind daemon runtime artifacts` | daemon IPC; daemon runtime; PID | `follow-up` | Daemon runtime artifacts are an unrelated lifecycle authority surface. | `80BBDFA3` |
| `f223c2e2154f` | `fix(security): preserve standalone PID cleanup` | daemon; PID cleanup | `follow-up` | PID cleanup behavior is independent of code-graph source containment. | `80BBDFA3` |
| `5a34610f0e29` | `fix(security): preserve daemon lock path identity` | daemon lock | `follow-up` | Lock identity is daemon singleton/runtime work. | `80BBDFA3` |
| `304201c8e98f` | `refactor(security): retain capability root identity` | daemon lock; workspace-reader test identity | `follow-up` | The workspace-reader addition is a test-only accessor coupled to daemon lock tests, not source enumeration. | `80BBDFA3` |
| `eb7a72a55013` | `test(security): cover runtime artifact exclusion races` | daemon IPC/lock; dehydration; metrics; retrieval; workspace artifacts | `follow-up` | Runtime exclusions and mutable artifact races are outside source-read closure. | `A4E72E5D`, `80BBDFA3` |
| `f83750392c8f` | `fix(security): preserve daemon lock inode identity` | daemon lock | `follow-up` | Lock inode identity belongs to daemon runtime authority. | `80BBDFA3` |
| `9e245336978d` | `fix(security): retain runtime artifact capabilities` | daemon; dehydration; metrics; retrieval; workspace artifacts | `follow-up` | Retained runtime/write capabilities are lifecycle architecture beyond the claimed race. | `A4E72E5D`, `80BBDFA3` |
| `f951d5b6725c` | `fix(security): use capability socket permissions` | workspace reader socket permissions | `follow-up` | Socket permissions are daemon runtime namespace work. | `80BBDFA3` |
| `949f81f3addd` | `test(security): cover traversal and runtime namespace races` | code-graph discovery barrier and source race tests; daemon lock/log; metrics; socket test | `mixed-extract-only` | The `code_graph.rs` barrier and `code_graph/source_race_tests.rs` root/ancestor replacement tests are deterministic scoped RED coverage. Daemon, metrics, and socket hunks are expansion. Never cherry-pick wholesale. | `80BBDFA3`, `4F3E2EC3` |
| `67678aea3ac9` | `fix(security): capability-enumerate source discovery` | code-graph traversal; metrics aggregate read limit | `mixed-extract-only` | **Scoped core:** replace ambient `WalkBuilder` traversal with capability-rooted `list_directory_blocking`, classify entries without following links, and preserve localized failure semantics. **Exclude:** the metrics JSONL limit hunk. Apply only after extracting the `d62d7cb2` reader API and the `949f81f3` RED tests. Never cherry-pick wholesale. | `4F3E2EC3` |
| `8ad60f809679` | `fix(security): isolate daemon runtime authority` | Cargo `rustix/process`; daemon runtime module; lock/socket/PID tests | `follow-up` | The manifest change enables process/runtime authority, not code-graph traversal. No Cargo hunk in this commit is directly coupled to scoped reintegration. | `80BBDFA3`, `0F833F6A` |
| `19f12d993cf5` | `test(security): cover authenticated runtime endpoint identity` | daemon IPC/lock; config; metrics | `follow-up` | Authenticated endpoint identity is separate daemon IPC security architecture. | `0F833F6A` |
| `7afd58edbc64` | `fix(security): authenticate daemon endpoint identity` | CLI; daemon IPC/protocol/runtime; DB; shim client/lifecycle; tools; tests | `follow-up` | Broad IPC authentication and lifecycle contracts do not close the source replacement race. | `0F833F6A` |
| `45efaa98a440` | `fix(config): bound metrics channel capacity` | metrics model; config; metrics service | `follow-up` | Metrics capacity is independent resource governance. | `4F3E2EC3` |
| `8f0d27c43523` | `fix(config): enforce metrics capacity invariant` | config validation | `follow-up` | The invariant is coupled only to the out-of-scope metrics capacity change. | `4F3E2EC3` |
| `2f528aff6b6f` | `fix(security): retire endpoint capability before close` | daemon IPC; shim IPC client | `follow-up` | Endpoint close ordering is daemon IPC lifecycle work. | `0F833F6A` |

## Ordered reintegration shortlist

> [!WARNING]
> Candidate status does not authorize implementation. Reapply or cherry-pick
> only after dependency and conflict review, and validate test-first against
> the current `117-S` boundary. Mixed commits must never be cherry-picked
> wholesale.

1. Extract only the `src/services/workspace_source.rs` capability enumeration
   hunks from `d62d7cb252d75544d531247def50d8f115d4207c`: the
   `CapabilityEntryKind` and `CapabilityDirectoryEntry` types plus
   `list_directory_blocking`. Exclude file tracker, hydration, and retrieval
   evaluation changes.
2. Reapply `ca1c86d4925a6be88fbe8aadac83ea7395db1936` only after confirming
   its oversized prepass/publication behavior still matches current tests and
   code-graph state.
3. Reapply `a4141be7c134f491e1d0652a23d9ed3e2ddb5d80` only after reviewing
   its dependency on the prior publication-state change.
4. Extract only `src/services/code_graph.rs` and
   `src/services/code_graph/source_race_tests.rs` hunks from
   `949f81f3adddf12a0e4442d971f023eed27be0a4`. Land the deterministic
   root and ancestor replacement RED tests before the enumeration fix. Exclude
   daemon lock/log, metrics, and socket-test hunks.
5. Extract only `src/services/code_graph.rs` from
   `67678aea3ac914d124b97a9ad7e4e7ccb10a4141`. This is the central
   capability-enumeration implementation. Exclude `src/services/metrics.rs`.

No Cargo manifest or lock change in the range is directly coupled to this
shortlist. The sole manifest hunk, `rustix/process` in `8ad60f80`, belongs to
daemon runtime authority and remains stashed.

## Follow-up clusters

* `EE8C4E35` - lifecycle source-read migration boundaries; next action:
  `deliberate`
* `122F86F2` - dedicated content indexer capability reads; next action:
  `deliberate`
* `7F71CB40` - config, registry, and lifecycle metadata reads; next action:
  `deliberate`
* `A4E72E5D` - capability-rooted mutable artifact writes; next action: `spike`
* `80BBDFA3` - daemon PID, lock, socket, log, and runtime authority; next
  action: `spike`
* `0F833F6A` - authenticated daemon IPC endpoint identity/lifecycle; next
  action: `deliberate`
* `4F3E2EC3` - metrics read/channel capacity invariants; next action:
  `deliberate`

Each entry is unharvested and contains full source commit references, the
`117-S` exclusion rationale, and its mandatory Stage gate.

## Accounting

All 31 commits are represented exactly once as table rows: two pure
reintegration candidates, three mixed extraction-only commits, and 26
follow-ups. Every follow-up or out-of-scope hunk in a mixed commit maps to one
or more of the seven stash clusters. The two pure candidates require no stash
destination.

## Evidence

* [HCL source-read TOCTOU security review](../closure/2026-08-16-hcl-source-read-toctou-security-review.md)
* [PR 342 TOCTOU continuation memory](../memory/2026-08-16/pr-342-toctou-continuation-memory.md)
* Safety ref `safety/117-s-scope-expansion-2f528aff` at
  `2f528aff6b6f05c0a88a66349f03f3e421c4c2eb`
* Scoped rollback boundary
  `22df6ce50f8e89b54f2bf65a9d3917c97dbb0e54`
