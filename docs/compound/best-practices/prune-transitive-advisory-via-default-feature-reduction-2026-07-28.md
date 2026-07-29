---
title: "Clearing a transitive RUSTSEC advisory by reducing a dependency's default features (no major bump)"
description: "When `cargo audit` flags a vulnerable crate reached only through an upstream dependency's DEFAULT feature you don't use, prune it by setting default-features = false and re-enabling exactly the features you rely on — instead of forcing a risky major-version bump. Verify the feature graph first, then confirm the lockfile drops the vulnerable sub-tree with zero test regression."
problem_type: "security_advisory"
category: "best-practices"
component: "Cargo.toml"
root_cause: "A transitive dependency (e.g. rustls-webpki 0.101.7 via minreq) is pulled in only because an upstream crate leaves its DEFAULT feature set enabled and one of those defaults (cozo's `requests` → `minreq` HTTP client) drags in a vulnerable sub-tree the consumer never uses"
resolution_type: "dependency_config"
severity: "medium"
message: "prune_transitive_advisory_via_default_feature_reduction"
file_path: "Cargo.toml"
date: "2026-07-28"
feature: "102-F"
shipment: "093-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/295"
  - "Cargo.toml (cozo dependency: default-features = false, features = [\"storage-sqlite\", \"storage-sqlite-src\", \"graph-algo\"])"
  - "docs/decisions/2026-07-28-cargo-audit-advisory-triage.md (full triage; net 10 → 1)"
tags:
  - "cargo-audit"
  - "rustsec"
  - "dependency-hygiene"
  - "feature-flags"
  - "default-features"
  - "transitive-deps"
  - "cozo"
  - "102-F"
---

# Clearing a transitive RUSTSEC advisory by reducing default features

## Problem

`cargo audit` flags a vulnerable crate (e.g. `rustls-webpki 0.101.7`, three
advisories) that your code never touches. Reverse-dependency inspection shows it
is reached ONLY through an upstream crate's **default** feature set — here
`cozo 0.7.6`'s default `compact` feature enables `requests`, which enables the
`minreq` HTTP client, which pins the old `rustls`/`rustls-webpki` stack. The
naive fix (major-bump the upstream crate to a patched line) has a large blast
radius and often isn't available in-range.

## Technique (prefer this over a major bump when applicable)

1. **Map the upstream feature graph** before touching anything. For cozo 0.7.6:
   `default = ["compact"]`; `compact = ["minimal", "requests", "graph-algo"]`;
   `minimal = ["storage-sqlite", "storage-sqlite-src"]`;
   `requests = ["dep:minreq"]`; `graph-algo = ["graph", "rayon"]`. Confirm the
   vulnerable sub-tree hangs off exactly ONE default feature you don't use
   (`requests`), and that everything you DO use lives under the other defaults.

2. **Disable defaults and re-enable exactly what you use.** In `Cargo.toml`:

   ```toml
   # default `compact` also enables `requests` (minreq HTTP client) which we
   # never use and which drags in the vulnerable old rustls stack. Re-enable
   # exactly the compact capabilities we rely on, minus `requests`.
   cozo = { version = "0.7.6", default-features = false, features = [
       "storage-sqlite", "storage-sqlite-src", "graph-algo",
   ] }
   ```

3. **Re-resolve and diff the lockfile.** `cargo update`/`cargo check`, then
   confirm the vulnerable crates are GONE
   (`Select-String -Path Cargo.lock -Pattern '^name = "minreq"'` → absent).
   Here dependency count dropped 557 → 552 (minreq, rustls 0.21.12,
   rustls-webpki 0.101.7 all pruned) with no version bump.

4. **Prove zero functional regression.** Full quality gates: `cargo fmt
   --check` → `cargo clippy … -D pedantic` → full test suite → `cargo audit`.
   The advisory count must drop and the suite must stay green.

## When it does NOT apply (know the boundary)

The same PR hit the opposite case: `lz4_flex 0.10.0` (RUSTSEC-2026-0041) is
reached via `cozo → swapvec → lz4_flex`, but `swapvec` is a **non-optional**
cozo dependency — there is no feature to drop, and it pins `^0.10`. Feature
reduction cannot help; only a cozo 0.8+ major bump would, so that one was
**accepted-with-rationale** and deferred (stash `99AFF44B`). Rule of thumb:
feature reduction clears an advisory **iff** the vulnerable crate is reachable
solely through an *optional* upstream feature you can turn off without losing
capability.

## Why it matters

Feature reduction is a low-risk, in-range remediation: it changes NO source,
NO public API, and NO upstream major version, yet it can clear multiple
advisories at once by deleting an unused sub-tree. Reach for it before a major
bump whenever the audit path runs through a default feature you don't consume —
but always verify the feature graph and the resolved lockfile rather than
assuming a feature is safe to drop.
