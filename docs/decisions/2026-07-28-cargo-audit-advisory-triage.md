# Cargo audit advisory triage & remediation (102-F / 102.001-T)

- **Date:** 2026-07-28
- **Feature / task:** 102-F / 102.001-T (shipment 093-S)
- **Source stash:** F97D51DF
- **Scope:** Dependency-hygiene only — `Cargo.lock` version bumps plus one
  `Cargo.toml` cozo default-feature reduction (disable an unused optional HTTP
  feature). No source/functional change.
- **Baseline:** PR #291 snapshot — `cargo audit` reported **10 vulnerabilities**
  (errors) + **13 allowed warnings** (unmaintained/unsound, cargo-audit default
  informational classification; no `audit.toml`/`deny.toml` present). CI runs
  `cargo audit` with `continue-on-error: true`, so it is non-blocking.

## Outcome summary

| Result | Vulnerabilities | Advisory IDs |
|---|---|---|
| **Remediated (in-range bump)** | 6 | RUSTSEC-2026-0204, -0049, -0098, -0099, -0104 (webpki 0.103.9), -0189 |
| **Remediated (feature reduction)** | 3 | RUSTSEC-2026-0098, -0099, -0104 (webpki 0.101.7) |
| **Accepted-with-rationale** | 1 | RUSTSEC-2026-0041 (lz4_flex) |

Net advisory delta: **10 → 1** vulnerabilities (−9). The 13 allowed
warnings (unmaintained/unsound) are unchanged and out of scope for this task
(scope = the 10 vulnerabilities).

## Remediated (compatible in-range bumps)

All applied via `cargo update` staying within existing semver ranges — no
`Cargo.toml` manifest edits, no source changes.

1. **crossbeam-epoch** `0.9.18 → 0.9.20` — RUSTSEC-2026-0204 (invalid pointer
   deref in `fmt::Pointer`). Patch bump within `^0.9`; pulled by
   `crossbeam 0.8.4` (cozo) and `crossbeam-deque 0.8.6` (ignore, rayon).
2. **rustls-webpki** `0.103.9 → 0.103.13` — RUSTSEC-2026-0049, -0098, -0099,
   -0104 (4 advisories: faulty CRL distribution-point matching, URI/wildcard
   name-constraint acceptance, CRL-parse panic). Patch bump within `^0.103`;
   pulled by `rustls 0.23.36 → ureq → hf-hub → fastembed`.
3. **rmcp** `1.1.0 → 1.8.0` (+ `rmcp-macros 1.8.0`) — RUSTSEC-2026-0189 (DNS
   rebinding in the Streamable HTTP **server** transport). In-range under the
   direct dep `rmcp = "1.1"` (`^1.1`); the 2.x major release was deliberately
   **not** taken (out of scope per task guardrail). Additional context: engram
   configures rmcp with `features = ["server", "transport-io"]` (stdio), i.e.
   it does **not** use the vulnerable Streamable HTTP server transport — the
   bump clears the advisory regardless and is source-compatible (build, fmt,
   clippy-pedantic, and full test suite all green).

## Remediated (cozo default-feature reduction)

**rustls-webpki 0.101.7** — RUSTSEC-2026-0098, -0099, -0104 (URI/wildcard
name-constraint acceptance, CRL-parse panic).

- **Chain (before):** `engram → cozo 0.7.6 → minreq 2.14.1 → {rustls 0.21.12,
  rustls-webpki 0.101.7}`.
- **Root cause:** cozo's `minreq` HTTP client is optional, gated behind cozo's
  `requests` feature. engram enabled it only implicitly — cozo's default
  `compact` feature = `minimal` + `requests` + `graph-algo`, and engram left
  cozo default-features enabled. engram does **not** use any cozo HTTP utility.
- **Fix (no version bump):** set `default-features = false` on the cozo
  dependency and explicitly re-enable exactly the `compact` capabilities engram
  uses — `storage-sqlite`, `storage-sqlite-src`, `graph-algo` — dropping only
  `requests`. Re-resolving the lockfile removes `minreq`, `rustls 0.21.12`, and
  `rustls-webpki 0.101.7` entirely (dependency count 557 → 552). Stays within
  cozo 0.7.6. Verified: build, fmt, clippy-pedantic, and the full test suite
  remain green (graph-algo Datalog + bundled SQLite preserved).

## Accepted-with-rationale (no fix without a cozo major bump)

1. **lz4_flex 0.10.0** — RUSTSEC-2026-0041 (decompressing invalid data can leak
   uninitialized memory / reused output buffer).
   - **Chain:** `engram → cozo 0.7.6 → swapvec 0.3.0 → lz4_flex 0.10.0`.
     `swapvec` is a **non-optional** cozo dependency (not behind a feature flag,
     so it cannot be dropped like `requests` was); `swapvec 0.3.0` pins
     `lz4_flex ^0.10` and the fix (`≥0.11.6`) is a `0.x` breaking bump the range
     rejects. Clearing it requires a cozo/swapvec upstream update or a cozo 0.8+
     major bump — a large graph-backend blast radius, explicitly out of scope
     for this low-priority task (guardrail: "Do NOT pull in major-version breaks
     under this low-priority task — defer those as separate items").
   - **Exposure:** `swapvec` is cozo's internal on-disk spill for large query
     intermediates; the compressed bytes are produced and consumed by swapvec
     itself (trusted round-trip), not attacker-controlled input. Low real-world
     exposure for engram's embedded local usage.
   - **Revisit trigger:** cozo/swapvec release using `lz4_flex ≥0.11.6`, or the
     deferred cozo-major-bump follow-up.

## Deferred follow-up

- **cozo 0.7 → 0.8+ major bump** to clear the one remaining accepted advisory
  (the lz4_flex chain via the non-optional `swapvec` dependency). High-churn
  breaking change on the graph backend — split into its own scoped task per the
  102.001-T guardrail.

## Quality gates (post-change)

- `cargo fmt --all -- --check` → **PASS**
- `cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic` → **PASS**
- `cargo test --no-default-features --features cozo-backend,embeddings --all-targets` → **PASS** (CI-equivalent, `ENGRAM_DATA_DIR` unset)
- `cargo audit` → 1 vulnerability remaining (lz4_flex, accepted above), 13 allowed warnings

> Note: on a dev shell where `ENGRAM_DATA_DIR` points at a persistent shared
> data dir (e.g. `.engram/`), the pre-existing `integration_retrieval_eval_thresholds`
> isolation tests can read stale corpus and false-fail — an environmental
> artifact documented in that test's own header, unrelated to this change and
> green under CI / an isolated data dir.
