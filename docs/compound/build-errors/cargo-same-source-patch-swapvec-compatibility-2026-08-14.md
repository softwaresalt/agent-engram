---
title: "Cargo same-source patches cannot widen an indirect semver requirement"
description: "A registry-only lz4_flex patch is rejected and cannot bypass swapvec's ^0.10.0 requirement"
problem_type: "dependency resolution failure"
category: "build-errors"
component: "Cargo dependency graph (cozo/swapvec/lz4_flex)"
root_cause: "Cargo patches must use a different source, and swapvec 0.3.0 requires lz4_flex ^0.10.0, which excludes 0.11.6"
resolution_type: "workaround"
severity: "high"
message: "Cargo patch points to the same source"
file_path: "tmp/rustsec-2026-0041/Cargo.toml"
citations:
  - "docs/decisions/2026-08-10-rustsec-2026-0041-remediation-spike-findings.md"
  - "docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md"
tags:
  - cargo
  - dependencies
  - lz4_flex
  - swapvec
  - rustsec-2026-0041
---

## Problem

The RustSec remediation spike needed to validate `lz4_flex 0.11.6` while the
locked graph remained `engram -> cozo 0.7.6 -> swapvec 0.3.0 -> lz4_flex
0.10.0`. A sandbox-only attempt using `[patch.crates-io] lz4_flex =
"=0.11.6"` failed before compilation with Cargo's same-source patch error.
Even if that restriction were bypassed, `swapvec 0.3.0`'s `lz4_flex ^0.10.0`
requirement is semver-incompatible with `0.11.6`.

## Root Cause

Cargo's `[patch]` mechanism overrides a package from another source; a
version-only entry that still points at crates.io is not a valid replacement.
The indirect dependency's semver range is independently enforced by the
resolver, so a root manifest cannot force an incompatible leaf version without
changing the source package or dependency metadata.

## Resolution

For isolated feasibility validation only, an immutable local copy of
`swapvec 0.3.0` was used with its manifest dependency declaration changed from
`lz4_flex 0.10.0` to `0.11.6`. A path patch selected that copy, while
`lz4_flex 0.11.6` remained the registry package with its recorded checksum.
The bridge passed the default-feature build, 599-test development suite,
all-target tests, pedantic clippy, formatting, audit with zero vulnerabilities,
and 53 focused synthetic Cozo tests.

This bridge is not a production change. Production adoption requires a
reviewed maintained swapvec compatibility bridge or an upstream swapvec release
that widens the requirement. The production manifest and lockfile remained
unchanged.

## Prevention

Before proposing a transitive version pin, inspect both Cargo source rules and
the indirect dependency's declared semver range. Treat a local second-package
manifest bridge as a feasibility harness, not as a production patch, and record
its scope explicitly before execution.
