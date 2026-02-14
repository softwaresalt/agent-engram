# Full Specification Quality Checklist: T-Mem Core MCP Daemon

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
- [ ] CHK011 - Are requirements defined for what files `set_workspace` creates when initializing a new `.tmem/` directory? [Completeness, Gap — US3 scenario 3 says "initializes empty workspace structure" but doesn't enumerate which files]
- [ ] CHK012 - Are requirements defined for `.tmem/.version` file handling — migration behavior when version mismatch is detected? [Completeness, Gap — data-model.md §Schema Migration describes forward migration but spec.md has no FR for it]
- [ ] CHK013 - Are requirements defined for `max_workspaces` behavior when the same workspace is re-bound by a different client — does it count as one or two? [Completeness, Gap — FR-009a says "concurrent active workspaces" but doesn't define counting semantics]
- [ ] CHK014 - Is the `work_item_id` format validation requirement (ADO `AB#\d+` or GitHub `[\w-]+/[\w-]+#\d+`) specified in spec.md, or only in data-model.md? [Completeness, Gap — FR-017 says "reference storage only" without format validation; data-model.md adds regex constraint]
- [ ] CHK015 - Are requirements defined for what happens when a workspace path is valid but the `.tmem/` directory contains unparseable files? [Completeness, Gap — US3 scenario 4 covers corrupted DB but not corrupted `.tmem/` files]

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
- [ ] CHK045 - Are requirements defined for what `flush_state` does when the workspace has no tasks and no context — empty `.tmem/` files? [Coverage, Gap — US3 covers populated workspace; empty-state flush unspecified]
- [ ] CHK046 - Are requirements defined for `query_memory` when the workspace has zero searchable documents (no specs, tasks, or context)? [Coverage, Gap — US4 assumes populated workspace]
- [ ] CHK047 - Are requirements defined for the daemon receiving MCP calls before any SSE connection is established (HTTP POST to `/mcp` without SSE)? [Coverage, Gap — protocol flow assumes SSE first]
- [ ] CHK048 - Are requirements defined for `register_decision` uniqueness — can two decisions with the same topic be registered? [Coverage, Gap — idempotency annotation says non-idempotent but no dedup requirement exists]

## Edge Case Coverage

- [ ] CHK049 - Are requirements defined for workspace paths at OS max path length (260 chars Windows, 4096 Linux)? [Edge Case, Gap]
- [ ] CHK050 - Are requirements defined for task title containing special characters (newlines, null bytes, Unicode, markdown syntax)? [Edge Case, Gap — data-model.md says max 200 chars but no character set restriction]
- [ ] CHK051 - Are requirements defined for `.tmem/tasks.md` containing tasks that were deleted from the database but still exist in the file? [Edge Case, Gap — hydration behavior for orphan file entries unspecified]
- [ ] CHK052 - Are requirements defined for `flush_state` when the filesystem is read-only or disk is full? [Edge Case, Gap — error code 5001/5002 exist but no FR describes the behavior]
- [ ] CHK053 - Are requirements defined for SSE connection behavior when the daemon is under memory pressure (approaching 500MB limit)? [Edge Case, Gap — SC-006 sets idle limit but load ceiling behavior unspecified]
- [ ] CHK054 - Are requirements defined for hydration when `.tmem/graph.surql` references task IDs not present in `.tmem/tasks.md`? [Edge Case, Gap — orphan edge handling unspecified]
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
