---
title: Adversarial review — 075-S engram usage MEASUREMENT
date: 2026-07-05
shipment: 075-S
branch: 075-usage-measurement
reviewers:
  - gemini-3.5-flash (Tier 1)
  - claude-sonnet-4.6 (Tier 2)
  - gpt-5.4 (Tier 3)
verdict: ship (no gate-blocking defects)
---

# Adversarial review — 075-S

Multi-model adversarial review of the usage-MEASUREMENT diff (`main...075-usage-measurement`).
Three independent reviewers across model tiers; consensus-weighted findings.

## Verdict

**No P0 / gate-blocking defects.** Core aggregation correct; serde back-compat
preserved (`USAGE_SCHEMA_VERSION` held at 2; additive `#[serde(default)]`
fields); coverage complete; privacy intact (no query text or workspace paths in
the new surface); constitution-compliant (saturating/`try_from` idioms, no
panics, no `unsafe`). **Ship it.**

## Findings and resolution

| # | Finding | Conf | Sev | Resolution |
|---|---|---|---|---|
| C-1 | `session_count` is structurally always 0 (no production path sets `connection_id`); 075-S newly surfaced it as an adoption metric | HIGH | P1 | **Fixed** — dropped `session_count` from the adoption `metrics` object; documented as reserved/always-0 in the consumption contract |
| C-2 / M-2 | Heavy `by_correlation_id` map leaked into frequently-polled `get_health_report` / `get_branch_metrics` / `summary.json` (token-efficiency regression) | HIGH | P1 | **Fixed** — moved the map off the shared `MetricsSummary` into a dedicated `correlation_metrics()` fn; surfaced only by `get_token_savings_report`. Two cheap scalar counts remain on the shared struct |
| M-1 | RFC-3339 `String` lexicographic min/max is latently fragile for non-canonical formats | MED | P2 | **Mitigated** — invariant documented in code + docs (emitter always writes canonical `+00:00`); string compare retained (verified order-preserving by the Tier-3 reviewer) |
| U-1 | Empty-timestamp sentinel could pin `time_range.start` to `""` | LOW | P3 | **Fixed** — filter empty timestamps before min/max |
| M-3 | Design: extend `get_token_savings_report` vs a new `get_usage_report` tool | MED | P2 | **Declined** — decision-018 documents the trade-off; the stash sanctioned extending the existing tool |
| U-2 | `try_from(...).unwrap_or(MAX)` "infallible" nit | LOW | P3 | **No-op** — false positive; it is the constitution-preferred saturating idiom |
| U-3 | No output schema for the structured `metrics` object | LOW | P3 | **Advisory** — documented in the consumption contract; MCP output schema not required |

## Test evidence

- Unit + contract + proptest for the affected surfaces pass under the CI feature
  set (`--no-default-features --features cozo-backend,embeddings`).
- clippy `-D warnings -D clippy::pedantic` clean; `cargo fmt` clean.
- Full suite green except the documented Windows-only CozoDB
  `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` flake (fails identically on
  `main` in the same environment — not a regression; passes on Ubuntu CI).
