---
session: 026-F execution — 002-S shipment close
date: 2026-04-15
branch: 026-code-graph-infrastructure
commit: 8f2c0d9
---

# Session Memory — 026-F: Code Graph Infrastructure

## Tasks Completed

All 8 tasks of 026-F shipped and archived. 002-S shipment closed at commit 8f2c0d9.

| Task | Title | Status |
|------|-------|--------|
| 026.001-T | Language enum and parse dispatch scaffold | done |
| 026.002-T | Python parser implementation | done |
| 026.003-T | TypeScript and JavaScript parser implementation | done |
| 026.004-T | Go and C# parser implementation | done |
| 026.005-T | Branch-aware storage relocation and schema bump | done |
| 026.006-T | Code graph orchestration and config update | done |
| 026.007-T | Tool layer updates for storage parameters | done |
| 026.008-T | Integration and smoke tests | done |

## Files Modified

- `Cargo.toml` — grammar crates pinned to 0.23.x; new deps added
- `Cargo.lock` — updated for 0.23.x resolution
- `src/services/parsing.rs` — Language enum, TryFrom, parse_source dispatcher
- `src/services/parsing/rust.rs` — Rust parser (existing, refactored to submodule)
- `src/services/parsing/python.rs` — Python parser (new)
- `src/services/parsing/typescript.rs` — TypeScript parser (new)
- `src/services/parsing/javascript.rs` — JavaScript parser (new)
- `src/services/parsing/go_lang.rs` — Go parser (new)
- `src/services/parsing/csharp.rs` — C# parser (new)
- `src/services/hydration.rs` — branch-aware path, accepts schema 3.0.0 + 4.0.0
- `src/services/dehydration.rs` — SCHEMA_VERSION 4.0.0, branch-aware path
- `src/services/code_graph.rs` — parse_source dispatch, language_from_path go/cs
- `src/models/config.rs` — default_supported_languages() all 6 languages
- `src/tools/lifecycle.rs` — hydrate_code_graph call updated with data_dir/branch
- `src/tools/write.rs` — hydrate + dehydrate calls updated
- `src/errors/codes.rs` + `src/errors/mod.rs` — ParseFailed { reason } variant, code 7008
- `tests/contract/lifecycle_test.rs` — use canonicalize_workspace for path comparison
- `tests/integration/graph_vector_rehydration_test.rs` — nodes.jsonl paths with .join("main")

## Key Decisions and Findings

### Grammar ABI Compatibility

`tree-sitter v0.24.x` runtime accepts grammar ABI 13-14 only.
Grammar crates v0.23.x emit ABI 14 ✓. Grammar crates v0.24+/v0.25+ emit ABI 15 ✗.
Error only manifests at runtime: `cargo check`/`clippy` pass with ABI 15 grammars.
Fix: pin all grammar crates to `"0.23"` in Cargo.toml.

### Windows Path Normalization

`std::fs::canonicalize` on Windows returns `\\?\C:\...` (extended-length prefix).
`canonicalize_workspace` (src/db/workspace.rs) strips this prefix via `normalize_canonical`.
Test fix: use `engram::db::workspace::canonicalize_workspace(&path)` in tests to get
the same normalized path the server returns. Do NOT use `std::fs::canonicalize` directly
in path comparison tests.

### Flaky Timing Test

`t020_s001_s005_daemon_becomes_healthy_within_2_seconds` in `contract_shim_lifecycle`
failed once during full `cargo test` (took 11.82s vs 10s debug threshold) due to system
load from parallel test execution. Passes in ~1.33s when run in isolation. Pre-existing
flaky test; not related to this feature.

## Quality Gate Results

- [x] `cargo fmt --all -- --check` — PASSED
- [x] `cargo clippy -- -D warnings -D clippy::pedantic` — PASSED
- [x] `cargo test --test unit_parsing` — PASSED (15/15)
- [x] `cargo test --test integration_smoke` — PASSED (7/7)
- [x] `cargo test --test integration_graph_vector_rehydration` — PASSED (3/3)
- [x] `cargo test --test contract_lifecycle` — PASSED (8/8, after path fix)
- [~] `cargo test` (full suite) — PASSED with 1 pre-existing flaky timing failure

## Next Steps

- Create PR: `026-code-graph-infrastructure` → `main`
- 024-F (atomic policy snapshot) is next in the queue (024.001-T is ready)
