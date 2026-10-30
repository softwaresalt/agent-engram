---
type: circuit-breaker
timestamp: 2026-10-29T21:05:49Z
agent: "Copilot"
skill: "build-feature"
breaker_type: universal
operation: "start.ps1 preflight Build"
attempts: 3
identity: "workspace-startup:Build:relay_deadline"
---

# Circuit Breaker - Copilot preflight Build

## Failure chain

All three attempts ran from the repository root on
`fix/copilot-preflight-budget`, during runtime verification, using
`start.ps1 --version`. The stable failure is `stage=Build cause=relay_deadline`.
Changing the budget or development build did not reset the counter.

### Attempt 1

* Native exit: 1
* Budget: 300000 ms
* Observed elapsed: 299107 ms
* Evidence: session tool output and unpublished candidate
  `.engram/generations/preflight-1793300236368-36764`
* Normalized message: Build relay deadline expired; Copilot was not launched

### Attempt 2

* Native exit: 1
* Budget: 1800000 ms
* Observed elapsed: 1799136 ms
* Evidence: shell `preflight-live-long` and unpublished candidate
  `.engram/generations/preflight-1793300629272-20920`
* Normalized message: Build relay deadline expired; Copilot was not launched

### Attempt 3

* Native exit: 1
* Budget: 1800000 ms
* Observed elapsed: 1799091 ms
* Evidence: shell `copilot-startup-final`, completed at 2026-10-29T21:05:49Z
* Executable: workspace-local `target/startup/debug/engram.exe`
* Normalized message: Build relay deadline expired; Copilot was not launched
* No SQLite progress reads were performed during this attempt

## Distinct diagnostic failures

An intermediate optimized preflight returned `cause=verifier`, not a deadline.
A temporary current-thread candidate-build diagnostic subsequently exposed
`Index(System(DatabaseError { reason: "database is locked (code 5)" }))`.
Read-only SQLite progress probes overlapped those runs and could interfere with
writers. They were discontinued. The temporary example source was removed.
Neither verifier failure is counted again as a deadline failure.

A supported direct full sync against the intermediate unpublished candidate
succeeded in 634059 ms with no file errors. It skipped 334 existing files and
parsed 113, so this is not evidence that a fresh full candidate build succeeds.

## Context

* Work item: 143-F, blocked pending operator disposition
* Session memory: [startup repair](./copilot-preflight-timeout-memory.md)
* Affected surfaces: both launchers, launcher contract tests, Cargo development
  profile, generated-directory ignore rules, and troubleshooting documentation
* Identity: observed stage, cause, command, workspace, and workflow phase match;
  no hidden-output reclassification or counter reset was used
* Logging controls: no raw diagnostic capture; only bounded, redacted summaries
  and session shell references retained; no environment dump or payload content
* Resolution: universal circuit breaker triggered; no fourth startup attempt
* Next step: operator disposition and a separately scoped investigation of
  full-build performance, potentially using a fully optimized release build

The repair remains incomplete. Mocked launcher success does not establish
successful real Copilot startup.
