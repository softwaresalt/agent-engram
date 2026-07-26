//! Canonical-identity infrastructure for **Python** module-namespace call
//! resolution (feature 096-F).
//!
//! This module builds the Python analogue of the Rust [`super::canonical`]
//! substrate — deriving a per-file module namespace and (in later units) the
//! symbol-level import bindings — so a cross-module same-name Python call can be
//! resolved to a single canonical target instead of being dropped by name-only
//! ambiguity. It reuses the language-agnostic canonical DB layer and the
//! singleton/duplicate fail-closed core with **zero canonical-schema change**.
//!
//! Design invariant (013-D, absolute): every derivation is *fail-closed* — any
//! ambiguity, unprovable package layout, star/relative/dynamic import, or
//! competing binding yields no canonical identity rather than a guess.

pub mod module_path;

pub use module_path::python_module_path_for_file;
