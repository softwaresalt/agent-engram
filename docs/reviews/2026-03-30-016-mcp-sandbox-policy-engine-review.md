---
title: "Code Review: 016-mcp-sandbox-policy-engine → main"
date: 2026-03-30
mode: interactive
gate: conditional-pass
reviewers: [Rust Safety Reviewer, Constitution Reviewer, MCP Protocol Reviewer, Concurrency Reviewer, Learnings Researcher]
tasks: [TASK-015, TASK-016, TASK-017]
---
<!-- markdownlint-disable-file -->
# Code Review: 016-mcp-sandbox-policy-engine → main

## Summary

| Severity | Count | Status |
|----------|-------|--------|
| P0 | 0 | — |
| P1 | 2 | Pending user decision |
| P2 | 4 | Pending user decision |
| P3 | 3 | Advisory |
| Resolved by PR | 2 | LR-001, LR-002 (compound learnings already applied) |
| Clean pass | MCP Protocol | 0 findings |

**Required checks:**
- ✅ `cargo clippy -- -D warnings` → clean, 0 warnings
- 🔄 `cargo test --all` → compiling (result pending)

---

## Findings

### 🔴 P1 — High Impact

#### CR-001 — `src/models/policy.rs:8-17`
**UnmatchedPolicy defaults to Allow — tension with Principle 9 (Security by Default)**

When an operator sets `enabled = true` and defines rules for known agents, unrecognized agents are _silently allowed_ (the default). An operator who explicitly enables policy enforcement would reasonably expect the reverse: unknown = blocked.

> `#[default] Allow` means: if you opt in to policy but forget `unmatched = "deny"`, all agents not in your rule list can call any tool.

**Recommendation:** Change `#[default]` to `Deny`. Since `PolicyConfig::enabled` defaults to `false`, this is backward-safe — no existing installation is affected until they explicitly write a `[policy]` section with `enabled = true`. Add a CHANGELOG note. `Allow` remains available via explicit config.

---

#### CR-003 — `src/services/policy.rs:38-76`
**No tracing instrumentation on policy evaluation — violates Principle 4 (Observability First)**

Policy denials are security-relevant events. Currently they return an error to the agent caller but emit no structured log event. Operators reviewing server logs for unauthorized access patterns have no observability signal.

**Recommendation:**
```rust
#[tracing::instrument(level = "debug", skip(config))]
pub fn evaluate(config: &PolicyConfig, agent_role: Option<&str>, tool_name: &str) -> Result<(), PolicyError> {
    // ...
    // on denial:
    tracing::warn!(agent_role = ?agent_role, tool_name, "policy denied tool call");
    // on allow:
    tracing::debug!(agent_role = ?agent_role, tool_name, "policy allowed tool call");
}
```

---

### 🟡 P2 — Moderate Issues

#### RS-002 — `src/services/evaluation.rs:56-65`
**`tokens_per_result` when `total_results == 0` uses `total_tokens as f64` — semantically ambiguous**

When an agent makes calls that return zero results (e.g., `list_symbols` with no matches), `tokens_per_result` is set to `total_tokens`. This conflates two different concepts and can create false anomaly flags when tokens are low (e.g., 5 tokens → `tokens_per_result = 5.0`, below the 10.0 threshold, so no spike flagged even though 0 results were produced).

**Recommendation:** Use `f64::INFINITY` when `total_results == 0` and `total_calls > 0`, forcing `token_eff = 0.0` in the scoring formula. Or add a dedicated "zero results" anomaly flag when `total_calls > 0 && total_results == 0`.

---

#### CR-004 — `src/tools/mod.rs:125-130`
**Policy-denied calls excluded from metrics — partial observability gap**

Denied attempts are invisible to `get_evaluation_report`. Operators cannot detect patterns like a misconfigured agent repeatedly hitting policy walls. The current comment documents the intent but doesn't offer a tracking alternative.

**Recommendation:** Record policy denials with `outcome: "policy_denied"` (early return at line ~129, before the `should_record_metrics` check) or add a dedicated atomic counter on `AppState` for denial telemetry surfaced in `get_health_report`.

---

#### CR-005 — `src/services/policy.rs:1-76` / `src/errors/mod.rs`
**`PolicyError::ConfigInvalid` (14002) is modeled, tested, and mapped — but never raised**

The error code is allocated, the variant is defined, and contract tests assert its numeric value, but no production code path raises it. `evaluate()` never validates the `PolicyConfig`.

**Recommendation:** Add a `PolicyConfig::validate()` that returns `Err(PolicyError::ConfigInvalid)` for contradictory rules (same tool in both `allow` and `deny`). Wire into config loading in `state.rs::set_workspace_config`. This closes the dead code path and makes the 14002 allocation meaningful.

---

#### CR-002 — `src/models/policy.rs:35-36`
**`PolicyConfig::enabled` defaults to false — intentional Principle 9 deviation**

This is a conscious backward-compat tradeoff — existing workspaces must not break on upgrade. However, it should be documented explicitly as a deviation.

**Recommendation:** No code change needed. Add a one-line doc comment to the `enabled` field: `/// Set to `true` to activate policy enforcement. Defaults to `false` for backward compatibility.` Optionally emit a startup `tracing::info!` when policy is disabled with a workspace loaded.

---

### 🔵 P3 — Advisory

#### CC-001 — `src/services/evaluation.rs:196-230`
**`detect_tool_hammering` uses `HashMap` → nondeterministic anomaly selection**

If multiple tools qualify for the hammering anomaly in a single evaluation, the tool name reported in the anomaly description varies across runs due to `HashMap` iteration order. This can make tests flaky.

**Recommendation:** Replace `HashMap<&str, Vec<i64>>` with `BTreeMap<&str, Vec<i64>>` in `detect_tool_hammering` for deterministic iteration.

---

#### RS-003 — `src/services/evaluation.rs:10-40`
**`unwrap_or(u64::MAX)` / `unwrap_or(u32::MAX)` fallback values**

Practically unreachable (requires > 18 quintillion events), but if ever hit, the MAX sentinel produces nonsensical scoring silently. Advisory only — no current production risk.

---

#### CR-006 — `src/tools/mod.rs:114-124`
**TOCTOU between policy check and dispatch — documented and tracked as TASK-018**

Acknowledged in code comments. No action needed in v1. When addressing TASK-018, prefer an `Arc<PolicyConfig>` snapshot pattern.

---

## Learnings Applied

### ✅ LR-001 — `clippy::derivable_impls` on enum Default (already resolved)
Prior compound document `clippy-derivable-impls-enum-default-2026-03-30.md` warned that manual `impl Default` returning a variant triggers `clippy::derivable_impls`. The PR correctly uses `#[derive(Default)]` + `#[default]` attribute. Confirmed by clippy clean pass.

### ✅ LR-002 — TempDir lifetime in contract tests (already resolved)
Prior compound document `tempdir-lifetime-in-contract-tests-2026-03-30.md` documented premature TempDir drop in test helpers. The PR's `setup_workspace_with_policy` correctly returns `(Arc<AppState>, tempfile::TempDir)` with callers binding `_workspace` to keep the directory alive.

---

## Residual Work

| ID | Severity | Tracking |
|----|----------|----------|
| CR-001 | P1 | Decide: flip UnmatchedPolicy default to Deny |
| CR-003 | P1 | Add tracing to policy::evaluate |
| RS-002 | P2 | Fix tokens_per_result=0-results semantics |
| CR-004 | P2 | Add policy-denied outcome to metrics/telemetry |
| CR-005 | P2 | Add PolicyConfig::validate() wired to 14002 |
| CC-001 | P3 | BTreeMap in detect_tool_hammering |
