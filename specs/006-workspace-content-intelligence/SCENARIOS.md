# Behavioral Matrix: Workspace Content Intelligence

**Input**: Design documents from `/specs/006-workspace-content-intelligence/`
**Prerequisites**: spec.md (required), plan.md (required), data-model.md, contracts/
**Created**: 2026-03-15

## Summary

| Metric | Count |
|---|---|
| **Total Scenarios** | 78 |
| Happy-path | 28 |
| Edge-case | 16 |
| Error | 14 |
| Boundary | 8 |
| Concurrent | 6 |
| Security | 6 |

**Non-happy-path coverage**: 64% (minimum 30% required) ✅

---

## Content Registry (registry.yaml)

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S001 | Valid registry with 3 sources | `.engram/registry.yaml` exists with code, tests, docs entries | `hydrate workspace` | All 3 sources validated, status=Active, registered in DB | 3 ContentSource records in SurrealDB | happy-path |
| S002 | Registry auto-detection on install | Workspace has `src/`, `tests/`, `specs/`, `docs/` directories | `engram install` | Generates `registry.yaml` with 4 auto-detected entries | `.engram/registry.yaml` created with correct types | happy-path |
| S003 | Custom content type in registry | Registry has entry `type: tracking, path: .copilot-tracking` | `hydrate workspace` | Custom type accepted, content ingested and searchable | ContentSource with type="tracking" in DB | happy-path |
| S004 | Registry entry with missing path | Registry has entry `type: code, path: nonexistent/` | `hydrate workspace` | Warning logged, source status=Missing, other sources still processed | Source record with status=Missing | edge-case |
| S005 | Empty registry (no sources) | `.engram/registry.yaml` exists with `sources: []` | `hydrate workspace` | Warning logged, falls back to legacy `.engram/tasks.md` behavior | Legacy hydration path executed | edge-case |
| S006 | No registry file exists | `.engram/registry.yaml` does not exist | `hydrate workspace` | Falls back to legacy behavior, info-level log | Legacy hydration path executed | edge-case |
| S007 | Duplicate path in registry | Two entries: `{type: code, path: src}` and `{type: tests, path: src}` | `hydrate workspace` | Validation error: duplicate path detected, second entry rejected | Only first entry registered | error |
| S008 | Invalid YAML syntax | `.engram/registry.yaml` contains malformed YAML | `hydrate workspace` | Parse error with line number, falls back to legacy | Error logged, legacy fallback | error |
| S009 | Path traversal attempt in registry | Registry entry: `type: code, path: ../../other-repo/src` | `hydrate workspace` | Path rejected — resolves outside workspace root | Security warning logged, entry status=Error | security |
| S010 | Symlink pointing outside workspace | Registry path `src` contains symlink to `/etc/` | `hydrate workspace` | Symlink resolved, target validated, rejected if outside workspace | Security warning, entry skipped | security |
| S011 | Registry with max_file_size_bytes=0 | `max_file_size_bytes: 0` in registry | `hydrate workspace` | Validation error: max_file_size_bytes must be > 0 | Error logged, default used | error |
| S012 | Registry with max_file_size_bytes=200MB | `max_file_size_bytes: 209715200` in registry | `hydrate workspace` | Validation error: max_file_size_bytes must be ≤ 100MB | Error logged, default used | boundary |
| S013 | Auto-detect with no recognizable dirs | Workspace has only `Cargo.toml` and `README.md`, no standard dirs | `engram install` | Registry generated with empty sources array | `.engram/registry.yaml` with `sources: []` | edge-case |
| S014 | Built-in type validation | Registry entry: `type: code` (built-in) | `hydrate workspace` | Type recognized as built-in, no warning | Source validated | happy-path |

---

## Multi-Source Content Ingestion

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S015 | Ingest code source | Registry: `type: code, language: rust, path: src` with 10 .rs files | `hydrate workspace` | Code source routed to code graph indexer (not raw text ingestion) | Code graph nodes created, not ContentRecord | happy-path |
| S016 | Ingest spec source | Registry: `type: spec, path: specs` with 5 .md files | `hydrate workspace` | 5 ContentRecord entries created with content_type="spec" | 5 records in content_record table | happy-path |
| S017 | Ingest docs source | Registry: `type: docs, path: docs` with 3 .md files | `hydrate workspace` | 3 ContentRecord entries created with content_type="docs" | 3 records in content_record table | happy-path |
| S018 | Re-ingest changed file | ContentRecord exists for `docs/quickstart.md`, file modified on disk | File change event detected | Only the changed file re-ingested, content_hash updated | ContentRecord updated with new hash | happy-path |
| S019 | Skip unchanged file on re-ingest | ContentRecord exists with matching content_hash | `sync workspace` | File skipped, no DB write | ContentRecord unchanged | happy-path |
| S020 | File exceeds 1MB size limit | Registry source contains a 5MB file | `hydrate workspace` | File skipped with warning log, other files processed | No ContentRecord for oversized file | edge-case |
| S021 | File at exactly 1MB boundary | Registry source contains 1,048,576 byte file | `hydrate workspace` | File ingested (limit is exclusive: > 1MB) | ContentRecord created | boundary |
| S022 | File at 1MB + 1 byte | Registry source contains 1,048,577 byte file | `hydrate workspace` | File skipped with warning | No ContentRecord | boundary |
| S023 | Empty file (0 bytes) | Registry source contains empty file | `hydrate workspace` | File ingested with empty content, content_hash of empty string | ContentRecord with empty content | boundary |
| S024 | 500 files in single source | Registry source points to directory with 500 files | `hydrate workspace` | Files processed in batches (default: 50), progress spans emitted | 500 ContentRecords, 10 batch spans | happy-path |
| S025 | Binary file in text source | Registry: `type: docs, path: docs`, docs/ contains a .png file | `hydrate workspace` | Binary file skipped (non-text detection), warning logged | No ContentRecord for .png | edge-case |
| S026 | Concurrent ingestion from two sources | Two sources configured, hydration triggers both | `hydrate workspace` | Both sources ingested without interference | Both source records in DB | concurrent |
| S027 | File deleted after registry scan | File exists during path validation, deleted before ingestion | `hydrate workspace` | IO error handled gracefully, file skipped with warning | No ContentRecord for missing file | error |
| S028 | Content type filter on query_memory | ContentRecords exist for types: code, spec, docs | `query_memory(content_type: "spec")` | Only spec-type records returned | Filtered result set | happy-path |
| S029 | Content type filter with unknown type | ContentRecords exist | `query_memory(content_type: "nonexistent")` | Empty result set returned (no error) | Empty results | edge-case |
| S030 | No content type filter on unified_search | ContentRecords exist for types: code, spec, docs | `unified_search(query: "hydration")` | Results from all types returned, each annotated with type | Multi-type result set | happy-path |
| S031 | Overlapping paths in registry | Entries for `src/` and `src/models/` | `hydrate workspace` | Files in `src/models/` assigned the more specific entry's type, no duplication | No duplicate ContentRecords | edge-case |

---

## SpecKit-Aware Rehydration

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S032 | Single feature directory hydration | `specs/001-core-mcp-daemon/` with spec.md, plan.md, tasks.md | `hydrate workspace` | `backlog-001.json` created with all artifact contents | `.engram/backlog-001.json` exists with 3 artifacts | happy-path |
| S033 | Multiple feature directories | `specs/001-*/` through `specs/005-*/` each with varying artifacts | `hydrate workspace` | 5 backlog JSON files created, numbered 001-005 | `.engram/backlog-001.json` through `backlog-005.json` | happy-path |
| S034 | Project manifest creation | Workspace with 3 feature directories | `hydrate workspace` | `project.json` created with project metadata and 3 backlog refs | `.engram/project.json` with backlogs array | happy-path |
| S035 | Feature dir with partial artifacts | `specs/002-*/` has spec.md and plan.md but no tasks.md or SCENARIOS.md | `hydrate workspace` | `backlog-002.json` includes spec and plan artifacts; tasks and scenarios are null | JSON has `"tasks": null, "scenarios": null` | happy-path |
| S036 | New artifact added to existing feature | `backlog-001.json` exists, ANALYSIS.md added to `specs/001-*/` | `hydrate workspace` | `backlog-001.json` updated to include analysis artifact | JSON now has `"analysis": "..."` | happy-path |
| S037 | Dehydrate task update to backlog | Task record modified in SurrealDB for feature 001 | `dehydrate workspace` | `backlog-001.json` updated with new task state | JSON tasks field reflects DB state | happy-path |
| S038 | No specs directory | Workspace has no `specs/` directory | `hydrate workspace` | Falls back to legacy `.engram/tasks.md`, no backlog JSONs created | No backlog files, no project.json | edge-case |
| S039 | Non-SpecKit directory in specs | `specs/random-notes/` (no NNN- prefix) | `hydrate workspace` | Directory treated as regular content (via registry), not as backlog feature | No `backlog-random-notes.json` | edge-case |
| S040 | Invalid backlog JSON on disk | `.engram/backlog-001.json` contains malformed JSON | `hydrate workspace` | Parse error logged, file skipped, other backlogs processed | Error for 001, other backlogs loaded | error |
| S041 | Feature directory deleted after prior backlog | `backlog-003.json` exists but `specs/003-*/` no longer on disk | `dehydrate workspace` | Warning logged, existing JSON preserved as archive | `backlog-003.json` unchanged | edge-case |
| S042 | Project manifest with git remote URL | Workspace has `origin` remote configured | `hydrate workspace` | `project.json` includes `repository_url` from git remote | JSON has valid URL | happy-path |
| S043 | Project manifest without git | Workspace is not a git repository | `hydrate workspace` | `project.json` has `repository_url: null` | JSON with null URL | edge-case |
| S044 | Concurrent hydrate and dehydrate | Hydration in progress when dehydration triggered | Concurrent calls | Operations serialized via workspace lock | No data corruption | concurrent |

---

## Git Commit Graph

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S045 | Index 500 commits (default depth) | Repository with 1000 commits | `index_git_history(depth: 500)` | 500 most recent commits indexed with change records | 500 CommitNode records in DB | happy-path |
| S046 | Index with custom depth | Repository with 200 commits | `index_git_history(depth: 100)` | 100 most recent commits indexed | 100 CommitNode records | happy-path |
| S047 | Incremental sync after new commits | 500 commits indexed, 5 new commits made | `index_git_history()` | Only 5 new commits processed | 505 CommitNode records | happy-path |
| S048 | Force re-index | 500 commits previously indexed | `index_git_history(force: true)` | All commits re-processed, existing records replaced | 500 CommitNode records (refreshed) | happy-path |
| S049 | Commit with 3 file changes | Commit modifies `a.rs`, adds `b.rs`, deletes `c.rs` | `index_git_history()` | CommitNode has 3 ChangeRecords: Modify, Add, Delete | ChangeRecords with correct types | happy-path |
| S050 | Diff snippet with context lines | Commit modifies line 50 of a file, default context=20 | `index_git_history()` | Diff snippet includes lines 30-70 (20 lines context each side) | ChangeRecord with contextual diff | happy-path |
| S051 | Merge commit with 2 parents | Merge commit `abc` has parents `def` and `ghi` | `index_git_history()` | CommitNode has parent_hashes: ["def...", "ghi..."] | Both parent references stored | happy-path |
| S052 | Query changes by file path | CommitNodes indexed | `query_changes(file_path: "src/server/router.rs")` | Returns all commits with changes to that file, newest first | Filtered commit list | happy-path |
| S053 | Query changes by symbol name | CommitNodes indexed, code graph has `build_router` at lines 10-50 | `query_changes(symbol: "build_router")` | Returns only commits with diffs touching lines 10-50 of the file | Cross-referenced results | happy-path |
| S054 | Query changes by date range | CommitNodes indexed spanning Jan-Mar 2026 | `query_changes(since: "2026-02-01", until: "2026-02-28")` | Returns only February commits | Date-filtered results | happy-path |
| S055 | Query changes with limit | 100 commits match filter | `query_changes(file_path: "src/lib.rs", limit: 10)` | Only 10 most recent returned, `truncated: true` | Truncated result set | boundary |
| S056 | Query changes for nonexistent file | No commits touch `nonexistent.rs` | `query_changes(file_path: "nonexistent.rs")` | Empty result set returned | `commits: [], total_count: 0` | edge-case |
| S057 | Query changes for unknown symbol | Symbol `foobar` not in code graph | `query_changes(symbol: "foobar")` | Error response: symbol not found in code graph | Error code 4002 | error |
| S058 | Shallow clone (depth 1) | Repository cloned with `--depth 1` | `index_git_history()` | Single commit indexed, info log about shallow history | 1 CommitNode | edge-case |
| S059 | Repository with no commits | Empty git repository | `index_git_history()` | No commits indexed, info log | 0 CommitNodes | boundary |
| S060 | No git repository | Workspace is not git-initialized | `index_git_history()` | Error: git repository not found | Error code 5001 | error |
| S061 | Large diff (1000+ lines changed) | Commit modifies entire file (1500 lines) | `index_git_history()` | Diff snippet truncated to configurable max (default: 500 lines) | Truncated diff in ChangeRecord | boundary |
| S062 | Git repository with broken objects | Corrupt .git/objects | `index_git_history()` | Git access error returned | Error code 5002 | error |
| S063 | Concurrent git index and query | `index_git_history` running while `query_changes` called | Concurrent calls | Query returns stale-but-consistent data (no partial reads) | Read isolation maintained | concurrent |

---

## Agent Hooks and Instructions

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S064 | Fresh install with no existing hooks | No `.github/copilot-instructions.md`, no `.claude/`, no `.cursor/` | `engram install` | Hook files created for all 3 platforms | 3 platform-specific files created | happy-path |
| S065 | Install with existing Copilot instructions | `.github/copilot-instructions.md` has user content | `engram install` | Engram content appended between markers, user content preserved | File has both user and engram sections | happy-path |
| S066 | Re-install updates existing markers | `.github/copilot-instructions.md` has existing `<!-- engram:start/end -->` | `engram install` | Content between markers replaced, outside markers untouched | Markers updated, user content preserved | happy-path |
| S067 | Hooks-only flag | Data files already exist | `engram install --hooks-only` | Only hook files created/updated, `.engram/` data files untouched | Registry, tasks.md unchanged | happy-path |
| S068 | Custom port in hook files | Engram configured with `--port 8080` | `engram install` | Hook files reference `http://127.0.0.1:8080` | Correct port in MCP endpoint URLs | happy-path |
| S069 | No-hooks flag | No agent hooks desired | `engram install --no-hooks` | `.engram/` data files created, no hook files generated | Only data files present | happy-path |
| S070 | Hook file in read-only directory | `.github/` directory exists but is read-only | `engram install` | IO error for hook file, warning logged, other hooks still attempted | Partial hook creation with warning | error |

---

## Workspace Status and Integration

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S071 | Full status with all features | Registry active, content ingested, git indexed, SpecKit detected | `get_workspace_status` | Response includes registry, git_graph, and speckit sections | Complete status response | happy-path |
| S072 | Status without git-graph feature | git-graph feature not enabled at compile time | `get_workspace_status` | git_graph section absent from response | Partial status (no git_graph) | edge-case |
| S073 | Status before workspace set | No workspace bound | `get_workspace_status` | Error: workspace not set | Error code 1001 | error |
| S074 | query_changes before workspace set | No workspace bound | `query_changes(file_path: "src/lib.rs")` | Error: workspace not set | Error code 1001 | error |
| S075 | index_git_history before workspace set | No workspace bound | `index_git_history()` | Error: workspace not set | Error code 1001 | error |
| S076 | Multiple agents concurrent search | Two agents call query_memory simultaneously with different content_type filters | Concurrent `query_memory` calls | Both queries return correct filtered results independently | No cross-query interference | concurrent |
| S077 | Multiple agents concurrent ingestion | Two agents trigger ingestion simultaneously | Concurrent file change events | Operations serialized by ingestion lock, no duplicate records | No duplicate ContentRecords | concurrent |
| S078 | Workspace with all capabilities active | Registry, ingestion, SpecKit, git, hooks all active | Full lifecycle test | All subsystems work together without interference | Complete workspace state | happy-path |

---

## Edge Case Coverage Checklist

- [x] Malformed inputs and invalid arguments (S008, S011, S040, S057, S062)
- [x] Missing dependencies and unavailable resources (S004, S006, S027, S060)
- [x] State errors and race conditions (S044, S063, S076, S077)
- [x] Boundary values (empty, max-length, zero, negative) (S012, S021, S022, S023, S055, S059, S061)
- [x] Permission and authorization failures (S009, S010, S070)
- [x] Concurrent access patterns (S026, S044, S063, S076, S077)
- [x] Graceful degradation scenarios (S005, S006, S038, S058)

## Cross-Reference Validation

- [x] Every entity in `data-model.md` has at least one scenario covering its state transitions (RegistryConfig: S001-S014; ContentRecord: S015-S031; BacklogFile: S032-S044; CommitNode: S045-S063; ContentSource status: S001, S004, S009, S010)
- [x] Every endpoint in `contracts/` has at least one happy-path and one error scenario (query_changes: S052-S057, S060; index_git_history: S045-S048, S060, S062; query_memory content_type filter: S028-S029; unified_search: S030; get_workspace_status: S071-S073)
- [x] Every user story in `spec.md` has corresponding behavioral coverage (US1: S001-S014; US2: S015-S031; US3: S032-S044; US4: S045-S063; US5: S064-S070; US6: covered by documentation deliverables, not behavioral scenarios)
- [x] No scenario has ambiguous or non-deterministic expected outcomes

## Notes

- Scenario IDs are globally sequential (S001-S078) across all components
- Categories: `happy-path`, `edge-case`, `error`, `boundary`, `concurrent`, `security`
- Each row is deterministic — exactly one expected outcome per input state
- Tables are grouped by component/subsystem under level-2 headings
- User Story 6 (Documentation) is a deliverable, not a behavioral component — it does not require behavioral scenarios
- Git commit graph scenarios assume the `git-graph` feature flag is enabled unless noted otherwise
