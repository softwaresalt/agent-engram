# 140-S Shipment Execution — Session Memory

**Session**: Dark-mode (P-017) resumed run, scope `shipment_ids=[140-S]` only.
**Branch**: `feat/140-s-migrate-services-to-pinned-context-and-enforce-read-path-pinning`
**Base**: `main` @ `2589c350` (Merge pull request #403)

## Items completed (all 7 manifest tasks — status: done)

| Task | Title | Commit(s) |
|---|---|---|
| 142.040-T | Migrate search and embedding service to pinned context | `95928d7a` |
| 142.041-T | Migrate registry and evaluation service to pinned context | `a809bc4d` |
| 142.042-T | Migrate retrieval evaluation service to pinned context | `bcf55cb4` |
| 142.043-T | Migrate metrics and query-stat service to pinned context | `3bf9ec31` |
| 142.044-T | Migrate DAX lint service to pinned context | `1ffad745` |
| 142.045-T | Migrate git graph service to pinned context | `3f68400c` |
| 142.046-T | Enforce read-path pinning with a static guard test | `00ac9358` + follow-up fix `3227a68b` |

## Cross-task defect found and remediated (in-scope, same shipment)

142.046-T's closing static guard (`tests/contract/read_path_pinning_enforcement_test.rs`)
initially went RED (2 of 4 subtests failing) against the fully-landed F25-F36 codebase,
revealing that 142.044-T (dax lint service) and 142.042-T (retrieval eval service /
`get_retrieval_eval_report` handler) left `workspace_root` / `.engram` path construction
threaded outside the approved single pinning seam. Since both offending files
(`src/services/dax_lint.rs`, `src/tools/lint.rs`, `src/tools/eval.rs`) are owned by
sibling tasks WITHIN this same shipment (140-S), and 142.046-T's own acceptance criterion
is "the guard is GREEN only after all of F25-F36 have landed", this was classified as
in-scope completion (P-021 C1: same contract surface — the shipment's own stated
read-path-pinning-migration-plus-enforcement deliverable) rather than a deferred
out-of-scope finding. Fixed in commit `3227a68b`. All 4 guard subtests plus all 6
sibling service pin tests are green after the fix.

## Second cross-task regression wave found and remediated (in-scope, same shipment)

After the guard went green (`3227a68b`), the full-suite run (`cargo test --all-targets
--no-fail-fast`) surfaced a second wave of regressions caused by the same class of bug:

* **`contract_lint_dax` (2/6 subtests failing)**: root cause — `dax_lint.rs`'s
  `pinned_lint_paths()` and `registry.rs`'s `load_registry_status()` both derived
  "workspace root" via `context.data_dir().parent()`-style guessing, which is
  fundamentally wrong because `data_dir` and workspace `path` are independently
  configured (confirmed via `WorkspaceSnapshot` test fixtures in `state.rs` /
  `lifecycle.rs` — sibling dirs, no fixed positional relationship; `ENGRAM_DATA_DIR`
  can decouple them arbitrarily). **Fix**: added `ReadRequestContext::root_path()`
  accessor (`src/server/state.rs`, populated from `snapshot.path`) and a shared
  `registry::registry_path_for(workspace_root)` helper (kept in `registry.rs`, not
  `dax_lint.rs`, to respect the 142.046-T guard's zero-tolerance ban on
  `.join(".engram")` inside `dax_lint.rs`/`eval.rs`/`lint.rs`). Updated both
  `pinned_lint_paths()` and `load_registry_status()` to use `context.root_path()`.
  Committed `31e4d3a6`.
* **`report_read_generation_pin` (1 subtest failing)**: root cause — a *test fixture*
  bug (not production code) in `report_read_generation_pin_test.rs`'s pre-existing
  `MetricsFixture` (predates 140-S): `data_dir: workspace.join(".engram-data")` never
  matched where `seed_metrics_workspace` actually wrote files
  (`workspace.join(".engram")`). This mismatch was inert before 142.043-T's migration
  (old code didn't consult `data_dir` for this path) and surfaced once
  `get_health_report` began reading via `metrics::load_summary(context, ...)`.
  Initially misapplied the dax_lint/registry fix pattern to `metrics.rs` itself —
  WRONG, because `metrics_service_pin_test.rs` (142.043-T's own harness, in-scope)
  explicitly mandates `data_dir()`-based resolution as the correct contract (matches
  production default `resolve_data_dir()` behavior). Reverted the `metrics.rs` change;
  fixed the stale fixture instead. Committed `eda83c03`.

Both fixes are in-scope per P-021 C1: same shipment, completing the already-authorized
read-path-pinning migration correctly (the guard test's own acceptance criterion
requires all F25-F36 services to be genuinely pinned, not just textually passing).

## Local review gate (Step 4.4) — outcome

Invoked the `review` skill (`mode:report-only`) against the full diff (`2589c350..HEAD`,
`src/` + `tests/`). Reviewer returned one P1 finding:

**Metrics writer/reader `data_dir` divergence under `ENGRAM_DATA_DIR`** — the metrics
usage-events WRITER (`usage_path`/`resolve_usage_path`/`append_event_line`, untouched by
this shipment) resolves its file from raw `workspace_path`, while 142.043-T's newly
migrated READ handlers resolve via `context.data_dir()`. These coincide only in the
default (unconfigured) case; under a configured `ENGRAM_DATA_DIR` they diverge, and reads
silently return empty/null instead of the real data — arguably violating 142.043-T's own
acceptance criterion "Managed-mode behavior is unchanged."

**Investigated and attempted a direct fix** (route the writer's default path through
`resolve_data_dir(workspace_path)` to match the reader) — this is technically "in-scope"
by P-021 C1 reasoning (same task's own acceptance criterion), so a same-shipment fix was
attempted first rather than deferred outright. **The fix was reverted** after it
reproduced two concrete regressions in `tests/integration/usage_telemetry_emit.rs`
(`t067_004_branch_aware_path_is_cross_platform`,
`t067_004_dual_source_correlation_id_emits_records`), root-caused to this dev/CI shell
having an ambient `ENGRAM_DATA_DIR` env var set to a fixed external path (confirmed via
`$env:ENGRAM_DATA_DIR`) — introducing a live env-var read into the writer path is
unsafe without first threading a single pinned `data_dir` value through the entire
background-writer subsystem (channel messages, branch-switch/generation transitions,
override/containment semantics), which is a materially larger change than 142.043-T's
stated scope (`owned files: src/services/metrics.rs` + its own pin test, narrowly about
the 4 read handlers). Reverted via `git checkout -- src/services/metrics.rs`; confirmed
`cargo check`, targeted metrics tests (`integration_usage_telemetry_emit`,
`integration_metrics_service_pin`, `integration_report_read_generation_pin`), clippy, and
fmt all clean at the reverted (i.e., 140-S's actual shipped) state.

**Captured as P-021 deferred stash entry `E6CA4ED1`** (kind: bug, priority: high,
requires-deliberation: yes) via the Step 4.4a threadless defer-capture procedure
(pre-PR, no review thread exists yet). Source refs: shipment_id=140-S, feature_id=142-F,
task_id=142.043-T, PR=N/A, review-thread=N/A.

## Local Review Readiness — final verdict

The dispatched reviewer's raw verdict was `BLOCKED` (based solely on the P1 finding
above, evaluated without P-021 deferral context). Applying Ship's P-021 C1/C2
classification per Step 4.4a: the P1 finding is a genuine regression introduced by
142.043-T, but the only way to fix it *safely and completely* (thread a single pinned
`data_dir` through the entire background-writer subsystem — channel messages,
branch-switch/generation transitions, override/containment semantics, plus new test
coverage for a real writer under a deliberately-diverged `ENGRAM_DATA_DIR`) is
materially larger than 142.043-T's stated 2-hour-rule, single-owned-file scope, and a
narrow attempt at the fix was proven unsafe (reproduced 2 new regressions). It therefore
fails the C1 "ONLY completing the exact change already authorized" test and correctly
follows the Step 4.4a defer-capture procedure rather than a direct fix.

**Final readiness outcome: `READY_WITH_FOLLOWUPS`.**

Follow-up / residual-risk items for the PR body:
* Stash `E6CA4ED1` (P1/high, requires deliberation) — metrics writer/reader `data_dir`
  consistency under `ENGRAM_DATA_DIR`.
* Stash `10EE5E43` (P2-equivalent/medium, requires deliberation) — release-archive
  smoke test flakiness investigation (later full-suite evidence suggests this may be
  transient/environmental rather than a deterministic defect; noted in the entry's
  Ship-owned residual-risk annotation, entry itself left unedited per the C2
  single-write invariant).

No P0 findings. No other unresolved P1 findings (the constitution/Rust/correctness/
maintainability/security/test-coverage passes all came back clean). Reviewed HEAD:
`cd55e95d` (session-memory update commit) at time of this review pass; no further
production-code changes were made afterward, so this remains the reviewed HEAD through
Step 5.

## Deferred out-of-scope finding (P-021 C2 captured)

During full-suite verification, `integration_release_archive_smoke_workflow`'s
`archive_verifier_runs_the_unpacked_native_binary` failed twice (once in a
mid-suite run, once in an isolated single-target re-run) with "non-JSON stdout from
MCP stdio". This test exercises native release-binary packaging / MCP-stdio transport
conformance — a contract surface with zero file overlap with 140-S's manifest.
Classified out of scope per P-021 C1 and captured via the Step 4.4a threadless
defer-capture procedure: **stash entry `10EE5E43`** (kind: bug, priority: medium,
requires-deliberation: yes). **Update**: a subsequent fully clean full-suite run
(538 test binaries, 0 failures) showed this exact test PASSING, along with the other
three tests that had failed transiently in earlier runs (`hcl_indexing_test`,
`integration_daemon_lifecycle`, `integration_daemon_startup_order`) — all four are
almost certainly timing/parallel-load-sensitive flakiness, not deterministic defects,
and none touch files in 140-S's diff. Per the P-021 C2 single-write invariant, stash
entry `10EE5E43` is NOT edited/re-classified to reflect this; the update is recorded
here in Ship's own residual-risk record for Stage's triage to consider (entry may be
downgraded/closed as non-reproducible flakiness upon investigation).

## Verification performed

* `cargo check --all-targets`: pass (multiple times, after each task and after both
  cross-task fix waves).
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: pass (final run
  clean).
* `cargo fmt --all -- --check`: pass (final run clean).
* Combined targeted run:
  `cargo test --test integration_search_service_pin --test integration_registry_service_pin
  --test integration_retrieval_eval_service_pin --test integration_metrics_service_pin
  --test integration_dax_lint_service_pin --test integration_git_graph_service_pin
  --test contract_read_path_pinning_enforcement` — all green (17+ tests), plus
  `report_read_generation_pin` green after the fixture fix.
* **Full `cargo test --all-targets --no-fail-fast`** (Ship Step 4.3 whole-suite gate):
  final clean run — **538 test binaries executed, 0 failures**. This is the
  authoritative full-build evidence for the PR's `## Local Review Readiness` block.

## Backlog state

* Shipment `140-S`: `active` (claimed at session start; claim cascaded all 7 manifest
  tasks from `queued` to `active` automatically — backlogit 1.10.1 behavior).
* All 7 manifest tasks: `done` (backlogit 1.10.1 auto-archives `done` tasks from
  `.backlogit/queue/` to `.backlogit/archive/` on the `move --status done` call — this
  is expected version behavior, not an integrity problem).
* Pre-archive reconciliation note: shipment-level closure archival (Step 6) has NOT
  run yet — that only happens after user-approved merge. The per-task archive moves
  observed here are backlogit's own task-level lifecycle, distinct from the
  shipment-closure safe-close sequence.
* Dependencies 138-S, 139-S: terminal/archived (confirmed before claim).
* P-001: no other active shipment; queued sibling shipments 141-S, 142-S untouched
  (never claimed, never processed) — 141-S explicitly excluded from this run's scope
  per the dark-mode activation record.

## Branch / worktree state

* Single implementation worktree at `C:/Source/GitHub/engram`; no spike/research
  worktree conflicts observed.
* `.backlogit/stash.jsonl` carries a pre-accepted dirty modification (operator-accepted
  per the dark-mode activation record) that has been preserved and integrated (not
  reverted or discarded) throughout every commit in this session — none of the task
  commits staged it.

## Commit history (final, base → HEAD)

```
95928d7a feat(142.040-T): migrate search and embedding service to pinned context
a809bc4d feat(142.041-T): migrate registry and evaluation service to pinned context
bcf55cb4 feat(142.042-T): migrate retrieval evaluation service to pinned context
3bf9ec31 feat(142.043-T): migrate metrics and query-stat service to pinned context
1ffad745 feat(142.044-T): migrate DAX lint service to pinned context
3f68400c feat(142.045-T): migrate git graph service to pinned context
00ac9358 feat(142.046-T): enforce read-path pinning with a static guard test
3227a68b fix(142.044-T,142.042-T,142.046-T): complete workspace-root pinning to satisfy the F37 read-path guard
94fff843 chore(140-S): record task completion state and session memory checkpoints
31e4d3a6 fix(142.041-T,142.044-T,142.046-T): resolve live workspace root via ReadRequestContext instead of guessing from data_dir
eda83c03 test(142.043-T): align report_read_generation_pin metrics fixture data_dir with the seeded metrics path
```

Base: `2589c350` (Merge pull request #403). Working tree clean aside from the
pre-accepted `.backlogit/stash.jsonl` modification.

## Next steps (as of this checkpoint)

1. ~~Run full local quality gate suite~~ — DONE, all green (clippy, fmt, full
   `cargo test --all-targets --no-fail-fast`: 538 binaries, 0 failures).
2. Run local review gate (`review` skill, `mode:report-only`) — Step 4.4.
3. Runtime-verification / operational-closure evaluation (touches read-path dispatch —
   runtime surface) — Step 5 items 7-8.
4. PR creation via `pr-lifecycle`, CI remediation as needed, P-018 copilot-review gate,
   P-014 readiness gate — Step 5.
5. Halt for explicit operator merge approval (merge_approval_pre_authorized=false,
   admin_fallback_pre_authorized=false per dark-mode activation record).
6. Only after operator-approved merge: Step 6 post-merge closure (safe-close via
   `shipment-reconcile`, operational-closure, compound-refresh, mandatory P-020
   compact-context, backlog index resync).

## PR #404 creation, CI, and Copilot-review disposition (this session segment)

* **PR #404 created**: `https://github.com/softwaresalt/agent-engram/pull/404`,
  base `main`, from the feature branch. Body includes the `## Local Review Readiness`
  block (updated twice this segment as HEAD advanced).
* **CI flake recurrence**: `start-launcher-windows`
  (`launcher_fails_open_to_copilot_within_one_prewarm_budget`, 8s wall-clock budget)
  failed intermittently on 3 of 4 pushed HEADs this segment (`adf2d274`: 3 failures
  then pass on rerun; `1745e531`: 1 failure then pass on rerun; `4f4d4486`: passed on
  first attempt). Confirmed environmental/hosted-runner timing variance, not a
  regression — PR diff never touches the relevant files, and this exact flake matches
  a pre-existing deferred entry, `F58ECAA8` (reused, no new capture, per the
  discovery/reuse protocol). `build` job passed cleanly on every HEAD.
* **Copilot-review engagement (2 passes, re-arms per push)**:
  * **Pass 1** (at HEAD `adf2d274`, after the reconciliation-frontmatter fix): 6
    threads raised, "Changes recommended."
    1. `dax_lint.rs` Generation-mode `root_path()` gap — pre-existing, confirmed via
       pre-migration `tools/lint.rs::pinned_workspace_root` also Managed-mode-only.
       Deferred as new stash `9BB01D31` (medium).
    2. 142.046-T static guard's substring-matching misses aliased vars
       (`root`/`scan_root`) and omits `registry.rs` from scope — guard-methodology
       redesign, out of scope (adding `registry.rs` to scope would make the guard RED
       against its own new `registry_path_for` helper with no F24-style allow-list).
       Deferred as new stash `A3E0E607` (medium).
    3. Restates the known metrics.rs writer/reader `data_dir` divergence — reused
       `E6CA4ED1` (late-surfacing thread, reply+resolve, no new entry).
    4. Same divergence pattern in `eval.rs`/retrieval_eval (different file/task,
       142.042-T) — distinct per the same-contract-surface discovery test. Deferred
       as new stash `7C23A682` (high).
    5. Critiques the `report_read_generation_pin_test.rs` fixture fix (`eda83c03`) as
       masking rather than fixing the divergence — reused `E6CA4ED1` (same root
       cause, reply+resolve, no new entry).
    6. Missing `recommendation:` frontmatter in
       `.backlogit/reconcile/140-S-pre-20260918T001534.md` — **in-scope** (140-S's
       own artifact, trivial completion), fixed directly, commit `adf2d274`.
    All 6 threads replied-to (citing the deferred entry ID or fix commit) and
    resolved via GraphQL. P-018 gate re-run: `SATISFIED` at `adf2d274`.
  * Pushed `1745e531` (runtime-verification + operational-closure docs) and
    `4f4d4486` (typo fix inside the closure doc, see below).
  * **Pass 2** (at HEAD `4f4d4486`, against the new closure docs): 2 more threads.
    7. Typo in `docs/closure/2026-09-18-140-s-operational-closure.md`'s validator
       evidence sentence ("harnesses +ent the static guard") — **in-scope** (own
       artifact, trivial typo), fixed directly, commit `4f4d4486`.
    8. `metrics.rs::load_usage_events`/`load_summary` join a caller-supplied
       `branch` string (from public `branch_name`/`compare_to` tool parameters) into
       a filesystem path with no sanitization (potential path-traversal / arbitrary
       file read). Confirmed **pre-existing**: `git show main:src/tools/read.rs`
       shows the identical unsanitized join already existed pre-140-S
       (`parsed.branch_name`/`compare_to` → `metrics::compute_summary` with no
       validation); 140-S only relocated the read call to the new pinned-context
       `load_summary`/`load_usage_events`. A real fix needs a shared branch-name
       sanitizer applied uniformly across the untouched writer path and all reader
       paths — materially larger than this shipment's scope. Deferred as new stash
       `DB0661A6` (high, requires_deliberation: true).
    Both threads replied-to and resolved. P-018 gate re-run: `SATISFIED` at
    HEAD `4f4d4486` (the HEAD current at that point in the session; a further
    checkpoint commit and a 3rd Copilot pass followed — see the correction
    note at the top of the "Pre-merge state as of commit `4f4d4486`" section
    below).
* **Discovery/reuse protocol applied consistently**: searched both
  `.backlogit/stash.jsonl` (active) and `.backlogit/archive/stash.jsonl` (archived)
  before every capture this segment. Confirmed 3 exact reuse matches (`F58ECAA8` for
  the CI flake, `E6CA4ED1` ×2 for the metrics.rs divergence) and confirmed zero prior
  matches before each of the 4 new captures (`9BB01D31`, `A3E0E607`, `7C23A682`,
  `DB0661A6`).
* **PR body updated twice** via `gh pr edit` to keep the `## Local Review Readiness`
  block current with the final reviewed HEAD (`4f4d4486`), all 8 Copilot findings and
  their dispositions, and the complete follow-up stash list.

## Runtime verification and operational closure (pre-merge)

* Built release binary (`cargo build --release --bin engram`, ~6m12s). `engram
  --version` PASS (embedded SHA matched HEAD). `engram daemon-status` — spawned an
  isolated test daemon (PID 38536, confirmed distinct from the ~9 pre-existing
  `C:\Tools\engram.exe` tooling daemons via `Get-Process`), all critical health checks
  green, only expected transient yellows; cleanly terminated afterward
  (`Stop-Process -Id 38536 -Force`).
* `contract_initialize`/`contract_tools` test names were stale (matches existing
  deferred entry `DA0AF326`, reused, no new capture) — used the correct current test
  names instead.
* Found 2 tests failing under `--release` profile only
  (`shim_aborts_unresolved_startup_after_client_disconnects`,
  `shim_recovers_after_timed_out_daemon_later_becomes_ready`) — confirmed unrelated to
  140-S's diff and release-profile-specific via an immediate debug-mode re-run (19/19
  green). Documented as an observation only (loosely related to existing entry
  `FFA32805` re: CI never testing release profile); no new stash entry.
* Wrote `docs/closure/2026-09-18-140-s-runtime-verification.md` — verdict
  `PASS_WITH_FOLLOW_UP`.
* Wrote `docs/closure/2026-09-18-140-s-operational-closure.md` — `mode: pre-merge`,
  overall status `READY_WITH_CONDITIONS` (condition: explicit operator merge approval
  per P-014 and the dark-mode activation record's
  `merge_approval_pre_authorized=false`). Compaction status left `pending` (finalized
  only at post-merge Step 6). Source-artifact-cleanup section is a placeholder
  awaiting post-merge Step 6.
* Committed both docs (`1745e531`), later fixed a typo in the closure doc (`4f4d4486`,
  per Copilot pass 2 finding 7 above).

## Pre-merge state as of commit `4f4d4486` (historical checkpoint)

**Corrected 2026-09-18 (readiness audit)**: this section, as originally
written in the commit containing this record, called `4f4d4486` the "final"
HEAD. That claim was already stale at the moment of that commit, because
the commit containing this record was itself pushed after `4f4d4486` (and a
3rd Copilot pass ran after that push, flagging the same staleness on this
file and on the operational-closure doc, plus a missing follow-up stash
entry there). This section is left below as an accurate historical record
of state at commit `4f4d4486`; it is not further edited to chase each
subsequent HEAD. The PR's mutable `## Local Review Readiness` block — not
this file — is the operator-visible, gating-authoritative source for
current-HEAD status.

* **PR #404**: as of `4f4d4486`: OPEN, `mergeable: MERGEABLE`,
  `mergeStateStatus: CLEAN`, HEAD `4f4d4486`.
* **CI**: `build` PASS (6m19s), `start-launcher-windows` PASS (2m4s) — both green at
  that HEAD.
* **P-018 copilot-review gate**: `SATISFIED` at HEAD `4f4d4486`, 0 unresolved
  threads at that point (8 total threads across 2 passes, all replied-to and resolved).
* **P-009 merge-strategy check**: repo settings confirmed
  `allow_merge_commit=true, allow_squash_merge=false, allow_rebase_merge=false` —
  merge-commit-only, compliant.
* **Pipeline-topology gate** (`--phase lifecycle`): re-run, exit 0, all 5 checks
  passed (`detect_before_consistency`, `active_shipment_invariant`,
  `branch_ownership` → `BRANCH_OK`, `worktree_topology` → `WORKTREE_TOPOLOGY_OK`,
  `shipment_readiness`).
* **Branch/worktree**: still on
  `feat/140-s-migrate-services-to-pinned-context-and-enforce-read-path-pinning`;
  `git status --short` shows only the pre-accepted dirty `.backlogit/stash.jsonl`
  (now carrying 4 new entries from this full session: `9BB01D31`, `A3E0E607`,
  `7C23A682`, `DB0661A6`, plus the 2 from the prior segment, `E6CA4ED1`, `10EE5E43`).
* **Halted for explicit operator merge approval** per
  `merge_approval_pre_authorized=false` /
  `admin_fallback_pre_authorized=false` — no merge attempted, no admin fallback
  attempted. This is the terminal state for this session.
* PR #396 and PR #390 were not mutated at any point this session (out of scope per
  the dark-mode activation record).
* 141-S was never claimed or processed.
