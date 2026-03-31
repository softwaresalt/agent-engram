---
title: "clippy::derivable_impls — Manual Default Impl for Enum Rejected"
problem_type: build_error
category: build_error
component: models
root_cause: missing_feature_gate
resolution_type: code_fix
severity: low
message: "Manual impl Default for enum with single-variant body should use #[derive(Default)] + #[default] attribute (Rust 1.62+, Clippy pedantic)"
file_path: "src/models/policy.rs"
resolved: true
bug_id: ""
tags: [clippy, pedantic, derive, default, enum, derivable_impls, rust-1.62, lint]
date: 2026-03-30
---

## Problem

When `clippy::pedantic` is enabled with `-D warnings`, manually implementing `Default` for an
enum triggers `clippy::derivable_impls` if the implementation can be expressed with
`#[derive(Default)]` and the `#[default]` attribute on the chosen variant (stabilised in
Rust 1.62).

## Symptoms

```
error: this `impl` can be derived
  --> src/models/policy.rs:42:1
   |
42 | impl Default for UnmatchedPolicy {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: remove the manual implementation and use `#[derive(Default)]` with `#[default]` on the variant
   = note: `-D clippy::derivable-impls` implied by `-D warnings`
```

The crate's `.cargo/config.toml` sets `rustflags = ["-Dwarnings"]`, so this is a hard build
error, not just a warning.

## What Did Not Work

- Suppressing with `#[allow(clippy::derivable_impls)]` — technically works, but defeats the
  purpose and will be flagged in review
- Using a struct newtype wrapper to work around enum Default — unnecessary complexity

## Solution

Replace the manual `impl Default` with `#[derive(Default)]` on the enum and `#[default]` on the
desired default variant.

### Before

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum UnmatchedPolicy {
    Allow,
    Deny,
}

impl Default for UnmatchedPolicy {
    fn default() -> Self {
        Self::Allow
    }
}
```

### After

```rust
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum UnmatchedPolicy {
    #[default]
    Allow,
    Deny,
}
```

## Why This Works

`#[default]` on an enum variant was stabilised in Rust 1.62 (RFC 3107). It allows
`#[derive(Default)]` to pick a specific variant as the default, eliminating the need for a
manual `impl Default`. Clippy's `derivable_impls` lint enforces this pattern under pedantic
because the manual form adds boilerplate with no additional expressiveness.

This repository sets `rust-version = "1.85"` in `Cargo.toml`, so `#[default]` is always
available.

## Prevention

- Apply `#[derive(Default)]` + `#[default]` when defining new enums that need a default variant
  rather than writing a manual `impl`
- The pattern extends to structs too: if every field implements `Default`, prefer
  `#[derive(Default)]` over a manual `impl` to stay ahead of `derivable_impls`
- Run `cargo clippy --all-targets -- -D warnings` early in development (not just at commit time)
  to catch lint errors before they accumulate across many files

## Related Solutions

No related solutions found in `.backlog/compound/` at time of writing.
