---
title: "Floating dtolnay/rust-toolchain@stable silently drifts Clippy versions and breaks unrelated PRs"
doc_type: learning
source: "119-S Ship session — PR #346 CI remediation"
description: >-
  CI pinned dtolnay/rust-toolchain by full SHA but requested the floating
  `stable` toolchain input, which silently tracks whatever Rust release is
  current when the runner resolves it. A new Clippy lint introduced in a
  Rust point release (1.98.0) started failing CI on files an unrelated PR
  never touched. Fix by pinning the action to its `master` branch HEAD and
  passing an explicit exact `toolchain:` version, not by suppressing the
  lint.
category: build-errors
date: 2026-08-20
confidence: high
evidence:
  - shipment: 119-S
  - feature: 123-F
  - pr: 346
  - failing_run: 32428317557
  - fixed_run: 32431095652
  - merge_commit: 0bc82aeb2a01ae69a231b54e9b04aa0e2ce99c4e
  - follow_up_stash: B1024A34
tags: [ci, rust, clippy, toolchain, dtolnay-rust-toolchain, github-actions, drift]
---

## Finding

`.github/workflows/ci.yml` and `.github/workflows/release.yml` pinned
`dtolnay/rust-toolchain` to a full commit SHA (good action-pinning hygiene)
but the SHA was the tip of the action's `stable` branch with `# stable` as
only a comment — the actual toolchain requested was still the floating
`stable` channel, resolved fresh by rustup on every runner. When upstream
Rust released 1.98.0 (introducing `clippy::unused_async_trait_impl`), CI
started failing `-D warnings` builds with 78 new errors in three
pre-existing files, on a PR (#346) whose diff never touched any of them.
The local dev toolchain was still 1.97.0 (rustup had not auto-updated), so
the failure was invisible locally and looked like a CI-only regression.

A crate-level `#[allow(clippy::unused_async_trait_impl)]` or
`#[allow(unknown_lints)]` was considered and rejected: the lint is unknown
to the still-supported 1.97.0 toolchain under `-D warnings`, so either
`allow` would itself fail to compile on the older, still-valid local/MSRV
toolchain — trading one deterministic failure for another.

## Root Cause

`dtolnay/rust-toolchain`'s toolchain selection is driven by **which ref
(branch/tag) the action itself is checked out at**, not by SHA alone. Each
version string (`stable`, `1.89.0`, `nightly`, ...) is a **separate branch**
in the action's repo with its own commit history. Pinning a SHA that
happens to belong to the `stable` branch still means "whatever `stable`
resolves to on the runner today," because the action's own code (which
reads the optional `toolchain:` input) only lives on the `master` branch.
A SHA from `stable` is provably **not an ancestor of `master`**
(`gh api .../compare/master...<stable-sha>` returns `status: diverged`),
so passing `with: toolchain: "1.97.0"` alongside a `stable`-branch SHA is
silently ignored.

## Safe Fix

1. Resolve `dtolnay/rust-toolchain`'s `master` branch HEAD SHA (`gh api
   repos/dtolnay/rust-toolchain/commits/master`).
2. Pin `uses: dtolnay/rust-toolchain@<master-HEAD-sha> # master` (not a
   `stable`/version-branch SHA).
3. Add `with: toolchain: "1.97.0"` (or whatever exact version was already
   locally validated) alongside the existing `components:`/`targets:`
   inputs.
4. Repeat identically for **every** job/workflow that installs the
   toolchain (PR CI build job, PR CI Windows launcher job, release.yml
   Quality Gates job, release.yml cross-platform build matrix job) — a
   partial pin re-creates the exact drift it is meant to prevent (this was
   caught by a Copilot review "suppressed comment" after the first,
   ci.yml-only pin).
5. Also pin `rust-toolchain.toml`'s `channel` from `stable` to the same
   exact version so local dev matches CI/release exactly.
6. Rerun local `cargo fmt --all -- --check`, `cargo clippy ... -- -D
   warnings -D clippy::pedantic`, `cargo dev-test`, and `cargo audit` after
   the pin to confirm the exact toolchain still resolves the same
   dependency-locked build.

## Guardrails

- Never add a crate-level `allow` for an unknown/newer-toolchain-only lint
  just to unblock CI — it weakens lint policy and can break older,
  still-supported toolchains.
- Verify the pin candidate SHA is an ancestor of the action's `master`
  branch before relying on the `toolchain:` input; a SHA from a
  version/`stable`/`nightly` branch will silently ignore that input.
- Repo-wide toolchain-version pinning has a bigger blast radius than a
  single feature's diff — call it out explicitly in the PR description
  (scope/evidence/risk) rather than silently bundling it, especially when
  review tooling (Copilot) flags the mismatch between PR scope and change
  footprint.
- Track the deliberate future upgrade (new Clippy version + lint
  redesign-or-reviewed-allow decision) as its own backlog item; do not let
  an urgent unblock become a silent permanent pin.

## Result

PR #346 (119-S/123-F) went from `build` failing deterministically on 3
consecutive pushes to fully green CI after the exact `1.97.0` pin was
applied consistently across `ci.yml`, `release.yml`, and
`rust-toolchain.toml`. Merged via merge commit
`0bc82aeb2a01ae69a231b54e9b04aa0e2ce99c4e`. Deliberate 1.98+ upgrade tracked
as backlog stash `B1024A34`.
