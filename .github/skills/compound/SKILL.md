---
name: compound
description: "Document a recently solved problem to compound institutional knowledge. Captures solutions in .backlog/compound/ with searchable YAML frontmatter for the learnings-researcher. Use after fixing bugs, resolving build issues, or discovering gotchas."
argument-hint: "[optional: brief context about the fix or discovery]"
---

# Compound Knowledge

Capture a recently solved problem while context is fresh. Produces structured documentation in `.backlog/compound/` with YAML frontmatter searchable by the `learnings-researcher` agent.

**Why "compound"?** Each documented solution compounds institutional knowledge. The first time you solve a problem takes research. Document it, and the next occurrence takes minutes.

## Agent-Intercom Communication (NON-NEGOTIABLE)

Call `ping` at session start. If agent-intercom is reachable, broadcast at every step. If unreachable, warn the user that operator visibility is degraded.

| Event | Level | Message prefix |
|---|---|---|
| Session start | info | `[COMPOUND] Starting knowledge capture: {context}` |
| Analyzer spawned | info | `[SPAWN] {analyzer_name} for knowledge extraction` |
| Analyzer returned | info | `[RETURN] {analyzer_name}: {summary}` |
| Solution doc written | success | `[COMPOUND] Solution written: {file_path}` |
| Session complete | success | `[COMPOUND] Complete: {title}` |

## Subagent Depth Constraint

This skill spawns analyzer subagents. Those subagents are leaf executors and MUST NOT spawn their own subagents. Maximum depth: compound skill -> analyzer subagent (1 hop).

## Workflow

### Phase 1: Parallel Research

Launch 3 subagents in parallel. Each returns text data to this orchestrator. Subagents MUST NOT write files.

#### 1. Context Analyzer

Extracts from conversation history:

- Problem type, component, symptoms
- Validates fields against schema enums (see below)
- Maps `problem_type` to category directory
- Suggests filename: `{sanitized-problem-slug}-{YYYY-MM-DD}.md`
- Returns: YAML frontmatter skeleton, category directory path, suggested filename

#### 2. Solution Extractor

Analyzes investigation steps:

- Identifies root cause
- Extracts working solution with code examples
- Develops prevention strategies
- Returns structured solution content block with sections: Problem, Symptoms, What Did Not Work, Solution, Why This Works, Prevention

#### 3. Related Docs Finder

Searches `.backlog/compound/` for related existing solutions:

- Uses engram `unified_search` first, grep fallback
- Identifies cross-references and related documents
- Returns list of related solutions with relevance assessment

### Phase 2: Synthesize and Write

The orchestrator (this skill) synthesizes subagent output and writes a single file.

**Only this orchestrator writes files. Subagents return text data only.**

Write to `.backlog/compound/{category}/{slug}-{YYYY-MM-DD}.md`

### Schema Enums

#### problem_type (maps to category directory)

| Value | Directory |
|---|---|
| `build_error` | `build-errors/` |
| `test_failure` | `test-failures/` |
| `runtime_error` | `runtime-errors/` |
| `database_issue` | `database-issues/` |
| `security_issue` | `security-issues/` |
| `concurrency_issue` | `concurrency-issues/` |
| `mcp_protocol_issue` | `mcp-protocol-issues/` |
| `best_practice` | `best-practices/` |
| `workflow_issue` | `workflow-issues/` |

#### component

`db_queries`, `db_schema`, `db_workspace`, `mcp_tools`, `mcp_server`, `sse_handler`, `code_graph`, `hydration`, `dehydration`, `embedding`, `config`, `models`, `errors`, `installer`, `daemon`, `ipc`

#### root_cause

`missing_import`, `type_mismatch`, `serde_flatten`, `race_condition`, `deadlock`, `thing_deserialization`, `nan_embedding`, `schema_mismatch`, `path_traversal`, `lock_contention`, `missing_feature_gate`, `incorrect_error_code`, `missing_test_registration`, `timeout`, `stale_lock_file`

#### resolution_type

`code_fix`, `config_change`, `test_fix`, `schema_update`, `dependency_update`, `feature_gate`, `error_mapping`, `documentation`

#### severity

`critical`, `high`, `medium`, `low`

### Document Template

The frontmatter includes fields aligned with the BugEvent model from the bug-logging feature to enable cross-referencing between compound solutions and recorded bugs.

```markdown
---
title: "{Problem Title}"
problem_type: {enum value}
category: {same as problem_type — alias for BugEvent cross-referencing}
component: {enum value}
root_cause: {enum value}
resolution_type: {enum value}
severity: {enum value}
message: "{Short description, max 200 chars — matches BugEvent.message}"
file_path: "{Affected file path, if applicable}"
resolved: true
bug_id: "{Reserved for future use — only populate once the bug-logging MCP tool exists and returns a valid BugEvent UUID. Leave empty until the bug-logging feature is implemented.}"
tags: [{relevant, searchable, keywords}]
date: YYYY-MM-DD
---

# {Problem Title}

## Problem

{1-2 sentence description}

## Symptoms

{Observable symptoms, error messages, behavior}

## What Did Not Work

{Failed approaches and why they failed}

## Solution

{The actual fix with code examples}

### Before

```rust
// Code before the fix
```

### After

```rust
// Code after the fix
```

## Why This Works

{Root cause explanation and why the solution addresses it}

## Prevention

{Strategies to avoid recurrence}

- {Concrete prevention step 1}
- {Concrete prevention step 2}

## Related Solutions

{Cross-references to related docs in .backlog/compound/}
```

Broadcast the file path when the document is written.
