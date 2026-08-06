---
title: "Shipment 108-S pre-merge operational closure"
date: 2026-08-05
shipment_id: "108-S"
feature_id: "112-F"
status: READY_WITH_CONDITIONS
compaction_status: pending
---

## Shipment 108-S Pre-Merge Operational Closure

### Change summary

Shipment `108-S` adds a focused ignored Windows cold-CLI characterization,
debug-only fixed workspace-local auto-spawn capture, a request-ID-bearing
terminal frame event, and a durable decision. It does not change production
timeout semantics, JSON-RPC wire bytes, startup or shutdown ordering,
persistence, S072, or audit behavior.

### Invariants to preserve

- Release builds do not enable test capture.
- Debug capture paths remain fixed beneath the owned workspace `.engram`.
- `ENGRAM_DATA_DIR` is not inherited across auto-spawn.
- Response bytes and JSON-RPC ID echo remain unchanged.
- No third live execution belongs to this shipment.
- Repository daemon state remains observation-only.

### Validator evidence

The command-adapter report is
`docs/closure/108-S-2026-08-05-runtime-verification.md`. Its verdict is
`BLOCKED`: both live runs completed and cleaned up, but the post-seam pretty
trace could not supply a JSON-decodable response-frame record. The final
JSON-format remediation has non-live coverage only.

### Pre-deploy audits

- No migration, schema, feature flag, credential, or data backfill exists.
- Local review must cover the exact PR HEAD with zero unresolved P0/P1 findings.
- CI, Copilot review, and all review threads must be clean before merge
  approval is requested.
- Merge remains operator-gated and must use a merge commit.

### Deployment and rollout path

This is a merge-only investigation release unit. No daemon deployment or
production rollout occurs in this session. The feature branch and PR remain
open until explicit operator merge approval.

### Post-merge checks

- Confirm the merge commit is present in `origin/main`.
- Re-run non-live focused tests, formatting, and lint through CI.
- Do not rerun the ignored cold live scenario under shipment `108-S`.
- Reconcile and ship the backlog shipment only after merge confirmation.

### Risky action record

| Action | Risk | Result |
|---|---|---|
| Debug-only contained capture and frame event | moderate | Implemented with fixed workspace-local paths |
| Two bounded cold CLI attempts | moderate | Completed at 2/2; both owned daemons cleaned |
| Force-terminate a surviving daemon | destructive | Not invoked |
| Change production timeout or IPC architecture | high | Not performed; fresh Stage intake required |

### Healthy and failure signals

Healthy signals are green deterministic tests and CI, unchanged release stdio,
unchanged wire behavior, one exact response ID in synthetic frame coverage, and
no owned process after the bounded runs.

Failure signals are release capture activation, capture outside `.engram`,
wire-response changes, a stale exact-HEAD review, failed CI, unresolved review
threads, or any attempt to treat the non-live JSON remediation as retained live
proof.

### Monitoring and rollback

- Monitoring: PR CI, exact-HEAD local review, Copilot review, thread inventory,
  and the focused deterministic integration target.
- Rollback trigger: any regression in release behavior, response framing, or
  workspace containment.
- Rollback procedure: revert the two source-file observability commit and the
  focused test registration together; preserve the decision evidence.
- Validation window: through PR merge approval and confirmed merge into
  `origin/main`.
- Owner: Ship until merge; operator for merge approval.

### Releasability evidence

The shipment's investigation DoD explicitly permits one concrete blocker in
place of a complete ID chain. That condition is met, cleanup is proven, and no
production timeout change is present. The PR may be prepared with the runtime
blocker and non-live-only remediation called out. Merge must remain conditional
on exact-current-HEAD review, green CI, completed Copilot review, zero unresolved
threads, clean PR state, and explicit operator approval.

Readiness: READY_WITH_CONDITIONS
