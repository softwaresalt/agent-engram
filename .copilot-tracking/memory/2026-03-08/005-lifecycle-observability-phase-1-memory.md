# Session Memory: 005-lifecycle-observability Phase 1

**Date**: 2026-03-08
**Spec**: 005-lifecycle-observability
**Phase**: 1 — Setup
**Status**: COMPLETE

---

## Task Overview

Phase 1 establishes all scaffolding required for the Lifecycle Observability feature:
- New optional dependencies (OTLP trace export behind `otlp-export` feature flag)
- Empty module files for new capabilities
- Error code constants and error type variants for all new error domains
- Configuration parameters for ledger, rollback, query, and OTLP settings

## Current State

### Tasks Completed (6/6)

| Task | Description | Status |
|------|-------------|--------|
| T001 | Add tracing-opentelemetry, opentelemetry, opentelemetry_sdk, opentelemetry-otlp to Cargo.toml behind `otlp-export` feature | ✅ DONE |
| T002 | Create src/models/event.rs, src/models/collection.rs, src/services/gate.rs, src/services/event_ledger.rs, src/server/observability.rs | ✅ DONE |
| T003 | Register new modules in models/mod.rs, services/mod.rs, server/mod.rs | ✅ DONE |
| T004 | Add error codes: TASK_BLOCKED (3015), ROLLBACK_DENIED (3020), EVENT_NOT_FOUND (3021), ROLLBACK_CONFLICT (3022), COLLECTION_EXISTS (3030), COLLECTION_NOT_FOUND (3031), CYCLIC_COLLECTION (3032), QUERY_REJECTED (4010), QUERY_TIMEOUT (4011), QUERY_INVALID (4012) | ✅ DONE |
| T005 | Add EventError, CollectionError, GraphQueryError sub-error enums and EngramError variants; add TaskError::Blocked | ✅ DONE |
| T006 | Add Config fields: event_ledger_max (500), allow_agent_rollback (false), query_timeout_ms (50), query_row_limit (1000), otlp_endpoint (Option<String>) | ✅ DONE |

### Files Modified

- `Cargo.toml` — added 4 OTLP optional deps, `otlp-export` feature flag, 9 new [[test]] entries
- `src/models/event.rs` — NEW: Event struct, EventKind enum with 10 variants
- `src/models/collection.rs` — NEW: Collection struct (id, name, description, timestamps)
- `src/models/mod.rs` — added `event` and `collection` module declarations and re-exports
- `src/services/gate.rs` — NEW: placeholder stub with doc comments
- `src/services/event_ledger.rs` — NEW: placeholder stub with doc comments
- `src/services/mod.rs` — added `event_ledger` and `gate` modules
- `src/server/observability.rs` — NEW: placeholder stub with doc comments
- `src/server/mod.rs` — added `observability` module (unconditional, no feature gate)
- `src/errors/codes.rs` — added 10 new error code constants
- `src/errors/mod.rs` — added EventError (3 variants), CollectionError (3 variants), GraphQueryError (3 variants), TaskError::Blocked; full to_response() coverage
- `src/config/mod.rs` — added 5 new Config fields with ENGRAM_ env var support
- `specs/005-lifecycle-observability/tasks.md` — marked T001-T006 as [X]
- 9 test stub files created in tests/contract/, tests/integration/, tests/unit/

### Test Results

- `cargo check` — ✅ PASS (exit 0)
- `cargo fmt --all -- --check` — ✅ PASS (exit 0)
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — ✅ PASS (exit 0)
- `cargo test --test unit_proptest` — ✅ PASS: 15 passed, 0 failed
- `cargo test --test unit_proptest` (full suite) — ✅ PASS (exit 0)

## Important Discoveries

### Code Conflict: Error Code Numbers
- T004 specified TASK_BLOCKED (3010) and CYCLIC_DEPENDENCY (3011), but:
  - 3010 was already `INVALID_ISSUE_TYPE`
  - 3011 was already `DUPLICATE_LABEL`
  - 3003 was already `CYCLIC_DEPENDENCY`
- **Resolution**: Used TASK_BLOCKED = 3015 (next available after 3014). CYCLIC_DEPENDENCY already existed at 3003 — no duplicate added. All other new codes (3020–3032, 4010–4012) had no conflicts.

### OTLP Dependency Resolution
- Specified versions: tracing-opentelemetry = "0.26", opentelemetry = "0.26", opentelemetry_sdk = "0.26", opentelemetry-otlp = "0.26"
- Locked versions: tracing-opentelemetry 0.26.0, opentelemetry 0.26.0, opentelemetry_sdk 0.26.0, opentelemetry-otlp 0.26.0
- Feature grpc-tonic on opentelemetry-otlp pulls in tonic 0.12.3, prost 0.13.5, h2 0.4.13 — adds ~16 locked packages
- Build time impact: significant (~2 min) on first compile due to tonic/protobuf

### server/observability.rs Module Registration
- The spec plan listed observability.rs under `server/` without a feature gate
- Registered unconditionally (not behind `legacy-sse`) since OTLP is behind its own `otlp-export` flag

## Next Steps

**Phase 2 — Foundational**:
- T007: Implement full Event model with SurrealDB-ready types, timestamps, and serde attributes
- T008: Implement full Collection model
- T009-T011: SurrealDB schema for event, collection, contains tables in src/db/schema.rs
- T012: Register new schemas in ensure_schema function
- T013: proptest round-trip tests for Event and Collection serialization

**Known Issues / Open Questions**:
- Event model currently uses `chrono::DateTime<Utc>` — need to verify SurrealDB 2 datetime compatibility pattern (prior memory: use `<datetime>` cast in SurrealQL)
- Collection `contains` relation may need RELATE semantics — check db/queries.rs patterns from Phase 004

## Context to Preserve

- Error codes file: `src/errors/codes.rs` (reference for next available codes: 3015 used, next would be 3016)
- Existing CYCLIC_DEPENDENCY at 3003 — do NOT add another one
- OTLP dependencies locked at 0.26.x — keep consistent when adding tracing-opentelemetry layer in Phase 4
- Config struct in `src/config/mod.rs` — all new fields use `ENGRAM_` prefix, default values set per spec
- Test stub files are empty — they'll be populated in Phases 2–8 per TDD workflow
