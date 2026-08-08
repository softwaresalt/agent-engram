---
title: "Dynamically escalate truncated test diagnostics"
description: "Persist complete test output inside the workspace when tool transport truncation hides the actionable failure, then return to normal logging after resolution."
problem_type: "test diagnostic transport truncation"
category: "workflow-issues"
component: "Ship quality-gate diagnostics"
root_cause: "The tool transport output limit truncated cargo test output before the failing-target summary, so repeated console-only retries could not reveal the actual failing test."
resolution_type: "workaround"
severity: "high"
message: "cargo test --all-targets exited 101 without a visible failure summary"
file_path: "logs/111-s-cargo-test-all-targets.log"
citations:
  - "docs/memory/2026-08-08/circuit-break-cargo-test-all-targets.md"
  - "shipment 111-S"
  - "docs/closure/2026-08-08-111-s-index-coordinator-observability-reliability.md"
tags:
  - "cargo-test"
  - "diagnostics"
  - "output-truncation"
  - "dynamic-escalation"
  - "circuit-breaker"
---

## Problem

`cargo test --all-targets` exited 101 three times during shipment 111-S, but
approximately 1.1 MiB of console output exhausted the tool transport limit
before Cargo's final failing-target summary. Repeating the same console-only
command, including quiet and in-memory filtering variants, did not make the
failure actionable.

## Root Cause

The initial blocker was diagnostic transport, not an unknown Cargo condition:
the complete output existed in the child process but the tool response exposed
only a truncated prefix. Once complete combined stdout/stderr was persisted in
the repository `logs/` directory, a bounded tail identified
`integration_smoke::s072_workspace_status_reports_code_graph_counts`, which
failed because an ambient `ENGRAM_DATA_DIR` redirected its disposable fixture
into the live workspace database and produced zero indexed functions.

## Resolution

1. Revalidated shipment 111-S and its ordered-batch gate before mutation.
2. Ran a materially different all-target diagnostic that captured complete
   combined stdout/stderr in
   `logs/111-s-cargo-test-all-targets.log` while returning Cargo's native 101.
3. Read only the bounded Cargo tail, located the `integration_smoke` target,
   and extracted that target's bounded failure block.
4. Reproduced only
   `s072_workspace_status_reports_code_graph_counts` with normal test output.
5. Isolated S072's data directory inside its disposable workspace so ambient
   live-workspace configuration cannot contaminate the fixture.
6. Returned immediately to focused and normal-verbosity quality gates; the
   high-volume workspace capture was not retained as the default execution
   mode.

## Prevention

Diagnostic handling MUST adapt to the failure's observability:

- Start with normal command output.
- If transport truncation hides the final error, do not repeat an equivalent
  console-only invocation and do not treat invisibility as the concrete error.
- Escalate dynamically by persisting complete combined output under workspace
  `logs/`, preserving the native process exit code.
- Inspect only a bounded tail or failure-target section, then narrow to the
  identified test or target.
- Record the concrete error before changing code.
- De-escalate to ordinary focused and gate logging as soon as the failure is
  actionable and fixed; do not leave verbose diagnostics enabled.
- Keep diagnostic artifacts only while they are needed for diagnosis and
  traceability, then archive or remove them under repository safeguards.

Formal circuit-breaker wording should adopt this escalation/de-escalation rule
through a separately reviewed follow-up rather than widening shipment 111-S.
