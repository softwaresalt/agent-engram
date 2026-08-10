---
title: "Circuit-breaker dynamic diagnostic escalation policy"
type: implementation-plan
date: 2026-08-10
source: docs/compound/workflow-issues/dynamic-diagnostic-escalation-2026-08-08.md
status: reviewed
source_stash_ids: [241B503F]
---

# Circuit-breaker dynamic diagnostic escalation policy

## Problem Frame

The authoritative `.github/instructions/circuit-breaker.instructions.md` preserves a universal three-consecutive-failure stop rule but does not tell agents how to react when tool transport truncation hides the concrete error. Shipment 111-S showed that repeated console-only retries can exhaust the breaker without making the failure actionable. The policy must require a bounded observability escalation, preserve the native exit/failure evidence, prevent secret or raw-payload leakage and unbounded logs, record the concrete error, and de-escalate immediately after diagnosis. This changes harness policy/prompt artifacts only; it does not change Engram runtime behavior.

## Requirements Trace

- Preserve `MAXIMUM_RETRY_THRESHOLD = 3` and stop/log/prompt/checkpoint semantics: U1 pins the contract; U2 retains it verbatim.
- Escalate before an information-loss failure becomes another equivalent blind retry: U1 asserts ordering; U2 adds the rule.
- Persist complete combined output under workspace `logs/`, preserve native exit, and inspect only bounded actionable sections: U1/U2.
- Record the concrete error before mutation and return to normal verbosity after diagnosis: U1/U2.
- Prevent secrets, raw payloads, out-of-workspace writes, duplicate captures, and unbounded files/retention: U1/U2.

## Implementation Units

### U1 — Add the RED policy contract harness

**Domain/files:** tests only; `tests/contract/verify_test.rs`. **Cap:** 100 minutes, one file, three scenarios.

Add repository-content contract tests that fail against the current instruction and prove: (1) the universal threshold remains exactly three with stop/log/prompt behavior; (2) after a hidden/truncated failure, diagnostic escalation precedes any equivalent console-only retry and does not reset the underlying failure counter; and (3) workspace-log containment, bounded extraction/retention, concrete-error recording, de-escalation, and secret/raw-payload safeguards are all required. Use `env!("CARGO_MANIFEST_DIR")`; do not add a new Cargo test target or production seam. Observe the focused RED result before U2.

### U2 — Amend the authoritative circuit-breaker instruction

**Domain/files:** instruction authoring only; `.github/instructions/circuit-breaker.instructions.md`. **Cap:** 100 minutes, one file, no generated-family fan-out.

Add a concise diagnostic-visibility section. A non-zero/timeout remains a failed operation and does not reset the universal counter. If the visible response is truncated before the concrete error, the next attempt must materially change observability: capture complete combined stdout/stderr for one bounded invocation under workspace `logs/`, preserve native exit, then inspect only a bounded tail or named failing section. An unchanged console-only rerun is an equivalent blind retry and counts normally. Record the concrete error before any code/policy mutation, then return to normal focused output after diagnosis.

Require preflight exclusion/redaction of secrets and sensitive raw payloads; never persist known credential-bearing output. Use one bounded capture per diagnostic step, command timeout and size/retention controls, no unbounded append, no out-of-workspace path, and no full-log ingestion into agent context. Logs are not committed by default and are archived/removed only under repository safeguards. Preserve all existing skill-loop exceptions and the universal three-failure breaker.

## Dependency Graph

U1 blocks U2. No other backlog or shipment dependency exists. The security spike shipment precedes this shipment only because the operator ordered security/reliability first.

## Decisions and Rationale

Put the rule in the authoritative circuit-breaker instruction rather than the compound learning or individual skills. Distinguish diagnostic escalation from a blind retry without forgiving the underlying command failure. Capture full output only for a bounded invocation, and bound what the agent reads and retains. Do not add a general logging framework, runtime code, new CLI flags, or duplicate wording across skills.

## Constitution Check

- Test first: U1 must be observed RED before U2.
- Workspace/security containment: captures stay under workspace `logs/`; secrets/raw payloads are excluded.
- Structured observability: concrete errors and capture paths become checkpoint evidence.
- Single responsibility: one authoritative policy plus its contract harness.
- Two-hour/width limits: one test file and one instruction file in separate <=100-minute units.
- Circuit breaker: the universal threshold and skill-managed exceptions are preserved, not weakened.

## Risks and Caveats

Ambiguous wording could let agents reset retry counters, dump secrets, ingest megabyte logs into context, or retain logs forever. Overly rigid byte limits could truncate the very failure being sought. The policy therefore bounds the command, capture count, extraction, and retention while requiring the complete available output for that bounded invocation; if a size guard stops capture, the agent records that explicit truncation and halts rather than silently claiming completeness.

## Plan Hardening Signals

- Public API/schema change: absent.
- Shared agent contract change: present; all agent retry loops consume this instruction.
- Security-sensitive behavior: present at the logging boundary because output may contain secrets or payloads.
- Migration/destructive action: absent.
- High runtime/rollback risk: moderate; bad wording can weaken safety behavior across workflows.

Requires plan hardening: yes

## Runtime Verification and Closure

Run focused `contract_verify`, then Markdown/YAML/frontmatter validation and repository instruction cross-reference checks; finally run normal ordered quality gates in Ship. Prompt-authoring review manually exercises three examples: truncated test output, secret-bearing command output, and a third same-error recurrence. Healthy behavior escalates once, extracts a bounded concrete failure, checkpoints it, de-escalates, and still stops at three. Rollback trigger: any test or review shows counter reset, a fourth equivalent retry, out-of-workspace logging, raw secret persistence, unbounded append/retention, or failure to de-escalate. Rollback is a reviewed instruction/test revert. Observe the next three Ship sessions or seven days, whichever is longer, for blind-retry recurrence and accidental log commits.

## Plan Hardening

Hardening is required because a shared safety contract and diagnostic data boundary change.

- **Protected invariants:** universal threshold three; every underlying non-zero/timeout counts; no secret/raw payload persistence; workspace-only logs; native exit preserved; bounded extraction and retention; concrete error before mutation; normal verbosity restored.
- **Reinforcing evidence:** dynamic-diagnostic-escalation learning, 111-S circuit-break/audit checkpoints and closure, circuit-breaker, strict-safety, and constitution instructions.
- **ProposedAction:** amend the universal circuit-breaker prompt contract. **ActionRisk:** moderate. **approval_required:** no additional approval; operator explicitly requested the policy update. **rollback:** reviewed revert of the test and instruction. **ActionResult:** planned.
- **ProposedAction:** capture full command output during a future diagnosis. **ActionRisk:** high when output may contain sensitive data. **approval_required:** preflight classification is mandatory; known secret-bearing/raw-payload output must use a safe repro or tool-native redaction instead. **rollback:** stop capture, protect the artifact, and follow repository safeguards. **ActionResult:** planned as policy, not executed here.

## Plan Review

**Gate: PASS.** Hardening requirement is satisfied. Constitution, Rust/test, scope-boundary, learnings, architecture, and prompt/instruction-authoring personas reviewed both units.

- **P0:** 0.
- **P1:** 0.
- **P2:** 0.
- **P3:** 0.

Prompt review confirms imperative ordering, no unresolved template variables, valid frontmatter/Markdown expectations, preserved three-failure semantics, explicit information-loss handling, bounded evidence extraction, secret/raw-payload exclusions, and de-escalation. The plan is ready for harvest.