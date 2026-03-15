# Backlog

## Feature Requests (unassigned)

- Create engram specific SDD agents, skills, instructions, prompts, and tools; enable them to support loop driven dev (LDD)
- Create loop driven development workflows that leverage the engram specific SDD.

## Feature: 006-workspace-content-intelligence

1. Need the ability to also track git commit numbers to changes in the repo through a graph representation with actual code and text snippets to enable faster change detection and search during agentic adversarial code reviews.
2. Need to include hooks with engram and instructions to ensure the agent uses engram for tasks and code memory.
3. Need proper documentation for this solution.
4. **SpecKit-aware rehydration via structured backlog JSON files.**
   Engram should not assume a single `tasks.md` at a fixed path. Workspaces using SpecKit have multiple feature folders (e.g., `specs/001-core-mcp-daemon/`, `specs/002-enhanced-task-management/`) each containing their own `spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `quickstart.md`, `ANALYSIS.md`, `SCENARIOS.md`, checklists, and contracts. Engram must capture ALL of this content as part of task and context management. Requirements:
   - **`project.json`** in `.engram/` — describes the overall project and links to each backlog file. Contains project-level metadata (name, description, repository URL, default branch).
   - **`backlog-00X.json`** files in `.engram/` — one per SpecKit feature, numbered to match the feature directory (e.g., `backlog-001.json` for `specs/001-core-mcp-daemon/`). Each backlog JSON contains:
     - Feature metadata: `id`, `name`, `title`, `git_branch`, `spec_path`, `description`, `status`, `spec_status`
     - Full contents of all SpecKit artifacts: analysis, research, scenarios, spec, plan, tasks
     - Sub-nodes for each feature item, each with at minimum: `id`, `name`, `description`
   - Hydration reads these JSON files (not raw markdown) to reconstruct workspace state in SurrealDB.
   - Dehydration writes back to these JSON files, preserving the SpecKit folder structure linkage.
   - The existing `.engram/tasks.md` format should be treated as a legacy/fallback for workspaces that don't use SpecKit.
5. **Content registry and multi-source ingestion pipeline (`registry.yaml`).**
   Engram currently has no awareness of workspace content outside `.engram/`. It does not ingest specs, docs, tests, tracking files, context files, or README. A developer-configurable content registry is needed.
   - **`.engram/registry.yaml`** — a registry file where the developer declares content sources, their types, languages, and paths. Engram reads this at hydration time to know what to monitor, ingest, and how to partition data in SurrealDB. Example entries:
     ```yaml
     sources:
       - type: code
         language: rust
         path: src
       - type: tests
         language: rust
         path: tests
       - type: spec
         language: markdown
         path: specs
       - type: docs
         language: markdown
         path: docs
       - type: memory
         language: markdown
         path: .copilot-tracking
       - type: context
         language: markdown
         path: .context
       - type: instructions
         language: markdown
         path: .github
     ```
   - Each registered source drives:
     - **File watcher scope**: the watcher monitors registered paths instead of relying on hardcoded exclude patterns.
     - **Ingestion pipeline**: on hydration and on file-change events, engram reads content from registered paths and loads it into SurrealDB, partitioned by `type` (e.g., separate tables or namespaces for code, specs, docs, memory, context).
     - **Search partitioning**: `query_memory`, `unified_search`, and future search tools can filter by content type, enabling queries like "search specs only" or "search code and tests".
     - **Code graph integration**: registered `type: code` sources feed into `index_workspace` / `sync_workspace` with the declared language, replacing the current Rust-only default.
   - The installer (`engram install`) should generate a default `registry.yaml` with sensible entries detected from the workspace structure (e.g., if `specs/` exists, add a spec source; if `src/` exists, add a code source).
   - The registry is additive — developers can add custom content types and paths as needed for their project structure.
