# Quickstart: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence

This guide walks you through setting up Engram's workspace content intelligence features in an existing project.

## Prerequisites

- Engram binary installed (see main README)
- A Git-initialized workspace with source code
- At least one AI coding assistant (GitHub Copilot, Claude Code, or Cursor)

## Step 1: Install Engram in Your Workspace

```bash
cd /path/to/your/project
engram install
```

This command:
1. Creates `.engram/` directory with default configuration
2. Auto-detects your project structure and generates `.engram/registry.yaml`
3. Generates agent hook files for supported AI platforms
4. Writes `.engram/.version` for schema compatibility

## Step 2: Review the Content Registry

Open `.engram/registry.yaml` to see what was auto-detected:

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
```

Add custom entries as needed:

```yaml
  - type: context
    language: markdown
    path: .context
  - type: instructions
    language: markdown
    path: .github
```

## Step 3: Start the Daemon

```bash
engram --workspace /path/to/your/project
```

On startup, Engram will:
1. Read the registry and validate all source paths
2. Ingest content from registered sources into SurrealDB
3. Index git history (if `git-graph` feature enabled)
4. Read SpecKit feature directories and create backlog JSON files (if applicable)

## Step 4: Verify Integration

From your AI coding assistant, verify Engram is connected:

```
> Use the get_workspace_status tool to check Engram connectivity
```

The response should show registered sources, content record counts, and git graph status.

## Step 5: Use Content-Aware Search

Query by content type:

```
> Search Engram for "hydration" in spec content only
```

This translates to a `query_memory` call with `content_type: "spec"`, returning only specification documents — not code or test files.

## Step 6: Query Git History

Find what changed in a specific file:

```
> Ask Engram what commits changed src/services/hydration.rs
```

This calls `query_changes` with a file path filter, returning commit details with actual code diff snippets.

## Common Tasks

| Task | Tool | Example |
|------|------|---------|
| Search specs only | `query_memory` with `content_type: "spec"` | "Find requirements about authentication" |
| Search all content | `unified_search` without filter | "Find all references to workspace isolation" |
| View file change history | `query_changes` with `file_path` | "What changed in router.rs?" |
| View function changes | `query_changes` with `symbol` | "What commits touched build_router?" |
| Check workspace status | `get_workspace_status` | "Show registry and indexing status" |

## Troubleshooting

- **Registry not found**: Run `engram install` to generate the default registry
- **Source path missing**: Check that the path in `registry.yaml` exists relative to workspace root
- **Git graph empty**: Ensure the `git-graph` feature is enabled at compile time
- **Agent not connecting**: Verify hook files were generated in `.github/`, `.claude/`, or `.cursor/`
