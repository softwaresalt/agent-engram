//! Append-only event ledger with rolling retention and rollback support.
//!
//! Records all state changes as immutable [`Event`] entries in SurrealDB.
//! Enforces a configurable maximum ledger size (`event_ledger_max`) by
//! pruning the oldest events when the limit is exceeded.
//!
//! See `specs/005-lifecycle-observability/spec.md` User Story 3 for requirements.
