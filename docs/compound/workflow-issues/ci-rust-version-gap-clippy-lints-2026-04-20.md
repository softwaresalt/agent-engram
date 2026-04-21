---
title: "CI Rust version ahead of local toolchain causes extra clippy lint failures"
description: "CI runs Rust 1.95 (April 2026); local is 1.85 — 5 lint classes only caught by 1.95"
problem_type: "ci_failure"
category: "workflow-issues"
component: "CI"
root_cause: "Rust stable advances monthly; CI auto-picks latest stable while local pin stays behind"
resolution_type: "workaround"
severity: "medium"
message: "error: clippy::useless_conversion / doc_markdown / unnecessary_hashes / uninlined_format_args"
file_path: ".github/workflows/ci.yml"
citations:
  - "docs/closure/003-s-cozodb-phase2-closure.md"
tags:
  - "rust"
  - "clippy"
  - "ci"
  - "toolchain"
  - "version-gap"
  - "1.95"
  - "doc_markdown"
  - "useless_conversion"
---

## Problem

4 CI fix iterations were required after passing local clippy because CI runs
Rust 1.95 while the local toolchain is 1.85. Each CI run surfaced a new class
of lints not present in 1.85.

## Root Cause

GitHub Actions CI auto-installs the latest stable Rust toolchain. As of
April 2026, that is Rust 1.95. The local workspace is pinned to the toolchain
used at project start (1.85). Ten months of toolchain drift introduced new
or tightened clippy lints that only fire on 1.95:

| Lint | Introduced / tightened in | Trigger |
|------|--------------------------|---------|
| `useless_conversion` | Tightened ~1.88+ | `.into_iter()` on `Vec` (already `IntoIterator`) |
| `doc_markdown` | Tightened ~1.90+ | Unbackticked capitalized identifiers (CozoDB, SurrealDB) |
| `unnecessary_hashes` | New ~1.92+ | `r#"..."#` without inner quotes |
| `uninlined_format_args` | Tightened ~1.91+ | Named vars not inlined in `format!` strings |
| `private_bounds` | New ~1.93+ | `pub fn` with `pub(crate)` trait bound |
| `collapsible_match` | Tightened ~1.94+ | `if cond { if let Some(x) { ... } }` inside match arm — fix with match guard `pattern if cond => { ... }` |

Each class required its own CI fix commit, causing 4 fix iterations total.

## Resolution (immediate)

Fix each class as it appears. Typical fixes:

```rust
// useless_conversion: remove .into_iter()
vec.into_iter().for_each(...)  →  vec.iter().for_each(...)

// doc_markdown: backtick identifiers in doc comments
/// CozoDB backend  →  /// `CozoDB` backend
/// SurrealDB       →  /// `SurrealDB`

// unnecessary_hashes: remove # when no inner quotes needed
r#"SELECT * FROM t"#  →  r"SELECT * FROM t"

// uninlined_format_args: inline variable
format!("{}", count)  →  format!("{count}")

// private_bounds: make trait pub or make fn pub(crate)
pub fn foo<T: pub(crate) Trait>  →  pub fn foo<T: Trait>  (make trait pub)

// collapsible_match: move outer `if` into match guard
match x {
    Some(v) => {
        if v.is_valid() { process(v); }  // ← collapsible
    }
}
// Fix: move condition into guard
match x {
    Some(v) if v.is_valid() => { process(v); }
}
```

## Resolution (permanent — stashed as follow-up `83B6BC5A`)

Update local Rust toolchain to match CI:
```bash
rustup update stable
```

Or pin CI to a specific version that matches local:
```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    toolchain: "1.85"
```

## Prevention

* Update `rustup update stable` at the start of each new feature/chore branch.
* When a new sprint begins, check `rustc --version` locally vs the latest stable.
* Consider adding `rust-toolchain.toml` to pin both local and CI to the same version,
  eliminating the gap entirely:
  ```toml
  [toolchain]
  channel = "1.95"
  ```
