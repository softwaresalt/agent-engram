---
id: TASK-006.06
title: '006-06: Project Documentation'
status: Done
assignee: []
created_date: '2026-03-15'
labels:
  - feature
  - 006
  - userstory
  - p6
dependencies: []
references:
  - specs/006-workspace-content-intelligence/spec.md
parent_task_id: TASK-006
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer evaluating or onboarding to Engram, I access comprehensive documentation covering installation, configuration, MCP tool reference, architecture overview, and troubleshooting — so that I can understand what Engram does, how to set it up, and how to diagnose issues without reading source code.

**Why this priority**: Documentation is essential for adoption but does not block any functional capability. It is P6 because it should be written after the features it documents (registry, ingestion, rehydration, git tracking, hooks) are specified and stable.

**Independent Test**: Verify that `docs/` contains at minimum: a quickstart guide, an MCP tool reference, a configuration reference, an architecture overview, and a troubleshooting guide. Verify the quickstart guide can be followed from scratch to a working Engram setup. Verify the MCP tool reference covers every registered tool with parameters, return types, and examples.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a new user reading the quickstart guide, **When** they follow the documented steps, **Then** they can install Engram, configure a workspace, start the daemon, and verify connectivity within 10 minutes
- [x] #2 **Given** the MCP tool reference, **When** a developer looks up any Engram tool (e.g., `query_memory`, `create_task`, `map_code`), **Then** they find the tool's purpose, required parameters, optional parameters, return schema, error codes, and at least one usage example
- [x] #3 **Given** the configuration reference, **When** a developer wants to change a setting (port, timeout, data directory, log format), **Then** the document explains each option, its default value, how to set it via CLI flag or environment variable, and any constraints
- [x] #4 **Given** the architecture overview, **When** a developer or contributor reads it, **Then** they understand the high-level component diagram (binary entrypoint, HTTP/SSE transport, MCP dispatch, SurrealDB persistence, code graph, content registry) and data flow between components
- [x] #5 **Given** a troubleshooting guide, **When** a developer encounters a common issue (daemon won't start, workspace binding fails, search returns no results), **Then** the guide provides diagnostic steps, expected log output to look for, and resolution actions --- ### Edge Cases - What happens when a `registry.yaml` references a path outside the workspace root? The system MUST reject it per workspace isolation (Constitution Principle IV) and log a security warning. - What happens when two registry entries declare overlapping paths (e.g., `src/` and `src/models/`)? The system MUST deduplicate content records to avoid double-ingestion, preferring the more specific path's type label for files in the overlap. - How does the system handle a `.engram/backlog-NNN.json` file that has been manually edited with invalid JSON? The system MUST report a parse error for that backlog file, skip it, and continue hydrating other backlog files. - What happens when the git history is shallow (e.g., `--depth 1` clone)? The system MUST index only the available commits and log an informational message that history depth is limited. - What happens when `engram install` is run in a workspace that already has `.engram/` files from a prior version? The installer MUST check the `.engram/.version` file, warn about version mismatch, and offer migration rather than overwriting existing data. - How does the system handle a registered path that contains symlinks pointing outside the workspace? Symlinks MUST be resolved and validated against workspace boundaries before ingestion. - What happens when a backlog JSON file references a SpecKit feature directory that no longer exists on disk? During dehydration, the system MUST log a warning and skip writing that backlog file, preserving the existing JSON as an archive.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 8: User Story 6 — Project Documentation (Priority: P6)

**Goal**: Comprehensive documentation in docs/ covering quickstart, MCP tool reference, configuration, architecture, and troubleshooting.

**Independent Test**: Verify all 5 doc files exist with required sections. Follow quickstart guide end-to-end.

### Implementation for User Story 6

- [x] T048 [P] [US6] Write quickstart guide in docs/quickstart.md — installation steps, workspace setup, daemon startup, agent connection verification, first search query
- [x] T049 [P] [US6] Write MCP tool reference in docs/mcp-tool-reference.md — every registered tool with purpose, required parameters, optional parameters, return schema, error codes, usage example; organized by category (lifecycle, read, write, graph)
- [x] T050 [P] [US6] Write configuration reference in docs/configuration.md — all CLI flags (--port, --timeout, --data-dir, --log-format, --workspace), all environment variables (ENGRAM_PORT, ENGRAM_TIMEOUT, etc.), defaults, constraints, examples
- [x] T051 [P] [US6] Write architecture overview in docs/architecture.md — component diagram (binary entrypoint, IPC transport, MCP dispatch, SurrealDB, code graph, content registry, git graph), data flow, workspace lifecycle, module responsibilities
- [x] T052 [P] [US6] Write troubleshooting guide in docs/troubleshooting.md — common issues (daemon won't start, workspace binding fails, search returns no results, registry validation errors), diagnostic steps, expected log output, resolution actions

**Checkpoint**: All documentation deliverables complete.

---
<!-- SECTION:PLAN:END -->

