---
title: "std::env::set_var is unsafe in Rust 2024 — prefer function parameters"
description: "std::env::set_var was stabilized as unsafe in Rust 2024 edition. Use function parameters to pass environment-derived values instead of mutating the environment."
problem_type: "safety_violation"
category: "best-practices"
component: "src/shim/mod.rs"
root_cause: "std::env::set_var became unsafe in Rust 2024 due to race conditions in multi-threaded programs"
resolution_type: "design_change"
severity: "high"
message: "use of unsafe function `std::env::set_var`"
file_path: "src/shim/mod.rs"
citations:
  - "PR #85 first round review, commit 4dca15a"
  - "docs/closure/2026-05-07-042-F-cli-parity-closure.md"
tags:
  - "rust-2024"
  - "safety"
  - "environment"
  - "shim"
---

## Problem

When adding workspace override support to `shim::run()`, the first approach was to call
`std::env::set_var("ENGRAM_WORKSPACE", override_value)` inside the function so the rest of
the shim logic could read `ENGRAM_WORKSPACE` from the environment as usual.

This approach fails under `#![forbid(unsafe_code)]` because `std::env::set_var` was
stabilized as `unsafe fn` in the Rust 2024 edition due to soundness issues with
multi-threaded programs (mutating the environment while other threads read it is a data race).

## Root Cause

In Rust 2024 edition, `std::env::set_var` and `std::env::remove_var` were classified as
`unsafe fn` because they can cause undefined behavior in multi-threaded programs. The
`#![forbid(unsafe_code)]` attribute in `src/lib.rs` prevents calling any unsafe function.

## Resolution

Change the function signature to accept the workspace value explicitly as a parameter, and
pass it through the call stack rather than via the environment:

```rust
// Before: tried to use set_var (unsafe in Rust 2024)
pub async fn run() {
    if let Some(ws) = std::env::var("ENGRAM_WORKSPACE").ok() {
        // ... uses ws
    }
}

// After: caller passes workspace override directly
pub async fn run(workspace_override: Option<&str>) {
    let workspace = workspace_override
        .map(str::to_owned)
        .or_else(|| std::env::var("ENGRAM_WORKSPACE").ok())
        .unwrap_or_else(|| std::env::current_dir()...);
    // ...
}
```

The caller in `engram.rs` passes `flags.workspace.as_deref()`:

```rust
shim::run(flags.workspace.as_deref()).await
```

## Prevention

- Never reach for `std::env::set_var` to thread values through a call stack. Use function parameters.
- In Rust 2024 projects with `#![forbid(unsafe_code)]`, `set_var` and `remove_var` are compile errors.
- Priority order for workspace resolution in shim: explicit arg → `ENGRAM_WORKSPACE` env var → `current_dir()`.
- This pattern applies to any function that previously relied on environment mutation to convey state.
