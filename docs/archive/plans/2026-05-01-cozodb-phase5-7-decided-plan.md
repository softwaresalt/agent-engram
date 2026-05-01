---
title: "CozoDB Migration Phases 5-7 — Decided Plan"
type: decided-plan
date: 2026-05-01
source_plan: docs/decisions/2026-05-01-cozodb-phase5-7-deliberation.md
shipment: 015-S
root_chore: 001-C
requires_hardening: true
---

# CozoDB Migration Phases 5-7 — Decided Plan

## Problem Statement

Phases 1-4 of the CozoDB migration shipped (PR #15, PR #53). The `cozo-backend`
feature flag has full CRUD and traversal parity with `surreal-backend`. This plan
covers the remaining work to verify integration correctness (Phase 5), flip the
default backend and produce operational closure (Phase 6), and remove the legacy
SurrealDB code (Phase 7). See the deliberation at
`docs/decisions/2026-05-01-cozodb-phase5-7-deliberation.md` for full analysis.

---

## Implementation Units

### Already Done — U5.1, U5.2, U5.3

| Task | Status | Evidence |
|------|--------|----------|
| U5.1 — content_record CRUD | done | `cozo_queries.rs` lines 2104-2330, `cozo_crud_test.rs` |
| U5.2 — commit_node CRUD | done | `cozo_queries.rs` lines 2334-2477, `cozo_crud_test.rs` |
| U5.3 — file_hash CRUD | done | `cozo_queries.rs` lines 2481-2553, `cozo_crud_test.rs` |

No further work required. Mark as `done` in backlog.

---

### U5.4 — Hydration Glue + Cold-Restart Proof

**Parent chore**: `001.006-C` (Phase 5)
**Effort**: ~1 hour
**Risk**: Low
**Execution posture**: Test-first

#### Scope Boundary

Update stale documentation and write one integration test proving the
hydration → dehydration round-trip works under the CozoDB backend after
a cold restart (data directory deletion).

#### Files to Modify

| File | Change |
|------|--------|
| `src/services/dehydration.rs` (line 1) | Update module doc comment: "SurrealDB" → backend-neutral wording |
| `src/services/hydration.rs` | Verify backend-neutral (no changes expected — reads from JSONL) |

#### Files to Create

| File | Purpose |
|------|---------|
| `tests/integration/cozo_cold_restart_test.rs` | Integration test: delete CozoDB data dir → `hydrate_workspace` → `dehydrate_code_graph` → verify nodes survive round-trip |

#### Acceptance Criteria

- [ ] Module doc on `dehydration.rs` line 1 no longer references "SurrealDB"
- [ ] `cozo_cold_restart_test` compiles under `--no-default-features --features cozo-backend`
- [ ] Test passes: round-trip produces non-empty code graph after cold restart
- [ ] `cargo clippy -- -D warnings -D clippy::pedantic` clean

#### Dependencies

None — independent starting point.

---

### U5.5 — Full Parity Smoke-Test Suite

**Parent chore**: `001.006-C` (Phase 5)
**Effort**: ~2 hours
**Risk**: Low-Medium
**Execution posture**: Test-first (expand existing dual-backend sweep)

#### Scope Boundary

Expand `cozo_dual_backend_sweep_test.rs` (or create a companion test module) to
compare MCP tool responses across backends for structural equivalence. The goal is
to catch hidden behavioral divergences before the default flip.

#### Files to Modify/Create

| File | Change |
|------|--------|
| `tests/integration/cozo_dual_backend_sweep_test.rs` | Expand with structural comparison cases |
| `tests/integration/cozo_parity_smoke_test.rs` (new, if splitting) | Optional separate file for MCP-level parity assertions |

#### Test Scenarios

1. `map_code` — identical output structure (node IDs may differ, topology must match)
2. `list_symbols` — same symbol names/types returned for identical workspace
3. `impact_analysis` — same affected files/symbols for identical input
4. `unified_search` — structural equivalence of result shape
5. Edge cases: empty workspace, single-file workspace

#### Acceptance Criteria

- [ ] Parity tests compile under both `--features surreal-backend` and `--features cozo-backend`
- [ ] Tests pass on both backends independently (structural comparison, not byte equality)
- [ ] At least 4 MCP tool response comparisons covered
- [ ] No new `unwrap()` or `expect()` usage

#### Dependencies

Depends on **U5.4** (hydration must work for test workspace setup).

---

### U5.8 — Vector Test Backend-Agnostic

**Parent chore**: `001.006-C` (Phase 5)
**Effort**: ~30 minutes
**Risk**: Low
**Execution posture**: Direct fix

#### Scope Boundary

Add `#[cfg(feature = "cozo-backend")]` module gate to `cozo_vector_test.rs` so it
runs cleanly in the CI matrix without requiring explicit `--no-default-features`.

#### Files to Modify

| File | Change |
|------|--------|
| `tests/integration/cozo_vector_test.rs` | Add `#[cfg(feature = "cozo-backend")]` module gate |

#### Verification Checklist

- [ ] No `fastembed` runtime dependency in test code (already uses synthetic `unit_vector()`)
- [ ] Test still compiles and passes with `--no-default-features --features cozo-backend`
- [ ] Test is correctly skipped when only `surreal-backend` is active
- [ ] Follows existing pattern from `cozo_dual_backend_sweep_test.rs`

#### Acceptance Criteria

- [ ] Module-level `#[cfg(feature = "cozo-backend")]` gate present
- [ ] CI matrix green for both backend axes
- [ ] No behavioral change to test logic

#### Dependencies

None — independent of other Phase 5 tasks.

---

### U6.1 — Flip Default Feature to cozo-backend

**Parent chore**: `001.007-C` (Phase 6)
**Effort**: ~30 minutes
**Risk**: HIGH
**Execution posture**: Careful mode — verify preconditions before applying

#### Scope Boundary

Change one line in `Cargo.toml` to flip the default database backend from
`surreal-backend` to `cozo-backend`. No code changes.

#### Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` (line 69) | `default = ["embeddings", "surreal-backend"]` → `default = ["embeddings", "cozo-backend"]` |

#### ProposedAction (strict-safety)

| Field | Value |
|-------|-------|
| summary | Flip default feature flag from surreal-backend to cozo-backend |
| targets | `Cargo.toml` |
| change_kind | config change |
| rollback | Revert single line in `Cargo.toml` |
| approval_required | Prefer (ActionRisk: high) |

#### Pre-Conditions (Gate)

- U5.4 cold-restart proof passing
- U5.5 parity smoke test green
- Full CI matrix green for both backends
- Documentation updates drafted (U6.2)

#### Acceptance Criteria

- [ ] `cargo build` (no flags) compiles with CozoDB backend
- [ ] `cargo test` (no flags) passes with CozoDB backend
- [ ] `cargo build --no-default-features --features surreal-backend` still compiles
- [ ] CI matrix tests both backends explicitly

#### Dependencies

Depends on **U5.4** and **U5.5** (both must pass green).

---

### U6.2 — Documentation Updates

**Parent chore**: `001.007-C` (Phase 6)
**Effort**: ~1 hour
**Risk**: Low
**Execution posture**: Direct edit

#### Scope Boundary

Update all documentation references from "SurrealDB" default to "CozoDB" default.
Note that `surreal-backend` remains available as a non-default feature.

#### Files to Modify

| File | Change |
|------|--------|
| `.github/copilot-instructions.md` | Technology table: "SurrealDB 2 (embedded)" → "CozoDB 0.7 (embedded, SQLite storage)"; update "Per-workspace namespace via SHA-256 hash" → CozoDB path-based isolation |
| `AGENTS.md` | Update any "SurrealDB" references in Technical Constraints or related sections |
| `docs/ARCHITECTURE.md` | Update database layer description |

#### Acceptance Criteria

- [ ] No remaining references to "SurrealDB" as the *default* backend in docs
- [ ] Clear note that `surreal-backend` remains available as non-default feature
- [ ] Markdown lint clean
- [ ] References to build/test commands remain accurate (unchanged — feature detection is compile-time)

#### Dependencies

Can be drafted in parallel with U5.5 but SHOULD merge after U6.1 for accuracy.

---

### U6.3 — Operational Closure

**Parent chore**: `001.007-C` (Phase 6)
**Effort**: ~1 hour
**Risk**: Low
**Execution posture**: Template-driven artifact production

#### Scope Boundary

Produce the operational closure artifact for Phases 5-6 per the
`release-observability` overlay.

#### Files to Create

| File | Purpose |
|------|---------|
| `docs/closure/2026-05-01-015-s-cozodb-phase5-6-closure.md` | Release closure: monitoring plan, pre-deploy audit, observation window, rollback trigger |

#### Required Sections

1. **Monitoring plan** — SLIs: daemon startup time, first-query latency, rehydration success rate
2. **Pre-deploy audit** — Feature flags verified, rollback path documented, backward-compatible
3. **Post-deploy observation window** — 7 days, owner: operator
4. **Rollback trigger** — Startup failure rate > 0% OR query error rate > baseline
5. **Rollback procedure** — Revert `Cargo.toml` line 69 to `surreal-backend`

#### Acceptance Criteria

- [ ] Closure artifact exists with all required sections
- [ ] Monitoring plan names specific SLIs and observation method
- [ ] Rollback trigger has named metric and threshold
- [ ] Observation window declares duration (7 days) and owner

#### Dependencies

Depends on **U6.1** and **U6.2** (documents the shipped state).

---

### U7.1 — Drop surrealdb Dependency

**Parent chore**: `001.008-C` (Phase 7)
**Effort**: ~30 minutes
**Risk**: DESTRUCTIVE (operator approval required)
**Execution posture**: Careful mode, strict-safety P-005

#### Scope Boundary

Remove the `surrealdb` crate from `[dependencies]` and delete the `surreal-backend`
feature definition. After this, `--features surreal-backend` becomes a compile error.

#### Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Remove `surrealdb` from `[dependencies]`; remove `surreal-backend` feature |
| `Cargo.lock` | Auto-updated by cargo (removes ~200 transitive deps) |

#### ProposedAction (strict-safety)

| Field | Value |
|-------|-------|
| summary | Remove surrealdb dependency and surreal-backend feature flag |
| targets | `Cargo.toml`, `Cargo.lock` |
| change_kind | deletion |
| rollback | `git revert` + `cargo update` |
| approval_required | yes (ActionRisk: destructive) |

#### Acceptance Criteria

- [ ] `cargo build` succeeds (only cozo-backend remains)
- [ ] `cargo test` passes
- [ ] `--features surreal-backend` produces a meaningful compile error
- [ ] Build time measurably reduced (fewer transitive deps)

#### Dependencies

Depends on **U6.3** completion + **7-day observation window** passing clean.

---

### U7.2 — Delete SurrealBackend Implementation

**Parent chore**: `001.008-C` (Phase 7)
**Effort**: ~1 hour
**Risk**: DESTRUCTIVE (operator approval required)
**Execution posture**: Careful mode, strict-safety P-005

#### Scope Boundary

Delete all SurrealDB-specific source code and remove feature-flag guards that are
no longer needed (CozoDB is the only backend).

#### Files to Delete

| File | Lines | Content |
|------|-------|---------|
| `src/db/queries.rs` | ~3400 | SurrealDB query helpers |
| `src/db/schema.rs` | ~200 | SurrealDB schema constants |

#### Files to Modify

| File | Change |
|------|--------|
| `src/db/mod.rs` | Remove `surreal_db` module block (lines 22-207); remove `compile_error!` mutual-exclusion guards (lines 7-17); remove `#[cfg(feature = "cozo-backend")]` guards (cozo is now the only backend); remove `#[cfg(feature = "surreal-backend")]` lines |
| All files with `#[cfg(feature = "cozo-backend")]` | Remove conditional compilation — cozo is unconditional |

#### ProposedAction (strict-safety)

| Field | Value |
|-------|-------|
| summary | Delete SurrealDB implementation files and remove feature-flag conditionality |
| targets | `src/db/queries.rs`, `src/db/schema.rs`, `src/db/mod.rs`, test files |
| change_kind | deletion |
| rollback | `git revert` |
| approval_required | yes (ActionRisk: destructive) |

#### Acceptance Criteria

- [ ] `src/db/queries.rs` and `src/db/schema.rs` deleted
- [ ] `src/db/mod.rs` simplified: no feature-flag guards, no compile_error macros
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings -D clippy::pedantic` clean
- [ ] No orphaned imports or dead code warnings
- [ ] ~3600+ lines of production code removed

#### Dependencies

Depends on **U7.1** (surrealdb dependency must be removed first).

---

### Deferred — U5.6 (concerns_edge Rename)

**Status**: Deferred to post-migration backlog
**Reason**: High blast radius (~70 locations across two backends) for zero user-facing
value. Current naming is documented as an intentional invariant in Phase 3-4 closure.
Renaming now would invalidate existing data stores and produce no correctness benefit.
**Follow-up**: Low-priority backlog item for post-SurrealDB-removal cleanup.

### Deferred — U5.7 (Datalog-Native BFS)

**Status**: Deferred to post-migration backlog
**Reason**: Rust-side BFS is correct, tested, and performant at current scale
(≤750 lightweight in-process queries). Optimizing to Datalog fixpoint risks correctness
regression and is easier to implement when only one backend exists.
**Follow-up**: Performance-labeled backlog item for post-Phase-7.

---

## Dependency Graph

```text
U5.4 (hydration glue + cold-restart)
  │
  ▼
U5.5 (parity smoke test)
  │
  ├──────────────────────────────────────┐
  ▼                                      │
U6.1 (flip default) ◄── depends U5.4+U5.5
  │                                      │
  ▼                                      │
U6.2 (docs update) ◄── after U6.1       │
  │                                      │
  ▼                                      │
U6.3 (operational closure) ◄── U6.1+U6.2│
  │                                      │
  │ ════ OBSERVATION WINDOW (7 days) ════│
  │                                      │
  ▼                                      │
U7.1 (drop surrealdb dep) ◄── U6.3+obs  │
  │                                      │
  ▼                                      │
U7.2 (delete surreal impl) ◄── U7.1     │
                                         │
U5.8 (vector test CI-ready) ── no deps ──┘
     (parallel with U5.4/U5.5)
```

**Parallel opportunities**:

- U5.8 is independent and can execute in parallel with U5.4/U5.5
- U6.2 can be drafted concurrently with U5.5, merges after U6.1

---

## PR Strategy

### PR A — Phases 5-6 (single branch, single shipment)

**Contains**: U5.4 + U5.5 + U5.8 + U6.1 + U6.2 + U6.3

**Commit sequence**:

1. `test(build): add cold-restart integration test for CozoDB` (U5.4 test)
2. `docs(build): update dehydration module doc to backend-neutral` (U5.4 docs)
3. `test(build): expand parity smoke-test suite` (U5.5)
4. `test(build): add cfg gate to cozo_vector_test` (U5.8)
5. `build(build): flip default feature to cozo-backend` (U6.1 — key commit)
6. `docs(docs): update documentation for CozoDB default` (U6.2)
7. `chore(docs): add phase 5-6 operational closure` (U6.3)

**Review**: Standard review + Copilot review. Highlight the default-flip commit.

### PR B — Phase 7 (separate branch, ships AFTER observation window)

**Contains**: U7.1 + U7.2

**Commit sequence**:

1. `build(build): remove surrealdb dependency and surreal-backend feature` (U7.1)
2. `refactor(build): delete SurrealBackend implementation` (U7.2)

**Review**: Standard review + operator approval (destructive).
Strict-safety P-005 gate applies. Operator must explicitly approve deletion.

**NON-NEGOTIABLE**: PR B MUST NOT ship in the same PR as PR A. The 7-day
observation window between them provides production confidence before
irrecoverable deletion.

---

## Execution Summary

| Step | Task | Effort | Risk | PR |
|------|------|--------|------|----|
| 1 | U5.4 — hydration glue + cold-restart test | 1h | Low | A |
| 2 | U5.5 — parity smoke-test suite | 2h | Low | A |
| 3 | U5.8 — vector test CI normalization | 30m | Low | A |
| 4 | U6.1 — flip default feature | 30m | HIGH | A |
| 5 | U6.2 — documentation updates | 1h | Low | A |
| 6 | U6.3 — operational closure | 1h | Low | A |
| — | OBSERVATION WINDOW (7 days) | — | — | — |
| 7 | U7.1 — drop surrealdb dependency | 30m | DESTRUCTIVE | B |
| 8 | U7.2 — delete SurrealBackend impl | 1h | DESTRUCTIVE | B |

**Total remaining effort**: ~7.5 hours across 8 active tasks.

---

## Constitution Check

| Principle | Compliance | Notes |
|-----------|-----------|-------|
| I. Safety-First Rust | ✅ | No `unsafe`, no `unwrap()`; all new code returns `Result<T, EngramError>` |
| II. Test-First Development | ✅ | U5.4 and U5.5 write tests before any production change; U6.1 is gated behind test passage |
| III. Workspace Isolation | ✅ | No file-system changes outside workspace root |
| IV. CLI Containment | ✅ | All operations within cwd |
| V. Structured Observability | ✅ | U6.3 produces explicit monitoring plan and closure artifact |
| VI. Single Responsibility | ✅ | Feature flags removed only after replacement proven; no speculative deps |
| VII. Destructive Approval | ✅ | U7.1 and U7.2 require explicit operator approval per strict-safety |
| VIII. Safety Modes | ✅ | U6.1 uses careful mode; U7.x uses careful + strict-safety |
| IX. Git-Friendly Persistence | ✅ | All artifacts are Markdown with YAML frontmatter |
| X. Context Efficiency | ✅ | No bulk file reads introduced |
| XI. Merge Commits | ✅ | Both PRs use merge commit strategy |

**Justified deviations**: None.

---

## Risk Register

| ID | Risk | Impact | Likelihood | Mitigation |
|----|------|--------|------------|------------|
| R1 | Default flip breaks developer workflows | High | Medium | Clear CHANGELOG, migration guide, 7-day observation window |
| R2 | Hidden behavioral divergence discovered post-flip | High | Low | U5.5 parity smoke test catches pre-merge |
| R3 | concerns_edge rename introduces regression | Medium | Medium | DEFERRED — no action needed |
| R4 | Datalog BFS correctness regression | Medium | Medium | DEFERRED — keep working Rust-side BFS |
| R5 | Phase 7 deletion leaves orphaned imports | Low | Low | `cargo check` catches immediately |
| R6 | SurrealDB removal breaks downstream forks | Low | Low | Announce in release notes; keep docs showing old flag |

---

## Plan Hardening Required

This plan requires hardening before execution:

- **Phase 6 (U6.1)**: High-risk config change affecting all default builds
- **Phase 7 (U7.1, U7.2)**: Destructive deletions requiring P-005 approval

The `plan-harden` skill should deepen:

1. Rollback procedure for U6.1 (step-by-step revert)
2. Verification checklist for observation window health
3. Explicit operator approval gates for U7.1 and U7.2
4. CI matrix validation after default flip

---

## Plan Hardening

### H1 — U6.1 Rollback Procedure

If the default flip to `cozo-backend` causes failures (build breakage, test
regressions, or runtime errors discovered during the observation window),
execute this numbered revert sequence:

1. **Create revert branch**: `git checkout -b revert/cozo-default-flip main`
2. **Edit `Cargo.toml` line 69**: Change `default = ["embeddings", "cozo-backend"]`
   back to `default = ["embeddings", "surreal-backend"]`
3. **Verify locally**:
   - `cargo build` — must compile with surreal-backend active
   - `cargo test --all-targets` — must pass
   - `cargo clippy -- -D warnings -D clippy::pedantic` — must be clean
4. **Commit**: `fix(build): revert default feature to surreal-backend`
5. **Open PR** targeting `main` with title: `revert: flip default back to surreal-backend`
6. **Fast-track merge** — this is a rollback; standard review applies but
   should be expedited
7. **Verify CI green** on main after merge
8. **Update closure artifact**: Record rollback in
   `docs/closure/2026-05-01-015-s-cozodb-phase5-6-closure.md` with
   ActionResult: `rolled-back`, timestamp, and root-cause summary
9. **Create backlog item**: Document the failure that triggered rollback as
   a blocking issue for a future re-attempt

**Rollback window**: Immediately available at any point during the 7-day
observation window. No data migration is involved — the flip is purely
compile-time. Any developer or operator can execute this procedure.

**Rollback trigger**: Any of the following conditions:

- `cargo build` (no flags) fails on any supported platform after the flip
- Test failure rate increases above 0% compared to pre-flip baseline
- Daemon startup failure observed in any environment
- First-query latency exceeds 2× pre-flip baseline

---

### H2 — CI Matrix Validation Checklist

After the default flip in U6.1, the CI matrix in `.github/workflows/ci.yml`
must be updated to ensure BOTH backends remain tested. The current matrix
uses `--no-default-features --features cozo-backend` for the cozo axis
because surreal is the current default. After the flip, this relationship
inverts.

#### Required CI Changes (verified before U6.1 merges)

- [ ] **Surreal axis updated**: Matrix entry for `surreal-backend` must use
      `--no-default-features --features surreal-backend` (no longer relies on
      being the default)
- [ ] **Cozo axis updated**: Matrix entry for `cozo-backend` uses `""` (empty
      features — it is now the default)
- [ ] **fmt step**: Runs on the default axis (cozo-backend after flip) — verify
      `if: matrix.backend == 'cozo-backend'`
- [ ] **clippy step**: Runs on both axes with correct `${{ matrix.features }}`
- [ ] **test step (blocking)**: Runs on the default axis (cozo-backend) — update
      condition to `if: matrix.backend == 'cozo-backend'`
- [ ] **test step (surreal-backend)**: Runs on the surreal axis — add step with
      `if: matrix.backend == 'surreal-backend'` and correct features flag
- [ ] **advisory flag removed**: `continue-on-error: true` removed from cozo test
      step (cozo is now the primary, not advisory)
- [ ] **audit step**: Runs once regardless of backend axis (no change needed)
- [ ] **Both axes green**: CI passes on both matrix legs before PR merges

#### Verification Command (local)

```bash
# Verify cozo (new default) builds and tests
cargo build && cargo test --all-targets

# Verify surreal (non-default) still builds and tests
cargo build --no-default-features --features surreal-backend
cargo test --no-default-features --features surreal-backend --all-targets
```

---

### H3 — Observation Window Protocol

The 7-day observation window between Phase 6 (U6.3 closure) and Phase 7
(U7.1 dependency removal) is a structured validation period, not passive
waiting.

#### Schedule

| Day | Check | Owner | Method |
|-----|-------|-------|--------|
| 0 (merge day) | CI green, daemon starts cleanly | Ship agent | Automated CI + local `cargo run` |
| 1 | Cold-restart test on fresh clone | Operator | `git clone` → `cargo build` → `cargo test` |
| 3 | Mid-window health check | Operator | Review any issues filed since merge |
| 5 | Cross-platform check (Windows, Linux, macOS) | Operator | Verify release builds (if available) or local builds on each OS |
| 7 | Window close decision | Operator | Explicit go/no-go for Phase 7 |

#### Health Signals to Monitor

| Signal | Healthy | Degraded | Trigger Escalation |
|--------|---------|----------|--------------------|
| `cargo build` (no flags) | Compiles cleanly | — | Any failure |
| `cargo test` (no flags) | All tests pass | Flaky test (pre-existing) | New test failure |
| Daemon startup time | ≤ 5 seconds | 5–10 seconds | > 10 seconds |
| First-query latency | ≤ 500ms | 500ms–1s | > 1 second |
| Rehydration success | 100% | — | Any failure |
| Issue tracker | No new CozoDB-related issues | — | Any CozoDB issue filed |

#### Escalation Procedure

If any degraded or escalation signal is observed during the window:

1. **Log the signal** in the closure artifact under "Observation Window Findings"
2. **Assess impact**: Is this a CozoDB-specific regression or pre-existing?
3. **If CozoDB regression**: Execute the U6.1 rollback procedure (H1 above)
4. **If pre-existing**: Document as unrelated; do not block Phase 7
5. **If ambiguous**: Extend the observation window by 3 days and investigate

#### Window Close Criteria

Phase 7 work MUST NOT begin until ALL of the following are true:

- [ ] 7 calendar days have elapsed since PR A merged
- [ ] No CozoDB-related issues have been filed
- [ ] All health signals remain in the "Healthy" range
- [ ] Operator has explicitly approved Phase 7 commencement
- [ ] Approval is recorded in the closure artifact with timestamp

---

### H4 — Phase 7 Approval Gates

U7.1 and U7.2 are classified `ActionRisk: destructive`. Per strict-safety
P-005, operator approval is required before execution. Proceeding without
approval is a policy violation.

#### U7.1 — Drop surrealdb Dependency

**ProposedAction**:

| Field | Value |
|-------|-------|
| summary | Remove `surrealdb` crate from `[dependencies]` and delete `surreal-backend` feature definition |
| targets | `Cargo.toml`, `Cargo.lock` |
| change_kind | deletion |
| rollback | `git revert <commit>` + `cargo update` |
| approval_required | **yes** (ActionRisk: destructive) |

**Approval Sequence**:

1. Ship agent broadcasts ProposedAction via intercom (if available) or
   surfaces in PR description
2. Agent enters **careful mode** and halts execution
3. Operator reviews the ProposedAction and responds:
   - `APPROVED` → Agent proceeds with U7.1 execution
   - `REJECTED` → Agent halts; records ActionResult: `abandoned`
   - `DEFERRED` → Agent halts; records ActionResult: `blocked`
4. After execution: Agent records ActionResult: `applied` with commit SHA
5. Agent verifies:
   - `cargo build` succeeds (only cozo-backend remains)
   - `cargo test` passes
   - `--features surreal-backend` produces a compile error

**Halt condition**: If the operator does not respond within the session
timeout, record ActionResult: `blocked` and checkpoint the session.

#### U7.2 — Delete SurrealBackend Implementation

**ProposedAction**:

| Field | Value |
|-------|-------|
| summary | Delete `src/db/queries.rs`, `src/db/schema.rs`; simplify `src/db/mod.rs`; remove all `#[cfg(feature = "cozo-backend")]` guards |
| targets | `src/db/queries.rs`, `src/db/schema.rs`, `src/db/mod.rs`, test files with surreal-specific cfg guards |
| change_kind | deletion |
| rollback | `git revert <commit>` |
| approval_required | **yes** (ActionRisk: destructive) |

**Approval Sequence**:

1. Ship agent broadcasts ProposedAction with deletion file list and line counts
2. Agent enters **careful mode** and halts execution
3. Operator reviews:
   - `APPROVED` → Agent proceeds with U7.2 execution
   - `REJECTED` → Agent halts; records ActionResult: `abandoned`
4. After execution: Agent records ActionResult: `applied` with commit SHA
5. Agent runs post-deletion verification (H5 below)

**Sequencing constraint**: U7.2 approval MUST NOT be requested until U7.1 has
ActionResult: `applied` and CI is green. The two approvals are sequential,
not batched.

**P-005 enforcement**: If at any point the ship agent detects that a
destructive action was executed without recorded operator approval, it MUST:

1. Broadcast a P-005 violation event
2. Halt immediately
3. Record the violation in the session memory checkpoint

---

### H5 — Post-Deletion Verification

After U7.2 completes, verify no orphaned code remains. This checklist must
pass before the PR B commit is finalized.

#### Automated Checks

- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo test --all-targets` passes
- [ ] `cargo clippy -- -D warnings -D clippy::pedantic` produces zero warnings
- [ ] No `dead_code` warnings from the compiler
- [ ] No `unused_imports` warnings from the compiler

#### Manual Verification (agent-executed grep checks)

- [ ] **No orphaned surreal imports**:
      `grep -r "surrealdb" src/` returns zero matches
- [ ] **No orphaned feature guards**:
      `grep -r "surreal-backend" src/` returns zero matches
- [ ] **No stale cfg attributes**:
      `grep -r '#\[cfg(feature = "surreal-backend")\]' src/` returns zero matches
- [ ] **cozo-backend guards removed** (cozo is now unconditional):
      `grep -r '#\[cfg(feature = "cozo-backend")\]' src/` returns zero matches
- [ ] **No compile_error macro remnants**:
      `grep -r "compile_error!" src/db/` returns zero matches
- [ ] **Cargo.toml clean**:
      `grep "surreal" Cargo.toml` returns zero matches (feature and dep both gone)
- [ ] **No orphaned test files**:
      `grep -r "surreal" tests/` returns zero matches (or only in file names
      scheduled for deletion)

#### Structural Verification

- [ ] `src/db/mod.rs` has no `#[cfg(...)]` feature-flag gates — CozoDB is unconditional
- [ ] `src/db/mod.rs` has no `compile_error!` mutual-exclusion macros
- [ ] `src/db/queries.rs` does not exist (deleted)
- [ ] `src/db/schema.rs` does not exist (deleted)
- [ ] `Cargo.lock` no longer contains the `surrealdb` crate or its unique transitive deps

#### Metric Verification

- [ ] Net lines removed: ≥ 3,600 lines of production code
- [ ] Net dependencies removed: verify `Cargo.lock` line count decreased
      significantly (~200 transitive deps expected to disappear)
- [ ] Build time comparison: record before/after `cargo build --release` time
      (expect measurable improvement from fewer deps)

---

## Plan Review

<!-- plan-review-attempt: 1 -->

**Verdict: PASS** — reviewed 2026-05-01

### Review Findings (P2 — execution notes for Ship agent)

| ID | Severity | Summary |
|----|----------|---------|
| RSE-1 | P2 | U6.1 scope should include `.github/workflows/ci.yml` as modified file — CI axis may rely on surreal being default |
| RSE-3 | P2 | U7.2 should enumerate `Cargo.toml` `[[test]]` `required-features` cleanup |
| TST-2 | P2 | U5.5 parity equivalence predicates should be defined per-tool during implementation |
| TST-4 | P2 | Verify U5.1-U5.3 cozo tests pass non-advisory before building on their "done" status |
| OPS-3 | P2 | CI YAML update should be part of commit 5 (U6.1 flip) in PR A |
| OPS-6 | P2 | Document `required-features` behavior after default flip in CI validation |

### Reviewers

- Rust Systems Engineer: PASS (2 P2, 2 P3)
- Test Strategy Reviewer: PASS (2 P2, 3 P3)
- Risk & Operations Reviewer: PASS (2 P2, 4 P3)

No P0/P1 findings. Plan is approved for harvest.
