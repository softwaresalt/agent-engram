---
title: "Closure: autoharness v1.3.0 tune + CI lint fix"
date: 2026-04-25
pr: 25
merge_commit: 36ce9d2b83f2f0f6b1fe3a898c67b72d82ad34ff
branch: chore/autoharness-tune-v1.3.0
mode: post-merge
status: READY
owner: softwaresalt
---

# Closure: autoharness v1.3.0 tune + CI lint fix

## Summary

Re-tuned the installed agent harness to align with autoharness v1.3.0 template
updates. Also resolved two pre-existing `collapsible_match` lint violations in
`src/services/parsing/markdown.rs` that were undetected locally (Rust 1.85) but
blocked CI (Rust 1.95 with `--all-targets`).

**Merge commit**: `36ce9d2b` — PR #25 → `main`
**CI**: ✅ green (cozo-backend: 47s, surreal-backend: 8m15s)
**Review**: All Copilot review threads resolved (3 rounds, 11 comments addressed)

---

## Invariants to Preserve

| Invariant | Status |
|---|---|
| Markdown parsing behaviour unchanged | ✅ lint fix only, no logic change |
| All test suites pass | ✅ `cargo test` green |
| MCP tool contract unchanged | ✅ no protocol changes |
| IPC daemon protocol unchanged | ✅ no daemon changes |
| Harness artifacts regenerable from v1.3.0 templates | ✅ backups in `.autoharness/backups/2026-04-25-v1.3.0/` |

---

## Pre-Deploy Audit

This is a local-first binary — there is no deployment target, canary, or
production environment to gate. All items below apply to the development
harness only.

| Check | Result |
|---|---|
| No schema migrations | ✅ n/a |
| No feature flags to set | ✅ n/a |
| No environment variable changes required | ✅ `ENGRAM_DATA_DIR` is set in `start.ps1` |
| `.mcp.json` workspace paths still need correction | ⚠️ known follow-up (see below) |
| Tavily API key still in `.mcp.json` | ⚠️ known follow-up (see below) |

---

## Deployment / Rollout Path

**Merge-only** — no deployment step. The harness changes take effect
immediately upon next agent session start. The `markdown.rs` fix ships
in the next binary build.

---

## Post-Deploy Checks

1. Start a new agent session; confirm the harness loads without template-variable errors.
2. Verify `backlogit_list_checkpoints` and `backlogit_poll_hook_events` appear in the
   backlog registry (`backlogit_list_operations` or inspect `.autoharness/backlog-registry.yaml`).
3. Run `cargo test` to confirm green after fresh checkout.
4. Run `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` to confirm lint clean.

---

## Risky Action Record

| Action | Risk | Approval | Result |
|---|---|---|---|
| Regenerate 9 skills from v1.3.0 templates | moderate | Operator (tune-harness session) | applied — all 11 customized files intentionally preserved |
| Fix `collapsible_match` in production parser | moderate | Implicit in CI gate | applied — lint fix only, no behavioural change verified by test suite |
| Merge with `--admin` to bypass review-required policy | moderate | Operator explicit "Merge approved" | applied |
| `rebase_merge` still enabled on repository | low | Deferred as follow-up | planned — should be disabled (P-009) |

---

## Healthy Signals

- `cargo test` green on fresh checkout of `main`
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` exits 0
- Agent sessions load instruction files without `{{VARIABLE}}` placeholder noise
- `backlogit_list_checkpoints` and `backlogit_poll_hook_events` are usable operations

---

## Failure Signals

- Any test regression in `tests/integration/markdown_indexing_test.rs` (markdown parser coverage)
- Any clippy error in `src/services/parsing/markdown.rs` on CI
- Agent sessions emit unknown template variable warnings from regenerated skills

---

## Monitoring Plan

This change has no runtime telemetry surface. Monitoring is limited to:

- CI runs on subsequent PRs (confirms no regression introduced)
- First agent session after merge (confirms harness loads cleanly)
- Manual `cargo test` on first significant code change to main

No dashboards, alerts, or metrics required.

---

## Rollback Trigger

Trigger rollback if any of the following occur within 7 days of merge:

- `cargo test` fails on `main` with a root cause traced to this PR
- Markdown indexing produces incorrect symbol output compared to pre-merge behaviour
- Agent sessions fail to load harness instructions due to a template regression

## Rollback Procedure

```bash
git revert 36ce9d2b --no-commit
git commit -m "revert: roll back autoharness v1.3.0 tune (36ce9d2b)"
git push origin main
```

The `.autoharness/backups/2026-04-25-v1.3.0/` directory contains pre-tune snapshots
of all regenerated files for reference during any manual reversal.

---

## Validation Window

**Duration**: 7 days (until 2026-05-02)
**Owner**: softwaresalt
**Observation method**: CI status on next PR + manual `cargo test` on first code change

---

## Follow-Up Items

| # | Item | Priority |
|---|---|---|
| 1 | Disable `allow_rebase_merge` on the repository (GitHub Settings → General → Pull Requests) to fully satisfy P-009 | medium |
| 2 | Fix `.mcp.json` workspace paths: `D:\GitHub\` → `D:\Source\GitHub\` | low |
| 3 | Remove or externalize Tavily API key from `.mcp.json` | medium |

---

## Readiness Status

**READY** — merge complete, CI green, all review threads resolved.
Follow-up items are non-blocking; stashed for Stage triage.
