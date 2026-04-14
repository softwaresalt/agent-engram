---
title: "Code Graph Infrastructure — Branch-Aware Storage + Multi-Language Parsing"
description: "Implementation plan for combined delivery of 003-F (storage relocation) and 004-F (multi-language tree-sitter parsing)"
source: "docs/decisions/2026-04-14-code-graph-infrastructure-deliberation.md"
status: "reviewed"
linked_artifacts:
  - "docs/decisions/2026-04-14-code-graph-infrastructure-deliberation.md"
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

Engram's code graph subsystem has two structural limitations that reduce its
value to non-Rust workspaces and introduce branch-switching inconsistencies:

1. **Storage misalignment (003-F):** Code graph JSONL files serialize to
   `.engram/code-graph/` (flat, workspace-global) while SurrealDB persists to
   `.engram/db/{branch}/` (branch-aware). Switching branches leaves stale JSONL
   data from the previous branch, causing hydration to load wrong-branch
   symbols into the new branch's database.

2. **Single-language parsing (004-F):** `parse_rust_source` in
   `src/services/parsing.rs` hard-codes `tree_sitter_rust::LANGUAGE`. Every
   non-Rust file is filtered out by `discover_files`, producing an empty graph
   for polyglot workspaces.

Both features touch the same subsystem — parsing, code graph orchestration,
dehydration, hydration — and share a schema version gate. Combined delivery
avoids two separate schema bumps and two re-index events.

### Code Paths Affected

| Module | Current Behavior | Required Change |
|---|---|---|
| `src/services/parsing.rs` | `parse_rust_source` only | `parse_source(source, Language)` dispatch to 6 parsers |
| `src/services/code_graph.rs:165` | Calls `parse_rust_source` directly | Call `parse_source` with detected language |
| `src/services/code_graph.rs:1079` | `language_from_path` maps rs/py/js/ts | Add go, cs mappings |
| `src/services/dehydration.rs:84` | `workspace_path.join(".engram").join("code-graph")` | `data_dir.join("code-graph").join(branch)` |
| `src/services/hydration.rs:132` | `path.join(".engram").join("code-graph")` | `data_dir.join("code-graph").join(branch)` + old-path fallback |
| `src/models/config.rs:159` | `default_supported_languages` → `["rust"]` | Add python, typescript, javascript, go, csharp |
| `src/tools/lifecycle.rs:82` | `hydrate_code_graph(&canonical, ...)` | Pass `data_dir` and `branch` |
| `src/tools/write.rs:62,68` | `hydrate/dehydrate_code_graph` with workspace_path | Pass `data_dir` and `branch` |
| `Cargo.toml:50-51` | tree-sitter 0.24, tree-sitter-rust 0.23 | Add 5 grammar crates |

## Requirements Trace

| Requirement (from deliberation/research) | Implementation Unit |
|---|---|
| R1: Language detection from file extension | Unit 1: Language enum |
| R2: Parse dispatch by detected language | Unit 1: parse_source dispatch |
| R3: Python function/class/import extraction | Unit 2: Python parser |
| R4: TypeScript class/function/interface extraction | Unit 3: TS/JS parsers |
| R5: JavaScript function/class/import extraction | Unit 3: TS/JS parsers |
| R6: Go func/struct/interface extraction | Unit 4: Go/C# parsers |
| R7: C# class/method/interface extraction | Unit 4: Go/C# parsers |
| R8: Unified symbol model (Function/Class/Interface) | Units 2-4: all map to existing ExtractedSymbol |
| R9: Branch-aware code-graph JSONL storage | Unit 5: Storage relocation |
| R10: Migration fallback from old flat path | Unit 5: Hydration fallback |
| R11: Schema version bump (3.0.0 → 4.0.0) | Unit 5: Schema bump |
| R12: Config default languages expanded | Unit 6: Config + orchestration |
| R13: Code graph orchestration uses parse_source | Unit 6: Orchestration update |
| R14: Tool layer passes data_dir/branch to hydration/dehydration | Unit 7: Tool layer |
| R15: Multi-language integration tests | Unit 8: Integration tests |
| R16: Branch-aware storage integration tests | Unit 8: Integration tests |

## Implementation Units

### Unit 1: Language Enum and Parse Dispatch Scaffold

**What:** Create a `Language` enum in `parsing.rs` with variants for each
Tier 1 language. Add `parse_source(source, language)` dispatch function that
routes to language-specific parsers. Rust routes to existing `parse_rust_source`.
Other languages return `ParseResult::empty()` (stubs) until their parsers are
implemented. Add grammar crate dependencies to `Cargo.toml`.

**Files affected:**
- `Cargo.toml` — add `tree-sitter-python`, `tree-sitter-typescript`,
  `tree-sitter-javascript`, `tree-sitter-go`, `tree-sitter-c-sharp`
- `src/services/parsing.rs` — `Language` enum, `Language::from_extension()`,
  `parse_source()` dispatch, `ParseResult::empty()` helper

**Tests:**
- Unit test: `parse_source` dispatches to Rust parser and returns valid result
- Unit test: `parse_source` with unsupported language returns empty result
- Unit test: `Language::from_extension` maps all expected extensions

**Execution posture:** Test-first. Write dispatch tests, verify they fail,
implement the enum and dispatch.

**Verification:** `cargo check` compiles with new dependencies. Unit tests pass.

**Risk:** Grammar crates at 0.25.x may require tree-sitter core 0.25. If so,
bump tree-sitter core first and verify tree-sitter-rust 0.23 remains compatible.
Resolve during dependency addition by checking `Cargo.toml` of each grammar
crate for their `tree-sitter` version requirement.

### Unit 2: Python Parser

**What:** Implement `parse_python_source` using `tree-sitter-python`. Map
Python AST node kinds to the existing `ExtractedSymbol` model:

| Python node kind | Maps to |
|---|---|
| `function_definition` | `ExtractedFunction` |
| `class_definition` | `ExtractedClass` |
| `import_statement` / `import_from_statement` | `ExtractedEdge::Imports` |
| `call` (inside function bodies) | `ExtractedEdge::Calls` |
| `decorated_definition` → inner function/class | Unwrap decorator, extract inner |

**Files affected:**
- `src/services/parsing.rs` — `parse_python_source` function, Python-specific
  `extract_top_level_python` and helpers

**Tests:**
- Unit test: extracts top-level function with name, lines, signature, docstring
- Unit test: extracts class with methods
- Unit test: extracts import edges
- Unit test: extracts call edges from function bodies

**Execution posture:** Test-first. Inline Python source fixtures as raw strings.

**Verification:** `cargo test --test unit_parsing` — Python tests pass.

### Unit 3: TypeScript and JavaScript Parsers

**What:** Implement `parse_typescript_source` and `parse_javascript_source`
using separate grammar crates. TypeScript and JavaScript share similar AST
structure but use different grammars (TypeScript includes type annotations).

| JS/TS node kind | Maps to |
|---|---|
| `function_declaration` | `ExtractedFunction` |
| `class_declaration` | `ExtractedClass` |
| `interface_declaration` (TS only) | `ExtractedInterface` |
| `method_definition` (inside class) | `ExtractedFunction` (qualified) |
| `import_statement` | `ExtractedEdge::Imports` |
| `call_expression` | `ExtractedEdge::Calls` |
| `arrow_function` (named via variable) | `ExtractedFunction` |
| `export_statement` → inner | Unwrap export, extract inner |

**Files affected:**
- `src/services/parsing.rs` — `parse_typescript_source`,
  `parse_javascript_source`, shared JS/TS extraction helpers

**Tests:**
- Unit test: TS class with interface implementation
- Unit test: JS function + arrow function extraction
- Unit test: TS import/export edges
- Unit test: JS call expression edges

**Execution posture:** Test-first. Separate test functions for TS and JS.

**Verification:** `cargo test --test unit_parsing` — TS/JS tests pass.

### Unit 4: Go and C# Parsers

**What:** Implement `parse_go_source` and `parse_csharp_source`.

| Go node kind | Maps to |
|---|---|
| `function_declaration` | `ExtractedFunction` |
| `method_declaration` | `ExtractedFunction` (qualified: `Type.method`) |
| `type_declaration` → `struct_type` | `ExtractedClass` |
| `type_declaration` → `interface_type` | `ExtractedInterface` |
| `import_declaration` | `ExtractedEdge::Imports` |
| `call_expression` | `ExtractedEdge::Calls` |

| C# node kind | Maps to |
|---|---|
| `method_declaration` | `ExtractedFunction` (qualified: `Class.method`) |
| `class_declaration` | `ExtractedClass` |
| `interface_declaration` | `ExtractedInterface` |
| `using_directive` | `ExtractedEdge::Imports` |
| `invocation_expression` | `ExtractedEdge::Calls` |
| `struct_declaration` | `ExtractedClass` |

**Files affected:**
- `src/services/parsing.rs` — `parse_go_source`, `parse_csharp_source`,
  language-specific helpers

**Tests:**
- Unit test: Go function + method + struct + interface extraction
- Unit test: Go import and call edges
- Unit test: C# class with methods + interface
- Unit test: C# using directives and invocation edges

**Execution posture:** Test-first.

**Verification:** `cargo test --test unit_parsing` — Go/C# tests pass.

### Unit 5: Branch-Aware Storage Relocation

**What:** Relocate code graph JSONL serialization from
`{workspace}/.engram/code-graph/` to `{data_dir}/code-graph/{branch}/`. Update
both dehydration (write) and hydration (read) paths. Add migration fallback:
hydration checks the new path first, falls back to the old flat path with a
tracing warning, and never auto-deletes the old directory.

Bump `SCHEMA_VERSION` from `"3.0.0"` to `"4.0.0"`. Update the version check in
`hydrate_workspace` to accept both `"3.0.0"` (old) and `"4.0.0"` (new) during
the transition, logging a migration info message for `"3.0.0"`.

**Signature changes:**

```rust
// dehydration.rs — add data_dir and branch parameters
pub async fn dehydrate_code_graph(
    cg_queries: &CodeGraphQueries,
    workspace_path: &Path,  // kept for .version file writes
    data_dir: &Path,        // NEW
    branch: &str,           // NEW
) -> Result<CodeGraphDehydrationResult, EngramError>

// hydration.rs — add data_dir and branch parameters
pub async fn hydrate_code_graph(
    path: &Path,            // workspace path (for old-path fallback + body re-derivation)
    data_dir: &Path,        // NEW
    branch: &str,           // NEW
    cg_queries: &CodeGraphQueries,
) -> Result<CodeGraphHydrationResult, EngramError>
```

**Path resolution:**
- New path: `{data_dir}/code-graph/{branch}/nodes.jsonl`
- Old path (fallback): `{workspace}/.engram/code-graph/nodes.jsonl`

**Files affected:**
- `src/services/dehydration.rs` — path construction, `SCHEMA_VERSION` bump,
  signature change
- `src/services/hydration.rs` — path construction with fallback, signature
  change, version check relaxation
- `src/services/dehydration.rs:63` — `SCHEMA_VERSION = "4.0.0"`

**Tests:**
- Unit test: dehydration writes to branch-aware path
- Unit test: hydration reads from branch-aware path
- Unit test: hydration falls back to old path when new path missing
- Unit test: version check accepts both 3.0.0 and 4.0.0

**Execution posture:** Test-first. Characterization test to capture current
behavior before changing paths.

**Verification:** `cargo test` — storage tests pass, no regressions.

### Unit 6: Code Graph Orchestration and Config

**What:** Update `code_graph.rs` to call `parse_source` instead of
`parse_rust_source`. Expand `language_from_path` with Go and C# extension
mappings. Update `default_supported_languages` in config to include all Tier 1
languages.

**Changes in code_graph.rs:**
- Line ~165: Replace `parse_rust_source(&source_clone)` with
  `parse_source(&source_clone, language)` where `language` is derived from
  `Language::from_extension` using the file's extension
- Line ~620 (sync_workspace): Same replacement
- Line ~1079 (`language_from_path`): Add `"go" => "go"`, `"cs" => "csharp"`

**Changes in config.rs:**
- `default_supported_languages` returns
  `vec!["rust", "python", "typescript", "javascript", "go", "csharp"]`

**Files affected:**
- `src/services/code_graph.rs` — parse call replacement, `language_from_path`
  expansion
- `src/models/config.rs` — `default_supported_languages` update

**Tests:**
- Integration test: index workspace with mixed-language files, verify all
  languages produce symbols
- Integration test: `discover_files` includes Python/TS/JS/Go/C# files
- Unit test: `language_from_path` returns correct identifiers for all extensions

**Execution posture:** Test-first for `language_from_path`. Integration test
for parse dispatch wiring.

**Verification:** `cargo test --test integration_code_graph` — multi-language
indexing works.

### Unit 7: Tool Layer Updates

**What:** Update all callers of `hydrate_code_graph` and `dehydrate_code_graph`
to pass the new `data_dir` and `branch` parameters. These callers are in the
tool layer:

- `src/tools/lifecycle.rs:82` — `set_workspace` calls `hydrate_code_graph`
- `src/tools/write.rs:62` — `flush_state` calls `hydrate_code_graph`
- `src/tools/write.rs:68` — `flush_state` calls `dehydrate_code_graph`

All three already have `data_dir` and `branch` in scope from their workspace
snapshot, so the change is mechanical: pass the existing variables through.

**Files affected:**
- `src/tools/lifecycle.rs` — add `data_dir`, `branch` args to
  `hydrate_code_graph` call
- `src/tools/write.rs` — add `data_dir`, `branch` args to both
  `hydrate_code_graph` and `dehydrate_code_graph` calls

**Tests:**
- Contract test: `flush_state` writes to branch-aware path
- Contract test: `set_workspace` hydrates from branch-aware path

**Execution posture:** Implementation after Unit 5 (storage relocation)
provides the new signatures.

**Verification:** `cargo test` — all contract and integration tests pass.

### Unit 8: Integration and Smoke Tests

**What:** Add end-to-end integration tests that exercise the full
multi-language + branch-aware storage pipeline. Update existing tests that
reference the old `.engram/code-graph/` path.

**New tests:**
- Integration test: index workspace with Python + Rust files, verify both
  produce symbols and edges in DB
- Integration test: dehydrate to branch-aware path, delete DB, rehydrate from
  branch-aware path, verify symbols restored
- Integration test: old-path migration — write JSONL to old flat path, hydrate
  with new code, verify fallback reads from old path

**Existing test updates:**
- `tests/integration/smoke_test.rs` — update path assertions from
  `.engram/code-graph/` to branch-aware path
- `tests/integration/graph_vector_rehydration_test.rs` — update path
  construction for new directory structure
- Any contract tests that assert on `files_written` paths in dehydration results

**Files affected:**
- `tests/integration/code_graph_test.rs` — new multi-language tests
- `tests/integration/smoke_test.rs` — path assertion updates
- `tests/integration/graph_vector_rehydration_test.rs` — path updates
- `Cargo.toml` — `[[test]]` blocks for any new test files

**Execution posture:** Write new tests after all implementation units are
complete. Update existing tests alongside their corresponding units.

**Verification:** Full `cargo test` — zero failures.

## Dependency Graph

```text
Unit 1: Language enum + dispatch scaffold
  │
  ├──→ Unit 2: Python parser
  │
  ├──→ Unit 3: TS/JS parsers
  │
  ├──→ Unit 4: Go/C# parsers
  │
  └──→ Unit 5: Branch-aware storage relocation
         │
         │   Units 2, 3, 4 ──→ Unit 6: Orchestration + config
         │                       │
         └───────────────────────┤
                                 │
                          Unit 7: Tool layer updates
                                 │
                          Unit 8: Integration tests
```

**Parallel opportunities:**
- Units 2, 3, 4 are independent of each other and of Unit 5 — all four can
  execute in parallel after Unit 1
- Unit 6 requires Units 2-4 (parsers exist for dispatch) and Unit 1
- Unit 7 requires Units 5 and 6
- Unit 8 requires Unit 7

## Decisions and Rationale

### D1: Branch-aware path structure `{data_dir}/code-graph/{branch}/`

**Choice:** Place branch-aware JSONL under `{data_dir}/code-graph/{branch}/`
rather than inside `{data_dir}/db/{branch}/code-graph/`.

**Rationale:** The `db/{branch}/` directory is managed by SurrealDB's internal
file layout. Placing our JSONL files inside it risks conflicts with SurrealDB
upgrades or cleanup. A parallel `code-graph/{branch}/` hierarchy keeps our
serialized data separate while achieving branch awareness.

### D2: Signature change over path computation inside functions

**Choice:** Add `data_dir` and `branch` parameters to `hydrate_code_graph` and
`dehydrate_code_graph` rather than having them compute the path internally.

**Rationale:** The callers already have `data_dir` and `branch` from their
workspace snapshot. Passing them explicitly avoids duplicating resolution logic
and makes the functions testable with arbitrary paths.

### D3: Migration fallback without auto-deletion

**Choice:** Hydration checks the new branch-aware path first. If missing, it
falls back to the old `.engram/code-graph/` path and logs a warning. The old
directory is never auto-deleted.

**Rationale:** Auto-deletion is destructive and hard to undo if something goes
wrong. Users can manually clean up old directories. The fallback ensures
existing workspaces continue to work after upgrading without requiring an
immediate re-index.

### D4: Schema version accepts both 3.0.0 and 4.0.0 during transition

**Choice:** The version check in `hydrate_workspace` accepts `"3.0.0"` (old
format, flat path) and `"4.0.0"` (new format, branch-aware path).

**Rationale:** Hard-failing on `"3.0.0"` would force every existing workspace
to re-index immediately on upgrade. Accepting both versions with a log message
provides a graceful migration path.

### D5: Separate parsers per language (no shared extraction)

**Choice:** Each language gets its own `parse_{language}_source` function with
language-specific node kind mappings, rather than a generic extractor
parameterized by node kind strings.

**Rationale:** Tree-sitter ASTs vary significantly across languages. Python
uses `function_definition` / `class_definition`, TypeScript uses
`function_declaration` / `class_declaration`, Go uses `function_declaration` /
`type_declaration`. A parameterized approach would need so many language-specific
branches that it would be less readable than separate functions. The common
output type (`ParseResult` with `ExtractedSymbol` + `ExtractedEdge`) provides
the unification point.

### D6: Grammar crate compatibility strategy

**Choice:** Pin grammar crates to versions compatible with tree-sitter 0.24
(current). If a grammar requires 0.25, evaluate upgrading tree-sitter core and
verify tree-sitter-rust 0.23 compatibility first.

**Rationale:** tree-sitter core upgrades can introduce breaking API changes.
Preferring compatible versions avoids cascading changes. The fallback is a
controlled core upgrade verified against the existing Rust parser.

## Risks and Caveats

### R1: Grammar crate API compatibility (Medium)

**Risk:** Some grammar crates (tree-sitter-python 0.25.0, tree-sitter-go
0.25.0, tree-sitter-javascript 0.25.0) may require tree-sitter core 0.25.

**Mitigation:** Resolve during Unit 1 dependency addition. Check each crate's
`Cargo.toml` for tree-sitter version requirements. If 0.25 is needed, upgrade
core and re-verify tree-sitter-rust compatibility. Prior versions of the grammar
crates (0.23.x or 0.24.x) may be available as fallbacks.

### R2: Tree-sitter node kind accuracy (Low)

**Risk:** The node kind mappings per language are based on documentation and
prior research. Actual tree-sitter grammars may use different or additional node
kinds for some constructs.

**Mitigation:** Each parser unit has test-first verification with real source
fixtures. Node kinds are validated empirically, not assumed from documentation.

### R3: Large workspace performance (Low)

**Risk:** Adding 5 more languages increases the number of files discovered and
parsed during indexing.

**Mitigation:** The existing `parse_concurrency` config and `spawn_blocking`
pattern already handle parallel parsing. Performance impact is proportional to
the number of source files, which is bounded by the workspace size. No
architectural change needed.

### R4: Branch directory proliferation (Low)

**Risk:** Each branch creates a `code-graph/{branch}/` directory with JSONL
files, potentially accumulating stale branch data.

**Mitigation:** This is identical to SurrealDB's existing `db/{branch}/`
pattern, which has the same proliferation characteristic. A future cleanup
mechanism can prune stale branches. Not blocking for this release.

### R5: Existing test breakage from path changes (Medium)

**Risk:** Tests that assert on `.engram/code-graph/` paths (smoke tests,
rehydration tests) will fail after storage relocation.

**Mitigation:** Unit 8 explicitly updates all affected tests. Test updates are
tracked as part of the plan, not left as incidental cleanup.

## Plan Hardening Signals

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | **Yes** | Schema version bump 3.0.0 → 4.0.0; hydrate/dehydrate signature changes |
| Security, auth, permission, or compliance-sensitive | No | No auth or permission changes |
| Migration, backfill, destructive data/config action | **Yes** | Storage path migration with fallback; version acceptance window |
| External integration, operator checkpoint, or external dependency | **Yes** | 5 new tree-sitter grammar crate dependencies |
| High runtime, rollout, or rollback risk | No | Local daemon, no deployment; rollback = revert + re-index |

**Requires plan hardening: yes**

The schema change, migration path, and new external dependencies warrant
hardening review. Hardening should focus on: migration rollback procedure,
grammar crate version pinning strategy, and branch-directory cleanup
expectations.

## Runtime Verification and Closure

### Changed Runtime Surfaces

| Unit | Runtime Surface | Verification |
|---|---|---|
| Units 2-4 | Code graph indexing now parses 5 additional languages | Index a polyglot workspace; verify symbols from each language appear in `list_symbols` and `map_code` results |
| Unit 5 | JSONL storage path moved to branch-aware location | After `flush_state`, verify files exist at `{data_dir}/code-graph/{branch}/`; verify old-path fallback on upgrade |
| Unit 6 | `discover_files` includes new file types | Index workspace with mixed files; verify `files_parsed` count matches expectations |
| Unit 7 | Tool layer hydration/dehydration uses new paths | `set_workspace` → `get_workspace_status` → verify code_graph stats are non-zero for polyglot workspace |

### Operational Closure Expectations

- **Monitoring:** `get_workspace_status` code_graph stats should report
  non-zero values for polyglot workspaces. `get_health_report` should not show
  elevated error rates after the change.
- **Rollback trigger:** If `hydrate_code_graph` fails with path errors after
  upgrade, the fallback to old path should activate. If fallback also fails,
  rollback is: revert code + delete `.engram/` and re-index.
- **Rollback procedure:** Git revert of the merge commit. Users may need to
  delete `.engram/code-graph/{branch}/` directories created by the new code.
  Old `.engram/code-graph/` flat directory (if still present) will be usable
  by the reverted code.
- **Validation window:** After merge, index 2-3 real polyglot workspaces and
  verify correct symbol extraction per language.
- **Owner:** Ship agent during post-merge closure.

## Constitution Check

| Principle | Compliance | Notes |
|---|---|---|
| I. Safety-First Rust | ✅ | No unsafe code. All grammar crates use safe Rust bindings |
| II. Test-First Development | ✅ | Every unit specifies test-first execution posture |
| III. Workspace Isolation | ✅ | All paths resolve within workspace root or data_dir |
| IV. CLI Containment | ✅ | No files created outside workspace |
| V. Structured Observability | ✅ | Migration fallback logs tracing warnings |
| VI. Single Responsibility | ✅ | 5 new deps justified by concrete parsing requirements |
| VII. Destructive Approval | N/A | No destructive operations |
| VIII. Safety Modes | N/A | Schema migration is non-destructive (additive) |
| IX. Git-Friendly Persistence | ✅ | JSONL files remain human-readable and Git-mergeable |
| X. Context Efficiency | ✅ | No impact on MCP tool response format |

## Plan Hardening

### Hardening Required: Yes

Three hardening signals are present: schema/contract change, migration path,
and new external dependencies. This section deepens verification, rollback, and
guardrail detail for those risk surfaces.

### Learnings and Instructions Consulted

| Source | Finding |
|---|---|
| `docs/compound/test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md` | Test helpers creating `TempDir` must return it alongside `AppState` to prevent premature directory deletion. All new test helpers in Units 2-8 must follow this pattern. |
| engram memory: TASK-010.04 (branch-aware metrics persistence) | Prior branch-aware directory pattern used `{data_dir}/metrics/{branch}/`. Validates the `{data_dir}/code-graph/{branch}/` approach. Branch switching sends a message to swap file handles — code graph dehydration is less continuous (on-demand), so no handle-swap needed. |
| `src/installer/mod.rs:39-70` (`detect_version_mismatch`) | The installer reads `SCHEMA_VERSION` from `dehydration.rs` and compares against `.engram/.version`. After bumping to `"4.0.0"`, `detect_version_mismatch` will report `Mismatch` for all existing `"3.0.0"` workspaces. This is informational (update/reinstall path), not a hard failure. No installer code changes needed. |
| `src/services/hydration.rs:69` (version check) | Current check is `version != SCHEMA_VERSION` — strict equality. Must relax to accept `"3.0.0"` during migration window. |
| ADR 0002: Static Flush Lock | `dehydrate_code_graph` is serialized via `FLUSH_LOCK` in `tools/write.rs`. New signature parameters do not affect lock semantics. |

### Risk Triggers and Protected Invariants

| Risk Trigger | Protected Invariant | Guardrail |
|---|---|---|
| Schema version bump 3.0.0 → 4.0.0 | Existing workspaces must not hard-fail on upgrade | Version check accepts both `"3.0.0"` and `"4.0.0"` |
| Storage path relocation | JSONL data from old workspaces must remain accessible | Hydration fallback reads old flat path when branch-aware path is empty |
| Installer version detection | `detect_version_mismatch` must report mismatch but not error | Installer uses `SCHEMA_VERSION` from dehydration.rs; `Mismatch` is informational |
| `.version` file write location | `.version` stays at `{workspace}/.engram/.version` (not branch-specific) | Dehydration writes `.version` to workspace root `.engram/`, not to `code-graph/{branch}/` |
| New grammar crate dependencies | Build must compile with tree-sitter 0.24 core | Pin grammar crates to 0.24-compatible versions; upgrade core only if no compatible grammar version exists |
| `TempDir` lifetime in tests | Test workspace must survive through assertions | Every test helper returns `TempDir` alongside `AppState` (compound learning) |

### Proposed Actions (Strict-Safety Classification)

#### PA-1: Schema Version Bump

- **Summary:** Change `SCHEMA_VERSION` in `dehydration.rs` from `"3.0.0"` to `"4.0.0"` and update `.version` file writes
- **Targets:** `src/services/dehydration.rs:63`, `src/services/hydration.rs:69`, `src/installer/mod.rs` (read-only impact)
- **Change kind:** Contract change (on-disk format version)
- **Rollback:** Revert the constant. Workspaces that wrote `"4.0.0"` to `.version` will need manual `.version` file edit or full re-install to reset
- **Approval required:** No (non-destructive, additive)
- **ActionRisk:** moderate
- **ActionResult:** planned

#### PA-2: Storage Path Relocation

- **Summary:** Move JSONL write path from `{workspace}/.engram/code-graph/` to `{data_dir}/code-graph/{branch}/`; add fallback read path
- **Targets:** `src/services/dehydration.rs:84`, `src/services/hydration.rs:132`
- **Change kind:** Migration (file location change with fallback)
- **Rollback:** Revert code. Old `.engram/code-graph/` directory is preserved (never auto-deleted). New `code-graph/{branch}/` directories can be manually deleted.
- **Approval required:** No (old data preserved, fallback prevents data loss)
- **ActionRisk:** moderate
- **ActionResult:** planned

#### PA-3: Add 5 Grammar Crate Dependencies

- **Summary:** Add `tree-sitter-python`, `tree-sitter-typescript`, `tree-sitter-javascript`, `tree-sitter-go`, `tree-sitter-c-sharp` to `Cargo.toml`
- **Targets:** `Cargo.toml`
- **Change kind:** External dependency addition
- **Rollback:** Remove crate lines from `Cargo.toml`, delete `parse_{language}_source` functions, revert `parse_source` dispatch to Rust-only
- **Approval required:** No (standard crates.io crates, no native code beyond tree-sitter C bindings already present)
- **ActionRisk:** low
- **ActionResult:** planned

#### PA-4: Hydrate/Dehydrate Signature Changes

- **Summary:** Add `data_dir: &Path` and `branch: &str` parameters to `hydrate_code_graph` and `dehydrate_code_graph`
- **Targets:** `src/services/hydration.rs:128`, `src/services/dehydration.rs:80`, all callers in `src/tools/lifecycle.rs` and `src/tools/write.rs`
- **Change kind:** Internal API change (not public MCP API)
- **Rollback:** Revert signatures and callers
- **Approval required:** No (internal refactor)
- **ActionRisk:** low
- **ActionResult:** planned

### Deepened Verification Detail

#### Migration Verification (Unit 5)

Test the following migration scenarios explicitly:

1. **Fresh workspace (no old data):** `hydrate_code_graph` with empty
   `data_dir/code-graph/{branch}/` and no old `.engram/code-graph/`. Verify
   result is empty, no errors.

2. **Old workspace (3.0.0 data, no branch-aware data):** Write JSONL to old
   flat path. Call `hydrate_code_graph` with new signature. Verify fallback
   reads from old path, logs `tracing::warn!`, and returns correct node/edge
   counts.

3. **Upgraded workspace (4.0.0 data exists):** Write JSONL to branch-aware
   path. Call `hydrate_code_graph`. Verify it reads from branch-aware path and
   does NOT read from old flat path.

4. **Mixed workspace (both paths exist):** Write different data to both paths.
   Verify branch-aware path takes precedence. Old path is NOT read.

5. **Version file transition:** Write `.version` containing `"3.0.0"`. Verify
   `hydrate_workspace` accepts it with a log message. Verify
   `dehydrate_code_graph` writes `.version` as `"4.0.0"` after next flush.

6. **Installer compatibility:** After version bump, verify
   `detect_version_mismatch` returns `Mismatch` for old workspaces and
   `UpToDate` for new workspaces. No crash or error.

#### Grammar Compatibility Verification (Unit 1)

Before implementing parsers, resolve the compatibility question:

1. Add each grammar crate one at a time to `Cargo.toml`
2. Run `cargo check` after each addition
3. If a grammar crate requires tree-sitter 0.25, check if an older version
   (0.23.x or 0.24.x) exists on crates.io
4. If no compatible version exists, upgrade tree-sitter core to 0.25 and verify
   `tree-sitter-rust 0.23` still compiles
5. Document the resolved version matrix in the Unit 1 commit message

#### TempDir Lifetime Verification (All Test Units)

Apply the compound learning from `tempdir-lifetime-in-contract-tests`:

- Every test helper that creates a `TempDir` must return it alongside other
  return values
- Caller binds as `let (_workspace, state) = setup_helper().await;`
- Never use bare `_` for `TempDir` bindings (immediate drop)

### Deepened Operational Closure Detail

#### Monitoring Signals

| Signal | Source | Healthy | Degraded |
|---|---|---|---|
| Files parsed per language | `index_workspace` result `files_parsed` | > 0 for each language present in workspace | 0 for a language that has source files |
| Hydration fallback activation | `tracing::warn` log line | Not emitted (branch-aware path used) | Emitted (old path fallback active — workspace needs re-flush) |
| Schema mismatch warnings | `tracing::info` log line in version check | Not emitted after first flush | Persistent emission = `.version` not being updated |
| Code graph query errors | `get_health_report` error_count | 0 | > 0 after indexing polyglot workspace |

#### Rollback Triggers

| Trigger | Metric | Threshold | Action |
|---|---|---|---|
| Hydration hard failure | `hydrate_code_graph` returns `Err` | Any occurrence after upgrade | Investigate path resolution; if systematic, revert merge |
| Symbol count regression for Rust | `get_workspace_status` `functions` count | < previous count for same workspace | Verify `parse_source` Rust arm matches `parse_rust_source` output |
| Build time regression | CI build duration | > 2x baseline | Investigate grammar crate compilation cost; consider making languages feature-gated |

#### Rollback Procedure

1. `git revert <merge-commit>` on the release branch
2. Run `cargo test` to verify revert compiles and passes
3. For workspaces that already wrote `"4.0.0"` to `.version`:
   - Option A: Delete `.engram/` entirely and re-index (clean slate)
   - Option B: Manually edit `.engram/.version` to `"3.0.0"` and delete
     `code-graph/{branch}/` directories
4. Old `.engram/code-graph/` flat directory is preserved by design — reverted
   code will read it without issues

#### Validation Window

- **Duration:** 48 hours after merge
- **Owner:** Ship agent during post-merge closure
- **Targets:** Index 3 real workspaces: one Rust-only (regression check), one
  polyglot (Python + TS), one with multiple branches (storage path validation)
- **Success criteria:** Symbol counts match expectations per language, no
  hydration errors, no schema mismatch hard failures

### Unresolved Operator Decisions

None. All decisions were resolved during deliberation. The grammar crate
version compatibility question is resolved empirically during Unit 1
(dependency addition) without requiring operator input.

## Plan Review

**Gate Decision: ADVISORY**

**Reviewed by:** 6 personas (Constitution Reviewer, Rust Reviewer, Scope
Boundary Auditor, Learnings Researcher, Architecture Strategist,
Agent-Native Parity Reviewer). Cross-model diversity: 4 personas on
caller model, 2 personas on claude-sonnet-4.

**Plan hardening required:** Yes — hardening signals present (schema change,
migration path, new external dependencies). Plan Hardening section is present
and materially complete. Strict-safety `ProposedAction` / `ActionRisk`
entries are classified for all 4 risky actions. **Requirement satisfied.**

### Raw Finding Counts (Pre-Merge)

| Persona | P0 | P1 | P2 | P3 | Total |
|---|---|---|---|---|---|
| Constitution Reviewer | 1 | 2 | 3 | 2 | 8 |
| Rust Reviewer | 0 | 4 | 9 | 1 | 14 |
| Scope Boundary Auditor | 0 | 4 | 5 | 0 | 9 |
| Learnings Researcher | 0 | 1 | 2 | 0 | 3 |
| Architecture Strategist | 1 | 1 | 2 | 2 | 6 |
| Agent-Native Parity | 0 | 1 | 2 | 1 | 4 |
| **Pre-merge total** | **2** | **13** | **23** | **6** | **44** |

### Severity Normalization and Deduplication

After cross-referencing findings against the existing codebase and
deduplicating overlapping concerns, all P0 and P1 findings were
reclassified:

**P0 → P3 (Constitution Reviewer: Principle VII label):** The plan
correctly avoids destructive auto-deletion of old data. The Constitution
Check table labels Principle VII as "N/A" when "✅ Addressed" is more
accurate. This is a documentation label fix, not a plan defect.

**P0 → P3 (Architecture Strategist: dependency direction):** The codebase
already passes `data_dir` and `branch` from tool layer to `connect_db()`
(db/mod.rs:38) and metrics (metrics.rs:37). Adding the same parameters to
hydration/dehydration follows the established convention, not a violation.

**P1 → P2 (Constitution: Units 3/4 width isolation):** TS+JS share similar
AST structures and are commonly co-located. Go+C# follow identical parser
patterns. Two languages per unit is a pragmatic grouping that stays within
the 2-hour estimate.

**P1 → P2 (Constitution: Unit 8 new+update tests):** Updating existing
test path assertions is a natural part of the integration testing unit,
not a separate concern requiring its own task.

**P1 → P2 (Rust: ParseResult::empty()):** Stubs are temporary — replaced
in Units 2-4. The concern about parse failure visibility is valid but is a
moderate gap (P2), not a high-impact one.

**P1 → P2 (Rust: spawn_blocking documentation):** The existing pattern in
code_graph.rs:165 demonstrates the correct approach. Documenting the
constraint for new parsers is advisory.

**P1 → P3 (Rust: branch name normalization):** Existing code uses
`.join(branch)` without normalization (db/mod.rs:38, metrics.rs:37). The
plan follows the established convention. Normalization, if needed, is a
codebase-wide concern outside this plan's scope.

**P1 → P3 (Rust: test fixtures in parsing.rs):** Existing tests use inline
raw strings in parsing.rs (lines 480-653). Following the established pattern
is correct.

**P1 → P2 (Scope: missing requirements / no scope boundary):** Valid scope
clarity gap — the plan should explicitly state what is deferred from the
research document. Not a plan defect.

**P1 → P3 (Scope: grammar compilation cost):** Already addressed in
rollback triggers section.

**P1 → P2 (Scope: Unit 8 test coverage ambiguity):** The "any contract
tests" phrasing should be more specific.

**P1 → P2 (Learnings: ADR 0012 not consulted):** Documentation gap —
parallel parsing deferral should be referenced.

**P1 → P2 (Agent-Native Parity: language discovery):** Adding supported
languages to workspace status is a follow-on improvement, not a defect in
this plan.

### Merged Findings (Post-Deduplication)

#### P2 Findings

**F-01: Add explicit scope boundary section** (Scope Boundary Auditor)

The plan links to the research document (30 requirements) but does not
state which requirements are in scope and which are deferred. Requirements
13, 14, and 17 from the research (CodeFile language field, language
filtering in MCP tools, excluded_languages config) have no corresponding
implementation unit.

*Recommendation:* Add a "Scope Boundary" subsection after Problem Frame
listing in-scope and out-of-scope items. Deferred requirements should
reference a follow-on feature ID.

**F-02: Replace ParseResult::empty() with error variant for unsupported
languages** (Rust Reviewer + Agent-Native Parity Reviewer)

`ParseResult::empty()` silently returns nothing for unsupported languages.
Agents cannot distinguish "no symbols found" from "language not supported"
or "parse failed." The existing `CodeGraphError::UnsupportedLanguage`
variant (errors/mod.rs) is available.

*Recommendation:* During Unit 1 implementation, use
`Err(EngramError::CodeGraph(...))` for unsupported languages instead of
`Ok(ParseResult::empty())`. Callers in code_graph.rs should handle the
error with context (skip file, log warning). This does not change the plan
structure — it refines the implementation approach within Unit 1.

**F-03: Document spawn_blocking constraint for new parsers** (Rust Reviewer)

New parser functions in Units 2-4 must be synchronous (`pub fn`, not
`async fn`) for `tokio::task::spawn_blocking`. The plan does not explicitly
state this constraint for each parser unit.

*Recommendation:* Add to each parser unit's execution posture: "Function
MUST be `pub fn` (synchronous, CPU-bound). Callers use
`tokio::task::spawn_blocking`."

**F-04: Use TryFrom for Language enum** (Rust Reviewer + Architecture
Strategist)

`Language::from_extension()` returning a String is less idiomatic than
`impl TryFrom<&str> for Language`. The Architecture Strategist suggested
trait-based dispatch, but this is YAGNI for the current scope — enum
dispatch is appropriate for 6 languages.

*Recommendation:* During Unit 1 implementation, prefer `TryFrom<&str>` over
a custom `from_extension` method. This is an implementation detail, not a
plan structure change.

**F-05: Strengthen test-first posture language in Units 6-7** (Constitution
Reviewer)

Units 6 and 7 use weaker test-first language than Units 1-5. Unit 7 says
"Implementation after Unit 5" without explicitly stating "test-first."

*Recommendation:* Update execution posture in Units 6-7 to explicitly
state: "Test-first. Write tests, verify they fail, implement, verify they
pass."

**F-06: Enumerate affected tests in Unit 8** (Constitution Reviewer + Scope
Boundary Auditor)

Unit 8 says "Any contract tests that assert on `files_written` paths"
without listing specific tests. This creates surprise regression risk.

*Recommendation:* Before implementation, run
`grep -r "code-graph" tests/` to enumerate all affected test files.
List them explicitly in Unit 8.

**F-07: Add ADR 0012 to consulted learnings** (Learnings Researcher)

The plan maintains sequential parsing via `spawn_blocking` (consistent with
ADR 0012's deferral of parallel parsing), but does not reference this prior
decision.

*Recommendation:* Add ADR 0012 to the "Learnings and Instructions
Consulted" table.

**F-08: Add deliberation document to consulted learnings** (Learnings
Researcher)

The deliberation document appears in frontmatter but not in the "Learnings
and Instructions Consulted" table.

*Recommendation:* Add the deliberation document to the table.

**F-09: Consider parsing module directory for maintainability**
(Architecture Strategist)

Adding 5 language parsers to a single `parsing.rs` file may grow it
significantly. A `src/services/parsing/` module directory with per-language
files would improve maintainability.

*Recommendation:* Evaluate during Unit 1 whether the file size warrants
splitting into a module directory. This is an implementation-time decision,
not a plan structure change.

**F-10: Document version acceptance window timeline** (Rust Reviewer +
Architecture Strategist)

The dual-version acceptance (3.0.0 + 4.0.0) has no deprecation timeline.
The plan does not specify when the 3.0.0 fallback is removed.

*Recommendation:* Add a deprecation note: "3.0.0 acceptance is transitional.
Remove fallback in the next major feature release. Dehydration always writes
4.0.0."

**F-11: Consider adding supported languages to workspace status**
(Agent-Native Parity Reviewer)

After this change, agents have no runtime discovery mechanism for which
languages the daemon supports.

*Recommendation:* Defer to a follow-on task. Add a field to
`get_workspace_status` response listing configured languages.

**F-12: Verify flush lock interaction with new parameters** (Learnings
Researcher)

The `FLUSH_LOCK` serializes dehydration writes. The plan adds `data_dir`
and `branch` parameters but does not specify whether resolution happens
inside or outside the lock.

*Recommendation:* During Unit 5 implementation, document whether parameter
resolution is inside or outside the critical section.

#### P3 Findings

**F-13: Constitution Check Principle VII label** — Change "N/A" to
"✅ Addressed — Plan avoids destructive auto-deletion."

**F-14: Dependency direction is established convention** — Tools passing
`data_dir`/`branch` to services follows db/mod.rs:38 and metrics.rs:37
patterns. No action needed.

**F-15: Branch name normalization** — Existing code uses `.join(branch)`
without normalization. Consistent with established convention.

**F-16: Test fixture organization** — Inline raw strings follow existing
parsing.rs test pattern. Consider fixtures directory for maintainability
if file grows beyond 1000 lines.

**F-17: Grammar compilation cost mitigation** — Already addressed in
rollback triggers with "consider making languages feature-gated."

### Runtime Verification and Operational Closure

The plan includes adequate runtime verification:
- ✅ Migration verification scenarios (5 test cases in hardening)
- ✅ Grammar compatibility verification protocol
- ✅ Monitoring signals (4 signals with healthy/degraded indicators)
- ✅ Rollback triggers (3 triggers with metrics and thresholds)
- ✅ Rollback procedure (4-step process)
- ✅ Validation window (48 hours, owner identified)

No gaps identified in verification or closure readiness.

### Gate Rationale

All original P0 and P1 findings were reclassified to P2 or P3 after
cross-referencing against the existing codebase. The plan follows
established codebase conventions (branch path handling, parameter passing
patterns, inline test fixtures). The 12 P2 findings are implementation
refinements and documentation improvements — none represent structural
plan defects that would cause significant rework. The 5 P3 findings are
advisory.

**Decision: ADVISORY — User decides whether to revise or proceed to
harvest.**
