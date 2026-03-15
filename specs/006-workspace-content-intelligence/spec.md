# Feature Specification: Workspace Content Intelligence

**Feature Branch**: `006-workspace-content-intelligence`
**Created**: 2026-03-15
**Status**: Draft
**Input**: User description: "Workspace content intelligence: git commit graph tracking with code snippets for change detection, agent hooks and instructions, documentation, SpecKit-aware rehydration via structured backlog JSON files, and content registry with multi-source ingestion pipeline"

## User Scenarios & Testing *(mandatory)*

<!--
  This feature consolidates five related backlog items into a single coherent capability:
  workspace content intelligence. All five items share a common theme — Engram currently
  has a narrow view of workspace content, limited to `.engram/tasks.md` and the code graph.
  This feature widens Engram's awareness to encompass git history, SpecKit artifacts,
  developer documentation, agent hooks, and arbitrary content sources declared by the
  developer.

  The five backlog items are:
  1. Git commit graph tracking with code/text snippets for change detection and search
  2. Agent hooks and instructions for ensuring agents use Engram
  3. Proper documentation for the solution
  4. SpecKit-aware rehydration via structured backlog JSON files
  5. Content registry and multi-source ingestion pipeline (registry.yaml)

  User stories are ordered by foundational dependency: the content registry (P1) must
  exist before multi-source ingestion (P2), SpecKit-aware rehydration (P3), git commit
  tracking (P4), agent hooks (P5), and documentation (P6) can build on it.
-->

### User Story 1 - Content Registry Declaration (Priority: P1)

As a developer setting up Engram for my workspace, I declare the content sources in my project (code, tests, specs, docs, context files, instructions) in a registry file so that Engram knows what to monitor, ingest, and make searchable — without hardcoded assumptions about my project structure.

**Why this priority**: The content registry is the foundational data model for all other stories. Without a developer-declared registry, Engram cannot know which paths to watch, which content types to partition in the database, or how to scope searches. Every other story in this feature depends on the registry existing and being readable.

**Independent Test**: Run `engram install` in a workspace containing `src/`, `tests/`, `specs/`, and `docs/` directories. Verify that a `.engram/registry.yaml` file is generated with auto-detected source entries. Manually add a custom entry (e.g., `type: context, path: .context`). Call `get_workspace_status` and verify it reports all registered sources. Call a search scoped to `type: specs` and verify only spec content is searched.

**Acceptance Scenarios**:

1. **Given** a workspace with no `.engram/registry.yaml`, **When** `engram install` is run, **Then** the installer detects common directories (src, tests, specs, docs) and generates a default `registry.yaml` with appropriate type and language entries for each detected directory
2. **Given** a `.engram/registry.yaml` with three source entries, **When** Engram hydrates the workspace, **Then** each source entry is validated (path exists, type is recognized) and registered in the database as a content source record
3. **Given** a registry entry with `type: code` and `language: rust`, **When** the source is registered, **Then** the code graph indexer uses this entry to determine which paths to index and which language grammar to use
4. **Given** a registry entry with a path that does not exist on disk, **When** Engram hydrates, **Then** the system logs a warning for the missing path but continues hydrating other sources without failing
5. **Given** a developer who adds a custom entry (e.g., `type: tracking, path: .copilot-tracking`), **When** Engram hydrates, **Then** the custom type is accepted and content from that path is ingested and searchable under the custom type label
6. **Given** a registry with no entries, **When** Engram hydrates, **Then** the system falls back to legacy behavior (`.engram/tasks.md` only) and logs a warning that no content sources are registered

---

### User Story 2 - Multi-Source Content Ingestion (Priority: P2)

As an AI agent querying Engram, I receive search results partitioned by content type (code, specs, docs, tests, context) so that I can request precisely the category of knowledge I need — reducing context pollution and improving retrieval relevance.

**Why this priority**: Once the registry declares what content exists, the ingestion pipeline makes that content queryable. This is the engine that transforms declared sources into searchable, type-partitioned data in SurrealDB. Without ingestion, the registry is just metadata.

**Independent Test**: Configure a registry with entries for `src/` (code), `specs/` (spec), and `docs/` (docs). Trigger hydration. Verify that SurrealDB contains content records partitioned by type. Call `query_memory` with a filter for `type: spec` and verify only spec content is returned. Call `unified_search` without a type filter and verify results from all types are returned with type labels.

**Acceptance Scenarios**:

1. **Given** a registry with entries for code, specs, and docs, **When** hydration runs, **Then** Engram reads files from each registered path and creates content records in SurrealDB partitioned by the declared content type
2. **Given** a file change event in a registered path (e.g., a spec file is modified), **When** the change is detected, **Then** Engram re-ingests only the changed file and updates its content record in the database
3. **Given** a `query_memory` call with `content_type: "spec"`, **When** the query executes, **Then** only content records from spec-type sources are searched and returned
4. **Given** a `unified_search` call with no content type filter, **When** the query executes, **Then** results from all content types are returned, each annotated with its source type and file path
5. **Given** a registered path containing 500 files, **When** initial ingestion runs, **Then** the system ingests files in batches, emits progress tracing spans, and completes without exhausting memory
6. **Given** a file in a registered path that exceeds 1 MB, **When** ingestion encounters it, **Then** the system skips the file with a warning rather than attempting to load it into memory
7. **Given** a registered `type: code` source, **When** ingestion runs, **Then** the existing code graph indexer (`index_workspace` / `sync_workspace`) is used for that source rather than raw text ingestion

---

### User Story 3 - SpecKit-Aware Structured Rehydration (Priority: P3)

As a developer using SpecKit for feature management, I expect Engram to understand my multi-feature workspace structure — reading from and writing to per-feature backlog JSON files in `.engram/` — so that all SpecKit artifacts (specs, plans, tasks, scenarios, research, analysis) are captured as part of workspace state.

**Why this priority**: SpecKit-organized workspaces have a richer structure than the single `tasks.md` file Engram currently expects. This story ensures Engram's hydration and dehydration cycles preserve the full SpecKit artifact tree, making feature-specific queries possible and preventing data loss across restart cycles.

**Independent Test**: Set up a workspace with `specs/001-core-mcp-daemon/` and `specs/002-enhanced-task-management/` each containing spec.md, plan.md, tasks.md, SCENARIOS.md, and research.md. Run `engram install` then hydrate. Verify `.engram/project.json` is created with project metadata and links to backlog files. Verify `.engram/backlog-001.json` and `.engram/backlog-002.json` exist with full artifact contents. Modify a task in SurrealDB, trigger dehydration, and verify the corresponding backlog JSON is updated.

**Acceptance Scenarios**:

1. **Given** a workspace with `specs/001-core-mcp-daemon/` containing spec.md, plan.md, tasks.md, SCENARIOS.md, and research.md, **When** hydration runs, **Then** a `backlog-001.json` file is created in `.engram/` containing feature metadata (id, name, title, git branch, spec path, description, status) and the full text contents of all SpecKit artifacts found in the feature directory
2. **Given** a workspace with multiple feature directories (001 through 005), **When** hydration runs, **Then** one `backlog-NNN.json` file is created per feature directory, numbered to match the feature directory number
3. **Given** hydration has run successfully, **When** the system writes `.engram/project.json`, **Then** the project file contains project-level metadata (name, description, repository URL, default branch) and an array of references to each backlog JSON file
4. **Given** a task record is modified in SurrealDB, **When** dehydration runs, **Then** the corresponding `backlog-NNN.json` file is updated with the new task state while preserving all other artifact contents
5. **Given** a workspace with no `specs/` directory, **When** hydration runs, **Then** the system falls back to legacy `.engram/tasks.md` behavior and does not create backlog JSON files
6. **Given** a feature directory that is missing some optional artifacts (e.g., no research.md), **When** hydration reads it, **Then** the backlog JSON includes only the artifacts that exist, with null or absent fields for missing ones
7. **Given** an existing `backlog-001.json` from a prior hydration, **When** a new SpecKit artifact (e.g., ANALYSIS.md) is added to `specs/001-*/`, **Then** the next hydration cycle detects the new file and adds its content to the existing backlog JSON

---

### User Story 4 - Git Commit Graph Tracking (Priority: P4)

As an AI agent performing code review, I query Engram for the commit history of a specific file or function so that I can understand what changed, when, and why — with actual code and text snippets attached to each change — enabling faster change detection and grounded adversarial review.

**Why this priority**: Git history is a critical dimension of workspace knowledge that Engram currently ignores. For adversarial code reviews, agents need to trace the evolution of code to detect regressions, understand intent, and validate that changes align with specifications. This story is P4 because it depends on the content registry (to know which paths to track) and the ingestion pipeline (to store change records).

**Independent Test**: Configure a registry with `type: code, path: src`. Make 5 commits modifying different files in `src/`. Call a new `query_changes` tool with a file path filter. Verify the response includes commit hashes, timestamps, authors, commit messages, and actual diff snippets (added/removed lines) for that file. Call `query_changes` with a function name and verify it returns only commits that touched that function.

**Acceptance Scenarios**:

1. **Given** a workspace with git history, **When** Engram indexes the commit graph, **Then** the system creates commit nodes in SurrealDB with attributes: hash, author, timestamp, message, and parent commit references
2. **Given** a commit that modifies 3 files, **When** the commit is indexed, **Then** each file change is stored as a change record linked to the commit node, containing the file path, change type (add/modify/delete), and a diff snippet showing the actual lines added and removed
3. **Given** a change record for a modified file, **When** the diff snippet is generated, **Then** the snippet includes up to 20 lines of context around each changed hunk (configurable), preserving enough surrounding code for an agent to understand the change without reading the full file
4. **Given** a `query_changes` call with `file_path: "src/server/router.rs"`, **When** the query executes, **Then** the system returns all commit nodes that include a change record for that file, ordered by timestamp descending
5. **Given** a `query_changes` call with `symbol: "build_router"`, **When** the query executes, **Then** the system cross-references the commit graph with the code graph to return only commits where the diff touched lines within the `build_router` function's line range
6. **Given** a repository with 10,000 commits, **When** initial git graph indexing runs, **Then** the system processes commits in reverse chronological order and supports a configurable depth limit (default: 500 most recent commits) to bound initial indexing time
7. **Given** a new commit is made after initial indexing, **When** incremental sync runs, **Then** only the new commits since the last indexed commit are processed and added to the graph
8. **Given** a merge commit with multiple parents, **When** the commit is indexed, **Then** all parent references are preserved in the commit node, enabling branch topology traversal

---

### User Story 5 - Agent Hooks and Integration Instructions (Priority: P5)

As a developer installing Engram, I receive ready-to-use hook configurations and agent instruction files so that my AI coding assistants (Claude Code, GitHub Copilot, Cursor) automatically use Engram for task memory and code context — without manual configuration.

**Why this priority**: Engram is only valuable if agents actually use it. Without hooks and instructions, developers must manually configure each agent to connect to Engram's MCP endpoint. This story automates that setup, reducing friction to zero. It's P5 because it's an integration concern that depends on the core features (registry, ingestion, rehydration) being functional first.

**Independent Test**: Run `engram install` in a fresh workspace. Verify that hook configuration files are created for at least two supported agent platforms (e.g., `.github/copilot-instructions.md` for Copilot, `.claude/settings.json` for Claude). Verify the instruction files contain correct MCP endpoint URLs and tool usage guidance. Start Engram, then start an agent session — verify the agent discovers and connects to Engram without additional user action.

**Acceptance Scenarios**:

1. **Given** a workspace with no existing agent hook files, **When** `engram install` runs, **Then** the installer creates agent instruction files for all supported platforms (GitHub Copilot, Claude Code, Cursor) with Engram MCP endpoint configuration and tool usage guidance
2. **Given** existing agent hook files in the workspace, **When** `engram install` runs, **Then** the installer detects existing files and appends Engram-specific configuration rather than overwriting user content, using clear section markers (e.g., `<!-- engram:start -->` / `<!-- engram:end -->`)
3. **Given** a generated instruction file, **When** an agent reads it, **Then** the instructions explain which Engram tools to use for common workflows: `set_workspace` on session start, `query_memory` for context retrieval, `create_task` / `update_task` for task management, and `map_code` for code navigation
4. **Given** `engram install --hooks-only`, **When** the command runs, **Then** only hook and instruction files are created/updated, without modifying `.engram/` data files or the registry
5. **Given** a workspace where Engram's port is configured to a non-default value, **When** hook files are generated, **Then** the MCP endpoint URL in the instructions reflects the configured port

---

### User Story 6 - Project Documentation (Priority: P6)

As a developer evaluating or onboarding to Engram, I access comprehensive documentation covering installation, configuration, MCP tool reference, architecture overview, and troubleshooting — so that I can understand what Engram does, how to set it up, and how to diagnose issues without reading source code.

**Why this priority**: Documentation is essential for adoption but does not block any functional capability. It is P6 because it should be written after the features it documents (registry, ingestion, rehydration, git tracking, hooks) are specified and stable.

**Independent Test**: Verify that `docs/` contains at minimum: a quickstart guide, an MCP tool reference, a configuration reference, an architecture overview, and a troubleshooting guide. Verify the quickstart guide can be followed from scratch to a working Engram setup. Verify the MCP tool reference covers every registered tool with parameters, return types, and examples.

**Acceptance Scenarios**:

1. **Given** a new user reading the quickstart guide, **When** they follow the documented steps, **Then** they can install Engram, configure a workspace, start the daemon, and verify connectivity within 10 minutes
2. **Given** the MCP tool reference, **When** a developer looks up any Engram tool (e.g., `query_memory`, `create_task`, `map_code`), **Then** they find the tool's purpose, required parameters, optional parameters, return schema, error codes, and at least one usage example
3. **Given** the configuration reference, **When** a developer wants to change a setting (port, timeout, data directory, log format), **Then** the document explains each option, its default value, how to set it via CLI flag or environment variable, and any constraints
4. **Given** the architecture overview, **When** a developer or contributor reads it, **Then** they understand the high-level component diagram (binary entrypoint, HTTP/SSE transport, MCP dispatch, SurrealDB persistence, code graph, content registry) and data flow between components
5. **Given** a troubleshooting guide, **When** a developer encounters a common issue (daemon won't start, workspace binding fails, search returns no results), **Then** the guide provides diagnostic steps, expected log output to look for, and resolution actions

---

### Edge Cases

- What happens when a `registry.yaml` references a path outside the workspace root? The system MUST reject it per workspace isolation (Constitution Principle IV) and log a security warning.
- What happens when two registry entries declare overlapping paths (e.g., `src/` and `src/models/`)? The system MUST deduplicate content records to avoid double-ingestion, preferring the more specific path's type label for files in the overlap.
- How does the system handle a `.engram/backlog-NNN.json` file that has been manually edited with invalid JSON? The system MUST report a parse error for that backlog file, skip it, and continue hydrating other backlog files.
- What happens when the git history is shallow (e.g., `--depth 1` clone)? The system MUST index only the available commits and log an informational message that history depth is limited.
- What happens when `engram install` is run in a workspace that already has `.engram/` files from a prior version? The installer MUST check the `.engram/.version` file, warn about version mismatch, and offer migration rather than overwriting existing data.
- How does the system handle a registered path that contains symlinks pointing outside the workspace? Symlinks MUST be resolved and validated against workspace boundaries before ingestion.
- What happens when a backlog JSON file references a SpecKit feature directory that no longer exists on disk? During dehydration, the system MUST log a warning and skip writing that backlog file, preserving the existing JSON as an archive.

## Requirements *(mandatory)*

### Functional Requirements

**Content Registry**

- **FR-001**: System MUST support a `.engram/registry.yaml` file that declares content sources with `type`, `language`, and `path` fields
- **FR-002**: System MUST auto-detect common workspace directories (src, tests, specs, docs) during `engram install` and generate a default registry with appropriate entries
- **FR-003**: System MUST validate each registry entry during hydration — confirming the path exists and the type is a recognized content type or a valid custom type
- **FR-004**: System MUST support the following built-in content types: `code`, `tests`, `spec`, `docs`, `memory`, `context`, `instructions`
- **FR-005**: System MUST accept developer-defined custom content types beyond the built-in set

**Multi-Source Ingestion**

- **FR-006**: System MUST ingest content from all registered sources during hydration, creating type-partitioned records in the database
- **FR-007**: System MUST re-ingest only changed files when file change events are detected in registered paths
- **FR-008**: System MUST support content type filters on `query_memory` and `unified_search` calls, allowing agents to scope searches to specific content types
- **FR-009**: System MUST skip files exceeding a configurable size limit (default: 1 MB) during ingestion with a logged warning
- **FR-010**: System MUST route `type: code` registry entries through the existing code graph indexer rather than raw text ingestion

**SpecKit-Aware Rehydration**

- **FR-011**: System MUST produce a `.engram/project.json` file containing project-level metadata and references to per-feature backlog JSON files
- **FR-012**: System MUST produce one `.engram/backlog-NNN.json` file per SpecKit feature directory, numbered to match the feature directory
- **FR-013**: Each backlog JSON MUST contain feature metadata (id, name, title, git branch, spec path, description, status) and the full text contents of all SpecKit artifacts found in the feature directory
- **FR-014**: System MUST update backlog JSON files during dehydration when task or context records change in SurrealDB
- **FR-015**: System MUST fall back to legacy `.engram/tasks.md` behavior for workspaces without SpecKit feature directories

**Git Commit Graph**

- **FR-016**: System MUST index git commits as graph nodes with hash, author, timestamp, message, and parent references
- **FR-017**: System MUST store per-file change records for each commit, including file path, change type, and a diff snippet with configurable context lines (default: 20)
- **FR-018**: System MUST support a `query_changes` tool that filters commit history by file path, symbol name, or date range
- **FR-019**: System MUST support a configurable commit depth limit for initial indexing (default: 500 most recent commits)
- **FR-020**: System MUST support incremental commit sync — processing only new commits since the last indexed commit hash

**Agent Hooks**

- **FR-021**: System MUST generate agent instruction and hook files during `engram install` for supported platforms (GitHub Copilot, Claude Code, Cursor)
- **FR-022**: System MUST detect existing agent configuration files and append rather than overwrite, using section markers for Engram-specific content
- **FR-023**: System MUST include MCP endpoint configuration, tool usage guidance, and recommended workflows in generated instruction files
- **FR-024**: System MUST support `engram install --hooks-only` to create/update only agent hook files without modifying data files

**Documentation**

- **FR-025**: System MUST include a quickstart guide that enables a new user to go from zero to a running Engram setup
- **FR-026**: System MUST include an MCP tool reference documenting every registered tool with parameters, return types, error codes, and examples
- **FR-027**: System MUST include a configuration reference covering all CLI flags, environment variables, and defaults
- **FR-028**: System MUST include an architecture overview with component descriptions and data flow
- **FR-029**: System MUST include a troubleshooting guide covering common failure modes with diagnostic steps

### Key Entities

- **ContentSource**: A declared content source from the registry — type (code, tests, spec, docs, etc.), language, file system path, status (active, missing, error)
- **ContentRecord**: An ingested piece of content — source type, file path, content hash, last ingested timestamp, optional embedding
- **BacklogFile**: A per-feature JSON file linking SpecKit artifacts — feature id, feature name, git branch, artifact contents (spec, plan, tasks, scenarios, research, analysis)
- **ProjectManifest**: The project-level metadata file — project name, description, repository URL, default branch, array of backlog file references
- **CommitNode**: A git commit in the graph — hash, author, timestamp, message, parent hashes, array of change records
- **ChangeRecord**: A per-file diff within a commit — file path, change type (add/modify/delete), diff snippet, line range

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Developers can declare workspace content sources in under 2 minutes by editing `registry.yaml`, and auto-detection generates a working default registry for standard project layouts with zero manual configuration
- **SC-002**: Agents searching for specification content receive only spec-type results when filtering by content type, with zero cross-type contamination in filtered queries
- **SC-003**: SpecKit-organized workspaces with up to 10 feature directories complete hydration/dehydration cycles with all artifacts preserved — no data loss across daemon restarts
- **SC-004**: Agents querying git change history for a specific file receive relevant commit details with code snippets in under 3 seconds for repositories with up to 10,000 commits
- **SC-005**: New developers complete Engram installation and agent connection following only the quickstart guide, without requiring external assistance or source code reading
- **SC-006**: Agent hook installation covers at least 3 major AI coding platforms, reducing manual MCP configuration steps from 5+ per platform to zero
- **SC-007**: Incremental content sync (file changes + new commits) processes updates in under 5 seconds for typical change sets (1-10 files), maintaining workspace freshness without full re-indexing

## Assumptions

- The workspace uses Git as its version control system. Non-Git workspaces receive all features except git commit tracking.
- SpecKit feature directories follow the naming convention `specs/NNN-feature-name/` where NNN is a zero-padded number. Directories not matching this pattern are treated as regular spec content sources.
- The `registry.yaml` format is YAML because it is human-readable, widely understood, and already used in CI/CD ecosystems familiar to the target audience.
- Agent hook file formats and locations are based on publicly documented conventions for each platform as of 2026. If a platform changes its convention, the installer must be updated.
- Documentation is written in Markdown and stored in the `docs/` directory of the repository, publishable via GitHub Pages or similar static site generators.
- The existing `.engram/tasks.md` legacy format will continue to be supported indefinitely as a fallback. It is not deprecated by this feature.
