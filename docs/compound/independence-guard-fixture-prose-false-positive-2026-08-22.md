---
title: "An independence guard that scans a human-authored fixture for forbidden tokens false-positives on the fixture's own policy prose"
description: "Feature 127-F's oracle independence guard (scripts/check-oracle-independence.{ps1,sh}) scanned three artifacts — the oracle test, its capture helper, and the JSON fixture — for the tokens 'tools_catalog' and 'all_tools'. The fixture's _policy note legitimately names the source contract src/shim/tools_catalog.rs to explain why it must be hand-authored, so the guard failed on the fixture's own documentation. The fix scopes the forbidden-token (derivation) scan to the two Rust sources that could actually `use` the module, and enforces the fixture's independence via the fixture-regeneration scan plus its header instead."
problem_type: "self_referential_static_check_false_positive"
category: "test-tooling-correctness"
component: "scripts/check-oracle-independence.{ps1,sh} / tests/contract/mcp_catalog_oracle_test.rs"
root_cause: "a forbidden-substring scan was applied uniformly to code and to a documentation-bearing data file; the data file must reference the very identifier the scan forbids in order to document its own independence policy"
resolution_type: "scope_token_scan_to_code_enforce_data_via_regeneration_and_header"
date: "2026-08-22"
shipment: "123-S"
---

# An independence guard that scans a data fixture for forbidden tokens false-positives on its own policy prose

## Problem

A "mechanically enforced independence" guard for the agent-visible MCP catalog
oracle scanned three files for the tokens `tools_catalog` and `all_tools`: the
oracle test, its capture helper, and the human-authored JSON fixture. The
fixture's `_policy` field deliberately states it is "transcribed and reviewed by
hand from the catalog source contract in `src/shim/tools_catalog.rs`". That
prose contains the token `tools_catalog`, so the guard reported a
FORBIDDEN-IMPORT violation against the fixture's own documentation — a false
positive that made the guard fail on a clean tree.

The same trap has two subtler forms encountered in the same shipment:

- An **in-test** mirror of the guard, living in the scanned test file, would
  embed the forbidden tokens as string literals and flag its own source. Fix:
  assemble the tokens from fragments at runtime (`["tools","catalog"].join("_")`).
- A single-line file crashed the PowerShell guard because `Get-Content` returns
  a scalar (not an array) for one-line files, so `$lines.Count` throws under
  `Set-StrictMode`. Fix: wrap in `@(Get-Content ...)`.

## Resolution

Distinguish **code-derivation** from **data-provenance**:

- The forbidden-token scan targets only the code that could `use` the production
  module (the two `.rs` oracle sources). Scanning a JSON data file for an
  identifier substring never prevented derivation anyway — identical values are
  identical regardless of whether the token is mentioned.
- The fixture's independence is enforced by the **fixture-regeneration scan**
  (no build/test/CI step writes the fixture) plus its human-authored header —
  the properties that actually matter for a data artifact.
- Enforce the invariant in **CI**, not only locally: run the guard as a CI step
  so the fixture-regeneration scan gates merges. (The forbidden-import invariant
  is additionally enforced inside `cargo test` by an in-test assertion.)

## Takeaway

When a static "purity" check covers both code and a documentation-bearing data
file, scope the code-oriented rule (imports/derivation) to code, and enforce the
data file's invariant through provenance rules (no regeneration) and a reviewed
header — not by forbidding it from naming the thing it must document. Make the
check self-consistent (fragment-built tokens, array-safe reads) and wire it into
CI so it actually gates.
