---
title: "Prove Windows canonical name and CapRoot identify one object"
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: reviewed-ready
source: docs/decisions/2026-08-24-windows-caproot-object-identity-spike.md
source_stash_id: "1C2A3CB3"
---

# Prove Windows canonical name and CapRoot identify one object

## Problem Frame

`CapRoot::prove_names_same_object` compares `(dev, ino)` on Unix but is a no-op on Windows. The spike found a safe stable route: public `cap_fs_ext::MetadataExt` exposes `dev()` and `ino()` for handle-derived capability metadata in pinned 4.0.2.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| Safe Rust only | Use public `cap_fs_ext::MetadataExt`; no raw handles or internal trait import. |
| Same-object proof on Windows | U2 compares identities from retained and named `CapRoot` handles. |
| Precise RED | U1 directly asks the private proof to compare two different directories on Windows. |
| Platform honesty | U3 records filesystem type and the ReFS 128-bit identifier caveat. |

## Implementation Units

### U1 — RED: Windows different-object policy test

Add one `#[cfg(windows)]` colocated test in `src/db/workspace.rs`: open a retained `CapRoot` for directory A, invoke `prove_names_same_object` with canonical directory B, and require rejection. In the same Windows-gated harness, call public `cap_fs_ext::MetadataExt::{dev, ino}` on the real value returned by `Dir::dir_metadata()` and assert the identity tuple can be formed without raw handles, private traits, `unsafe`, `unwrap`, `expect`, or production panic paths. This compile-and-behavior assertion is part of the gate: if the exact pinned 4.0.2 trait/type chain does not compile on Windows, the release unit fails closed rather than substituting an internal API. The current Windows no-op must fail the different-object assertion. This avoids a flaky rename race and directly tests the policy decision. One file, one scenario plus one API compile assertion, target 75 minutes.

### U2 — GREEN: cross-platform handle identity

Refactor `CapRoot::object_identity` to use public `cap_fs_ext::MetadataExt::{dev, ino}` over the exact `Dir::dir_metadata()` return type on supported platforms; remove the Windows no-op from `prove_names_same_object`. The Windows U1 compile assertion must prove the public trait/type bridge against pinned 4.0.2 before GREEN is accepted. Fail closed on identity-read/open errors and on inability to compile that public route. Do not import `_WindowsByHandle`, borrow raw handles, add unsafe, introduce panic-based production handling, or upgrade dependencies. U1 turns green and Unix behavior remains equivalent. One file, two functions, target 90 minutes.

### U3 — Windows verification and closure

Run the targeted test, workspace TOCTOU/reparse suites, primary/worktree admission, and CI. Record the test volume filesystem, concrete observation query/dashboard location, and measured admission/latency baseline. NTFS is the required gate. Record ReFS 128-bit file-ID truncation as a residual unless separate evidence proves the 64-bit pair sufficient. Verification only, target 90 minutes.

## Dependency Graph

`U1 -> U2 -> U3`. No dependency on the bind/daemon composition plans, but all share the adversarial review gate.

## Decisions and Rationale

- The spike supersedes the earlier safe-API blocker; `cap-fs-ext` is already pinned and public.
- Direct different-object comparison is the deterministic RED; Windows open-handle rename semantics make a race fixture unreliable.
- Preserve Unix behavior while sharing one capability extension trait.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| ReFS identity truncation | Do not claim ReFS closure; record filesystem and residual. |
| Hidden panic in dependency implementation | Only use metadata from an open directory handle, the precondition documented by `cap-fs-ext`. |
| Dependency/API drift | Keep exact 4.0.2 pin and compile on Windows/Linux. |

## Plan Hardening Signals

- Public API/schema/contract: absent.
- Security-sensitive behavior: present; trust-boundary same-object proof.
- Migration/destructive action: absent.
- External integration/checkpoint: present; Windows filesystem verification.
- High runtime/rollback risk: moderate; false mismatch rejects all workspaces.

Requires plan hardening: yes

## Runtime Verification and Closure

Verify primary and linked worktree admission on NTFS and Linux, with no new skip. Roll back U2 on legitimate `NotGitRoot`. Observation owner: Ship; window: 48 hours. Metrics: admission success 100%, no daemon-key change, latency within the 121-S absolute budget.

## Plan Hardening

| ProposedAction | targets | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|
| Enable Windows same-object rejection | `src/db/workspace.rs` | high | revert U2 to Windows residual | preferred | planned |

Protected invariants: handle-derived metadata only, compile-proven public `cap_fs_ext` trait/type bridge on the real Windows `dir_metadata()` result, fail closed, no unsafe/private/raw-handle fallback, no public API, no unqualified ReFS claim.

## Plan Review

Gate: **PASS (standard review only)**. Hardening required and present.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| R1 | P1 | The original stash premise missed the safe public `cap-fs-ext` wrapper. | Resolved by spike evidence and exact pinned API. |
| T1 | P1 | A rename-race RED can be impossible on Windows because retained handles block renames. | Resolved with direct different-object policy test. |
| S1 | P1 | ReFS can use 128-bit IDs. | Resolved by limiting required closure to recorded NTFS and preserving ReFS as residual. |

No unresolved standard-review P0/P1 finding remains. Review-fix cycles: 1 of 3.


## Standard Plan Review Rerun — Cycle 5

Gate: **PASS**. Six independent plan-review personas returned: Constitution Reviewer (one P3), Rust Reviewer (zero), Scope Boundary Auditor (zero), Learnings Researcher (zero), Architecture Strategist (zero), and Security Lens Reviewer (zero). No P0/P1/P2 finding remains.

- **M-02 verified:** U1/U2 require the pinned public `cap_fs_ext` 4.0.2 trait/type bridge to compile against the real Windows `Dir::dir_metadata()` return type and prohibit private, raw-handle, or unsafe fallbacks.
- **P3 operational advisory resolved in plan:** U3 records the concrete observation query/dashboard location, measured baseline, and NTFS volume evidence.

## Adversarial Multi-Model Review — Cycle 5 Final

Gate: **PASS WITH LOW ADVISORIES**. Three independent configured reviewers (`openai/gpt-5.4-mini`, `anthropic/claude-sonnet-4.6`, and `anthropic/claude-opus-4.6`) completed every required domain. M-02 closed 3/3. No HIGH, MEDIUM, P0, or P1 finding remains.

LOW advisories are acknowledged: keep this release boundary separate from `5DF94427`; record the actual Windows runner and filesystem in closure; require NTFS for this gate and make no ReFS closure claim. Review-fix cycles: 1 of 3.

Evidence: `docs/closure/2026-08-24-dark-factory-cycle5-four-plan-adversarial-review-rerun.md` and `docs/closure/2026-08-24-dark-factory-cycle5-four-plan-adversarial-review-final.md`.
