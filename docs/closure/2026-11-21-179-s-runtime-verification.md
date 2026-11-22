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
last_code_affecting_head: "cb47e7865f48c96a38d5fb44d58e18cfa45de1aa"
follow_up_stash:
  - "773841E3"
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
`cb47e7865f48c96a38d5fb44d58e18cfa45de1aa`.

| Evidence | Outcome |
|---|---|
| RED compilation | PASS: `cargo test --test integration_doctor_smoke --no-run` |
| RED witness | CONFIRMED: 3 new scenarios failed with `unrecognized subcommand 'doctor'` before implementation |
| Doctor smoke integration | PASS: 6/6 |
| DAX CLI parity | PASS: 11/11 |
| CLI tool catalog parity | PASS: 6/6 |
| Binary CLI tests | PASS: 2/2 |
| Read-server parity | PASS: 8/8 with the external-temp environment |
| Format | PASS: `cargo fmt --all -- --check` |
| Clippy | PASS for default and `cozo-backend,embeddings` feature sets |
| Full target build | PASS: `cargo build --all-targets` |
| Line endings | PASS: zero CRLF, BOM, lone-CR, mixed-newline, or EOF findings |

The full `cargo dev-test --no-fail-fast` attempt reached unrelated Windows-host
failures in the launcher contracts and a pre-existing daemon TTL test stall.
The daemon startup class is already captured in deferred entry `773841E3`.
Focused in-scope tests and all production build, format, and Clippy gates are
green.

## Validator evidence

| Surface and adapter | Expected signal | Observed signal | Outcome |
|---|---|---|---|
| CLI / command | Hidden `doctor --smoke` probes a live read server, exits 0, and returns `passed: true` | Integration scenario passed against a live `read_server` daemon with no provenance or workspace delta | PASS |
| CLI / command | `engram doctor` requires `--smoke` and exits 2 | Required flag was enforced and exit 2 observed | PASS |
| CLI / command | `engram --help` omits hidden `doctor` | Help output omitted the command | PASS |
| CLI / command | Non-git workspace returns a structured `DaemonSpawnFailed` error | Exit 1, `engram_code: 8005`, and `DaemonSpawnFailed` observed | PASS |
| CLI / process | Read-server daemon is not terminated by smoke | Follow-up `get_daemon_status` reported `read_server`; the test-owned child was then killed and waited by its drop guard | PASS |

## Invariants and pre-deploy audit

* The readiness probe is non-destructive and leaves no `.engram/cozo` state.
* The hidden command is absent from public help and MCP tool dispatch.
* The existing production MCP runner is not called from
  `src/cli/commands/report.rs`.
* No `unwrap`, `expect`, `allow`, `warn`, or unsafe code was added.
* `142-F`, `177-S`, and `142.058-T` remain active or queued as appropriate.
* No schema, migration, configuration rollout, or deployment is introduced.

## Monitoring and rollback handoff

| Requirement | Evidence and threshold | Owner |
|---|---|---|
| Healthy signal | Feature CI and next `main` CI are green; doctor smoke integration remains 6/6 | softwaresalt |
| Failure signal | Any `doctor --smoke` exit-code, envelope, hidden-help, provenance, or workspace-side-effect regression | softwaresalt |
| Monitoring plan | Observe feature PR checks, next `main` CI, and the downstream `177-S` parity matrix | softwaresalt |
| Rollback trigger | A confirmed daemon termination, side effect, structured-error regression, or leaked public command | softwaresalt |
| Rollback procedure | Revert the merged feature stack with a reviewed two-parent merge-commit revert | softwaresalt |
| Validation window | Feature PR CI, next `main` CI, and the immediate downstream `177-S` execution | softwaresalt |

## Risky action record

| Proposed action | ActionRisk | Approval and containment | ActionResult |
|---|---|---|---|
| Add a hidden CLI readiness subcommand | moderate | Authorized by `142.090-T`; scoped to three owned files | applied and verified |
| Merge the feature PR | high | Bounded P-017 approval applies only after local readiness, CI, P-018, P-009, and P-016 pass | pending |
| Deploy or publish a binary | high | Separate operator approval required | not started |

## Releasability

The CLI contract is `PASS_WITH_FOLLOW_UP`. Pre-merge releasability is
`READY_WITH_CONDITIONS`; CI and Copilot review for the final feature head remain
open automated conditions. Deferred full-suite instability remains tracked in
`773841E3`.
