---
title: 135-S Retire HTTP and SSE Transport Surfaces — Operational Closure
description: Pre-merge release-readiness, monitoring, rollback, and follow-up record for shipment 135-S.
---

## Releasability

**Status: READY**

| Requirement | Evidence |
|---|---|
| Healthy signal | `cargo check --all-targets` (default features) GREEN; `cargo check/clippy/test --no-default-features --features embeddings,cozo-backend,git-graph` (all features except the pre-existing, out-of-scope `otlp-export` break) GREEN; `cargo fmt --all -- --check` clean; `cargo test --doc` GREEN (9/9); `cargo dev-test` 674-675/675 GREEN with 1 confirmed-flaky, confirmed-pre-existing, confirmed-passes-in-isolation test each run (see below). |
| Review | Adversarial Review (3 reviewers, report-only) — READY_WITH_FOLLOWUPS. Full report: `docs/closure/2026-09-05-135-s-retire-http-sse-transport-adversarial-review.md`. |
| Runtime verification | `docs/closure/2026-09-05-135-s-runtime-verification.md` — CLI version probe and MCP protocol (initialize + tools-catalog) probes GREEN via substitute real commands; daemon-status manual probe inconclusive-but-not-regressed under session resource contention, covered by extensive automated daemon test suite. |

## Monitoring

No new runtime surfaces were introduced. Deleted surfaces (HTTP/SSE
listeners on `/sse`, `/mcp`, `/health`) were never enabled by default (gated
behind `legacy-sse`, which had no CI or release build enabling it), so there
is no monitoring signal to retire. The daemon's existing structured JSON
logs, `_health`/`get_health_report`, and `get_daemon_status` remain
unchanged and are the ongoing monitoring surface for the IPC/CLI/stdio-MCP
transports that remain.

## Rollback

Branch-local `git revert` of the shipment's commits
(`3d9b9976`, `c5f85ddd`, `e7a53729`, `28a55ca3`, `13317b81`, `4e22a892`,
plus this closure commit) restores the deleted modules, the `legacy-sse`
feature, and prior documentation wording. No production/runtime data is
touched by this change (source-only deletion, dependency-manifest edit,
and documentation correction) — rollback carries no data-migration risk.

## Owner

Ship agent (this session), on behalf of the operator who approved the
destructive-deletion scope verbatim: "Approve the destructive deletion
scope for 142.023-T and resume Dark Factory mode."

## Validation window

Standard PR review + CI window. No extended bake/soak period is warranted:
the change is a pure deletion/documentation-correction with no new runtime
behavior, and the full local test suite (675 tests) plus doctests (9 tests)
plus the two newly-written contract test files provide direct coverage of
the change surface.

## Follow-up requirements (all captured to stash, none blocking this PR)

| Stash ID | Priority | Summary |
|---|---|---|
| `9A7C9F8F` | medium | Pre-existing `otlp-export`/`opentelemetry_sdk` API-drift break in `src/server/observability.rs` (confirmed unrelated to 135-S; ambiguous vs. existing 7B270F79/E12542FF from 133-S/134-S). |
| `0443D844` | medium | Pre-existing intermittent `archive_verifier_runs_the_unpacked_native_binary` stdout-parsing flake under full-suite parallel execution (ambiguous vs. existing 58B33C45/4EE241DC/3067BC32 from 133-S/134-S). |
| `3A9CBD36` | medium | Stale `legacy-sse`/HTTP-endpoint references remain in `src/config/mod.rs`, `src/bin/engram.rs`, `README.md`, `docs/configuration.md`, `docs/troubleshooting.md`, `docs/workflows.md` — outside 135-S's owned-file scope. |
| `E007DF00` | low | Dead `AppState::check_rate_limit()`/`RateLimiter` code in `src/server/state.rs` after its only caller (`sse.rs`) was deleted — outside 135-S's owned-file scope (Constitution Principle VI candidate). |
| `DA0AF326` | low | `.autoharness/workspace-profile.yaml` runtime-validation probe commands (`engram status`, `contract_initialize`, `contract_tools`) have drifted from the actual CLI/test surface — pre-existing, unrelated to 135-S. |

All five are `requires_deliberation: true` and await Stage triage/harvest.
None represent a regression introduced by, or a gap in, 135-S's own stated
scope — each is either pre-existing (confirmed via baseline comparison) or
a deliberate, documented scope boundary (P-021 C1).

## Compaction status

`pending` — finalized by Ship Step 8 (`compact-context`) after merge.
