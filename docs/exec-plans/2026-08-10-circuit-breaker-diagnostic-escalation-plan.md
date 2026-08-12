---
title: "Circuit-breaker dynamic diagnostic escalation policy"
type: implementation-plan
date: 2026-08-10
source: docs/compound/workflow-issues/dynamic-diagnostic-escalation-2026-08-08.md
status: reviewed
source_stash_ids: [241B503F]
references:
  - docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md
  - .backlogit/archive/120.001-R-circuit-breaker-diagnostic-policy-plan-review.md
  - .backlogit/queue/116-S.md
  - templates/instructions/circuit-breaker.instructions.md.tmpl
  - .github/instructions/circuit-breaker.instructions.md
  - docs/memory/2026-08-11/pr-337-adversarial-remediation-memory.md
---

# Circuit-breaker dynamic diagnostic escalation policy

## Problem Frame

Transport truncation can hide a concrete failure, but diagnostic escalation must remain the next counted attempt and never reset, pause, split, or bypass `MAXIMUM_RETRY_THRESHOLD = 3`. The durable policy source is the autoharness product template `templates/instructions/circuit-breaker.instructions.md.tmpl`; `.github/instructions/circuit-breaker.instructions.md` is generated workspace output. A generated-only edit is invalid because supported harness regeneration would revert it.

This changes template/prompt and policy-contract artifacts only; it does not change Engram runtime behavior. No policy requirement or publication decision depends on a value in `.autoharness/config.yaml`.

## Requirements Trace

- Retry semantics: each non-zero/timeout counts; hidden failures use a provisional operation fingerprint; the diagnostic invocation occupies the next attempt; attempt three may be inspected but still trips the breaker; attempt four is forbidden; concrete identity links prior attempts without recounting; genuinely different observable errors retain the skill-loop exception.
- Durability: U2 edits the authoritative template first, then synchronizes the generated instruction through the supported harness render path and proves a clean rerender is byte-stable and does not remove the policy.
- Contract cohesion: U1 uses a dedicated `contract_circuit_breaker_policy` target at `tests/contract/circuit_breaker_policy_contract_test.rs`, not the unrelated CLI `contract_verify` target.
- CI hygiene: any new/changed workflow is least-privilege, uses repository-required SHA pins, and sets checkout `persist-credentials: false`.
- Operational order: `116-S` remains blocked by `115-S` shipment/archive/merge evidence and by the separately recorded successful cleanup approval/result/verification.

## Retry and Identity Contract

1. Record each non-zero exit or timeout immediately against the current operation.
2. While details are hidden, fingerprint normalized command/target, working directory, relevant environment, and workflow phase; conservatively classify another failed invocation of that fingerprint as substantially the same failure.
3. If the threshold is not reached, change diagnostic transport on the next invocation. That invocation consumes the next attempt and increments the same counter when non-zero/timeout. No reset, pause, parallel counter, or fourth run exists.
4. Inspect already captured evidence from failed attempt three, then trip the breaker; inspection is not re-execution.
5. Record one concrete identity from stable native exit/timeout, target/code, normalized message, affected path, and phase. Link provisional attempts without recounting. Hidden output never manufactures a different-error exception.
6. Stop high-volume capture immediately after diagnosis. De-escalation changes verbosity only.

## Implementation Units

### U1 — Add the RED dedicated policy contract

**Domain/files:** `tests/contract/circuit_breaker_policy_contract_test.rs`, the cohesive `[[test]] name = "contract_circuit_breaker_policy"` entry in `Cargo.toml`, and `.github/workflows/circuit-breaker-contract.yml`. **Cap:** 110 minutes, three scenarios, no production code.

Add RED repository-content scenarios for the full Retry and Identity Contract, bounded workspace logs, native exit, secret/raw-payload exclusion, bounded extraction/retention, and immediate de-escalation. The target is dedicated because `tests/contract/verify_test.rs`/`contract_verify` exercises the Engram CLI verify exit-code contract and is unrelated to harness policy content. Observe the dedicated target RED before U2.

The narrow workflow runs only `cargo test --test contract_circuit_breaker_policy` for changes to the authoritative template reference, generated instruction, dedicated test, Cargo target declaration, or workflow. Give it only required read permissions. Use SHA-pinned actions when repository policy requires pins; for the current repository convention, pin every action by full commit SHA. Set `actions/checkout` `persist-credentials: false`. Do not imply that the broad Rust workflow covers Markdown instruction changes.

### U2 — Update source template, synchronize generated output, and prove drift resistance

**Domain/files:** authoritative `templates/instructions/circuit-breaker.instructions.md.tmpl` in the autoharness source resolved from `autoharness home`, generated `.github/instructions/circuit-breaker.instructions.md` in the target workspace, and the harness manifest checksum only if the supported renderer requires it. **Cap:** 110 minutes, one template family/concern.

Apply this exact order:

1. Resolve and record `autoharness_home` via `autoharness home`; verify the authoritative template exists there. Record baseline bytes/SHA-256 for the template, generated instruction, and any manifest entry the renderer owns. Do not read policy semantics from or modify `.autoharness/config.yaml`; fail if the render proposes unrelated config or artifact drift.
2. Edit `templates/instructions/circuit-breaker.instructions.md.tmpl` first. Put all Retry and Identity Contract wording there. Do not hand-edit only the generated instruction.
3. Invoke the supported `tune-harness` skill with that `autoharness_home`, the current target `workspace_path`, `scope: instructions`, and the accepted circuit-breaker-only proposal. Permit only the generated instruction and renderer-owned manifest checksum to change in the target workspace.
4. Verify template and generated instruction both contain the contract, valid frontmatter/Markdown, no unresolved placeholders, bounded logging/sensitive-output safeguards, and identical intended semantics. Then run the dedicated target GREEN.
5. Perform the clean regeneration drift test through the supported harness path: render the instruction again from the changed template into clean harness staging (the `install-harness` dry-run/staging path or renderer-equivalent clean staging mode), compare staged bytes/SHA-256 with the checked-in generated instruction, and run the circuit-breaker-only tune render a second time. The staged/generated hashes must match, the second render must produce no policy or generated-file diff, and the policy text must remain. Any reversion, unrelated drift, or config-dependent result fails closure.

If source and generated changes require different repositories, record both repository-relative paths and commits in closure; do not pretend a downstream generated-only PR is durable. The template remains the source of truth.

## Dependency Graph

U1 blocks U2. Shipment `116-S` remains batch `dark-factory-2026-08-10`, order 2, predecessors `[115-S]`, with hard dependency `116-S -> 115-S`. It is technically independent of the spike recommendation, but not operationally claimable until `115-S` is shipped, shipment/items archived, merge evidence recorded, and the exact post-spike cleanup is approved, successful, and verified. If cleanup is not approved or fails, `116-S` stays blocked.

## Decisions and Rationale

- Put policy in the authoritative template and treat the workspace instruction as generated output.
- Use supported `tune-harness` synchronization plus clean staging/idempotence proof; never validate a hand-copied generated edit as durable.
- Use a dedicated policy contract target because `contract_verify` is an Engram CLI contract.
- Add a narrow workflow because broad CI ignores instruction Markdown; enforce least privilege and credential/pin hygiene.
- Keep policy semantics independent of unrelated `.autoharness/config.yaml` values.

## Plan Hardening

Hardening is required because this is a shared safety contract, generated-artifact durability boundary, CI workflow change, and ordered operational gate.

- **ProposedAction:** amend the authoritative circuit-breaker template and regenerate target output. **ActionRisk:** high shared-contract change. **approval_required:** implementation review and drift proof. **ActionResult:** planned.
- **ProposedAction:** add a narrow GitHub workflow. **ActionRisk:** moderate. **approval_required:** least-privilege/SHA-pin/credential review. **ActionResult:** planned.
- **Protected invariants:** threshold three, counted diagnostics, no fourth run, source-template authority, deterministic clean regeneration, no unrelated config dependence, bounded sensitive logging, dedicated contract target, and extended predecessor cleanup gate.

## Runtime Verification and Closure

Run targeted supported validation: dedicated policy contract, template/generated frontmatter and placeholder checks, reference existence (resolving `templates/...` against `autoharness_home`), workflow YAML/security inspection, supported clean staging hash comparison, second-render idempotence, and `git diff --check`. The known broad `autoharness verify-workspace` strict-schema blockers are a baseline issue and are not evidence against this targeted policy batch.

Rollback reverts template first, then reruns supported synchronization and repeats the clean staging/hash test. Never revert only the generated instruction. Observe the next three Ship sessions or seven days for blind retries, accidental fourth attempts, regeneration drift, and accidental log commits.

## Historical Plan Review

The 2026-08-10 review and first 2026-08-11 focused re-review are preserved in `120.001-R`. They predate the generated-template, dedicated-target, CI-hygiene, and cleanup-result findings and do not by themselves authorize execution.

## Focused Plan Re-review — 2026-08-11 (second remediation)

**Stage gate: PASS; final adversarial rerun still required.** Constitution, scope, architecture, template-authoring, prompt/instruction, CI security, test, durability, and operational-sequencing lenses found P0 0, P1 0, P2 0, P3 0 in this final Stage contract. The gate confirms template-first supported regeneration, clean staging/idempotence proof, the dedicated policy target, least-privilege workflow requirements, config independence, exact retry semantics, and the extended cleanup-result predecessor. It does not claim the final adversarial rerun has passed.
