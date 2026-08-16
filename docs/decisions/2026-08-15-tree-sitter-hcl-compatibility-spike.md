---
title: "Select a Rust/tree-sitter 0.25-compatible HCL grammar"
type: "spike"
date: "2026-08-15"
time_box: "2h"
conclusion: "proceed"
confidence: "high"
linked_parent_work_item: "121-F"
promoted_to:
  - "plan"
tags:
  - "hcl"
  - "dependency"
  - "tree-sitter"
---

# Tree-sitter HCL Compatibility Spike

## Goal

Can Engram safely select an exact HCL grammar package compatible with its Rust 2024 and `tree-sitter 0.25` dependency without executing untrusted grammar code or guessing?

## Success Criteria

Identify one non-yanked, licensed, checksum-addressed release whose published manifest and Rust binding use the modern `tree-sitter-language` bridge and explicitly test against tree-sitter 0.25; confirm fit with Engram's locked dependency graph; document provenance risks and runtime gates.

## Scope Constraints

Read-only investigation. No Cargo build, tests, lint, package installation, archive extraction to disk, generated-code execution, or source/config modification. Public metadata and selected archive members were streamed and inspected in memory only. No unsafe Rust is proposed in Engram.

## Investigation Approach

1. Inspect Engram's current parser architecture and locked tree-sitter versions.
2. Query crates.io metadata, exact version dependencies, ownership, license, checksum, and yanked state.
3. Stream the exact registry archive and inspect only `Cargo.toml`, `Cargo.toml.orig`, and `bindings/rust/lib.rs` without extraction or execution.
4. Compare the binding contract to existing parser registration and inspect the grammar's node vocabulary.
5. Define runtime and supply-chain gates for Ship.

## Findings

### What Was Discovered

Engram declares `tree-sitter = "0.25"`; Cargo.lock resolves `tree-sitter 0.25.10` and `tree-sitter-language 0.1.7`. Existing grammar crates expose `LANGUAGE` constants converted with `.into()` and parser modules call `Parser::set_language`.

Crates.io reports `tree-sitter-hcl 1.1.0` as the sole stable published version, non-yanked, Apache-2.0, with over two million downloads at inspection time. Owners are the `tree-sitter-grammars:crates` team and `ObserverOfTime`. The version is not marked Trusted Publishing.

The official exact-version endpoint reports archive checksum `5a7b2cc3d7121553b84309fab9d11b3ff3d420403eef9ae50f9fd1cd9d9cf012`. A streaming SHA-256 of the downloaded archive matched exactly. The published manifest has `tree-sitter-language = "0.1"`, `cc = "1.2"` as build dependency, and `tree-sitter = "0.25.3"` as dev dependency. It introduces no second runtime `tree-sitter` package.

The published Rust binding exports `pub const LANGUAGE: LanguageFn` and demonstrates `parser.set_language(&LANGUAGE.into())`, exactly matching Engram's tree-sitter 0.25 registration style. The binding contains the conventional generated grammar FFI constructor; Engram itself needs no unsafe code.

The grammar node vocabulary includes `body`, `block`, `attribute`, `expression`, `variable_expr`, `get_attr`, `index`, `function_call`, `identifier`, and `string_lit`. This is sufficient for syntactic block/attribute symbols and dotted traversal references.

The GitHub `v1.1.0` tag resolves to commit `636dbe70301ecbab8f353c8c78b3406fe4f185f5` from 2023 and exposes an older direct `tree-sitter ~0.20` binding, while the crates.io 1.1.0 archive published in 2025 contains the modern bridge. The tag is therefore not a faithful source-provenance pointer for the registry artifact. Current upstream `master` identifies an unreleased 1.2.0 manifest with the modern bridge, but selecting an unreleased branch would reduce reproducibility.

### What Was Tried and Failed

The unauthenticated crates.io endpoint initially rejected the request under its data-access policy; retrying with an identifying User-Agent returned official metadata. Raw GitHub paths for `tree-sitter.json` and query files at `v1.1.0` returned 404 because that historical tag does not contain the published archive layout. No code was run to work around these discrepancies.

Engram indexed search was unavailable due an IPC timeout followed by a locked database; discovery fell back explicitly to targeted Git/file reads after declaring the degradation. `rg` was unavailable, so no further `rg` attempts were made.

### Remaining Unknowns

Static metadata cannot prove runtime grammar ABI loading, parse-tree quality on the project's exact Terraform corpus, or daemon behavior. It also cannot independently establish how crates.io 1.1.0 was produced from upstream history. These unknowns are converted into blocking Ship tests and provenance checks.

## Recommendation

**Conclusion**: proceed.
**Confidence**: high for dependency compatibility; medium for source-tag provenance.

Use exact `tree-sitter-hcl = "=1.1.0"` from crates.io. Require Cargo.lock to retain the official checksum; reject a Git/path substitution or any second `tree-sitter` runtime. Before production implementation, the RED harness must exercise `Parser::set_language(&tree_sitter_hcl::LANGUAGE.into())` and parse representative `.hcl`, `.tf`, and `.tfvars` samples. If that gate fails, return the shipment blocked; do not use unsafe conversions or an unreviewed alternate grammar.

## Next Steps

- Carry the exact pin, checksum, license, ownership, and tag mismatch into plan hardening.
- Add dependency/provenance verification to the first production dependency task.
- Place ABI/load and representative parse tests before parser implementation.
- Require `cargo audit`, lockfile review, daemon IPC verification, and rollback-by-revert/removal in operational closure.

## References

- `Cargo.toml:63-73` and `Cargo.lock:4560-4630`
- `src/services/parsing.rs`
- `src/services/code_graph.rs`
- `src/daemon/debounce.rs`
- `https://crates.io/crates/tree-sitter-hcl/1.1.0`
- `https://crates.io/api/v1/crates/tree-sitter-hcl/1.1.0`
- `https://crates.io/api/v1/crates/tree-sitter-hcl/1.1.0/dependencies`
- `https://github.com/tree-sitter-grammars/tree-sitter-hcl`
- crates.io archive SHA-256: `5a7b2cc3d7121553b84309fab9d11b3ff3d420403eef9ae50f9fd1cd9d9cf012`
