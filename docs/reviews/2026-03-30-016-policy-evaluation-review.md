---
title: "Code Review: 016-mcp-sandbox-policy-engine"
branch: 016-mcp-sandbox-policy-engine
base: main
date: 2026-03-30
personas: [RustSafetyReviewer, ConstitutionReviewer, LearningsResearcher, MCPProtocolReviewer, ConcurrencyReviewer]
p0: 0
p1: 4
p2: 3
p3: 2
status: pending_decisions
---

## Review Summary

**P0: 0 | P1: 4 | P2: 3 | P3: 2**

Branch implements TASK-016 (MCP Sandbox Policy Engine) and TASK-017 (Observability & Evaluation Primitives).
9 commits, 23 source/test files changed, 48 new tests. All tests pass. Clippy clean.
No P0 findings (no data corruption, no unsafe, no panics in lib code).
4 P1 findings must be resolved before merge.

---

## P1 Findings (Blocking)

### FIND-01 — `get_evaluation_report` Missing from MCP Tool Catalog

**Source:** MCP-1 (MCPProtocolReviewer) + CON-II-001 (ConstitutionReviewer) — merged  
**File:** `src/shim/tools_catalog.rs`  
**Principle:** II (MCP Protocol Fidelity)

The new `get_evaluation_report` tool is dispatched correctly in `src/tools/mod.rs:162` but is not
registered in the MCP tool catalog (`all_tools()` in `src/shim/tools_catalog.rs`). MCP clients
calling `tools/list` will never discover the tool, even though direct invocation works. This
violates Principle II: all MCP tools must be unconditionally visible to every connected agent.

**Fix:** Add `get_evaluation_report` entry to `all_tools()`. Update `TOOL_COUNT`. Add it in the
Observability section after `get_token_savings_report`. Update any catalog count assertions.

---

### FIND-02 — Error Outcomes Never Persisted; `error_rate`/`error_burst` Detection Is Dead

**Source:** RS-1 (RustSafetyReviewer)  
**File:** `src/tools/mod.rs:180`

`UsageEvent` is only emitted inside `if let Ok(value) = &result { … }`. The `outcome` field is
therefore always `"success"` for every recorded event. Error-path calls are never persisted.
Downstream in `evaluation.rs`, `error_rate` is computed by filtering `outcome != "success"` —
which always yields `0.0`. The `error_burst` anomaly detection block and the `error_rate` field
on `AgentEfficiency` are structurally dead code.

**Fix:** Move the `UsageEvent` record site to run for both success and error outcomes:
```rust
let outcome = if result.is_ok() { "success" } else { "error" };
// build event with outcome and zero defaults for error-path fields
metrics::record(..., outcome: outcome.to_string());
```

---

### FIND-03 — `TempDir` Dropped Early in `setup_workspace_with_policy`

**Source:** LR-001 (LearningsResearcher)  
**File:** `tests/contract/policy_contract_test.rs:20`  
**Compound:** `.backlog/compound/test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md`

`setup_workspace_with_policy` creates a `tempfile::TempDir` but returns only `Arc<AppState>`.
The `TempDir` is dropped at end of the helper (line 44), deleting the workspace directory before
the test body runs. Tests currently pass because policy evaluation is in-memory and doesn't
access the filesystem post-`set_workspace` — but this is a latent bug inconsistent with the
correctly-fixed `setup_workspace_with_events` pattern.

**Fix:** Change return type to `(Arc<AppState>, tempfile::TempDir)` and update all 5 callsites:
```rust
async fn setup_workspace_with_policy(policy: PolicyConfig) -> (Arc<AppState>, tempfile::TempDir) {
    // ...
    (state, workspace)
}
// callsites:
let (state, _workspace) = setup_workspace_with_policy(...).await;
```

---

### FIND-04 — Policy TOCTOU Window: Config Snapshot Dropped Before Tool Execution

**Source:** CC-1 (ConcurrencyReviewer)  
**File:** `src/tools/mod.rs:114`

The policy gate snapshots `policy_config` from `state.policy_config().await` and immediately
drops the read lock before the tool runs. A concurrent `set_workspace_config` call between the
policy check and tool execution can apply a tighter policy while an already-approved call still
proceeds under the stale snapshot. More critically: `policy_config()` returns `None` when no
config is present. During the window between workspace binding and config publication, the policy
gate is bypassed entirely (`None` → skip enforcement).

**Fix (preferred):** Snapshot both workspace state and config atomically in a single lock. At
minimum, fail closed for non-lifecycle tools when workspace is bound but config is `None`:
```rust
// Instead of silently skipping when config is None after workspace bind,
// treat missing config as deny-all for non-lifecycle tools
```

---

## P2 Findings (Recommended)

### FIND-05 — Tool Hammering Detector Fragile Invariant Coupling

**Source:** RS-2 (RustSafetyReviewer)  
**File:** `src/services/evaluation.rs:215`

`timestamps.windows(21)` always yields slices of exactly 21 elements, so `window.first()` and
`window.last()` are always `Some`. The `unwrap_or(&0)` fallbacks are dead but semantically
wrong: if the guard threshold and window size were ever desynchronized, `&0` would produce
`span = 0`, triggering a spurious hammering anomaly for every heavily-used tool.

**Fix:** Replace with direct indexing to make the invariant explicit:
```rust
for window in timestamps.windows(21) {
    let span = window[20] - window[0];  // invariant: windows(21) guarantees 21 elements
    if span <= 60 { … }
}
```

---

### FIND-06 — No Tool Discovery Contract Test

**Source:** CON-III-002 (ConstitutionReviewer)  
**File:** `tests/contract/evaluation_contract_test.rs`  
**Principle:** III (Test-First)

TASK-017 adds `get_evaluation_report` to dispatch but has no contract test verifying the tool
appears in the MCP `tools/list` response. Direct invocation is tested (C017-01 through C017-04)
but discoverability is not.

**Fix:** Add `c017_05_evaluation_report_discoverable_via_tools_list()` that verifies the tool
appears in `all_tools()`. Consider a meta-test verifying dispatch entries match catalog entries.

---

### FIND-07 — Policy Gate Unconditionally Bypassed Before Workspace Is Bound

**Source:** RS-3 (RustSafetyReviewer)  
**File:** `src/tools/mod.rs:114`  
**Confidence:** Medium

The initial `set_workspace` call always bypasses the policy engine because no workspace
config exists yet (`policy_config()` returns `None`). If an operator intends to deny a
particular agent role from calling `set_workspace`, the first call will always succeed.
Post-bind `set_workspace` calls ARE governed by the loaded policy.

**Fix:** Document explicitly. If intentional (workspace policy only governs post-bind ops),
add a doc comment to `AppState::policy_config` and a call-site comment explaining the constraint.
Daemon-level policy (independent of workspace config) would be needed for true pre-bind gating.

---

## P3 Findings (Advisory)

### FIND-08 — `_meta` Leaks Into Tool Handler Params

**Source:** MCP-2 (MCPProtocolReviewer)  
**File:** `src/tools/mod.rs:111`

`_meta` is read for policy extraction but the original `params` object (including `_meta`) is
forwarded unchanged to every tool handler. Transport metadata crosses the protocol boundary.
Current handlers ignore unknown fields, so no failures today, but schema isolation is weakened.

**Suggestion:** Strip `_meta` from params after extraction and forward only tool-defined args.

---

### FIND-09 — Redundant Item-Level `#[allow]` Annotations

**Source:** RS-4 (RustSafetyReviewer)  
**File:** `src/services/evaluation.rs:17,176`

`lib.rs` declares `#![allow(clippy::too_many_lines)]` and `#![allow(clippy::cast_precision_loss)]`
crate-wide. The same annotations on `evaluate()` (line 17) and the `weight` binding (line 176)
are redundant no-ops. They mislead reviewers into thinking targeted analysis was done.

**Suggestion:** Remove the two redundant annotations. Keep line 182's
`#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]` — it is justified and not
crate-suppressed.

---

## Positive Confirmations

* ✅ `evaluation_contract_test.rs`: Correct `TempDir` lifetime pattern — returns `(Arc<AppState>, TempDir)`
* ✅ `src/models/policy.rs`: Correct `#[derive(Default)]` + `#[default]` on `UnmatchedPolicy`
* ✅ All `unwrap_or` usage in lib code uses safe fallback patterns
* ✅ No `unsafe` code introduced anywhere
* ✅ All 48 new tests pass; clippy clean
