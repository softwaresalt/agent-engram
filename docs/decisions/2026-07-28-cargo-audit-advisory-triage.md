# Cargo audit advisory triage & remediation (102-F / 102.001-T)

- **Date:** 2026-07-28
- **Feature / task:** 102-F / 102.001-T (shipment 093-S)
- **Source stash:** F97D51DF
- **Scope:** Dependency-hygiene only — `Cargo.lock` version bumps of transitive
  and direct dependencies. No source/functional change.
- **Baseline:** PR #291 snapshot — `cargo audit` reported **10 vulnerabilities**
  (errors) + **13 allowed warnings** (unmaintained/unsound, cargo-audit default
  informational classification; no `audit.toml`/`deny.toml` present). CI runs
  `cargo audit` with `continue-on-error: true`, so it is non-blocking.

## Outcome summary

| Result | Vulnerabilities | Advisory IDs |
|---|---|---|
| **Remediated (bump)** | 6 | RUSTSEC-2026-0204, -0049, -0098, -0099, -0104 (webpki 0.103.9), -0189 |
| **Accepted-with-rationale** | 4 | RUSTSEC-2026-0041, -0098, -0099, -0104 (webpki 0.101.7) |

Net advisory delta: **10 → 4** vulnerabilities (−6). The 13 allowed
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

## Accepted-with-rationale (no compatible patch; fix requires a major bump)

Both remaining crates are locked by **cozo 0.7.6** (latest published 0.7.x).
Clearing them requires a cozo 0.8+ upgrade, which is a `0.x` major (breaking)
change with a large blast radius on the graph backend — explicitly out of scope
for this low-priority, manifest-only task (task guardrail: "Do NOT pull in
major-version breaks under this low-priority task — defer those as separate
items"). Deferred as follow-up.

1. **lz4_flex 0.10.0** — RUSTSEC-2026-0041 (decompressing invalid data can leak
   uninitialized memory / reused output buffer).
   - **Chain:** `engram → cozo 0.7.6 → swapvec 0.3.0 → lz4_flex 0.10.0`.
     `swapvec 0.3.0` pins `lz4_flex ^0.10`; the fix (`≥0.11.6`) is a `0.x`
     breaking bump the range rejects.
   - **Exposure:** `swapvec` is cozo's internal on-disk spill for large query
     intermediates; the compressed bytes are produced and consumed by swapvec
     itself (trusted round-trip), not attacker-controlled input. Low real-world
     exposure for engram's embedded local usage.
   - **Revisit trigger:** cozo/swapvec release using `lz4_flex ≥0.11.6`, or the
     deferred cozo-major-bump follow-up.
2. **rustls-webpki 0.101.7** — RUSTSEC-2026-0098, -0099, -0104 (URI/wildcard
   name-constraint acceptance, CRL-parse panic).
   - **Chain:** `engram → cozo 0.7.6 → minreq 2.14.1 → {rustls 0.21.12,
     rustls-webpki 0.101.7}`. `minreq 2.14.1` pins `rustls ^0.21` /
     `rustls-webpki ^0.101`; the fix (`≥0.103.12`) is a `0.x` breaking bump the
     range rejects. Reaching it requires a newer minreq (using rustls 0.23),
     which is gated behind a cozo upgrade.
   - **Exposure:** `minreq` is cozo's optional HTTP client; engram uses cozo as
     an embedded, local SQLite-backed graph store and does not drive minreq TLS
     connections in normal operation. Low real-world exposure.
   - **Revisit trigger:** cozo release whose minreq/rustls stack uses
     `rustls-webpki ≥0.103.12`.

## Deferred follow-up

- **cozo 0.7 → 0.8+ major bump** to clear the remaining 4 accepted advisories
  (lz4_flex + rustls-webpki 0.101.7 chain). High-churn breaking change on the
  graph backend — split into its own scoped task per the 102.001-T guardrail.

## Quality gates (post-change)

- `cargo fmt --all -- --check` → **PASS**
- `cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic` → **PASS**
- `cargo test --no-default-features --features cozo-backend,embeddings --all-targets` → **PASS** (CI-equivalent, isolated `ENGRAM_DATA_DIR`)
- `cargo audit` → 4 vulnerabilities remaining (accepted above), 13 allowed warnings

> Note: on a dev shell where `ENGRAM_DATA_DIR` points at a persistent shared
> data dir (e.g. `.engram/`), the pre-existing `integration_retrieval_eval_thresholds`
> isolation tests can read stale corpus and false-fail — an environmental
> artifact documented in that test's own header, unrelated to this change and
> green under CI / an isolated data dir.
