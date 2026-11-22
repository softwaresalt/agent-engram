---
title: "179-S runtime verification and pre-merge operational readiness"
date: 2026-11-21
doc_type: closure
source: "179-S / 142.090-T"
shipment: 179-S
task: 142.090-T
feature: 142-F
mode: pre-merge
surface: cli
verification_status: PASS_WITH_FOLLOW_UP
closure_status: READY_WITH_CONDITIONS
compaction_status: pending
owner: softwaresalt
last_code_affecting_head: "7c20d3856455f64d06c5ba8b3ba3f7c56edaa575"
follow_up_stash:
  - "A8C1F6D5"
  - "A8ECECB2"
  - "B7886A16"
  - "7FB49791"
  - "663DFB04"
  - "7998F677"
  - "FB9F34F2"
  - "22F070FD"
  - "E39E7B68"
  - "773841E3"
  - "8C21E5DA"
  - "28DE8213"
  - "FC4AEE3B"
  - "5214C10D"
conditions:
  - condition: "GitHub Actions CI is green on the PR's final head"
    satisfied: false
    evidence: "The feature PR and its final-head CI result are not yet available"
    owner: softwaresalt
    due_utc: "2026-11-22T23:59:59Z"
    attestation_evidence: "Required build and start-launcher-windows checks pass on the final head"
    escalation_path: "Hold merge and invoke fix-ci within scope; hand off at the circuit breaker"
    backlog_item_id: "N/A"
  - condition: "Copilot review is satisfied for the final head"
    satisfied: false
    evidence: "The feature PR has not yet been reviewed"
    owner: softwaresalt
    due_utc: "2026-11-22T23:59:59Z"
    attestation_evidence: "gsd gate copilot-review reports PASS for the final head"
    escalation_path: "Hold merge until all threads and review-body findings are dispositioned"
    backlog_item_id: "N/A"
---

## Scope and immutable evidence

Shipment `179-S` adds a hidden `engram doctor --smoke` CLI readiness
workflow for task `142.090-T`. The last code-affecting revision observed is
`7c20d3856455f64d06c5ba8b3ba3f7c56edaa575`.

| Evidence | Outcome |
|---|---|
| RED compilation | PASS: `cargo test --test integration_doctor_smoke --no-run` |
| RED witness | CONFIRMED: 3 new scenarios failed with `unrecognized subcommand 'doctor'` before implementation |
| Doctor smoke integration | PASS: 14/14 |
| DAX CLI parity | PASS: 11/11 |
| CLI tool catalog parity | PASS: 6/6 |
| Binary CLI tests | PASS: 2/2 |
| Read-server parity | PASS: 8/8 with the external-temp environment |
| Format | PASS: `cargo fmt --all -- --check` |
| Clippy | PASS for default and `cozo-backend,embeddings` feature sets |
| Full target build | PASS: `cargo build --all-targets` |
| Line endings | PASS: zero CRLF, BOM, lone-CR, mixed-newline, or EOF findings |
| Local adversarial review | P0=0 and P1=0 after in-scope daemon teardown fix; follow-ups retained |
| Documentation audit | `AUDIT_FOLLOWUPS` with deferred Stage-owned and lifecycle hardening findings |

The full `cargo dev-test --no-fail-fast` attempt reached unrelated Windows-host
failures in the launcher contracts and a pre-existing daemon TTL test stall.
The daemon startup class is already captured in deferred entry `773841E3`.
The launcher class is captured in `5214C10D`. Focused in-scope tests and all
production build, format, and Clippy gates are green.

The local `cargo audit` run completed and found 19 dependency vulnerabilities
plus 22 allowed warnings. Dependency remediation is outside the authorized
surface and remains tracked in `8C21E5DA`.

## Validator evidence

| Surface and adapter | Expected signal | Observed signal | Outcome |
|---|---|---|---|
| CLI / command | Hidden `doctor --smoke` probes a live read server, exits 0, and returns `passed: true` | Integration scenario passed against a live `read_server` daemon with no provenance or workspace delta | PASS |
| CLI / command | `engram doctor` requires `--smoke` and exits 2 | Required flag was enforced and exit 2 observed | PASS |
| CLI / command | `engram --help` omits hidden `doctor` | Help output omitted the command | PASS |
| CLI / command | Non-git workspace returns a structured `DaemonSpawnFailed` error | Exit 1, `engram_code: 8005`, and `DaemonSpawnFailed` observed | PASS |
| CLI / process | Read-server daemon is not terminated by smoke | Follow-up `get_daemon_status` reported `read_server`; the test then terminated and reaped its explicitly owned child | PASS |

## Invariants and pre-deploy audit

* The reused `ReadServer` probe is non-destructive and leaves no
  `.engram/cozo` state; the existing read server is neither rebound nor
  terminated.
* A cold Managed smoke run can initialize normal persistent `.engram` state and
  leave its doctor-started daemon detached to finish hydration safely. Doctor
  itself sends no write or control RPC in that mode, so the probe remains
  read-only after startup.
* The hidden command is absent from public help and MCP tool dispatch.
* The existing production MCP runner is not called from
  `src/cli/commands/report.rs`.
* No `unwrap`, `expect`, `allow`, `warn`, or unsafe code was added.
* `142-F`, `177-S`, and `142.058-T` remain active or queued as appropriate.
* No schema, migration, configuration rollout, or deployment is introduced.

## Monitoring and rollback handoff

| Requirement | Evidence and threshold | Owner |
|---|---|---|
| Healthy signal | Feature CI and next `main` CI are green; doctor smoke integration remains 14/14 | softwaresalt |
| Failure signal | Any `doctor --smoke` exit-code, envelope, hidden-help, provenance, or workspace-side-effect regression | softwaresalt |
| Monitoring plan | Observe feature PR checks, next `main` CI, and the downstream `177-S` parity matrix | softwaresalt |
| Rollback trigger | A confirmed reused-daemon termination, unexpected ReadServer workspace delta, mid-hydration kill, structured-error regression, or leaked public command | softwaresalt |
| Rollback procedure | Revert the merged feature stack with a reviewed two-parent merge-commit revert | softwaresalt |
| Validation window | Feature PR CI, next `main` CI, and the immediate downstream `177-S` execution | softwaresalt |

## Risky action record

| Proposed action | ActionRisk | Approval and containment | ActionResult |
|---|---|---|---|
| Add a hidden CLI readiness subcommand | moderate | Authorized by `142.090-T`; scoped CLI implementation plus in-scope review fixes to doctor and shim lifecycle ownership | applied and verified |
| Merge the feature PR | high | Bounded P-017 approval applies only after local readiness, CI, P-018, P-009, and P-016 pass | pending |
| Deploy or publish a binary | high | Separate operator approval required | not started |

## Releasability

The CLI contract is `PASS_WITH_FOLLOW_UP`. Pre-merge releasability is
`READY_WITH_CONDITIONS`; CI and Copilot review for the final feature head remain
open automated conditions. Deferred full-suite instability remains tracked in
`773841E3`, `5214C10D`, and the dependency-audit follow-up `8C21E5DA`.

## Documentation consistency audit

Verdict: `AUDIT_FOLLOWUPS`.

The audit examined the candidate's backlog, closure, memory, task, plan, and
runtime evidence against the ten checkable categories. The in-scope
read-server test teardown defect and the pre-existing Managed-daemon mutation
risk were fixed and re-verified at the observed code head. Broader Stage-owned
lifecycle and case-reference inconsistencies, existing shipment intake items,
and `28DE8213`'s residual lifecycle-hardening portions remain captured in the
`follow_up_stash` IDs above for Stage disposition. Final mutable-surface review
readiness remains in the feature PR body.
