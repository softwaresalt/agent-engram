---
title: "Code Graph Infrastructure — Branch-Aware Storage + Multi-Language Parsing"
description: "Unified deliberation for relocating code graph JSONL into per-branch storage and adding Tier 1 multi-language tree-sitter support"
topic: "Group A: Code Graph Infrastructure (003-F + 004-F)"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/research/2026-03-28-multi-language-tree-sitter-requirements.md"
  - ".backlogit/queue/003-F.md"
  - ".backlogit/queue/004-F.md"
tags:
  - "code-graph"
  - "tree-sitter"
  - "multi-language"
  - "storage-migration"
---

## Problem Frame

Engram's code graph has two structural limitations that reduce its usefulness:

1. **Storage misalignment (003-F):** Code graph JSONL files are stored at
   `.engram/code-graph/` (workspace root), while the SurrealDB database is stored
   at `.engram/db/{branch}/`. This means graph dehydration data is NOT
   branch-aware — switching branches does not switch the serialized graph state.
   The DB and the JSONL backup are conceptually part of the same data set but live
   in different directory hierarchies.

2. **Single-language parsing (004-F):** The `parse_rust_source` function in
   `src/services/parsing.rs` hard-codes `tree_sitter_rust::LANGUAGE`. Every
   non-Rust file produces an empty graph. This limits engram to Rust-only
   workspaces, excluding the majority of real-world projects.

These two features are grouped because they both touch the code graph subsystem
and benefit from coordinated delivery. Relocating the storage (003-F) is a
natural prerequisite to avoid writing multi-language graph data to a
non-branch-aware location.

## Research Findings

### Prior Research

`docs/research/2026-03-28-multi-language-tree-sitter-requirements.md` provides a
comprehensive requirements document for multi-language support, including:

- Tier 1 languages: Python, TypeScript/JS, Go, C#
- Tier 2 languages: C, C++, Java, Kotlin, Swift, SQL, PowerShell (deferred)
- Dispatch architecture: `detect_language` + `parse_source` replacing `parse_rust_source`
- Unified symbol model: all languages map to `Function`, `Class`, `Interface`
- Key decisions D1-D4 already ratified

### Codebase Analysis

| Surface | Current State | Impact |
|---|---|---|
| `src/services/parsing.rs` | Rust-only, `parse_rust_source` | Replace with `parse_source(source, language)` dispatch |
| `src/services/code_graph.rs` | Calls `parse_rust_source` directly | Update to use `parse_source` |
| `src/services/dehydration.rs` | Writes to `.engram/code-graph/` | Change to `.engram/db/{branch}/code-graph/` |
| `src/services/hydration.rs` | Reads from `.engram/code-graph/` | Change to `{data_dir}/code-graph/` with fallback |
| `src/models/config.rs` | `supported_languages: vec!["rust"]` | Expand default to Tier 1 languages |
| `src/db/schema.rs` | SCHEMA_VERSION: "2.0.0" | No change (DB schema unchanged) |
| `src/services/dehydration.rs` | SCHEMA_VERSION: "3.0.0" | Bump to "4.0.0" (storage layout change) |

### Grammar Crate Availability

| Language | Crate | Latest Version |
|---|---|---|
| Rust | `tree-sitter-rust` | 0.23 (current) |
| Python | `tree-sitter-python` | 0.25.0 |
| TypeScript | `tree-sitter-typescript` | 0.23.2 |
| JavaScript | `tree-sitter-javascript` | 0.25.0 |
| Go | `tree-sitter-go` | 0.25.0 |
| C# | `tree-sitter-c-sharp` | 0.23.5 |

Version compatibility with `tree-sitter 0.24` core needs verification during
implementation. If newer grammar crates require `tree-sitter 0.25`, the core
dependency may need upgrading (low risk — tree-sitter has stable API evolution).

## Options Evaluated

### Option A: Sequential — Relocate First, Then Multi-Language

Implement 003-F (storage relocation + schema bump) as a standalone change, then
004-F (multi-language) builds on the new storage layout.

- **Pros:** Smaller incremental PRs, lower risk per change, 003-F is independently useful
- **Cons:** Two schema version bumps, two re-index cycles for users, more total effort
- **Effort:** Medium (two separate task trees)

### Option B: Combined — Single Cohesive Change

Implement both features together: relocate storage AND add multi-language support
in one coordinated release unit. One schema version bump, one re-index.

- **Pros:** Single schema bump, single re-index, coherent user experience, less total effort
- **Cons:** Larger change set, higher review complexity
- **Effort:** Medium-high (one larger task tree)

### Option C: Multi-Language Only — Skip Relocation

Implement 004-F without relocating the code-graph directory. Keep `.engram/code-graph/`
at root level.

- **Pros:** Smallest scope, fastest to deliver
- **Cons:** Leaves the storage misalignment unresolved, multi-language data still not branch-aware
- **Effort:** Medium

## Trade-off Comparison

| Criterion | Option A: Sequential | Option B: Combined | Option C: Skip Relocation |
|---|---|---|---|
| Complexity per PR | Low | Medium-high | Medium |
| Schema version bumps | 2 | 1 | 1 |
| User disruption | 2 re-indexes | 1 re-index | 1 re-index |
| Storage correctness | Full | Full | Partial (misaligned) |
| Total effort | Higher | Moderate | Lower |
| Risk | Lower per step | Moderate | Low |

## Decision

**Option B: Combined delivery** — implement storage relocation and multi-language
parsing as a single coordinated release unit.

Rationale:

- One schema version bump (3.0.0 → 4.0.0) instead of two
- Users re-index once, not twice
- The relocation is small (~20 lines changed across hydration/dehydration) and
  naturally integrates with the multi-language work
- Both features touch overlapping code surfaces — combining avoids two passes
  through the same files

## Rejected Alternatives

- **Option A** rejected because two sequential schema bumps create unnecessary
  user friction for a pre-1.0 product
- **Option C** rejected because shipping multi-language data to a non-branch-aware
  location perpetuates an architectural inconsistency

## Unresolved Questions

1. **Grammar crate compatibility:** Do `tree-sitter-python 0.25` and `tree-sitter-go
   0.25` require `tree-sitter 0.25` core? If so, the core dependency needs upgrading.
   Resolve during task 1 (dependency addition).

2. **TypeScript vs JavaScript grammars:** The research doc asks whether `.js/.jsx`
   should use the TypeScript grammar or separate JavaScript grammar. Decision:
   **use separate grammars** — TypeScript grammar may produce different AST node kinds
   for JavaScript features, and `tree-sitter-javascript` is lightweight.

3. **Migration from old path:** When `.engram/code-graph/` exists at the old location
   but not at the new `db/{branch}/code-graph/` location, hydration should check
   the old path as a fallback and log a migration warning. The old directory is NOT
   auto-deleted — users clean up manually or on next full index.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Grammar crate API incompatibility with tree-sitter 0.24 | Medium | High | Upgrade tree-sitter core if needed; grammar API is stable |
| Increased binary size from 5 grammar crates | Low | Low | Grammars are compile-time generated C code, ~1-3 MB each |
| Parsing correctness for non-Rust languages | Medium | Medium | Fixture-based tests per language, use well-known reference files |
| Old `.engram/code-graph/` path confusion | Low | Low | Fallback + warning log during hydration |
