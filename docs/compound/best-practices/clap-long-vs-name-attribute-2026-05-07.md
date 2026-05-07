---
title: "clap v4: long= sets flag name; name= sets arg ID only"
description: "In clap v4 derive macros, #[arg(name = \"foo\")] sets the argument ID, not the long flag. Use #[arg(long = \"foo\")] to create --foo."
problem_type: "design_misuse"
category: "best-practices"
component: "src/bin/engram.rs"
root_cause: "Confusion between clap's name= (arg ID) and long= (long flag name) attributes"
resolution_type: "code_fix"
severity: "medium"
message: "flag --node-type appears instead of intended --type"
file_path: "src/bin/engram.rs"
citations:
  - "PR #85 second Copilot review round, commit 5b7d832"
  - "docs/closure/2026-05-07-042-F-cli-parity-closure.md"
tags:
  - "clap"
  - "cli"
  - "arg-parsing"
  - "rust"
---

## Problem

When adding a `--type` flag to a clap struct field using the derive macro, the field was
annotated `#[arg(long, name = "type")]`. The intent was to create a `--type` flag. Instead,
clap created `--node-type` (derived from the field name `node_type`) and set the arg ID
(internal identifier) to `type`.

```rust
// Wrong: creates --node-type, ID is "type"
#[arg(long, name = "type")]
node_type: Option<String>,

// Correct: creates --type <KIND>, ID is "node_type" (derived from field)
#[arg(long = "type", value_name = "KIND")]
node_type: Option<String>,
```

## Root Cause

In clap v4 derive macros:

- `name = "foo"` — sets the argument's internal ID (used in error messages and programmatic access). Does NOT set the long flag name.
- `long = "foo"` — sets the long CLI flag as `--foo`.
- `long` (bare, no value) — derives the long flag from the field name using snake_case → kebab-case conversion.

Because `name = "type"` was combined with bare `long`, clap used the field name `node_type`
to derive `--node-type` and only changed the internal ID to `type`.

## Resolution

Replace `name = "type"` with `long = "type"`:

```rust
#[arg(long = "type", value_name = "KIND")]
node_type: Option<String>,
```

The `value_name = "KIND"` sets the placeholder in help text: `--type <KIND>`.

## Prevention

- When you want `--custom-name` on a field with a different name, always use `long = "custom-name"`.
- `name =` in clap v4 derives is for arg ID, not the flag name — rarely needed unless you need programmatic arg access by a specific ID.
- When a flag produces an unexpected name in `--help` output, check whether `long` or `name` was used.
- Test `--help` output during development to catch naming mismatches early.
