---
title: "String + &String type error E0277 — use format! instead"
description: "String implements Add<&str> not Add<&String>; borrow .repeat() result as &str"
problem_type: "type_error"
category: "build-errors"
component: "src/daemon/ipc_server.rs"
root_cause: "String::Add is implemented for &str not &String; .repeat() returns String, &String is not &str"
resolution_type: "code_fix"
severity: "medium"
message: "error[E0277]: cannot add `&String` to `String`"
file_path: "src/daemon/ipc_server.rs"
citations:
  - "docs/closure/003-s-cozodb-phase2-closure.md"
tags:
  - "rust"
  - "type-error"
  - "string"
  - "add"
  - "E0277"
---

## Problem

CI (Rust 1.95) reported:

```text
error[E0277]: cannot add `&String` to `String`
  --> src/daemon/ipc_server.rs:755:47
```

The offending expression was:
```rust
"/tmp/".to_owned() + &"a".repeat(90)
```

## Root Cause

`String` implements `Add<&str>` (takes `&str` on the right-hand side), **not**
`Add<&String>`. The expression `"a".repeat(90)` returns a `String`. Taking a
reference to that gives `&String`, which does not coerce to `&str` in the `Add`
context — even though `&String` derefs to `&str` in most other contexts.

This error was caught by Rust 1.95 on CI but not locally on Rust 1.85 because
the coercion rules were tightened between versions.

## Resolution

Replace string concatenation with `format!`:

```rust
// BEFORE (fails E0277)
let path = "/tmp/".to_owned() + &"a".repeat(90);

// AFTER (correct)
let path = format!("/tmp/{}", "a".repeat(90));
```

`format!` never requires `Add` trait bounds and handles owned `String` values
directly. It is always the safer and more readable option for runtime string
construction.

## Prevention

* Prefer `format!` over `+` operator for string construction involving
  non-literal operands.
* If `+` is necessary for performance reasons, ensure the RHS is explicitly
  `&str`: `string_val.as_str()` or borrow a literal directly.
* Run `cargo check` or `cargo clippy` on CI Rust version before finalizing
  any string concatenation involving `.repeat()`, `.to_string()`, or other
  `String`-returning methods.
