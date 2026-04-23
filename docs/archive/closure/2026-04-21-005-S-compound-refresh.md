---
type: compound-refresh
date: 2026-04-21
context: "005-S post-merge closure — tree-sitter 0.25 upgrade; Swift/C/C++ parsers"
mode: apply
---

# Compound Refresh Report — 005-S

## Scope

All entries in `docs/compound/` reviewed for impact from shipment 005-S:
- tree-sitter runtime upgrade 0.24 → 0.25
- Swift, C, C++ parsers added
- Kotlin deferred (ABI incompatibility confirmed)
- `clippy::collapsible_match` surfaced by CI Rust 1.95

## Entries Reviewed

| Entry | File | Classification | Reason |
|---|---|---|---|
| tree-sitter Grammar ABI Constraint | `build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md` | **update** | ABI table had `0.25.x+ → TBD`; now confirmed. Swift pin noted. Kotlin deferral documented. |
| CI Rust version gap clippy lints | `workflow-issues/ci-rust-version-gap-clippy-lints-2026-04-20.md` | **update** | `collapsible_match` was caught in 005-S CI run; added to lint table with fix pattern. |
| ship-shipment overscoped manifest | `workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` | **keep** | Not affected by 005-S. |
| mutually-exclusive features | `workflow-issues/mutually-exclusive-features-no-default-features-2026-04-20.md` | **keep** | Not affected by 005-S. |
| pub visibility for external test harness | `best-practices/pub-visibility-for-external-test-harness-2026-04-20.md` | **keep** | Not affected by 005-S. |
| string-add-string-ref type error | `build-errors/string-add-string-ref-type-error-2026-04-20.md` | **keep** | Not affected by 005-S. |
| clippy-derivable-impls-enum-default | `build-errors/clippy-derivable-impls-enum-default-2026-03-30.md` | **keep** | Not affected by 005-S. |
| tempdir-lifetime-in-contract-tests | `test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md` | **keep** | Not affected by 005-S. |

## Applied Changes

### tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md

- Filled `0.25.x → ABI 13–15` row (was TBD)
- Updated "Grammar crate version to pin" column to distinguish most (0.23.x) from swift (=0.7.1)
- Added `tree-sitter-swift = "=0.7.1"` exception note
- Documented Kotlin blocked status and activation condition
- Updated solution text: removed "while the `tree-sitter` dependency stays at `0.24.x`" (no longer true)
- Added shipment 005-S citation
- Updated `updated:` frontmatter date

### ci-rust-version-gap-clippy-lints-2026-04-20.md

- Added `collapsible_match` row to the lint table (tightened ~1.94+)
- Added fix pattern for `collapsible_match` to the Resolution code block

## Follow-Up Items

None. All changes are evidence-backed from shipped code in PR #17.
