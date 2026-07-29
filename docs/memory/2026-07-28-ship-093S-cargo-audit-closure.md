---
title: "Ship 093-S — cargo audit transitive-advisory remediation (102-F / F97D51DF) merge closure"
date: "2026-07-28"
type: "ship-closure-memory"
feature: "102-F"
shipment: "093-S"
pr: 295
merge_commit: "308c04bcb0a3aa2b127ee62336291891c6d98c53"
status: "shipped"
---

# Ship 093-S — merge closure memory

## Outcome

Feature 102-F (task 102.001-T) shipped via **PR #295** (merge commit
`308c04bcb0a3aa2b127ee62336291891c6d98c53`, merge-commit strategy per P-009 —
2 parents `661a88c8` ∪ `9362f58c`). Bounded dependency-hygiene maintenance:
triaged + remediated the 10 pre-existing transitive `cargo audit`
vulnerabilities (PR #291 baseline, source stash F97D51DF). **`cargo audit`
10 → 1** — no source/functional change, only `Cargo.lock` version bumps plus
one `Cargo.toml` cozo default-feature reduction.

## Git base decision

`main` was protected/unpushable, so the feature branch
`102-cargo-audit-transitive-advisories` was based on the merged `origin/main`
tip `bb968963` (post-#293/094-S). Mid-flight the PR branch was updated with a
`Merge branch 'main'` commit (`9362f58c`) that reset the Copilot review clock;
the gate was re-satisfied at that HEAD before merge. The pre-existing
`start.ps1` working-tree modification was never touched or committed.

## What was remediated (9) vs accepted (1)

**Remediated via in-range `cargo update` bumps (6):**

- `crossbeam-epoch` 0.9.18 → 0.9.20 — RUSTSEC-2026-0204
- `rustls-webpki` 0.103.9 → 0.103.13 — RUSTSEC-2026-0049 / 0098 / 0099 / 0104
- `rmcp` (+`rmcp-macros`) 1.1.0 → 1.8.0 — RUSTSEC-2026-0189 (stayed within
  `^1.1`; rmcp 2.x major deliberately NOT taken — engram uses `transport-io`
  stdio, not the vulnerable Streamable-HTTP server transport)

**Remediated via cozo default-feature reduction (3):**

- `rustls-webpki 0.101.7` ×3 — RUSTSEC-2026-0098 / 0099 / 0104. cozo 0.7.6
  pulled `minreq` (an HTTP client engram never uses) ONLY via its default
  `compact` → `requests` feature. Set `default-features = false` on the cozo
  dependency and re-enabled exactly the capabilities engram uses
  (`storage-sqlite`, `storage-sqlite-src`, `graph-algo` = `compact` minus
  `requests`). Re-resolving pruned `minreq`, `rustls 0.21.12`, and
  `rustls-webpki 0.101.7` (dependency count 557 → 552), staying within cozo
  0.7.6 — **no major bump required**.

**Accepted-with-rationale (1):**

- `lz4_flex 0.10.0` — RUSTSEC-2026-0041. Reached via `cozo → swapvec →
  lz4_flex`; `swapvec` is a **non-optional** cozo dependency (cannot be
  feature-dropped like `requests`) and pins `^0.10`. A fix needs a cozo 0.8+
  major bump (graph-backend blast radius) — out of scope per the 102.001-T
  guardrail. `swapvec` is cozo's internal disk-spill (trusted round-trip data,
  not attacker-controlled). Deferred as stash `99AFF44B`.

Full record: `docs/decisions/2026-07-28-cargo-audit-advisory-triage.md`.

## Copilot review

Cycle 1 raised 1 VALID inline finding — that the 3 `rustls-webpki 0.101.7`
advisories did NOT need a cozo major bump because `minreq` is optional behind
the default `requests` feature. Fixed via the default-feature reduction above
(commit `cfdcc925`); replied + thread resolved. Subsequent reviews (at
`cfdcc925` and the merge-from-main HEAD `9362f58c`) generated no new comments.
One suppressed low-confidence note (add YAML frontmatter to the decision doc)
was left as-is per operator decision. **1 review-fix cycle of 3 — no
circuit-breaker.**

## Quality gates (all GREEN)

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --no-default-features --features cozo-backend,embeddings
  --all-targets -- -D warnings -D clippy::pedantic` — PASS
- `cargo test … --all-targets` — PASS (196 result groups, 0 failed;
  CI-equivalent with `ENGRAM_DATA_DIR` unset)
- `cargo audit` — 10 → 1 (only accepted `lz4_flex` remains)
- CI `build` — SUCCESS

## Landmine — ENGRAM_DATA_DIR test isolation

The dev shell inherits `ENGRAM_DATA_DIR=…\engram\.engram` (a persistent shared
data dir). `integration_retrieval_eval_thresholds` tests false-fail
(`sample_size: 7` instead of 0) unless `ENGRAM_DATA_DIR` is **removed** before
`cargo test` — setting it to a single fresh temp dir does NOT fix the
`--all-targets` run (parallel test binaries share that one dir and contaminate
each other). CI passes because CI never sets it. CI-equivalent local run:
`if (Test-Path Env:ENGRAM_DATA_DIR){Remove-Item Env:ENGRAM_DATA_DIR}` first.
Unrelated to the Cargo.lock bumps.

## Follow-up stashed (1)

- `99AFF44B` (task, low) — cozo 0.8+ major-version bump to clear the remaining
  `lz4_flex` RUSTSEC-2026-0041 advisory.
