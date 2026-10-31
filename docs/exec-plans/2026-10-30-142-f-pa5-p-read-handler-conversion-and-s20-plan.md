---
title: "142-F PA5-P implementation plan: PA-5 read-handler conversions, S-20 parity harness split, and Slot-19.k to Slot-21 assembly"
description: "Implementation plan and careful-mode hardening for 142-F section 7.6 (PA5-P): converts G1/G2 read handlers to serve opened generations, adds ToolDescriptor.generation_backed, splits 142.058-T into Slot-20a..20d under rulings R-A6 and R-A7, and assembles Slot-19.1..19.7, Slot-20a..20d, and Slot-21 shipments"
date: 2026-10-30
feature: "142-F"
source: "docs/decisions/2026-09-27-pa5-read-handler-conversion-deliberation.md"
related_decisions:
  - "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
  - "docs/decisions/2026-10-04-142-f-rev18-execution-rulings.md"
  - "docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md"
  - "docs/decisions/2026-09-26-142-f-launcher-preflight-command-deliberation.md"
stash_source: "86F93068, 4628001C"
status: approved-ready-for-harvest
---

## Problem Frame

In `DaemonMode::ReadServer`, only `get_workspace_statistics` currently serves the admitted generation's database (via `142.065-T` PRE-4b and `142.066-T` PRE-4, which introduced `GENERATION_PINNED_READS = &["get_workspace_statistics"]` in `src/daemon/request_entry.rs` and `queries_from_read_context` in `src/tools/read.rs`). Every other generation-backed read handler still resolves storage from the mutable managed workspace binding (`snapshot_dispatch_context`, `ReadRequestContext::from_managed_state`, or `connect_db(&snapshot.data_dir, &snapshot.branch)`), and `process_request` dispatches those handlers without a pinned `Arc<ReadRequestContext>`, so their responses carry no `provenance.generation_id`:

* `src/tools/lifecycle.rs::get_workspace_status` (line 1115) opens `connect_db(&snapshot.data_dir, &snapshot.branch)` unconditionally for its `code_graph` block (stash `4628001C`), creating managed-mode database side effects in read-server mode and leaving F54's CLI/MCP equivalence test without generation provenance.
* `src/tools/read.rs` code-graph handlers (`query_memory`, `list_symbols`, `map_code`, `impact_analysis`, `query_graph`, `unified_search`, and `query_changes` under `git-graph`) take two arguments `(state, params)` and call `pinned_read_request_context` or `pinned_dispatch_context` / `pinned_queries` against managed state instead of accepting a caller-supplied `Option<&Arc<ReadRequestContext>>` from `tools::dispatch_with_read_context` (stash `86F93068`).
* `src/tools/capabilities.rs::ToolDescriptor` has no field distinguishing generation-backed reads (`get_workspace_status`, `get_workspace_statistics`, `query_memory`, `map_code`, `list_symbols`, `unified_search`, `impact_analysis`, `query_graph`, and `query_changes`) from daemon- or workspace-file-scoped reads that remain available on a read server without reading the generation database (`get_daemon_status`, `get_health_report`, `get_branch_metrics`, `get_token_savings_report`, `get_evaluation_report`, `get_mutable_script_retry_metrics`, `get_retrieval_eval_report`, and `lint_dax`).
* `tests/contract/read_server_cli_mcp_parity_test.rs` (`142.058-T`, F54) is a 1,633-line harness at parked commit `6d216d19` whose fixture publishes a `:create probe_row {id => val}` database instead of a real indexed generation, and whose `S-20` split (`142.058a`..`142.058c`, `142.058-T`), dependency edges (`E3`, `E6`, `E15`), section 9 content, and shipments (`Slot-19.k`, `Slot-20a`..`Slot-20d`, `Slot-21`) were deferred to `PA5-P` by Ruling `R-A7` (`docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md`).

This plan implements section 7.6 (`PA5-P`) of the frozen `142-F` decomposition plan Revision 18 (`docs/exec-plans/2026-09-30-142-f-decomposition-plan.md`) together with Rulings `R-A1` through `R-A8`.

## Requirements Trace

| Source requirement | Authority | Implementation unit(s) |
|---|---|---|
| Convert `get_workspace_status` so only its `code_graph` block reads from the admitted generation DB; `path`, `branch`, `db_path`, `stale_files`, `scan_status`, and `generation` keep reporting the managed binding (`OD-5` Q3, `R-1`); resolve stash `4628001C`; add `"get_workspace_status"` to `GENERATION_PINNED_READS` while keeping no-gate / pre-activation status calls accepted with no provenance (`R6.8`, `142.066-T` Scenario 3) | Deliberation `PA5-C`, `OD-5` Q3, `R-1`, `R6.8`, Stash `4628001C` & `86F93068` | `PA5-T1` (`Slot-19.1`, `142.080-T`) |
| Convert `query_memory` and `list_symbols` to accept `Option<&Arc<ReadRequestContext>>`, read through `queries_from_read_context`, and join `GENERATION_PINNED_READS` | Deliberation `PA5-C`, Stash `86F93068` | `PA5-T2` (`Slot-19.2`, `142.081-T`) |
| Convert `map_code` and `impact_analysis` to accept `Option<&Arc<ReadRequestContext>>`, read through `queries_from_read_context`, and join `GENERATION_PINNED_READS` | Deliberation `PA5-C`, Stash `86F93068` | `PA5-T3` (`Slot-19.3`, `142.082-T`) |
| Split provisional `PA5-T4` under `SG-1` (Plan §4.2, §7.6): convert `query_graph` in its own slot and `unified_search` in its own slot | Deliberation `PA5-C`, Plan §4.2 & §7.6, Stash `86F93068` | `PA5-T4a` (`Slot-19.4`, `142.083-T`), `PA5-T4b` (`Slot-19.5`, `142.084-T`) |
| Convert `query_changes` (`#[cfg(feature = "git-graph")]`) to accept `Option<&Arc<ReadRequestContext>>`, read through `queries_from_read_context`, and join `GENERATION_PINNED_READS` | Deliberation `PA5-C`, Stash `86F93068` | `PA5-T5` (`Slot-19.6`, `142.085-T`) |
| Add `pub generation_backed: bool` to `ToolDescriptor` in `src/tools/capabilities.rs` (`OD-5` Q1, `R-1`) without editing F54 (`R-2`: F54 half moves to `Slot-20d`) | Deliberation `PA5-C`, `OD-5` Q1, `R-1`, `R-2`, Plan §7.6 | `PA5-T6` (`Slot-19.7`, `142.086-T`) |
| Split `142.058-T` (`S-20`) into `142.058a` (`Slot-20a`), `142.058b` (`Slot-20b`), `142.058c` (`Slot-20c`), and `142.058-T` (`Slot-20d`) with `R-A6` authored/adapted line counts $\le 400$ lines, `SG-1` compliance, complete AC map, and §9.1/§9.2 content | Plan §4.3 `S-20`, §7.6, §9.1, §9.2, Rulings `R-A6` & `R-A7` | `S-20` family: `142.087-T` (`Slot-20a`), `142.088-T` (`Slot-20b`), `142.089-T` (`Slot-20c`), `142.058-T` (`Slot-20d`) |
| Wire task edges `E7`, `E9`, `E3`, `E6`, `E15` and create one-task shipments `Slot-19.1`..`Slot-19.7`, `Slot-20a`..`Slot-20d`, and `Slot-21` (`142.059-T`) with mirrored shipment `blocks` edges and `custom_fields.queue_position` | Plan §5.1, §5.2, §7.6, Ruling `R-A7` | Harvest & Shipment Assembly (`167-S`..`178-S`) |

## Prior Learnings Consulted

Before structuring the units, `docs/compound/` was searched via `Learnings Researcher` (`confidence: high`, 103 files searched):

* `docs/compound/capability-rewrite-must-convert-every-consumer-2026-08-21.md` — When threading `ReadRequestContext` into read handlers, verify that no inner helper re-opens `connect_db(&workspace_path, &branch)` or re-derives workspace state independently on the `Some(context)` path.
* `docs/compound/concurrency-issues/cozo-sqlite-busy-locked-reopen-panic-catch-unwind-2026-07-15.md` and `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md` — `connect_db` creates advisory `.db.lock` and SQLite `-wal`/`-shm` sidecars; serving reads from `OpenedGeneration::db()` via `queries_from_read_context` avoids per-request reopens and keeps sealed generation directories free of stray sidecars.
* `docs/compound/workspace-status-code-graph-cfg-gate-false-premise-2026-07-07.md` — `get_workspace_status` reads `CodeGraphQueries::count_*` unconditionally in all builds and must be tested against a populated index (`code_files > 0`), never an empty database.
* `docs/compound/test-failures/truncated-test-output-review-hid-call-site-regression-2026-09-12.md` — Check all call sites of `get_workspace_status`, `GENERATION_PINNED_READS`, and `src/tools/read.rs` functions, including existing source-contract tests (`contract_read_path_pinning_enforcement`, `integration_search_service_pin`, `integration_core_read_generation_pin`, `integration_report_read_generation_pin`, `integration_lifecycle_read_generation_pin`, `integration_read_server_generation_wiring`).
* `docs/compound/test-probe-resets-idle-ttl-livelock-2026-09-04.md` and `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md` — Strip `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, and `ENGRAM_DIRECT` on every test child `Command`, and use the `PRE-3F` activation-settle barrier (`tests/helpers/activation_settle.rs`).
* `docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md` and `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md` — Never run `backlogit sync` on top of a divergent `.backlogit/backlogit.db`, and keep all `142-F` slot shipment manifests strictly one-task (`[<task_id>]`), never adding `142-F`.

## Implementation Units

Every code/test unit below uses **Route `(c)`** (Plan §4, §9.1 `U4`): at Step 2 the `harness-architect` writes the failing test against the unit's own placeholder (`Worker: <task_id>`); `build-feature` never authors the test and replaces only the placeholder at Step 4.2. Specifically, for handler-conversion units `PA5-T1`..`PA5-T5` (`142.080-T`..`142.085-T`), matching the `142.065-T` (`PRE-4b`) precedent, at Step 2 the `harness-architect` lands the new `pub async fn <handler>_with_context(...)` stub returning `Err(EngramError::System(SystemError::InvalidParams { reason: "Worker: 142.08x-T".to_owned() }))` (plus `assert!(GENERATION_PINNED_READS.contains(&"<method>"), "Worker: 142.08x-T ...")` in the test), and each test calls `<handler>_with_context` first before any publish or activation so it fails deterministically at Step 2; `build-feature` replaces only the stub body and `GENERATION_PINNED_READS` list entry at Step 4.2. `Slot-21` (`142.059-T`, already rewritten at assembly) is a `docs`-route task (`harness-verification-gated`) that receives its one-task shipment here.

### Unit 1 — `PA5-T1` (`Slot-19.1`, `142.080-T`): Pinned `get_workspace_status` `code_graph` counts

* **Title**: `PA5-T1: Serve pinned get_workspace_status code_graph counts from the opened generation`
* **Slot / `queue_position`**: `Slot-19.1` / `191`
* **Depends on**: `142.063-T` (`E7`), `142.065-T` (`PRE-4b`), `142.066-T` (`PRE-4`)
* **Lineage**: Consumes stash `4628001C` and the `get_workspace_status` slice of stash `86F93068`; `OD-5` Q3, `R-1`, `R6.8`
* **Execution posture & safety mode**: test-first, Route `(c)`; safety mode: `careful`
* **Files edited**:
  * Production: `src/tools/lifecycle.rs` (1 substantive file), plus registration-sized glue in `src/tools/mod.rs` (1 dispatch arm) and `src/daemon/request_entry.rs` (add `"get_workspace_status"` to `GENERATION_PINNED_READS` with inline observability fallback in `process_request` when no generation is active)
  * Test harness: `tests/integration/generation_pinned_workspace_status_test.rs` (new target `integration_generation_pinned_workspace_status`) + 1 `[[test]]` stanza in root `Cargo.toml`
* **What changes**:
  * Keep `pub async fn get_workspace_status(state: &AppState) -> Result<WorkspaceStatus, EngramError>` in `src/tools/lifecycle.rs` as a 1-line delegation `get_workspace_status_with_context(state, None).await`.
  * Add `pub async fn get_workspace_status_with_context(state: &AppState, read_context: Option<&Arc<ReadRequestContext>>) -> Result<WorkspaceStatus, EngramError>` in `src/tools/lifecycle.rs` (leaving `src/tools/read.rs` untouched in `PA5-T1`):
    * Always takes `state.snapshot_dispatch_context().await` and triggers `maybe_pause_generation_pin_test_hook("get_workspace_status").await` so `path`, `branch`, `db_path`, `last_flush`, `stale_files`, `connection_count`, `scan_status`, `retrieval_eval_enabled`, and `generation` continue to report the managed workspace binding and F45 generation observability (`OD-5` Q3, `R-1`, `integration_lifecycle_read_generation_pin`).
    * For the `code_graph` block only: match directly on `read_context.and_then(|ctx| ctx.generation())`. On `Some(generation)`, construct `CodeGraphQueries::new(CozoDb { inner: Arc::new(generation.opened_generation().db().clone()) })` directly in `src/tools/lifecycle.rs` and read `code_files`, `functions`, `classes`, `interfaces`, and `edges` with `.unwrap_or(0)` from the opened generation DB without ever calling `connect_db`. On `None` (no `read_context` or `ctx.generation()` is `None`): when `state.mode() == DaemonMode::Managed`, keep today's `connect_db(&snapshot.data_dir, &snapshot.branch).await` with `.unwrap_or(0)` and `warn!` on connect failure byte-for-byte; when `state.mode() == DaemonMode::ReadServer` (no generation active yet), return `CodeGraphStats::default()` without calling `connect_db` (resolving stash `4628001C`).
  * In `src/tools/mod.rs`: update the `"get_workspace_status"` arm in `dispatch_with_read_context` to call `lifecycle::get_workspace_status_with_context(state.as_ref(), read_context).await?`.
  * In `src/daemon/request_entry.rs`: add `"get_workspace_status"` to `GENERATION_PINNED_READS`. Inside `process_request`'s existing `GENERATION_PINNED_READS` match arm (inline, introducing no new helper functions in `request_entry.rs`), guard the observability fallback strictly on `method == "get_workspace_status"`: when `state.read_server_gate().await` is `None` (no generations root) or `admit_read(&gate, &request).await` returns `ReadAdmission::Refused(_)` (after `admit(state, &request)` already validated the JSON-RPC frame envelope, so refusal is solely due to pending initial activation / no active generation), dispatch `tools::dispatch(Arc::clone(state), method, request.params).await` (`read_context = None`, accepted with no `provenance` and `CodeGraphStats::default()`). For every other method in `GENERATION_PINNED_READS`, `None` gate or `ReadAdmission::Refused(resp)` returns the refusal unchanged. When `admit_read` returns `ReadAdmission::Admitted(context)`, dispatch via `tools::dispatch_with_read_context(Arc::clone(state), method, request.params, Some(&context)).await`.
* **Test scenarios (3)**:
  1. **Pinned read (`OD-5` Q3, `R-1`)**: against a PRE-1/PRE-2 published and activated generation (`code_files == 1`, `functions == 2`), `dispatch_with_read_context(state, "get_workspace_status", None, Some(&ctx))` returns `code_graph.code_files == 1`, `code_graph.functions == 2`, and `/provenance/generation_id == <id>`, while `path`, `branch`, and `db_path` match the workspace snapshot.
  2. **Real generation data vs managed data**: for the same workspace with an unindexed managed `data_dir`, `dispatch(state, "get_workspace_status", None)` returns `code_graph.code_files == 0` and no `provenance` (`pinned["code_graph"]["code_files"] != managed["code_graph"]["code_files"]`).
  3. **No stray databases & quiescence (`4628001C`, `R7.7`)**: the file listing under `ctx.data_dir()` is equal before and after the pinned `get_workspace_status` read (no `connect_db` bootstrap on the generation context), and `GENERATION_PINNED_READS.contains(&"get_workspace_status")` holds while `integration_read_server_generation_wiring` and `integration_lifecycle_read_generation_pin` remain GREEN.
* **F54 case mapping (§9.2)**: enables `get_workspace_status` (`direct_ipc`, `cli`, `stdio_mcp`) provenance and `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` in `142.058-T` (`Slot-20d`).
* **SG-1**: files 2 (`lifecycle.rs`, `generation_pinned_workspace_status_test.rs`, plus glue in `mod.rs`, `request_entry.rs`, and `Cargo.toml`); functions 3 (`get_workspace_status`, `get_workspace_status_with_context`, inline branch in `process_request`; `< 5`); scenarios 3; open ST 0; `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

### Unit 2 — `PA5-T2` (`Slot-19.2`, `142.081-T`): Pinned `query_memory` and `list_symbols`

* **Title**: `PA5-T2: Serve pinned query_memory and list_symbols reads from the opened generation`
* **Slot / `queue_position`**: `Slot-19.2` / `192`
* **Depends on**: `142.080-T` (`PA5-T1`, `E9`)
* **Lineage**: Stash `86F93068`; Deliberation `PA5-C`
* **Execution posture & safety mode**: test-first, Route `(c)`; safety mode: `careful`
* **Files edited**:
  * Production: `src/tools/read.rs` (1 substantive file), plus glue in `src/tools/mod.rs` (2 dispatch arms) and `src/daemon/request_entry.rs` (add `"query_memory"` and `"list_symbols"` to `GENERATION_PINNED_READS`)
  * Test harness: `tests/integration/generation_pinned_memory_and_symbols_test.rs` (new target `integration_generation_pinned_memory_and_symbols`) + 1 `[[test]]` stanza in root `Cargo.toml`
* **What changes**:
  * In `src/tools/read.rs`:
    * **Source-contract invariant (`tests/integration/search_service_pin_test.rs:166-176`)**: `search_service_pin_test.rs` extracts the first function matching `"pub async fn query_memory"` (prefix match without `(`) and asserts that its body contains `"pinned_read_request_context("` and `"query_memory_results("`. Place `pub async fn query_memory_with_context(state: SharedState, params: Option<Value>, read_context: Option<&Arc<ReadRequestContext>>) -> Result<Value, EngramError>` immediately BEFORE `pub async fn query_memory(state: SharedState, params: Option<Value>) -> Result<Value, EngramError>` (which delegates `query_memory_with_context(state, params, None).await`). Inside `query_memory_with_context`: validate `QueryMemoryParams` and `embedding::validate_query_length(&parsed.query)?` first; resolve `context` as `Some(c) => Arc::clone(c)` or `None => pinned_read_request_context(&state, "query_memory").await?`; open `queries_from_read_context(context.as_ref()).await?`; call `query_memory_results(context.as_ref(), &queries, ...)`.
    * Keep `pub async fn list_symbols(state: SharedState, params: Option<Value>) -> Result<Value, EngramError>` as a 1-line wrapper calling `list_symbols_with_context(state, params, None).await`.
    * Add `pub async fn list_symbols_with_context(state: SharedState, params: Option<Value>, read_context: Option<&Arc<ReadRequestContext>>) -> Result<Value, EngramError>`: validate `ListSymbolsParams` and clamp `limit` first; resolve `context` as `Some(c) => Arc::clone(c)` or `None => pinned_read_request_context(&state, "list_symbols").await?`; open `queries_from_read_context(context.as_ref()).await?`; run `cg_queries.list_symbols(&filter).await?`.
  * In `src/tools/mod.rs`: update the `"query_memory"` and `"list_symbols"` arms in `dispatch_with_read_context` to call `read::query_memory_with_context(state.clone(), params, read_context).await` and `read::list_symbols_with_context(state.clone(), params, read_context).await`.
  * In `src/daemon/request_entry.rs`: add `"query_memory"` and `"list_symbols"` to `GENERATION_PINNED_READS`.
* **Test scenarios (3)**:
  1. **Pinned read**: against a PRE-1/PRE-2 published and activated generation, `dispatch_with_read_context` for `"query_memory"` and `"list_symbols"` serves data from the opened generation DB (`total_count == 2` symbols for `alpha` and `beta`) and attaches `/provenance/generation_id == <id>`.
  2. **Real generation data vs managed data**: against an unindexed managed `data_dir` for the same workspace, `dispatch(state, "list_symbols", Some(json!({})))` returns `total_count == 0` and no `provenance` (`pinned["total_count"] != managed["total_count"]`), and `query_memory` carries no `provenance` on the managed path.
  3. **No stray databases & regression**: the file listing under `ctx.data_dir()` is unchanged before and after both pinned reads; `GENERATION_PINNED_READS` contains `"query_memory"` and `"list_symbols"`; `integration_core_read_generation_pin`, `integration_search_service_pin`, and `contract_read_path_pinning_enforcement` remain GREEN without editing those test files.
* **F54 case mapping (§9.2)**: enables `query_memory` (`direct_ipc`, `cli`, `stdio_mcp`) and `list_symbols` (`direct_ipc`, `cli`, `stdio_mcp`) provenance in `142.058-T` (`Slot-20d`).
* **SG-1**: files 2 (`read.rs`, `generation_pinned_memory_and_symbols_test.rs`); functions 4; scenarios 3; open ST 0; `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

### Unit 3 — `PA5-T3` (`Slot-19.3`, `142.082-T`): Pinned `map_code` and `impact_analysis`

* **Title**: `PA5-T3: Serve pinned map_code and impact_analysis reads from the opened generation`
* **Slot / `queue_position`**: `Slot-19.3` / `193`
* **Depends on**: `142.081-T` (`PA5-T2`, `E9`)
* **Lineage**: Stash `86F93068`; Deliberation `PA5-C`
* **Execution posture & safety mode**: test-first, Route `(c)`; safety mode: `careful`
* **Files edited**:
  * Production: `src/tools/read.rs` (1 substantive file), plus glue in `src/tools/mod.rs` (2 dispatch arms) and `src/daemon/request_entry.rs` (add `"map_code"` and `"impact_analysis"` to `GENERATION_PINNED_READS`)
  * Test harness: `tests/integration/generation_pinned_map_and_impact_test.rs` (new target `integration_generation_pinned_map_and_impact`) + 1 `[[test]]` stanza in root `Cargo.toml`
* **What changes**:
  * In `src/tools/read.rs`:
    * **Pinning-seam contract invariant (`tests/contract/read_path_pinning_enforcement_test.rs:195-242`)**: `workspace_snapshot_reads_stay_inside_the_pinning_seams` enforces that `snapshot_dispatch_context()`, `snapshot_workspace()`, and `ReadRequestContext::from_managed_state(` in `src/tools/read.rs` appear ONLY in `pinned_dispatch_context`, `maybe_pinned_dispatch_context`, and `pinned_read_request_context` (1 occurrence each). Never call `state.snapshot_dispatch_context()` directly inside `map_code_with_context` or `impact_analysis_with_context`.
    * Keep `pub async fn map_code(state: SharedState, params: Option<Value>) -> Result<Value, EngramError>` as a 1-line wrapper calling `map_code_with_context(state, params, None).await`.
    * Add `pub async fn map_code_with_context(state: SharedState, params: Option<Value>, read_context: Option<&Arc<ReadRequestContext>>) -> Result<Value, EngramError>`: when `read_context` is `None`, preserve `let context = pinned_dispatch_context(&state, "map_code").await?;`, parse `MapCodeParams`, clamp `depth`/`max_nodes` with `context.config.code_graph`, and open `queries_from_context(&context).await?` (managed path byte-identical, preserving `integration_core_read_generation_pin`). When `read_context` is `Some(ctx)`, parse `MapCodeParams`, read `code_graph` bounds via the existing seam `maybe_pinned_dispatch_context(&state, "map_code").await.map(|s| s.config.code_graph).unwrap_or_default()` (or `state.workspace_config().await.map(|c| c.code_graph).unwrap_or_default()`), and open `queries_from_read_context(ctx.as_ref()).await?` (never calling `connect_db` or `from_managed_state`).
    * Keep `pub async fn impact_analysis(state: SharedState, params: Option<Value>) -> Result<Value, EngramError>` as a 1-line wrapper calling `impact_analysis_with_context(state, params, None).await`.
    * Add `pub async fn impact_analysis_with_context(state: SharedState, params: Option<Value>, read_context: Option<&Arc<ReadRequestContext>>) -> Result<Value, EngramError>` following the exact same `Some(ctx)` vs `None` split as `map_code_with_context`.
  * In `src/tools/mod.rs`: update the `"map_code"` and `"impact_analysis"` arms in `dispatch_with_read_context` to call `read::map_code_with_context(state.clone(), params, read_context).await` and `read::impact_analysis_with_context(state.clone(), params, read_context).await`.
  * In `src/daemon/request_entry.rs`: add `"map_code"` and `"impact_analysis"` to `GENERATION_PINNED_READS`.
* **Test scenarios (3)**:
  1. **Pinned read**: against a PRE-1/PRE-2 published and activated generation where `beta` calls `alpha`, `dispatch_with_read_context` for `"map_code"` and `"impact_analysis"` on `"alpha"` returns the symbol and its neighborhood from the opened generation DB with `/provenance/generation_id == <id>`.
  2. **Real generation data vs managed data**: against an unindexed managed `data_dir` for the same workspace, `dispatch(state, "map_code", ...)` returns `root == null` with no `provenance` and `dispatch(state, "impact_analysis", ...)` returns `CodeGraphError::SymbolNotFound` (`7004`), proving the pinned responses come from the opened generation DB.
  3. **No stray databases & regression**: the file listing under `ctx.data_dir()` is unchanged before and after both pinned reads; `GENERATION_PINNED_READS` contains `"map_code"` and `"impact_analysis"`; `integration_core_read_generation_pin` and `contract_read_path_pinning_enforcement` remain GREEN.
* **F54 case mapping (§9.2)**: enables `map_code` (`direct_ipc`, `cli`, `stdio_mcp`) and `impact_analysis` (`direct_ipc`, `cli`, `stdio_mcp`) provenance in `142.058-T` (`Slot-20d`).
* **SG-1**: files 2 (`read.rs`, `generation_pinned_map_and_impact_test.rs`); functions 4; scenarios 3; open ST 0; `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

### Unit 4 — `PA5-T4a` (`Slot-19.4`, `142.083-T`): Pinned `query_graph`

* **Title**: `PA5-T4a: Serve pinned query_graph read from the opened generation`
* **Slot / `queue_position`**: `Slot-19.4` / `194`
* **Depends on**: `142.082-T` (`PA5-T3`, `E9`)
* **Lineage**: Stash `86F93068`; Deliberation `PA5-C`; split from `PA5-T4` per Plan §4.2 row `19.k` and §7.6 (`PA5-T4 is split (SG-1)`)
* **Execution posture & safety mode**: test-first, Route `(c)`; safety mode: `careful`
* **Files edited**:
  * Production: `src/tools/read.rs` (1 substantive file), plus glue in `src/tools/mod.rs` (1 dispatch arm) and `src/daemon/request_entry.rs` (add `"query_graph"` to `GENERATION_PINNED_READS`)
  * Test harness: `tests/integration/generation_pinned_query_graph_test.rs` (new target `integration_generation_pinned_query_graph`) + 1 `[[test]]` stanza in root `Cargo.toml`
* **What changes**:
  * In `src/tools/read.rs`:
    * Keep `pub async fn query_graph(state: SharedState, params: Option<Value>) -> Result<Value, EngramError>` as a 1-line wrapper calling `query_graph_with_context(state, params, None).await`.
    * Add `pub async fn query_graph_with_context(state: SharedState, params: Option<Value>, read_context: Option<&Arc<ReadRequestContext>>) -> Result<Value, EngramError>`: when `read_context` is `None`, pin `pinned_dispatch_context(&state, "query_graph").await?`, validate `GraphQuery` params, and open `queries_from_context(&context).await?` (managed path byte-identical); when `read_context` is `Some(ctx)`, validate `GraphQuery` params first, then open `queries_from_read_context(ctx.as_ref()).await?` (serving from the opened generation DB, never calling `connect_db`).
  * In `src/tools/mod.rs`: update the `"query_graph"` arm in `dispatch_with_read_context` to call `read::query_graph_with_context(state.clone(), params, read_context).await`.
  * In `src/daemon/request_entry.rs`: add `"query_graph"` to `GENERATION_PINNED_READS`.
* **Test scenarios (3)**:
  1. **Pinned read**: against a PRE-1/PRE-2 published and activated generation where `beta` calls `alpha`, `dispatch_with_read_context` for `"query_graph"` (`neighborhood` / `find_path`) returns the graph nodes/path from the opened generation DB with `/provenance/generation_id == <id>`.
  2. **Real generation data vs managed data**: against an unindexed managed `data_dir` for the same workspace, `dispatch(state, "query_graph", ...)` returns `node_count == 0` / `found == false` with no `provenance` (`pinned["node_count"] != managed["node_count"]`).
  3. **No stray databases & regression**: the file listing under `ctx.data_dir()` is unchanged before and after the pinned read; `GENERATION_PINNED_READS` contains `"query_graph"`; `integration_core_read_generation_pin` remains GREEN.
* **F54 case mapping (§9.2)**: enables `query_graph` (`direct_ipc`, `cli`, `stdio_mcp`) provenance in `142.058-T` (`Slot-20d`).
* **SG-1**: files 2 (`read.rs`, `generation_pinned_query_graph_test.rs`); functions 2; scenarios 3; open ST 0; `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

### Unit 5 — `PA5-T4b` (`Slot-19.5`, `142.084-T`): Pinned `unified_search`

* **Title**: `PA5-T4b: Serve pinned unified_search read from the opened generation`
* **Slot / `queue_position`**: `Slot-19.5` / `195`
* **Depends on**: `142.083-T` (`PA5-T4a`, `E9`)
* **Lineage**: Stash `86F93068`; Deliberation `PA5-C`; split from `PA5-T4` per Plan §4.2 row `19.k` and §7.6 (`PA5-T4 is split (SG-1)`)
* **Execution posture & safety mode**: test-first, Route `(c)`; safety mode: `careful`
* **Files edited**:
  * Production: `src/tools/read.rs` (1 substantive file), plus glue in `src/tools/mod.rs` (1 dispatch arm) and `src/daemon/request_entry.rs` (add `"unified_search"` to `GENERATION_PINNED_READS`)
  * Test harness: `tests/integration/generation_pinned_unified_search_test.rs` (new target `integration_generation_pinned_unified_search`) + 1 `[[test]]` stanza in root `Cargo.toml`
* **What changes**:
  * In `src/tools/read.rs`:
    * Place `pub async fn unified_search_with_context(state: SharedState, params: Option<Value>, read_context: Option<&Arc<ReadRequestContext>>) -> Result<Value, EngramError>` immediately BEFORE `pub async fn unified_search(state: SharedState, params: Option<Value>) -> Result<Value, EngramError>` (which delegates `unified_search_with_context(state, params, None).await`).
    * **Source-contract invariant (`tests/integration/core_read_generation_pin_test.rs:239-252`)**: `core_read_generation_pin_test.rs` extracts the first function matching `"pub async fn unified_search"` and asserts that its body contains `"pinned_queries("` at an index strictly before `"embedding::embed_text("` and contains no `"connect_db("` or `"snapshot_workspace("`. Inside `unified_search_with_context`, resolve `queries` BEFORE `embedding::embed_text(trimmed)` as:
      ```rust
      let queries = match read_context {
          Some(context) => queries_from_read_context(context.as_ref()).await?,
          None => pinned_queries(&state, "unified_search").await?.1,
      };
      ```
      This satisfies `core_read_generation_pin_test.rs` without editing that test file, keeps `pinned_queries` reachable only on the `None` (managed) branch, and serves `Some(context)` from the opened generation DB without ever calling `connect_db`.
  * In `src/tools/mod.rs`: update the `"unified_search"` arm in `dispatch_with_read_context` to call `read::unified_search_with_context(state.clone(), params, read_context).await`.
  * In `src/daemon/request_entry.rs`: add `"unified_search"` to `GENERATION_PINNED_READS`.
* **Test scenarios (3)**:
  1. **Pinned read**: against a PRE-1/PRE-2 published and activated generation, `dispatch_with_read_context` for `"unified_search"` (`query: "alpha"`, `region: "code"`) returns matches from the opened generation DB (`total_count >= 1`) and attaches `/provenance/generation_id == <id>`.
  2. **Real generation data vs managed data**: against an unindexed managed `data_dir` for the same workspace, `dispatch(state, "unified_search", ...)` returns `total_count == 0` and no `provenance` (`pinned["total_count"] != managed["total_count"]`).
  3. **No stray databases & regression**: the file listing under `ctx.data_dir()` is unchanged before and after the pinned read; `GENERATION_PINNED_READS` contains `"unified_search"`; `integration_core_read_generation_pin` (including its `pinned_queries(` < `embedding::embed_text(` source-order assertion) remains GREEN.
* **F54 case mapping (§9.2)**: enables `unified_search` (`direct_ipc`, `cli`, `stdio_mcp`) provenance in `142.058-T` (`Slot-20d`).
* **SG-1**: files 2 (`read.rs`, `generation_pinned_unified_search_test.rs`); functions 2; scenarios 3; open ST 0; `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

### Unit 6 — `PA5-T5` (`Slot-19.6`, `142.085-T`): Pinned `query_changes` (`git-graph`)

* **Title**: `PA5-T5: Serve pinned query_changes read from the opened generation under git-graph`
* **Slot / `queue_position`**: `Slot-19.6` / `196`
* **Depends on**: `142.084-T` (`PA5-T4b`, `E9`)
* **Lineage**: Stash `86F93068`; Deliberation `PA5-C`
* **Execution posture & safety mode**: test-first, Route `(c)`; safety mode: `careful`
* **Files edited**:
  * Production: `src/tools/read.rs` (1 substantive file), plus glue in `src/tools/mod.rs` (1 `#[cfg(feature = "git-graph")]` dispatch arm) and `src/daemon/request_entry.rs` (add `"query_changes"` to `GENERATION_PINNED_READS`)
  * Test harness: `tests/integration/generation_pinned_query_changes_test.rs` (new target `integration_generation_pinned_query_changes`) + 1 `[[test]]` stanza in root `Cargo.toml`
* **What changes**:
  * In `src/tools/read.rs`:
    * **Source-contract invariant (`tests/integration/report_read_generation_pin_test.rs:295-308`)**: `report_read_generation_pin_test.rs` asserts `source.contains("#[cfg(feature = \"git-graph\")]\npub async fn query_changes")` and checks `named_function_body(&source, "query_changes")` for no `"connect_db("` or `"snapshot_workspace("`. Keep `#[cfg(feature = "git-graph")]\npub async fn query_changes(state: SharedState, params: Option<Value>) -> Result<Value, EngramError>` with `#[cfg(feature = "git-graph")]` immediately preceding `pub async fn query_changes`, delegating `query_changes_with_context(state, params, None).await`.
    * Add `#[cfg(feature = "git-graph")]\npub async fn query_changes_with_context(state: SharedState, params: Option<Value>, read_context: Option<&Arc<ReadRequestContext>>) -> Result<Value, EngramError>`: when `read_context` is `None`, preserve `let context = pinned_dispatch_context(&state, "query_changes").await?;`, parse `QueryChangesParams` and RFC-3339 timestamps, and open `queries_from_context(&context).await?` (managed path byte-identical, preserving `query_changes_keeps_the_old_generation_after_publication`); when `read_context` is `Some(ctx)`, parse and validate `QueryChangesParams` and timestamps first, then open `queries_from_read_context(ctx.as_ref()).await?` (serving from the opened generation DB, never calling `connect_db`).
  * In `src/tools/mod.rs`: update the `#[cfg(feature = "git-graph")] "query_changes"` arm in `dispatch_with_read_context` to call `read::query_changes_with_context(state.clone(), params, read_context).await`.
  * In `src/daemon/request_entry.rs`: add `"query_changes"` to `GENERATION_PINNED_READS`.
* **Test scenarios (3)**:
  1. **Default-build & `git-graph` pinned read**: in default builds (`cargo dev-test`), asserts `GENERATION_PINNED_READS.contains(&"query_changes")` and the `src/tools/read.rs` `query_changes_with_context` source contract (failing RED at Step 2 with `Worker: 142.085-T` before implementation); under `--features git-graph` (`cargo test --test integration_generation_pinned_query_changes --features git-graph`), builds and activates a generation containing indexed git commits and verifies `dispatch_with_read_context(state, "query_changes", ..., Some(&ctx))` returns `total == 1` and `/provenance/generation_id == <id>`.
  2. **Real generation data vs managed data (`--features git-graph`)**: against an unindexed managed `data_dir` for the same workspace, `dispatch(state, "query_changes", ...)` returns `total == 0` and no `provenance` (`pinned["total"] != managed["total"]`).
  3. **No stray databases & regression**: under `--features git-graph`, the file listing under `ctx.data_dir()` is unchanged before and after the pinned read; `integration_report_read_generation_pin` passes in both default and `--features git-graph` builds.
* **F54 case mapping (§9.2)**: enables `query_changes` (`direct_ipc`, `stdio_mcp` under `git-graph`) provenance in `142.058-T` (`Slot-20d`).
* **SG-1**: files 2 (`read.rs`, `generation_pinned_query_changes_test.rs`); functions 2; scenarios 3; open ST 0; `Size: S | Complexity: low`, estimate `1 h`. Verdict: **OK**.

### Unit 7 — `PA5-T6` (`Slot-19.7`, `142.086-T`): F19 `ToolDescriptor.generation_backed` field

* **Title**: `PA5-T6: Add generation_backed field to F19 ToolDescriptor for read-server provenance classification`
* **Slot / `queue_position`**: `Slot-19.7` / `197`
* **Depends on**: `142.063-T` (`E7`), `142.066-T` (`PRE-4`), `142.085-T` (`PA5-T5`, `E9`)
* **Lineage**: Stash `86F93068`; Deliberation `PA5-C`, `OD-5` Q1, `R-1`, `R-2` (re-cut per Plan §7.6 so `PA5-T6` edits only `src/tools/capabilities.rs` and its own test; the F54 `DaemonScopedRead` half is in `Slot-20d` `142.058-T`)
* **Execution posture & safety mode**: test-first, Route `(c)`; safety mode: `standard`
* **Files edited**:
  * Production: `src/tools/capabilities.rs` (1 substantive file)
  * Test harness: `tests/contract/tool_descriptor_registry_test.rs` (1 substantive file; existing target `contract_tool_descriptor_registry`)
  * Does **not** edit `tests/contract/read_server_cli_mcp_parity_test.rs` (`PA-7`, `R-2`).
* **What changes**:
  * In `src/tools/capabilities.rs`:
    * Add `pub generation_backed: bool` to `ToolDescriptor` (with doc comment explaining how it refines `read_server_available` + `InputOwnership::DaemonHandler` by distinguishing `GenerationBackedRead` (`GENERATION_PINNED_READS`) from `DaemonScopedRead`) and `generation_backed: bool` to `Declaration`, wired in `all_descriptors()`.
    * At Step 2 (Route `(c)`): `harness-architect` adds `pub generation_backed: bool` initialized to `false` (placeholder) and adds the three contract test scenarios in `tests/contract/tool_descriptor_registry_test.rs` failing with `Worker: 142.086-T`.
    * At Step 4.2: `build-feature` sets `generation_backed: true` on the G1/G2 generation-DB reads (`get_workspace_status`, `get_workspace_statistics`, `query_memory`, `map_code`, `list_symbols`, `unified_search`, `impact_analysis`, `query_graph`, and `#[cfg(feature = "git-graph")] query_changes`) and `generation_backed: false` on all other declarations (`get_daemon_status`, `get_health_report`, `get_branch_metrics`, `get_token_savings_report`, `get_evaluation_report`, `get_mutable_script_retry_metrics`, `get_retrieval_eval_report`, `lint_dax`, `_health`, `_shutdown`, `doctor --smoke`, and all `Write`/`Control` declarations) per `OD-5` Q1 and `R-1`.
* **Test scenarios (3)**:
  1. **Generation-backed read classification (`OD-5` Q1, `R-1`)**: asserts that the exact set of descriptors with `generation_backed == true` is `{"get_workspace_status", "get_workspace_statistics", "query_memory", "map_code", "list_symbols", "unified_search", "impact_analysis", "query_graph"}` in the default build (plus `"query_changes"` under `git-graph`).
  2. **Non-generation read classification (`OD-5` Q1, `R-1`)**: asserts that `get_daemon_status`, the six G3 report/metrics reads (`get_health_report`, `get_branch_metrics`, `get_token_savings_report`, `get_evaluation_report`, `get_mutable_script_retry_metrics`, `get_retrieval_eval_report`), and G4 `lint_dax` remain `CapabilityClass::Read`, `read_server_available: true`, `InputOwnership::DaemonHandler`, and `generation_backed: false`.
  3. **Invariants & `GENERATION_PINNED_READS` parity**: asserts that every descriptor with `generation_backed == true` has `capability == CapabilityClass::Read`, `read_server_available == true`, and `input_ownership == InputOwnership::DaemonHandler`, and that the set of `generation_backed == true` descriptor names in `all_descriptors()` equals `engram::daemon::request_entry::GENERATION_PINNED_READS.iter().copied().filter(|name| cfg!(feature = "git-graph") || *name != "query_changes").collect::<BTreeSet<_>>()` (matching `FEATURE_GATED_MCP_TOOLS` in `tool_descriptor_registry_test.rs`).
* **F54 case mapping (§9.2)**: enables `142.058-T` (`Slot-20d`) to classify non-generation reads (`get_daemon_status`, `get_health_report`, `get_branch_metrics`, `get_token_savings_report`, `get_evaluation_report`, `get_mutable_script_retry_metrics`, `get_retrieval_eval_report`, `lint_dax`) as `ExpectedOutcome::DaemonScopedRead` via `descriptor.generation_backed`.
* **SG-1**: files 2 (`capabilities.rs`, `tool_descriptor_registry_test.rs`); functions 2 (`all_descriptors`, `DECLARATIONS`); scenarios 3; open ST 0; `Size: S | Complexity: low`, estimate `1 h`. Verdict: **OK**.

---

### Units 8–11 — `S-20` Split of `142.058-T` (`Slot-20a` to `Slot-20d`) under Rulings `R-A6` and `R-A7`

Under Ruling `R-A6` (`docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md`), for the `142.058` family (`142.058a`, `142.058b`, `142.058c`, `142.058-T`), the 400-line bounds in Plan §4.2, §4.3 `S-20`, and `SB-1 (c)` count **only the lines the `harness-architect` authors or adapts** (any line that differs from the `6d216d19` blob, including compile-only path/import adaptations). The verbatim load from `6d216d19` (`1,633` lines total) is measured and recorded per slot.

#### `R-A6` Measurement of `6d216d19:tests/contract/read_server_cli_mcp_parity_test.rs` (1,633 lines)

| Slot | Task ID | Unit | Verbatim `6d216d19` regions loaded | Verbatim line count | Authored / adapted lines (capped at $\le 400$ by `R-A6`) | `SG-1` verdict |
|---|---|---|---|---|---|---|
| `Slot-20a` | `142.087-T` (`142.058a`) | `F54a` harness core (FL), coverage assertion, declared-surface rule, `_health` on direct IPC only | `6d216d19` lines `1..=1351` (shared harness core `1..=1319` + structural/coverage test `1320..=1351`; full `1..=1633` also exercised during Step 2 `V3`/`V4` FL runs) | `1,351` lines (`1,633` at `V3`/`V4`) | **~52 lines** ($\le 400$): (1) `#[path = "../helpers/activation_settle.rs"] mod activation_settle;` (1 line); (2) `activation_settle::await_activation_settled(&self.endpoint, self.temporary_root.path(), 1).await?;` in `ensure_daemon` (3 lines); (3) `read_server_layout` replacing `workspace_hash` + import in `publish_fixture_generation` (8 lines); (4) `Slot-20a` test for declared-surface rule (`get_retrieval_eval_report` has no hard-coded CLI absence) and `_health` on direct IPC only (40 lines, adapted from `1352..=1405`); plus `(d2)` verification that all `6d216d19` child `Command` spawns (`spawn_daemon`, `invoke_cli`, `invoke_stdio_mcp`) already strip `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, and `ENGRAM_DIRECT` | **OK** (`S \| medium`, 1.5 h) |
| `Slot-20b` | `142.088-T` (`142.058b`) | `F54b` refusal classes with no side effects, and unknown methods (`.003-ST`) | `6d216d19` lines `1407..=1562` (`control_descriptors_are_refused_without_side_effects`, `unknown_ipc_methods_are_refused_without_side_effects`, `exercise_unknown_method`) | `156` lines | **~75 lines** ($\le 400$): (1) refusal-class matrix test covering `Write`, `Control`, unsafe lifecycle `set_workspace`, and direct mutation workflow descriptors across declared surfaces (`ExpectedOutcome::Refusal { .. }`, adapted from `1352..=1405`, ~45 lines); (2) if needed for stdio MCP refusal framing, read-before-stdin-close adaptation in `invoke_stdio_mcp` (~30 lines) | **OK** (`S \| medium`, 1.5 h) |
| `Slot-20c` | `142.089-T` (`142.058c`) | `F54c` real indexed fixture replacing `probe_row` (`PA5-T7`, `OD-5` Q2, `V5`) | Replaces `probe_row` in `6d216d19` lines `476..=525` (`publish_fixture_generation`) and updates `request_parameters` / `cli_arguments` `query_graph` root (`537..=545`, `867..=873`) | `0` net new verbatim lines (`78` lines of `6d216d19` context updated) | **~75 lines** ($\le 400$): (1) `publish_fixture_generation` rewritten to build, seal, and publish a real indexed generation of the fixture's `src/f54_fixture.rs` (`f54_fixture_root`, `f54_fixture_caller`) via `read_server_layout`, `open_generation_store`, `GenerationId::new(EXPECTED_GENERATION)`, `build_candidate_generation`, `seal_candidate_inventory`, `publish_sealed_candidate` (~45 lines); (2) `Slot-20c` test verifying the real indexed generation activates at revision 1 and resolves `f54_fixture_root` (~30 lines) | **OK** (`S \| medium`, 1.5 h) |
| `Slot-20d` | `142.058-T` (parent) | `F54d` read success and provenance, `DaemonScopedRead` (`OD-5` Q1, F54 half of `PA5-T6`), CLI/MCP equivalence, full matrix GREEN (`.002-ST`, `M12`) | `6d216d19` lines `1352..=1405` (`generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior`) and `1563..=1633` (`normalize_parity_result`, `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read`) | `125` lines | **~35 lines** ($\le 400$): (1) `ExpectedOutcome::DaemonScopedRead` variant; (2) `generation_read` / `expected_outcome` updated to use `descriptor.generation_backed` (`true` -> `GenerationBackedRead`, `false` on `Read` + `read_server_available` + `DaemonHandler` -> `DaemonScopedRead`); (3) `record_matrix_failures` arm for `DaemonScopedRead` asserting `observation.accepted`, `observation.generation.is_none()`, and `observation.side_effect_count == 0` | **OK** (`S \| medium`, 1.5 h) |

#### Pre-Enumerated `B2 (vi)` PA-5 Row Changes for the `probe_row` Fixture (Plan `R9.5` item 3)

At `Slot-20a` Step 2 (`V3`, `V4`), Ship runs the full `6d216d19` FL body (with the `probe_row` fixture, `activation_settle::await_activation_settled`, and `read_server_layout`) three consecutive times and compares the per-case maps to `B0` (`docs/memory/2026-10-04-ship-142-f-a0-b0.md`, on tag `T0 = parked/142-s-split-0f2cca20`) under rule `B2`. Because `Slot-20a` runs after `PA5-T1`..`PA5-T6` (`Slot-19.1`..`Slot-19.7`), `B2` item `(vi)` requires this plan to enumerate in advance every row change caused by `PA5-T1`..`PA5-T6` against the `probe_row` fixture:

| `B0` test / row | `B0` status / signature | Expected FL status / signature after `PA5-T1`..`PA5-T6` against `probe_row` | Attributable `PA-5` task ID |
|---|---|---|---|
| Row 5 `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read`, and Row 2 `get_workspace_status` (`direct_ipc`, `cli`, `stdio_mcp`) | `RED`: `expected serving-generation provenance` (`generation=None`) | `get_workspace_status` now runs through `admit_read` + `get_workspace_status_with_context`, tolerates the `probe_row` DB via `.unwrap_or(0)` counts, and carries `provenance.generation_id == "generation-142"`. Row 2 `get_workspace_status` passes on all declared surfaces (unless `stdio_mcp` hits the `6d216d19` `drop(stdin)` race), and Row 5 turns GREEN (or fails only if `stdio_mcp` hits `drop(stdin)`). | `142.080-T` (`PA5-T1`) |
| Row 2 `query_memory` (`direct_ipc`, `cli`, `stdio_mcp`) | `RED`: `expected serving-generation provenance` (`accepted=true, generation=None` from managed DB) | Routed through `admit_read` + `query_memory_with_context` against `probe_row` DB (which has no `content_record` relation): fails with `expected an accepted read result` (missing relation in `probe_row` until `142.089-T` `Slot-20c` lands the real indexed fixture). | `142.081-T` (`PA5-T2`) |
| Row 2 `list_symbols` (`direct_ipc`, `cli`, `stdio_mcp`) | `RED`: `expected serving-generation provenance` (`accepted=true, generation=None` from managed DB) | Routed through `admit_read` + `list_symbols_with_context` against `probe_row` DB: fails with `expected an accepted read result` (missing symbol relations in `probe_row` until `142.089-T` `Slot-20c`). | `142.081-T` (`PA5-T2`) |
| Row 2 `map_code` and `impact_analysis` (`direct_ipc`, `cli`, `stdio_mcp`) | `RED`: `expected serving-generation provenance` (`accepted=true, generation=None` from managed DB) | Routed through `admit_read` + `map_code_with_context` / `impact_analysis_with_context` against `probe_row` DB: fails with `expected an accepted read result` (missing symbol relations / `SymbolNotFound` in `probe_row` until `142.089-T` `Slot-20c`). | `142.082-T` (`PA5-T3`) |
| Row 2 `query_graph` (`direct_ipc`, `cli`, `stdio_mcp`) | `RED`: `expected serving-generation provenance` (`accepted=true, generation=None` from managed DB) | Routed through `admit_read` + `query_graph_with_context` against `probe_row` DB: fails with `expected an accepted read result` (missing graph relations in `probe_row` until `142.089-T` `Slot-20c`). | `142.083-T` (`PA5-T4a`) |
| Row 2 `unified_search` (`direct_ipc`, `cli`, `stdio_mcp`) | `RED`: `expected serving-generation provenance` (`accepted=true, generation=None` from managed DB) | Routed through `admit_read` + `unified_search_with_context` against `probe_row` DB: fails with `expected an accepted read result` (missing vector/symbol relations in `probe_row` until `142.089-T` `Slot-20c`). | `142.084-T` (`PA5-T4b`) |
| Row 2 `query_changes` (`direct_ipc`, `stdio_mcp`, under `git-graph` only) | Not in default-build `B0` | Under `git-graph`, routed through `admit_read` + `query_changes_with_context` against `probe_row` DB: fails with `expected an accepted read result` until `142.089-T` `Slot-20c`. | `142.085-T` (`PA5-T5`) |

#### Unit 8 — `142.058a` (`Slot-20a`, `142.087-T`): F54a harness core (FL), coverage assertion, declared-surface rule, `_health` on direct IPC only

* **Title**: `F54a: Land parity harness core (FL), descriptor coverage, declared-surface rule and direct-IPC _health`
* **Slot / `queue_position`**: `Slot-20a` / `200`
* **Depends on**: `142.063-T` (`E3`), `142.066-T`, `142.065-T` (`E15`), and the PA-5 tasks (`142.086-T` `PA5-T6` sink, plus `142.080-T`..`142.085-T`, `E6`)
* **Route**: Route `(c)` (marker `Worker: 142.087-T`); FL provenance and `B2` verification at Step 2 (`V2`, `V3`, `V4`).
* **Files edited**: `tests/contract/read_server_cli_mcp_parity_test.rs` only (`PA-7`, `V1`; includes `tests/helpers/activation_settle.rs` via `#[path]` and never edits it).
* **Test scenarios (3)**:
  1. `generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces` (`058-T AC1`, `AC2`, `V2`, `058.001-ST AC1`–`AC5`, `V1`): matrix derived programmatically from `all_descriptors()`, every descriptor appears once, surfaces match F19 declarations.
  2. Declared-surface rule (`058-T AC7`, `058.002-ST AC4`): `get_retrieval_eval_report` is exercised only on the surfaces its F19 descriptor declares, with no hard-coded assertion that it lacks a CLI surface.
  3. Direct-IPC `_health` (`058-T AC6`, `058.002-ST AC5`): `_health` is declared on `DirectIpc` only and returns a ready health response on the settled read-server fixture daemon.
* **SG-1**: files 1 (`read_server_cli_mcp_parity_test.rs`); functions 4 (adapted/authored: `ensure_daemon`, `publish_fixture_generation`, `descriptor_parity_matrix`, plus the `Slot-20a` test); scenarios 3; open ST 0; authored/adapted lines ~52 ($\le 400$, `R-A6`); `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

#### Unit 9 — `142.058b` (`Slot-20b`, `142.088-T`): F54b refusal classes with no side effects, and unknown methods

* **Title**: `F54b: Assert read-server refusal classes, unknown IPC methods and absence of side effects`
* **Slot / `queue_position`**: `Slot-20b` / `201`
* **Depends on**: `142.087-T` (`142.058a`, `E15`)
* **Lineage**: Carries `142.058.003-ST` (Plan §4.3 `S-20`)
* **Route**: Route `(c)` (marker `Worker: 142.088-T`)
* **Files edited**: `tests/contract/read_server_cli_mcp_parity_test.rs` only (`PA-7`, `V1`)
* **Test scenarios (3)**:
  1. `control_descriptors_are_refused_without_side_effects` (`058.003-ST AC2`, `AC3`, `AC6`): `Control` descriptors (`set_workspace` retargeting with `16_003`, `_shutdown` with `16_001`) are refused on every declared surface with zero filesystem or workspace-binding side effects.
  2. `unknown_ipc_methods_are_refused_without_side_effects` (`058.003-ST AC5`, `AC6`): undeclared IPC method (`not_a_declared_method`) is refused with `16_001` without triggering generation activation or side effects.
  3. Non-Read capability refusal matrix (`058-T AC4`, `AC5`, `058.003-ST AC1`, `AC4`, `AC6`, `V1`–`V3`): every `Write` and non-`Read` descriptor in the generated matrix is refused on every declared surface with stable F38 code `16_001` and zero side effects.
* **SG-1**: files 1 (`read_server_cli_mcp_parity_test.rs`); functions 3 (`control_descriptors_are_refused_without_side_effects`, `unknown_ipc_methods_are_refused_without_side_effects`, `exercise_unknown_method` + non-Read refusal test); scenarios 3; open ST 0; authored/adapted lines ~75 ($\le 400$, `R-A6`); `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

#### Unit 10 — `142.058c` (`Slot-20c`, `142.089-T`): F54c real indexed fixture replacing `probe_row`

* **Title**: `F54c: Publish real indexed generation in F54 parity fixture replacing probe_row`
* **Slot / `queue_position`**: `Slot-20c` / `202`
* **Depends on**: `142.088-T` (`142.058b`, `E15`)
* **Lineage**: Deliberation `PA5-T7`, `OD-5` Q2, `R-2`, `R9.7 V5`, Plan §4.3 `S-20`
* **Route**: Route `(c)` (marker `Worker: 142.089-T`)
* **Files edited**: `tests/contract/read_server_cli_mcp_parity_test.rs` only (`PA-7`, `V1`, `V5`)
* **What changes**:
  * Replaces the manual `:create probe_row {id => val}` SQLite setup in `publish_fixture_generation` with the `PRE-1`/`PRE-2` candidate build, seal, and publish pipeline (`read_server_layout`, `open_generation_store(&layout, true)`, `GenerationId::new(EXPECTED_GENERATION)`, `build_candidate_generation(&layout, &store, &id, &CodeGraphConfig::default()).await`, `seal_candidate_inventory(built)`, `publish_sealed_candidate(&layout, &store, sealed, "f54-contract-fixture")`) over the fixture workspace's `src/f54_fixture.rs` (`pub fn f54_fixture_root() {}` and `pub fn f54_fixture_caller() { f54_fixture_root(); }`).
  * Removes the unindexed managed `code_graph::sync_workspace` seed from `ReadServerFixture::new` (or keeps the managed `.engram` directory unindexed) so that generation-backed reads genuinely prove they read the published generation rather than managed state.
  * Ensures `map_code` (`cli_arguments`: `"f54_fixture_root"`), `impact_analysis` (`"f54_fixture_root"`), and `query_graph` resolve valid symbols/roots in the published generation.
* **Test scenarios (2)**:
  1. Fixture generation activation & real indexed symbol witness (`V5`, `OD-5` Q2): the read-server daemon activates revision 1 (`EXPECTED_GENERATION`) of the real indexed generation and settles (`Settled::Active`), and `get_workspace_statistics` / `list_symbols` over direct IPC observe `code_files == 1`, `functions == 2` (`f54_fixture_root`, `f54_fixture_caller`) with `/provenance/generation_id == EXPECTED_GENERATION`.
  2. Existing `Slot-20a` and `Slot-20b` tests remain GREEN against the upgraded real indexed fixture.
* **SG-1**: files 1 (`read_server_cli_mcp_parity_test.rs`); functions 3 (`ReadServerFixture::new`, `publish_fixture_generation`, `cli_arguments`); scenarios 2; open ST 0; authored/adapted lines ~75 ($\le 400$, `R-A6`); `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

#### Unit 11 — `142.058-T` (`Slot-20d`, parent rewritten): F54d read provenance, `DaemonScopedRead`, CLI/MCP equivalence, full matrix GREEN

* **Title**: `F54d: Assert generation-backed read provenance, DaemonScopedRead, CLI/MCP equivalence and close F54`
* **Slot / `queue_position`**: `Slot-20d` / `203`
* **Depends on**: `142.089-T` (`142.058c`, `E15`), `142.066-T`, `142.065-T`, plus the 15 archived `done` predecessors (`142.001-T`, `142.010-T`, `142.023-T`..`142.026-T`, `142.028-T`, `142.031-T`, `142.032-T`, `142.046-T`, `142.048-T`..`142.051-T`, `142.053-T`)
* **Lineage**: Carries `142.058.002-ST`, the F54 half of `PA5-T6` (`DaemonScopedRead`, `OD-5` Q1, `R-1`, `R-2`), and `M12` (`V6`–`V8`, Plan §4.3 `S-20`)
* **Route**: Route `(c)` (marker `Worker: 142.058-T`)
* **Files edited**: `tests/contract/read_server_cli_mcp_parity_test.rs` only (`PA-7`, `V1`)
* **What changes**:
  * Adds `ExpectedOutcome::DaemonScopedRead` to `ExpectedOutcome` in `tests/contract/read_server_cli_mcp_parity_test.rs`.
  * Updates `generation_read(descriptor)` to check `descriptor.generation_backed` (added in `PA5-T6` `142.086-T`), and updates `expected_outcome(descriptor)` so descriptors with `capability == CapabilityClass::Read && read_server_available && input_ownership == InputOwnership::DaemonHandler` map to `ExpectedOutcome::GenerationBackedRead` when `descriptor.generation_backed` is `true` and to `ExpectedOutcome::DaemonScopedRead` when `descriptor.generation_backed` is `false` (`get_daemon_status`, the six G3 report/metrics reads, and G4 `lint_dax`).
  * In `record_matrix_failures`, adds the `ExpectedOutcome::DaemonScopedRead` arm asserting `observation.accepted == true`, `observation.generation.is_none()` (no false generation provenance claim), and `observation.side_effect_count == 0`.
  * Lands `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` (`6d216d19` lines `1352..=1405`) and `normalize_parity_result` + `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` (`6d216d19` lines `1563..=1633`).
  * Produces the `M12` / `V6` closure evidence: three consecutive identical full-output runs of `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture` with every `F54 settle:` line `Active` and every test case GREEN, plus `cargo dev-test --no-fail-fast` `PASS`.
* **Test scenarios (3)**:
  1. `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` (`058-T AC3`, `058.002-ST AC1`, `AC2`, `OD-5` Q1): every `GenerationBackedRead` row succeeds with `generation == Some(EXPECTED_GENERATION)` across all declared surfaces; every `DaemonScopedRead` row succeeds with `generation == None` and `side_effect_count == 0` across all declared surfaces; `ReadyHealth`, `SuccessfulWorkflow`, and `Refusal` rows all pass.
  2. `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` (`058-T AC8`, `058.002-ST AC3`, `V2`): `get_workspace_status` succeeds on both CLI and stdio MCP with `generation == Some(EXPECTED_GENERATION)` and equivalent normalized payloads.
  3. Three consecutive identical full-output runs of `contract_read_server_cli_mcp_parity` with every `F54 settle:` line `Active` (`M12`, `V6`, `V8`).
* **SG-1**: files 1 (`read_server_cli_mcp_parity_test.rs`); functions 3 adapted (`generation_read`, `expected_outcome`, `record_matrix_failures`, plus 1 enum variant `ExpectedOutcome::DaemonScopedRead`; `normalize_parity_result` and the 2 test functions are verbatim `6d216d19` loads under `R-A6`); scenarios 3; open ST 0 (`142.058.002-ST` and `142.058.003-ST` stay records closed by Stage under §10.1 after `Slot-20d`); authored/adapted lines ~35 ($\le 400$, `R-A6`); `Size: S | Complexity: medium`, estimate `1.5 h`. Verdict: **OK**.

#### Complete `S-20` Acceptance-Criteria & Verification Map (Plan §4.3, `R-A4`, `R-A6`, `R-A7`)

Every item from `142.058-T`, `142.058.001-ST` (archived `done`), `142.058.002-ST`, `142.058.003-ST`, `R9.7` (`V1`–`V8`), and `R6.5`/`R7.1`/`R9.5` (`M1`–`M15`) maps to **exactly one** `S-20` family task:

| Item | Original text (start) | Owner |
|---|---|---|
| `058-T AC1` | The matrix is GENERATED from descriptor declarations, not hand-maintained | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058-T AC2` | Each descriptor is exercised only on the surfaces it declares | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058-T AC3` | Read descriptors assert success and correct generation provenance on every declared surface (with `DaemonScopedRead` for non-generation reads per `OD-5` Q1) | `142.058-T` (`Slot-20d`) |
| `058-T AC4` | Write, Control, unsafe lifecycle, direct mutation workflow and unknown methods assert refusal with the stable F38 code | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058-T AC5` | Every refusal case asserts NO side effects occurred | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058-T AC6` | `_health` (an internal direct-IPC liveness probe, not a user-facing tool) is exercised on direct IPC only | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058-T AC7` | No test hard-codes the ABSENCE of a CLI surface for any user-facing tool (`get_retrieval_eval_report` exercised on declared surfaces) | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058-T AC8` | CLI and MCP produce equivalent results for the same read | `142.058-T` (`Slot-20d`) |
| `058-T V1` | `cargo test --test contract_read_server_cli_mcp_parity` is GREEN (whole family matrix) | `142.058-T` (`Slot-20d`) |
| `058-T V2` | Coverage assertion: every descriptor appears in the generated matrix | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058-T V3` | `cargo dev-test`, `cargo ci` (whole F54 close) | `142.058-T` (`Slot-20d`) |
| `058.001-ST AC1` | The matrix is derived programmatically from descriptor declarations | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058.001-ST AC2` | Every descriptor appears in the matrix exactly once | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058.001-ST AC3` | A descriptor declaring one surface produces exactly one surface row | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058.001-ST AC4` | Adding a descriptor changes the matrix with no manual harness edit | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058.001-ST AC5` | No hand-maintained surface list remains in the harness | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058.001-ST V1` | Matrix coverage assertion against the F19 descriptor list | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058.002-ST AC1` | Each Read descriptor succeeds on every surface it declares | `142.058-T` (`Slot-20d`) |
| `058.002-ST AC2` | Each successful response carries provenance identifying the serving generation (`GenerationBackedRead` vs `DaemonScopedRead`, `OD-5` Q1) | `142.058-T` (`Slot-20d`) |
| `058.002-ST AC3` | CLI and stdio MCP return equivalent results for the same read | `142.058-T` (`Slot-20d`) |
| `058.002-ST AC4` | `get_retrieval_eval_report` is asserted on exactly the surfaces its F19 descriptor declares; no hard-coded CLI absence | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058.002-ST AC5` | `_health` is asserted on direct IPC only | `142.087-T` (`142.058a`, `Slot-20a`) |
| `058.002-ST V1` | Success and provenance assertions across all declared read surfaces | `142.058-T` (`Slot-20d`) |
| `058.002-ST V2` | CLI/MCP equivalence assertion | `142.058-T` (`Slot-20d`) |
| `058.003-ST AC1` | Write capability calls are refused in ReadServer mode with the stable F38 code | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058.003-ST AC2` | Control capability calls (including `_shutdown`) are refused | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058.003-ST AC3` | Unsafe lifecycle calls - notably workspace retargeting - are refused | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058.003-ST AC4` | Direct mutation workflows are refused and direct operators to `engram-indexer` | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058.003-ST AC5` | Unknown methods are refused without activating a generation | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058.003-ST AC6` | EVERY refusal case asserts no side effects: no generation change, no workspace mutation, no file write | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058.003-ST V1` | Refusal assertions for every non-Read capability class | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058.003-ST V2` | Before/after state equality assertion for each refused call | `142.088-T` (`142.058b`, `Slot-20b`) |
| `058.003-ST V3` | Assertion that refused and unknown methods do not trigger activation | `142.088-T` (`142.058b`, `Slot-20b`) |
| `R9.7 V1` | Sole editor of `tests/contract/read_server_cli_mcp_parity_test.rs` (PA-7); includes `tests/helpers/activation_settle.rs` via `#[path]` and never edits it | `142.087-T` (`142.058a`, `Slot-20a`) |
| `R9.7 V2` | Provenance revalidation `R9.5` item 1 (a)–(e) and `(d2)`, recorded before FL | `142.087-T` (`142.058a`, `Slot-20a`) |
| `R9.7 V3` | The FL commit, `R9.5` item 2, including `await_activation_settled` placement in `ensure_daemon` before `binding_before`/`files_before` | `142.087-T` (`142.058a`, `Slot-20a`) |
| `R9.7 V4` (`M1` FL, `M2`–`M5`, `M7`–`M11`, `M13`, `M14`, `M15` FL) | Three FL runs compared with `B0` under `B2` (`R9.5` item 3), using the `T2` per-row signature definition | `142.087-T` (`142.058a`, `Slot-20a`) |
| `R9.7 V5` | The real-snapshot fixture change specified by the reviewed PA-5 plan (`OD-5` Q2, `PA5-T7`), in this file only | `142.089-T` (`142.058c`, `Slot-20c`) |
| `R9.7 V6` (`M1` M12, `M6`, `M12`, `M15` M12) | Three consecutive identical F54 runs after PA-5 and the real-snapshot fixture land, every `F54 settle:` line `Active`, every case GREEN, Step 4.3 `PASS` | `142.058-T` (`Slot-20d`) |
| `R9.7 V7` | History kept: `.001-ST` done; `7bd9e504` and `6d216d19` records and `B0` stay; later records added, not substituted | `142.058-T` (`Slot-20d`) |
| `R9.7 V8a` | Verification steps 1–2: provenance record (`V2`) and FL record + three FL maps with `B2` comparison (`V4`) | `142.087-T` (`142.058a`, `Slot-20a`) |
| `R9.7 V8b` | Verification steps 3–4: `M12` maps and `cargo dev-test --no-fail-fast` `PASS`, lint and format, `cargo ci` | `142.058-T` (`Slot-20d`) |

---

## Dependency Graph

### Task Edges (`blocks`, Plan §5.1 & Ruling `R-A7`)

| Edge | From (dependent task) | To (predecessor task) | Category |
|---|---|---|---|
| `E7` | `142.080-T` (`PA5-T1`, `Slot-19.1`) | `142.063-T` (`PRE-3F`, `Slot-19`), `142.065-T` (`PRE-4b`, `Slot-05`), `142.066-T` (`PRE-4`, `Slot-06`) | PA-5 root → `142.063-T` + PRE-4b/PRE-4 |
| `E9` | `142.081-T` (`PA5-T2`, `Slot-19.2`) | `142.080-T` (`PA5-T1`, `Slot-19.1`) | PA-5 internal chain |
| `E9` | `142.082-T` (`PA5-T3`, `Slot-19.3`) | `142.081-T` (`PA5-T2`, `Slot-19.2`) | PA-5 internal chain |
| `E9` | `142.083-T` (`PA5-T4a`, `Slot-19.4`) | `142.082-T` (`PA5-T3`, `Slot-19.3`) | PA-5 internal chain |
| `E9` | `142.084-T` (`PA5-T4b`, `Slot-19.5`) | `142.083-T` (`PA5-T4a`, `Slot-19.4`) | PA-5 internal chain |
| `E9` | `142.085-T` (`PA5-T5`, `Slot-19.6`) | `142.084-T` (`PA5-T4b`, `Slot-19.5`) | PA-5 internal chain |
| `E7`, `E9` | `142.086-T` (`PA5-T6`, `Slot-19.7`) | `142.085-T` (`PA5-T5`, `Slot-19.6`), `142.063-T` (`Slot-19`), `142.066-T` (`Slot-06`) | PA-5 internal chain + `142.063-T`/`142.066-T` |
| `E3`, `E6`, `E15` | `142.087-T` (`142.058a`, `Slot-20a`) | `142.063-T` (`E3`), `142.066-T`, `142.065-T` (`E15`), `142.086-T` (`E6`, plus `142.085-T`) | `142.058a` → `063`, `066`, `065`, PA-5 sinks |
| `E15` | `142.088-T` (`142.058b`, `Slot-20b`) | `142.087-T` (`142.058a`, `Slot-20a`) | `S-20` chain |
| `E15` | `142.089-T` (`142.058c`, `Slot-20c`) | `142.088-T` (`142.058b`, `Slot-20b`) | `S-20` chain |
| `E15` | `142.058-T` (`F54d`, `Slot-20d`) | `142.089-T` (`142.058c`, `Slot-20c`) (plus existing `142.066-T`, `142.065-T`, and 15 archived `done` predecessors) | `S-20` chain |
| Existing | `142.059-T` (`F55`, `Slot-21`) | `142.058-T` (`Slot-20d`), `142.055-T` (`Slot-16`), `142.057-T` (`Slot-18`), plus archived `142.025-T`, `142.026-T` | `Slot-21` after `Slot-20d`, `Slot-16`, `Slot-18` |

### One-Task Shipments, Mirrored `blocks` Edges, and `queue_position` (Plan §4, §5.2, §7.6)

| Slot | Expected Shipment ID | Task ID | Unit | `queue_position` | Mirrored Shipment `blocks` Predecessors |
|---|---|---|---|---|---|
| `Slot-19.1` | `167-S` | `142.080-T` | `PA5-T1` | `191` | `166-S` (`Slot-19`), `150-S` (`Slot-05`), `151-S` (`Slot-06`) |
| `Slot-19.2` | `168-S` | `142.081-T` | `PA5-T2` | `192` | `167-S` (`Slot-19.1`) |
| `Slot-19.3` | `169-S` | `142.082-T` | `PA5-T3` | `193` | `168-S` (`Slot-19.2`) |
| `Slot-19.4` | `170-S` | `142.083-T` | `PA5-T4a` | `194` | `169-S` (`Slot-19.3`) |
| `Slot-19.5` | `171-S` | `142.084-T` | `PA5-T4b` | `195` | `170-S` (`Slot-19.4`) |
| `Slot-19.6` | `172-S` | `142.085-T` | `PA5-T5` | `196` | `171-S` (`Slot-19.5`) |
| `Slot-19.7` | `173-S` | `142.086-T` | `PA5-T6` | `197` | `172-S` (`Slot-19.6`), `166-S` (`Slot-19`), `151-S` (`Slot-06`) |
| `Slot-20a` | `174-S` | `142.087-T` | `F54a` | `200` | `166-S` (`Slot-19`), `151-S` (`Slot-06`), `150-S` (`Slot-05`), `173-S` (`Slot-19.7`), `172-S` (`Slot-19.6`) |
| `Slot-20b` | `175-S` | `142.088-T` | `F54b` | `201` | `174-S` (`Slot-20a`) |
| `Slot-20c` | `176-S` | `142.089-T` | `F54c` | `202` | `175-S` (`Slot-20b`) |
| `Slot-20d` | `177-S` | `142.058-T` | `F54d` | `203` | `176-S` (`Slot-20c`), `151-S` (`Slot-06`), `150-S` (`Slot-05`) |
| `Slot-21` | `178-S` | `142.059-T` | `F55` | `210` | `177-S` (`Slot-20d`), `163-S` (`Slot-16`), `165-S` (`Slot-18`) |

## Decisions and Rationale

1. **Option `PA5-C` (`OD-5` Q1–Q3, `R-1`)**: Only reads whose data genuinely comes from the generation database (`get_workspace_status` `code_graph` counts, `get_workspace_statistics`, `query_memory`, `map_code`, `list_symbols`, `unified_search`, `impact_analysis`, `query_graph`, and `query_changes` under `git-graph`) are converted to read through `queries_from_read_context` and join `GENERATION_PINNED_READS`. Non-generation reads (`get_daemon_status`, the six G3 report/metrics reads, and G4 `lint_dax`) keep their existing data paths, declare `generation_backed: false` on `ToolDescriptor` (`PA5-T6`), and are expected by F54 (`Slot-20d`) as `ExpectedOutcome::DaemonScopedRead` (accepted with no generation provenance and zero side effects).
2. **`get_workspace_status` counts-only conversion and no-gate observability (`OD-5` Q3, `R-1`, `R6.8`)**: In `get_workspace_status_with_context`, only `code_graph` changes source when `read_context` carries an admitted generation (matching directly on `read_context.and_then(|ctx| ctx.generation())` in `src/tools/lifecycle.rs` so `src/tools/read.rs` stays untouched in `PA5-T1`). `path`, `branch`, `db_path`, `stale_files`, `scan_status`, and `generation` continue to report the managed binding and activator observability snapshot so F54's `workspace_binding_fingerprint` remains stable. Furthermore, when a `ReadServer` daemon has no generation gate installed (`state.read_server_gate().await` is `None`) or no generation is active yet (`admit_read` refuses while the frame is valid), `process_request` falls through to `tools::dispatch` (`read_context = None`, returning `CodeGraphStats::default()` without calling `connect_db`) so `get_workspace_status` stays accepted with no provenance on no-gate / pre-activation daemons (`R6.8`, `142.066-T` Scenario 3, and `PRE-3F` `NoActivator`).
3. **`PA-7` & `R-2` F54 file boundary**: No `PA-5` task (`Slot-19.1`..`Slot-19.7`) touches `tests/contract/read_server_cli_mcp_parity_test.rs`. Deliberation `PA5-T7` (real indexed fixture) is `Slot-20c` (`142.089-T`), and the F54 half of `PA5-T6` (`DaemonScopedRead`) is `Slot-20d` (`142.058-T`).
4. **Sequential `PA-5` ordering (`E9`)**: `PA5-T1` through `PA5-T6` are chained sequentially (`191..=197`) because `PA5-T1`..`PA5-T5` all edit `src/tools/mod.rs` and `src/daemon/request_entry.rs` (`GENERATION_PINNED_READS`), `PA5-T2`..`PA5-T5` all edit `src/tools/read.rs`, and `PA5-T6` verifies that the `generation_backed == true` set equals `GENERATION_PINNED_READS`.
5. **Preserving existing source-contract tests (`read_path_pinning_enforcement_test`, `search_service_pin_test`, `core_read_generation_pin_test`, and `report_read_generation_pin_test`)**:
   * `tests/contract/read_path_pinning_enforcement_test.rs::workspace_snapshot_reads_stay_inside_the_pinning_seams` enforces that `snapshot_dispatch_context()`, `snapshot_workspace()`, and `ReadRequestContext::from_managed_state(` in `src/tools/read.rs` appear ONLY in `pinned_dispatch_context`, `maybe_pinned_dispatch_context`, and `pinned_read_request_context` (1 occurrence each). `PA5-T3` (`map_code_with_context`, `impact_analysis_with_context`) therefore reads `CodeGraphConfig` on the `Some(ctx)` branch via `maybe_pinned_dispatch_context(&state, "<method>").await.map(|s| s.config.code_graph).unwrap_or_default()` (or `state.workspace_config().await.map(|c| c.code_graph).unwrap_or_default()`) and never calls `state.snapshot_dispatch_context()` directly.
   * `tests/integration/search_service_pin_test.rs` extracts the first function matching `"pub async fn query_memory"` and asserts `pinned_read_request_context(` and `query_memory_results(` appear in its body. Placing `pub async fn query_memory_with_context` immediately before `pub async fn query_memory` in `PA5-T2` preserves that assertion without editing `search_service_pin_test.rs`.
   * `tests/integration/core_read_generation_pin_test.rs` extracts the first function matching `"pub async fn unified_search"` and asserts `pinned_queries(` appears before `embedding::embed_text(`. Placing `pub async fn unified_search_with_context` immediately before `pub async fn unified_search` and keeping `None => pinned_queries(&state, "unified_search").await?.1` inside `unified_search_with_context` in `PA5-T4b` preserves that test without modifying it.
   * `tests/integration/report_read_generation_pin_test.rs` asserts `source.contains("#[cfg(feature = \"git-graph\")]\npub async fn query_changes")`. Keeping `#[cfg(feature = "git-graph")]` immediately above `pub async fn query_changes` in `PA5-T5` preserves that assertion.

## Risks and Caveats

* **Risk 1 — Regressing `integration_read_server_generation_wiring` or `PRE-3F` `NoActivator` when `"get_workspace_status"` joins `GENERATION_PINNED_READS`**: Mitigation: `PA5-T1` explicitly keeps `get_workspace_status` accepted (with `read_context = None`, no `provenance`, and `CodeGraphStats::default()` without `connect_db`) when no gate is installed or before initial activation completes, while routing through `Some(&context)` whenever `admit_read` admits the request.
* **Risk 2 — Breaking `contract_read_path_pinning_enforcement`, `integration_search_service_pin`, `integration_core_read_generation_pin`, or `integration_report_read_generation_pin` source-text assertions**: Mitigation: `PA5-T2`..`PA5-T5` preserve the two-argument wrappers, confine snapshot reads in `src/tools/read.rs` to the established pinning seams, place `query_memory_with_context` and `unified_search_with_context` immediately before their 2-arg wrappers, and keep `#[cfg(feature = "git-graph")]\npub async fn query_changes` intact.
* **Risk 3 — `Slot-20b` (`142.088-T`) encountering un-refused non-Read IPC calls (deferred stash `65EF8399` / `1918AFD2`)**: `B0` recorded that `flush_state`, `index_workspace`, `sync_workspace`, `run_retrieval_eval`, `_shutdown`, and `not_a_declared_method` were not refused with `16_001` at `f6f3171f`, and Ship captured stash `65EF8399` during `151-S` (`142.066-T`). Because `65EF8399` is outside the P-017 `DARK_MODE_ACTIVE` stash scope (`86F93068`, `4628001C`) and `142.088-T` edits only `tests/contract/read_server_cli_mcp_parity_test.rs` (`PA-7`), if `Slot-20b` cannot reach `PASS` without production refusal gating in `src/daemon/request_entry.rs`, Ship must halt with `R15-SLOT-NOT-SELF-CONTAINED 142.088-T` (Plan §12) so the operator can authorize triaging `65EF8399` rather than `Slot-20b` editing production files out of scope.
* **Risk 4 — Backlogit SQLite cache union landmine**: `.backlogit/backlogit.db` holds items from `fix/copilot-preflight-budget` (`143-F`, `143.001-T`, stash `25A1BE9E`). Stage must NOT run `backlogit sync` during `PA5-P` (Plan §11) and must verify `git status --short -- .backlogit/` and `git diff -- .backlogit/stash.jsonl` after every mutation.

## Harvest Content Verification Checklist (`U1`–`U9` and §9.2, Ruling `R-A7`)

Under Ruling `R-A7` (`docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md`), Parent Plan §8 Step 6's `U1`–`U9` and §9.2 content writes and checks (`R16-CONTENT-CHECK <task> <req>` and `R16-STALE-TEXT <task> <string>`) for `142.080-T`..`142.089-T` and `142.058-T` execute during `PA5-P` harvest before the staging PR is committed:

1. **`U1` (no stale group/hold tokens)**: scan `description`, `acceptance-criteria`, `implementation-notes`, and `verification` (history and comments exempt; `U7` `STATUS-RESET` line exempt) of all 10 harvested/rewritten tasks (`142.080-T`..`142.089-T`, `142.058-T`) plus `142.059-T` for banned tokens (`\bS[1-5]\b`, `142-S`, `PA-1`, `pending-pa-1`, `EXPECTED_PENDING_RED` mapped to another task, `after-edit map`, `F54 settle`, `R12.4`, `frozen`). Any hit raises `R16-STALE-TEXT <task> <string>`.
2. **`U2` (Shipment, Depends on, Size/Estimate)**: every task carries `"Shipment: Slot-<NN> (one-task shipment, 142-F decomposition plan Revision 18)"`, a `"Depends on:"` line whose task IDs equal `backlogit dep list <task>`, and the §4.2 size and estimate.
3. **`U3` (`PASS` only)**: every task states that Step 4.3 and the final readiness run are `PASS` only and never rely on another task's pending RED.
4. **`U4` (Route form)**: `142.080-T`..`142.089-T` and `142.058-T` state Route `(c)` verbatim ("the harness-architect writes the failing test at Step 2 against this task's own placeholder; build-feature never authors it (R12.5 A16 template, R11.10)").
5. **`U5` (Prose `Size | Complexity`)**: every task embeds enum-validated prose `Size: <S> | Complexity: <low|medium> | size_source: agent | size_ruleset_version: engram-stage-2h-rule-v1`.
6. **`U6` (No cross-task file ownership or signature freeze, `R12.3`)**: every task states `"Files edited"` / `"Production files edited"` (rewriting `"Owned files:"` on `142.058-T` to `"Files edited:"`), claims no ownership of another task's file, and forbids `#[path]` includes of another task's production file (`R12.3`; `Slot-20a`..`20d` include only the test helper `tests/helpers/activation_settle.rs` per `V1`).
7. **`U7` (`STATUS-RESET` and `PA-6-LIFTED`)**: `142.058-T` retains its `STATUS-RESET` and `PA-6-LIFTED 2026-10-04 (OD-6)` lines in `history` and removes the temporary `R-A7` deferral note from `implementation-notes`.
8. **`U8` (Merge commit & SHA)**: every task states that its slot PR merges to `main` with a merge commit (`P-009`) and records its merge SHA.
9. **`U9` (`SB-1` quoted)**: every task quotes `SB-1` (Plan §4.4) verbatim in `implementation-notes` with the slot's `1.5 ×` time limit (`2.25 h` for `1.5 h` slots, `1.5 h` for `1.0 h` slots) and `R-A6` authored/adapted 400-line bound for `Slot-20a`..`20d`.
10. **§9.2 `PA-5` and `S-20` items**:
    * Each `PA-5` task (`142.080-T`..`142.086-T`) maps every F54 refusal and provenance case it enables to its `058` slot (`142.088-T` / `142.058-T`), and `142.080-T` states that pinned `get_workspace_status` changes only `code_graph` (`R-1`).
    * `142.087-T`..`142.089-T` and `142.058-T` carry the complete `S-20` AC/verification map table, their `V1`–`V8` share, the three FL points and `FL-RED-HALT` (`R11.6`), `R10.4` addition 5 (`T0` tag `parked/142-s-split-0f2cca20` and `B0` path `docs/memory/2026-10-04-ship-142-f-a0-b0.md`, `R9.9`), the `(d2)` `ENGRAM_*` environment-strip check, the pre-claim drift ledger (`git log --oneline f6f3171f..<slot_base> -- src/tools/capabilities.rs`), the `R-A6` line-range and adaptation rules proven by `cargo check --tests` before RED, `.001-ST` history preservation, `V5` real-fixture specification on `142.089-T`, and `M6`/`V6` consumer-target and three-run evidence on `142.058-T`.

## Constitution Check

| Principle / Rule | Compliance in `PA5-P` |
|---|---|
| **I. Safety-First Rust** | `#![forbid(unsafe_code)]` preserved; errors propagated via `Result<_, EngramError>` and `?`; `.unwrap_or(0)` retained only on `get_workspace_status` count queries as required by `OD-5` Q3 / `4628001C`; `src/db/` untouched; pedantic clippy clean across default and `--features git-graph` builds. |
| **II. Test-First Development (NON-NEGOTIABLE)** | Every code/contract task (`142.080-T`..`142.089-T`, `142.058-T`) uses Route `(c)`: `harness-architect` lands the failing test against the task's own `Worker: <task_id>` placeholder at Step 2 and confirms RED before `build-feature` replaces the placeholder at Step 4.2. **Documented Route `(c)` placeholder form for `PA5-T6` (`142.086-T`)**: because `ToolDescriptor` is a data struct constructed in `all_descriptors()`, at Step 2 `harness-architect` adds `pub generation_backed: bool` initialized to `false` and places the `Worker: 142.086-T` marker in the `contract_tool_descriptor_registry` assertions (matching the `142.066-T` `R14.3` constant-placeholder precedent approved in Parent Plan §13). Rejected alternative: adding a separate panicking getter method on `ToolDescriptor`, rejected as unnecessary API surface churn. |
| **III & IV. Workspace Isolation & CLI Containment (NON-NEGOTIABLE)** | All paths stay within the workspace; test fixtures use `TempDir`, strip `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, and `ENGRAM_DIRECT`, and enforce `D5-D` (no fallback to managed storage on a generation context). |
| **V. Structured Observability** | Admitted generation reads report `provenance.generation_id`; `DaemonScopedRead` tools report `provenance: None`; `F54 settle:` lines and per-slot closure records are preserved. |
| **VI. Single Responsibility** | One task per shipment (`[<task_id>]`); no new dependencies; each unit touches a single skill domain (`code` + its test in `PA5-T1`..`PA5-T6`, `tests` only in `Slot-20a`..`20d`, `docs` only in `Slot-21`). |
| **VII & VIII. Destructive Command Approval & Safety Modes (`strict-safety`)** | `Requires plan hardening: yes`; hardened in `careful` mode; risky actions `PA5P-1`..`PA5P-7` classified with `ProposedAction`, `ActionRisk`, `ActionResult`, and rollback paths. |
| **IX. Git-Friendly Persistence** | Markdown source of truth in `.backlogit/`; no `backlogit sync` over the divergent SQLite cache; `142-F.md` SHA-256 verified after every write (§10.3). |
| **X. Agent Context Efficiency** | Compound learnings retrieved first; `SB-1` bounds every slot session. |
| **XI. Merge Commit History Preservation (NON-NEGOTIABLE)** | Every staging and slot PR merges with a merge commit (`P-009`, `U8`). |
| **Task Granularity (2-Hour Rule, `SG-1`, `R-A6`, `R-A7`)** | All 11 tasks pass `SG-1` (`< 3` substantive files, `< 5` functions, `< 4` test scenarios, `0` open subtasks, `Size: S`, $\le 1.5\text{ h}$). **Operator-approved Ruling `R-A6` for `Slot-20a`..`20d`**: the 400-line bound in Plan §4.2, §4.3 `S-20`, and `SB-1 (c)` counts only lines the `harness-architect` authors or adapts (`~52`, `~75`, `~75`, `~35` lines $\le 400$), while the verbatim load from `6d216d19` (`1,633` lines total) is measured and diff-verified. Rejected alternatives (`docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md`): Option B (re-cutting `S-20` by layer, which still required a ~1,000-line shared-core exemption and added plan churn) and splitting the shared core without callers (which fails `-D warnings` `dead_code` under the `R11.10` L1 `allow` ban). **Subtask close consolidation (`PA5P-1`)**: closing the 6 finished-family subtasks (`142.064.001-ST`..`003-ST` and `142.054.001-ST`..`003-ST` per Plan §10.1) in the `PA5-P` staging branch was explicitly directed by the operator's P-017 `DARK_MODE_ACTIVE` activation record while preserving every §10.1 check and §10.3 rollup guard. |

## Plan Hardening Signals

* **Public API, schema, or contract change**: **Present** — `ToolDescriptor` in `src/tools/capabilities.rs` gains `pub generation_backed: bool` (`PA5-T6`), `GENERATION_PINNED_READS` in `src/daemon/request_entry.rs` expands from 1 to 9 entries (8 default-build methods + `"query_changes"` under `git-graph`, `PA5-T1`..`PA5-T5`), and `src/tools/lifecycle.rs` / `src/tools/read.rs` gain `*_with_context` public functions.
* **Security, auth, permission, or compliance-sensitive behavior**: **Present** — read-server admission and generation isolation in `src/daemon/request_entry.rs` and `src/tools/` enforce the read-only generation boundary (`D5-D`: never fall back to managed data on a generation context).
* **Migration, backfill, destructive data/config action, or irreversible step**: **Present** — splitting `142.058-T` (`S-20`), wiring `blocks` edges across queued tasks and shipments, and archiving consumed stash entries `86F93068` and `4628001C`.
* **External integration, operator checkpoint, or external dependency**: **Absent** — all tests run against local isolated daemon/CLI/MCP subprocesses and in-process fixtures.
* **High runtime, rollout, or rollback risk**: **Present** — `PA5-T1`..`PA5-T5` modify shared tool dispatch (`src/tools/mod.rs`) and IPC request admission (`src/daemon/request_entry.rs`).

`Requires plan hardening: yes`

## Runtime Verification and Closure

| Unit / Slot | Runtime surface changed? | Runtime verification required | Operational closure expectations |
|---|---|---|---|
| `PA5-T1` (`Slot-19.1`, `142.080-T`) | Yes (`get_workspace_status` IPC/CLI/MCP dispatch) | `cargo test --test integration_generation_pinned_workspace_status -- --nocapture`, `integration_read_server_generation_wiring`, `integration_lifecycle_read_generation_pin`, plus live `engram workspace-status` probe when daemon Ready (or tracked under stash `6B3DD712` if blocked) | SLI: 100% of admitted read-server `get_workspace_status` calls report `provenance.generation_id` and generation `code_graph` counts with zero `connect_db` side effects; rollback: revert slot merge commit |
| `PA5-T2`..`PA5-T5` (`Slot-19.2`..`19.6`, `142.081-T`..`085-T`) | Yes (`query_memory`, `list_symbols`, `map_code`, `impact_analysis`, `query_graph`, `unified_search`, `query_changes` dispatch) | Each slot's `integration_generation_pinned_*` target (`--features git-graph` as well for `PA5-T5`), `contract_read_path_pinning_enforcement`, `integration_search_service_pin`, `integration_core_read_generation_pin`, `integration_report_read_generation_pin`, `integration_read_server_generation_wiring` | SLI: 100% of admitted generation-backed reads serve `OpenedGeneration::db()` with matching `/provenance/generation_id` and zero `.engram/cozo` sidecars; rollback: revert slot merge commit |
| `PA5-T6` (`Slot-19.7`, `142.086-T`) | Yes (`ToolDescriptor` metadata in `src/tools/capabilities.rs`) | `cargo test --test contract_tool_descriptor_registry -- --nocapture` (default and `--features git-graph`) | SLI: `generation_backed` classification matches `GENERATION_PINNED_READS` with 0 drift; rollback: revert slot merge commit |
| `S-20` (`Slot-20a`..`20d`, `142.087-T`..`089-T`, `142.058-T`) | No (test-only contract harness `tests/contract/read_server_cli_mcp_parity_test.rs`) | `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture` (3 consecutive identical runs at `Slot-20a` FL and `Slot-20d` M12, every `F54 settle:` line `Active`), `cargo dev-test --no-fail-fast` `PASS` | Closure record per slot; after `Slot-20d` closes, Stage closes `142.058.002-ST` and `142.058.003-ST` per Plan §10.1 |
| `Slot-21` (`142.059-T`) | No (docs-only `docs/troubleshooting.md` and compound supersession note) | `backlogit docs lint --path docs`, cross-check against `src/errors/codes.rs` and F54 matrix (`harness-verification-gated`) | Final slot of `142-F`; followed by `CP-FINAL` (`142-F` close, Plan §10.2) |

## Plan Hardening

### 1. Hardening Confirmation and Mode

Hardening is **required** (`Requires plan hardening: yes`) and executed in **`careful` safety mode** per Plan §7.6 ("then plan-harden in careful mode") and `.github/instructions/strict-safety.instructions.md`. The plan touches shared request-entry admission (`src/daemon/request_entry.rs`), tool dispatch (`src/tools/mod.rs`), descriptor contracts (`src/tools/capabilities.rs`), and multi-slot backlog/shipment state.

### 2. Protected Invariants and Instructions Consulted

* **Instructions and policies consulted**: `.github/policies/workflow-policies.md` (P-001, P-002, P-003, P-005, P-006, P-009, P-010, P-012, P-014, P-015, P-016, P-017, P-018, P-020, P-021), `.github/instructions/strict-safety.instructions.md`, `.github/instructions/backlogit.instructions.md`, `.github/instructions/technology-rust.instructions.md`.
* **Learnings reinforced**:
  * `docs/compound/capability-rewrite-must-convert-every-consumer-2026-08-21.md`
  * `docs/compound/concurrency-issues/cozo-sqlite-busy-locked-reopen-panic-catch-unwind-2026-07-15.md`
  * `docs/compound/workspace-status-code-graph-cfg-gate-false-premise-2026-07-07.md`
  * `docs/compound/test-failures/truncated-test-output-review-hid-call-site-regression-2026-09-12.md`
  * `docs/compound/test-probe-resets-idle-ttl-livelock-2026-09-04.md`
  * `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`
  * `docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md`
  * `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`
* **Protected invariants**:
  1. **Managed-mode byte-identity**: every 2-argument handler (`get_workspace_status`, `query_memory`, `list_symbols`, `map_code`, `impact_analysis`, `query_graph`, `unified_search`, `query_changes`) remains a byte-identical delegation to `*_with_context(..., None)`.
  2. **Zero `connect_db` on generation contexts (`D5-D`)**: when `read_context` is `Some(ctx)` with `ctx.generation().is_some()`, `connect_db`, `pinned_read_request_context`, `pinned_dispatch_context`, `pinned_queries`, and `ReadRequestContext::from_managed_state` are unreachable; there is never a fallback to managed data, and no downstream helper called after `queries_from_read_context` re-opens `connect_db` or reads mutable managed state.
  3. **`src/db/` untouched & F37 pinning-seam counts preserved**: all handler conversions construct `CodeGraphQueries` through `queries_from_read_context` (or `CozoDb { inner: Arc::new(generation.opened_generation().db().clone()) }` in `lifecycle.rs`); `src/db/` is never modified, and `src/tools/read.rs` adds zero direct `connect_db(`, `snapshot_dispatch_context()`, `snapshot_workspace()`, or `ReadRequestContext::from_managed_state(` calls outside the three existing seam functions (`contract_read_path_pinning_enforcement`).
  4. **No-gate / pre-activation `get_workspace_status` continuity (`R6.8`)**: `get_workspace_status` remains accepted with no provenance (and without calling `connect_db`) on a `ReadServer` daemon when no gate is installed or before initial activation completes.
  5. **F54 file ownership (`PA-7`, `R-2`)**: only `142.087-T` (`Slot-20a`), `142.088-T` (`Slot-20b`), `142.089-T` (`Slot-20c`), and `142.058-T` (`Slot-20d`) edit `tests/contract/read_server_cli_mcp_parity_test.rs`.
  6. **Covering feature `142-F` invariance (§10.3)**: `.backlogit/queue/142-F.md` remains `status: active` with SHA-256 `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` across every Stage and Ship mutation.

### 3. `strict-safety` Classification of Risky Actions (`ProposedAction` / `ActionRisk` / `ActionResult`)

| ID | Owner | ProposedAction & Target | `change_kind` | `ActionRisk` | `approval_required` | Approval / `ActionResult` | Rollback / Containment |
|---|---|---|---|---|---|---|---|
| `PA5P-1` | Stage | Close finished-family subtasks `142.064.001-ST`..`003-ST` and `142.054.001-ST`..`003-ST` via `backlogit move <id> --status done` (Plan §10.1) | backlog state transition (`queued` → `done` in `archive/`) | `moderate` | pre-authorized in `DARK_MODE_ACTIVE` scope + Plan §10.1 | `applied` (verified `142-F.md` SHA-256 unchanged after each move) | `git checkout -- .backlogit/` before merge; revert PR after merge |
| `PA5P-2` | Stage | Create 7 `PA-5` tasks (`142.080-T`..`142.086-T`) and 3 `S-20` split tasks (`142.087-T`..`142.089-T`) under `142-F`; rewrite `142.058-T` to its `Slot-20d` share (Plan §7.6, `R-A6`, `R-A7`) | backlog task creation and body update | `moderate` | pre-authorized in `DARK_MODE_ACTIVE` scope + `OD-5`, `R-A6`, `R-A7` | `approved` (executed at Step C harvest) | `git restore` / revert staging branch commit before merge |
| `PA5P-3` | Stage | Wire task `blocks` edges `E7`, `E9`, `E3`, `E6`, `E15` and verify zero DAG cycles (Plan §5.1, `R-A7`) | backlog dependency graph mutation | `moderate` | pre-authorized in `DARK_MODE_ACTIVE` scope + Plan §5.1, `R-A7` | `approved` (executed at Step C) | `backlogit dep remove` / `git restore` on staging branch |
| `PA5P-4` | Stage | Create 12 one-task shipments (`167-S`..`178-S` for `Slot-19.1`..`19.7`, `Slot-20a`..`20d`, `Slot-21`), wire mirrored shipment `blocks` edges, and set `custom_fields.queue_position` (`191..=197`, `200..=203`, `210`) | backlog shipment creation and ordering | `moderate` | pre-authorized in `DARK_MODE_ACTIVE` scope + Plan §5.2, §7.6 | `approved` (executed at Step C) | `git restore` on staging branch before merge |
| `PA5P-5` | Stage | Archive consumed stash entries `86F93068` and `4628001C` via `backlogit stash archive` after harvest and shipment assembly (Stage Step 5.6, P-021 C6) | stash archival (non-destructive retirement from active stash) | `moderate` | pre-authorized in `DARK_MODE_ACTIVE` scope + Deliberation disposition | `approved` (executed at Step 5.6; inspect `git diff -- .backlogit/stash.jsonl` to ensure `25A1BE9E` is not unioned) | `git checkout -- .backlogit/stash.jsonl` if cache union occurs |
| `PA5P-6` | Ship (downstream) | Per-slot manual safe-close at Step 6 of `167-S`..`178-S` (`backlogit archive <S>` + `reconcile-shipped`, never `backlogit shipment ship`, P-015) | shipment archival | `destructive` (queue file removed, preserved in `archive/`) | per-slot `DARK_MODE_ACTIVE` activation contract | `planned` (approved per-slot under `DARK_MODE_ACTIVE`) | `git revert` of closure commit; `142-F.md` SHA-256 guard |
| `PA5P-7` | Stage (downstream, after `Slot-20d`) | Close `142.058.002-ST` and `142.058.003-ST` via `backlogit move <id> --status done` after `Slot-20d` (`177-S`, `142.058-T`) finishes (Plan §10.1) | backlog state transition (`queued` → `done` in `archive/`) | `moderate` | Plan §10.1 | `planned` (runs after `Slot-20d` closes) | `git revert` of subtask-close commit; `142-F.md` SHA-256 guard |

### 4. Deepened Verification, Rollback, and Operational Closure Detail

* **Pre-execution environment checks (Ship Step 1 & CG-D / CG-S)**:
  * Before claiming any slot in `167-S`..`178-S`, Ship verifies: (1) every task predecessor is archived `done` and every shipment predecessor is archived `done`/`shipped` (`CG-D`, Plan §5.3); (2) the shipment manifest holds `[<task_id>]` only and the task is `queued` (`CG-Q`, `CG-S`); (3) tag `T0 = parked/142-s-split-0f2cca20` exists locally and on `origin` (for `Slot-20a`..`20d`, which also record the `R9.9` drift ledger `git log --oneline f6f3171f..<slot_base> -- src/tools/capabilities.rs` before claim).
* **Mandatory regression suite for `PA5-T1`..`PA5-T6` (`Slot-19.1`..`Slot-19.7`)**:
  * In addition to each slot's own new target and `cargo dev-test --no-fail-fast`, Ship must run and record PASS for:
    * `cargo test --test integration_read_server_generation_wiring -- --nocapture` (3 consecutive careful-mode runs with identical per-case maps for `PA5-T1`, confirming `get_workspace_status` quiescence and Scenario 3 no-root acceptance).
    * `cargo test --test contract_read_path_pinning_enforcement -- --nocapture` (`PA5-T1`..`PA5-T5`).
    * `cargo test --test integration_search_service_pin -- --nocapture` (`PA5-T2`).
    * `cargo test --test integration_core_read_generation_pin -- --nocapture` (`PA5-T2`..`PA5-T4b`).
    * `cargo test --test integration_report_read_generation_pin -- --nocapture` and `cargo test --test integration_report_read_generation_pin --features git-graph -- --nocapture` (`PA5-T5`).
    * `cargo test --test integration_lifecycle_read_generation_pin -- --nocapture` (`PA5-T1`).
    * `cargo test --test contract_tool_descriptor_registry -- --nocapture` and `cargo test --test contract_tool_descriptor_registry --features git-graph -- --nocapture` (`PA5-T6`).
    * `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` (plus `--features git-graph` for `PA5-T5` and `PA5-T6`).
* **Blocked-path handling**:
  * Live runtime CLI probes against the workspace's detached daemon: if the detached daemon is not in `ready` state during post-merge closure, Ship records `READY_WITH_CONDITIONS` citing existing stash `6B3DD712` without killing or respawning an unowned daemon PID.
  * If any slot exceeds `SB-1` (`1.5 ×` estimate, 3rd failing build-fix loop at Step 4, or > 400 authored/adapted harness lines at Step 2), Ship writes a checkpoint and halts with `R17-SLOT-BUDGET <slot> <a|b|c>`.

### 5. Review-Gate Capability Risks and P-012 Marker Contract

* Reviewer subagent dispatch (`task` tool with persona subagents `Constitution Reviewer`, `Rust Reviewer`, `Scope Boundary Auditor`, `Learnings Researcher`, `Architecture Strategist`, `Agent-Native Parity Reviewer`, `Security Lens Reviewer`) is available in this session (`TOOL_OK: reviewer-subagent-dispatch`).
* The appended `## Plan Review` section MUST include the literal machine-readable markers:
  * `dispatch_mode: multi-agent` (or declared degradation if any persona requires fallback)
  * `decision: PASS` (with 0 P0 and 0 P1 findings before harvest may proceed).
* Unresolved operator decisions blocking safe execution: **none** (`OD-1` to `OD-7`, `R-A1` to `R-A8` are all decided).

## Plan Review

### Attempt 1 — `FAIL` (Remediated in Revision 2 of this plan)

* `dispatch_mode: multi-agent`
* `decision: FAIL` (Attempt 1; 0 P0, 2 P1, 3 P2, 8 P3 across 7 personas; all 2 P1s, all 3 P2s, and all 8 P3s remediated in place above before Attempt 2 re-review)

| Persona | Dispatch Mode | Attempt 1 Verdict | P0 | P1 | P2 | P3 | Disposition in Revision 2 |
|---|---|---|---|---|---|---|---|
| Constitution Reviewer | subagent (`multi-agent`) | `ADVISORY` | 0 | 0 | 2 | 3 | **Closed**: added standalone `## Constitution Check` (`P2-1`), `## Harvest Content Verification Checklist (U1–U9 and §9.2)` (`P2-2`), explicit Route `(c)` Step 2 stub bodies (`P3-1`), canonical `ActionResult: planned` + `PA5P-7` (`P3-2`), and clarified `SG-1` function counts (`P3-3`). |
| Rust Reviewer | subagent (`multi-agent`) | `PASS` | 0 | 0 | 1 | 1 | **Closed**: placed `pub async fn query_memory_with_context` immediately before `pub async fn query_memory` (`P2` / `P1-2`) and filtered `GENERATION_PINNED_READS` by `cfg!(feature = "git-graph") \|\| *name != "query_changes"` in `PA5-T6` Scenario 3 (`P3`). |
| Scope Boundary Auditor | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 1 | **Closed**: kept the `get_workspace_status` observability fallback inline inside `process_request`'s `GENERATION_PINNED_READS` arm (`P3`). |
| Learnings Researcher | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 1 | **Closed**: added invariant in `Plan Hardening §2` that no downstream helper called after `queries_from_read_context` re-opens `connect_db` or reads mutable managed state (`P3`). |
| Architecture Strategist | subagent (`multi-agent`) | `FAIL` | 0 | 2 | 1 | 1 | **Closed**: replaced `state.snapshot_dispatch_context()` in `PA5-T3` with `maybe_pinned_dispatch_context(&state, "<method>").await.map(\|s\| s.config.code_graph).unwrap_or_default()` to preserve `contract_read_path_pinning_enforcement` (`P1-1`); placed `query_memory_with_context` before `query_memory` for `integration_search_service_pin` (`P1-2`); specified `cfg!(feature = "git-graph")` filter on `GENERATION_PINNED_READS` in `PA5-T6` (`P2-3`); constructed `CodeGraphQueries` directly in `lifecycle.rs` (`P3`). |
| Agent-Native Parity Reviewer | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 2 | **Closed**: specified `cfg!(feature = "git-graph")` filter in `PA5-T6` Scenario 3 and documented `ToolDescriptor.generation_backed` vs `request_entry::is_generation_backed_read` (`P3`). |
| Security Lens Reviewer | subagent (`multi-agent`) | `ADVISORY` | 0 | 0 | 0 | 3 | **Closed**: matched directly on `read_context.and_then(\|ctx\| ctx.generation())` in `lifecycle.rs` (`P3-1`), guarded `process_request` fallthrough strictly on `method == "get_workspace_status"` after `admit()` validation (`P3-2`), and included `(d2)` `.env_remove(...)` verification in `Slot-20a` (`P3-3`). |

<!-- plan-review-attempt: 1 -->

### Attempt 2 — `PASS` (All 7 Personas `PASS` with 0 P0, 0 P1, 0 P2, 0 P3)

* `dispatch_mode: multi-agent`
* `decision: PASS`
* **Gate rationale**: All 7 selected personas (`Constitution Reviewer`, `Rust Reviewer`, `Scope Boundary Auditor`, `Learnings Researcher`, `Architecture Strategist`, `Agent-Native Parity Reviewer`, `Security Lens Reviewer`) ran as independent subagents on Revision 2 of this plan and returned `PASS` with **0 P0, 0 P1, 0 P2, and 0 P3 findings**. All Attempt 1 findings (`P1-1`, `P1-2`, `P2-1`..`P2-3`, and `P3` items) were verified resolved. Plan hardening (`Requires plan hardening: yes`) and `strict-safety` `ProposedAction` / `ActionRisk` / `ActionResult` classifications are complete. The plan is approved for `PA5-P` harvest and shipment assembly.

| Persona | Dispatch Mode | Attempt 2 Verdict | P0 | P1 | P2 | P3 | Notes |
|---|---|---|---|---|---|---|---|
| Constitution Reviewer | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 0 | All Attempt 1 findings (`P2-1`, `P2-2`, `P3-1`..`P3-3`) verified resolved; Principles I–XI and P-001..P-021 compliant. |
| Rust Reviewer | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 0 | `query_memory_with_context` placement, `maybe_pinned_dispatch_context` seam usage, and `git-graph` filtering verified resolved. |
| Scope Boundary Auditor | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 0 | Strictly within `86F93068`, `4628001C`, `S-20`, and `Slot-21` scope; `SG-1`, `R-A6`, and `S-20` AC map verified complete. |
| Learnings Researcher | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 0 | 103 compound files searched (`confidence: high`); all prior solutions incorporated with 0 contradictions. |
| Architecture Strategist | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 0 | `P1-1` (`contract_read_path_pinning_enforcement`) and `P1-2` (`integration_search_service_pin`) verified resolved; DAG acyclic. |
| Agent-Native Parity Reviewer | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 0 | Cross-surface IPC/CLI/MCP parity, `ToolDescriptor.generation_backed`, and `DaemonScopedRead` verified. |
| Security Lens Reviewer | subagent (`multi-agent`) | `PASS` | 0 | 0 | 0 | 0 | Read-server admission, `D5-D` generation isolation, `4628001C` no-sidecar guarantee, and `(d2)` env-strip verified. |

<!-- plan-review-attempt: 2 -->
