# Research: Enhanced Task Management

**Phase**: 0 — Outline & Research
**Created**: 2026-02-11
**Purpose**: Resolve all NEEDS CLARIFICATION items and document technology decisions

## Research Tasks

### R1: TOML Configuration Parsing

**Decision**: Use the `toml` crate (v0.8+) for parsing `.tmem/config.toml`.

**Rationale**: The `toml` crate is the de facto standard Rust TOML parser, directly compatible with serde derive. It supports nested tables natively (e.g., `[compaction]` section maps to nested struct or flat fields via `#[serde(rename)]`). Alternatives like `toml_edit` preserve formatting but add complexity unnecessary for read-only config loading.

**Alternatives Considered**:
- `toml_edit`: Preserves comments and formatting on write-back. Rejected because config.toml is read-only (t-mem never writes config).
- `serde_json` with JSON config: Rejected because TOML is more human-readable for workspace config files committed to Git.
- Environment variables only: Rejected because per-workspace configuration requires file-based config, not process-level env vars.

### R2: TOML Nested Table to Flat Struct Mapping

**Decision**: Use flat `WorkspaceConfig` struct with `#[serde(rename)]` attributes for nested TOML keys.

**Rationale**: The spec defines nested TOML keys like `compaction.threshold_days` and `batch.max_size`. Two approaches exist:
1. **Nested sub-structs** (`CompactionConfig`, `BatchConfig`) with `#[serde(flatten)]` — more idiomatic TOML but adds types.
2. **Flat struct with `#[serde(rename)]`** — simpler, fewer types, matches the single `WorkspaceConfig` entity definition.

Approach 2 was chosen per the spec's Assumptions section: "Nested TOML configuration keys map to flat WorkspaceConfig struct fields via serde rename attributes." This keeps the model layer simple (Constitution IX).

**Alternatives Considered**:
- Nested sub-structs: More Rust-idiomatic for deeply nested config. Rejected because the config is shallow (only 2 levels) and the flat struct is simpler.

**Update**: After further analysis, the `toml` crate desrializes `[compaction]` sections into nested structs naturally. A hybrid approach works best: use small inner structs (`CompactionConfig`, `BatchConfig`) that the `toml` crate maps directly, then expose flat accessor methods on `WorkspaceConfig`. This avoids `#[serde(rename)]` complexity while keeping the public API flat.

### R3: Ready-Work Query Performance

**Decision**: Use a single SurrealQL query with inline subqueries for blocking/deferral checks.

**Rationale**: The ready-work query must filter by status, defer_until, blocking dependencies, duplicate_of edges, pinned status, and optional filters (label, priority, type, assignee) — all in a single query returning sorted results. SurrealDB supports subqueries in WHERE clauses and multi-column ORDER BY.

**Approach**:
```surql
SELECT * FROM task
WHERE status NOT IN ['done', 'blocked']
  AND (defer_until IS NULL OR defer_until <= time::now())
  AND id NOT IN (SELECT in FROM depends_on WHERE type IN ['hard_blocker', 'blocked_by'] AND out.status != 'done')
  AND id NOT IN (SELECT in FROM depends_on WHERE type = 'duplicate_of')
ORDER BY pinned DESC, priority_order ASC, created_at ASC
LIMIT $limit
```

Optional filters are appended dynamically via parameterized query building in the `Queries` struct. Each filter dimension (label, priority, type, assignee) adds one WHERE clause.

**Performance Target**: <50ms for <1000 tasks (SC-011). SurrealDB indexes on `task_status`, `task_priority`, `task_defer_until`, and `task_assignee` keep this within budget.

**Alternatives Considered**:
- Multiple queries with application-level join: Rejected because it increases round-trips and complicates sorting.
- Materialized view / computed field for "ready" status: Rejected as YAGNI — the query approach is fast enough and avoids dual-write complexity.

### R4: Agent-Driven Compaction Strategy

**Decision**: Two-phase MCP flow with rule-based truncation fallback.

**Rationale**: Agents call `get_compaction_candidates()` → receive eligible tasks → generate summaries externally → call `apply_compaction(summaries)`. This avoids embedding an LLM in t-mem or managing API keys. For non-agent callers (e.g., CI pipelines), a rule-based truncation fallback truncates descriptions to 500 characters at word boundaries.

**Key Design Choices**:
- Compaction is **one-way**: original content is not recoverable from t-mem (exists in Git history via `.tmem/tasks.md` commits).
- `compaction_level` counter increments on each application, allowing agents to detect already-compacted tasks.
- Pinned tasks are excluded from candidates (they serve as permanent context).
- Graph relationships (all edge types) are preserved — only description/content is compressed.

**Alternatives Considered**:
- Embedded local LLM (e.g., via candle): Rejected because it adds >500MB model weight, GPU dependency, and contradicts the spec's explicit decision.
- API key-based summarization in t-mem: Rejected per spec — the calling agent provides summaries.
- Automatic compaction on `flush_state`: Rejected because compaction requires agent judgment for quality summaries.

### R5: Claim Semantics and Conflict Resolution

**Decision**: Last-write-wins with explicit rejection on already-claimed tasks.

**Rationale**: Task claiming uses a simple assignee field. The DB query `UPDATE task SET assignee = $claimant WHERE id = $task_id AND assignee IS NULL` is atomic within SurrealDB. If the field is already set, the handler checks the current claimant and returns error 3005. Any client can release any claim (no ownership restriction) to prevent stale locks from crashed agents.

**Alternatives Considered**:
- Optimistic concurrency with version counter: Rejected as over-engineering for the single-user daemon model.
- TTL-based auto-expiring claims: Rejected for v0 per spec — adds complexity; manual `release_task` is sufficient.
- Owner-only release: Rejected per Clarification Q3 — any client can release to handle crashed agents.

### R6: Label Storage Design

**Decision**: Separate `label` table with task_id foreign key, not an array field on task.

**Rationale**: Labels need efficient multi-label AND filtering (`SELECT task_id FROM label WHERE name IN $names GROUP BY task_id HAVING count() = $count`). A separate table enables this with standard SQL/SurrealQL grouping. An array field on task would require array intersection logic that SurrealDB supports less efficiently.

**Trade-off**: Slightly higher write overhead (INSERT into label table vs. array append) but significantly better query performance for filtering operations which are the primary use case.

**Serialization**: Despite separate DB storage, labels are serialized as a `labels` array in task YAML frontmatter (FR-031b) for human readability. Hydration populates the label table from the array; dehydration queries labels per task and writes them back to the array.

**Alternatives Considered**:
- Array field on task: Simpler storage but poor query performance for AND-filtering across multiple labels. Rejected.
- Junction table with label_definition: Over-normalized for free-form strings. Rejected.

### R7: Comment Storage and Serialization

**Decision**: Separate `comment` table in DB, serialized to `.tmem/comments.md` file.

**Rationale**: Comments are append-only discussion entries separate from context notes (which track system events). Storing them in a separate table keeps the context table clean. Serialization to a dedicated `.tmem/comments.md` file avoids bloating task frontmatter and allows easy human review of discussions.

**File Format**:
```markdown
## task:abc123

### 2026-02-11T10:30:00Z — agent-1

Fixed the authentication flow by switching to JWT tokens.

### 2026-02-11T11:00:00Z — developer

Confirmed — now passes integration tests.

---

## task:def456

### 2026-02-11T12:00:00Z — orchestrator

Spike complete. Recommend approach B per ADR-003.
```

**Alternatives Considered**:
- Inline in task frontmatter: Bloats task entries; rejected for readability.
- Separate file per task: Too many small files; rejected.
- Combined with context notes: Muddies the distinction between system audit trail and human discussion; rejected per spec.

### R8: Batch Operation Atomicity

**Decision**: Per-item atomicity, not all-or-nothing.

**Rationale**: `batch_update_tasks` iterates over items and applies each update individually using the existing `update_task` logic. If one item fails (e.g., invalid task ID), the others still succeed. The response includes per-item success/failure results. Error 3007 (`BatchPartialFailure`) is returned if any item fails.

This matches the spec's acceptance scenario US9-2: "valid updates succeed, the invalid one returns an error, and the response includes per-item results."

**Alternatives Considered**:
- All-or-nothing transaction: SurrealDB supports transactions, but the spec explicitly requires per-item results with partial success. Rejected.
- Fire-and-forget: No per-item feedback; rejected because debugging batch failures requires per-item results.

### R9: Priority Sorting with Ordinal Extraction

**Decision**: Parse numeric suffix from priority string for sorting, with ASC ordering.

**Rationale**: Priorities are stored as strings (`"p0"`, `"p1"`, `"p4"`, possibly `"p10"`) per spec. SurrealDB can sort strings, but `"p10"` would sort before `"p2"` lexicographically. The ready-work query must extract the numeric suffix and sort numerically.

**Approach**: Use SurrealDB's `string::slice` or `math::int` functions in the ORDER BY clause, or store the numeric value as an additional indexed field during task creation. The simpler approach: store a `priority_order` integer field alongside the `priority` string, set during write operations. This avoids runtime parsing in every query.

**Final Decision**: Use a `priority_order: u32` derived field, computed and stored on any priority write. The ready-work query sorts by `priority_order ASC`. This keeps the query simple and fast.

**Alternatives Considered**:
- Runtime string parsing in SurrealQL: Possible but fragile; SurrealDB's string functions are limited. Rejected.
- Enum-based priority: Too rigid for custom priority levels defined in config. Rejected.
