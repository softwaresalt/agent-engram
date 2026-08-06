---
title: "Ship continuity — shipment 108-S"
date: 2026-08-05
agent: ship
shipment_id: "108-S"
feature_id: "112-F"
status: pr-preparation
---

## Ship Continuity — Shipment 108-S

### P-011 recovery

The authorized third recovery attempt reconfirmed identical trees
`effec8125bf584907cc7f57a1a169312680fd84e`, an empty commit-to-commit diff,
target-branch absence, and the exact Stage dirty set. The non-destructive switch
created `feat/108-s-cold-cli-correlation` from
`54bac42ad74dff5569114821719634ec12438d69` while carrying all Stage artifacts
unchanged. Shipment `108-S` was the only claimed shipment.

### Completed work

- `112.001-T`: focused RED-first cold CLI correlation harness.
- `112.002-T`: debug-only contained capture and terminal frame event.
- `112.003-T`: durable correlation decision and future-fix gate.
- `112-F`: completed after all three dependency-ordered tasks.

Commits:

- `92b47c6a` — `test: add cold CLI correlation harness (112.001-T)`
- `ca398bd6` — `feat: add cold CLI frame correlation seam (112.002-T)`
- `a34d7052` — `docs: publish cold CLI correlation decision (112.003-T)`

### Live attempts and cleanup

The hard live cap is exhausted at `2/2`.

1. RED: 8,438 ms aggregate; PID `16360`; pipe
   `\\.\pipe\engram-9d8bcb92-ec0c-4b25-85a3-7d87314baaf8`; frame records 0.
2. Post-seam: 7,770 ms aggregate; PID `29700`; pipe
   `\\.\pipe\engram-994b0a45-ad68-44f2-b69f-ef62b2862088`; frame records 0
   because pretty tracing was not JSON-decodable.

Both exact PIDs are dead, both pipes are unreachable, both temporary workspaces
were removed, and no force-kill was used. Repository daemon PID `16084`
remained healthy and observation-only.

### Durable result

The final classification is recorded in
`docs/decisions/2026-08-05-cold-cli-request-frame-correlation-follow-up.md`.
The final JSON-format remediation is non-live verified only. No third run is
permitted. Any timeout-contract change or fresh live proof requires a new Stage
intake.

### Next steps

1. Commit backlog, plan, closure, and continuity artifacts.
2. Run full quality gates and exact-current-HEAD report-only review.
3. Push and create a PR with requested reviewers set to none.
4. Require successful CI, completed Copilot review, zero unresolved threads,
   and clean PR state.
5. Stop before merge and wait for explicit operator approval.
