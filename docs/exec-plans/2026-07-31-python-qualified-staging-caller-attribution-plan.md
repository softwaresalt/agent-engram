# Python qualified-staging caller attribution fail-closed plan

Origin: `docs/decisions/2026-07-31-python-qualified-staging-caller-attribution-decision.md`  
Source stash: `42FB7CC5`  
Planning state: hardened; plan-review PASS

## Problem Frame

PR #301 promoted provable function-local Python imports into qualified `python_local` staging. Both staging producers in `src/services/code_graph.rs` still resolve the caller name with first-match `find_function_id`: the full-index path near the extracted-edge loop and the incremental-sync path near its mirrored loop. Duplicate top-level caller names make the edge origin ambiguous even when the canonical target is trusted. Staging under the first duplicate can mint a wrong-origin canonical edge.

The existing bare-call flow already solves the same origin-identity problem with typed `find_unique_function_id`. The fix is to preserve that fail-closed caller guard at the two qualified/provenance staging sites without changing target resolution, schemas, keys, or global helper semantics.

## Requirements Trace

| Requirement | Implementation action | Verification |
|---|---|---|
| No arbitrary duplicate caller attribution | Use `find_unique_function_id` at both qualified staging producers | Duplicate-caller target-identity fixtures show no staged or resolved wrong-origin edge |
| Preserve unique-caller behavior | Stage only `Unique(from_id)` exactly as today | Unique-caller control keeps the exact canonical target |
| Keep index/sync symmetric | Apply the same typed match at both mirrored sites | One full-index scenario and one incremental-sync scenario |
| Preserve observability | Increment `same_file_ambiguous_dropped` for an ambiguous qualified caller | Assert the result counter where the harness exposes it |
| Avoid broad semantic change | Leave `find_function_id`, staged-call schema/key, and target resolver unchanged | Existing unique controls and code review |

## Prior Learnings Consulted

- `docs/compound/bugs/same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md`: first-match identity is unsafe when duplicate definitions exist; fail closed.
- `100-F` / `100.002-T`: use the additive typed `find_unique_function_id` helper and keep `find_function_id` byte-identical for unrelated consumers.
- `docs/decisions/2026-07-29-python-last-wins-recall-recovery-decision.md` (`016-D`): reject blanket last-wins; zero false edges governs and duplicate-name recall remains a non-goal.
- `013-D` and `082-F`: target correctness takes precedence over speculative recall.

## Decisions and Rationale

1. Reuse the existing typed helper rather than add a new caller-specific resolver. It already expresses `Unique`, `Ambiguous`, and `NotFound` and is proven at the adjacent bare-call sites.
2. Fail closed for every language flowing through the shared qualified staging branch. An ambiguous origin is unsound regardless of parser; the reported regression and required fixture remain Python-specific.
3. Count only `Ambiguous` as an ambiguity drop. `NotFound` retains the current silent no-stage behavior.
4. Keep the two mirrored index/sync edits in one code-only unit because they are one invariant in one file and fewer than five function sites.
5. Keep the regression harness separate from production code to preserve width isolation and the RED-before-GREEN milestone.

## Implementation Units

### U1 — Duplicate-caller qualified-staging regression harness (tests only)

Execution posture: test-first / RED.

- Extend the existing `tests/integration/same_file_shadowing_acceptance_test.rs` harness, which already exercises full-index and sync duplicate-name ambiguity and exposes `same_file_ambiguous_dropped`.
- Scenario 1: full index of a Python corpus with duplicate top-level caller names and a provable function-local import call. Assert target identity: neither duplicate caller owns a canonical edge to the trusted target; no arbitrary staged provenance survives.
- Scenario 2: incremental sync introduces or changes the duplicate-caller qualified call. Assert no staged row is keyed to either duplicate caller and the ambiguity-drop result is observable.
- Scenario 3: a unique-caller control still stages/resolves the exact canonical target in the corresponding paths.
- Compile and fail for the expected wrong-origin attribution before the production fix; no sleeps or timing dependence.

Width check: one test file, at most three scenarios, tests domain only, approximately two hours.

### U2 — Ambiguity-aware qualified caller staging (code only)

Execution posture: GREEN implementation.

- In `src/services/code_graph.rs`, replace first-match caller attribution at the full-index qualified/provenance staging site with a match on `find_unique_function_id`.
- Mirror the same change in the incremental-sync site.
- `Unique(from_id)`: execute the existing enclosing-type calculation and staged-call write unchanged.
- `Ambiguous`: increment `same_file_ambiguous_dropped` and do not stage.
- `NotFound`: do nothing, preserving current behavior.
- Do not change the implementation semantics of `find_function_id` or `find_unique_function_id`, the staged-call relation/key, Python target resolution, or extraction-version behavior. Update the nearby helper and call-site doc comments that currently describe ambiguity-aware lookup as direct-edge-only or qualified caller attribution as intentionally first-match, so documentation matches the new four-site use.
- Make U1 GREEN and preserve the unique-caller control.

Width check: one source file, two mirrored sites, code-graph domain only, approximately two hours.

## Dependency Graph

`U1 (RED regression harness) -> U2 (GREEN staging guard)`

No external or cross-feature dependency is required.

## Risks and Caveats

- Fail-closed behavior intentionally drops recall for a rare ambiguous caller; this is the existing `016-D` policy, not a new trade-off.
- Fixing only one producer would create index/sync divergence; both sites are release-blocking.
- A test that checks only edge row existence can miss wrong-origin attribution; fixtures must assert exact caller and target IDs.
- If PR #301 behavior has already shipped in a published binary, persisted wrong-origin edges may require a full reindex to be removed. Ship must check release exposure before merge and add an operator reindex checkpoint if exposure exists.
- The incremental sync path defers canonical post-pass work, so its regression should inspect staged provenance rather than expect immediate canonical resolution.

## Plan Hardening Signals

- Public API, schema, or contract change: absent. Internal edge-admission behavior only; no exposed request/response or storage shape changes.
- Security, auth, permission, or compliance-sensitive behavior: absent.
- Migration, backfill, destructive data/config action, or irreversible step: absent in the planned change. Conditional reindex is an operator checkpoint only if an affected build was already released.
- External integration, operator checkpoint, or external dependency: conditional. Release exposure check may require an operator reindex checkpoint.
- High runtime, rollout, or rollback risk: present. The change controls persisted call-graph edge origin and must preserve zero false edges across two producers.

Requires plan hardening: yes

## Runtime Verification and Closure

Runtime surface: persisted code-graph call edges produced by full index and incremental sync.

Before closure, Ship must prove on a temporary Python corpus:

1. Full index produces zero wrong-origin canonical edges for duplicate callers.
2. Incremental sync produces no arbitrary staged caller row.
3. A unique caller still resolves the exact canonical target.
4. The ambiguity-drop signal increases for the duplicate-caller case.

Monitoring plan:

- SLI: wrong-origin edges on the adversarial target-identity corpus. Baseline before fix: one arbitrary edge is possible; healthy after fix: zero.
- SLI: unique-caller canonical edge count. Healthy: unchanged and non-zero.
- Signal/query: targeted integration output plus staged/canonical edge query during runtime verification; no external dashboard exists.
- Alert/rollback threshold: any wrong-origin edge, any index/sync asymmetry, or loss of the unique-caller control.
- Owner and window: Ship/operator for the merge verification and the first full-index plus incremental-sync cycle after merge; record the result in operational closure.
- Rollback: revert the two qualified-staging caller-lookup substitutions. No schema or data-format rollback is required. If an affected build was released, force a full reindex after rollback or correction to refresh persisted edges.

## Plan Hardening

Hardening is required because this narrow change alters persisted runtime call-graph admission and a one-sided fix could create producer divergence. The plan is constrained to one test file and one production file and protects these invariants: exact caller identity, exact target identity, index/sync symmetry, unique-caller recall, and unchanged storage shape.

Reinforcing guidance consulted: strict-safety instructions, release-observability instructions, `013-D`, `016-D`, `100-F`, and the same-file first-match compound learning.

ProposedAction: replace first-match caller attribution with typed unique-only attribution at the two qualified staging producers.  
ActionRisk: moderate — shared runtime code-graph behavior changes, but no public contract, schema, migration, or destructive action.  
Approval required: yes; the operator explicitly approved non-destructive Stage planning/backlog mutation in the request.  
Rollback: revert the two guarded call-site edits and reindex any exposed affected workspace.  
ActionResult: approved for planning; implementation remains Ship-owned.

Additional guardrails:

- U1 must be RED for the expected first-match attribution, then GREEN only after U2.
- Review must reject any last-wins inference, global `find_function_id` semantic change, schema/key change, or single-path fix.
- Release exposure is a Ship pre-merge checkpoint. If an affected binary has shipped, the closure plan must include a full reindex instruction and verification; otherwise record that no backfill is needed.
- No destructive commands or automatic data migration are part of this shipment.

## Plan Review

Gate decision: **PASS** after one review-fix cycle.  
Review date: 2026-07-31  
Hardening required: yes; satisfied by the `Plan Hardening` section and strict-safety action record.

### Persona results

- Constitution Reviewer: PASS. U1 is a tests-only RED milestone; U2 is a code-only GREEN milestone. Both satisfy the two-hour rule, width isolation, TDD order, and no-unsafe/error-handling constraints.
- Rust Reviewer: PASS after remediation. Cycle 1 found that the initial plan prohibited touching helper documentation even though current comments say ambiguity-aware lookup is direct-edge-only and caller attribution remains first-match. The plan now requires comment-only corrections while keeping helper implementations unchanged.
- Scope Boundary Auditor: PASS. Scope is one existing integration-test file plus one production file; no schema, CLI, daemon, migration, or unrelated PR-review residue is included.
- Learnings Researcher: PASS after remediation. `100-F`, `016-D`, and the same-file first-match compound learning all support reuse of `find_unique_function_id` and reject last-wins. Cycle 1 advised extending the existing `same_file_shadowing_acceptance_test.rs` harness instead of creating a parallel fixture; the plan now does so.
- Architecture Strategist: PASS. Full-index and sync producers are treated symmetrically, staged-call persistence remains unchanged, and the existing ambiguity counter supplies observability.
- Agent-Native Parity Reviewer: not triggered; no MCP/CLI surface or user-agent parity contract changes.
- Security Lens Reviewer: not triggered; no trust-boundary, auth, secret, or external-integration change.

### Findings

#### P0

None.

#### P1

- Resolved in cycle 1: stale helper/call-site documentation would have contradicted the expanded ambiguity-aware usage. U2 now includes same-file comment maintenance without changing helper semantics.

#### P2

- Resolved in cycle 1: avoid a duplicate test harness. U1 now extends `tests/integration/same_file_shadowing_acceptance_test.rs`, which already covers index/sync ambiguity and exposes the result counter.

#### P3

None.

### Gate rationale

The reviewed plan preserves the governing zero-false-edge invariant, asserts exact caller and target identity, covers both producers, preserves a unique-caller control, and supplies monitoring, rollback, ownership, and a release-exposure checkpoint. No unresolved P0 or P1 finding remains. The plan is ready for harvest.
