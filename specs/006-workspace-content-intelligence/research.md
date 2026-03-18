# Research: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15

## Research Areas

### R1: YAML Registry Format Design

**Decision**: Use `serde_yaml` 0.9 for parsing `.engram/registry.yaml` with a simple flat list of source entries.

**Rationale**: YAML is the standard for developer-facing configuration files (CI/CD, Kubernetes, GitHub Actions). The schema is intentionally flat — each entry has `type`, `language`, and `path` — avoiding nested complexity. `serde_yaml` is the de facto Rust YAML library with mature serde integration.

**Alternatives considered**:
- TOML: Already used for Cargo.toml but YAML is more natural for list-heavy configs. TOML arrays-of-tables syntax is less readable for this use case.
- JSON: Valid but less human-friendly for manual editing. JSON lacks comments.
- Custom format: Unnecessary complexity. YAML handles the requirements.

**Schema**:
```yaml
# .engram/registry.yaml
sources:
  - type: code        # Built-in or custom content type
    language: rust     # Language hint (used by code graph indexer)
    path: src          # Relative path from workspace root
  - type: tests
    language: rust
    path: tests
```

### R2: Git Commit Graph Access Strategy

**Decision**: Use `git2` 0.19 (libgit2 bindings) behind a `git-graph` feature flag.

**Rationale**: The Constitution forbids shell execution (Security > Process Security: "No shell execution — never spawn shells or execute arbitrary commands"). This rules out shelling out to `git log`, `git diff`, etc. `git2` provides direct programmatic access to git objects, diffs, and commit walks without spawning processes. It adds ~2MB to the binary but is the only safe option.

**Alternatives considered**:
- `gix` (gitoxide): Pure Rust git implementation. More aligned with safety-first principles but less mature for diff generation. Consider migrating in a future version when gix diff support stabilizes.
- Shell `git` commands: Ruled out by Constitution. Security risk and parsing complexity.
- Reading `.git/` directly: Too low-level, error-prone, and would duplicate git's complex pack file logic.

**Implementation notes**:
- Use `git2::Repository::open()` to access the workspace git repo
- Walk commits with `git2::Revwalk` in reverse chronological order
- Generate diffs with `git2::Diff::tree_to_tree()` for each commit
- Extract hunks with context lines from `git2::DiffHunk`
- Use `spawn_blocking` for all git2 operations (they are synchronous/blocking)

### R3: SpecKit Backlog JSON Schema

**Decision**: Define a structured JSON schema for `backlog-NNN.json` files that captures feature metadata and artifact contents.

**Rationale**: SpecKit artifacts are structured data with multiple fields and nested relationships. JSON preserves this structure faithfully, is natively supported by `serde_json` (already a dependency), and is text-based (Git-friendly per Constitution VI). Markdown would lose the hierarchical structure.

**Schema (backlog-NNN.json)**:
```json
{
  "id": "001",
  "name": "core-mcp-daemon",
  "title": "Core MCP Daemon",
  "git_branch": "001-core-mcp-daemon",
  "spec_path": "specs/001-core-mcp-daemon",
  "description": "...",
  "status": "complete",
  "spec_status": "approved",
  "artifacts": {
    "spec": "# Feature Specification: ...",
    "plan": "# Implementation Plan: ...",
    "tasks": "# Task Breakdown: ...",
    "scenarios": "# Behavioral Matrix: ...",
    "research": "# Research: ...",
    "analysis": "# Analysis: ...",
    "data_model": null,
    "quickstart": null
  },
  "items": [
    {
      "id": "T001",
      "name": "setup-project-structure",
      "description": "..."
    }
  ]
}
```

**Schema (project.json)**:
```json
{
  "name": "agent-engram",
  "description": "MCP daemon for persistent task memory",
  "repository_url": "https://github.com/softwaresalt/agent-engram",
  "default_branch": "main",
  "backlogs": [
    { "id": "001", "path": ".engram/backlog-001.json" },
    { "id": "002", "path": ".engram/backlog-002.json" }
  ]
}
```

### R4: Content Ingestion and SurrealDB Partitioning Strategy

**Decision**: Use a single `content_record` table in SurrealDB with a `content_type` field for partitioning. Type-filtered queries use `WHERE content_type = $type`.

**Rationale**: SurrealDB's query language handles field-based filtering efficiently. A single table avoids schema proliferation (one table per content type would require dynamic table creation for custom types). The `content_type` field enables both filtered and unfiltered queries with a single index.

**Alternatives considered**:
- Separate tables per type: Would require dynamic `DEFINE TABLE` for custom types. SurrealDB doesn't support parameterized table names in queries, making this fragile.
- Separate namespaces: Overkill — namespaces are for workspace isolation, not content type separation.
- Tags/labels: Could work but adds unnecessary indirection when we have a clear type field.

### R5: Agent Hook File Conventions

**Decision**: Support three platforms at launch — GitHub Copilot, Claude Code, and Cursor — with idempotent section-marker-based insertion.

**Research findings**:
- **GitHub Copilot**: Instructions via `.github/copilot-instructions.md` (workspace-level) or VS Code `settings.json` under `github.copilot.chat.codeGeneration.instructions`.
- **Claude Code**: MCP server configuration via `.claude/settings.json` with `mcpServers` key; instructions via `.claude/instructions.md`.
- **Cursor**: MCP configuration via `.cursor/mcp.json`; rules via `.cursorrules` or `.cursor/rules/`.

**Section marker strategy**:
```markdown
<!-- engram:start -->
[Engram-generated content here]
<!-- engram:end -->
```
On subsequent runs, content between markers is replaced; content outside markers is preserved.

### R6: File Size and Ingestion Limits

**Decision**: Default max file size 1 MB, configurable via `registry.yaml` top-level `max_file_size_bytes` field. Default batch size 50 files per ingestion cycle.

**Rationale**: 1 MB covers virtually all source code and documentation files. Files exceeding this are typically generated artifacts (compiled output, large data files) that should not be ingested. Batch processing prevents memory exhaustion when ingesting hundreds of files.

## Unresolved Items

None — all research questions resolved with concrete decisions.
