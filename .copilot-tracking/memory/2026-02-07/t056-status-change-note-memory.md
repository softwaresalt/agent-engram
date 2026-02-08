<!-- markdownlint-disable-file -->
# Memory: T056 Status Change Note Implementation

**Created:** 2026-02-07 | **Last Updated:** 2026-02-07

## Task Overview
Complete Phase 4 (User Story 2) by implementing T056: automatic context note creation on every task update, per FR-015. The system MUST append context notes on task updates (never overwrite existing context).

## Current State
- **T056 complete**: `create_status_change_note` service function in `src/services/connection.rs` wired into `update_task` in `src/tools/write.rs`.
- **Phase 4 fully complete**: All tasks T041–T056 marked done in `specs/001-core-mcp-daemon/tasks.md`.
- All source files pass VS Code language server checks with zero errors.
- Pre-existing `cargo check` failure in `ort-sys` (transitive via `fastembed` crate) due to missing TLS feature flag — unrelated to Phase 4 work.

### Files Modified
- `src/tools/write.rs` — Replaced manual context note creation block in `update_task` with call to `create_status_change_note` service; added import for `crate::services::connection::create_status_change_note`.
- `specs/001-core-mcp-daemon/tasks.md` — Marked T056 checkbox as complete.

### Files Previously Modified (earlier sessions, unchanged this session)
- `src/services/connection.rs` — Already contained `create_status_change_note` function from prior session.

## Important Discoveries

### Decisions
- **Always-fire context note:** `update_task` now creates a context note on every invocation, not just when user supplies explicit `notes`. This satisfies FR-015 requirement that context notes always append on task updates.
- **Status transition recording:** The note content includes `"Status changed from {previous} to {new}"` with optional user notes appended below, providing an audit trail of all status transitions.
- **Return type change:** `update_task` response `context_id` field changed from `Option<String>` to `String` since a context note is now always created.

### Reasoning
- Reviewed `src/services/connection.rs` and found `create_status_change_note` already implemented (from prior session) but not yet wired into `update_task`.
- Confirmed `Uuid` and `Context` imports still needed in `write.rs` for `add_blocker` and `register_decision` — no dead import cleanup needed.
- Verified the change via `get_errors` on both modified files (zero errors) since `cargo check` has a pre-existing unrelated failure.

### Failed Approaches
- None. The service function was already implemented; only wiring was needed.

### Pre-existing Issues Noted
- `fastembed = "3"` in `Cargo.toml` pulls in `ort-sys` which requires a TLS feature (`tls-rustls`, `tls-native`, etc.). This blocks full `cargo check`/`cargo test` and will need resolution before Phase 6 (US4: Semantic Memory) work.

## Next Steps
1. Fix `fastembed` TLS feature flag in `Cargo.toml` to unblock `cargo check` and `cargo test`.
2. Begin Phase 5 (User Story 3 — Git-Backed Persistence): T057–T070.
3. Run full test suite once TLS issue resolved to validate all Phase 4 contract tests pass end-to-end.

## Context to Preserve
* **Sources:** Task checklist [specs/001-core-mcp-daemon/tasks.md](specs/001-core-mcp-daemon/tasks.md); service function [src/services/connection.rs](src/services/connection.rs#L64-L97); wired call site [src/tools/write.rs](src/tools/write.rs#L95-L105).
* **Agents:** `memory.agent.md` (this session).
* **Questions:** How should `fastembed` TLS feature be configured — `tls-rustls` or `tls-native`? Needs decision before Phase 6.
