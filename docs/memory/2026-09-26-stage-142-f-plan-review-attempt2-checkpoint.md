# Stage checkpoint: 142-F plan-review attempt 2 failed; re-entry 2 is in progress

- **Status:** Mid-session context rollover. Resume from "Next action" below.
- **Supersedes:** `2026-09-26-stage-142-f-preflight-command-checkpoint.md` (read that one first for the earlier context).
- **Role:** Stage, working for shipment 142-S on branch `feat/142-s-...` at HEAD `4995d681`.
- **Hard limits:**
  - Do not edit source, test, or config files.
  - Do not build, claim, open a PR, or commit.
  - Do not change the 142-S manifest or any active task (including 142.054/055/057/058) without fresh operator approval.
- **Session gates:**
  - Tools passed.
  - `backlogit sync` returned OK.
  - The checkpoint scan found 34 clean entries and no active Stage checkpoint (the parent verified this).

## Goal and acceptance criteria

1. Finish reviewing the plan `docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md`, now at revision 3.
2. Work out how F50's concrete-verifier API gaps are closed.
3. If the plan passes review, harvest the ready tasks under 142-F using the `backlogit` CLI. Report their IDs. Do NOT add them to 142-S.
4. If the plan fails again, follow the circuit breaker and stop with the exact blockers listed.
5. Also:
   - complete the stash duplicate-check and late-ID records
   - write the memory file
   - list the exact approvals needed (named IDs and dependency edges), with no deadlock
6. The final report must give the review status and the task IDs that are ready to admit.

## Artifacts created this session (uncommitted)

- **Stash `49809128`** (high priority, task):
  - DEFERRED SCOPE EXPANSION: the root-library facades F50 needs.
  - Source refs: 142.054-T, 002-ST, 003-ST, 142-F, 142-S.
  - PR and review thread are N/A.
- **Deliberation `docs/decisions/2026-09-26-142-f-launcher-preflight-command-deliberation.md`:** frontmatter updated, and Amendment 1 appended.
  - A1.1: the workspace argument is now positional.
  - A1.2: duplicate scans for `03AA00A8` and `49809128` are both clean. For both, "no late identifier found", so N/A stands.
  - A1.3: new evidence.
  - D4-A: root-library facades.
- **Plan, now revision 3:**
  - Adds a Problem Frame covering gaps G1–G4.
  - Adds trace rows, PRE-1..PRE-6 units, and guidance for completing F50.
  - Updates the dependency graph and execution order, decisions, Constitution rows, risks, runtime table, and PA-1/PA-2b/PA-4.
  - Adds invariants 7–9.
  - Its last marker is still `<!-- plan-review-attempt: 1 -->`.
  - **The attempt-2 FAIL has NOT yet been appended.**

## Established facts (verified in source)

- `engram-indexer` depends only on `engram` and `tokio`.
- `file_tracker::compute_file_hash` is public.
- `read_current_manifest_revision` is private.
- F10 `index_sealed_target` uses the Candidate as the discovery root. `connect_db` writes `data_dir/cozo/<branch_safe>/engram.db`.
- Activation needs `<gen>/engram.db`. `GenerationActivator::new(store, runtime_root abs, ExpectedIdentity, Duration)`.
- Lost plan units: P11, P12, and P34.
- **G3 (production wiring gap):**
  - No production code constructs `GenerationActivator` or `ReadServerStartupGate`.
  - `run_read_server_startup` (`lifecycle_policy.rs:159`) sets hydration-ready immediately.
  - `process_request` calls `tools::dispatch`, which passes `None` context (`tools/mod.rs:360`).
  - Read handlers use `ReadRequestContext::from_managed_state` and `connect_db(context.data_dir)` (`tools/read.rs:95-133`). They never read the opened generation.
  - `OpenedGeneration::db() -> &cozo::DbInstance`, stored as an `Arc`. `Db = CozoDb`, so a conversion may be needed; if so, HALT.
  - `get_workspace_statistics` uses `pinned_read_request_context`.
- **Real-binary read-server consumers** that PRE-3 could break:
  - `tests/integration/read_server_restart_test.rs` (`daemon-status`, no generation)
  - `read_server_lifecycle_test.rs` (in-process `run_with_shutdown_v2`)
  - `doctor_smoke_test.rs`
  - `direct_sync_mode_test.rs`
  - F54
- **Helpers:**
  - `canonicalize_workspace(&str)` is public. It requires `.git` and strips `\\?\`.
  - `resolve_git_branch` is public; the daemon falls back to "default".
  - `workspace_hash` is public.
  - CLI auto-spawn goes through `shim::lifecycle::ensure_daemon_running`.
- This repo's `.engram/config.toml` is in managed mode (the PA-4 issue).

## Review attempt 2 (5 personas): FAIL

P1 findings, after dedup:

1. **Rust P1-1.** On Windows, an inherited grandchild keeps the pipe open, so EOF never arrives and NEW-4/PRE-5/PRE-6 hang. Fix: key on child exit with a `try_wait` loop, use a reader thread capped at cap+1, drain for 250 ms, never join or read to EOF, and add a grandchild fixture.
2. **Rust P1-2.** The public signature of `probe_mcp_read` exposes `serde_json::Value`. Fix: add `ProbeRead { cli_args, mcp_tool, mcp_arguments_json }` and a `pub const PREFLIGHT_READ`. Architecture also says to move the module to top-level `src/preflight_probe.rs` plus a line in `src/lib.rs`.
3. **Rust P1-3 (Architecture and Learnings agree).** Identity derivation is wrong. Fix:
   - `read_server_layout` uses `canonicalize_workspace` with a git fixture; a non-git workspace gives a typed error.
   - Add `runtime_root` and an activation-deadline const.
   - PRE-3 must REPLACE the startup derivation with the layout.
4. **Scope P1.** G3 has no stash or deliberation lineage. Fix: a new stash entry (C2 fields) and deliberation Amendment 2 / D5. PA-1 must name D5.
5. **Scope and Learnings P1.** PRE-3 breaks done tests that run read-server with no generation. Planned fix: keep `_health` hydration semantics UNCHANGED, and gate only generation-backed reads through `admit_read`. Run background initial activation after bind, with retry/backoff while unpublished. List the consumers above as regression targets, and HALT if any goes RED. Record a follow-up stash to align `_health` with the gate.
6. **Architecture P1.** PRE-4 only labels provenance; the data still comes from managed state. Fix:
   - Add PRE-4b, owning `src/tools/mod.rs` and `src/tools/read.rs`. The pinned read (`get_workspace_statistics`) receives the admitted context, and `queries_from_read_context` uses the opened generation's DB.
   - Assert the data field differs from managed data; probes compare the data field.
   - Record a stash for the remaining handlers (F54 lineage, which blocks F54 GREEN).
   - Add an edge from 142.058-T to PRE-4b.
7. **Learnings.** PA-3 is not yet bundled with PA-1. Fix: mark it "bundled with PA-1".

Key P2s to absorb cheaply:

- Mint the ID in `production_factory` (NEW-2 make); Build must never mint.
- Add the scoped `dead_code` allow to NEW-4.
- PA-2b adds the F50 clippy acceptance explicitly.
- Rename-safety: WAL/SHM sidecar check, retry on error 32, and a symbol query in PRE-2.
- Symlinked `.engram` containment check. `open_generation_store(layout, create)` goes in PRE-2 (it creates the root).
- NEW-5 clap `id = "preflight_workspace"`.
- NEW-4 `checked_add` for `T + grace`.
- Probes `env_remove` `CARGO_BIN_EXE_engram`, `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, `ENGRAM_DIRECT`.
- Retention follow-up stash.
- `normalize_verdict(&[u8], Option<i32>)`.
- Drop the redundant PRE-2→054 edge.
- Constitution rows for V, VIII, and IX.
- Bounded probe backoff.

## Decision on the circuit breaker

- The user said "max two re-entries". Attempt 2 was re-entry 1, so re-entry 2 is allowed: revise to revision 4, then run attempt 3 with the same 5 personas.
- If attempt 3 FAILs:
  - Append the FAIL with `<!-- plan-review-attempt: 3 -->`.
  - Apply the escalation protocol. The route is `claude-opus-5.5`/anthropic/high, the same as the Stage route, so it degrades to `ESCALATION_DEGRADED` and an operator halt.
  - Stop with the exact blockers.

## Remaining work (in order)

1. Append "### Attempt 2: FAIL" to the plan, listing the 7 P1s above and a P2 summary. Add `<!-- plan-review-attempt: 2 -->`.
2. Capture the new stash entries:
   - G3 production read-server wiring (PRE-3/PRE-4/PRE-4b). DEFERRED SCOPE EXPANSION, all C2 fields, PR and thread N/A.
   - Remaining read handlers still serving managed state in read-server mode (blocks F54).
   - Follow-ups: generation retention/GC, incremental seeding, aligning `_health` with the gate, and a gate for workspace-wide `dev-test`/clippy on `engram-indexer`.
3. Append deliberation Amendment 2 / D5, covering G3 and the data path. Record the duplicate scan and late-ID outcomes for the new entries.
4. Revise the plan to revision 4, applying fixes P1 1–7 and the P2s above. Then run attempt 3.
5. **If PASS or ADVISORY:**
   - Harvest with `backlogit add --type task --parent 142-F --status queued`. Include:
     - title, description
     - `--section` for the acceptance criteria and implementation notes, with Size/Complexity prose, `size_source` agent, and `size_ruleset_version engram-stage-2h-rule-v1`
     - owned files
   - Order: PRE-1, PRE-2, PRE-3, PRE-4, PRE-4b, PRE-5, PRE-6, NEW-1..NEW-6.
   - Add dependencies among the new tasks only (`backlogit dep add`). NEW-1 also depends on 142.054-T, which is allowed because it is an edge on the new task.
   - Log Step 5.5 as blocked on PA-1; create no shipment.
   - Archive `03AA00A8` and `49809128` with `backlogit stash archive`, forward-referencing the new IDs.
6. Write the final memory file in `docs/memory` and run `backlogit sync`.
7. Write the final answer. It must include:
   - the review status
   - the IDs
   - the PA-1 phrase: "I authorize Stage to add <IDs> to active shipment 142-S; make 142.054-T depend on <PRE-6> (and PRE-2); 142.058-T on <PRE-4b>; 142.055-T and 142.057-T on <NEW-5>; confirm D2-A, D4-A, D5; bundle PA-2/PA-2b/PA-3"
   - PA-4 (the read_server config), required before F51 is used on this repo
   - the execution order
   - that D1-A can be applied by Ship now
   - that F50 stays active and blocked until PRE-6

## Stopping criterion

Stop when either:

- attempt 3 PASSes, harvest is done, and the IDs and approvals are reported; or
- attempt 3 FAILs and the blockers are reported exactly.

## Next action

Append the "### Attempt 2: FAIL" record and the `<!-- plan-review-attempt: 2 -->` marker to the end of the plan file.
