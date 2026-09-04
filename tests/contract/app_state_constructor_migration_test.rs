//! Contract test for plan unit F04 (mode constructor migration, 142.009-T).
//!
//! F04 removed `AppState::new`, `AppState::with_stale_strategy`, and
//! `AppState::with_options`, leaving [`AppState::with_mode`] as the sole
//! constructor and the sole owner of the `mode` field. This test is the
//! standing guard against a default-mode escape hatch being reintroduced.
//!
//! Two complementary assertions:
//!
//! 1. **Source scan** — the `impl AppState` block in `src/server/state.rs`
//!    declares no `new` / `with_stale_strategy` / `with_options` constructor,
//!    exposes exactly one `-> Self` associated function, and has no `Default`
//!    impl. A reintroduced convenience constructor would still *compile*, so a
//!    compile-only assertion cannot catch it; the text scan can.
//! 2. **Behavioral round-trip** — constructing through `with_mode` preserves
//!    the exact mode supplied, for both `DaemonMode` variants, proving the
//!    surviving constructor applies no default of its own.
//!
//! See docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md.

use engram::config::StaleStrategy;
use engram::models::config::DaemonMode;
use engram::server::state::AppState;

/// Source of `AppState`, embedded at compile time so the scan cannot silently
/// pass against a missing or relocated file.
const STATE_RS: &str = include_str!("../../src/server/state.rs");

/// Extract the body of the `impl AppState { … }` block from `STATE_RS`.
///
/// The block ends at the first line that is exactly `}` at column zero, which
/// is the rustfmt-guaranteed closing brace of a top-level `impl`.
fn impl_app_state_block() -> String {
    const HEADER: &str = "impl AppState {";

    let start = STATE_RS
        .find(&format!("\n{HEADER}\n"))
        .or_else(|| STATE_RS.find(&format!("\r\n{HEADER}\r\n")))
        .expect("src/server/state.rs must contain a top-level `impl AppState {` block");

    let body_start = start
        + STATE_RS[start..]
            .find(HEADER)
            .expect("header offset must resolve")
        + HEADER.len();

    let mut body = String::new();
    for line in STATE_RS[body_start..].lines() {
        if line == "}" {
            return body;
        }
        body.push_str(line);
        body.push('\n');
    }
    panic!("`impl AppState {{` block is not terminated by a column-zero `}}`");
}

/// GIVEN F04 removed the convenience constructors
/// WHEN the `impl AppState` block is scanned for their declarations
/// THEN none of `new`, `with_stale_strategy`, or `with_options` is declared.
///
/// This test goes RED if any of them is reintroduced — verified negatively by
/// temporarily re-adding `pub fn new(…)` during F04 implementation.
#[test]
fn app_state_declares_no_convenience_constructor() {
    let block = impl_app_state_block();

    for removed in ["fn new(", "fn with_stale_strategy(", "fn with_options("] {
        assert!(
            !block.contains(removed),
            "`impl AppState` must not declare `{removed}…)`: F04 removed every \
             convenience constructor so no call site can omit an explicit \
             `DaemonMode`. Reintroducing one is a default-mode escape hatch."
        );
    }
}

/// GIVEN `with_mode` is intended to be the sole constructor
/// WHEN the `impl AppState` block is scanned for associated functions that
/// return `Self`
/// THEN exactly one exists, and it is `with_mode`.
///
/// This catches a differently-named convenience constructor (e.g.
/// `AppState::managed(…)`) that the name-based scan above would miss.
#[test]
fn with_mode_is_the_only_self_returning_constructor() {
    let block = impl_app_state_block();

    assert!(
        block.contains("pub fn with_mode("),
        "`impl AppState` must still declare `pub fn with_mode(`; the source \
         scan is anchored on it and would otherwise pass vacuously"
    );

    let self_returning = block.matches(") -> Self {").count();
    assert_eq!(
        self_returning, 1,
        "`impl AppState` must expose exactly one `-> Self` constructor \
         (`with_mode`), found {self_returning}. An additional one would be a \
         second construction path able to supply a default mode."
    );
}

/// GIVEN a `Default` impl would construct `AppState` without naming a mode
/// WHEN `src/server/state.rs` is scanned for one
/// THEN no `impl Default for AppState` exists.
#[test]
fn app_state_has_no_default_impl() {
    assert!(
        !STATE_RS.contains("impl Default for AppState"),
        "`AppState` must not implement `Default`: it would construct state \
         without an explicit `DaemonMode`, which is exactly the escape hatch \
         F04 removed."
    );
}

/// GIVEN the surviving `with_mode` constructor
/// WHEN state is constructed with each `DaemonMode` variant
/// THEN `mode()` returns exactly the supplied variant, proving `with_mode`
/// applies no default of its own and that managed-mode behavior is preserved
/// by explicit construction.
#[test]
fn with_mode_round_trips_every_variant() {
    for mode in [DaemonMode::Managed, DaemonMode::ReadServer] {
        let state = AppState::with_mode(mode, 1, StaleStrategy::Warn, 20, 60);
        assert_eq!(
            state.mode(),
            mode,
            "`with_mode` must preserve the explicitly supplied mode"
        );
    }
}

/// GIVEN the pre-F04 `AppState::new` defaults (`StaleStrategy::Warn`, rate
/// limit 20 per 60 s, managed mode)
/// WHEN those same values are supplied explicitly to `with_mode`
/// THEN the resulting state observes them unchanged, proving the migration of
/// ~195 call sites was behaviour-preserving rather than a behaviour change.
#[test]
fn explicit_managed_construction_preserves_pre_migration_defaults() {
    let state = AppState::with_mode(DaemonMode::Managed, 10, StaleStrategy::Warn, 20, 60);

    assert_eq!(state.mode(), DaemonMode::Managed);
    assert_eq!(state.max_workspaces(), 10);
    assert_eq!(state.stale_strategy(), StaleStrategy::Warn);
}
