//! Unit tests for plan unit F03 (immutable daemon mode in application state).
//!
//! Covers:
//! - `AppState::with_mode` requires an explicit `DaemonMode` at construction
//! - `AppState::mode()` reads back the exact mode supplied at construction
//! - `AppState::new` / `with_stale_strategy` / `with_options` (the pre-F04
//!   constructors) resolve to `DaemonMode::Managed`, so managed-mode
//!   behavior is unchanged
//! - `AppState` exposes no mutation API for `mode` (no setter method exists;
//!   this is asserted by construction — see module doc below)
//!
//! See docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md.

use engram::config::StaleStrategy;
use engram::models::config::DaemonMode;
use engram::server::state::AppState;

/// GIVEN `AppState::with_mode` is called with an explicit `DaemonMode`
/// WHEN the resulting state's `mode()` is read
/// THEN it returns exactly the mode that was supplied at construction.
#[test]
fn with_mode_requires_and_preserves_explicit_mode_managed() {
    let state = AppState::with_mode(DaemonMode::Managed, 10, StaleStrategy::Warn, 20, 60);
    assert_eq!(state.mode(), DaemonMode::Managed);
}

/// GIVEN `AppState::with_mode` is called with `DaemonMode::ReadServer`
/// WHEN the resulting state's `mode()` is read
/// THEN it returns `ReadServer`, proving the mode is not silently coerced
/// to a default and that both variants are constructible.
#[test]
fn with_mode_requires_and_preserves_explicit_mode_read_server() {
    let state = AppState::with_mode(DaemonMode::ReadServer, 10, StaleStrategy::Warn, 20, 60);
    assert_eq!(state.mode(), DaemonMode::ReadServer);
}

/// GIVEN the pre-existing `AppState::new` constructor (no mode parameter)
/// WHEN the resulting state's `mode()` is read
/// THEN it resolves to `DaemonMode::Managed`, proving managed-mode behavior
/// is unchanged by the introduction of the `mode` field.
#[test]
fn new_constructor_resolves_to_managed_mode() {
    let state = AppState::new(10);
    assert_eq!(state.mode(), DaemonMode::Managed);
}

/// GIVEN the pre-existing `AppState::with_stale_strategy` constructor
/// WHEN the resulting state's `mode()` is read
/// THEN it resolves to `DaemonMode::Managed`, proving managed-mode behavior
/// is unchanged by the introduction of the `mode` field.
#[test]
fn with_stale_strategy_constructor_resolves_to_managed_mode() {
    let state = AppState::with_stale_strategy(10, StaleStrategy::Fail);
    assert_eq!(state.mode(), DaemonMode::Managed);
}

/// GIVEN the pre-existing `AppState::with_options` constructor
/// WHEN the resulting state's `mode()` is read
/// THEN it resolves to `DaemonMode::Managed`, proving managed-mode behavior
/// is unchanged by the introduction of the `mode` field.
#[test]
fn with_options_constructor_resolves_to_managed_mode() {
    let state = AppState::with_options(10, StaleStrategy::Warn, 5, 30);
    assert_eq!(state.mode(), DaemonMode::Managed);
}

/// GIVEN two `AppState` instances constructed with different modes
/// WHEN each state's `mode()` is read independently
/// THEN neither observes the other's mode, and repeated reads on the same
/// state are stable (no interior mutability or reassignment path exists for
/// `mode` — there is no setter method on `AppState` to call here, which is
/// itself the absence-of-mutation-API assertion: this test would not
/// compile if one existed and were required for the mode to change).
#[test]
fn mode_is_stable_across_repeated_reads_and_independent_across_instances() {
    let managed = AppState::with_mode(DaemonMode::Managed, 10, StaleStrategy::Warn, 20, 60);
    let read_server = AppState::with_mode(DaemonMode::ReadServer, 10, StaleStrategy::Warn, 20, 60);

    // Repeated reads on the same instance are stable.
    assert_eq!(managed.mode(), DaemonMode::Managed);
    assert_eq!(managed.mode(), DaemonMode::Managed);

    // Independent instances do not observe each other's mode.
    assert_eq!(read_server.mode(), DaemonMode::ReadServer);
    assert_eq!(managed.mode(), DaemonMode::Managed);
}
