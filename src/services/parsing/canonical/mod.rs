//! Canonical-identity infrastructure for Rust qualified/method call resolution
//! (Option C, feature 091-F, Unit A — **precision-neutral**).
//!
//! This module builds the *substrate* for resolving a Rust qualified or method
//! call to a single workspace-global canonical identity:
//!
//! - [`module_path`] (A1) — deterministic per-file module tree + the workspace
//!   crate-name set (workspace-vs-external classification).
//! - [`use_graph`] (A2) — full per-file `use` graph (groups, globs, `as`
//!   aliases, `self`/`super`/`crate` roots, `pub use` re-export flags).
//! - [`resolver`] (A3/A4) — a pure, fail-closed resolver from a path expression
//!   to a single canonical identity, with bounded `pub use` re-export closure.
//! - [`generics`] (A5) — generic-parameter normalisation so definition and call
//!   spellings converge on one canonical form.
//!
//! It is **precision-neutral**: it produces identity *data* only and emits no
//! call edges (qualified/method calls remain dropped in `code_graph`). Edge
//! enablement is gated to Unit B (088-S).
//!
//! Design invariant (013-D, kept absolute): every derivation is *fail-closed* —
//! any ambiguity, external crate, glob, macro, or non-derivable mapping yields
//! `None` rather than a guess, so a wrong identity (and therefore a mis-resolved
//! edge) can never originate here.

pub mod generics;
pub mod module_path;
pub mod reexport;
pub mod resolver;
pub mod use_graph;

pub use module_path::{
    ModulePath, WorkspaceCrates, discover_workspace_crates, module_path_for_file,
};
pub use resolver::{CanonicalId, ResolveContext, canonical_path_for_def, resolve_path};
pub use use_graph::{UseGraph, extract_use_graph};
