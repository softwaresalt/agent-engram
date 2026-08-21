---
title: Capability-rooted no-follow workspace identity and Git metadata resolution
date: 2026-08-21
type: implementation-plan
status: reviewed
source_stash_id: 568B257C
source: docs/decisions/2026-08-21-568b257c-workspace-identity-toctou-threat-model.md
agent: stage
---

## Problem Frame

`src/db/workspace.rs::resolve_git_metadata` proves native-worktree authenticity
through roughly twenty independent **path-based** filesystem resolutions
(`canonicalize`, `symlink_metadata`, `read_to_string`), each re-walking the full
path from the filesystem root. Validation and use therefore address potentially
different objects (threats T1–T3). `require_plain_directory`,
`require_plain_reference_storage`, and `read_metadata_file` gate on
`is_symlink()`, which does not cover every Windows reparse-point class (T4) —
even though the same file already applies the broader
`FILE_ATTRIBUTE_REPARSE_POINT` test to `.workspace-id`. Because
`load_or_create_workspace_id` and `daemon_key_for_workspace` build on this
result, a successful race poisons workspace identity and the daemon IPC key (T5).

## Requirements Trace

| Requirement (stash 568B257C) | Implementation action |
|---|---|
| Retained, capability-rooted directory handles | U3 introduces a retained-root resolver; U4 and U5 route git metadata and identity through it. |
| No-follow resolution | U3 opens every component with `FollowSymlinks::No` / `OFlags::NOFOLLOW`, one component at a time. |
| Close parent-directory swap TOCTOU | U1 (ancestor-swap harness) + U3/U4 handle-relative resolution. |
| Close metadata check/read TOCTOU | U1 (leaf-swap harness) + U4 reads content from the already-validated handle. |
| Cover symlinks | U2 Unix adversarial harness; U4 no-follow opens. |
| Cover Windows junctions / reparse points | U2 Windows adversarial harness; U6 uniform reparse rejection. |
| Add adversarial tests | U1 and U2 are dedicated adversarial harness units. |

## Implementation Units

### U1 — RED: TOCTOU adversarial harness (platform-neutral)

* Changes: new unit test harness that drives `resolve_git_metadata` against a
  fixture whose ancestor directory and whose metadata leaf are swapped at a
  controlled point during resolution, plus a handle-identity assertion.
* Files: `tests/unit/workspace_toctou_test.rs`, `Cargo.toml` (`[[test]]` entry).
* Tests: 3 scenarios — ancestor swap, leaf check/read swap, handle-identity
  equivalence between validated and admitted object.
* Posture: test-first (RED). Compiles, fails.

### U2 — RED: platform reparse/symlink adversarial harness

* Changes: `#[cfg(windows)]` scenarios substituting `worktrees`, `objects`, and
  `refs` with a junction and with a non-symlink reparse point; `#[cfg(unix)]`
  scenarios substituting the same with symlinks.
* Files: `tests/unit/workspace_reparse_test.rs`, `Cargo.toml`.
* Tests: 3 Windows scenarios, 2 Unix scenarios (compile-gated per platform).
* Posture: test-first (RED).

### U3 — GREEN: retained capability-root resolver

* Changes: introduce a private resolver type in `src/db/workspace.rs` that owns a
  `cap_std::fs::Dir` root handle and exposes component-at-a-time no-follow open,
  handle-derived metadata, and handle-backed file read. No call sites changed yet.
* Files: `src/db/workspace.rs`.
* Tests: U1 handle-identity scenario turns green.
* Posture: paired GREEN for U1.

### U4 — GREEN: route Git metadata resolution through retained handles

* Changes: rewrite `resolve_git_metadata` to use the U3 resolver — one retained
  root handle for the workspace, one for the linked-worktree admin root; all
  metadata reads served from already-validated handles; remove the
  `canonicalize` / `symlink_metadata` mixed-semantics comparisons.
* Files: `src/db/workspace.rs`.
* Tests: U1 ancestor-swap and leaf-swap scenarios turn green.
* Posture: paired GREEN for U1.

### U5 — GREEN: route identity persistence through retained handles

* Changes: `load_or_create_workspace_id` and `daemon_key_for_workspace` consume
  the retained root handle from U4 instead of re-resolving paths; `.engram/` and
  `.workspace-id` access becomes handle-relative.
* Files: `src/db/workspace.rs`.
* Tests: U1 handle-identity scenario extended to the identity path.
* Posture: paired GREEN for U1.

### U6 — GREEN: uniform reparse-point rejection

* Changes: replace the `is_symlink()`-only gates in `require_plain_directory`,
  `require_plain_reference_storage`, and the handle-backed metadata read with the
  broader `FILE_ATTRIBUTE_REPARSE_POINT` rejection already used by
  `is_workspace_id_link_or_reparse`, applied uniformly on Windows; keep the
  symlink rejection on Unix.
* Files: `src/db/workspace.rs`.
* Tests: U2 turns green on both platforms.
* Posture: paired GREEN for U2.

### U7 — Docs: security model record

* Changes: document the capability-rooted admission model, the cross-boundary
  admin-root exception for linked worktrees, and the invariants a future change
  must not break.
* Files: `docs/architecture.md`.
* Tests: none (documentation unit).
* Posture: docs-only.

### U8 — Runtime verification and closure

* Changes: runtime verification that a primary checkout and a native worktree are
  both still admitted on Windows and on Linux, with the adversarial suite green;
  operational closure record.
* Files: `docs/closure/2026-08-21-568b257c-runtime-verification.md`.
* Posture: verification-only.

## Dependency Graph

```text
U1 ─> U3 ─> U4 ─> U5 ─┐
U2 ─────────> U6 ─────┼─> U7 ─> U8
```

* U1 and U2 are parallel RED units.
* U3 depends on U1. U4 depends on U3. U5 depends on U4.
* U6 depends on U2 and on U3's handle-backed metadata accessor.
* U7 depends on U5 and U6. U8 depends on U7.

## Decisions and Rationale

1. **Capability handles over re-validation.** Re-checking after use narrows the
   window but cannot close it; only holding a handle to the validated object
   makes check and use address the same object.
2. **Component-at-a-time no-follow.** Full-path resolution is the vulnerability.
   `cap-std` already provides the primitive and is already a dependency, so no
   new dependency is introduced (Constitution Principle VI).
3. **Two retained roots, not one.** Linked-worktree admin metadata is outside the
   workspace root by design; pretending otherwise would either break worktrees or
   silently widen containment. An explicit second root makes the exception
   reviewable.
4. **Reparse breadth over symlink-only.** The file already applies the broader
   test for `.workspace-id`; applying it uniformly removes an internal
   inconsistency rather than inventing a new policy.
5. **Behavior preserved for legitimate inputs.** No admission that succeeds today
   for a genuine checkout or genuine worktree may start failing.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Handle-relative rewrite breaks legitimate worktree admission | U8 verifies primary checkout and native worktree on both platforms; U1 includes a no-regression scenario. |
| `cap-std` no-follow semantics differ across platforms | U2 asserts the *observable* rejection on each platform rather than assuming a shared mechanism. |
| Adversarial tests are inherently racy and could flake | U1 uses a deterministic swap point rather than timing; a test that can only fail by luck is not accepted. |
| Windows adversarial fixtures require privileges to create links | Junction creation does not require elevation; symlink scenarios that do are skipped with an explicit skip reason, never silently passed. |
| Performance regression from per-component opens | Admission runs once per process start; measured in U8, budget is the existing admission latency plus a small constant. |
| Scope creep into daemon IPC binding | Freeze-scope: this release unit edits `src/db/workspace.rs` and its tests only. |

## Plan Hardening Signals (REQUIRED)

* Public API, schema, or contract change: **present (internal)** — the private
  resolution contract inside `src/db/workspace.rs` changes; the public
  `canonicalize_workspace` signature does not.
* Security, auth, permission, or compliance-sensitive behavior: **present** —
  this is a security fix closing TOCTOU and link-substitution attacks on the
  trust boundary that gates all workspace filesystem access.
* Migration, backfill, destructive data/config action, or irreversible step:
  **absent** — no data mutation, no migration; `.workspace-id` semantics unchanged.
* External integration, operator checkpoint, or external dependency: **absent** —
  `cap-std` and `cap-fs-ext` are existing dependencies.
* High runtime, rollout, or rollback risk: **present** — admission gates every
  engram entry point; a false rejection makes the product unusable.

Requires plan hardening: **yes**

## Runtime Verification and Closure

| Unit | Runtime surface | Verification | Closure artifact |
|---|---|---|---|
| U4, U5 | Workspace admission and identity | Primary checkout and native worktree admitted on Windows and Linux; adversarial suite green | U8 closure record |
| U6 | Platform link handling | Junction, reparse, and symlink substitution all rejected | U8 closure record |
| U5 | Daemon IPC key derivation | Daemon key stable across restarts for an unchanged workspace | U8 closure record |

## Plan Hardening

Triggered by three hardening signals (internal contract change, security-sensitive
behavior, high runtime risk). Security-sensitive work — adversarial review
escalation applies.

### Protected Invariants

1. The object admitted as the workspace root is provably the same object that was
   validated, established via a retained handle rather than path equality.
2. No metadata value used in the worktree-authenticity proof is read from a path
   that was resolved a second time after its check.
3. On Windows, any validated directory or file carrying
   `FILE_ATTRIBUTE_REPARSE_POINT` is rejected regardless of reparse tag. On Unix,
   any symlink in the validated chain is rejected.
4. A legitimate primary checkout and a legitimate native `git worktree` are both
   still admitted on every supported platform.
5. The linked-worktree admin root is the only region outside the workspace root
   that is opened, and it is opened as an explicit, separately retained root.

### Risky Actions

| ProposedAction | targets | change_kind | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|---|
| Rewrite `resolve_git_metadata` onto retained capability handles | `src/db/workspace.rs` | security-critical local edit to the admission gate | high | revert U3–U5; admission returns to path-based resolution with the known TOCTOU exposure | prefer approval (high blast radius) | planned |
| Broaden Windows reparse rejection beyond `is_symlink()` | `src/db/workspace.rs` | security policy tightening | high | revert U6; risk of newly rejecting cloud-synced or container-isolated checkouts | prefer approval | planned |
| Route identity persistence through the retained root | `src/db/workspace.rs` | local edit affecting daemon IPC key derivation | moderate | revert U5 | no | planned |
| Create junction / reparse fixtures in adversarial tests | `tests/unit/` | test-only local file creation inside the test temp root | low | tests clean up their own fixtures | no | planned |

No `destructive` action is present. Two `high` actions are flagged for preferred
operator approval at implementation time per strict-safety guidance; neither is
irreversible.

### Adversarial Review Escalation

This release unit meets the escalation criteria in
`.github/instructions/adversarial-review.instructions.md`: the workspace is
security-sensitive and the diff touches the filesystem trust boundary. Ship MUST
escalate the code review for U4, U5, and U6 from standard review to adversarial
multi-model review, and MUST treat HIGH-confidence P0/P1 consensus findings as
gate-blocking.

### Reinforced Verification

* U1's ancestor-swap scenario MUST use a deterministic interception point. A test
  that depends on winning a timing race is rejected as non-deterministic.
* U2 MUST assert rejection, not merely "does not crash".
* Windows scenarios that genuinely require elevation MUST be skipped with an
  explicit, logged skip reason. A silently-passing skipped security test is
  treated as a failing test.
* U6 MUST include a no-regression case for a workspace on a path containing a
  legitimate reparse point *above* the workspace root, to confirm the policy is
  scoped to the validated chain and not to unrelated ancestors.
* U8 MUST measure admission latency before and after and record both.

### Monitoring Plan

| SLI | Where observed | Baseline | Alert threshold | Owner |
|---|---|---|---|---|
| Workspace admission success on legitimate roots | `engram workspace-status` on a checkout and a worktree | 100% | any `NotGitRoot` on a known-good root | Ship agent during validation window |
| Admission latency | U8 measurement, then daemon startup timing | current admission latency | > 2x baseline | Ship agent |
| Adversarial suite result | CI, per platform | all green | any failure or unexplained skip | CI |
| Daemon key stability | `engram daemon-status` across restarts | stable | key change without workspace change | Ship agent |

### Pre-Deploy Audit

* No feature flag: an admission gate cannot be safely half-enabled.
* Rollback procedure: revert U3–U6 commits. `.workspace-id` file format is
  unchanged, so no persisted state needs unwinding and rollback is clean.
* No migration and no schema change; `.workspace-id` remains readable by both the
  old and new code paths.
* Cross-platform: Windows and Linux verification are both required before merge.
* Dependent surfaces: daemon IPC endpoint derivation and `.engram/` state
  location both consume this result and are covered by the monitoring plan.

### Post-Deploy Observation Window

Duration: 48 hours across at least one Windows and one Linux session. Owner: Ship
agent through runtime verification, then operator. Outcome recorded in the U8
closure artifact as healthy, degraded, or rolled back.

### Rollback Triggers

1. Any legitimate checkout or native worktree is rejected with `NotGitRoot` —
   revert immediately.
2. Daemon discovery key changes for an unchanged workspace — revert U5.
3. Admission latency exceeds 2x baseline — investigate, revert if unresolved.
4. Any adversarial scenario regresses to passing-by-skip — treat as a failed gate.

## Plan Review

Gate: **PASS**

Personas dispatched: Security Lens (lead), Architecture Lens, Test Strategy Lens,
Operational Readiness Lens.

### Findings

| ID | Persona | Severity | Finding | Disposition |
|---|---|---|---|---|
| S1 | Security | P0 | Original scope described "no-follow handles" but did not require that *metadata* be derived from the open handle. Reopening by path for `fstat` would leave T2 open and the fix would be cosmetic. | Resolved: invariant 1 and U3's handle-derived metadata accessor are now explicit. Gate-clearing. |
| S2 | Security | P1 | `is_symlink()` breadth was described as a Windows junction gap. Rust's Windows `is_symlink()` does cover `MOUNT_POINT`, so a junction-only framing would have understated *and* misidentified the residual risk. | Resolved in the threat model (T4): the real gap is reparse tags outside `SYMLINK`/`MOUNT_POINT`, and the fix is the uniform `FILE_ATTRIBUTE_REPARSE_POINT` test the file already uses for `.workspace-id`. Gate-clearing. |
| S3 | Security | P1 | Broadening reparse rejection can newly reject legitimate checkouts under cloud-sync or container-isolation providers — a security fix that becomes an availability bug. | Resolved in hardening: U6 no-regression case for reparse points above the workspace root; rollback trigger 1. Gate-clearing. |
| S4 | Security | P2 | The linked-worktree admin root is outside the containment boundary; without an explicit statement this reads as a Principle III violation. | Resolved: decision 3 and invariant 5 make the exception explicit and reviewable. |
| A1 | Architecture | P2 | Threading a retained handle through `load_or_create_workspace_id` and `daemon_key_for_workspace` risks leaking `cap_std` types into wider call sites. | Accepted into U5 acceptance criteria: the resolver stays private to `src/db/workspace.rs`; public signatures unchanged. |
| T1 | Test Strategy | P1 | Ancestor-swap TOCTOU tests are classically flaky; a timing-based harness would produce a green suite that proves nothing. | Resolved in hardening: deterministic interception required, timing-race harnesses rejected. Gate-clearing. |
| T2 | Test Strategy | P2 | Windows link-creation privileges vary; a skipped security test could be mistaken for a passing one. | Resolved: explicit logged skip reason; silent skip treated as failure. |
| O1 | Operational Readiness | P2 | No rollback trigger covered the false-rejection failure mode, which is the most likely operational consequence. | Resolved: rollback trigger 1. |
| A2 | Architecture | P3 | `normalize_canonical` and `strip_extended_length_prefix` remain path-string operations after the rewrite. | Advisory; they operate on already-validated handle-derived paths and are out of scope. |

One P0 and three P1 findings were raised and all were resolved during hardening
before the gate decision. No unresolved P0/P1 remains. Decomposition satisfies
the 2-hour rule and width isolation: U1/U2 test-only, U3–U6 production code in a
single file, U7 docs-only, U8 verification-only.

Review-fix cycles used: 2 of 3.
