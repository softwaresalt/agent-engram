//! Contract test for plan unit F04 (mode constructor migration, 142.009-T).
//!
//! F04 removed `AppState::new`, `AppState::with_stale_strategy`, and
//! `AppState::with_options`, leaving [`AppState::with_mode`] as the sole
//! constructor and the sole owner of the `mode` field. This test is the
//! standing guard against a default-mode escape hatch being reintroduced.
//!
//! Two complementary assertions:
//!
//! 1. **Source scan** — every top-level `impl AppState { … }` block found
//!    anywhere under `src/` (not just `src/server/state.rs`) declares no
//!    `new` / `with_stale_strategy` / `with_options` constructor, and the
//!    union of all such blocks exposes exactly one `-> Self` associated
//!    function. `AppState` already has a second inherent `impl` block in
//!    `src/daemon/lifecycle_policy.rs`, so a single-file scan would let a
//!    convenience constructor added to *that* block (or any future one)
//!    bypass this guard entirely. A reintroduced convenience constructor
//!    would still *compile*, so a compile-only assertion cannot catch it;
//!    the text scan can — as long as it is not scoped to one file.
//! 2. **Behavioral round-trip** — constructing through `with_mode` preserves
//!    the exact mode supplied, for both `DaemonMode` variants, proving the
//!    surviving constructor applies no default of its own.
//!
//! See docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md.

use engram::config::StaleStrategy;
use engram::models::config::DaemonMode;
use engram::server::state::AppState;

/// Recursively collect the contents of every `.rs` file under `dir`.
///
/// Runtime filesystem walk (rather than a hardcoded `include_str!` file
/// list) so a new file that declares `impl AppState { … }` is picked up
/// automatically instead of silently escaping this guard.
fn read_all_rust_sources(dir: &std::path::Path) -> Vec<(std::path::PathBuf, String)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(read_all_rust_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                out.push((path, content));
            }
        }
    }
    out
}

/// Every `src/` `.rs` file, read fresh at test time.
fn crate_sources() -> Vec<(std::path::PathBuf, String)> {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    read_all_rust_sources(&src_dir)
}

/// Extract the bodies of every top-level `impl AppState { … }` block found in
/// `content`.
///
/// A block ends at the first line that is exactly `}` at column zero, which
/// is the rustfmt-guaranteed closing brace of a top-level `impl`. Byte offsets
/// are tracked explicitly while iterating lines (rather than re-`find`ing the
/// terminating `"}"` line as a substring) because `"}"` commonly recurs much
/// earlier inside a block's own body, which would otherwise resolve to the
/// wrong offset and desynchronize the next block's search start.
fn app_state_impl_blocks(content: &str) -> Vec<String> {
    const HEADER: &str = "impl AppState {";
    let mut blocks = Vec::new();
    let mut search_from = 0usize;

    while let Some(rel_start) = content[search_from..]
        .find(&format!("\n{HEADER}\n"))
        .or_else(|| content[search_from..].find(&format!("\r\n{HEADER}\r\n")))
    {
        let header_pos = search_from + rel_start;
        let body_start = header_pos
            + content[header_pos..]
                .find(HEADER)
                .expect("header offset must resolve")
            + HEADER.len();

        let mut body = String::new();
        let mut cursor = body_start;
        let mut terminated_at = None;
        for line in content[body_start..].lines() {
            let line_end = cursor + line.len();
            if line == "}" {
                terminated_at = Some(line_end);
                break;
            }
            body.push_str(line);
            body.push('\n');
            // Advance past this line and its line terminator. `lines()` does
            // not report the terminator width itself, so re-derive it from
            // the source: prefer `\r\n`, fall back to `\n`.
            cursor = if content[line_end..].starts_with("\r\n") {
                line_end + 2
            } else {
                line_end + 1
            };
        }

        blocks.push(body);
        match terminated_at {
            Some(pos) => search_from = pos,
            None => break, // unterminated block: nothing further to scan
        }
        if search_from >= content.len() {
            break;
        }
    }

    blocks
}

/// Every `impl AppState { … }` block across the whole crate, concatenated.
///
/// GIVEN there are (at least) two files that declare an inherent
/// `impl AppState` block today (`src/server/state.rs` and
/// `src/daemon/lifecycle_policy.rs`), the constructor and `-> Self` guards
/// below must see the union of both, not either file in isolation.
fn all_app_state_impl_blocks() -> Vec<String> {
    let mut blocks = Vec::new();
    for (_, content) in crate_sources() {
        blocks.extend(app_state_impl_blocks(&content));
    }
    assert!(
        !blocks.is_empty(),
        "no `impl AppState {{` block found anywhere under src/ — the scan itself is broken"
    );
    blocks
}

/// GIVEN F04 removed the convenience constructors
/// WHEN every `impl AppState` block across the crate is scanned for their
/// declarations
/// THEN none of `new`, `with_stale_strategy`, or `with_options` is declared
/// in any of them.
///
/// This test goes RED if any of them is reintroduced in *any* `impl AppState`
/// block — verified negatively by temporarily re-adding `pub fn new(…)`
/// during F04 implementation, and re-verified against
/// `src/daemon/lifecycle_policy.rs`'s block specifically for this finding.
#[test]
fn app_state_declares_no_convenience_constructor() {
    for block in all_app_state_impl_blocks() {
        for removed in ["fn new(", "fn with_stale_strategy(", "fn with_options("] {
            assert!(
                !block.contains(removed),
                "an `impl AppState` block must not declare `{removed}…)`: F04 \
                 removed every convenience constructor so no call site can omit \
                 an explicit `DaemonMode`. Reintroducing one anywhere in the \
                 crate is a default-mode escape hatch.\nblock:\n{block}"
            );
        }
    }
}

/// GIVEN `with_mode` is intended to be the sole constructor
/// WHEN every `impl AppState` block across the crate is scanned for
/// associated functions that return `Self`
/// THEN exactly one exists in total, and it is `with_mode`.
///
/// This catches a differently-named convenience constructor (e.g.
/// `AppState::managed(…)`) added to *any* `impl AppState` block — not just
/// `src/server/state.rs` — that the name-based scan above would miss.
#[test]
fn with_mode_is_the_only_self_returning_constructor() {
    let blocks = all_app_state_impl_blocks();

    let with_mode_present = blocks
        .iter()
        .any(|block| block.contains("pub fn with_mode("));
    assert!(
        with_mode_present,
        "some `impl AppState` block must still declare `pub fn with_mode(`; \
         the source scan is anchored on it and would otherwise pass vacuously"
    );

    let self_returning: usize = blocks
        .iter()
        .map(|block| block.matches(") -> Self {").count())
        .sum();
    assert_eq!(
        self_returning, 1,
        "the union of all `impl AppState` blocks across the crate must expose \
         exactly one `-> Self` constructor (`with_mode`), found \
         {self_returning}. An additional one would be a second construction \
         path able to supply a default mode."
    );
}

/// GIVEN a `Default` impl would construct `AppState` without naming a mode
/// WHEN every `.rs` file under `src/` is scanned for one
/// THEN no `impl Default for AppState` exists anywhere in the crate.
#[test]
fn app_state_has_no_default_impl() {
    for (path, content) in crate_sources() {
        assert!(
            !content.contains("impl Default for AppState"),
            "`AppState` must not implement `Default` (found reference in \
             {path}): it would construct state without an explicit \
             `DaemonMode`, which is exactly the escape hatch F04 removed.",
            path = path.display()
        );
    }
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
