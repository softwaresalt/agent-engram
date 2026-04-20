---
title: "Mutually exclusive Cargo features require --no-default-features when testing non-default"
description: "--features foo adds to defaults; use --no-default-features --features foo to replace them"
problem_type: "compile_error"
category: "workflow-issues"
component: "Cargo.toml"
root_cause: "cargo --features adds to default features, not replaces; both mutually exclusive backends activate"
resolution_type: "config_change"
severity: "high"
message: "error: Features `surreal-backend` and `cozo-backend` are mutually exclusive"
file_path: "Cargo.toml"
citations:
  - "docs/closure/003-s-cozodb-phase2-closure.md"
  - "src/db/mod.rs"
tags:
  - "rust"
  - "cargo"
  - "features"
  - "no-default-features"
  - "mutually-exclusive"
  - "cozo-backend"
  - "surreal-backend"
---

## Problem

Running `cargo clippy --features "cozo-backend"` triggered the `compile_error!`
guard in `src/db/mod.rs`:

```text
error: Features `surreal-backend` and `cozo-backend` are mutually exclusive;
       enable exactly one database backend at a time.
```

The intent was to lint the CozoDB backend path only.

## Root Cause

`Cargo.toml` declares `default = ["embeddings", "surreal-backend"]`. The
`--features` flag **adds** to the default feature set rather than replacing it.
So `--features "cozo-backend"` activates **both** `surreal-backend` (from
defaults) and `cozo-backend` (explicitly added), triggering the mutual
exclusion guard.

```toml
[features]
default = ["embeddings", "surreal-backend"]  # surreal-backend is on by default
surreal-backend = ["surrealdb"]
cozo-backend = ["cozo"]
```

## Resolution

Use `--no-default-features` to clear the default feature set before adding the
desired backend:

```bash
# WRONG — activates both backends simultaneously
cargo clippy --features "cozo-backend"

# CORRECT — disables surreal-backend (default), enables cozo-backend only
cargo clippy --no-default-features --features "cozo-backend"
cargo test --no-default-features --features "cozo-backend"
cargo build --no-default-features --features "cozo-backend"
```

This matches how CI runs the cozo-backend matrix leg (confirmed in CI logs:
`cozo-backend, --no-default-feature...`).

Note: `--no-default-features` also disables `embeddings`. If embedding tests
are needed alongside cozo-backend, add it back explicitly:
`--no-default-features --features "cozo-backend,embeddings"`

## Prevention

When any feature group uses `compile_error!` for mutual exclusion, document the
correct invocation pattern in `Cargo.toml` comments and in CI workflow comments:

```toml
# cozo-backend: Enable with --no-default-features --features cozo-backend
# (surreal-backend is the default; both cannot be active simultaneously)
cozo-backend = ["cozo"]
```

The comment is already present in `Cargo.toml` at line 70; the key is to
**also** test the command locally before assuming it matches CI behavior.
