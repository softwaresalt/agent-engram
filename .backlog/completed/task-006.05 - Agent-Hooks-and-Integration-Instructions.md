---
id: TASK-006.05
title: '006-05: Agent Hooks and Integration Instructions'
status: Done
assignee: []
created_date: '2026-03-15'
labels:
  - feature
  - 006
  - userstory
  - p5
dependencies: []
references:
  - specs/006-workspace-content-intelligence/spec.md
parent_task_id: TASK-006
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer installing Engram, I receive ready-to-use hook configurations and agent instruction files so that my AI coding assistants (Claude Code, GitHub Copilot, Cursor) automatically use Engram for task memory and code context — without manual configuration.

**Why this priority**: Engram is only valuable if agents actually use it. Without hooks and instructions, developers must manually configure each agent to connect to Engram's MCP endpoint. This story automates that setup, reducing friction to zero. It's P5 because it's an integration concern that depends on the core features (registry, ingestion, rehydration) being functional first.

**Independent Test**: Run `engram install` in a fresh workspace. Verify that hook configuration files are created for at least two supported agent platforms (e.g., `.github/copilot-instructions.md` for Copilot, `.claude/settings.json` for Claude). Verify the instruction files contain correct MCP endpoint URLs and tool usage guidance. Start Engram, then start an agent session — verify the agent discovers and connects to Engram without additional user action.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with no existing agent hook files, **When** `engram install` runs, **Then** the installer creates agent instruction files for all supported platforms (GitHub Copilot, Claude Code, Cursor) with Engram MCP endpoint configuration and tool usage guidance
- [x] #2 **Given** existing agent hook files in the workspace, **When** `engram install` runs, **Then** the installer detects existing files and appends Engram-specific configuration rather than overwriting user content, using clear section markers (e.g., `<!-- engram:start -->` / `<!-- engram:end -->`)
- [x] #3 **Given** a generated instruction file, **When** an agent reads it, **Then** the instructions explain which Engram tools to use for common workflows: `set_workspace` on session start, `query_memory` for context retrieval, `create_task` / `update_task` for task management, and `map_code` for code navigation
- [x] #4 **Given** `engram install --hooks-only`, **When** the command runs, **Then** only hook and instruction files are created/updated, without modifying `.engram/` data files or the registry
- [x] #5 **Given** a workspace where Engram's port is configured to a non-default value, **When** hook files are generated, **Then** the MCP endpoint URL in the instructions reflects the configured port ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 7: User Story 5 — Agent Hooks and Integration Instructions (Priority: P5)

**Goal**: `engram install` generates agent hook files for GitHub Copilot, Claude Code, and Cursor with MCP endpoint configuration and tool usage guidance.

**Independent Test**: Run `engram install`, verify hook files for 3 platforms with correct MCP URLs and section markers.

### Tests for User Story 5

- [x] T043 [P] [US5] Integration test for hook file generation in tests/integration/installer_test.rs — verify S064 (fresh install creates 3 platform files), S065 (existing file → append with markers), S066 (re-install → replace marker content), S067 (--hooks-only flag), S068 (custom port in URLs), S069 (--no-hooks flag)

### Implementation for User Story 5

- [x] T044 [US5] Implement hook file templates in src/installer/mod.rs — define template content for GitHub Copilot (.github/copilot-instructions.md), Claude Code (.claude/settings.json + .claude/instructions.md), Cursor (.cursor/mcp.json) with MCP endpoint URL, tool listing, and recommended workflows
- [x] T045 [US5] Implement section-marker insertion logic in src/installer/mod.rs — detect existing files, find `<!-- engram:start -->` / `<!-- engram:end -->` markers, replace content between markers (or append if no markers), preserve all user content outside markers
- [x] T046 [US5] Implement --hooks-only and --no-hooks CLI flags in src/config/mod.rs and src/installer/mod.rs — add flags to clap config, when --hooks-only: skip data file creation, when --no-hooks: skip hook generation
- [x] T047 [US5] Implement port-aware URL generation in src/installer/mod.rs — read configured port from Config, substitute into MCP endpoint URLs in hook templates

**Checkpoint**: Agent hooks auto-generated for 3 platforms with idempotent marker-based updates.

---
<!-- SECTION:PLAN:END -->

