# Feature Specification: Unified Code Knowledge Graph

**Feature Branch**: `003-unified-code-graph`  
**Created**: 2026-02-11  
**Status**: Draft  
**Input**: Unified code knowledge graph: AST-based code structure indexing, graph-backed dependency walking, cross-region task-to-code linking, and structured retrieval for grounded agent context

## User Scenarios & Testing *(mandatory)*

<!--
  User stories derived from the context width problem in AI agent workflows.
  Features 001 (core daemon) and 002 (enhanced task management) address context depth
  and partial context width via task/plan data compaction. This feature addresses context
  width in the code domain by building a structural code graph that enables precise,
  graph-navigated retrieval instead of brute-force vector search.

  The unified schema places code graph nodes (Region A: Spatial Memory) alongside
  task graph nodes (Region B: Temporal Memory) in the same SurrealDB address space,
  linked by cross-region "concerns" edges. This enables grounded queries that span
  both domains simultaneously.
-->

### User Story 1 - Code Structure Indexing (Priority: P1)

As a developer or orchestrator, I index a project workspace so that the code structure (files, functions, classes, interfaces, and their relationships) is stored as a navigable graph, enabling precise retrieval without reading entire source files.

**Why this priority**: This is the foundational capability. Without a populated code graph, none of the downstream traversal, linking, or retrieval features can function. Indexing transforms raw source code into structured, queryable knowledge.

**Independent Test**: Point the indexer at a workspace containing 50 Rust source files. Call `index_workspace`. Verify that file, function, class, and interface nodes exist in the graph with correct `calls`, `imports`, and `inherits_from` edges. Verify that each function node has an embedding generated from its summary.

**Acceptance Scenarios**:

1. **Given** a workspace with source files, **When** `index_workspace()` is called, **Then** the system parses each file and creates graph nodes for files, functions, classes, and interfaces with their attributes (name, file path, line range, signature, docstring)
2. **Given** a parsed source file with function calls, **When** indexing completes, **Then** `calls` edges exist between caller and callee function nodes
3. **Given** a parsed source file with import statements, **When** indexing completes, **Then** `imports` edges exist between the importing file node and the imported module or symbol nodes
4. **Given** a class that extends another class, **When** indexing completes, **Then** an `inherits_from` edge exists between the child class and the parent class
5. **Given** a function node whose source body fits within the embedding model's token limit, **When** indexing completes, **Then** the node is tagged `explicit_code` and its embedding is generated from the raw source body
6. **Given** a function node whose source body exceeds the embedding model's token limit, **When** indexing completes, **Then** the node is tagged `summary_pointer`, its embedding is generated from the function signature and docstring summary only, and the full source body is stored separately for retrieval
7. **Given** a `summary_pointer` node matched by a vector search, **When** the result is returned to the caller, **Then** the system returns the full stored source body (not the summary used for embedding)
8. **Given** a workspace with files in unsupported languages, **When** `index_workspace()` is called, **Then** unsupported files are skipped with a warning and indexing continues for supported files

---

### User Story 2 - Graph-Backed Dependency Walking (Priority: P2)

As an AI agent preparing to modify code, I retrieve the structural neighborhood of a symbol so that my prompt contains exactly the functions, classes, and files that are directly relevant, drastically reducing context pollution.

**Why this priority**: This is the primary value delivery. Once the code graph exists, dependency walking converts it from static storage into an active context-pruning engine. An agent asking "what do I need to know about `process_payment`?" receives precisely the call tree and dependents rather than 10 loosely-related vector chunks.

**Independent Test**: Index a workspace, then call `map_code("process_payment")`. Verify the result contains the function definition plus all direct callers, callees, and type dependencies within the requested traversal depth. Verify that increasing depth to 2 includes transitive dependencies.

**Acceptance Scenarios**:

1. **Given** an indexed workspace with a function `process_payment`, **When** `map_code("process_payment")` is called, **Then** the system returns the function node plus all nodes reachable via 1-hop `calls`, `imports`, and `inherits_from` edges
2. **Given** a `map_code` call with `depth: 2`, **When** the function has transitive dependencies, **Then** the system traverses 2 hops and returns the expanded neighborhood
3. **Given** a symbol name that matches multiple nodes (e.g., overloaded functions in different files), **When** `map_code` is called, **Then** the system returns results for all matches, grouped by file
4. **Given** a symbol name that does not exist in the graph, **When** `map_code` is called, **Then** the system falls back to vector search across function summaries and returns the closest semantic matches
5. **Given** a `map_code` call with `depth: 3` on a highly connected node, **When** the traversal result exceeds a configurable node limit (default: 50), **Then** the system truncates results at the limit, prioritizing direct dependencies, and includes a `truncated: true` indicator

---

### User Story 3 - Incremental Code Sync (Priority: P3)

As a developer committing code changes, I expect the code graph to stay current without re-indexing the entire workspace, so that retrieval always reflects the latest code state.

**Why this priority**: Full re-indexing is expensive for large codebases. Incremental sync keeps the graph fresh by processing only changed files, making the system practical for active development workflows.

**Independent Test**: Index a workspace, modify 3 files and delete 1 file, then call `sync_workspace`. Verify that only the 4 affected files are re-parsed, their old nodes and edges are replaced, the deleted file's nodes are removed, and all other nodes remain unchanged.

**Acceptance Scenarios**:

1. **Given** a previously indexed workspace with file modifications, **When** `sync_workspace()` is called, **Then** the system detects changed files (via mtime or content hash), re-parses only those files, and updates their graph nodes and edges
2. **Given** a deleted source file, **When** `sync_workspace()` is called, **Then** all nodes and edges originating from that file are removed from the graph
3. **Given** a newly added source file, **When** `sync_workspace()` is called, **Then** the new file is parsed and its nodes and edges are added to the graph
4. **Given** a renamed file (detected as delete + add), **When** `sync_workspace()` is called, **Then** the old file's nodes are removed and the new file's nodes are created, with edges reflecting the new file path
5. **Given** no files have changed since the last index, **When** `sync_workspace()` is called, **Then** the system reports "no changes detected" and performs no graph mutations

---

### User Story 4 - Cross-Region Task-to-Code Linking (Priority: P4)

As an orchestrator or agent, I link tasks to the specific code symbols they concern so that the agent can retrieve both the task context and the relevant code in a single query, answering questions like "which files are affected by this bug fix?"

**Why this priority**: Cross-region linking is the "golden edge" that unifies the task graph (Region B, temporal memory) and the code graph (Region A, spatial memory). Without it, the two regions remain siloed and the agent must perform separate queries and manually correlate results.

**Independent Test**: Create a task "Fix authentication timeout," link it to functions `login_user` and `validate_token` via `link_task_to_code`, then call `get_active_context`. Verify the response includes both the task details and the linked function definitions with their dependency neighborhoods.

**Acceptance Scenarios**:

1. **Given** a task and a code symbol, **When** `link_task_to_code(task_id, symbol_name)` is called, **Then** a `concerns` edge is created between the task node and the matching code node
2. **Given** a task linked to 3 functions, **When** `get_active_context()` is called and that task has status `in_progress`, **Then** the response includes the task details plus the definitions and 1-hop dependency neighborhoods of all 3 linked functions
3. **Given** a symbol name that resolves to multiple nodes, **When** `link_task_to_code` is called, **Then** the system links to all matching nodes and returns the count of links created
4. **Given** a task with no code links, **When** `get_active_context()` is called, **Then** the response includes the task details with an empty `relevant_code` section
5. **Given** a code node that is deleted during `sync_workspace`, **When** the node had `concerns` edges from tasks, **Then** the orphaned `concerns` edges are cleaned up and affected tasks receive a context note recording the broken link

---

### User Story 5 - Unified Semantic Search Across Regions (Priority: P5)

As an AI agent, I perform a single natural language query that searches across both code symbols and task/context data so that I receive a holistic view of the workspace relevant to my question.

**Why this priority**: Unified search is the convergence point. Rather than issuing separate `query_memory` (tasks/specs) and `map_code` (code) calls, the agent can ask one question and receive ranked results spanning both regions, weighted by relevance.

**Independent Test**: Populate a workspace with tasks about "billing" and code containing `process_payment`, `TaxCalculator`, and `PaymentGateway` functions. Query `unified_search("billing logic")`. Verify results include both the billing-related tasks and the semantically related code symbols, ranked by combined relevance.

**Acceptance Scenarios**:

1. **Given** an indexed workspace with tasks and code, **When** `unified_search("billing logic")` is called, **Then** the system returns ranked results from both code nodes (functions, classes) and task/context nodes, merged by relevance score
2. **Given** a unified search result containing a code node, **When** the result is returned, **Then** it includes the node type (function/class/file), file path, line range, and summary
3. **Given** a unified search result containing a task node, **When** the result is returned, **Then** it includes the task title, status, priority, and linked code symbols (if any)
4. **Given** a query that matches only code nodes, **When** `unified_search` is called, **Then** only code results are returned (no empty task section padding)
5. **Given** a `unified_search` call with `region: "code"` filter, **When** executed, **Then** only code graph nodes are searched, bypassing the task region entirely

---

### User Story 6 - Impact Analysis Queries (Priority: P6)

As an orchestrator planning a refactor, I query the system to discover which active tasks are affected by changes to a specific code symbol so that I can assess risk and coordinate work across the team.

**Why this priority**: Impact analysis is where the unified graph delivers its highest strategic value. A query like "which active tasks are blocked by the `UserAuth` class refactor?" is impossible for task-only systems or code-only vector stores to answer. It requires traversing cross-region edges.

**Independent Test**: Create 5 tasks, link 3 of them to functions that depend on `UserAuth`. Call `impact_analysis("UserAuth")`. Verify the response lists all 3 linked tasks with their status, priority, and the specific dependency path from `UserAuth` to the linked function.

**Acceptance Scenarios**:

1. **Given** a code symbol with `concerns` edges from tasks, **When** `impact_analysis("UserAuth")` is called, **Then** the system returns all tasks linked (directly or transitively via code dependencies) to that symbol, with the dependency path for each
2. **Given** a code symbol with no task links, **When** `impact_analysis` is called, **Then** the system returns the code dependency neighborhood with a note that no tasks reference this symbol
3. **Given** a symbol with transitive task links (task → function A → calls → function B, and `UserAuth` → called_by → function B), **When** `impact_analysis("UserAuth", depth: 2)` is called, **Then** the task linked to function A is included because function A transitively depends on `UserAuth`
4. **Given** an `impact_analysis` call with `status_filter: "in_progress"`, **When** executed, **Then** only tasks with matching status are included in results

---

### User Story 7 - Code Graph Persistence (Priority: P7)

As a developer, I expect the code graph metadata (embeddings, hashes, edges, cross-region links) to be serialized to `.engram/` files alongside task data so that the graph state travels with the repository, can be committed and shared, and does not require a full re-embedding after cloning.

**Why this priority**: Persistence completes the lifecycle. Source code is the canonical store for code bodies, but embeddings and graph edges are expensive to regenerate. Persisting metadata enables fast hydration by re-parsing source files (cheap) and skipping re-embedding for symbols whose `body_hash` has not changed (expensive).

**Independent Test**: Index a workspace, call `flush_state`, verify `.engram/code-graph/` files contain node metadata (hashes, embeddings, embed types) and edges but NOT full source bodies. Delete the SurrealDB database, call `set_workspace`, and verify the code graph is hydrated by parsing source files and restoring persisted embeddings for unchanged symbols without re-embedding.

**Acceptance Scenarios**:

1. **Given** an indexed code graph, **When** `flush_state()` is called, **Then** code graph metadata (embeddings, body hashes, embed types, edges, `concerns` links) is serialized to `.engram/code-graph/` files, excluding source bodies
2. **Given** a workspace with persisted code graph metadata, **When** `set_workspace` hydrates the workspace, **Then** source files are parsed to populate node bodies, `body_hash` values are compared against persisted hashes, and only symbols with changed hashes are re-embedded
3. **Given** a persisted code graph where all source files are unchanged, **When** hydration runs, **Then** zero re-embedding occurs and the graph is fully operational within seconds
4. **Given** a `.engram/code-graph/` directory with corrupted metadata files, **When** hydration fails, **Then** the system falls back to a full re-index from source (parse + embed all), logs the recovery, and continues

---

### Edge Cases

* What happens when a file exceeds a configurable size limit (default: 1 MB)? The file is skipped during indexing with a warning to avoid excessive memory use.
* How does the system handle circular import chains? Circular imports are valid in many languages. The graph stores them as-is; only task dependency cycles are rejected.
* What happens when the same symbol name exists in multiple files? Each node is scoped by file path. `map_code` returns all matches grouped by file, with disambiguation metadata.
* How does indexing handle generated code (e.g., build artifacts in `target/`, `node_modules/`)? The indexer respects `.gitignore` and a configurable exclusion list in `.engram/config.toml`.
* What happens when the parser encounters a syntax error in a source file? The file is partially indexed up to the error point, a warning is emitted, and indexing continues with remaining files.
* How does the system handle very large codebases (100,000+ files)? Indexing is parallelized across CPU cores. A configurable concurrency limit prevents resource exhaustion. Progress is reported via SSE events.
* What happens when a `concerns` edge targets a renamed symbol after sync? The sync process removes old nodes and creates new ones. Orphaned `concerns` edges are cleaned up and affected tasks receive context notes.
* What happens when an AST node is exactly at the 512-token boundary? Nodes at or below 512 tokens are classified as Tier 1 (`explicit_code`). Only nodes strictly exceeding the limit use Tier 2 (`summary_pointer`).
* What happens when a Tier 2 node has no docstring and a minimal signature? The system embeds whatever signature text is available. If the resulting embedding is too sparse for useful similarity, the node still participates in graph traversal (its structural edges remain navigable) even if vector search recall is degraded.
* What happens when a source file is deleted but its metadata persists in `.engram/code-graph/`? During hydration, any persisted metadata referencing files that no longer exist on disk is discarded, and associated `concerns` edges generate cleanup context notes on affected tasks.
* What happens when a function moves to a different file but keeps the same name and body? File-level hash detects both files as changed. The old file's nodes are removed. The new file's nodes are created with the same `body_hash`, so the existing embedding is reused (no re-embedding). `concerns` edges are automatically re-linked to the new node via hash-resilient identity matching (FR-124), and affected tasks receive a context note recording the path change.
* What happens when `sync_workspace` is called without a prior `index_workspace`? The system treats it as a first-time full index — all files are parsed, all symbols are embedded, and the result is identical to calling `index_workspace`. No error is returned.
* What happens when a `code_file` node is the target of a `concerns` edge? `code_file` nodes do not have a `body` field. When included in retrieval responses, the system reads the entire file content from disk and returns it as the code context. File content is bounded by `max_file_size_bytes` (FR-117).
* What happens when the `bge-small-en-v1.5` embedding model fails to load (missing files, out of memory)? The daemon fails to start with a descriptive error. There is no degraded mode — embeddings are required for all code graph and task/context operations.
* Can a long-running `index_workspace` operation be cancelled? In v0, indexing is not cancellable. The agent must wait for the operation to complete. Cancellation support is deferred to a future version.
* What happens when `get_active_context` is called and zero tasks have `in_progress` status? The response returns `primary_task: null` and `other_tasks: []`. No error is returned.
* What happens when `map_code` vector-search fallback also returns zero results? The response returns an empty result set with `fallback_used: true`, `root: null`, `neighbors: []`, and `matches: []`. No error is returned.
* What happens when a source file contains zero extractable symbols (e.g., only `mod` declarations or `use` statements)? A `code_file` node is still created (to support `imports` edges), but no function/class/interface nodes are generated.
* Can two connections to the same workspace see each other's code graph changes? Yes. The code graph is stored in shared SurrealDB tables scoped to the workspace namespace. All connections to the same workspace see mutations immediately after the operation completes.
* What happens when `flush_state` is called during an active `index_workspace` operation? `flush_state` returns error 7003 (`IndexInProgress`). The agent must wait for indexing to complete before flushing.
* What happens when a source file is 0 bytes? Empty files are skipped during indexing (`size_bytes` must be > 0 per schema validation). No warning is emitted — 0-byte files are treated the same as non-source files.
* What happens when persisted `line_start`/`line_end` values in JSONL no longer match the current source file? Hydration re-parses source files to obtain fresh line ranges. Persisted line ranges in JSONL are NOT authoritative — they are replaced by values from the current parse.
* What happens when a file is added to `code_graph.exclude_patterns` after `concerns` edges were created to its symbols? On the next `sync_workspace`, the file's nodes are removed (it is now excluded), `concerns` edges to those nodes are orphaned, and affected tasks receive context notes per FR-112.
* What happens when `link_task_to_code` is called twice with the same task and symbol? The operation is idempotent. If a `concerns` edge already exists between the task and the matching code node(s), no duplicate edges are created. The response reports `links_created: 0`.
* What happens when a source file is replaced by a directory with the same name (or vice versa) between syncs? If the path is now a directory, it is skipped (not a file). The old `code_file` node and its children are removed as a deletion. If a directory is replaced by a file, it is treated as a new file addition.
* How large can `.engram/code-graph/nodes.jsonl` grow at scale? At 10,000 nodes with 384-dimensional embeddings serialized as JSON floats, the file is approximately 30–50 MB. This is acceptable for Git storage and JSONL streaming reads. No additional streaming or chunking requirements apply in v0.
* What happens when a source file is modified between AST parsing and hash computation within a single sync cycle? The system computes `body_hash` from the parsed content (the snapshot read into memory), not from a second disk read. A subsequent `sync_workspace` detects any further changes via file-level `content_hash` mismatch.

## Clarifications

### Session 2026-02-11

- Q: How should the system handle existing embeddings generated by the v0 `all-MiniLM-L6-v2` model when migrating to `bge-small-en-v1.5`? → A: Clean replacement — no coexistence or lazy migration. Nobody uses v0 before this refactor, so all embeddings are regenerated with the new model on hydration. No `model_version` field is needed.
- Q: Should code node source bodies be stored inline in the DB and flushed to `.engram/` files, or derived from source? → A: Source-canonical with hash-gated re-embedding. Source code is the canonical persistence for code graph bodies. The graph stores `body_hash` per symbol for diff-rehydration. `.engram/code-graph/` persists only metadata (embeddings, hashes, edges, `concerns` links), not source bodies. On hydration: parse source → populate bodies → compare `body_hash` → re-embed only changed symbols.
- Q: How much of each neighbor node should `map_code` include in its response? → A: Full source body for every node in the traversal result. If a node’s body is not loaded in the graph at query time, the system MUST read the source file from disk using the node’s file path and line range to fetch the full body before returning it to the caller.
- Q: Should `concerns` edges break when a function is moved to a different file during refactoring? → A: Hash-resilient identity. On sync, if a symbol with the same name and `body_hash` appears at a new file path, existing `concerns` edges are automatically re-linked to the new node. The task receives a context note recording the path change, not a broken-link warning.
- Q: How should `get_active_context` behave when multiple tasks are `in_progress` simultaneously? → A: All tasks, bounded code. Return all `in_progress` tasks with metadata, but expand full code neighborhoods (with source bodies) only for the highest-priority task. Remaining tasks include linked code symbol names only. The agent can drill deeper on specific tasks with `map_code`.

### Session 2026-02-12

- Q: How should agents handle non-fatal errors (7001, 7002, 7006, 7007) in indexing and sync responses? → A: Non-fatal errors are collected in the response's `errors` array alongside successful results. The agent should process all successful results normally, log or display errors for user awareness, and does NOT need to retry — the operation completed successfully for all non-erroring files. Re-running `sync_workspace` after fixing the underlying issue (e.g., syntax error) resolves the reported errors.
- Q: Is `get_active_context` a new tool or an enhancement of an existing one? → A: `get_active_context` is a NEW tool introduced in 003. It did not exist in 001 or 002. Plan.md labels it "EXTENDED" in `read.rs` because the file is extended with a new function, not because an existing tool is modified.
- Q: How is "highest-priority task" determined when multiple `in_progress` tasks share the same priority level? → A: Tiebreaker is `created_at` (oldest first), consistent with `get_ready_work` ordering in 002. This produces a deterministic single primary task.
- Q: What scoring algorithm does `unified_search` use? → A: Cosine similarity on embedding vectors. Scores are raw cosine values in the range [0, 1]. Results from code and task regions are merged into a single list sorted by descending score. No cross-region normalization or boosting is applied in v0.
- Q: Does the `depth` parameter in `impact_analysis` count `concerns` edges as hops? → A: No. `depth` counts code-only hops (traversal across `calls`, `imports`, `inherits_from` edges). The `concerns` junction from code nodes to task nodes is a single-step lookup after the code BFS completes — it does NOT consume a depth unit.
- Q: What traversal algorithm do `map_code` and `impact_analysis` use? → A: Breadth-first search (BFS). This guarantees that closer neighbors (lower depth) are always included before distant ones when `max_nodes` truncation applies.
- Q: What format should the `linked_by` field on `concerns` edges use? → A: Free-form string identifying the MCP client that created the link (e.g., the `source_client` from the SSE connection, or a descriptive agent name like `"copilot-agent-1"`). No strict format is enforced — it is an audit/provenance field.
- Q: For Tier 1 nodes (`explicit_code`), is the `summary` field a duplicate of the `body`? → A: Yes. For Tier 1 nodes, `summary` = the full body text. This duplication is intentional for schema consistency — agents should always use the `body` field for code content. The `summary` field is the text that was embedded, regardless of tier.
- Q: What happens when a function is renamed (different name, same body_hash)? → A: The hash-resilient identity match uses the tuple `(name, body_hash)`. A renamed function does NOT match — the old node is removed, the new node is created with a new name, and the old `concerns` edge becomes orphaned. This is intentional: a rename changes the symbol's identity. The affected task receives an orphaned-link context note.
- Q: Is the "4 chars per token" approximation (FR-142) a stable contract? → A: It is a stable heuristic for tier classification within this system. It does NOT claim to match the actual model tokenizer's output. The tier boundary is computed as `body.len() / 4` compared against `embedding.token_limit`. This formula is guaranteed not to change within a major version.
- Q: Does `symbol_name` in tool inputs accept qualified names (e.g., `crate::billing::process_payment`)? → A: No. `symbol_name` accepts unqualified names only (e.g., `process_payment`). Qualified path lookup is not supported in v0. If the unqualified name matches multiple nodes across files, all matches are returned (grouped by file per US2 scenario 3).
- Q: Why does `class` lack a `signature` field that `function` has? → A: Intentional. In Rust, `struct_item` definitions do not have a callable signature — their structure IS the body. The `body` field contains the full struct definition. `function` has `signature` because function signatures are meaningful for embedding and display independently of the body.
- Q: Does FR-148 (full source body guarantee) apply to `unified_search`? → A: No. `unified_search` returns `summary` text only, not full source bodies. Including full bodies for up to 50 results would produce multi-megabyte responses. Agents should use `map_code` to retrieve full bodies for specific results of interest. FR-148 applies to `map_code`, `get_active_context`, and `impact_analysis` only.
- Q: Does `link_task_to_code` match `code_file` nodes when the `symbol_name` equals a file path? → A: No. `symbol_name` matches against function, class, and interface `name` fields only. `code_file` nodes are not "symbols" and cannot be linked via `link_task_to_code`. The `concerns` edge schema allows `code_file` targets for programmatic edge creation, but the tool API does not expose this in v0.
- Q: When `link_task_to_code` symbol resolves to nodes of different types (e.g., a function and a class both named `Config`), are all linked? → A: Yes. The tool links to ALL matching nodes regardless of type. The response's `matched_nodes` array includes each linked node with its type and file path.
- Q: Does the spec formally depend on the `priority` field from 002's Task model? → A: Yes. `get_active_context` requires `priority` for determining the primary task (FR-127), and `TaskSummary` in mcp-tools.json includes `priority`. This is a cross-spec dependency on the 002 Task model.
- Q: Is the `concerns` edge lifecycle fully covered end-to-end? → A: Yes. The lifecycle is: `index_workspace` (create code nodes) → `link_task_to_code` (create concerns edges, FR-110) → `sync_workspace` (re-link via hash-resilient identity FR-124, or orphan cleanup FR-112) → task context notes for all changes. Retrieval via `get_active_context` (FR-127) and `impact_analysis` (FR-129) closes the loop.

## Requirements *(mandatory)*

### Functional Requirements

**Code Graph Schema (Region A):**

* **FR-101**: System MUST define a `code_file` node type with attributes: path (string, unique per workspace), language (string), size_bytes (integer), content_hash (string), last_indexed_at (datetime)
* **FR-102**: System MUST define a `function` node type with attributes: name (string), file_path (string), line_start (integer), line_end (integer), signature (string), docstring (optional string), body (string, populated at runtime from source, not persisted to `.engram/`), body_hash (string, content hash of the source body for diff-rehydration), token_count (integer), embed_type (string: `explicit_code` or `summary_pointer`), embedding (array of floats), summary (string)
* **FR-103**: System MUST define a `class` node type with attributes: name (string), file_path (string), line_start (integer), line_end (integer), docstring (optional string), body (string, runtime-only), body_hash (string), token_count (integer), embed_type (string: `explicit_code` or `summary_pointer`), embedding (array of floats), summary (string)
* **FR-104**: System MUST define an `interface` node type with attributes: name (string), file_path (string), line_start (integer), line_end (integer), docstring (optional string), body (string, runtime-only), body_hash (string), token_count (integer), embed_type (string: `explicit_code` or `summary_pointer`), embedding (array of floats), summary (string)
* **FR-105**: System MUST define a `calls` edge type connecting function-to-function, representing direct invocation relationships
* **FR-106**: System MUST define an `imports` edge type connecting code_file-to-code_file or code_file-to-symbol, representing module dependencies
* **FR-107**: System MUST define an `inherits_from` edge type connecting class-to-class or class-to-interface
* **FR-108**: System MUST define a `defines` edge type connecting code_file-to-function and code_file-to-class, establishing containment relationships

**Cross-Region Linking (The Golden Edge):**

* **FR-109**: System MUST define a `concerns` edge type connecting any task node (from Region B) to any code node (function, class, interface, code_file from Region A)
* **FR-110**: System MUST expose a `link_task_to_code(task_id, symbol_name)` tool that creates `concerns` edges between a task and all matching code nodes
* **FR-111**: System MUST expose an `unlink_task_from_code(task_id, symbol_name)` tool that removes `concerns` edges
* **FR-112**: System MUST clean up orphaned `concerns` edges when code nodes are removed during sync, appending context notes to affected tasks

**Indexing:**

* **FR-113**: System MUST expose an `index_workspace()` tool that parses all supported source files and populates the code graph
* **FR-114**: System MUST support Rust as an indexed language at launch, with an extensible parser architecture for adding languages
* **FR-115**: System MUST use AST-level parsing (not regex) to extract code structure, leveraging `tree-sitter` grammars for language support
* **FR-116**: System MUST respect `.gitignore` patterns and a configurable exclusion list in `.engram/config.toml` (key: `code_graph.exclude_patterns`)
* **FR-117**: System MUST skip files exceeding a configurable size limit (default: 1 MB, key: `code_graph.max_file_size_bytes`)
* **FR-118**: System MUST generate embeddings using the `bge-small-en-v1.5` model (384 dimensions, 512-token input limit) via `fastembed`, superseding the v0 `all-MiniLM-L6-v2` model for all embedding operations (code graph and task/context regions alike). On first hydration after the model switch, all existing task/context embeddings from prior specs MUST be re-generated with the new model. No coexistence or lazy migration is supported.
* **FR-119**: System MUST parallelize file parsing across available CPU cores with a configurable concurrency limit (key: `code_graph.parse_concurrency`, default: 0, where 0 means auto-detect the number of logical CPUs at runtime)
* **FR-120**: System MUST report indexing progress via SSE events (files parsed, total files, errors encountered)

**Hierarchical AST Chunking (Tiered Embedding):**

* **FR-141**: Each AST node (function, class, interface) MUST be treated as a single embedding chunk — one chunk per symbol, not arbitrary sliding windows
* **FR-142**: System MUST estimate tokens for each AST node body using character-based approximation (4 chars per token) before generating an embedding
* **FR-143**: **Tier 1 (Direct Embedding)**: If an AST node body is at or below the model's token limit (≤512 tokens, where token count = body length in characters / 4), the system MUST embed the raw source code directly and tag the node `embed_type: explicit_code`
* **FR-144**: **Tier 2 (Summarized Embedding)**: If an AST node body exceeds the token limit, the system MUST embed only the function signature and docstring summary, tag the node `embed_type: summary_pointer`, and store the full source body in the `body` field for retrieval
* **FR-145**: When a vector search matches a `summary_pointer` node, retrieval tools MUST return the full source `body` stored in the database, not the summary text used for embedding

**Embedding Model Resource Sharing:**

* **FR-146**: System MUST load the embedding model exactly once into shared read-only memory, enabling all concurrent connections and workspaces to share a single model instance without duplication
* **FR-147**: System MUST support batched embedding requests, combining embedding work from multiple files or workspaces into a single inference pass to maximize throughput

**Incremental Sync:**

* **FR-121**: System MUST expose a `sync_workspace()` tool that detects changed, added, and deleted files since the last index
* **FR-122**: System MUST use content hashing at two levels to detect changes: file-level `content_hash` on `code_file` nodes to identify which files changed, and per-symbol `body_hash` on function/class/interface nodes to identify which symbols within a changed file actually require re-embedding
* **FR-123**: System MUST remove all nodes and edges originating from deleted files before re-indexing; for modified files, the system MUST re-parse and compare per-symbol `body_hash` values, re-embedding only symbols whose hash changed
* **FR-124**: System MUST preserve `concerns` edges across sync using hash-resilient identity: if a symbol with the same name and `body_hash` is re-created at a different file path (file move), existing `concerns` edges MUST be automatically re-linked to the new node and affected tasks MUST receive a context note recording the path change. Only when both the name and `body_hash` no longer match any post-sync node are `concerns` edges treated as orphaned.
* **FR-125**: System MUST record the sync timestamp and a summary of changes (files added, modified, deleted, unchanged) as a context note

**Retrieval Tools:**

* **FR-126**: System MUST expose a `map_code(symbol_name, depth?, max_nodes?)` tool that returns a symbol's definition plus its graph neighborhood to the specified traversal depth (default: 1, max: 5) using breadth-first search (BFS). The response MUST include the full source body for every node in the traversal result.
* **FR-127**: System MUST expose a `get_active_context()` tool that returns all `in_progress` tasks. For the highest-priority task, the response MUST include full source bodies and 1-hop dependency neighborhoods of all its `concerns`-linked code nodes. For remaining `in_progress` tasks, the response MUST include task metadata and linked code symbol names only (no expanded neighborhoods). The agent can use `map_code` to drill into any specific task's code context.
* **FR-128**: System MUST expose a `unified_search(query, region?, limit?)` tool that performs hybrid vector+keyword search across both code and task regions, returning merged, ranked results
* **FR-129**: System MUST expose an `impact_analysis(symbol_name, depth?, status_filter?)` tool that traverses code dependencies and cross-region `concerns` edges to find all tasks affected by a symbol
* **FR-130**: `map_code` MUST fall back to vector search across code node embeddings when the exact symbol name is not found in the graph
* **FR-131**: `unified_search` MUST support a `region` parameter with values `"code"`, `"task"`, or `"all"` (default: `"all"`) to scope the search
* **FR-148**: Retrieval tools `map_code`, `get_active_context`, and `impact_analysis` MUST guarantee that each code node in the response includes the full source body. If the body is not loaded in the graph at query time, the system MUST read the source file from disk using the node's file path and line range to fetch the complete body before returning results. `unified_search` is exempt — it returns `summary` text only to keep response sizes manageable.

**Persistence (Source-Canonical Model):**

* **FR-132**: System MUST serialize code graph metadata to `.engram/code-graph/nodes.jsonl` (embeddings, body hashes, embed types, token counts, signatures, summaries, line ranges) and `.engram/code-graph/edges.jsonl` (all edge types including `concerns` links) during `flush_state`. Source bodies MUST NOT be included in persisted files — source code is the canonical store.
* **FR-133**: On workspace activation, the system MUST hydrate the code graph by: (1) parsing source files via AST to populate node bodies at runtime, (2) loading persisted metadata from `.engram/code-graph/`, (3) comparing each symbol's current `body_hash` against the persisted hash, and (4) re-embedding only symbols whose hash has changed
* **FR-134**: System MUST detect stale code graph metadata by comparing persisted `body_hash` values against hashes computed from current source content during hydration. File-level staleness is detected first via `code_file.content_hash`, then symbol-level via `body_hash`.
* **FR-135**: System MUST support full re-index (parse + embed all symbols) as a recovery path when code graph metadata files are corrupted, missing, or when no persisted hashes exist (first-time index)

**Configuration:**

* **FR-136**: System MUST read code graph configuration from `.engram/config.toml` under the `[code_graph]` section
* **FR-137**: System MUST support the following configuration keys: `exclude_patterns` (array of glob strings), `max_file_size_bytes` (integer, default 1048576), `parse_concurrency` (integer, default 0 meaning auto-detect CPU count), `max_traversal_depth` (integer, default 5), `max_traversal_nodes` (integer, default 50), `supported_languages` (array of strings, default `["rust"]`), `embedding.token_limit` (integer, default 512)
* **FR-138**: System MUST fall back to built-in defaults when code graph configuration keys are absent
* **FR-149**: When a tool parameter (`depth` or `max_nodes`) exceeds the corresponding configuration limit (`max_traversal_depth` or `max_traversal_nodes`), the system MUST silently clamp the parameter to the config limit without returning an error. The response MUST include the effective value used.

**Error Taxonomy Extension:**

* **FR-139**: System MUST define new error codes in the 7xxx range for code graph operations: parse error (7001), unsupported language (7002), index in progress (7003), symbol not found (7004), file too large (7006), sync conflict (7007). Code 7005 is retired (previously `TraversalDepthExceeded`, removed per FR-149 silent-clamping design) and MUST NOT be reused.
* **FR-140**: All new error codes MUST follow the existing `ErrorResponse` format with code, name, message, and details fields

**Agent Consumability:**

* **FR-150**: System MUST expose a `list_symbols(file_path?, node_type?, name_prefix?, limit?)` tool that returns a paginated list of indexed code symbols (functions, classes, interfaces) with their name, type, and file path. This enables agents to discover valid symbol names before invoking `map_code`, `link_task_to_code`, or `impact_analysis`.
* **FR-151**: All code graph tools (`index_workspace`, `sync_workspace`, `map_code`, `link_task_to_code`, `unlink_task_from_code`, `get_active_context`, `unified_search`, `impact_analysis`, `list_symbols`) require `set_workspace` to have been called first. Calling any code graph tool without a bound workspace MUST return error 1003 (`WORKSPACE_NOT_SET`). Additionally, `map_code`, `link_task_to_code`, `impact_analysis`, and `list_symbols` require the code graph to be populated (via `index_workspace` or hydration); calling them on an empty graph MUST return error 7004 with a suggestion to run `index_workspace`. Error 7004 has two distinct cases that agents MUST distinguish via the `details` object:
  * **Symbol not found** (named lookup): `details.symbol_name` is present, containing the unmatched name. Agent action: verify spelling or re-index.
  * **Empty graph** (no symbols indexed): `details.symbol_name` is absent and the message reads "Code graph is empty — run index_workspace first". Agent action: call `index_workspace`.
* **FR-152**: `link_task_to_code` MUST be idempotent — calling it with the same `(task_id, symbol_name)` pair when a `concerns` edge already exists MUST NOT create duplicate edges. The response MUST report `links_created: 0` for already-linked pairs.
* **FR-153**: `flush_state` MUST return error 7003 (`IndexInProgress`) if an indexing operation is currently running. The agent must wait for indexing to complete before flushing.

**Operational Resilience:**

* **FR-154**: If the embedding model (`bge-small-en-v1.5`) fails to load at daemon startup (missing files, insufficient memory, corrupted model), the daemon MUST fail to start with a descriptive error message. There is no degraded mode — embeddings are required for all operations.
* **FR-155**: `index_workspace` MUST use per-file atomic commits to the database. If the process is terminated mid-index, the graph contains a valid (partial) set of fully-indexed files. The agent can re-run `index_workspace` to complete the remaining files.
* **FR-156**: `.engram/code-graph/` JSONL files are accessed by a single daemon instance per workspace (enforced by existing 001 workspace isolation). No file-level locking is required. Concurrent daemon instances for the same workspace are NOT supported.
* **FR-157**: `unified_search` MUST return error 4001 for empty queries (after whitespace trimming). If embedding generation fails for a non-empty query (model inference error), the system MUST return error 5001 (`SystemError`) with the underlying cause in `details`.

### Key Entities

* **code_file**: A source file tracked in the code graph. Key attributes: path (unique), language, size, content hash, last indexed timestamp. Serves as the containment root for function and class nodes.
* **function**: A callable code unit extracted from a source file. Key attributes: name, file path, line range, signature, docstring, body (runtime-only, populated from source during hydration, not persisted), body_hash (content hash for diff-rehydration), token count, embed type (`explicit_code` or `summary_pointer`), embedding vector, summary. When `embed_type` is `explicit_code`, the embedding represents the raw source. When `summary_pointer`, the embedding represents only the signature/docstring and the full body is returned on retrieval. Connected to other functions via `calls` edges and to its containing file via `defines` edges.
* **class**: A type definition extracted from a source file. Key attributes: name, file path, line range, docstring, body (runtime-only), body_hash, token count, embed type, embedding vector, summary. Same tiered embedding and source-canonical semantics as function. Connected to other classes/interfaces via `inherits_from` edges.
* **interface**: A trait or interface definition extracted from a source file. Key attributes: same as class with tiered embedding and source-canonical semantics. Used as inheritance targets.
* **calls**: Directed edge from one function to another, representing a direct invocation. Attributes: created_at.
* **imports**: Directed edge from one code_file to another code_file or symbol, representing a module dependency. Attributes: created_at, import_path (string).
* **inherits_from**: Directed edge from a class to its parent class or interface. Attributes: created_at.
* **defines**: Directed edge from a code_file to a function, class, or interface it contains. Attributes: created_at.
* **concerns**: Cross-region directed edge from a task (Region B) to a code node (Region A). The "golden edge" that unifies temporal and spatial memory. Attributes: created_at, linked_by (string, the client that created the link).

## Success Criteria *(mandatory)*

### Measurable Outcomes

* **SC-101**: `index_workspace` processes a 500-file Rust workspace in under 30 seconds (p95 over 10 runs) on a reference machine with ≥4 logical CPU cores, ≥16 GB RAM, and SSD storage
* **SC-102**: `sync_workspace` processes 10 changed files in under 3 seconds (p95), regardless of total workspace size
* **SC-103**: `map_code` returns 1-hop dependency results within 50ms (p95) for graphs with fewer than 10,000 nodes
* **SC-104**: `get_active_context` returns task details plus linked code neighborhoods within 100ms (p95)
* **SC-105**: `unified_search` returns merged ranked results from both regions within 200ms (p95)
* **SC-106**: `impact_analysis` traverses 2-hop code-to-task paths within 150ms (p95) for graphs with fewer than 10,000 nodes
* **SC-107**: Code graph metadata round-trip persistence (flush then hydrate from source + metadata) preserves 100% of embeddings (within 1e-6 epsilon per float element for serialization tolerance), edges, and hash values with zero data loss
* **SC-108**: Incremental sync detects changed files and re-embeds only changed symbols within those files, re-embedding fewer than 5% of nodes when fewer than 5% of symbols changed
* **SC-109**: Agents using graph-backed retrieval include 80% fewer irrelevant code tokens in their prompts compared to vector-only retrieval. Measured by comparing total token counts of graph-retrieved context vs. vector-only retrieval across the same 20 queries on the project's own codebase, where "irrelevant" means tokens from code symbols outside the 2-hop dependency neighborhood of the query target.
* **SC-110**: Cross-region queries (`impact_analysis`, `get_active_context`) return results that span both task and code domains in a single response, eliminating the need for multi-tool query chaining
* **SC-111**: Embedding model memory footprint remains under 150 MB regardless of the number of concurrent connections or active workspaces (single shared instance)
* **SC-112**: Batched embedding of 100 code nodes completes within 2 seconds (p95) on the reference machine
* **SC-113**: Tier 2 summary embeddings for large functions (over 512 tokens) produce semantic search recall within 10% of full-body embedding recall on a benchmark set of 50 natural-language queries generated from the project's own codebase during implementation validation
* **SC-114**: Code graph in-memory footprint (SurrealDB tables and indexes, excluding embedding model RAM) remains under 50 MB for a 10,000-node graph
* **SC-115**: Hydration of a persisted 10,000-node code graph from `.engram/code-graph/` JSONL files completes within 10 seconds (p95, excluding re-embedding of changed symbols)
* **SC-116**: `.engram/code-graph/` files occupy no more than 60 MB on disk for a 10,000-node graph

## Assumptions

* The workspace is a Git repository (consistent with 001 and 002 workspace validation).
* `tree-sitter` provides sufficient AST fidelity for extracting function, class, interface, and call-site information from supported languages.
* The `bge-small-en-v1.5` embedding model (384 dimensions, 512-token limit, ~130 MB RAM) supersedes the v0 `all-MiniLM-L6-v2` model. This is a clean replacement — no migration, coexistence, or version tagging required because no production users exist on the v0 model. All existing embeddings (if any) are discarded and regenerated with the new model on next hydration. The model switch applies to all embeddings (code graph, task/context, and `query_memory`).
* The 512-token hard limit of `bge-small-en-v1.5` is addressed by the hierarchical AST chunking strategy: small code nodes are embedded directly while large nodes use signature/docstring summaries as embedding proxies, with full source stored for retrieval.
* Rust is the minimum viable language for initial release. Language extensibility via `tree-sitter` grammars enables incremental additions post-launch.
* Cross-region `concerns` edges are created explicitly by agents or orchestrators. Automatic link inference (e.g., matching task descriptions to code symbols) is deferred to a future version.
* Code graph persistence uses a source-canonical model: source code files are the canonical store for code bodies, while `.engram/code-graph/` persists only derived metadata (embeddings, body hashes, edges, `concerns` links) in JSONL format for streaming reads/writes. Bodies are populated at runtime by parsing source files. This avoids duplicating source content in both files and the database.
* The two-level hash strategy (file-level `content_hash` + per-symbol `body_hash`) enables efficient diff-rehydration: file hashes identify which files changed, and symbol hashes identify which functions within a changed file actually need re-embedding, avoiding unnecessary embedding calls.
* `tree-sitter` Rust bindings are pure safe Rust wrappers. No native FFI conflicts with SurrealDB's embedded engine are expected, as both use distinct C/Rust interfaces.
* The `priority` field on the Task model from 002 (enhanced task management) is a cross-spec dependency. `get_active_context` (FR-127) and `TaskSummary` in mcp-tools.json depend on tasks having a `priority` attribute for deterministic primary-task selection.

## Prerequisites

### PRQ-001: Codebase Rename from engram to Engram

**Status**: Required before 003 implementation begins  
**Scope**: Full rename and rebrand of the service  

The server is being renamed from "engram" (also appearing as `engram`, `engram`, `engram`, `engram`, `engram`) to **Monocoque Agent Engram**.

| Surface | Old value | New value |
| -------- | --------- | --------- |
| Display name | engram | Monocoque Agent Engram |
| Code name | engram / engram | engram |
| Crate name (Cargo.toml `name`) | engram | engram |
| Rust import path | `use engram::` | `use engram::` |
| Binary name | engram | engram |
| Binary source file | `src/bin/engram.rs` | `src/bin/engram.rs` |
| Env var prefix | `ENGRAM_` | `ENGRAM_` |
| Workspace directory | `.engram/` | `.engram/` |
| Data directory segment | `~/.local/share/engram/` | `~/.local/share/engram/` |
| Model cache path | `engram/models/` | `engram/models/` |
| DB path segment | `engram/db/` | `engram/db/` |
| Tracing filter | `engram=debug` | `engram=debug` |
| Lib constant `APP_NAME` | `"engram"` | `"engram"` |
| CLI command name | `engram` | `engram` |
| CLI about text | "engram MCP daemon" | "Monocoque Agent Engram MCP daemon" |
| Authors | "engram Contributors" | "Engram Contributors" |
| Graph header | "Generated by engram" | "Generated by Engram" |
| Config file path | `.engram/config.toml` | `.engram/config.toml` |

**What must change:**

1. **Cargo.toml**: Package `name`, `description`, `authors`, `[[bin]]` name and path.
2. **Source files (`src/`)**: All `engram`, `engram`, `engram`, `.engram` references in code, constants, path literals, doc comments, and inline comments.
3. **Test files (`tests/`)**: All `use engram::` imports, `ENGRAM_dir` variable names, `.engram` path literals, `"engram"` and `"ENGRAM_"` string literals, and function names containing `engram`.
4. **Configuration (`src/config/mod.rs`)**: All `ENGRAM_` env annotations → `ENGRAM_`, clap command name and about text, default data directory path.
5. **Persistence (`src/services/hydration.rs`, `dehydration.rs`, `config.rs`)**: All `.engram` path references → `.engram`.
6. **Embedding (`src/services/embedding.rs`)**: Model cache path `engram/models/` → `engram/models/`.
7. **Database (`src/db/mod.rs`)**: DB storage path `engram/db/` → `engram/db/`.
8. **Specs and documentation**: All spec files (001, 002, 003), README, and doc comments updated to reflect the new name.

**Verification gates (all must pass before 003 work begins):**

* `cargo check` succeeds with zero errors
* `cargo test --all-targets` passes all tests
* `cargo clippy -- -D warnings` produces zero warnings
* Case-insensitive search for `t.mem`, `engram`, `T.MEM`, or `engram` across `src/`, `tests/`, and `Cargo.toml` returns zero matches

**Why this is a prerequisite for 003**: Feature 003 introduces new modules, configuration keys (`.engram/config.toml` code graph section), and persistence paths (`.engram/code-graph/`) that must use the canonical name from the start. Implementing 003 against the old name and then renaming afterward would double the churn and risk introducing inconsistencies.

**Note on existing workspaces**: No automatic migration of on-disk `.engram/` directories is provided. Users rename them manually to `.engram/` or re-initialize. The `copilot-instructions.md` already references the new naming (`ENGRAM_` prefix, `.engram/` directory).

## Known Issues

* **fastembed TLS on ort-sys**: The `fastembed = "3"` dependency requires a TLS feature flag on `ort-sys` that currently blocks `cargo check` and `cargo test` (documented in project root copilot-instructions). This MUST be resolved before 003 implementation begins, as the entire code graph and model switch (FR-118) depend on fastembit. Resolution path: pin `ort` version with correct feature flags or upstream fix.
* **Indexing cancellation**: `index_workspace` is not cancellable in v0. For very large workspaces, this may block the agent for the full indexing duration (≤30 s per SC-101). Cancellation support is deferred to a future version.

## Out of Scope (v0)

* Automatic code-to-task linking based on NLP analysis of task descriptions
* Real-time file watching for live code graph updates (indexing and sync are explicit tool calls)
* Cross-workspace code graph queries or linking
* Source code generation or modification based on graph analysis
* Language support beyond Rust at initial launch (extensible architecture supports future additions)
* AST diff-based incremental parsing (files are re-parsed entirely on change)
* Code graph visualization or rendering
* Integration with external code intelligence providers (LSP, Sourcegraph)
* Embedding model fine-tuning for code-specific semantics
* Dynamic model selection or per-node model routing (all nodes use the same model)
* Token-level streaming or partial embedding for nodes near the token limit
