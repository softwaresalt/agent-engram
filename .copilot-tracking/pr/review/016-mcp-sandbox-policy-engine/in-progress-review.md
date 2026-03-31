<!-- markdownlint-disable-file -->
# PR Review Status: 016-mcp-sandbox-policy-engine

## Review Status

* Phase: 3 — Delegated Review (pr-review agent session restart, 2026-03-30)
* Last Updated: 2026-03-30 (session resumed)
* Summary: Three feature epics (TASK-015 model routing, TASK-016 policy engine, TASK-017 observability/eval) — 10 commits, 24 core source files, 167 total changed files

## Branch and Metadata

* Normalized Branch: `016-mcp-sandbox-policy-engine`
* Source Branch: `016-mcp-sandbox-policy-engine`
* Base Branch: `main`
* Linked Work Items: TASK-016, TASK-017 (all subtasks Done)
* Commits: 9 (51da38f → ab47eb7)

## Diff Mapping

| File | Type | New Lines | Notes |
|------|------|-----------|-------|
| `src/errors/codes.rs` | Modified | ~6 | POLICY_DENIED=14001, POLICY_CONFIG_INVALID=14002 |
| `src/errors/mod.rs` | Modified | +33 | PolicyError enum, EngramError::Policy variant, to_response arms |
| `src/models/config.rs` | Modified | +10 | policy + evaluation fields in WorkspaceConfig |
| `src/models/evaluation.rs` | New | 1–149 | AgentEfficiency, EvaluationReport, AnomalyFlag, ScoringWeights, EvaluationConfig |
| `src/models/metrics.rs` | Modified | +10 | agent_role + outcome fields on UsageEvent |
| `src/models/mod.rs` | Modified | +2 | pub mod policy, pub mod evaluation |
| `src/models/policy.rs` | New | 1–44 | PolicyRule, PolicyConfig, UnmatchedPolicy |
| `src/server/state.rs` | Modified | +22 | policy_config() and evaluation_config() accessors |
| `src/services/evaluation.rs` | New | 1–230 | evaluate(), detect_tool_hammering() |
| `src/services/metrics.rs` | Modified | +13 | load_events() extracted from compute_summary() |
| `src/services/mod.rs` | Modified | +2 | pub mod policy, pub mod evaluation |
| `src/services/policy.rs` | New | 1–76 | evaluate(), extract_agent_role(), ToolCallContext |
| `src/tools/mod.rs` | Modified | +14 | policy gate before dispatch, agent_role in UsageEvent |
| `src/tools/read.rs` | Modified | +36 | get_evaluation_report() handler |
| `Cargo.toml` | Modified | +16 | 4 new [[test]] blocks |
| `tests/contract/error_codes_test.rs` | Modified | +25 | policy error code tests |
| `tests/contract/evaluation_contract_test.rs` | New | 1–191 | C017-01 to C017-04 |
| `tests/contract/policy_contract_test.rs` | New | 1–206 | C016-01 to C016-05 |
| `tests/integration/evaluation_test.rs` | New | 1–394 | 15 BDD tests |
| `tests/integration/policy_test.rs` | New | 1–402 | 22 BDD tests |
| `tests/unit/metrics_collector_test.rs` | Modified | +2 | outcome field |
| `tests/unit/metrics_model_test.rs` | Modified | +6 | agent_role + outcome fields |
| `tests/unit/proptest_models.rs` | Modified | +4 | policy + evaluation in WorkspaceConfig literal |

## Instruction Files Reviewed

* `.github/instructions/rust.instructions.md` — applies to all `src/**/*.rs` and `tests/**/*.rs`; enforces forbid(unsafe), Result error handling, clippy pedantic, test coverage
* `.github/instructions/constitution.instructions.md` — applies to `**`; enforces Principles I–IX

## Review Items (Merged — 9 unique findings after dedup)

### P1 — Blocking (4 findings)

* **FIND-01** `src/shim/tools_catalog.rs` — `get_evaluation_report` missing from MCP tool catalog [MCP-1 + CON-II-001 merged]
* **FIND-02** `src/tools/mod.rs:180` — Error outcomes never persisted; `error_rate`/`error_burst` detection is dead code [RS-1]
* **FIND-03** `tests/contract/policy_contract_test.rs:20` — `TempDir` dropped early in `setup_workspace_with_policy`; workspace deleted before test runs [LR-001]
* **FIND-04** `src/tools/mod.rs:114` — Policy TOCTOU window: config snapshot dropped before tool execution; concurrent rebind bypasses policy [CC-1]

### P2 — Recommended (3 findings)

* **FIND-05** `src/services/evaluation.rs:215` — Tool hammering detector `unwrap_or(&0)` fallback has semantically wrong default; invariant coupling between window size and guard is fragile [RS-2]
* **FIND-06** `tests/contract/evaluation_contract_test.rs` — No contract test verifying `get_evaluation_report` is discoverable via tools/list [CON-III-002]
* **FIND-07** `src/tools/mod.rs:114` — Policy gate bypassed before workspace bound; `set_workspace` always ungated; requires doc comment at minimum [RS-3]

### P3 — Advisory (2 findings)

* **FIND-08** `src/tools/mod.rs:111` — `_meta` not stripped before forwarding params to tool handlers; transport metadata leaks past protocol boundary [MCP-2]
* **FIND-09** `src/services/evaluation.rs:17,176` — Two item-level `#[allow]` annotations are redundant with crate-level suppressions [RS-4]

### ✅ Positive Confirmations

* `evaluation_contract_test.rs`: Correct `TempDir` lifetime pattern applied [LR-002]
* `src/models/policy.rs`: Correct `#[derive(Default)]` + `#[default]` on `UnmatchedPolicy` [LR-003]
* All `unwrap_or` usage in lib code uses safe fallback patterns [CON-I-001]

### ❌ Rejected / No Action

*(pending user decisions)*

## Phase 2 Risk Areas Identified

### 🔒 Security
1. **`extract_agent_role` relies on trusting `_meta.agent_role`** — any caller can set any role; no signature or capability token. This is an inherent v1 design choice (accept string assertions), but worth documenting clearly.
2. **Outcome field defaults to `"success"`** in the `UsageEvent` written after the result — error outcome is never written for denied calls (the `map_err` returns early before the metrics write at line 190 in `tools/mod.rs`). Error calls may be under-counted in evaluation.

### ⚠️ Logic
3. **`tokens_per_result` when `total_results == 0`** uses `total_tokens as f64` as the denominator fallback — this produces very high ratios for tools that return 0 results (like `set_workspace`), potentially triggering false `token_ratio_spike` anomalies.
4. **`agent_role` written to `UsageEvent` only on success path** — the metrics event is only recorded after `result.is_ok()` check; denied calls never appear in evaluation data.
5. **`evaluate()` receives `config: &EvaluationConfig` but `ScoringWeights` fields are read but the `weights` struct isn't actually used in the scoring formula** — the composite score uses hardcoded multipliers (0.4, 0.3, 0.15, 0.15), not `config.weights`. Configuration has no effect.

### 💡 Advisory
6. **`ToolCallContext` struct** in `services/policy.rs` is defined but never populated or passed anywhere — dead code (though deliberately deferred per session notes).
7. **`evaluation.rs` has `#[allow(clippy::too_many_lines)]`** — consider extracting `score_agent()` helper to reduce complexity.
8. **No `get_evaluation_report` entry in any tool catalog/schema listing** — if `tools_catalog_test.rs` enumerates all tool names, it may need updating.

## Next Steps

* [ ] Invoke review skill in Phase 3
* [ ] Present findings to user for decisions
* [ ] Generate handoff.md
* [ ] Create PR
