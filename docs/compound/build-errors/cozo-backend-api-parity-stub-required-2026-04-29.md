---
title: "cozo-backend API Parity: Every pub(crate) Method in queries.rs Needs a Stub in cozo_queries.rs"
description: "When a new method is added to the surreal-backend queries.rs, the cozo-backend build fails with 'method not found' unless a matching stub is added to cozo_queries.rs"
problem_type: "build_failure"
category: "build-errors"
component: "src/db/cozo_queries.rs"
root_cause: "Both backends expose the same DatabaseQueries trait (or equivalent struct API); cozo-backend callers compile against the same call sites but cozo_queries.rs does not auto-inherit new methods"
resolution_type: "code_fix"
severity: "high"
message: "error[E0599]: no method named `resolve_reference_target` found for struct `CozoQueries`"
file_path: "src/db/cozo_queries.rs"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/49"
  - "docs/closure/2026-04-29-016-S-sql-reference-resolution-closure.md"
tags:
  - "cozo-backend"
  - "api-parity"
  - "dead-code"
  - "allow-dead-code"
  - "016-S"
  - "feature-flags"
---

## Problem

CI with `--features cozo-backend` failed immediately after adding `resolve_reference_target`
and `get_class_by_name_ci` to `src/db/queries.rs`. The surreal-backend build was clean but the
cozo-backend build reported:

```
error[E0599]: no method named `resolve_reference_target` found for struct `CozoQueries`
```

## Root Cause

The project gates two backend implementations behind mutually exclusive Cargo features
(`surreal-backend` / `cozo-backend`). Both backends must expose the same public API surface
because call sites in `src/services/code_graph.rs` compile against whichever backend is
selected. Adding a new method to `SurrealQueries` (in `queries.rs`) does not automatically
create the method on `CozoQueries` (in `cozo_queries.rs`).

Additionally, cozo-backend stubs that are not actually called in the cozo path will trigger
`dead_code` warnings which, under `#[deny(warnings)]`, become errors.

## Resolution

1. Added a stub implementation of `resolve_reference_target` to `cozo_queries.rs` that
   returns `Ok(None)` (cozo path does not perform re-resolution at this time).
2. Added a stub `get_class_by_name_ci` to `cozo_queries.rs` with `#[allow(dead_code)]`
   because the method is called internally by `resolve_reference_target` in the surreal
   path but the cozo stub for `resolve_reference_target` never calls it.

```rust
#[allow(dead_code)]
pub(crate) async fn get_class_by_name_ci(
    &self,
    _name: &str,
) -> Result<Option<ClassNameIdRow>, EngramError> {
    Ok(None)
}
```

## Prevention

**Rule:** Every `pub(crate)` method added to `src/db/queries.rs` (surreal-backend) must have
a corresponding stub in `src/db/cozo_queries.rs` (cozo-backend). Run the cozo build locally
before pushing:

```bash
cargo build --features cozo-backend --no-default-features
```

Stubs that are unreachable in the cozo code path need `#[allow(dead_code)]` to silence
the pedantic lint. The allowed rationale is API-parity: the method exists to satisfy the
shared call-site contract, not because cozo uses it.
