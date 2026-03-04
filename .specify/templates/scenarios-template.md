---
description: "Behavioral matrix template for feature scenario coverage"
---

# Behavioral Matrix: [FEATURE NAME]

**Input**: Design documents from `/specs/[###-feature-name]/`
**Prerequisites**: spec.md (required), plan.md (required), data-model.md, contracts/
**Created**: [DATE]

**Note**: This template is filled in by the `/speckit.behavior` command. See the `speckit.behavior` agent for the execution workflow.

## Summary

| Metric | Count |
|---|---|
| **Total Scenarios** | [N] |
| Happy-path | [n] |
| Edge-case | [n] |
| Error | [n] |
| Boundary | [n] |
| Concurrent | [n] |
| Security | [n] |

**Non-happy-path coverage**: [X]% (minimum 30% required)

<!-- 
  ============================================================================
  IMPORTANT: The scenarios below are SAMPLE ROWS for illustration purposes only.
  
  The /speckit.behavior command MUST replace these with actual scenarios based on:
  - User inputs, arguments, flags, and system states from spec.md
  - Technical architecture and component boundaries from plan.md
  - Entity definitions and state transitions from data-model.md (if exists)
  - API contracts and endpoint signatures from contracts/ (if exists)
  
  Scenarios MUST be:
  - Deterministic (exactly one expected outcome per input state)
  - Grouped by component or subsystem
  - Translatable into parameterized tests (e.g., Rust #[rstest] blocks)
  
  DO NOT keep these sample scenarios in the generated SCENARIOS.md file.
  ============================================================================
-->

## [Component / Subsystem 1]

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S001 | [Brief human-readable description] | [Preconditions and input values] | [Command, flag, API call, or user action] | [What should happen on success] | [Post-condition or exit code] | happy-path |
| S002 | [Description of an edge case] | [Edge-case preconditions] | [Trigger] | [Expected behavior] | [State / exit code] | edge-case |
| S003 | [Description of an error condition] | [Error preconditions] | [Trigger] | [Expected error output] | [Error state / non-zero exit] | error |

---

## [Component / Subsystem 2]

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S004 | [Description] | [Input state] | [Trigger] | [Expected output] | [State / exit code] | happy-path |
| S005 | [Boundary value test] | [Boundary input: empty, max-length, zero, negative] | [Trigger] | [Expected behavior at boundary] | [State / exit code] | boundary |
| S006 | [Concurrent access pattern] | [Concurrent preconditions] | [Parallel triggers] | [Expected concurrent behavior] | [State / exit code] | concurrent |
| S007 | [Security scenario] | [Unauthorized / malicious input] | [Trigger] | [Expected rejection or sanitization] | [State / exit code] | security |

---

[Add more component sections as needed, following the same table structure]

---

## Edge Case Coverage Checklist

The following categories must be addressed across all components:

- [ ] Malformed inputs and invalid arguments
- [ ] Missing dependencies and unavailable resources
- [ ] State errors and race conditions
- [ ] Boundary values (empty, max-length, zero, negative)
- [ ] Permission and authorization failures
- [ ] Concurrent access patterns (where applicable)
- [ ] Graceful degradation scenarios

## Cross-Reference Validation

- [ ] Every entity in `data-model.md` has at least one scenario covering its state transitions
- [ ] Every endpoint in `contracts/` has at least one happy-path and one error scenario
- [ ] Every user story in `spec.md` has corresponding behavioral coverage
- [ ] No scenario has ambiguous or non-deterministic expected outcomes

## Notes

- Scenario IDs are globally sequential (S001, S002, …) across all components
- Categories: `happy-path`, `edge-case`, `error`, `boundary`, `concurrent`, `security`
- Each row must be deterministic — exactly one expected outcome per input state
- Tables are grouped by component/subsystem under level-2 headings
- Scenarios should map directly to parameterized test cases during implementation
