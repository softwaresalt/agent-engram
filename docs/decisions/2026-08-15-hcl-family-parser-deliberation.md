---
title: "Shared HCL parser for Terraform-family files"
description: "Choose one composable HCL parser for .hcl, .tf, and .tfvars indexing."
topic: "HCL and Terraform-family code graph support"
depth: "deep"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - ".backlogit/archive/018-D.md"
  - ".backlogit/queue/121-F.md"
  - ".backlogit/archive/121.001-R-reviewed-gate-for-shared-hcl-parser-plan.md"
  - ".backlogit/queue/117-S.md"
  - "docs/decisions/2026-08-15-tree-sitter-hcl-compatibility-spike.md"
  - "docs/exec-plans/2026-08-15-hcl-family-parser-plan.md"
tags:
  - "hcl"
  - "tree-sitter"
  - "code-graph"
---

# Shared HCL Parser for Terraform-Family Files

## Problem Frame

Stash `4BC7A6DE` requests code-graph support for `.hcl`, `.tf`, and `.tfvars`: canonical language detection, parser registration, symbol/reference extraction, startup traversal, live-sync routing, dependency compatibility, and contract/unit/integration coverage. The operator prioritizes reliability and security, then feature value; simplicity over complexity; and composability over duplicated feature paths.

Success means all three extensions traverse the same parser and canonical `hcl` language identity, with deterministic graph output and no workspace-containment regression. The release stays one cohesive feature but is split into test-first, single-domain work units under two hours.

Out of scope: Terraform evaluation, provider schema resolution, HCL1, plan/state parsing, expression execution, cross-workspace resolution, schema migration, and separate Terraform-specific parser services.

## Research Findings

The current architecture uses `Language` plus direct enum dispatch in `src/services/parsing.rs`, per-language modules under `src/services/parsing/`, canonical extension routing in `src/services/code_graph.rs::language_from_path`, `.gitignore`-aware discovery in `discover_files`, and an independent live-sync extension allowlist in `src/daemon/debounce.rs`. `src/models/config.rs::default_supported_languages` controls zero-config startup indexing.

`ExtractedSymbol::Class` is the existing structural declaration carrier and `ExtractedEdge::References` persists file-to-target reference edges, including unresolved self-loops with a `target_hint`. This permits additive HCL support without a database/schema change.

The exact crates.io archive for `tree-sitter-hcl = 1.1.0` has SHA-256 `5a7b2cc3d7121553b84309fab9d11b3ff3d420403eef9ae50f9fd1cd9d9cf012`, Apache-2.0 licensing, a normal dependency on `tree-sitter-language = 0.1`, and a development dependency on `tree-sitter = 0.25.3`. Its `LANGUAGE: LanguageFn` converts through `.into()`, matching this repository's `tree-sitter = 0.25` and existing grammar pattern. No crate code was executed.

The repository already resolves `tree-sitter 0.25.10` and `tree-sitter-language 0.1.7`. The upstream Git tag `v1.1.0` points to an older source snapshot whose Rust binding is not the published crate binding; therefore the crates.io archive checksum and lockfile, not that tag, are the reproducibility authority for this version.

## Options Evaluated

### Option A: Separate Terraform and Generic HCL Parsers

Create extension-specific parser services and dispatch `.tf`/`.tfvars` separately from `.hcl`. This makes dialect-specific behavior easy to add but duplicates grammar loading, extraction, tests, and routing. It conflicts with simplicity and makes semantic drift likely. Effort and maintenance risk are high.

### Option B: One Canonical HCL Parser Service

Add one `Language::Hcl` and one `parsing/hcl.rs`; route `.hcl`, `.tf`, and `.tfvars` to canonical `hcl`. Extract block headers and top-level attributes as structural symbols, and expression traversals as references. This matches the fixed-language enum architecture, minimizes coupling, and preserves future composability. Effort is medium and risk is bounded by test-first compatibility and runtime gates.

### Option C: Defer or Vendor a Grammar

Defer until a newer release or vendor generated grammar sources. Deferral leaves the feature unmet despite a compatible published crate. Vendoring increases source and supply-chain ownership, complicates updates, and is unnecessary. Effort and long-term complexity are high.

## Trade-off Comparison

| Criterion | Separate parsers | Shared HCL service | Defer/vendor |
|---|---|---|---|
| Reliability | Drift-prone | One tested path | No feature or larger owned surface |
| Security | More code paths | Exact pin plus one boundary | Vendored C/JS review burden |
| Simplicity | Low | High | Low |
| Composability | Low | High | Medium |
| Tree-sitter 0.25 fit | Possible | Verified by manifest/binding inspection | Unnecessary |

## Decision

Choose Option B. Exact-pin `tree-sitter-hcl = "=1.1.0"`; retain Cargo.lock checksum verification; add no workspace `unsafe` blocks. Use one `Language::Hcl` and map all three extensions to `hcl`. Include `hcl` in default supported languages so Terraform-family projects are indexed without hidden opt-in. Keep dialect identity in the file extension/path rather than creating duplicate language variants.

Symbol contract: each top-level HCL block becomes one structural class symbol
named from its header segments joined by dots and prefixed with `hcl.block.`,
such as `hcl.block.resource.aws_instance.web`,
`hcl.block.data.aws_ami.ubuntu`, `hcl.block.module.vpc`, or
`hcl.block.variable.region`. Each top-level attribute becomes
`hcl.attribute.<key>`, which gives `.tfvars` useful symbols. Emit a `Defines`
edge for each.

Reference contract: for expression traversals, emit one normalized dotted
target such as `var.region`, `local.name`, `module.vpc.id`,
`data.aws_ami.ubuntu.id`, or `aws_vpc.main.id`; attribute the reference to the
containing file through the existing persistence contract. Do not execute
expressions or infer provider schemas. Deduplicate by `(file, target)` in
deterministic first-encounter order because the current persistence key cannot
preserve source context as edge identity. HCL v1 persists only a file
self-loop and target hint, never global name resolution.

## Rejected Alternatives

Separate Terraform-specific services were rejected because extension aliases do not justify duplicate parser logic. A trait/plugin parser abstraction was rejected because the shipped architecture deliberately uses direct enum dispatch and a new trait would add indirection without a second implementation need. Vendoring and Git dependencies were rejected because the exact registry package is compatible and checksum-addressed.

## Unresolved Questions

Runtime grammar ABI loading, exact AST behavior on representative Terraform constructs, daemon IPC indexing, and live-sync routing remain implementation-time tests. These are verification gates, not design ambiguity. If the exact dependency fails the ABI RED/GREEN contract, Ship must stop and return the shipment blocked rather than add unsafe shims, transmute language handles, or silently substitute another crate.

## Risks and Mitigations

- Supply chain: exact pin, lock checksum, license/provenance review, `cargo audit`, and no Git dependency.
- Published/tag mismatch: record the mismatch; treat crates.io checksum as authority; do not claim tag reproducibility.
- Unsafe boundary: generated grammar dependencies contain their conventional FFI binding, but Engram adds no workspace unsafe Rust and keeps `#![forbid(unsafe_code)]`.
- False graph semantics: limit v1 to syntactic declarations and traversals; no Terraform evaluation claims.
- Routing drift: centralize the extension family in a shared helper consumed by discovery/language detection and live-sync where feasible, and lock parity with tests.
- Local daemon regression: use isolated temporary workspaces, bounded IPC polling, startup index plus modified-file sync checks, and post-merge observation/rollback criteria.
