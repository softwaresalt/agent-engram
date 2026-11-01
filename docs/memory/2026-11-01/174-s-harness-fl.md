---
title: "174-S Slot-20a harness provenance and FL evidence"
date: 2026-11-01
task: 142.087-T
shipment: 174-S
state: harness-ready
---

## Pre-FL provenance

Freeze scope: only `tests/contract/read_server_cli_mcp_parity_test.rs` for
source/test writes. No helper, production, config, plan or review edits.
Route (c), test-first. Structural coverage is separate from the intended
behavior RED roster (declared retrieval surfaces and direct-IPC health).
The latter roster will be authored only after FL verification.

R9.5(a): local ancestry commands for `7bd9e504` and `6d216d19` to
`0f2cca20` both exited 0. Local and remote tag object:
`bb82ad4ef0f9b167e0ab7f039bdfd39f99ab7311`; peeled commit:
`0f2cca208827f09d9ac3e76b3db110391c6976c7`. `git ls-remote origin`
returned both exact values (exit 0). Thus both SHAs are reachable from
the independently verified remote parked tag.

R9.5(b), as amended by R10.2: historical records remain available.
Parent `142.058-T` records commit `7bd9e504` and the historical
`not implemented: Worker: F54` evidence; its preserved history records
the reset and historical `6d216d19` evidence. Current archived
`142.058.001-ST` declares `status: done` and commit
`6d216d1956d41510222d596d9283cf4679792fa1`. Its historical comment records
the structural 1/1 PASS; the parent historical comment records 1 passed /
4 failed and the behavior signatures. The parked tag's queue subtask
still declares queued: location/status there is not substituted for the
completed subtask record. B0 was read using
`git show 0f2cca20:docs/memory/2026-10-04-ship-142-f-a0-b0.md`.
No historical record or label was rewritten.

R9.5(c): base `8366194d0118aace901878e9bf75c174649754b7` F54 file matches
`3f890662`'s placeholder (LF/CRLF representation normalized).
`git log 8366194d..HEAD -- tests/contract/read_server_cli_mcp_parity_test.rs`
returned no commits before FL.

R9.5(d): the full 1,633-line blob has no retrieval-report-specific
CLI-absence assertion or hard-coded retrieval surface list.
The general `!read_descriptor.supports(Cli)` check is a positive
requirement on `get_workspace_status`, not retrieval-report absence.

R9.5(d2): every child constructor in the blob (lines 282, 810, 905:
`spawn_daemon`, `invoke_cli`, `invoke_stdio_mcp`) removes all three
`ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, and `ENGRAM_DIRECT` variables.

R9.5(e): deferred until the three current FL runs, never credited from B0.

## Drift ledger and comparison contract

`git log --oneline f6f3171f..8366194d -- src/tools/capabilities.rs`:

```text
b8721d67 feat(capabilities): classify generation-backed reads for 142.086-T
a13c4245 test(capabilities): scaffold 142.086-T generation provenance harness
```

Both are 142.086-T PA5-T6 changes merged in PR #480 (`99878753`).
The descriptor change adds generation classification; FL retains the old
classification function unchanged. No new surface row was identified.
B2(vi) PA5-P's pre-enumerated table is the authority: workspace-status
provenance may pass; generation reads against `probe_row` may instead
fail acceptance; query_changes is absent from default-build B0.
All other signatures remain governed by B2(i), (ii), (iii), (v).
Same OS as B0: Windows. Full output stays in tool evidence, not redirected.

## FL load

Full `6d216d19` lines 1..=1633 were read and loaded first.
Only the three R9.5 adaptation groups differ:

* Include `../helpers/activation_settle.rs`
* Call the settle barrier once at the ready-return point of `ensure_daemon`
* Replace `workspace_hash` with `read_server_layout` and use its workspace ID
* Operator-approved compile-only import correction:
  `use engram::services::generations::candidate_build::read_server_layout;`
  replaces the wrong grouped `read_server_layout` import

All database bytes, generation ID, revision, digest, root and producer
are unchanged. `git diff 6d216d19 -- <test>` is retained in tool evidence.
Baseline `cargo check --all-targets` passed with
`CARGO_TARGET_DIR=target/validation`. FL check and maps are pending.
SB-1a deadline remains 2026-11-01T07:39:14Z.

## Mandatory compile halt

`FL-RED-HALT 560b17ee4907dc22ff7e1146590ec8e80af32000`
(uncommitted FL blob; no FL or HC commit exists).

Command: `cargo check --tests`, with `CARGO_TARGET_DIR=target/validation`.
Native exit: 101. Exact failure:

```text
error[E0432]: unresolved import `engram::services::generations::read_server_layout`
  --> tests\contract\read_server_cli_mcp_parity_test.rs:25:53
   |
25 |     WorkspaceIdentity, publish_generation_manifest, read_server_layout,
   |                                                     ^^^^^^^^^^^^^^^^^^ no `read_server_layout` in `services::generations`

For more information about this error, try `rustc --explain E0432`.
error: could not compile `engram` (test "contract_read_server_cli_mcp_parity") due to 1 previous error
```

No import fix after this guard, no FL test run, no B2 comparison verdict,
no Slot-20a share/behavior placeholders, no label/comment mutation,
no HC commit, no amend/reset/push. Full test output is unavailable because
compilation halted first. R9.5(e) is unverified.
The final diff against the 1,633-line reference has 14 added/adapted lines
and 3 removed lines (17 total diff lines, excluding context). The intended
1,351-line share and its authored measurement were not reached.

Intake report remains PROCEED for intake only:
`.backlogit/reconcile/174-S-intake-20261101T053200Z.md`.
Caller claim dirt is preserved. Task and shipment remain active.
The protected feature hash was rechecked and is unchanged.
PID 47916 was not touched. No helper or production file was edited.
Operator decision is required before resuming.

## Operator-authorized resume

Operator explicitly authorized the exact `candidate_build` import above.
The pre-adaptation `cargo check --all-targets` reproduced the known E0432
(exit 101); this is the previously recorded halt, not a verified baseline
or a new FL check. Prior successful baseline and provenance remain as
recorded above. No checkpoint, claim, status or history operation was run.
Current FL verification begins after this authorized correction.

## Resume FL halt evidence

`FL-RED-HALT 22303a693d0f45b68d1ca8f36392998901f34694`

Corrected full FL body: original `6d216d19` lines `1..=1633`,
plus the three adaptation groups and exact compile-only import above.
Diff against `6d216d19`: 14 additions, 2 removals (16 changed lines,
conservative added-plus-removed measure), below 400.
No Slot-20a share was landed. The intended `1..=1351` share and RED
roster were not reached.

`cargo check --tests`: exit 0, 3.76 s.
`cargo test --test contract_read_server_cli_mcp_parity -- --nocapture`:
FL run 1 exit 101; build 53.42 s, tests 28.06 s. Full output is in
tool evidence (`fl1`); no output was redirected by the actor.
Both commands set `CARGO_TARGET_DIR=target/validation` in their shell.
Default parallelism was retained. All 76 `F54 settle:` lines were `Active`;
no `F54-BLOCK` signature was observed.

### Complete FL run 1 map

| B0 row | Test | Current result |
|---|---|---|
| 1 | `generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces` | GREEN |
| 2 | `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` | RED |
| 3 | `control_descriptors_are_refused_without_side_effects` | RED |
| 4 | `unknown_ipc_methods_are_refused_without_side_effects` | RED |
| 5 | `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` | GREEN |

Row 4 retains the single assertion signature:
`F54-RED: not_a_declared_method: unknown method did not use read-server write/control refusal code 16_001`.
Its response has `engram_code=5005`, `engram_name=InvalidParams`.
Row 5 has no failure signature.

The following table is the complete ordered row 2 failing signature map.
Within each table row, methods expand in listed order, then surfaces in
listed order, then assertions in listed order. `all` expands to
`direct_ipc`, `cli`, `stdio_mcp`.

| Methods | Surfaces | Assertions |
|---|---|---|
| `get_daemon_status` | all | P |
| `flush_state` | all | R, C, S |
| `index_workspace`, `sync_workspace` | all | C |
| `query_memory`, `get_workspace_statistics`, `map_code`, `list_symbols`, `unified_search`, `impact_analysis`, `query_graph` | all | A, P |
| `lint_dax`, `get_health_report`, `get_branch_metrics`, `get_token_savings_report`, `get_evaluation_report`, `get_mutable_script_retry_metrics` | all | P |
| `run_retrieval_eval` | all | R, C |
| `get_retrieval_eval_report` | `direct_ipc`, `stdio_mcp` | P |
| `_shutdown` | `direct_ipc` | R, C, S |

Assertion keys (exact expectation prefixes):

* A: `expected an accepted read result`
* P: `expected serving-generation provenance`
* R: `expected a read-server refusal`
* C: `expected stable F38 refusal code 16001`
* S: `expected zero filesystem and workspace-binding side effects`

Row 3's entire ordered map is `_shutdown | direct_ipc | R, C, S`.
Its observed shutdown result is accepted, no refusal code, no generation,
side-effect count 1. The B0 `set_workspace | stdio_mcp | expected stable
F38 refusal code 16003` failure is absent.

### B2 verdict and R9.5(e)

B0 was read directly from
`git show 0f2cca20:docs/memory/2026-10-04-ship-142-f-a0-b0.md`.
Same OS: Windows. PA5-P Unit 8 and its pre-enumerated B2(vi) table
were read alongside R-A6/R-A7 and R9.5.

* B2(i)/(vi), `142.080-T`: workspace-status failure signatures disappear
  on all surfaces and row 5 turns GREEN, as explicitly permitted
* B2(iii): workspace-statistics now also fails acceptance
* B2(vi), `142.081-T`..`142.084-T`: listed generation-read rows now fail
  acceptance as enumerated; unchanged classification also retains P
* B2(vi), `142.085-T`: `query_changes` remains absent in this default build
* B2(ii): shutdown assertion prefixes remain; teardown consistency cannot
  be credited without three runs
* B2(v): verified capabilities ledger contains generation classification
  changes only; it attributes neither mismatch below
* Mismatch 1: B0 row 2 `get_daemon_status | stdio_mcp | A` disappeared
* Mismatch 2: B0 row 3 `set_workspace | stdio_mcp | expected stable F38
  refusal code 16003` disappeared

Neither mismatch is enumerated by B2(vi) or attributed by the existing
B2(v) ledger. B2 verdict: FAIL; R9.5(e) is not verified.
Row 5 GREEN is explicitly allowed, not itself a mismatch.
The immediate mismatch guard stopped work after inspecting run 1.
Runs 2 and 3 were not executed; identical-three-map evidence is unavailable.

No new label, implementation note, HC, IC, commit, reset, amend, push,
claim, move, completion or archive operation occurred. Structural row 1
passed outside any future expected-RED roster. No Worker roster was authored
or executed. Production, helper, config, plans and review files were untouched.
Operator disposition is required; parent must not treat this as harness-ready.

## Ship recovery and final handoff

At 2026-11-01T05:39:56Z, Ship resolved the operator-selected
`checkpoint-20261101-053712.json` after owner validation, bound-state
read, bounded prune summary and confirmed resume. The subsequent full
checkpoint enumeration showed zero active records and no quarantine
anomalies. Engram PID 47916 was reachable in `read_server` mode; its
bound state reported `main` and no active generation, so no indexed
code-analysis result was relied upon.

The resumed `post_claim` topology gate passed with `174-S` as the sole
active shipment and the correct branch/worktree. All five prerequisites
have archived `done` provenance. Hook polling returned no events.
Fresh configuration validated against the installed harness schema.

After the new FL B2 halt, Ship created the single active recovery pointer
`checkpoint-20261101-054458.json` through the official V1 create operation.
It is intentionally unresolved pending Operator/Stage disposition.
The branch remains `feat/174-s-142-f-slot-20a-142-087-t-f54a`;
there is no feature or closure PR, no HC or IC, no full-suite run, and
no P-021 capture. No push occurred because the binding FL halt prohibits
one without a new operator decision.

## Stage-attributed resume and three-run FL verification

The historical halt sections above remain evidence of the decisions made
at those points; they are not erased or retroactively changed to PASS.
The current task, read through `backlogit get 142.087-T`, now records Stage
B2 (iv)/(vi) attribution for both startup-race signatures. Operator
authorized this continuation after Ship restored, pruned, resumed and
resolved `checkpoint-20261101-054458.json`, confirmed zero active
checkpoints, and passed `post_claim` topology. These recovery statements
are caller-provided; this actor performed no checkpoint or claim operation.

The full adapted FL blob remained
`22303a693d0f45b68d1ca8f36392998901f34694` throughout FL2 and FL3.
No correction occurred between FL runs. Baseline
`cargo check --all-targets` exited 0 (39.69 s) before any new test write.

Both consecutive commands were exactly:

```powershell
$env:CARGO_TARGET_DIR='target/validation'
cargo test --test contract_read_server_cli_mcp_parity -- --nocapture
```

Working directory: `C:\Source\GitHub\engram`. No
`RUST_TEST_THREADS` override, runner substitution, or full-suite run.
Complete stdout/stderr was returned as tool evidence for `fl2` and `fl3`;
the actor used no output redirection. FL2 exited 101 (build 7.02 s,
tests 25.57 s); FL3 exited 101 (build 1.52 s, tests 25.00 s).
Each run discovered five tests, with 2 GREEN / 3 RED, 76/76 settle lines
`Active`, and zero `F54-BLOCK` signatures.

### Complete FL2 and FL3 maps

The following maps use the same B0 row order as FL1, regardless of
parallel test completion order.

| B0 row | Test | FL2 | FL3 |
|---|---|---|---|
| 1 | `generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces` | GREEN | GREEN |
| 2 | `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` | RED | RED |
| 3 | `control_descriptors_are_refused_without_side_effects` | RED | RED |
| 4 | `unknown_ipc_methods_are_refused_without_side_effects` | RED | RED |
| 5 | `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` | GREEN | GREEN |

This is the complete ordered row 2 signature map for each run.
Expand methods first, then surfaces, then assertions, in listed order.
`all` means `direct_ipc`, `cli`, `stdio_mcp`, in that order.
Assertion keys A, P, R, C, S retain their exact FL1 expectation prefixes
defined above. Each column expands to 89 signatures.

| Methods | Surfaces | FL2 assertions | FL3 assertions |
|---|---|---|---|
| `get_daemon_status` | all | P | P |
| `flush_state` | all | R, C, S | R, C, S |
| `index_workspace`, `sync_workspace` | all | C | C |
| `query_memory`, `get_workspace_statistics`, `map_code`, `list_symbols`, `unified_search`, `impact_analysis`, `query_graph` | all | A, P | A, P |
| `lint_dax`, `get_health_report`, `get_branch_metrics`, `get_token_savings_report`, `get_evaluation_report`, `get_mutable_script_retry_metrics` | all | P | P |
| `run_retrieval_eval` | all | R, C | R, C |
| `get_retrieval_eval_report` | `direct_ipc`, `stdio_mcp` | P | P |
| `_shutdown` | `direct_ipc` | R, C, S | R, C, S |

Row 3's complete ordered map in both runs is
`_shutdown | direct_ipc | R, C, S` (three signatures).
Row 4's only signature in both runs is:

```text
F54-RED: not_a_declared_method: unknown method did not use read-server write/control refusal code 16_001
```

Row 4 retains `engram_code=5005`, `engram_name=InvalidParams`.
Rows 1 and 5 have no failure signatures.

An executable comparison expanded the FL1 table and compared every
ordered `(method, surface, expectation-prefix)` tuple in FL2 and FL3,
plus the control and unknown-method maps and named outcomes.
Verdict: **FL1 = FL2 = FL3, PASS**, including shutdown teardown consistency.
There was no additional signature, missing case, wrong prefix, or
non-Active settle observation.

### Current B2 verdict

All earlier B2(i)/(ii)/(iii)/(v)/(vi) comparisons remain as recorded.
The two formerly unattributed changes are now explicitly accepted by
Stage B2 (iv)/(vi), as recorded on the task:

* Row 2 `get_daemon_status | stdio_mcp | A` disappears; the read is
  accepted and fails only P, matching direct IPC and CLI
* Row 3 `set_workspace | stdio_mcp | expected stable F38 refusal code
  16003` disappears; the observed refusal is `16_003` with zero side effects
* Both are attributed to `142.064-T` PRE-3 startup gate installation
  before readiness and `142.063-T` PRE-3F settle in `ensure_daemon`,
  eliminating B0's pre-settle stdio MCP startup race

Current B2 verdict: **PASS**. R9.5(e) is now verified by three identical
complete maps, with every settle line Active. This is not a fix to any
other task's code or an unrecorded B2 allowance.

## Slot-20a HC evidence

Only after FL verification, the test was trimmed to the matching
`6d216d19` regions `1..=1351`: shared core `1..=1319` (1,319 lines) and
structural coverage `1320..=1351` (32 lines), total 1,351 reference lines.
The listed settle, layout/workspace-ID and compile-only import
adaptations are unchanged. Database fixture bytes and generation
metadata remain unchanged. No structural-test reference line was adapted.

The appended 57 authored lines contain two behavior tests and a task-owned
`slot20a_assertion_boundary` placeholder. Both tests exercise the real
fixture and finish marker-verified daemon cleanup before reaching their
distinct `unimplemented!` markers. Retrieval surfaces come only from
the descriptor; no hard-coded CLI absence or retrieval surface list is
introduced. Retrieval acceptance/catalog coverage is checked without
claiming Slot-20d generation provenance. The health case uses direct IPC
and the shared ready-health assertion machinery.

Matching-region unified diff: 71 additions, 2 removals,
**73 conservative authored/adapted lines <= 400** under R-A6.
The 57 authored lines plus 14 added and 2 removed adaptation lines
produce a 1,420-line landed file. The estimate was approximately 52,
not a 52-line cap. R-A7's PA5-P ownership and split remain unchanged.
The exact matching-region diff is preserved below.

### Compilation and RED

`cargo check --all-targets`: exit 0, 12.40 s.
`cargo fmt --all -- --check`: exit 0.
`cargo clippy --all-targets -- -D warnings -D clippy::pedantic`:
exit 0, 1m 24s. No `allow`, `expect` or `warn` attribute was added.
Every cargo shell set `CARGO_TARGET_DIR=target/validation`.

Exact bounded RED command:

```powershell
$env:CARGO_TARGET_DIR='target/validation'
cargo test --test contract_read_server_cli_mcp_parity -- --nocapture
```

Native exit 101, build 22.67 s, tests 2.54 s; three discovered,
one structural PASS, two intended behavior failures, none ignored or
filtered. All three settle lines were Active. The command remained
bounded as requested; the `dev-test` alias adds `--all-targets`, so it was
not used to launch a prohibited full suite during HC.

| Expected-RED roster test | Native outcome | Exact unimplemented output |
|---|---|---|
| `retrieval_report_exercises_exact_descriptor_declared_surfaces` | FAILED | `not implemented: Worker: 142.087-T:retrieval_report_exercises_exact_descriptor_declared_surfaces` |
| `health_is_ready_on_direct_ipc_only` | FAILED | `not implemented: Worker: 142.087-T:health_is_ready_on_direct_ipc_only` |

Both fail at the task-owned `unimplemented!` line, not an assertion
bearing a marker, compilation failure or another task's placeholder.
The case suffixes are deterministic and asserted pairwise distinct.
Outside the roster:
`generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces`
PASS (structural, no stub). No characterization tests were generated.

```text
running 3 tests
test generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces ... ok
not implemented: Worker: 142.087-T:health_is_ready_on_direct_ipc_only
test health_is_ready_on_direct_ipc_only ... FAILED
not implemented: Worker: 142.087-T:retrieval_report_exercises_exact_descriptor_declared_surfaces
test retrieval_report_exercises_exact_descriptor_declared_surfaces ... FAILED
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
```

Compilation: **PASS**. Red Phase: **CONFIRMED**.
Only after these observations, `backlogit update 142.087-T --labels
harness-ready` succeeded and the official `backlogit comment add` recorded
the command, roster/markers, FL comparison and adaptation measure
(`actor=harness-architect`, `ok=true`). The official JSONL comment log
is ignored by repository policy; the durable HC evidence is recorded here.

HC staging includes the test, this evidence file, and only the task's
`harness-ready` label and lifecycle timestamp. Pre-existing task
claim/status and Stage task-text dirt, shipment claim, intake and
checkpoint files remain separate for Ship's later persistence.
Protected parent SHA256 was rechecked unchanged:
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770`.
Deadline remains `2026-11-01T07:39:14Z`.
Build-feature may replace only this task-owned placeholder and validate
the three slot scenarios GREEN. HC stops without IC, PR, push, claim,
checkout, task completion or full-suite execution.

### Exact matching-region diff

```diff
--- 6d216d19:1..1351
+++ tests/contract/read_server_cli_mcp_parity_test.rs
@@ -9,0 +10,3 @@
+#[path = "../helpers/activation_settle.rs"]
+mod activation_settle;
+
@@ -19 +22 @@
-use engram::db::workspace::workspace_hash;
+use engram::services::generations::candidate_build::read_server_layout;
@@ -263,0 +267,6 @@
+                activation_settle::await_activation_settled(
+                    &self.endpoint,
+                    self.temporary_root.path(),
+                    1,
+                )
+                .await?;
@@ -477,0 +487,3 @@
+    let layout = read_server_layout(&workspace.to_string_lossy()).unwrap_or_else(|error| {
+        panic!("{F54_BLOCK_MARKER}: derive fixture read-server layout: {error}")
+    });
@@ -511 +523 @@
-        WorkspaceIdentity::new(workspace_hash(workspace, FIXTURE_BRANCH)),
+        WorkspaceIdentity::new(layout.workspace_id),
@@ -1351,0 +1364,57 @@
+// Route (c): build-feature replaces this task-owned assertion boundary.
+// Requests and marker-verified cleanup run before the placeholder is reached.
+fn slot20a_assertion_boundary(case: &str) {
+    assert_ne!(
+        "retrieval_report_exercises_exact_descriptor_declared_surfaces",
+        "health_is_ready_on_direct_ipc_only"
+    );
+    unimplemented!("Worker: 142.087-T:{case}");
+}
+
+#[tokio::test]
+async fn retrieval_report_exercises_exact_descriptor_declared_surfaces() -> Result<(), String> {
+    let report = descriptor("get_retrieval_eval_report")
+        .ok_or_else(|| "F19 must declare get_retrieval_eval_report".to_owned())?;
+    let mut fixture = ReadServerFixture::new().await;
+    let mut exercised = Vec::with_capacity(report.surfaces.len());
+    let mut observations = Vec::with_capacity(report.surfaces.len());
+    for &surface in &report.surfaces {
+        observations.push(exercise_declared_surface(&report, surface, &mut fixture).await?);
+        exercised.push(surface);
+    }
+    fixture.stop_daemon().await?;
+    slot20a_assertion_boundary("retrieval_report_exercises_exact_descriptor_declared_surfaces");
+    assert_eq!(exercised, report.surfaces);
+    assert!(!exercised.is_empty());
+    for (surface, observation) in exercised.iter().zip(&observations) {
+        assert!(observation.accepted, "{surface}: {observation:?}");
+        if *surface == ToolSurface::StdioMcp {
+            assert_eq!(observation.surface_advertised, Some(true));
+        }
+    }
+    // Serving-generation provenance belongs to Slot-20d, not this slot.
+    Ok(())
+}
+
+#[tokio::test]
+async fn health_is_ready_on_direct_ipc_only() -> Result<(), String> {
+    let health = descriptor("_health").ok_or_else(|| "F19 must declare _health".to_owned())?;
+    let mut fixture = ReadServerFixture::new().await;
+    let observation =
+        exercise_declared_surface(&health, ToolSurface::DirectIpc, &mut fixture).await?;
+    fixture.stop_daemon().await?;
+    let mut failures = Vec::new();
+    record_matrix_failures(
+        &health,
+        DeclaredSurfaceExpectation {
+            surface: ToolSurface::DirectIpc,
+            outcome: ExpectedOutcome::ReadyHealth,
+        },
+        &observation,
+        &mut failures,
+    );
+    slot20a_assertion_boundary("health_is_ready_on_direct_ipc_only");
+    assert_eq!(health.surfaces, [ToolSurface::DirectIpc]);
+    assert!(failures.is_empty(), "{}", failures.join("\n"));
+    Ok(())
+}
```

The protected `.backlogit/queue/142-F.md` SHA-256 remains
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770`.
The slot deadline remains 2026-11-01T07:39:14Z.
`175-S` pre-claim was not invoked: `174-S` remains active and incomplete.
Next action is Operator/Stage disposition of the two B2 mismatches,
then explicit selection and confirmation of the new Ship checkpoint.
