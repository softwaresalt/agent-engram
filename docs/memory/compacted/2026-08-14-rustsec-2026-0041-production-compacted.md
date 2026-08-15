---
title: "RustSec 2026-0041 production remediation compacted memory"
type: memory
date: 2026-08-14
status: complete
sources:
  - docs/archive/memory/2026-08-14/rustsec-2026-0041-sandbox-closure-memory.md
  - docs/archive/memory/2026-08-14/rustsec-2026-0041-production-remediation-memory.md
---

# RustSec 2026-0041 production remediation compacted memory

The RustSec-2026-0041 spike validated `lz4_flex 0.11.6` in an isolated
sandbox, then production adopted the approved compatibility-fork strategy.
The immutable `softwaresalt/swapvec` fork is based on upstream `swapvec 0.3.0`
commit `3369988`; revision
`72b99cef424a739470cefc08f9a37b934a0afcd4` changes only the dependency
requirement to `lz4_flex 0.11.6`. Root `Cargo.toml` pins that revision and
`Cargo.lock` records the Git source and patched dependency graph.

Production verification passed: `cargo check --locked --all-targets`,
`cargo dev-test` with 599 tests and no failures, pedantic all-target Clippy,
format checking, and `cargo audit` with zero vulnerabilities. Cargo tree
confirmed the chain ends at `lz4_flex 0.11.6`; the vulnerable `0.10.0` package
is absent. The all-features OpenTelemetry Clippy incompatibility is unrelated
and remains a separate follow-up.

The production change and backlog/evidence updates were committed and pushed
as `ed78d5780e22f86601d0139a07128acfe194c3d4`, which pinned the fork by its
short `72b99ce` revision prefix. A subsequent PR review fix finalized the
manifest to the full 40-character SHA
(`72b99cef424a739470cefc08f9a37b934a0afcd4`) in commit `d8932057`, so the
durable provenance now reaches the fully reviewed full-SHA state. Sandbox
artifacts under
`tmp/rustsec-2026-0041/` were subsequently approved for deletion and removed
(~32.7 GB), verified absent, and documented in
`docs/closure/2026-08-14-rustsec-2026-0041-sandbox-cleanup-completion.md`.
