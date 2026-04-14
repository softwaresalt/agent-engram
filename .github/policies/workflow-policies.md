---
description: Workflow policy registry for the agent-engram harness — defines cross-agent sequencing, gate conditions, and violation handling
applyTo: '**'
---

# Workflow Policy Registry

**Version**: 1.0.0 | **Ratified**: 2026-03-31 | **Source**: `.backlog/research/Agent-Harness-Evaluation-Report.md §8`

This registry is the authoritative source for cross-agent workflow policies in the engram harness. Each policy declares the agents it governs, the gate point where it is enforced, the preconditions that must be true before the agent may act, the postconditions that must hold before control advances, and the action to take on violation.

Agents must read this file at each declared gate point and enforce the relevant policy before proceeding. Policy compliance is non-negotiable.

---

## P-001: Single-Feature Completion

| Field | Value |
|---|---|
| Policy ID | P-001 |
| Version | 1.0.0 |
| Applies To | `build-orchestrator` |
| Gate Point | Pre-flight (Step 1) |

**Statement**: The build-orchestrator must complete one feature through to PR merge before starting a new feature. Parallel in-flight features on separate branches create branch conflicts, context fragmentation, and agent interference.

**Precondition**: No backlog tasks with status `In Progress` exist under any feature number other than `${input:feature}`.

**Postcondition**: All tasks under the current feature are `Done`, the PR is created and merged (or explicitly parked by the operator), before the orchestrator claims work on a new feature.

**Enforcement**: At Step 1 of the pre-flight sequence, call `backlog-task_list` with `status: "In Progress"`. For any returned tasks whose ID prefix does not match `TASK-${input:feature}`, this policy is violated.

**Violation Action**:

```
broadcast(error, "[POLICY] P-001 violated — feature {other_feature} has {count} task(s) In Progress.
  Complete or explicitly park feature {other_feature} before starting feature ${input:feature}.
  In Progress tasks: {task_id_list}")
```

Halt. Do not proceed until the operator resolves the conflict or explicitly overrides with `skip_policy: P-001`.

**Override Condition**: The operator may pass `skip_policy: "P-001"` as an explicit input to the orchestrator. When present, log the override as a policy violation event in the broadcast stream and continue.

---

## P-002: TDD Gate (Harness-Ready Precondition)

| Field | Value |
|---|---|
| Policy ID | P-002 |
| Version | 1.0.0 |
| Applies To | `build-orchestrator` (consumer), `harness-architect` (producer) |
| Gate Point | Queue building (Step 2) and task claiming (Step 3) |

**Statement**: The build-orchestrator may only claim and implement a task after the harness-architect has confirmed that the test harness compiles and all tests fail in the red phase. This enforces the TDD mandate (Constitution Principle III) as a machine-checkable gate rather than a convention.

**Precondition** (build-orchestrator): The task carries the `harness-ready` label in the backlog.

**Postcondition** (harness-architect): The task has a `harness-ready` label, an implementation note with the harness command, and the harness-architect has verified `cargo check` passes and all tests fail with `unimplemented!()`.

**Enforcement** (harness-architect): After operator approval and red-phase confirmation in Step 6, call `backlog-task_edit` with `labels: ["harness-ready"]` for each task. The label is the TDD gate seal.

**Enforcement** (build-orchestrator): When building the ready queue in Step 2, filter to only tasks carrying the `harness-ready` label. Tasks with `To Do` status but without `harness-ready` are not yet implementable.

**Violation Action** (build-orchestrator):

```
broadcast(warning, "[POLICY] P-002 — No harness-ready tasks in feature ${input:feature} queue.
  The harness-architect must generate and confirm test harnesses before implementation can begin.
  Run: harness-architect feature=${input:feature}")
```

Halt and suggest running the harness-architect.

**Violation Action** (harness-architect): If the harness fails compilation or the red phase cannot be confirmed after 3 attempts, do NOT apply the `harness-ready` label. Broadcast the failure and halt. The build-orchestrator will not be able to claim the task until the harness is corrected.

---

## P-003: Decomposition Chain Integrity

| Field | Value |
|---|---|
| Policy ID | P-003 |
| Version | 1.0.0 |
| Applies To | `backlog-harvester` |
| Gate Point | Pre-harvest validation (before Step 3.1) |

**Statement**: Each stage of the decomposition chain must reference its parent artifact and pass structural validation before the next stage may proceed. Research → Plan → Feature Epic → Sub-Epics → Tasks is a directed, validated pipeline, not a freeform decomposition.

**Precondition**: Before harvest begins, the following structural requirements must pass:

1. The source document exists at the declared path.
2. If a plan was generated (Phase 1), the plan file contains a reference back to the source document path.
3. Every sub-epic candidate derived from the plan references the plan file and the feature epic ID.
4. Every task candidate references its parent sub-epic.
5. Every Level 3 task includes at least one acceptance criterion.

**Postcondition**: All created backlog tasks have a non-empty `description`, a `parentTaskId`, and at least one acceptance criterion or reference to the source document.

**Enforcement**: Run the decomposition chain validation check (Step 3.0) before creating any backlog entries. Report the validation results in the broadcast stream.

**Violation Action**:

```
broadcast(error, "[POLICY] P-003 violated — decomposition chain broken at stage: {stage}.
  Failure: {reason}
  Fix the source artifact and re-run the harvester.")
```

Halt. Do not create partial task hierarchies with broken lineage.

---

## P-004: Red Phase Before Implementation

| Field | Value |
|---|---|
| Policy ID | P-004 |
| Version | 1.0.0 |
| Applies To | `harness-architect` |
| Gate Point | Step 6 (Operator Approval Gate) |

**Statement**: The harness-architect must confirm the red phase — all harness tests compile and fail with `unimplemented!()` panics — before the `harness-ready` label is applied. A harness that compiles but has no failing tests, or fails with compilation errors rather than runtime panics, does not satisfy the TDD gate.

**Precondition**: `cargo check --tests` exits 0 AND `cargo test --test {feature}_test` exits non-zero with `unimplemented!` in the output for every test function.

**Postcondition**: The harness manifest records `Compilation: PASS` and `Red Phase: CONFIRMED`.

**Enforcement**: Run both checks in Step 5 (Generate the Harness) before requesting operator approval. Do not request approval unless both checks pass.

**Violation Action**:

```
broadcast(error, "[POLICY] P-004 violated — harness does not meet red-phase criteria.
  Compilation: {PASS|FAIL}
  Red phase: {CONFIRMED|NOT CONFIRMED — reason: {reason}}
  harness-ready label NOT applied. Fix harness before re-submitting.")
```

---

## P-005: Policy Violation Telemetry

| Field | Value |
|---|---|
| Policy ID | P-005 |
| Version | 1.0.0 |
| Applies To | All agents governed by this registry |
| Gate Point | Any policy violation event |

**Statement**: All policy violations must be recorded as structured broadcast events. Policy violations are first-class observability signals and must be surfaced in the Slack channel (via agent-intercom), included in the PR description as compliance annotations, and noted in the memory checkpoint for the affected task.

**Precondition**: A policy gate has been triggered and found to be violated.

**Required Broadcast Format**:

```
broadcast(error, "[POLICY] {policy_id} violated — {one-line summary}")
broadcast(info,  "[POLICY] {policy_id} context — {details and recommended remediation}")
```

**Required PR Annotation**: When a policy was violated and overridden during a build session, the PR description must include a `## Policy Compliance` section noting which policies were violated, the override rationale, and the operator who approved the override.

**Required Memory Checkpoint Entry**: Include a `policy_violations` field in the memory checkpoint for any task where a policy gate was triggered, whether it halted execution or was overridden.

---

## Amendment Log

| Version | Date | Change | Reason |
|---|---|---|---|
| 1.0.0 | 2026-03-31 | Initial registry | Implements §8 Workflow Policy Primitive from Agent Harness Evaluation Report |
