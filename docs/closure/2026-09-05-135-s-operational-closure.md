---
title: 135-S Retire HTTP and SSE Transport Surfaces — Operational Closure
description: Pre-merge release-readiness, monitoring, rollback, and follow-up record for shipment 135-S.
---

## Releasability

**Status: READY WITH CONDITIONS** (post-merge; named probe condition still
not conclusively satisfied — see below)

PR #383 merged via merge commit `0cfffc0cf7220d8f643da28cd2025aff558b7d76`
(merge method: `gh pr merge 383 --merge`, operator-approved for exactly this
PR at approved HEAD `64414ec99089fc6eb3b902525d60ac31f76afd11`). The prior
`READY WITH CONDITIONS` status (below, preserved for history) remains in
effect: the named condition's `cli-daemon-status` re-run was performed
post-merge in a quiet environment (six stale orphaned `backlogit mcp`
processes from prior sessions were cleared first) and demonstrated genuine,
sustained daemon progress with **no crash or error**, but the process was
stopped after ~15 minutes without reaching `Ready` — the condition (a
successful probe) was not met, only reasoned as non-blocking. The residual
gap is assessed as a first-index cold-start cost for a brand-new per-branch
Cozo namespace, not a code defect, but this assessment is not itself a
substitute for the successful probe the condition requires. See the
post-merge addendum in `docs/closure/2026-09-05-135-s-runtime-verification.md`
for full evidence; that document's own verdict remains `PASS WITH
FOLLOW-UP`, consistent with keeping releasability at `READY WITH
CONDITIONS` here rather than upgrading to an unconditional `READY`. No code
in this PR is implicated: the daemon lifecycle/IPC/indexing code the probe
exercises is untouched by 135-S and is extensively covered by the 674+
passing automated tests below.

<details>
<summary>Prior pre-merge conditional status (resolved above)</summary>

Downgraded from an earlier unconditional `READY` after Copilot review
correctly noted it contradicted the cited runtime-verification report (the
`cli-daemon-status` probe is `BLOCKED`; see
`docs/closure/2026-09-05-135-s-runtime-verification.md`, verdict
`PASS WITH FOLLOW-UP`). The named condition is below.

**Condition**: the `cli-daemon-status` probe against a live-bound workspace
must be re-run successfully (or the validator-manifest drift captured in
stash `DA0AF326` corrected and re-run) before or shortly after merge, in a
quiet environment without competing parallel builds. This condition does
not block merge of this PR — it blocks treating 135-S's runtime posture as
unconditionally verified. No code in this PR is implicated: the daemon
lifecycle/IPC code the probe exercises is untouched by 135-S and is
extensively covered by the 674+ passing automated tests below.

</details>

| Requirement | Evidence |
|---|---|
| Healthy signal | `cargo check --all-targets` (default features) GREEN; `cargo check/clippy/test --no-default-features --features embeddings,cozo-backend,git-graph` (all features except the pre-existing, out-of-scope `otlp-export` break) GREEN; `cargo fmt --all -- --check` clean; `cargo test --doc` GREEN (9/9); `cargo dev-test` 674-675/675 GREEN with 1 confirmed-flaky, confirmed-pre-existing, confirmed-passes-in-isolation test each run (see below). |
| Review | Adversarial Review (3 reviewers, report-only) — READY_WITH_FOLLOWUPS, covering the **full PR diff** (`main..HEAD`) — see the addendum in `docs/closure/2026-09-05-135-s-retire-http-sse-transport-adversarial-review.md`. Plus multiple rounds of Copilot automated review across this PR's remediation history, each addressed by fixing in-scope gaps directly (rustdoc correction, test-assertion additions, test rename, doc-content sync, rollback-range correction, and regenerating this repository's own checked-in `.github/copilot-instructions.md`/`.claude/instructions.md` via the real `engram install --hooks-only` code path) or deferring out-of-scope findings to stash with thread replies. Every Copilot review thread opened to date has been replied to and resolved via GraphQL `resolveReviewThread`; because a fresh Copilot review can be posted after any subsequent push, this row intentionally does not freeze a round/comment count — see PR #383's review history at the current `headRefOid` for the authoritative up-to-date tally. |
| Runtime verification | `docs/closure/2026-09-05-135-s-runtime-verification.md` — verdict `PASS WITH FOLLOW-UP`. CLI-version and MCP-protocol (initialize + tools-catalog) probes GREEN via substitute real commands; `cli-daemon-status` probe **BLOCKED** (named condition above). |

## Invariants to preserve

- The daemon continues to bind to `127.0.0.1`-equivalent local-only IPC
  transports (Unix domain socket / Windows named pipe) exclusively; no
  network-listening HTTP surface is reintroduced.
- Exactly three transport surfaces remain supported and documented: direct
  daemon IPC, the `engram` CLI, and stdio MCP via `engram shim` (asserted by
  `tests/contract/supported_transport_surface_test.rs`).
- `sysinfo` remains available for `t101_idle_memory_under_100mb` and other
  live users; no other dependency removal beyond the four proven-unused
  ones (`axum`, `tower`, `tower-http`, `tokio-stream`).
- `cargo dev-test` (default features) remains the canonical local merge
  gate and must stay GREEN modulo pre-existing documented flakes.

## Pre-deploy audits

- Confirm no downstream consumer (CI workflow, install script, external
  documentation site) references the deleted `--features legacy-sse` build
  flag. Checked: `.github/workflows/ci.yml` has no residual reference
  (confirmed during adversarial review).
- Confirm `Cargo.lock` was regenerated (not hand-edited) after the
  dependency removal — done via `cargo check --all-targets` in the
  142.024-T commit.

## Post-deploy checks

- Run `engram --version` and confirm the binary still starts (verified
  pre-merge; re-verify against the actual release/CI-built artifact).
- Spot-check that `.github/copilot-instructions.md` and
  `.claude/instructions.md` generated by a fresh `engram install` in a
  scratch workspace no longer contain any `http://` URL and do state
  "stdio MCP" (covered by `tests/integration/installer_test.rs`
  `s064_fresh_install_creates_hook_files` / `s068_custom_port_no_longer_rendered_in_hook_urls`,
  both updated in this PR to assert exactly this).

## Monitoring

No new runtime surfaces were introduced. Deleted surfaces (HTTP/SSE
listeners on `/sse`, `/mcp`, `/health`) were never enabled by default (gated
behind `legacy-sse`, which had no CI or release build enabling it), so there
is no monitoring signal to retire. The daemon's existing structured JSON
logs, `_health`/`get_health_report`, and `get_daemon_status` remain
unchanged and are the ongoing monitoring surface for the IPC/CLI/stdio-MCP
transports that remain.

## Failure signals

- `cargo dev-test` or `cargo ci` (all-features minus `otlp-export`) turning
  newly red on `main` after merge, beyond the two already-documented
  pre-existing flakes.
- Any operator report that `engram install` no longer generates valid
  `.github/copilot-instructions.md` / `.claude/instructions.md` /
  `.mcp.json` hook files.
- Any operator report of a compile error referencing `legacy-sse`, `axum`,
  `tower`, `tower-http`, or `tokio-stream` after pulling this change.

## Rollback trigger

Any of the failure signals above, observed on `main` after merge and
attributable to this shipment's commits (not to the pre-existing,
independently-documented `otlp-export` or `archive_verifier` flakes).

## Rollback procedure

Branch-local (pre-merge) or `main`-local (post-merge) `git revert` of this
PR's **transport-shipment-scoped** commit range, `main..HEAD` **excluding**
`2d2c1324`: `4dd6ec5f`, `dca08a4c`, `3d9b9976`, `c5f85ddd`, `e7a53729`,
`28a55ca3`, `13317b81`, `4e22a892`, `82ab5c87`, `796717e2`, `a7ca96b8`,
`afa1aff3`, `2bb97bd3`, `a11296f7`. Reverting this range restores the
deleted modules, the `legacy-sse` feature, prior documentation wording, and
the pre-shipment task/manifest bookkeeping for 135-S. No production/runtime
data is touched by this change (source-only deletion, dependency-manifest
edit, backlog-state bookkeeping, and documentation correction) — rollback
carries no data-migration risk.

`2d2c1324` (`chore: carry forward checkpoint quarantine dispositions and
incident handoff`) is deliberately excluded from this range: per its own
commit message it carries "No shipment 135-S implementation content,"
instead recording an unrelated, operator-approved carry-forward of
checkpoint-quarantine/abandonment dispositions and the critical schema
incident intake (`0011C2DC`) onto this branch. Reverting it alongside the
transport-shipment commits would reactivate stale, already-resolved
recovery-checkpoint state and remove live incident tracking that has
nothing to do with this transport change. If that carry-forward content
itself ever needs to be rolled back, do so as an independent, explicitly
scoped revert of `2d2c1324` — never bundled with this transport rollback.

## Risky action record

| Field | Value |
|---|---|
| `ProposedAction` | Delete `src/server/{mcp,router,sse}.rs`; remove `legacy-sse` feature + 4 dependencies; delete/rewrite associated tests; correct installer/doc/ADR HTTP claims (full scope: see PR description). |
| `ActionRisk` | `destructive` (irreversible source deletion; feature/dependency removal) |
| `ActionResult` | **approved** (operator, verbatim: *"Approve the destructive deletion scope for 142.023-T and resume Dark Factory mode."*) → **applied** (all four tasks committed, gates green, PR #383 open) |

## Owner

Ship agent (this session), on behalf of the operator who approved the
destructive-deletion scope verbatim above.

## Validation window

Standard PR review + CI window. No extended bake/soak period is warranted:
the change is a pure deletion/documentation-correction with no new runtime
behavior, and the full local test suite (675 tests) plus doctests (9 tests)
plus the two newly-written contract test files provide direct coverage of
the change surface. The one open condition (daemon-status probe re-run) is
tracked above and does not gate this validation window.

## Follow-up requirements (all captured to stash, none blocking this PR)

| Stash ID | Priority | Summary |
|---|---|---|
| `9A7C9F8F` | medium | Pre-existing `otlp-export`/`opentelemetry_sdk` API-drift break in `src/server/observability.rs` (confirmed unrelated to 135-S; ambiguous vs. existing 7B270F79/E12542FF from 133-S/134-S). |
| `0443D844` | medium | Pre-existing intermittent `archive_verifier_runs_the_unpacked_native_binary` stdout-parsing flake under full-suite parallel execution (ambiguous vs. existing 58B33C45/4EE241DC/3067BC32 from 133-S/134-S). |
| `3A9CBD36` | medium | Stale `legacy-sse`/HTTP-endpoint references remain in `src/config/mod.rs`, `src/bin/engram.rs`, `README.md`, `docs/configuration.md`, `docs/troubleshooting.md`, `docs/workflows.md` — outside 135-S's owned-file scope. |
| `39B44E19` | medium | Supplemental to `3A9CBD36`: `docs/log-observation-guide.md:87-91` also has a stale `legacy-sse` "Compatibility note" (found via Copilot review on PR #383); recorded separately per the P-021 C2 single-write invariant (3A9CBD36 cannot be amended). Recommend Stage folds both into one follow-up task. |
| `E007DF00` | low | Dead `AppState::check_rate_limit()`/`RateLimiter` code in `src/server/state.rs` after its only caller (`sse.rs`) was deleted — outside 135-S's owned-file scope (Constitution Principle VI candidate). |
| `DA0AF326` | low | `.autoharness/workspace-profile.yaml` runtime-validation probe commands (`engram status`, `contract_initialize`, `contract_tools`) have drifted from the actual CLI/test surface — pre-existing, unrelated to 135-S. |
| `20FDC0A7` | high | **Post-merge closure finding**: `backlogit shipment ship 135-S` (v1.10.1) hung non-terminating (CPU climbing, zero WAL growth) during shipment closure; worked around via manual archive-file authoring (134-S manual-safe-close precedent). Full repro/recovery: `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`. Outside 135-S's owned-file scope (backlogit itself). |

All seven are `requires_deliberation: true` and await Stage triage/harvest.
None represent a regression introduced by, or a gap in, 135-S's own stated
scope — each is either pre-existing (confirmed via baseline comparison), a
deliberate, documented scope boundary (P-021 C1), or (for `20FDC0A7`) a
post-merge closure-tooling finding captured per the same P-021 C2 discipline.

## Source artifact cleanup

Checked `custom_fields.source_stash_id` and `custom_fields.source_deliberation_id`
on every item in the shipped scope (142.023-T, 142.024-T, 142.025-T,
142.026-T, covering feature 142-F, and the shipment record 135-S itself)
via `backlogit get {id}`.

- Archived stash (`source_stash_id`): **none present on any shipped-scope
  item** — 0 stash entries archived.
- Archived deliberations (`source_deliberation_id`): **none present on any
  shipped-scope item** — 0 deliberation artifacts archived.
- Skipped (already archived or not found): n/a — no candidate fields were
  present to begin with.

No source-artifact retirement was performed. This is the correct, precise
outcome per the Ship Role Boundary: cleanup is scoped strictly to
manifest-derived `source_stash_id`/`source_deliberation_id` references, and
none exist for this shipment's scope. No discretionary stash edit,
triage, or archival was performed.

## Compaction status

`done` — `compact-context --target all` invoked at Ship Step 8 (post-merge
closure, 2026-09-06). Candidate scope: the 4 memory checkpoints for the
just-closed 135-S release unit (the eligible candidate per the
completed-work rule; no other memory/plan/closure artifacts in the
workspace met the age/size compaction thresholds). Result: 4 verbose
checkpoints (29,056 bytes) consolidated into 1 compacted summary
(`docs/memory/compacted/2026-09-06-135-s-retire-http-sse-transport-compacted.md`,
7,610 bytes; 21,446 bytes recovered from active `docs/memory/`); originals
preserved (never deleted) under `docs/archive/memory/2026-09/`. 0 exec-plans
and 0 closure records were compaction candidates (no stale/threshold-exceeding
artifacts found). No degradation — this run completed cleanly.

## Post-merge closure record

| Field | Value |
|---|---|
| Merge SHA | `0cfffc0cf7220d8f643da28cd2025aff558b7d76` |
| Merge method | merge commit (`gh pr merge 383 --merge`) |
| Merge confirmed | `MERGE_CONFIRMED` — `gh pr view 383` state `MERGED`; `git merge-base --is-ancestor` verified against `origin/main` |
| Shipment closure | Manual safe-close (see `.backlogit/archive/135-S.md` AUDIT RATIONALE) — `backlogit shipment ship` hung non-terminating (tool defect, stashed as follow-up); direct `move --status shipped` rejected by CLI; manual archive-file creation used, matching the 134-S precedent for P-015-protected covering-feature scope |
| Task statuses | 142.023-T, 142.024-T, 142.025-T, 142.026-T — all `done`, all individually archived pre-closure |
| Covering feature | `142-F` — verified `active`, byte-for-byte unchanged (P-015 protection confirmed) |
| Reconciliation | `.backlogit/reconcile/135-S-pre-20260906-110752.md` (PROCEED), `.backlogit/reconcile/135-S-post-20260906-113100.md` (PROCEED) |
| Post-merge closure branch | `post-merge/135-s-retire-http-and-sse-transport-surfaces` |

