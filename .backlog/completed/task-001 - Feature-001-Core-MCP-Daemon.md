---
id: TASK-001
title: '001: Core MCP Daemon'
status: Done
type: feature
assignee: []
created_date: '2026-02-05'
labels:
  - feature
  - '001'
  - mcp
  - daemon
  - state-management
  - persistence
milestone: m-0
dependencies: []
references:
  - specs/001-core-mcp-daemon/spec.md
  - src/bin/engram.rs
  - src/config/mod.rs
  - src/db/mod.rs
  - src/db/schema.rs
  - src/db/workspace.rs
  - src/server/router.rs
  - src/server/sse.rs
  - src/server/mcp.rs
  - src/server/state.rs
  - src/errors/mod.rs
  - src/errors/codes.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
# Feature Specification: engram Core MCP Daemon

**Feature Branch**: `001-core-mcp-daemon`  
**Created**: 2026-02-05  
**Status**: Draft  
**Input**: Implement engram v0 core MCP daemon: a high-performance local-first state engine serving as the shared brain for software development environments with SurrealDB backend, SSE transport, workspace isolation, and git-backed persistence


## Clarifications

### Session 2026-02-09

- Q: What is the maximum number of concurrent workspaces per daemon? → A: Configurable upper bound with default of 10 (matches FR-002 client limit)
- Q: What is the default conflict strategy for concurrent external edits to `.engram/` files? → A: Default warn (emit stale-workspace warning, proceed with in-memory state); configurable to rehydrate or fail

### Session 2026-02-12

- Q: How are new tasks introduced into a workspace? → A: Dedicated `create_task` MCP tool (separate from `update_task`); tool budget expands beyond 10 to accommodate
- Q: How should keyword scores be normalized before combining with vector similarity in hybrid search? → A: Min-max normalization (scale keyword scores to [0, 1] per query result set) before applying 0.7/0.3 weights
- Q: Which memory metric should `get_daemon_status` report and SC-006 validate? → A: RSS (Resident Set Size) via `sysinfo` crate
- Q: What test corpus and relevance definition should SC-010 use? → A: 10 queries with 3 expected-result IDs each; relevant = expected document appears in top-5 results (precision@5)
- Q: What defines a "large" database and what degradation is acceptable? → A: Large = >10K tasks; acceptable = up to 3× baseline latency for all operations

## Requirements *(mandatory)*

### Functional Requirements

**Connection & Lifecycle:**

* **FR-001**: System MUST start as a daemon binding to `127.0.0.1` on a configurable port
* **FR-002**: System MUST accept multiple simultaneous SSE connections (minimum 10 concurrent)
* **FR-003**: System MUST assign unique connection IDs (UUID v4) to each client
* **FR-004**: System MUST implement 15-second keepalive pings on SSE connections
* **FR-005**: System MUST timeout inactive connections after 60 seconds (configurable)
* **FR-006**: System MUST flush all active workspaces on graceful shutdown (SIGTERM/SIGINT)

**Workspace Management:**

* **FR-007**: System MUST validate workspace paths as existing directories with `.git/` subdirectory
* **FR-008**: System MUST reject paths containing `..` after canonicalization (path traversal prevention)
* **FR-009**: System MUST map each workspace to an isolated SurrealDB database via deterministic path hash
* **FR-009a**: System MUST enforce a configurable maximum number of concurrent active workspaces (default: 10); exceeding the limit returns an error prompting the client to release an existing workspace
* **FR-010**: System MUST hydrate workspace state from `.engram/` files on first access
* **FR-011**: System MUST dehydrate workspace state to `.engram/` files on `flush_state` call
* **FR-012**: System MUST preserve user comments in `tasks.md` during dehydration using structured diff merge (via `similar` crate)
* **FR-012a**: System MUST detect external modifications to `.engram/` files (via mtime or content hash) before flush or hydrate operations
* **FR-012b**: System MUST default to warn-and-proceed when stale files are detected (emit error 2004 StaleWorkspace as warning, continue with in-memory state); behavior MUST be configurable to `rehydrate` or `fail`

**Task Operations:**

* **FR-013**: System MUST support task status values: `todo`, `in_progress`, `done`, `blocked`
* **FR-013a**: System MUST provide a `create_task` MCP tool that accepts title, description, and optional parent task ID; new tasks default to `todo` status with generated UUID
* **FR-014**: System MUST automatically update `updated_at` timestamp on task modifications
* **FR-015**: System MUST append context notes on task updates (never overwrite existing context)
* **FR-016**: System MUST detect cyclic dependencies when adding task relationships
* **FR-017**: System MUST support linking tasks to external work item IDs (reference storage only)

**Memory & Search:**

* **FR-018**: System MUST generate embeddings using `all-MiniLM-L6-v2` model (384 dimensions)
* **FR-019**: System MUST perform hybrid search combining vector similarity (0.7 weight) and keyword matching (0.3 weight); keyword scores MUST be min-max normalized to [0, 1] per result set before weighting
* **FR-020**: System MUST lazily download embedding model on first query if not cached
* **FR-021**: System MUST operate offline if model exists in local cache

**Observability:**

* **FR-022**: System MUST expose daemon status via `get_daemon_status()` tool (version, uptime, RSS memory usage via `sysinfo` crate)
* **FR-023**: System MUST log all operations with structured tracing and correlation IDs
* **FR-024**: System MUST return structured error responses with numeric codes per error taxonomy
* **FR-025**: System MUST implement connection rate limiting to prevent resource exhaustion (error 5003 RateLimited); threshold: maximum 20 new connections per 60-second sliding window per source IP
* **FR-026**: System MUST expose an HTTP GET `/health` endpoint returning daemon status and active workspace count (per constitution VII)

### Key Entities

* **Spec**: High-level requirement captured from specification files. Attributes: title, content, embedding, file_path, timestamps.
* **Task**: Unit of work derived from specs. Attributes: title, status, work_item_id (optional), description, context_summary, timestamps.
* **Context**: Ephemeral knowledge captured during execution. Attributes: content, embedding, source_client, created_at.
* **depends_on**: Graph edge representing task dependencies. Attributes: type (hard_blocker, soft_dependency).
* **implements**: Graph edge linking Task to Spec for traceability.
* **relates_to**: Graph edge linking Task to Context for memory association.

## Success Criteria *(mandatory)*

### Measurable Outcomes

* **SC-001**: Daemon cold start completes in under 200ms to accepting connections
* **SC-002**: Workspace hydration completes in under 500ms for projects with fewer than 1000 tasks
* **SC-003**: `query_memory` hybrid search returns results in under 50ms
* **SC-004**: `update_task` write operations complete in under 10ms
* **SC-005**: `flush_state` completes in under 1 second for full workspace dehydration
* **SC-006**: Daemon consumes less than 100MB RSS when idle with no active workspaces
* **SC-007**: Daemon handles 10 simultaneous client connections without request failures
* **SC-008**: Round-trip serialization (hydrate → modify → dehydrate → hydrate) preserves 100% of user comments in markdown files
* **SC-009**: All MCP tool errors return structured responses with appropriate error codes (no internal errors exposed)
* **SC-010**: 95% of `query_memory` results are relevant to the query, evaluated against a test corpus of 10 queries with 3 expected-result IDs each; relevant = expected document appears in top-5 results (precision@5)

## Assumptions

* Target platform is local developer workstations (Windows, macOS, Linux)
* Users have Git installed and workspaces are Git repositories
* Network access available for initial model download; subsequent operation can be offline
* Workspaces are on local filesystems (not network shares)
* Single user per daemon instance (no multi-user authentication required for localhost)

## Out of Scope (v0)

* Bidirectional sync with external work item trackers (ADO, GitHub Issues)
* Multi-user authentication/authorization
* Remote daemon access (always localhost)
* Real-time file watching for `.engram/` changes
* Web UI or dashboard
* Workspace archival/cleanup utilities
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Daemon cold start completes in under 200ms to accepting connections (SC-001)
- [x] #2 Workspace hydration completes in under 500ms for projects with fewer than 1000 tasks (SC-002)
- [x] #3 query_memory hybrid search returns results in under 50ms (SC-003)
- [x] #4 update_task write operations complete in under 10ms (SC-004)
- [x] #5 flush_state completes in under 1 second for full workspace dehydration (SC-005)
- [x] #6 Daemon consumes less than 100MB RSS when idle with no active workspaces (SC-006)
- [x] #7 Daemon handles 10 simultaneous client connections without request failures (SC-007)
- [x] #8 Round-trip serialization preserves 100% of user comments in markdown files (SC-008)
- [x] #9 All MCP tool errors return structured responses with appropriate error codes (SC-009)
- [x] #10 95% of query_memory results are relevant at precision@5 (SC-010)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
### Full Spec

# Full Specification Quality Checklist: engram Core MCP Daemon

**Purpose**: Deep requirements-quality validation across all 5 user stories, functional requirements, success criteria, edge cases, and cross-cutting concerns. Designed for consumption by automated agent development workflows.
**Created**: 2026-02-12
**Feature**: [spec.md](../spec.md) | [plan.md](../plan.md) | [tasks.md](../tasks.md) | [data-model.md](../data-model.md)
**Depth**: Deep (exhaustive, release-gate rigor)
**Audience**: Automated agent workflows

---

## Requirement Completeness

- [ ] CHK001 - Are all valid `TaskStatus` transitions explicitly enumerated in spec.md, or does the spec only list the four status values without a transition matrix? [Completeness, Gap — transition rules appear only in data-model.md §Task State Transitions, not in spec.md FR-013]
- [ ] CHK002 - Are requirements defined for what happens when `create_task` is called with a `parent_task_id` that does not exist? [Completeness, Gap — FR-013a specifies optional parent but no error for invalid parent]
- [ ] CHK003 - Are requirements defined for maximum task `description` length, or is it intentionally unbounded? [Completeness, Gap — data-model.md says "may be empty string but not null" but specifies no upper bound]
- [ ] CHK004 - Are requirements defined for the `context_summary` field lifecycle — who populates it, when, and under what conditions? [Completeness, Gap — field exists in data-model.md but no FR describes generation or update]
- [ ] CHK005 - Are requirements defined for what `get_task_graph` returns when `root_task_id` does not exist? [Completeness, Gap — US2 scenario 4 assumes task exists; error path unspecified]
- [ ] CHK006 - Are requirements defined for what `check_status` returns when a `work_item_id` has no matching task? [Completeness, Gap — US2 scenario implies lookup but no empty-result behavior specified]
- [ ] CHK007 - Are requirements defined for the `implements` and `relates_to` graph edges — which MCP tools create them? [Completeness, Gap — data-model.md defines the edges but no FR or tool creates `implements` or `relates_to` edges explicitly]
- [ ] CHK008 - Are requirements defined for deleting or archiving tasks, specs, or context nodes? [Completeness, Gap — only CRUD create/update paths exist; no delete/archive FR]
- [ ] CHK009 - Are requirements defined for `source_client` validation on Context creation — what values are valid? [Completeness, Gap — data-model.md says "alphanumeric + underscore" but no FR enforces this]
- [ ] CHK010 - Are requirements defined for how the daemon determines its own `version` string reported by `get_daemon_status`? [Completeness, Gap — FR-022 says "version" but source of truth (Cargo.toml, hardcoded, env var) is unspecified]
- [ ] CHK011 - Are requirements defined for what files `set_workspace` creates when initializing a new `.engram/` directory? [Completeness, Gap — US3 scenario 3 says "initializes empty workspace structure" but doesn't enumerate which files]
- [ ] CHK012 - Are requirements defined for `.engram/.version` file handling — migration behavior when version mismatch is detected? [Completeness, Gap — data-model.md §Schema Migration describes forward migration but spec.md has no FR for it]
- [ ] CHK013 - Are requirements defined for `max_workspaces` behavior when the same workspace is re-bound by a different client — does it count as one or two? [Completeness, Gap — FR-009a says "concurrent active workspaces" but doesn't define counting semantics]
- [ ] CHK014 - Is the `work_item_id` format validation requirement (ADO `AB#\d+` or GitHub `[\w-]+/[\w-]+#\d+`) specified in spec.md, or only in data-model.md? [Completeness, Gap — FR-017 says "reference storage only" without format validation; data-model.md adds regex constraint]
- [ ] CHK015 - Are requirements defined for what happens when a workspace path is valid but the `.engram/` directory contains unparseable files? [Completeness, Gap — US3 scenario 4 covers corrupted DB but not corrupted `.engram/` files]

## Requirement Clarity

- [ ] CHK016 - Is "inactive connection" in FR-005 defined — does it mean no SSE events received, no MCP calls made, or no TCP activity? [Clarity, Spec §FR-005]
- [ ] CHK017 - Is "structured tracing and correlation IDs" in FR-023 quantified — what constitutes a correlation ID (per-request UUID? per-session?) and which spans are required? [Clarity, Spec §FR-023]
- [ ] CHK018 - Is "deterministic path hash" in FR-009 specified with enough detail — hash algorithm (SHA-256), input (canonical path string), output format (hex)? [Clarity, Spec §FR-009 — algorithm named in plan.md/data-model.md but not in spec.md itself]
- [ ] CHK019 - Is "configurable" in FR-001 (port), FR-005 (timeout), FR-009a (max workspaces) defined with configuration precedence — CLI > env var > config file > default? [Clarity, Spec §FR-001/005/009a — precedence unstated]
- [ ] CHK020 - Is "minimum 10 concurrent" SSE connections in FR-002 a hard floor or a soft target — does it mean exactly 10, at least 10, or configurable with 10 as default? [Clarity, Spec §FR-002]
- [ ] CHK021 - Is the 0.7/0.3 weighting in FR-019 configurable or hardcoded — and is min-max normalization defined with sufficient precision (per-query or global)? [Clarity, Spec §FR-019 — clarification says "per query result set" but FR text is ambiguous]
- [ ] CHK022 - Does "append context notes on task updates" in FR-015 apply to all write tools (`update_task`, `add_blocker`, `create_task`) or only `update_task`? [Clarity, Spec §FR-015]
- [ ] CHK023 - Is "lazy download" in FR-020 defined with error behavior — what error code when download fails due to network issues? [Clarity, Spec §FR-020 — no error code specified for download failure]
- [ ] CHK024 - Is the rate limiting "sliding window" in FR-025 specified as a precise algorithm (e.g., token bucket, fixed window, sliding log)? [Clarity, Spec §FR-025]
- [ ] CHK025 - Is "structured diff merge" in FR-012 defined with conflict resolution behavior — what happens when structural conflicts prevent clean merge? [Clarity, Spec §FR-012 — no fallback defined for merge failure]

## Requirement Consistency

- [ ] CHK026 - Do task title length constraints align between spec.md (FR-013a: unspecified), data-model.md (max 200 chars), and mcp-tools.json (`maxLength: 200`)? [Consistency — spec.md lacks the 200-char limit that data-model.md and contracts define]
- [ ] CHK027 - Does the Spec title max length (500 chars in data-model.md) have a corresponding FR in spec.md? [Consistency, Gap — data-model.md §Spec says max 500 but no FR validates it]
- [ ] CHK028 - Do the `create_task` error codes align between mcp-tools.json (`[1003, 3001, 3003]`) and the error taxonomy in error-codes.md — is 3005 (`TaskTitleEmpty`) missing from the contract? [Consistency — tasks.md T132 adds error 3005 but mcp-tools.json lists `[1003, 3001, 3003]`]
- [ ] CHK029 - Does the `done → todo` "reopen" transition in data-model.md align with spec.md acceptance scenarios — is reopening covered by any user story? [Consistency, Gap — data-model.md allows `done → todo` but no acceptance scenario exercises it]
- [ ] CHK030 - Does the edge case "up to 3× baseline latency" for >10K tasks align with success criteria SC-001 through SC-005 — are degraded-mode targets specified? [Consistency — edge case and clarification define "3×" but SC items only specify ideal targets]
- [ ] CHK031 - Does FR-012a (stale detection via "mtime or content hash") align with the implementation — is only one method required, or both? [Consistency, Spec §FR-012a — "or" is ambiguous; data-model.md §Workspace Metadata only lists `file_mtimes`]
- [ ] CHK032 - Are the `flush_state` output `warnings` array (mcp-tools.json) and the stale strategy `warn` mode (FR-012b) consistently defined — is the warning format specified? [Consistency — stale warnings appear in flush output but no schema for warning objects]
- [ ] CHK033 - Does FR-006 "flush all active workspaces on graceful shutdown" align with US5 scenario 4 "in-memory state is preserved" — are these contradictory for non-shutdown disconnects? [Consistency, Spec §FR-006 vs US5§4]

## Acceptance Criteria Quality

- [ ] CHK034 - Can SC-001 (<200ms cold start) be measured deterministically given variability in SurrealDB initialization time? [Measurability, Spec §SC-001]
- [ ] CHK035 - Can SC-003 (<50ms hybrid search) be measured without the embedding model load time — is it first-query or steady-state? [Measurability, Spec §SC-003]
- [ ] CHK036 - Can SC-008 (100% comment preservation) be tested with a finite set of edge cases — are representative comment patterns defined (inline, block, nested, adjacent to frontmatter)? [Measurability, Spec §SC-008]
- [ ] CHK037 - Can SC-010 (95% relevance) be independently reproduced — is the test corpus of 10 queries with 3 expected-result IDs published as a test fixture? [Measurability, Spec §SC-010]
- [ ] CHK038 - Is SC-007 (10 simultaneous clients) testable without specifying the operation mix — should the 10 clients perform reads, writes, or both? [Measurability, Spec §SC-007]
- [ ] CHK039 - Are acceptance scenarios for US1-US5 written as verifiable Given-When-Then with measurable outcomes, or do any rely on subjective assessment? [Acceptance Criteria — all scenarios use GWT format ✓, but US5§1 "within 50ms" may need tolerance]
- [ ] CHK040 - Is SC-006 (<100MB RSS idle) measured at a specific point — after startup, after one workspace cycle, or steady-state? [Measurability, Spec §SC-006]

## Scenario Coverage

- [ ] CHK041 - Are error scenarios defined for all 11 MCP tools when workspace is not bound (error 1003)? [Coverage — only `get_daemon_status` is workspace-independent; spec should confirm 1003 for all others]
- [ ] CHK042 - Are requirements defined for the `add_blocker` tool when the task is already `blocked` — is it a no-op, additional blocker, or error? [Coverage, Gap — US2§3 assumes non-blocked task; re-block path unspecified]
- [ ] CHK043 - Are requirements defined for `update_task` when the task is already in the target status — is same-status update allowed? [Coverage, Gap — idempotency annotation says "no-op for status" but no FR specifies this]
- [ ] CHK044 - Are requirements defined for concurrent `set_workspace` calls to the same path from different connections — second call hydration behavior? [Coverage, Gap — FR-010 says "first access" but doesn't address same-workspace re-binding]
- [ ] CHK045 - Are requirements defined for what `flush_state` does when the workspace has no tasks and no context — empty `.engram/` files? [Coverage, Gap — US3 covers populated workspace; empty-state flush unspecified]
- [ ] CHK046 - Are requirements defined for `query_memory` when the workspace has zero searchable documents (no specs, tasks, or context)? [Coverage, Gap — US4 assumes populated workspace]
- [ ] CHK047 - Are requirements defined for the daemon receiving MCP calls before any SSE connection is established (HTTP POST to `/mcp` without SSE)? [Coverage, Gap — protocol flow assumes SSE first]
- [ ] CHK048 - Are requirements defined for `register_decision` uniqueness — can two decisions with the same topic be registered? [Coverage, Gap — idempotency annotation says non-idempotent but no dedup requirement exists]

## Edge Case Coverage

- [ ] CHK049 - Are requirements defined for workspace paths at OS max path length (260 chars Windows, 4096 Linux)? [Edge Case, Gap]
- [ ] CHK050 - Are requirements defined for task title containing special characters (newlines, null bytes, Unicode, markdown syntax)? [Edge Case, Gap — data-model.md says max 200 chars but no character set restriction]
- [ ] CHK051 - Are requirements defined for `.engram/tasks.md` containing tasks that were deleted from the database but still exist in the file? [Edge Case, Gap — hydration behavior for orphan file entries unspecified]
- [ ] CHK052 - Are requirements defined for `flush_state` when the filesystem is read-only or disk is full? [Edge Case, Gap — error code 5001/5002 exist but no FR describes the behavior]
- [ ] CHK053 - Are requirements defined for SSE connection behavior when the daemon is under memory pressure (approaching 500MB limit)? [Edge Case, Gap — SC-006 sets idle limit but load ceiling behavior unspecified]
- [ ] CHK054 - Are requirements defined for hydration when `.engram/graph.surql` references task IDs not present in `.engram/tasks.md`? [Edge Case, Gap — orphan edge handling unspecified]
- [ ] CHK055 - Are requirements defined for `set_workspace` with a valid Git repo that has an empty `.git/` directory (bare repo or fresh `git init`)? [Edge Case, Gap — FR-007 requires `.git/` subdirectory but doesn't define minimum Git state]
- [ ] CHK056 - Are requirements defined for the embedding model file being corrupted or truncated in the cache directory? [Edge Case, Gap — FR-020/021 address download and offline use but not cache corruption]
- [ ] CHK057 - Are requirements defined for `get_task_graph` with `depth` set to 0 or negative values? [Edge Case — mcp-tools.json sets `minimum: 1` but spec.md has no depth validation FR]
- [ ] CHK058 - Are requirements defined for clock skew affecting `updated_at` timestamps in last-write-wins conflict resolution? [Edge Case, Gap — US5§2 uses timestamps for ordering but doesn't address clock issues]

## Non-Functional Requirements

- [ ] CHK059 - Are platform-specific requirements defined for Windows path handling (drive letters, UNC paths, case-insensitivity)? [NFR, Gap — Assumptions say "Windows, macOS, Linux" but no path normalization FR]
- [ ] CHK060 - Are log rotation or log size limits specified for structured tracing output? [NFR, Gap — FR-023 requires structured tracing but no storage management]
- [ ] CHK061 - Are startup failure modes specified — what does the daemon do when the configured port is already in use? [NFR, Gap — FR-001 says "configurable port" but no error for bind failure]
- [ ] CHK062 - Are resource cleanup requirements defined for abandoned SurrealDB databases when workspaces are removed? [NFR, Gap — data_dir accumulates DB files but no cleanup FR]
- [ ] CHK063 - Is the `/health` endpoint response schema defined in mcp-tools.json or spec.md? [NFR, Gap — FR-026 says "returning daemon status and active workspace count" but no response schema]
- [ ] CHK064 - Are graceful degradation requirements defined for when `sysinfo` crate cannot determine RSS (e.g., on some Linux containers)? [NFR, Gap — FR-022 mandates RSS but no fallback]

## Dependencies and Assumptions

- [ ] CHK065 - Is the assumption "single user per daemon instance" validated against US5 (multi-client) — are multiple clients from different OS users in scope? [Assumption, Spec §Assumptions]
- [ ] CHK066 - Is the assumption "workspaces are on local filesystems" enforceable — does the daemon detect and reject network paths, or silently degrade? [Assumption, Gap — Edge Cases mention "not officially supported" but no detection mechanism]
- [ ] CHK067 - Is the `mcp-sdk 0.0.3` dependency stable enough for a v0 release — is there a pinning or compatibility strategy? [Dependency — plan.md lists 0.0.3; pre-1.0 semver allows breaking changes]
- [ ] CHK068 - Is the `fastembed 3` TLS/`ort-sys` issue documented in spec.md or plan.md as a known risk with a mitigation plan? [Dependency — copilot-instructions.md mentions the issue; not reflected in spec]
- [ ] CHK069 - Is the SurrealDB 2 embedded backend (`surrealkv`) documented as a hard dependency with minimum version requirements? [Dependency — plan.md says "SurrealDB 2" but no minimum patch version]

## Ambiguities and Conflicts

- [ ] CHK070 - Is the FR-012b stale warning delivery mechanism specified — does the warning reach the client via MCP tool response, separate SSE event, or only server logs? [Ambiguity, Spec §FR-012b]
- [ ] CHK071 - Does "idempotent" in mcp-tools.json for `update_task` conflict with FR-015 "MUST append context notes on task updates" — appending notes makes it non-idempotent for context even when status is unchanged? [Conflict, Spec §FR-015 vs mcp-tools.json]
- [ ] CHK072 - Is the `Out of Scope` section binding — does "no real-time file watching" mean the daemon must NOT watch files, or simply that it is not required to? [Ambiguity, Spec §Out of Scope]
- [ ] CHK073 - Does "reference storage only" in FR-017 mean `work_item_id` is never validated against external systems, or that it's validated on format only? [Ambiguity, Spec §FR-017 vs data-model.md regex validation]
- [ ] CHK074 - Is the `query_memory` tool's `type` field in the output schema (`"enum": ["spec", "context"]`) missing "task" — since tasks are included as search candidates per implementation? [Conflict — mcp-tools.json output enum is `["spec", "context"]` but tasks are searched too]

## Traceability

- [ ] CHK075 - Does every functional requirement (FR-001 through FR-026) have at least one acceptance scenario exercising it? [Traceability — FR-003, FR-004, FR-008, FR-009, FR-014, FR-016, FR-023, FR-024 lack direct acceptance scenarios]
- [ ] CHK076 - Does every success criterion (SC-001 through SC-010) have a corresponding benchmark or test task in tasks.md? [Traceability — SC-001→T097, SC-002→T098, SC-003→T099, SC-004→T100, SC-005→T119, SC-006→T101, SC-007→T087, SC-008→T059/T060, SC-009→T107, SC-010→T120 ✓]
- [ ] CHK077 - Does the error taxonomy in error-codes.md cover all error codes referenced in mcp-tools.json tool definitions? [Traceability — mcp-tools.json references codes 1001-5003; error-codes.md must include all]
- [ ] CHK078 - Does every constitution principle (I–IX) have at least one task or quality gate ensuring compliance? [Traceability — III (coverage) addressed by T137; IV (idempotency) addressed by mcp-tools.json annotations; VII (/health) addressed by T128; IX (feature flags) lacks a specific task for fastembed gating]

---

## Notes

- Total items: 78
- Items with `[Gap]` marker: 36 (indicating missing requirements that may need spec updates)
- Items referencing specific spec sections: 56 (72% traceability)
- Focus: requirements quality validation for automated agent consumption — each item is machine-parseable as a pass/fail gate
- Companion checklists may be generated for narrower domains (e.g., `security.md`, `api.md`)

### Requirements

# Specification Quality Checklist: engram Core MCP Daemon

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-02-05
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Validation Summary

| Category | Status | Notes |
|----------|--------|-------|
| Content Quality | PASS | Spec focuses on WHAT not HOW |
| Requirement Completeness | PASS | 24 FRs, 10 SCs, all testable |
| Feature Readiness | PASS | 5 user stories with acceptance scenarios |

## Notes

* Spec derived from existing engram v0 technical specification
* Implementation details (Rust, SurrealDB, axum) intentionally excluded from this spec
* Technical stack decisions documented in constitution and will be referenced in plan.md
* Ready for `/speckit.plan` phase
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Plan

# Implementation Plan: engram Core MCP Daemon

**Branch**: `001-core-mcp-daemon` | **Date**: 2026-02-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-core-mcp-daemon/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Implement the engram v0 core MCP daemon: a high-performance local-first state engine that serves as a shared brain for software development environments. The daemon uses axum 0.7 for HTTP/SSE transport, SurrealDB 2 (embedded `surrealkv`) for graph-relational storage with workspace isolation via SHA-256 path hashing, and `fastembed` for offline-capable semantic search. Git-backed persistence via `.engram/` markdown files enables state to travel with the codebase. See [research.md](research.md) for detailed technology decisions.

## Technical Context

**Language/Version**: Rust 2024 edition, `rust-version = "1.85"` (stable toolchain)
**Primary Dependencies**: axum 0.7, tokio 1 (full), mcp-sdk 0.0.3, surrealdb 2, fastembed 3 (optional), clap 4, sysinfo 0.30
**Storage**: SurrealDB 2 embedded (`surrealkv` backend) — graph-relational with MTREE vector indexes
**Testing**: `cargo test` — contract tests, integration tests, property tests (`proptest`), stress tests
**Target Platform**: Windows, macOS, Linux developer workstations (localhost daemon)
**Project Type**: Single Rust crate (library + binary)
**Performance Goals**: <200ms cold start, <50ms hybrid search, <10ms task writes, <1s full flush, 10 concurrent clients
**Constraints**: <100MB RSS idle, localhost-only (`127.0.0.1`), offline-capable (cached embedding model), `#![forbid(unsafe_code)]`
**Scale/Scope**: <10K tasks per workspace, up to 10 concurrent workspaces, 10 simultaneous SSE connections

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | Evidence |
|---|-----------|--------|----------|
| I | Rust Safety First | **PASS** | `#![forbid(unsafe_code)]` at crate root; `clippy::pedantic` enforced; all public APIs return `Result` |
| II | Async Concurrency Model | **PASS** | Tokio-only runtime; `Arc<RwLock>` shared state; cancellation tokens; `spawn_blocking` for file I/O |
| III | Test-First Development | **PASS** | Contract, integration, unit, and property test targets defined in `Cargo.toml`; TDD workflow in quickstart |
| IV | MCP Protocol Compliance | **PASS** | SSE transport; JSON-RPC via `mcp-sdk`; structured error responses; tool contracts in `mcp-tools.json` |
| V | Workspace Isolation | **PASS** | Canonicalized paths; `..` rejection; SHA-256 DB namespace isolation; localhost binding only |
| VI | Git-Friendly Persistence | **PASS** | Markdown format; `similar` crate for comment preservation; atomic writes; no binary files in `.engram/` |
| VII | Observability & Debugging | **PASS** | `tracing` with structured spans; `/health` endpoint; `sysinfo` for RSS metrics; correlation IDs |
| VIII | Error Handling & Recovery | **PASS** | `thiserror` in lib, `anyhow` in bin; typed `EngramError` enum; re-hydration on DB corruption |
| IX | Simplicity & YAGNI | **PASS** | Single crate; `fastembed` behind optional feature flag; configurable max workspaces |

**Gate result**: All principles satisfied. No violations requiring justification.

## Project Structure

### Documentation (this feature)

```text
specs/001-core-mcp-daemon/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output — technology decisions
├── data-model.md        # Phase 1 output — entity definitions
├── quickstart.md        # Phase 1 output — developer onboarding
├── contracts/
│   ├── mcp-tools.json   # Phase 1 output — MCP tool schemas
│   └── error-codes.md   # Phase 1 output — error taxonomy
├── checklists/
│   └── requirements.md  # Requirements traceability
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── lib.rs               # Crate root: forbid(unsafe_code), warn(clippy::pedantic)
├── bin/engram.rs          # Binary entrypoint: Config, Router, graceful shutdown
├── config/mod.rs         # Config struct (port, timeout, data_dir, log_format) via clap
├── db/
│   ├── mod.rs            # connect_db(workspace_hash) -> Db, schema bootstrap
│   ├── schema.rs         # DEFINE TABLE statements (spec, task, context, edges)
│   ├── queries.rs        # Queries struct: task CRUD, graph edges, cyclic detection
│   └── workspace.rs      # SHA-256 workspace path hashing, canonicalization
├── errors/
│   ├── mod.rs            # EngramError enum with domain sub-errors
│   └── codes.rs          # u16 error code constants (1xxx–5xxx)
├── models/
│   ├── mod.rs            # Re-exports
│   ├── task.rs           # Task, TaskStatus
│   ├── spec.rs           # Spec
│   ├── context.rs        # Context
│   └── graph.rs          # DependencyType
├── server/
│   ├── mod.rs            # Module re-exports
│   ├── router.rs         # build_router(SharedState) with /sse, /mcp, /health
│   ├── sse.rs            # SSE handler: keepalive, timeout, connection ID
│   ├── mcp.rs            # MCP JSON-RPC handler: deserialize, dispatch, respond
│   └── state.rs          # AppState, SharedState = Arc<AppState>
├── services/
│   ├── mod.rs            # Module re-exports
│   ├── connection.rs     # ConnectionLifecycle, workspace validation
│   ├── hydration.rs      # Hydrate workspace from .engram/ files
│   ├── dehydration.rs    # Dehydrate workspace state to .engram/ files
│   ├── embedding.rs      # Lazy model loading, vector generation
│   └── search.rs         # Hybrid search (vector + keyword)
└── tools/
    ├── mod.rs            # dispatch(state, method, params) -> Result<Value>
    ├── lifecycle.rs      # set_workspace, get_daemon_status, get_workspace_status
    ├── read.rs           # get_task_graph, check_status, query_memory
    └── write.rs          # create_task, update_task, add_blocker, register_decision, flush_state

tests/
├── contract/
│   ├── lifecycle_test.rs # MCP tool contract tests (workspace-not-set assertions)
│   ├── read_test.rs      # Read tool contract tests
│   └── write_test.rs     # Write tool contract tests
├── integration/
│   ├── connection_test.rs # SSE connection lifecycle tests
│   └── hydration_test.rs  # Hydration/dehydration round-trip tests
└── unit/
    ├── proptest_models.rs        # Property-based model tests
    └── proptest_serialization.rs # Serialization round-trip tests
```

**Structure Decision**: Single Rust crate with library + binary. Source modules mirror domain boundaries (server, db, models, services, tools). Tests separated into contract, integration, and unit directories per constitution III.

## Complexity Tracking

> No violations detected. All constitution gates pass without exceptions.

### Task Breakdown

# Tasks: engram Core MCP Daemon

**Input**: Design documents from `/specs/001-core-mcp-daemon/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

**Tests**: TDD is REQUIRED per constitution. Tests are included for all user stories.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single Rust workspace**: `src/`, `tests/` at repository root
- Binary: `src/bin/engram.rs`
- Library modules: `src/{module}/mod.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and Cargo workspace structure

- [X] T001 Create Cargo.toml workspace manifest with dependencies per research.md
- [X] T002 [P] Create src/lib.rs with crate-level attributes (`#![forbid(unsafe_code)]`, `#![warn(clippy::pedantic)]`)
- [X] T003 [P] Create src/bin/engram.rs binary entrypoint skeleton
- [X] T004 [P] Configure .cargo/config.toml for clippy and rustfmt settings
- [X] T005 [P] Create rust-toolchain.toml specifying Rust 2024 edition (1.82+)
- [X] T006 [P] Create .github/workflows/ci.yml for cargo fmt, clippy, test, audit

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Error Infrastructure

- [X] T007 Create src/errors/mod.rs with EngramError enum wrapping all error types
- [X] T008 [P] Create src/errors/codes.rs with error code constants (1xxx-5xxx per error taxonomy)
- [X] T009 [P] Implement MCP-compatible JSON error response serialization in src/errors/mod.rs

### Domain Models

- [X] T010 Create src/models/mod.rs re-exporting all model types
- [X] T011 [P] Create src/models/task.rs with Task struct and TaskStatus enum per data-model.md
- [X] T012 [P] Create src/models/spec.rs with Spec struct per data-model.md
- [X] T013 [P] Create src/models/context.rs with Context struct per data-model.md
- [X] T014 [P] Create src/models/graph.rs with DependencyType enum and relationship types

### Database Layer

- [X] T015 Create src/db/mod.rs with database connection management
- [X] T016 Create src/db/schema.rs with SurrealDB schema definitions (DEFINE TABLE statements)
- [X] T017 [P] Create src/db/queries.rs with query builder functions

### Configuration

- [X] T018 Create src/config/mod.rs with Config struct (port, timeout, paths)
- [X] T019 [P] Implement environment variable and CLI arg parsing in src/config/mod.rs

### Observability

- [X] T020 Create tracing subscriber initialization in src/lib.rs with JSON/pretty format toggle
- [X] T021 [P] Add correlation ID middleware structure in src/server/mod.rs placeholder

### Clarification Updates (Session 2026-02-09)

- [X] T109 [P] Add `StaleStrategy` enum and `max_workspaces`, `stale_strategy`, `data_dir` fields to Config struct in src/config/mod.rs
- [X] T110 [P] Add error code 1005 `WorkspaceLimitReached` to src/errors/codes.rs and `LimitReached` variant to WorkspaceError in src/errors/mod.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Daemon Connection & Workspace Binding (Priority: P1) 🎯 MVP

**Goal**: MCP clients can connect via SSE and bind to a Git repository workspace

**Independent Test**: Start daemon, connect via SSE curl, call set_workspace, verify ACTIVE state returned

### Tests for User Story 1

> **TDD: Write tests FIRST, ensure they FAIL, then implement**

- [X] T022 [P] [US1] Contract test for set_workspace in tests/contract/lifecycle_test.rs
- [X] T023 [P] [US1] Contract test for get_daemon_status in tests/contract/lifecycle_test.rs
- [X] T024 [P] [US1] Contract test for get_workspace_status in tests/contract/lifecycle_test.rs
- [X] T025 [P] [US1] Integration test for SSE connection lifecycle in tests/integration/connection_test.rs
- [X] T026 [P] [US1] Unit test for workspace path validation in src/services/connection.rs

### Implementation for User Story 1

- [X] T027 Create src/server/mod.rs with module structure
- [X] T028 Create src/server/router.rs with axum Router setup and routes
- [X] T029 Create src/server/sse.rs with SSE connection handling and connection ID assignment
- [X] T030 Create src/server/mcp.rs with MCP JSON-RPC request/response handling
- [X] T031 Create src/db/workspace.rs with workspace path hashing and namespace isolation
- [X] T032 Create src/services/mod.rs with module structure
- [X] T033 Create src/services/connection.rs with ConnectionState enum and lifecycle management
- [X] T034 [US1] Implement set_workspace tool in src/tools/lifecycle.rs (path validation, hydration trigger)
- [X] T035 [US1] Implement get_daemon_status tool in src/tools/lifecycle.rs
- [X] T036 [US1] Implement get_workspace_status tool in src/tools/lifecycle.rs
- [X] T037 [US1] Create src/tools/mod.rs with MCP tool registry and dispatch
- [X] T038 [US1] Add SSE keepalive ping (15s interval) in src/server/sse.rs
- [X] T039 [US1] Add connection timeout handling (60s configurable) in src/server/sse.rs
- [X] T040 [US1] Wire up daemon main() in src/bin/engram.rs with graceful shutdown (SIGTERM/SIGINT)

### Clarification Updates (Session 2026-02-09)

- [X] T111 [US1] Implement workspace limit check in set_workspace tool (FR-009a) returning error 1005 when max_workspaces exceeded in src/tools/lifecycle.rs
- [X] T112 [P] [US1] Contract test for workspace limit exceeded (error 1005) in tests/contract/lifecycle_test.rs

### Analyze Remediation (Session 2026-02-12)

- [X] T128 [US1] Implement HTTP GET /health endpoint in src/server/router.rs returning daemon status and active workspace count (FR-026, constitution VII; moved from Phase 8)

**Checkpoint**: Daemon starts, accepts SSE connections, binds workspaces, enforces workspace limits, exposes /health

---

## Phase 4: User Story 2 - Task State Management (Priority: P2)

**Goal**: Clients can create, update, and query tasks with graph relationships

**Independent Test**: Connect, update_task to change status, get_task_graph to verify, add_blocker to block

### Tests for User Story 2

- [X] T041 [P] [US2] Contract test for update_task in tests/contract/write_test.rs
- [X] T042 [P] [US2] Contract test for add_blocker in tests/contract/write_test.rs
- [X] T043 [P] [US2] Contract test for register_decision in tests/contract/write_test.rs
- [X] T044 [P] [US2] Contract test for get_task_graph in tests/contract/read_test.rs
- [X] T045 [P] [US2] Contract test for check_status in tests/contract/read_test.rs
- [X] T046 [P] [US2] Unit test for cyclic dependency detection in src/db/queries.rs
- [X] T047 [P] [US2] Property test for Task serialization round-trip in tests/unit/proptest_models.rs
- [X] T129 [P] [US2] Contract test for create_task returning WorkspaceNotSet (1003) when workspace not bound in tests/contract/write_test.rs
- [X] T130 [P] [US2] Contract test for create_task with empty title returning TaskTitleEmpty (3005) in tests/contract/write_test.rs
- [X] T131 [P] [US2] Integration test for create_task with parent_task_id creating depends_on edge in tests/integration/hydration_test.rs

### Implementation for User Story 2

- [X] T048 [US2] Implement task CRUD operations in src/db/queries.rs
- [X] T049 [US2] Implement graph edge operations (depends_on, implements, relates_to) in src/db/queries.rs
- [X] T050 [US2] Implement cyclic dependency detection before edge insert in src/db/queries.rs
- [X] T051 [US2] Implement update_task tool in src/tools/write.rs
- [X] T052 [US2] Implement add_blocker tool in src/tools/write.rs
- [X] T053 [US2] Implement register_decision tool in src/tools/write.rs
- [X] T054 [US2] Implement get_task_graph tool in src/tools/read.rs (recursive graph traversal)
- [X] T055 [US2] Implement check_status tool in src/tools/read.rs
- [X] T056 [US2] Add context note creation on task update in src/services/connection.rs

### Create Task Tool (FR-013a, Session 2026-02-12)

- [X] T132 [US2] Add TaskTitleEmpty (3005) error variant to TaskError enum and wire to error code mapping in src/errors/mod.rs
- [X] T133 [US2] Add error code constant `TASK_TITLE_EMPTY: u16 = 3005` in src/errors/codes.rs
- [X] T134 [US2] Add `create_task` query method to Queries struct: insert task with generated UUID, `todo` status, optional parent via depends_on edge in src/db/queries.rs
- [X] T135 [US2] Implement `create_task` tool: validate title (non-empty, max 200 chars), accept optional description/parent_task_id/work_item_id, call Queries, return created task in src/tools/write.rs
- [X] T136 [US2] Add `create_task` dispatch route to tools::dispatch() match arm in src/tools/mod.rs

### Gap Analysis Updates (Session 2026-02-09)

- [X] T121 [US2] Implement task status transition validation per data-model.md state machine (reject invalid transitions like done→blocked) in src/tools/write.rs
- [X] T122 [P] [US2] Contract test for invalid task status transition (error 3002) in tests/contract/write_test.rs
- [X] T127 [P] [US2] Contract test for work_item_id assignment and retrieval via update_task and get_task_graph in tests/contract/write_test.rs (FR-017 coverage)

**Checkpoint**: Full task CRUD (including create), graph operations, and state transition validation functional

---

## Phase 5: User Story 3 - Git-Backed Persistence (Priority: P3)

**Goal**: Workspace state serializes to .engram/ files preserving user comments

**Independent Test**: Modify state, flush_state, verify tasks.md human-readable with comments preserved, hydrate verifies round-trip

### Tests for User Story 3

- [X] T057 [P] [US3] Contract test for flush_state in tests/contract/write_test.rs
- [X] T058 [P] [US3] Integration test for hydration from .engram/ files in tests/integration/hydration_test.rs
- [X] T059 [P] [US3] Integration test for dehydration preserving comments in tests/integration/hydration_test.rs
- [X] T060 [P] [US3] Property test for markdown round-trip in tests/unit/proptest_serialization.rs
- [X] T061 [P] [US3] Unit test for stale file detection in src/services/hydration.rs

### Implementation for User Story 3

- [X] T062 [US3] Create src/services/hydration.rs with .engram/ file parsing (pulldown-cmark)
- [X] T063 [US3] Implement tasks.md parser extracting YAML frontmatter and descriptions
- [X] T064 [US3] Implement graph.surql parser for RELATE statements
- [X] T065 [US3] Implement .version and .lastflush file handling
- [X] T066 [US3] Create src/services/dehydration.rs with DB → file serialization
- [X] T067 [US3] Implement structured diff merge comment preservation using `similar` crate
- [X] T068 [US3] Implement atomic file writes (temp file + rename pattern)
- [X] T069 [US3] Implement flush_state tool in src/tools/write.rs
- [X] T070 [US3] Implement stale file detection comparing .lastflush to file mtime
- [X] T071 [US3] Implement corruption recovery (delete DB, re-hydrate from files)
- [X] T072 [US3] Wire hydration into set_workspace flow in src/tools/lifecycle.rs

### Clarification Updates (Session 2026-02-09)

- [X] T113 [US3] Record file mtimes at hydration time in workspace metadata for stale detection (FR-012a) in src/services/hydration.rs
- [X] T114 [US3] Implement configurable stale strategy (warn/rehydrate/fail per FR-012b) in src/services/dehydration.rs and src/services/hydration.rs
- [X] T115 [P] [US3] Integration test for stale strategy `warn` mode (emit 2004 warning, proceed with in-memory state) in tests/integration/hydration_test.rs
- [X] T116 [P] [US3] Integration test for stale strategy `rehydrate` mode (reload from disk on external change) in tests/integration/hydration_test.rs
- [X] T117 [P] [US3] Integration test for stale strategy `fail` mode (reject operation on stale files) in tests/integration/hydration_test.rs
- [X] T123 [US3] Wire `stale_files` boolean from workspace metadata into `get_workspace_status` response in src/tools/read.rs

### Analyze Remediation (Session 2026-02-12)

- [X] T108 [US3] Add graceful shutdown flush of all active workspaces on SIGTERM/SIGINT in src/bin/engram.rs (FR-006 MUST requirement; moved from Phase 8)

**Checkpoint**: Git-backed persistence with comment preservation, stale-file detection, and shutdown flush functional

---

## Phase 6: User Story 4 - Semantic Memory Query (Priority: P4)

**Goal**: Hybrid vector + keyword search returns relevant context for natural language queries

**Independent Test**: Populate specs/context, query_memory with natural language, verify ranked relevant results

### Tests for User Story 4

- [X] T073 [P] [US4] Contract test for query_memory in tests/contract/read_test.rs
- [X] T074 [P] [US4] Unit test for embedding generation in src/services/embedding.rs
- [X] T075 [P] [US4] Unit test for hybrid scoring (0.7 vector + 0.3 keyword) in src/services/search.rs
- [X] T076 [P] [US4] Integration test for lazy model download in tests/integration/embedding_test.rs

### Implementation for User Story 4

- [X] T077 [US4] Create src/services/embedding.rs with fastembed-rs integration
- [X] T078 [US4] Implement lazy model download to ~/.local/share/engram/models/
- [X] T079 [US4] Implement embedding generation for spec and context content
- [X] T080 [US4] Create src/services/search.rs with hybrid search logic
- [X] T081 [US4] Implement vector similarity search using SurrealDB MTREE index
- [X] T082 [US4] Implement keyword matching (BM25-style) for text content
- [X] T083 [US4] Implement weighted score combination (0.7 * vector + 0.3 * keyword)
- [X] T084 [US4] Implement query_memory tool in src/tools/read.rs
- [X] T085 [US4] Add query character limit validation (2000 characters max, error 4001)
- [X] T086 [US4] Wire embedding generation into hydration for missing embeddings

### Gap Analysis Updates (Session 2026-02-09)

- [X] T125 [US4] ~~Update query_memory character limit from 500 tokens to 2000 characters per updated spec (FR-018, SC-003) in src/tools/read.rs~~ — Already implemented in T085 (`MAX_QUERY_CHARS = 2000`)

**Checkpoint**: Semantic search returns relevant ranked results

---

## Phase 7: User Story 5 - Multi-Client Concurrent Access (Priority: P5)

**Goal**: 10+ clients access same workspace concurrently without conflicts

**Independent Test**: Connect 10 clients, interleaved read/write, verify consistent state, no corruption

### Tests for User Story 5

- [X] T087 [P] [US5] Stress test with 10 concurrent clients in tests/integration/concurrency_test.rs
- [X] T088 [P] [US5] Test last-write-wins for simple fields in tests/integration/concurrency_test.rs
- [X] T089 [P] [US5] Test append-only semantics for context in tests/integration/concurrency_test.rs
- [X] T090 [P] [US5] Test FIFO serialization of concurrent flush_state calls in tests/integration/concurrency_test.rs

### Implementation for User Story 5

- [X] T091 [US5] Implement connection registry with Arc<RwLock<HashMap>> in src/services/connection.rs
- [X] T092 [US5] Implement per-workspace write lock for flush_state in src/services/dehydration.rs
- [X] T093 [US5] Implement last-write-wins with updated_at timestamps in src/db/queries.rs
- [X] T094 [US5] Verify append-only context insertion (no overwrite) in src/db/queries.rs
- [X] T095 [US5] Add connection cleanup on disconnect in src/server/sse.rs
- [X] T096 [US5] Implement workspace state preservation across client disconnects
- [X] T118 [US5] Implement connection rate limiting returning error 5003 when threshold exceeded (FR-025) in src/server/sse.rs
- [X] T124 [P] [US5] Contract test for rate limiting (error 5003) in tests/contract/lifecycle_test.rs

**Checkpoint**: Multi-client concurrent access stable

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Performance optimization, documentation, final hardening

### Performance Validation

- [X] T097 Benchmark cold start time (target: < 200ms) and document results
- [X] T098 Benchmark hydration time with 1000 tasks (target: < 500ms)
- [X] T099 Benchmark query_memory latency (target: < 50ms)
- [X] T100 Benchmark update_task latency (target: < 10ms)
- [X] T101 Profile memory usage idle and under load (targets: < 100MB / < 500MB)
- [X] T119 Benchmark flush_state latency with full workspace (target: < 1s per SC-005)
- [X] T120 Create test corpus and evaluation script for query_memory relevance validation (target: 95% per SC-010)

### Documentation

- [X] T102 Create README.md with installation and usage instructions
- [X] T103 Add rustdoc comments to all public APIs in src/lib.rs
- [X] T104 Update specs/001-core-mcp-daemon/quickstart.md with final implementation details
- [X] T126 Run cargo doc --deny warnings and verify zero documentation warnings (constitution quality gate #4)

### Final Hardening

- [X] T105 Run cargo audit and resolve any vulnerabilities
- [X] T106 Run full test suite with --release optimizations
- [X] T107 Verify all error codes match contracts/error-codes.md
- [X] T137 Run `cargo tarpaulin` (or equivalent) and verify ≥80% line coverage for all `src/` modules (constitution III quality gate)

---

## Dependency Graph

```
Phase 1 (Setup)
    ↓
Phase 2 (Foundational + Clarification Updates) ─────────────┐
    ↓                                                        │
Phase 3 (US1: Connection + Workspace Limits + /health) ← MVP           │
    ↓                                                        │
Phase 4 (US2: Tasks) ← Depends on US1                       │
    ↓                                                        │
Phase 5 (US3: Persistence + Stale Detection + Shutdown Flush) ← Depends US1,2│
    ↓                                                        │
Phase 6 (US4: Search) ← Depends on US1, integrates US3      │
    ↓                                                        │
Phase 7 (US5: Concurrency) ← Depends on ALL previous        │
    ↓                                                        │
Phase 8 (Polish) ← Final validation after all stories ──────┘
```

### Clarification Task Dependencies

- T109, T110 (Phase 2): No dependencies on existing incomplete tasks; extend Config and Error modules
- T111 (Phase 3): Depends on T109 (max_workspaces config field) and T110 (error 1005)
- T112 (Phase 3): Depends on T111 (implementation to test against)
- T113 (Phase 5): Depends on T109 (StaleStrategy enum in config)
- T114 (Phase 5): Depends on T109 (stale_strategy config) and T113 (mtime recording)
- T115, T116, T117 (Phase 5): Depend on T114 (strategy implementation to test)
- T121 (Phase 4): No cross-phase dependencies; refines existing update_task (T051)
- T122 (Phase 4): Depends on T121 (transition validation implementation to test)
- T123 (Phase 5): Depends on T113 (stale_files data from workspace metadata)
- T124 (Phase 7): Depends on T118 (rate limiting implementation to test)
- T125 (Phase 6): Resolved — already implemented in T085; marked complete
- T128 (Phase 3): Moved from Phase 8; depends on T028 (router setup) — constitution VII infrastructure
- T108 (Phase 5): Moved from Phase 8; depends on T069 (flush_state tool) — FR-006 MUST requirement

## Parallel Execution Examples

**Within Phase 2 (Foundational)**:
- T007, T008, T009 can run in parallel (error infrastructure)
- T010, T011, T012, T013, T014 can run in parallel (models)
- T015, T016, T017 can run sequentially (DB layer)
- T109, T110 can run in parallel (clarification config/error updates)

**Within Phase 3 (US1)**:
- T022, T023, T024, T025, T026 can run in parallel (all tests)
- T027 → T028 → T029 → T030 (server layer sequential)
- T034, T035, T036 can run in parallel after T037 (tools)
- T112 can start after T111 (workspace limit test after implementation)

**Within Phase 4 (US2)**:
- T121 → T122 (transition validation before test)

**Within Phase 5 (US3)**:
- T057, T058, T059, T060, T061 can run in parallel (all tests)
- T115, T116, T117 can run in parallel (stale strategy tests, after T114)
- T113 → T114 (mtime tracking before strategy application)
- T123 can run in parallel with T115-T117 (after T113)

**Across Phases (after Phase 2 complete)**:
- US1 implementation blocks US2, US3, US4, US5
- Within each US phase, tests can run in parallel before implementation
- T109/T110 should complete before T111 (Phase 2 before Phase 3 clarification tasks)
- T124 can run in parallel with T087-T090 (after T118)
- T125 can run independently within Phase 6

## Implementation Strategy

1. **MVP First**: Complete Phase 1-3 for minimal working daemon (including workspace limits)
2. **Incremental Delivery**: Each user story phase delivers testable functionality
3. **Test Coverage**: TDD required; 80% minimum per constitution
4. **Performance Last**: Optimize only after correctness validated (Phase 8)
5. **Clarification Tasks**: T109-T117 integrate requirements from spec clarifications (FR-009a, FR-012a, FR-012b)
6. **Gap Analysis Tasks**: T121-T125 fill coverage gaps found during cross-document consistency analysis
7. **Analyze Remediation Tasks**: T126-T128, T137 fill gaps identified by `/speckit.analyze`
8. **Moved Tasks**: T108 → Phase 5 (FR-006 MUST), T128 → Phase 3 (constitution VII), T125 → resolved as duplicate

**Suggested MVP Scope**: Phases 1-3 (Setup + Foundational + US1) deliver a daemon that accepts connections, binds workspaces, and enforces workspace concurrency limits.

### Task Summary

| Phase | Scope | Total | Complete | Remaining |
|-------|-------|-------|----------|-----------|
| 1 | Setup | 6 | 6 | 0 |
| 2 | Foundational | 17 | 17 | 0 |
| 3 | US1: Connection | 22 | 22 | 0 |
| 4 | US2: Tasks | 27 | 27 | 0 |
| 5 | US3: Persistence | 23 | 23 | 0 |
| 6 | US4: Search | 15 | 15 | 0 |
| 7 | US5: Concurrency | 12 | 12 | 0 |
| 8 | Polish | 14 | 14 | 0 |
| **Total** | | **137** | **137** | **0** |
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
### Research

# Research: engram Core MCP Daemon

**Phase**: 0 — Research & Decision Documentation
**Created**: 2026-02-05
**Purpose**: Document technology decisions, alternatives considered, and best practices

## Technology Decisions

### 1. HTTP Server Framework

**Decision**: `axum` 0.7+

**Rationale**:
- Native Tokio integration — no runtime mismatch
- Tower middleware ecosystem — composable layers for logging, auth, etc.
- Type-safe extractors — compile-time request validation
- First-class SSE support via `axum::response::sse::Sse`
- Strong community adoption in Rust async ecosystem

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| `actix-web` | Uses its own runtime, complicates Tokio integration |
| `warp` | Filter-based API less intuitive; weaker type safety |
| `hyper` directly | Too low-level; would reinvent routing/middleware |
| `poem` | Smaller ecosystem; less community support |

### 2. MCP Protocol Implementation

**Decision**: `mcp-sdk-rs` with SSE transport

**Rationale**:
- Official Rust SDK for Model Context Protocol
- SSE transport preferred for daemon (long-lived connections, server-push)
- Handles JSON-RPC framing and tool registration
- Actively maintained by Anthropic ecosystem

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| Custom implementation | Protocol complexity; maintenance burden |
| WebSocket transport | More complex; SSE sufficient for our use case |
| stdio transport | Not suitable for multi-client daemon |

### 3. Embedded Database

**Decision**: `surrealdb` 2.0+ with `surrealkv` backend

**Rationale**:
- Graph-relational model fits task/spec/context domain
- Native record links for relationships (no join tables)
- Built-in vector search with MTREE indexes
- Embedded mode (no separate process)
- SurrealQL expressive for complex queries
- Namespace/database isolation for multi-tenancy

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| SQLite + FTS5 | No native vector search; graph queries complex |
| PostgreSQL | External process; too heavy for local daemon |
| Redis | No embedded mode; persistence complexity |
| RocksDB directly | Too low-level; would rebuild query layer |
| LanceDB | Vector-only; no graph relationships |

**Configuration**:
```rust
// Embedded mode with surrealkv backend
let db = Surreal::new::<SurrealKv>("~/.local/share/engram/db").await?;
db.use_ns("tmem").use_db(workspace_hash).await?;
```

### 4. Embedding Model

**Decision**: `all-MiniLM-L6-v2` via `fastembed-rs`

**Rationale**:
- 384 dimensions — compact, fast indexing
- Good semantic quality for code/documentation
- ~90MB model size — reasonable download
- Rust-native via ONNX runtime — no Python dependency
- MIT licensed

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| OpenAI embeddings | Requires API key; not offline-capable |
| `nomic-embed-text` | Larger (768 dims); slower queries |
| `bge-small-en` | Slightly worse quality on code |
| Python sentence-transformers | FFI complexity; deployment burden |

**Model Storage**:
- Cache: `~/.local/share/engram/models/`
- Lazy download on first `query_memory` call
- Offline mode if model already cached

### 5. Markdown Parsing

**Decision**: `pulldown-cmark` for parsing, custom serializer for writing

**Rationale**:
- Fast, standards-compliant CommonMark parser
- Event-based API allows streaming
- No dependencies on C libraries
- Well-tested in mdBook and other Rust projects

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| `comrak` | GFM extensions we don't need; larger binary |
| `markdown-rs` | Less mature; fewer features |
| `pest` + grammar | Would need to define full grammar |

### 6. Diff/Merge for Comment Preservation

**Decision**: `similar` crate with patience diff algorithm

**Rationale**:
- Pure Rust implementation
- Patience diff produces cleaner diffs for structured text
- Supports unified diff format
- Line-level and word-level diff options

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| `diff` crate | Simpler but less accurate on structured text |
| `diffy` | Less actively maintained |
| Git diff library | External dependency; overkill |

**Strategy**:
1. Parse existing `tasks.md` into task blocks
2. Generate new task content from DB
3. Merge: preserve non-task lines (comments), update task blocks
4. Write merged content

### 7. Error Handling Strategy

**Decision**: `thiserror` in library, `anyhow` in binary

**Rationale**:
- Per constitution: typed errors in library code
- `thiserror` for domain-specific error types with error codes
- `anyhow` in binary for easy error propagation
- Structured error responses to MCP clients

**Error Hierarchy**:
```rust
#[derive(thiserror::Error, Debug)]
pub enum EngramError {
    #[error("Workspace error: {0}")]
    Workspace(#[from] WorkspaceError),
    
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    
    #[error("Hydration error: {0}")]
    Hydration(#[from] HydrationError),
    
    #[error("Query error: {0}")]
    Query(#[from] QueryError),
}
```

### 8. Logging & Observability

**Decision**: `tracing` with `tracing-subscriber`

**Rationale**:
- Structured logging with spans and events
- Correlation ID propagation via span context
- Multiple output formats (JSON, pretty)
- Integration with Tokio for async spans
- Per constitution: observability required

**Configuration**:
```rust
tracing_subscriber::fmt()
    .with_env_filter("engram=debug,surrealdb=warn")
    .with_span_events(FmtSpan::CLOSE)
    .json() // or .pretty() for development
    .init();
```

### 9. Workspace Concurrency Limits

**Decision**: Configurable max concurrent workspaces, default 10

**Rationale**:
- Each workspace opens an isolated SurrealDB database (memory + file handles)
- Unbounded workspaces risk OOM on developer laptops
- Default of 10 matches FR-002 concurrent client limit (natural parity)
- Configurable via CLI flag `--max-workspaces` or env `engram_MAX_WORKSPACES`

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| Unlimited (memory-bounded) | Unpredictable resource usage; hard to debug OOM |
| Single workspace per daemon | Too restrictive for multi-repo workflows |
| LRU eviction | Implicit eviction risks data loss; explicit release preferred |

**Implementation**:
- Track active workspaces in `AppState` via `HashMap<String, WorkspaceHandle>`
- Check count before `set_workspace`; return new error code if at limit
- Clients can release workspaces explicitly or via disconnect cleanup

### 10. Stale File Detection Strategy

**Decision**: Default warn-and-proceed; configurable to `rehydrate` or `fail`

**Rationale**:
- Local-first tool should never silently discard in-memory work
- Warning (error 2004 StaleWorkspace) alerts the user without blocking
- `rehydrate` mode useful for CI or scripted scenarios
- `fail` mode useful for strict data integrity requirements

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| Always rehydrate | Discards in-memory deltas silently; data loss risk |
| Always fail | Too disruptive for normal development workflow |
| File watching | Out of scope for v0; adds inotify/kqueue complexity |

**Detection Mechanism**:
- Record mtime of `.engram/` files at hydration time
- Before flush or re-hydrate, compare current mtime to recorded value
- If mtime differs, apply configured strategy (warn/rehydrate/fail)
- Expose `stale_files` boolean in `get_workspace_status` response

## Best Practices Applied

### Rust Async Patterns

1. **Cancellation Safety**: All async operations check `CancellationToken`
2. **Spawn Blocking**: File I/O in `spawn_blocking` to avoid blocking executor
3. **Bounded Channels**: Use `mpsc::channel(capacity)` to prevent unbounded memory growth
4. **Graceful Shutdown**: 
   ```rust
   tokio::select! {
       _ = shutdown_signal() => { flush_all_workspaces().await; }
       _ = server.run() => {}
   }
   ```

### Connection Management

1. **Per-connection state**: Each SSE connection owns its workspace binding
2. **Weak references**: Workspace state uses `Arc<RwLock<_>>` for shared access
3. **Cleanup on disconnect**: Connection registry removes entry on stream close
4. **Timeout handling**: Tokio `timeout` wrapper on idle connections

### Database Patterns

1. **Transaction per tool call**: Each MCP tool executes in single transaction
2. **Optimistic locking**: Use `updated_at` for conflict detection (last-write-wins)
3. **Schema migrations**: Version check on workspace hydration
4. **Connection pooling**: SurrealDB handle is `Clone`; share across tasks

### Testing Strategy

1. **Unit tests**: Co-located in `src/` modules with `#[cfg(test)]`
2. **Integration tests**: Full daemon startup in `tests/integration/`
3. **Contract tests**: MCP tool schemas validated in `tests/contract/`
4. **Property tests**: Serialization round-trips with `proptest`
5. **Stress tests**: 10 concurrent clients hitting same workspace

## Open Questions (Resolved)

All initial unknowns have been resolved during research:

| Question | Resolution |
|----------|------------|
| Which MCP SDK? | `mcp-sdk-rs` — official Rust implementation |
| Which embedding model? | `all-MiniLM-L6-v2` via `fastembed-rs` |
| How to preserve markdown comments? | `similar` crate with block-level merge |
| Vector index type? | MTREE in SurrealDB — built-in |
| How to hash workspace paths? | SHA256 of canonicalized path |
| Max concurrent workspaces? | Configurable upper bound, default 10 (matches FR-002 client limit) |
| Stale `.engram/` file conflict strategy? | Default: warn-and-proceed (emit 2004 StaleWorkspace, continue with in-memory state); configurable to `rehydrate` or `fail` |

## Dependencies Summary

```toml
[dependencies]
# Server
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# MCP Protocol
mcp-sdk = "0.0.3"  # or mcp-sdk-rs depending on crate name

# Database
surrealdb = { version = "2", features = ["kv-surrealkv"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Markdown
pulldown-cmark = "0.10"

# Embeddings
fastembed = "3"

# Error Handling
thiserror = "1"
anyhow = "1"

# Utilities
uuid = { version = "1", features = ["v4"] }
similar = "2"
chrono = { version = "0.4", features = ["serde"] }

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

[dev-dependencies]
proptest = "1"
tokio-test = "0.4"
```

## References

- [MCP Specification](https://modelcontextprotocol.io/specification)
- [SurrealDB Documentation](https://surrealdb.com/docs)
- [axum User Guide](https://docs.rs/axum)
- [fastembed-rs Repository](https://github.com/Anush008/fastembed-rs)
- [Tokio Best Practices](https://tokio.rs/tokio/topics/bridging)

### Data Model

# Data Model: engram Core MCP Daemon

**Phase**: 1 — Design & Contracts
**Created**: 2026-02-05
**Purpose**: Define entity structures, relationships, and validation rules

## Overview

engram uses a **graph-relational** data model where core entities (Spec, Task, Context) are connected via typed edges (implements, depends_on, relates_to). This enables both tabular queries and graph traversals.

## Entity Definitions

### Spec

Represents a high-level requirement captured from specification files.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<spec>` | Auto | SurrealDB record ID (e.g., `spec:abc123`) |
| `title` | `string` | Yes | Human-readable title extracted from spec |
| `content` | `string` | Yes | Full text content of the specification |
| `embedding` | `array<f32>` | Optional | 384-dimensional vector for semantic search |
| `file_path` | `string` | Yes | Relative path to source file in repo |
| `created_at` | `datetime` | Auto | First import timestamp |
| `updated_at` | `datetime` | Auto | Last modification timestamp |

**Validation Rules**:
- `title` must be non-empty, max 500 characters
- `file_path` must be a valid relative path (no `..`, no absolute paths)
- `file_path` must be unique per workspace (indexed)
- `embedding` must have exactly 384 elements when present

**State Transitions**: N/A (specs are imported, not state machines)

---

### Task

Represents an actionable unit of work derived from specifications.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<task>` | Auto | SurrealDB record ID (e.g., `task:xyz789`) |
| `title` | `string` | Yes | Brief description of the task |
| `status` | `string` | Yes | Current state: `todo`, `in_progress`, `done`, `blocked` |
| `work_item_id` | `option<string>` | No | External tracker reference (e.g., `AB#12345`) |
| `description` | `string` | Yes | Detailed task description |
| `context_summary` | `option<string>` | No | AI-generated summary of progress |
| `created_at` | `datetime` | Auto | Task creation timestamp |
| `updated_at` | `datetime` | Auto | Last modification timestamp |

**Validation Rules**:
- `title` must be non-empty, max 200 characters
- `status` must be one of: `todo`, `in_progress`, `done`, `blocked`
- `work_item_id` format: `AB#\d+` (ADO) or `[\w-]+/[\w-]+#\d+` (GitHub) when present
- `description` may be empty string but not null

**State Transitions**:

```
┌─────────────────────────────────────────┐
│                                         │
│   ┌──────┐      ┌─────────────┐        │
│   │ todo │─────▶│ in_progress │        │
│   └──────┘      └─────────────┘        │
│       │               │    │            │
│       │               │    └───────┐    │
│       │               ▼            │    │
│       │         ┌─────────┐        │    │
│       │         │ blocked │────────┤    │
│       │         └─────────┘        │    │
│       │               │            │    │
│       ▼               ▼            ▼    │
│   ┌──────────────────────────────────┐ │
│   │              done                │ │
│   └──────────────────────────────────┘ │
│                                         │
└─────────────────────────────────────────┘
```

**Allowed Transitions**:
- `todo` → `in_progress`, `done`
- `in_progress` → `done`, `blocked`, `todo`
- `blocked` → `in_progress`, `todo`, `done`
- `done` → `todo` (reopen)

---

### Context

Represents ephemeral knowledge captured during task execution.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<context>` | Auto | SurrealDB record ID |
| `content` | `string` | Yes | The captured knowledge/note |
| `embedding` | `array<f32>` | Optional | 384-dimensional vector for semantic search |
| `source_client` | `string` | Yes | Client that created this (e.g., `cli`, `ide`) |
| `created_at` | `datetime` | Auto | Creation timestamp |

**Validation Rules**:
- `content` must be non-empty
- `source_client` must be a valid identifier (alphanumeric + underscore)
- `embedding` must have exactly 384 elements when present

**Notes**:
- Context is append-only; never updated or deleted during normal operation
- Context nodes are linked to tasks via `relates_to` edges

---

## Relationship Definitions

### depends_on (Task → Task)

Tracks blocking relationships between tasks.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | The dependent task (blocked by `out`) |
| `out` | `record<task>` | The blocking task |
| `type` | `string` | `hard_blocker` or `soft_dependency` |
| `created_at` | `datetime` | When dependency was created |

**Validation Rules**:
- Cannot create self-referential edges (`in` ≠ `out`)
- Cannot create cycles (validate with graph traversal before insert)
- `type` must be one of: `hard_blocker`, `soft_dependency`

**Semantics**:
- `hard_blocker`: Task cannot progress until blocker is `done`
- `soft_dependency`: Task may proceed but blocker provides important context

---

### implements (Task → Spec)

Links tasks to the specifications they fulfill.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | The implementing task |
| `out` | `record<spec>` | The specification being implemented |
| `created_at` | `datetime` | When link was created |

**Validation Rules**:
- One task may implement multiple specs
- One spec may be implemented by multiple tasks

---

### relates_to (Task → Context)

Associates context nodes with tasks.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | The task |
| `out` | `record<context>` | The related context |
| `created_at` | `datetime` | When link was created |

**Validation Rules**:
- One task may have many related context nodes
- One context may relate to multiple tasks (rare but allowed)

---

## Workspace Metadata

Each workspace has implicit metadata tracked outside the main schema:

| Field | Type | Description |
|-------|------|-------------|
| `path` | `string` | Canonicalized absolute path to Git repo root |
| `hash` | `string` | SHA256 of path, used as database name |
| `schema_version` | `string` | Version of `.engram/` schema (e.g., `1.0.0`) |
| `last_flush` | `datetime` | Timestamp of last dehydration |
| `file_mtimes` | `HashMap<String, SystemTime>` | Recorded mtime of each `.engram/` file at hydration; used for stale-file detection |
| `stale_files` | `bool` | Whether external modifications have been detected since last hydration |

---

## Daemon Configuration

Runtime-configurable settings (CLI flags, env vars, or config file):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | `u16` | `7437` | Listening port on 127.0.0.1 |
| `max_workspaces` | `usize` | `10` | Maximum concurrent active workspaces (FR-009a) |
| `connection_timeout_secs` | `u64` | `60` | Idle connection timeout (FR-005) |
| `keepalive_interval_secs` | `u64` | `15` | SSE keepalive ping interval (FR-004) |
| `stale_strategy` | `StaleStrategy` | `Warn` | Behavior on stale `.engram/` files: `Warn`, `Rehydrate`, `Fail` (FR-012b) |
| `data_dir` | `PathBuf` | `~/.local/share/engram/` | Location for SurrealDB files and model cache |

---

## Indexes

### Primary Indexes

| Table | Index Name | Columns | Type | Purpose |
|-------|------------|---------|------|---------|
| `spec` | `spec_file_path` | `file_path` | UNIQUE | Fast lookup by file |
| `spec` | `spec_embedding` | `embedding` | MTREE (384, COSINE) | Vector search |
| `task` | `task_status` | `status` | STANDARD | Filter by status |
| `task` | `task_work_item` | `work_item_id` | STANDARD | External ID lookup |
| `task` | `task_updated` | `updated_at` | STANDARD | Recent changes |
| `context` | `context_source` | `source_client` | STANDARD | Filter by client |
| `context` | `context_created` | `created_at` | STANDARD | Chronological order |
| `context` | `context_embedding` | `embedding` | MTREE (384, COSINE) | Vector search |

---

## File Format: `.engram/tasks.md`

Tasks are serialized to Markdown with YAML frontmatter:

```markdown
# Tasks

<!-- User comments here are preserved across flushes -->

## task:abc123

---
id: task:abc123
title: Implement user authentication
status: in_progress
work_item_id: AB#12345
created_at: 2026-02-05T10:00:00Z
updated_at: 2026-02-05T14:30:00Z
---

Detailed description of the task goes here.
Multiple paragraphs are supported.

<!-- User can add notes that will be preserved -->

## task:def456

---
id: task:def456
title: Write unit tests for auth module
status: todo
created_at: 2026-02-05T10:05:00Z
updated_at: 2026-02-05T10:05:00Z
---

Write comprehensive tests for the authentication service.
```

**Parsing Rules**:
1. Each `## task:*` heading starts a new task block
2. YAML frontmatter between `---` delimiters contains structured fields
3. Content after frontmatter is `description`
4. Content outside task blocks (comments) is preserved verbatim

---

## File Format: `.engram/graph.surql`

Graph relationships are serialized as SurrealQL:

```surql
-- Generated by engram. Do not edit manually.
-- Schema version: 1.0.0
-- Generated at: 2026-02-05T14:30:00Z

-- Dependencies
RELATE task:abc123->depends_on->task:def456 SET type = 'hard_blocker';
RELATE task:ghi789->depends_on->task:abc123 SET type = 'soft_dependency';

-- Implementations
RELATE task:abc123->implements->spec:auth_spec;

-- Context Relations
RELATE task:abc123->relates_to->context:note001;
RELATE task:abc123->relates_to->context:note002;
```

**Parsing Rules**:
1. Skip comments (lines starting with `--`)
2. Parse `RELATE` statements to reconstruct edges
3. Ignore unknown statement types

---

## Schema Migration

Version stored in `.engram/.version` file.

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial schema |

**Migration Strategy**:
- On hydration, compare `.version` to current daemon version
- Apply forward migrations automatically
- Reject backward-incompatible versions with error

---

## Rust Type Definitions

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    pub file_path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    pub source_client: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    HardBlocker,
    SoftDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleStrategy {
    Warn,
    Rehydrate,
    Fail,
}
```

### Quickstart

# Quickstart: engram Development

**Purpose**: Get developers up and running with engram development
**Prerequisites**: Rust 1.85+, Git

## Environment Setup

### 1. Install Rust Toolchain

```bash
# Install rustup (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ensure 2024 edition support
rustup update stable
rustup default stable

# Verify version (1.85+ required)
rustc --version
```

### 2. Install Development Tools

```bash
# Code formatting
rustup component add rustfmt

# Linting
rustup component add clippy

# Security audit
cargo install cargo-audit

# Coverage (optional)
cargo install cargo-tarpaulin
```

### 3. Clone and Build

```bash
# Clone repository
git clone https://github.com/softwaresalt/engram.git
cd engram

# Build all targets
cargo build

# Run tests
cargo test

# Run lints
cargo clippy -- -D warnings

# Check formatting
cargo fmt --check
```

---

## Project Structure

```
engram/
├── Cargo.toml           # Workspace manifest
├── src/
│   ├── lib.rs           # Library root
│   ├── bin/engram.rs     # Daemon binary entry
│   ├── server/          # HTTP/SSE layer
│   ├── db/              # SurrealDB layer
│   ├── models/          # Domain entities
│   ├── services/        # Business logic
│   ├── tools/           # MCP tool implementations
│   ├── errors/          # Error types
│   └── config/          # Configuration
├── tests/               # Integration tests
└── specs/               # Feature specifications
```

---

## Running the Daemon

### Development Mode

```bash
# Start with debug logging
RUST_LOG=engram=debug cargo run

# Start on specific port
PORT=7437 cargo run

# Start with custom workspace limit and stale-file strategy
cargo run -- --max-workspaces 5 --stale-strategy rehydrate
```

### Configuration Options

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--port` | `ENGRAM_PORT` | `7437` | Listening port on 127.0.0.1 |
| `--max-workspaces` | `ENGRAM_MAX_WORKSPACES` | `10` | Max concurrent active workspaces |
| `--request-timeout-ms` | `ENGRAM_REQUEST_TIMEOUT_MS` | `60000` | Request timeout in milliseconds |
| `--log-format` | `ENGRAM_LOG_FORMAT` | `pretty` | Tracing output: `json` or `pretty` |
| `--stale-strategy` | `ENGRAM_STALE_STRATEGY` | `warn` | Stale `.engram/` file behavior: `warn`, `rehydrate`, `fail` |
| `--data-dir` | `ENGRAM_DATA_DIR` | `~/.local/share/engram/` | SurrealDB and model cache directory |

### Testing with curl

```bash
# Connect to SSE endpoint
curl -N http://127.0.0.1:7437/sse

# Send MCP tool call (in another terminal)
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_daemon_status",
      "arguments": {}
    },
    "id": 1
  }'
```

---

## Development Workflow

### 1. Create Feature Branch

```bash
git checkout -b 001-core-mcp-daemon
```

### 2. Write Tests First (TDD)

```rust
// tests/contract/lifecycle_test.rs
#[tokio::test]
async fn test_set_workspace_valid_path() {
    let daemon = TestDaemon::spawn().await;
    let response = daemon.call("set_workspace", json!({
        "path": "/tmp/test-repo"
    })).await;
    
    assert!(response.is_ok());
    assert!(response["hydrated"].as_bool().unwrap());
}

#[tokio::test]
async fn test_set_workspace_invalid_path() {
    let daemon = TestDaemon::spawn().await;
    let response = daemon.call("set_workspace", json!({
        "path": "/nonexistent"
    })).await;
    
    assert!(response.is_err());
    assert_eq!(response.error_code(), 1001);
}
```

### 3. Run Tests (Expect Failure)

```bash
cargo test test_set_workspace
# Should fail - not yet implemented
```

### 4. Implement Feature

```rust
// src/tools/lifecycle.rs
pub async fn set_workspace(
    state: &AppState,
    path: String,
) -> Result<WorkspaceResult, EngramError> {
    let canonical = std::fs::canonicalize(&path)
        .map_err(|_| WorkspaceError::NotFound { path: path.clone() })?;
    
    if !canonical.join(".git").is_dir() {
        return Err(WorkspaceError::NotGitRoot { 
            path: canonical.display().to_string() 
        }.into());
    }
    
    // ... hydration logic
}
```

### 5. Run Tests (Expect Pass)

```bash
cargo test test_set_workspace
# Should pass now
```

### 6. Lint and Format

```bash
cargo fmt
cargo clippy -- -D warnings
```

### 7. Commit

```bash
git add -A
git commit -m "feat(lifecycle): implement set_workspace tool"
```

---

## Testing Guide

### Unit Tests

Co-located with source files:

```rust
// src/services/hydration.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_task_markdown() {
        let md = "## task:abc123\n---\nid: task:abc123\n---\n";
        let task = parse_task_block(md).unwrap();
        assert_eq!(task.id, "task:abc123");
    }
}
```

### Integration Tests

Full daemon tests in `tests/`:

```rust
// tests/integration/round_trip_test.rs
#[tokio::test]
async fn test_hydration_dehydration_round_trip() {
    let repo = TempGitRepo::new();
    repo.write_ENGRAM_tasks(sample_tasks());
    
    let daemon = TestDaemon::spawn().await;
    daemon.set_workspace(repo.path()).await.unwrap();
    
    // Modify state
    daemon.update_task("task:1", "in_progress", "Starting").await.unwrap();
    
    // Flush
    daemon.flush_state().await.unwrap();
    
    // Verify file content
    let content = repo.read_ENGRAM_tasks();
    assert!(content.contains("status: in_progress"));
}
```

### Property Tests

Serialization round-trips:

```rust
// tests/unit/proptest_models.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn task_roundtrip(task in arb_task()) {
        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(task, parsed);
    }
}
```

---

## Common Tasks

### Add a New MCP Tool

1. Define schema in `contracts/mcp-tools.json`
2. Add error codes in `contracts/error-codes.md`
3. Write contract tests in `tests/contract/`
4. Implement in `src/tools/`
5. Register in MCP router

### Add a New Entity

1. Define in `data-model.md`
2. Add Rust struct in `src/models/`
3. Add SurrealDB schema in `src/db/schema.rs`
4. Add serialization tests

### Debug Database Issues

```bash
# Connect to embedded SurrealDB
# (requires surreal CLI)
surreal sql --conn file://~/.local/share/engram/db/{workspace_hash}

# Query tasks
SELECT * FROM task;

# Check relationships
SELECT * FROM depends_on;
```

---

## Troubleshooting

### Build Failures

```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update
```

### Test Failures

```bash
# Run with verbose output
cargo test -- --nocapture

# Run single test
cargo test test_name -- --exact
```

### Performance Issues

```bash
# Build with release optimizations
cargo build --release

# Profile with flamegraph
cargo install flamegraph
cargo flamegraph --bin engram
```

---

## Resources

- [Feature Spec](spec.md) — User stories and requirements
- [Implementation Plan](plan.md) — Technical approach
- [Research](research.md) — Technology decisions
- [Data Model](data-model.md) — Entity definitions
- [MCP Tools](contracts/mcp-tools.json) — API contracts
- [Error Codes](contracts/error-codes.md) — Error taxonomy
- [Constitution](../../.specify/memory/constitution.md) — Development principles

### Contract: Error Codes

# Error Codes: engram MCP Daemon

**Version**: 0.1.0
**Purpose**: Define structured error codes for MCP tool responses

## Error Response Format

All errors follow this structure:

```json
{
  "error": {
    "code": 1001,
    "name": "WorkspaceNotFound",
    "message": "Human-readable error description",
    "details": {
      "additional": "context-specific fields"
    }
  }
}
```

## Error Categories

### 1xxx: Workspace Errors

Errors related to workspace binding and path validation.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 1001 | `WorkspaceNotFound` | Specified path does not exist | No | Verify path |
| 1002 | `NotAGitRoot` | Path exists but lacks `.git/` directory | No | Use Git repo root |
| 1003 | `WorkspaceNotSet` | Tool requires workspace but `set_workspace` not called | No | Call `set_workspace` first |
| 1004 | `WorkspaceAlreadyActive` | `set_workspace` called with same path (warning) | N/A | Proceed normally |
| 1005 | `WorkspaceLimitReached` | Maximum concurrent workspaces reached (default: 10) | No | Release an existing workspace or increase `--max-workspaces` |

**Example: WorkspaceNotFound**
```json
{
  "error": {
    "code": 1001,
    "name": "WorkspaceNotFound",
    "message": "Path '/invalid/path' does not exist",
    "details": {
      "path": "/invalid/path",
      "suggestion": "Verify the path exists and is accessible"
    }
  }
}
```

---

### 2xxx: Hydration Errors

Errors during workspace state loading from `.engram/` files.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 2001 | `HydrationFailed` | Failed to parse `.engram/` files | No | Fix file syntax |
| 2002 | `SchemaMismatch` | `.engram/` version incompatible with daemon | No | Upgrade daemon or migrate files |
| 2003 | `CorruptedState` | Database or file integrity check failed | Auto | Re-hydrate from files |
| 2004 | `StaleWorkspace` | External modifications detected (warning) | N/A | Consider re-hydrate |

**Example: HydrationFailed**
```json
{
  "error": {
    "code": 2001,
    "name": "HydrationFailed",
    "message": "Failed to parse tasks.md",
    "details": {
      "file": ".engram/tasks.md",
      "line": 42,
      "error": "Invalid YAML frontmatter: missing 'id' field"
    }
  }
}
```

---

### 3xxx: Task Errors

Errors during task operations.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 3001 | `TaskNotFound` | Task ID does not exist | No | Verify task ID |
| 3002 | `InvalidStatus` | Status value not in allowed set | No | Use valid status |
| 3003 | `CyclicDependency` | Adding dependency would create cycle | No | Remove conflicting edge |
| 3004 | `BlockerExists` | Task already has active blocker | No | Clear existing blocker first |
| 3005 | `TaskTitleEmpty` | Task title is empty or exceeds 200 chars | No | Provide valid title |

**Example: TaskNotFound**
```json
{
  "error": {
    "code": 3001,
    "name": "TaskNotFound",
    "message": "Task 'task:nonexistent' does not exist",
    "details": {
      "task_id": "task:nonexistent",
      "suggestion": "Use get_task_graph to list available tasks"
    }
  }
}
```

**Example: CyclicDependency**
```json
{
  "error": {
    "code": 3003,
    "name": "CyclicDependency",
    "message": "Adding dependency would create cycle",
    "details": {
      "from": "task:abc123",
      "to": "task:def456",
      "cycle_path": ["task:def456", "task:ghi789", "task:abc123"]
    }
  }
}
```

---

### 4xxx: Query Errors

Errors during semantic search operations.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 4001 | `QueryTooLong` | Query exceeds maximum token limit | No | Shorten query |
| 4002 | `ModelNotLoaded` | Embedding model failed to initialize | Yes | Retry or check disk/network |
| 4003 | `SearchFailed` | Vector/keyword search internal error | Yes | Retry |

**Example: ModelNotLoaded**
```json
{
  "error": {
    "code": 4002,
    "name": "ModelNotLoaded",
    "message": "Failed to load embedding model",
    "details": {
      "model": "all-MiniLM-L6-v2",
      "cache_path": "~/.local/share/engram/models/",
      "reason": "Download failed: network timeout",
      "suggestion": "Check network connection or manually download model"
    }
  }
}
```

---

### 5xxx: System Errors

Internal system errors.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 5001 | `DatabaseError` | SurrealDB operation failed | Yes | Retry or check logs |
| 5002 | `FlushFailed` | Could not write to `.engram/` directory | No | Check permissions |
| 5003 | `RateLimited` | Too many requests from connection | Yes | Back off and retry |
| 5004 | `ShuttingDown` | Daemon is in graceful shutdown | No | Reconnect after restart |

**Example: FlushFailed**
```json
{
  "error": {
    "code": 5002,
    "name": "FlushFailed",
    "message": "Failed to write workspace state",
    "details": {
      "path": "/repo/.engram/tasks.md",
      "reason": "Permission denied",
      "suggestion": "Check file permissions for .engram/ directory"
    }
  }
}
```

---

## Error Handling Guidelines

### For Clients

1. **Check error code first** — use code for programmatic handling
2. **Display message to users** — message is human-readable
3. **Log details for debugging** — details contain diagnostic info
4. **Retry on 4002, 4003, 5001, 5003** — transient errors may recover

### For Daemon Implementation

1. **Never expose internal errors** — wrap all errors in typed responses
2. **Include actionable suggestions** — tell users how to recover
3. **Log full stack traces internally** — emit to tracing, not MCP response
4. **Use correlation IDs** — link MCP error to internal log entries

---

## Rust Error Type Mapping

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Path '{path}' does not exist")]
    NotFound { path: String },
    
    #[error("Path '{path}' is not a Git repository root")]
    NotGitRoot { path: String },
    
    #[error("No workspace bound to this connection")]
    NotSet,
    
    #[error("Workspace '{path}' already active")]
    AlreadyActive { path: String },
    
    #[error("Maximum concurrent workspaces reached (limit: {limit})")]
    LimitReached { limit: usize },
}

impl WorkspaceError {
    pub fn code(&self) -> u16 {
        match self {
            Self::NotFound { .. } => 1001,
            Self::NotGitRoot { .. } => 1002,
            Self::NotSet => 1003,
            Self::AlreadyActive { .. } => 1004,
            Self::LimitReached { .. } => 1005,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "WorkspaceNotFound",
            Self::NotGitRoot { .. } => "NotAGitRoot",
            Self::NotSet => "WorkspaceNotSet",
            Self::AlreadyActive { .. } => "WorkspaceAlreadyActive",
            Self::LimitReached { .. } => "WorkspaceLimitReached",
        }
    }
}
```

### Contract: Mcp Tools

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "engram MCP Tools",
  "description": "Model Context Protocol tool definitions for engram daemon",
  "version": "0.1.0",
  "tools": {
    "set_workspace": {
      "idempotent": true,
      "description": "Bind the connection to a specific Git repository workspace. Must be called before any workspace-scoped operations. Idempotent: re-binding to the same workspace is a no-op.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": {
            "type": "string",
            "description": "Absolute path to Git repository root directory"
          }
        },
        "required": ["path"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "workspace_id": {
            "type": "string",
            "description": "Hash identifier for the workspace"
          },
          "path": {
            "type": "string",
            "description": "Canonicalized workspace path"
          },
          "task_count": {
            "type": "integer",
            "description": "Number of tasks in workspace"
          },
          "hydrated": {
            "type": "boolean",
            "description": "Whether workspace was freshly hydrated from files"
          }
        },
        "required": ["workspace_id", "path", "task_count", "hydrated"]
      },
      "errors": [1001, 1002, 1004, 1005, 2001, 2002, 2003]
    },
    "get_daemon_status": {
      "idempotent": true,
      "description": "Get daemon health and operational metrics. Does not require workspace binding.",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "version": {
            "type": "string",
            "description": "Daemon version string"
          },
          "uptime_seconds": {
            "type": "integer",
            "description": "Seconds since daemon started"
          },
          "active_workspaces": {
            "type": "integer",
            "description": "Number of active workspace bindings"
          },
          "active_connections": {
            "type": "integer",
            "description": "Number of connected clients"
          },
          "memory_bytes": {
            "type": "integer",
            "description": "Current memory usage in bytes"
          },
          "model_loaded": {
            "type": "boolean",
            "description": "Whether embedding model is loaded"
          },
          "model_name": {
            "type": "string",
            "description": "Name of loaded embedding model"
          }
        },
        "required": ["version", "uptime_seconds", "active_workspaces", "active_connections", "memory_bytes", "model_loaded"]
      },
      "errors": []
    },
    "get_workspace_status": {
      "idempotent": true,
      "description": "Get status of currently bound workspace.",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "path": {
            "type": "string",
            "description": "Workspace path"
          },
          "task_count": {
            "type": "integer",
            "description": "Total number of tasks"
          },
          "context_count": {
            "type": "integer",
            "description": "Total number of context nodes"
          },
          "last_flush": {
            "type": "string",
            "format": "date-time",
            "description": "Timestamp of last dehydration"
          },
          "stale_files": {
            "type": "boolean",
            "description": "Whether .engram/ files have external modifications"
          },
          "connection_count": {
            "type": "integer",
            "description": "Number of connections to this workspace"
          }
        },
        "required": ["path", "task_count", "context_count", "stale_files", "connection_count"]
      },
      "errors": [1003]
    },
    "query_memory": {
      "idempotent": true,
      "description": "Perform hybrid semantic + keyword search across specs and context.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": {
            "type": "string",
            "description": "Natural language search query",
            "maxLength": 2000
          },
          "limit": {
            "type": "integer",
            "description": "Maximum results to return",
            "default": 10,
            "minimum": 1,
            "maximum": 100
          }
        },
        "required": ["query"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "results": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": {
                  "type": "string",
                  "description": "Record ID"
                },
                "type": {
                  "type": "string",
                  "enum": ["spec", "context"],
                  "description": "Result type"
                },
                "content": {
                  "type": "string",
                  "description": "Matched content snippet"
                },
                "score": {
                  "type": "number",
                  "description": "Relevance score (0-1)"
                }
              },
              "required": ["id", "type", "content", "score"]
            }
          },
          "total_matches": {
            "type": "integer",
            "description": "Total matching documents before limit"
          }
        },
        "required": ["results", "total_matches"]
      },
      "errors": [1003, 4001, 4002, 4003]
    },
    "get_task_graph": {
      "idempotent": true,
      "description": "Get task dependency tree starting from a root task.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "root_task_id": {
            "type": "string",
            "description": "Task ID to start from (e.g., task:abc123)"
          },
          "depth": {
            "type": "integer",
            "description": "Maximum traversal depth",
            "default": 5,
            "minimum": 1,
            "maximum": 20
          }
        },
        "required": ["root_task_id"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "root": {
            "$ref": "#/$defs/TaskNode"
          }
        },
        "required": ["root"]
      },
      "errors": [1003, 3001]
    },
    "check_status": {
      "idempotent": true,
      "description": "Get current status for work item IDs.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "work_item_ids": {
            "type": "array",
            "items": {
              "type": "string"
            },
            "description": "List of external work item IDs to check"
          }
        },
        "required": ["work_item_ids"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "statuses": {
            "type": "object",
            "additionalProperties": {
              "type": "object",
              "properties": {
                "task_id": {
                  "type": "string",
                  "description": "Internal task ID"
                },
                "status": {
                  "type": "string",
                  "enum": ["todo", "in_progress", "done", "blocked"]
                },
                "updated_at": {
                  "type": "string",
                  "format": "date-time"
                }
              }
            },
            "description": "Map of work_item_id to status info"
          }
        },
        "required": ["statuses"]
      },
      "errors": [1003]
    },
    "create_task": {
      "idempotent": false,
      "description": "Create a new task in the current workspace. New tasks default to 'todo' status. Non-idempotent: each call creates a new task.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "title": {
            "type": "string",
            "description": "Brief description of the task",
            "maxLength": 200
          },
          "description": {
            "type": "string",
            "description": "Detailed task description"
          },
          "parent_task_id": {
            "type": "string",
            "description": "Optional parent task ID to create a subtask (e.g., task:abc123)"
          },
          "work_item_id": {
            "type": "string",
            "description": "Optional external work item reference (e.g., AB#12345)"
          }
        },
        "required": ["title"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Generated task ID (e.g., task:uuid)"
          },
          "title": {
            "type": "string",
            "description": "Task title as stored"
          },
          "status": {
            "type": "string",
            "enum": ["todo"],
            "description": "Initial status (always 'todo')"
          },
          "parent_task_id": {
            "type": "string",
            "description": "Parent task ID if subtask was created"
          },
          "created_at": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["task_id", "title", "status", "created_at"]
      },
      "errors": [1003, 3001, 3003]
    },
    "update_task": {
      "idempotent": true,
      "description": "Update task status and append progress notes. Idempotent: updating to the same status is a no-op for status but appends a new context note if notes provided.",
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Updated task ID"
          },
          "previous_status": {
            "type": "string",
            "description": "Status before update"
          },
          "new_status": {
            "type": "string",
            "description": "Status after update"
          },
          "context_id": {
            "type": "string",
            "description": "ID of created context node (if notes provided)"
          },
          "updated_at": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["task_id", "previous_status", "new_status", "updated_at"]
      },
      "errors": [1003, 3001, 3002]
    },
    "add_blocker": {
      "idempotent": false,
      "description": "Set task status to blocked and record reason. Non-idempotent: each call creates a new blocker context node.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID to block"
          },
          "reason": {
            "type": "string",
            "description": "Explanation of why task is blocked"
          }
        },
        "required": ["task_id", "reason"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string"
          },
          "blocker_context_id": {
            "type": "string",
            "description": "ID of context node with blocker reason"
          },
          "updated_at": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["task_id", "blocker_context_id", "updated_at"]
      },
      "errors": [1003, 3001, 3004]
    },
    "register_decision": {
      "idempotent": false,
      "description": "Record an architectural decision. Non-idempotent: each call creates a new decision context node.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "topic": {
            "type": "string",
            "description": "Decision topic/title"
          },
          "decision": {
            "type": "string",
            "description": "The decision content and rationale"
          }
        },
        "required": ["topic", "decision"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "decision_id": {
            "type": "string",
            "description": "ID of created decision context"
          },
          "created_at": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["decision_id", "created_at"]
      },
      "errors": [1003]
    },
    "flush_state": {
      "idempotent": true,
      "description": "Dehydrate workspace state to .engram/ files. Idempotent: flushing unchanged state produces the same files.",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "files_written": {
            "type": "array",
            "items": {
              "type": "string"
            },
            "description": "List of files written"
          },
          "warnings": {
            "type": "array",
            "items": {
              "type": "string"
            },
            "description": "Any warnings during flush"
          },
          "flush_timestamp": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["files_written", "warnings", "flush_timestamp"]
      },
      "errors": [1003, 2004, 5001, 5002]
    }
  },
  "$defs": {
    "TaskNode": {
      "type": "object",
      "properties": {
        "id": {
          "type": "string"
        },
        "title": {
          "type": "string"
        },
        "status": {
          "type": "string",
          "enum": ["todo", "in_progress", "done", "blocked"]
        },
        "dependencies": {
          "type": "array",
          "items": {
            "$ref": "#/$defs/TaskNode"
          }
        },
        "blockers": {
          "type": "array",
          "items": {
            "$ref": "#/$defs/TaskNode"
          }
        }
      },
      "required": ["id", "title", "status"]
    }
  }
}
<!-- SECTION:NOTES:END -->
