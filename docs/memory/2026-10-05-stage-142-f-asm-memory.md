---
title: "Stage 142-F landing step ASM (assembly) memory: section 8 steps 1-10 complete, awaiting PR and operator merge"
date: 2026-10-05
agent: stage
feature: "142-F"
plan: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
plan_revision: 18
rulings: "docs/decisions/2026-10-04-142-f-rev18-execution-rulings.md (R-A1 to R-A5); docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md (R-A6, R-A7, R-A8)"
landing_step: ASM
branch: chore/stage-142-f-assembly
base: "origin/main @ ce851761e887694cb18d2a45311c2af6e5890ae3 (PR #412, H3)"
routed_model: "claude-opus-5.5 / anthropic / high"
status: assembly-complete-awaiting-pr-and-merge
progress: "section 8 steps 1-10 PASS (step 6 close-out 26/26; step 7 24 shipments and 31 edges; step 8 R-A4 U7 7/7; step 9 old Constitution Check replaced by a pointer to section 13; step 10 CG-Q 24/24); pre-PR self-review PASS (0 P0, 0 P1, 2 P2 fixed, 3 P3: 1 fixed, 2 no action); post-merge step 11 owed"
halt_token: "none open (historical: R17-SIZE-GATE 142.058a FL-share, resolved by operator options A and C, recorded as R-A6 and R-A7, 2026-10-05 22:58)"
halt_location: "plan section 8 step 4 (splits), S-20 pre-creation share measurement (plan 4.3 S-20, last sentence; 4.2 row 20a-20d)"
copilot_session: a9421395-751d-42b5-9e69-6d40f356a110
---

# Stage 142-F ASM: memory

## Outcome

**Final status, 2026-10-06: assembly steps 4-10 are complete. The branch is
waiting for its PR and the operator's merge (CG-M).** Section 8 steps 1-10 all
passed on `chore/stage-142-f-assembly`, and the pre-PR self-review passed with
no open P0 or P1 (see "Pre-PR self-review" below). The branch holds 24 one-task
slot shipments, `143-S` to `166-S` (Slot-01 to Slot-19), each `queued`, with 31
mirrored `blocks` edges and `custom_fields.queue_position` in the Markdown. The
handoff map is in "Handoff map" below. The S-20 split and Slot-19.k, Slot-20a to
Slot-20d and Slot-21 belong to PA5-P (R-A7). Nothing was pushed and no PR was
opened; the Orchestrator does both. The cache rebuild and the other step-11
actions run only after the merge (see "Post-merge step-11 actions still owed").

### Outcome history (2026-10-05 halt, superseded)

**Update 2026-10-06: step 4 resumed under R-A7 and PASSED** (see "Step 4,
resumed" below). **Step 5 (edges) PASSED** the same day (see "Step 5, edges"
below). The text that follows, down to the step 4 section, records the
original 2026-10-05 halt and is kept as history.

Assembly ran section 8 steps 1-3 to completion. It stopped at step 4 with
**HALT `R17-SIZE-GATE 142.058a FL-share`**, before any split task was created.
Steps 4-10 did not run at that point, so there were no new tasks, no edge
changes, no content rewrites, no shipments, and no lift comments. The step-3
resets were committed on this branch. The cache rebuild was not run.

(Superseded: "Do not push or open a PR from this branch until the operator rules
on the HALT and Stage resumes assembly at step 4." The operator ruled, R-A6 and
R-A7, and steps 4-10 have since passed.)

## Session setup

| Check | Result |
|---|---|
| Step 0.0 tool gate | No backlogit MCP tool in this session. The registered CLI fallback (`backlogit` 1.11.0, `.autoharness/backlog-registry.yaml`) was used throughout: `DEGRADED_MODE: backlogit MCP -> CLI fallback`. `backlogit metadata catalog` and `metadata templates` were refreshed first. Task sections are `description`, `acceptance-criteria` and `implementation-notes`; shipment sections are `description`, `items` and `blocked-returns`. `features.sizing` is absent, so sizing stays in prose. |
| Step 0.1 index sync | Skipped on purpose. Plan section 11 and `docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md` warn that a plain `backlogit sync` can union a stale cache, and H3 saw broad merge-sync drift. Step 2's `get` vs Markdown check covers the index instead. |
| Checkpoint recovery | `backlogit checkpoint list`: no active `stage` checkpoint and no anomalous record, so this was a normal zero-candidate start. |
| Branch | `git fetch origin`; `git switch -c chore/stage-142-f-assembly origin/main` at `ce851761`. The upstream was then unset so nothing can push to `main` by accident. There is one worktree. The only untracked path is `.backlogit/.locks/` (tool runtime state), which was never staged. |

## Section 8 steps

### Step 1, preconditions: PASS

* The plan frontmatter has `revision: 18` and `status: frozen-rev18`.
* `origin/main` merges include PR #409 (H0, `7984f896`), #410 and #411 (H1, `c5949973` and `6ae48561`), and #412 (H3, `ce851761`).
* PS-5 is applied. `.backlogit/archive/142-S.md` has `status: archived`, `archived_status: abandoned` and `archived_from: .backlogit/queue/142-S.md`, and `.backlogit/queue/142-S.md` is absent.
* T0 `parked/142-s-split-0f2cca20` exists locally and on origin: tag object `bb82ad4e`, which peels to commit `0f2cca20`.

### Step 2, read-only re-check: PASS

* `backlogit get --json` was compared with the Markdown frontmatter and body (status, parent, title, priority, archived_status, type, commit, dependencies, labels, custom_fields and body) for 37 items. The items were `142-F`, `142-S`, `137-S`, `140-S`, `141-S`, `142.054-T` to `142.074-T`, every `142.054.00x-ST`, `142.058.00x-ST` and `142.064.00x-ST`, and the archived dependencies `142.025-T` and `142.026-T`. Result: **all 37 MATCH (0 diverged)**, so `R14-CACHE-DIVERGED` was not raised.
* `142-S` is archived `abandoned`, and `142-F` is `active`.
* R14.2 feasibility holds. `.autoharness/config.yaml` has no uncommented `lifecycle_hooks` or `pre_task_completion` key; the whole block is commented out, so the hook is disabled. `backlogit move --help` covers task and subtask completion, the subtask status enum includes `done`, `.backlogit/registry.yaml` routes `done` to `archive/`, and `142.058.001-ST` is a working precedent. `R14-SUBTASK-MOVE-UNSUPPORTED` was not raised.

### Step 3, PS-6 resets under R-A5: PASS

| ProposedAction | ActionRisk | Approval | ActionResult |
|---|---|---|---|
| PS-6: reset `142.054-T` to `142.059-T` and the open subtasks `142.054.001-ST`, `.002-ST`, `.003-ST`, `142.058.002-ST` and `.003-ST` from `active` to `queued` | moderate | 2026-09-29 18:14, extended by the R-A5 approval of 2026-10-04 23:16 | Applied. 11 `backlogit move <id> --status queued` calls, all exit 0. |

The order followed R-A5: within each family the subtasks went first, then the parent.
The full sequence was 054.001, 054.002, 054.003, 054-T, 055-T, 056-T, 057-T,
058.002, 058.003, 058-T, 059-T. After each write, the index and the Markdown both
read `queued`. The rollup guard (10.3) ran after each write. `142-F` stayed
`active` (index and Markdown agree), and each subtask's parent stayed `active`
until its own reset, so `R15-ROLLUP-DRIFT` was not raised. `142.058.001-ST`
(archived `done`) was not touched. The diff is 11 queue files, with only
`status` and `updated_at` changed in each. backlogit logged an INFO line
"index may be stale after mutation" on each move, but every read-back afterwards
was consistent.

**CG-S caution.** The shared local index now holds `queued` for these 11 items,
while `main`'s Markdown still says `active`. If this worktree switches to `main`
and anything runs a plain `backlogit sync` before the assembly PR merges, the two
diverge. Keep the worktree on this branch until assembly resumes.

### Step 4, resumed 2026-10-06 under R-A7: PASS

The operator chose options A and C (R-A6, R-A7, 2026-10-05 22:58). Under R-A7,
step 4 creates only the S-02, S-04 and S-09 split tasks and rewrites only
`142.061-T`, `142.064-T` and `142.054-T`. The S-20 split, `142.058a` to `c`, E3
and the 142.058 chain of E15 move to PA5-P. `142.058-T` and its subtasks were
not touched.

**Pre-checks: PASS.** Branch `chore/stage-142-f-assembly` at HEAD `e7832c6c`.
The tree was clean apart from the untracked `.backlogit/.locks/`, which was
never staged. The 11 PS-6 resets still read `queued` in both `backlogit get`
and the Markdown. Tooling: backlogit CLI only (no MCP), and no `backlogit sync`
of any kind. `backlogit get` reads the Markdown body, so edits made with the
edit tool are visible to `get`.

**Placeholder → real ID map (the handoff for steps 5-8):**

| Placeholder | Real ID | Slot | Title | Size \| Complexity |
|---|---|---|---|---|
| `142.061a` | `142.075-T` | 02a | PRE-1a: Add read-server layout, identity and generation-ID minting | S \| medium (1 h) |
| — | `142.061-T` | 02b | PRE-1b: Build and seal candidate generation (PRE-1 close) | S \| medium (1 h) |
| `142.064a` | `142.076-T` | 04a | PRE-3a: Replace read-server startup identity with read_server_layout | S \| medium (1.5 h) |
| `142.064b` | `142.077-T` | 04b | PRE-3b: Add read-server gate slot, install before readiness, drive activation | S \| high, de-risked (1.5 h) |
| — | `142.064-T` | 04c | PRE-3c: Retry transient same-revision activation and close PRE-3 | S \| medium (1.5 h) |
| `142.054a` | `142.078-T` | 09a | F50a: Define typed preflight stages, transitions and mock harness | S \| medium (1.5 h) |
| `142.054b` | `142.079-T` | 09b | F50b: Implement Build, Seal and Publish preflight stages | S \| medium (1.5 h) |
| — | `142.054-T` | 09c | F50c: Implement preflight probe stages and real-chain cases | S \| medium (1.5 h) |

Every new task has parent `142-F`, status `queued` and no dependencies. Each
new task copies its parent's priority: medium for the PRE-1 and PRE-3 families,
high for F50. The PRE tasks carry the label `preflight`. `pending-pa-1` was not
copied, and `harness-ready` was not copied to F50a or F50b. Sizing is prose
only, because `features.sizing` is absent. Each family task has its share in
`description`, `acceptance-criteria` and `verification`, with the size, the
SG-1 record, the HALT conditions and the full family AC map in
`implementation-notes`. Each rewritten parent was retitled, and `142.061-T`,
`142.064-T` and `142.054-T` got a history line recording the split.

**SG-1 (4.1), re-run on each family task: all OK, no `R17-SIZE-GATE`.**

| Task | Files | Fns | Scen. | Open ST | Est. | Verdict |
|---|---|---|---|---|---|---|
| `142.075-T` | 2 (+ `mod.rs` line, stanza) | 4 (const counted) | 1 | 0 | S, 1 h | OK |
| `142.061-T` | 2 | 2 | 2 | 0 | S, 1 h | OK |
| `142.076-T` | 2 (+ stanza) | 1-2 | 2 | 0 | S, 1.5 h | OK |
| `142.077-T` | 2 + glue (`state.rs` slot) | 3 | 3 | 0 | S, 1.5 h | OK (high, de-risked: careful mode, own RED, recorded HALT) |
| `142.064-T` | 2 | 1 | 2 | 0 | S, 1.5 h | OK |
| `142.078-T` | 2 | 3 (est) | 3 | 0 | S, 1.5 h | OK |
| `142.079-T` | 2 | 4 (est) | 3 | 0 | S, 1.5 h | OK |
| `142.054-T` | 2 | 4 (est) | 3 | 0 | S, 1.5 h | OK |

"Open ST 0" follows plan 4.2 and 10.1. The `142.064.00x-ST` and
`142.054.00x-ST` subtasks stay as records under the parents, untouched. Stage
closes them through 10.1 after Slot-04c and Slot-09c, so they aren't separate
Ship work. Two counts differ from 4.2, and both still pass. For `142.075-T`, the
`READ_SERVER_ACTIVATION_DEADLINE` const body was counted, giving 4 functions
where 4.2 lists 3. For `142.061-T`, the harness file was counted, giving 2 files
where 4.2 lists 1.

**AC map checks (4.3, R-A4): PASS for all three families, with no
`R17-SPLIT-AC-UNMAPPED` and no double mapping.** The check was a script against
the AC and verification tags plus every task's map table; a negative test
showed it catches both an unmapped item and a double mapping. Item inventories
were confirmed against the bullet counts at `e7832c6c`.

* **S-02:** 12 items. `142.075-T` has 4 (AC2-AC5) and `142.061-T` has 8.
* **S-04:** 40 items. `142.076-T` has 10, `142.077-T` has 10, `142.064-T` has
  12, and 8 are "moved to `142.058-T` per M-n". The moved items are M1-M5,
  M6 (`.003-ST` closure, F54 part), M14 (the `.001-ST` F54 map) and M15 (the
  scope and HALT-note text). Scenario 1, scenario 2 and the consumer-baseline
  bullet are mapped by part (a/b/c), as the 4.3 S-04 wording requires.
* **S-09:** 36 items. `142.078-T` has 16, `142.079-T` has 7 and `142.054-T`
  has 13.

**Edges: none written at step 4.** No family task needs a predecessor copy at
creation:

* `142.061-T` has no open predecessor.
* `142.064-T`'s only open predecessor is `142.063-T`. E1 removes that edge, so
  under R-A2 `142.076-T` copies nothing.
* `142.054-T`'s only open predecessor is `142.068-T`; all its others are
  archived. The 5.1 E14 row lists the copy itself, as "054a → 068", so step 5
  adds it.

**Rollup guard (10.3): PASS after every write.** `142-F` stayed `active`, and
every family task and parent stayed `queued` in both the index and the
Markdown. `R15-ROLLUP-DRIFT` was not raised. The dependency frontmatter of
`142.064-T` (`142.063-T`) and `142.054-T` (13 dependencies) is unchanged.
`backlogit get` matches the Markdown on title, body, parent and dependencies
for all 8 items.

**Step 5 needs to know (real IDs for E5, E1, E12-E14):**

* E5 is `142.076-T` → `142.062-T`.
* E1 removes both `142.064-T` → `142.063-T` and the R-A2 edge `142.076-T` →
  `142.063-T`. The second edge doesn't exist, so that half is a no-op. Then
  `get_dependencies` must show `142.062-T` only for `142.076-T`; anything else is
  `R15-DAG-ORDER Slot-04a <predecessor>`.
* E12 is `142.061-T` → `142.075-T`.
* E13 is `142.077-T` → `142.076-T` and `142.064-T` → `142.077-T`.
* E14 is `142.078-T` → `142.068-T`, `142.079-T` → `142.078-T` and `142.054-T` →
  `142.079-T`.
* E3 and the 142.058 chain of E15 are skipped (R-A7).

**Step 6 needs to know:**

* The split tasks hold their share text and AC maps, but not the full section 9
  content (U2 Shipment and "Depends on" lines, U4, U8, U9, 9.2).
* `pending-pa-1` is still on `142.061-T` and `142.064-T` (U1/T16). The new tasks
  don't have it.
* The map tables quote no U1-banned token. A scan of all 8 tasks, excluding
  history sections, found only that `pending-pa-1` label.

**Step 7 needs to know:** the Slot-02a, 04a, 04b, 09a and 09b shipments take
`142.075-T`, `142.076-T`, `142.077-T`, `142.078-T` and `142.079-T`, at
`queue_position` 20, 40, 41, 90 and 91.

### Step 4, splits: HALT, no task created (2026-10-05, superseded above)

**HALT `R17-SIZE-GATE 142.058a FL-share`**, raised before creation. Plan 4.3 S-20
says "At the split, Stage measures each slot's share of the `6d216d19` body; a
share over 400 lines is split again by case class before creation." Plan 4.2
gives the same gate for rows 20a-20d: "OK if each FL share ≤ 400 lines". SB-1 (c)
then halts any Step 2 harness diff over 400 lines.

Measurement, read-only, of `git show 6d216d19:tests/contract/read_server_cli_mcp_parity_test.rs`:
the blob is 1,633 lines.

| Lines | Region | Size | Used by |
|---|---|---|---|
| 1-152 | module docs, imports, consts, `Observation`, `ExpectedOutcome`, matrix structs, `MarkedDaemon` | 152 | all |
| 153-443 | `ReadServerFixture` (spawn, IPC, binding fingerprint, snapshot, stop, Drop) | 291 | every daemon case |
| 444-526 | `create_workspace`, `publish_fixture_generation`, `engram_binary` | 83 | every daemon case |
| 527-669 | `request_parameters`, `sample_schema_value`, `sample_object_value` | 143 | `exercise_declared_surface` |
| 670-754 | `exercise_declared_surface` | 85 | every daemon case |
| 755-1042 | keepalive wait, `invoke_direct_ipc`, `invoke_cli`, `cli_arguments`, `invoke_stdio_mcp`, MCP frames and decode | 288 | `exercise_declared_surface` (it matches on all three surfaces) |
| 1043-1145 | refusal-code helpers, `snapshot_directory`, `changed_entry_count` | 103 | `exercise_declared_surface` |
| 1146-1319 | outcome classifiers, `descriptor_parity_matrix`, `describe_failure`, `record_matrix_failures` | 174 | all matrix cases |
| 1320-1633 | the five tests, `exercise_unknown_method`, `normalize_parity_result` | 314 | per case |

Every daemon case runs through `exercise_declared_surface`, which calls the direct
IPC, CLI and stdio MCP invokers, the snapshot and binding helpers, and the
fixture. The refusal, equivalence and read-provenance cases all do this. So the
first slot that lands any daemon case must also land about 1,145 lines of shared
harness, or about 1,264 with the matrix-failure recorder. Only the no-daemon
structural/coverage test (about 270 lines) could fit under 400.

Section 9.2 assigns `142.058a` (Slot-20a) the declared-surface rule and `_health`
on direct IPC. Both are daemon cases, so Slot-20a's share is at least about
1,145 lines, nearly three times the cap. A further split "by case class" can't
help, because the shared core isn't a case class. Spreading the helpers across
earlier slots, without their callers, would trip `dead_code` under
`-D warnings`. The usual escape, `allow`, `expect` or `warn` attributes, is
banned by R11.10 L1 and A15. That leaves no split within the plan and rulings
that brings each share to 400 lines or fewer. Stage made no repair and no
improvised re-cut. Stage also halted before creating any of the eight split
tasks, so no partial split hierarchy exists.

**Options for the operator** (Stage recommends; the operator decides):

* **A (recommended): a ruling R-A6 on S-20 sizing.** The 400-line FL-share bound
  (4.2, 4.3 S-20) and SB-1 (c) would count only the lines the harness-architect
  authors or adapts. The verbatim `6d216d19` load is byte-identical, already
  reviewed test code, and 9.2 already requires its compile-only adaptations to
  be listed and proven by `cargo check --tests`. That load would be measured and
  recorded but not capped. S-20 keeps its four tasks, and assembly resumes at
  step 4 unchanged.
* **B: re-cut S-20 by layer.** Slot-20a would hold only the structural/coverage
  case (about 270 lines), followed by a harness-core slot (about 1,000 lines or
  more), then the rest. The core slot still needs option A's exemption, and B
  adds slot labels, `queue_position` values and an AC re-map, which is a plan
  change. Stage sees no benefit over A.
* **C (can combine with A): decouple.** Assembly would create Slot-01 to Slot-19
  now and move the S-20 split to PA5-P, which already creates the 058-family
  shipments. That means creating `142.058a` to `c`, adding E3 and E15, and
  writing the 058-family content in 9.2 there. None of Slot-01 to Slot-19
  depends on the 058 family. This needs a ruling, because section 8 steps 4-6
  name those items.

Related: the 058-T share has the same problem. CLI/MCP equivalence and read
provenance both need the same core, so option A or C would settle it too.

**Pre-checks done without writes, for the resumed step 4:** for S-02, S-04 and
S-09, the 4.2 SG-1 verdicts stand (OK). Under R-A2, `142.064a` doesn't copy the
`142.064-T` → `142.063-T` edge. The planned post-step-5 DAG (below) has no cycle.

### Step 5, edges, 2026-10-06 under R-A2 and R-A7: PASS

Run on `chore/stage-142-f-assembly` from HEAD `5fde317b`, with the backlogit CLI
1.11.0 only (`backlogit dep add|remove|list`, no MCP, no `backlogit sync` of any
kind). Every edge is `--type blocks`. `dep add` and `dep remove` write the
`dependencies` frontmatter of the item's Markdown file, so the edges are durable
in the commit. Nothing else in the files changed.

| Edge | Operation | Result |
|---|---|---|
| E5 | add `142.076-T` → `142.062-T` | added |
| E1 | remove `142.064-T` → `142.063-T` | removed |
| E1 (R-A2) | remove `142.076-T` → `142.063-T` | no-op: the edge never existed (the CLI reports "Removed" anyway; the file was unchanged) |
| E2 | add `142.063-T` → `142.066-T` | added |
| E3 | — | skipped (R-A7, moved to PA5-P) |
| E11 | remove `142.060-T` → 054, 055, 056, 057, 058 | 5 removed; `142.060-T` has no dependencies |
| E8 | add `142.063-T` → `142.060-T` | added |
| E10 | add `142.065-T` → `142.064-T` | added |
| E12 | add `142.061-T` → `142.075-T` | added |
| E13 | add `142.077-T` → `142.076-T`, `142.064-T` → `142.077-T` | 2 added |
| E14 | add `142.078-T` → `142.068-T`, `142.079-T` → `142.078-T`, `142.054-T` → `142.079-T` | 3 added |
| E15 | — | 142.058 chain skipped (R-A7, moved to PA5-P) |

**Checks after each edge: PASS.** `backlogit dep list` showed the expected
dependency set after every write. After E5 and E1, `142.076-T` depends on
`142.062-T` only, so there's no `R15-DAG-ORDER Slot-04a`. The rollup guard
(10.3) re-read `142-F` and both endpoints of each edge in the index and the
Markdown after every write. `142-F` stayed `active` and every task stayed
`queued`, so `R15-ROLLUP-DRIFT` was not raised.

**Full cycle check after the last edge: no cycle, no `R15-DAG-CYCLE`.** It ran
a DFS over the Markdown `dependencies` of all 130 open queue items (90 edges
between open items) and over the index edges of `142.054-T` to `142.079-T`.
The index and the Markdown agree for all 26 tasks.

**Planned-DAG comparison: match, no difference.** The final graph for
`142.054-T` to `142.079-T` equals the planned post-step-5 DAG below with the
058a-c lines removed. `142.058-T` keeps 066 and 065 plus its archived
predecessors, and `142.059-T` keeps 055, 057, 058, 025 and 026.

**Edge counts for `142.054-T` to `142.079-T`:** 63 before step 5, 10 added, 6
removed (the R-A2 half of E1 was a no-op), 67 after. Of the 67, 36 point at
open tasks and 31 at archived ones. Files changed: `142.054-T`, `142.060-T`,
`142.061-T`, `142.063-T`, `142.064-T`, `142.065-T`, `142.076-T`, `142.077-T`,
`142.078-T` and `142.079-T`.

**Step 6 needs to know:** E3 and the 142.058 chain of E15 are not in the graph;
PA5-P adds them (R-A7). Any section 9 "Depends on" line for `142.058-T` or
`142.063-T` that mentions `142.058a` describes a PA5-P edge, not a current one.

### Step 6, content, batch 1 of 4 (Slot-01 to Slot-04c), 2026-10-06: PASS

Run on `chore/stage-142-f-assembly` from HEAD `dd6fedfd`, backlogit CLI only, no
`backlogit sync` of any kind. Body text was rewritten with the edit tool;
`pending-pa-1` was removed with `backlogit update <id> --labels preflight`, which
changes only the label list and `updated_at`. Each task got a dated step-6 line
in its Markdown history section (U1-exempt). `142.064-T`'s history line also
records that its 2026-10-04 SUPERSEDED-body note no longer applies.

| Slot | Task | Depends on (= `dep list`) | Route | Size, est. | Label removed | Check |
|---|---|---|---|---|---|---|
| 01 | `142.060-T` | none (E11) | (a-1) | S \| medium, 1.5 h | — | PASS |
| 02a | `142.075-T` | none | (a-1) | S \| medium, 1 h | — | PASS |
| 02b | `142.061-T` | `142.075-T` (E12) | (a-1) | S \| medium, 1 h | `pending-pa-1` | PASS |
| 03 | `142.062-T` | `142.061-T` | (c) | S \| medium, 1.5 h | `pending-pa-1` | PASS |
| 04a | `142.076-T` | `142.062-T` (E5) | (c) | S \| medium, 1.5 h | — | PASS |
| 04b | `142.077-T` | `142.076-T` (E13) | (c) | S \| high, de-risked, 1.5 h | — | PASS |
| 04c | `142.064-T` | `142.077-T` (E13) | (c) | S \| medium, 1.5 h | `pending-pa-1` | PASS |

**Checks per task (scratch script `tmp/asm/step6-check.ps1`, gitignored):**
U2 (the Shipment line, a "Depends on" line whose IDs equal `backlogit dep list`,
the 4.2 size and estimate), U3, U4 (route form; the exact route (c) wording), U5,
U6 (R12.3 line, no "Owned files"), U8 (merge commit and SHA), U9 (SB-1 quoted, with
the slot's 1.5 × limit), and the 9.2 strings: `142.060-T` SEQUENCING, "only code
task of its shipment", the Slot-01 PR residual-risk record and `a47b8aff` as
reference; `142.076-T` "Depends on: PRE-2 (E5)"; `142.077-T` both gate accessors
`pub`; all three S-04 tasks with no F54 ownership, own RED and comparison against
`142.076-T`'s record; `142.064-T` three identical careful-mode runs of its own
target, not F54 (M6, M15, R7.8; added as an AC and verification step 4). No
`R16-CONTENT-CHECK`. The U1 scan (description, acceptance, notes and verification;
history and comment sections exempt; the regexes passed a negative test) found
nothing, so no `R16-STALE-TEXT`; the `pending-pa-1` label is gone from all seven.
U7 STATUS-RESET doesn't apply to this batch (R-A4), and `PA-6-LIFTED` for
`142.060-T` is step 8's. Rollup guard after every write: `142-F` `active` and each
task `queued` in the index and the Markdown; no `R15-ROLLUP-DRIFT`. `backlogit get`
body equals the Markdown body for all seven.

**Removed U1 text:** `142.060-T` "final code task of 142-S … active shipment 142-S
(plan PA1)", "opened forward after the F50 cycle", the 142-S PR residual-risk
record and the "mapped pending-red … interim gate" risk line; `142.062-T`
"OUTSIDE shipment 142-S: do not start until PA-1 is granted". "Owned files" in
`142.060-T` and `142.062-T` became "Files edited" / "Production files edited" (U6).

**Notes for batches 2-4:** `pending-pa-1` is still on `142.063-T` and `142.065-T` to
`142.074-T`. Keep each "Depends on" line on its own line (the check takes every
task ID on the line, so a "Downstream" list must go on a separate line).
`142.055-T` to `142.059-T` need the STATUS-RESET line in their history section
(U7, R-A4); `142.063-T` alone keeps the `F54 settle:` contract string (R-A4 U1
exemption; its R9.6 "S4" wording is still rewritten in slot terms). `142.068-T`
uses `engram --workspace <W> --json workspace-status`, never a bare `status` (R-A3).

### Step 6, content, batch 2 of 4 (Slot-05 to Slot-09c), 2026-10-06: PASS

Run on `chore/stage-142-f-assembly` from HEAD `a0546051`, backlogit CLI only, no
`backlogit sync` of any kind. `pending-pa-1` was removed from `142.065-T` to
`142.068-T` with `backlogit update <id> --labels preflight`. Each body was
rewritten from a scratch draft spliced in after the frontmatter
(`tmp/asm/b2/*.body.md` and `splice.py`, gitignored); the 054-family AC map
tables were carried over byte for byte. Each task got a dated step-6 line in
its Markdown history section (U1-exempt).

| Slot | Task | Depends on (= `dep list`) | Route | Size, est. | Label removed | Check |
|---|---|---|---|---|---|---|
| 05 | `142.065-T` | `142.062-T`, `142.064-T` (E10) | (c) | S \| medium, 1.5 h | `pending-pa-1` | PASS |
| 06 | `142.066-T` | `142.064-T`, `142.065-T` | (c) | S \| medium, 1.5 h | `pending-pa-1` | PASS |
| 07 | `142.067-T` | `142.066-T` | (c) | S \| medium, 1.5 h | `pending-pa-1` | PASS |
| 08 | `142.068-T` | `142.067-T` | (c) | S \| medium, 1.5 h | `pending-pa-1` | PASS |
| 09a | `142.078-T` | `142.068-T` (E14) | (a-1) | S \| medium, 1.5 h | — | PASS |
| 09b | `142.079-T` | `142.078-T` (E14) | (a-1) | S \| medium, 1.5 h | — | PASS |
| 09c | `142.054-T` | `142.079-T` (E14), `142.068-T` + 12 archived `done` | (a-1) | S \| medium, 1.5 h | — | PASS |

**Checks per task** (`tmp/asm/step6-check.ps1` through the batch-2 wrapper
`tmp/asm/b2/check2.ps1`, both gitignored; every task was first run against its
pre-rewrite text as a negative test, and each check fired): U2 (Shipment line,
every "Depends on" line equal to `dep list`, 4.2 size and estimate), U3, U4 (the
exact route (c) or (a-1) wording), U5, U6 (R12.3 line, no "Owned files"), U8,
U9 (SB-1 quoted, limit (a) 2.25 h), the U1 scan, and the 9.2 strings:

* `142.065-T`: the verbatim line "Consumer regression targets unchanged against
  the PRE-3 RED-phase baseline. HALT if any goes RED."; verification step 3 is
  consumer targets only; no `F54`, `PRE-3F` or `after-edit` in the checked text.
* `142.066-T`: the inline R7.7 rule (F1/G1, snapshot 1, 250 ms, snapshot 2,
  F2/G2; no IPC between the snapshots; snapshots and status compared separately;
  25 ms re-poll, 30 s deadline, never pass on timeout); `= &[];` at Step 2, the
  implementation adds `"get_workspace_statistics"`; the exact `assert!(...contains(...))`
  statement; no `F54`, "final value" or `assert_eq!`. M9-M11 removed; the R7.8
  three careful-mode runs added as an AC and verification step 3.
* `142.067-T`: the R14.6 row (`probe_cli_read` with a nonexistent exe, `Spawn`,
  then the `PREFLIGHT_READ.mcp_tool` membership assertion).
* `142.068-T`: inline `structuredContent` extraction (M17) and
  `engram --workspace <W> --json workspace-status` (R-A3); no bare `--json status`.
* `142.078-T`, `142.079-T`, `142.054-T`: re-qualification on the slot's base
  tree, parked `c269fa79`/`41dd5081` reference only, PA-2b real-chain RED is
  `142.054-T`'s, and the `cargo test -p engram-indexer --all-targets --no-fail-fast`
  and engram-indexer clippy gates with no pending-RED allowance. `142.079-T`'s
  step-4 "(derived)" clippy criterion became that gate.
* `142.054-T` U7: the STATUS-RESET line in A16 form is in its history section
  (first line verbatim; base tree filled as Slot-09c; harness evidence
  `c269fa79`, `41dd5081`; label `harness-ready` kept; subtasks `.001`-`.003-ST`).
  No precedent existed (no Markdown or log line held one), so the A16 template
  was filled from plan R11.2 and A16, and a separate step-6 history line says so.

No `R16-CONTENT-CHECK`, no `R16-STALE-TEXT`. Rollup guard after every write:
`142-F` `active` and each task `queued` in the index and the Markdown; no
`R15-ROLLUP-DRIFT`. `backlogit get` body equals the Markdown for all seven.

**Removed U1 text:** "OUTSIDE shipment 142-S; requires PA-1" (065-068); the
"PA-1 proposes … edge" and "under PA-1 (proposed edge …)" lines (066, 068); the
065 F54/after-edit-map clause (M7-M8); the 066 F54 rows (M9-M11) and the F54
equivalence-RED note; the tag name `parked/142-s-split-0f2cca20` in `142.054-T`
(now "the parked tag T0, plan 7.3"). "Owned files" became "Production files
edited" (065, 067).

**Notes for batches 3-4:**

* R7.7's snapshot labels `S1`/`S2` hit the U1 `\bS[1-5]\b` scan, so `142.066-T`
  writes them "filesystem snapshot 1/2"; the meaning is unchanged. Watch the same
  regex in `142.063-T` (R9.6 "S4" wording) and in any quoted R-text.
* The U1 scan is case-insensitive, so the tag name `parked/142-s-split-…` trips
  `142-S`; write "the parked tag T0 (plan 7.3)".
* Every "Depends on:" line must list every edge, including archived `done`
  predecessors (`142.056-T`, `142.057-T` and `142.059-T` have archived ones).
* Out of scope, not changed: `142.066-T` scenarios 2 and 3 still say the harness
  sends `status` / `status` is accepted. The CLI has no `status` subcommand
  (`workspace-status`, `src/bin/engram.rs` line 84). R-A3 covers `142.068-T`
  only, so this is left for an Orchestrator ruling. (Settled by R-A8; applied in
  batch 3, below.)

### Step 6, content, batch 3 of 4 (R-A8, Slot-10 to Slot-15), 2026-10-06: PASS

Run on `chore/stage-142-f-assembly` from HEAD `6818c6e9`, backlogit CLI only, no
`backlogit sync` of any kind. `pending-pa-1` was removed from `142.069-T` to
`142.074-T` with `backlogit update <id> --labels ...`, which changes only the label
list and `updated_at`. Each body was rewritten from a scratch draft spliced in after
the frontmatter (`tmp/asm/b3/*.body.md`, `tmp/asm/b3/splice.py`, gitignored). None of
the six had a history section, so each got a new one holding its dated step-6 line
(U1-exempt).

**R-A8 (`142.066-T`).** Scenario 2 now reads "the harness sends only `stats`,
`workspace-status`, and `_health`", and scenario 3 reads "`daemon-status` and
`workspace-status` are accepted". "status call 1/2" in the R7.7 rule and the
`get_workspace_status` mentions are unchanged. A dated R-A8 line was added to its
history section. Full re-check: the batch-2 checks (U1-U6, U8, U9, every 9.2 R7.7
and constant string) PASS, plus an R-A8 check (no backticked bare `status` and no
`--json status` in the checked text): PASS. Its `backlogit get` body equals the
Markdown body.

| Slot | Task | Depends on (= `dep list`) | Route | Size, est. | Labels | Check |
|---|---|---|---|---|---|---|
| 10 | `142.069-T` | `142.054-T` | (c) | S \| medium, 1.5 h | `pending-pa-1` removed | PASS |
| 11 | `142.070-T` | `142.069-T` | (c) | S \| medium, 1.5 h | `pending-pa-1` removed | PASS |
| 12 | `142.071-T` | `142.070-T`, `142.054-T` | (c) | XS \| medium, 1 h | `pending-pa-1` removed | PASS |
| 13 | `142.072-T` | `142.069-T` | (c) | S \| medium, 1.5 h (at the limit) | `pending-pa-1` removed | PASS |
| 14 | `142.073-T` | `142.072-T`, `142.071-T` | (c) | S \| medium, 1.5 h | `pending-pa-1` removed | PASS |
| 15 | `142.074-T` | `142.073-T` | docs | XS \| trivial, 0.5 h | `pending-pa-1` removed; `harness-verification-gated` added | PASS |

**Checks per task** (`tmp/asm/step6-check.ps1` through the batch-3 wrapper
`tmp/asm/b3/check3.ps1`, both gitignored; each task was first run against its
pre-rewrite text as a negative test, and 21-32 checks fired per task): U2, U3, U4
(the exact route (c) wording plus the task's `Worker: <id>` marker; for `142.074-T`
"Route: docs (plan section 4)" and `harness-verification-gated` in text and label),
U5, U6, U8, U9 (limit (a): 2.25 h; 1.5 h for Slot-12; 0.75 h for Slot-15), the U1
scan, a history-section check, and the 9.2 strings:

* `142.069-T`: one crate-local test file under `crates/engram-indexer/tests/` using
  only public `engram_indexer::preflight` and `::preflight_verdict` paths; no
  `[[test]]` stanza, no `#[path]` include, no `allow`/`expect`/`warn` (A6-A8);
  `cargo test -p engram-indexer --test <recorded name>`; the A20 test and clippy
  gate; the line "- Scenario 3: `stage_name` is exhaustive and equals the seven F50
  `Failure` variants." Negative: no F52, F53, `PreflightStage` or old target name.
* `142.070-T`: the same crate-local public-API form with the three public paths and
  no lint attribute (A9-A10); A20 gate.
* `142.071-T`: `harness_cmd` `cargo test -p engram-indexer --test
  preflight_entrypoint_test` with its expected count, and any other count, zero
  included, is a HALT (LD3); A20 gate. Its placeholder is a `preflight` branch in
  `main`; scenario 3 (legacy path) is recorded as characterization, with
  `R12-ROUTE-C-HALT ... F3` as the fallback.
* `142.072-T`: `pub const RELAY_STAGES: [&str; 7]` holding the seven full `Failed`
  lines; `normalize_verdict` returns the element it equals and uses no other list
  (A21, A22 rev. R14); the crate-local stage-set test proves two-way set equality
  against the lines built from `stage_name(f)` (A11 rev. R14); the guard is a `match`
  with no wildcard and distinct arm values (variant index 0-6 or `stage_name(f)`), or
  a single or-pattern arm, so `clippy::match_same_arms` is clean; A12/A13 commands;
  A20 test and clippy. Negative: no `#[allow`, no frozen/freeze/`R12.4` text, no
  `#[path]`-include clause.
* `142.073-T`: `Cli::command().debug_assert()` in the `#[cfg(test)]` module of
  `src/bin/engram.rs` (existing `mod tests`, with `use clap::CommandFactory;`), not
  under `tests/contract`, run by `cargo test --bin engram`; `unit_cli_parser`,
  `integration_cli_e2e` and `contract_cli_tool_catalog_parity` kept (M13); A20 final
  test and clippy. Negative: no `F54`, `read_server_cli_mcp_parity` or "per-case map".
* `142.074-T`: docs-only, only `docs/cli-mcp-parity.md`; a line reading exactly
  "Downstream: none". Negative: no "Superseded by 142-F" string and no compound-doc
  path in the checked text.

No `R16-CONTENT-CHECK`, no `R16-STALE-TEXT`. Rollup guard after every write: `142-F`
`active` and each task `queued` in the index and the Markdown; no
`R15-ROLLUP-DRIFT`. `backlogit get` body equals the Markdown body exactly
(normalized for CR and trim) for all six and for `142.066-T`.

**Removed U1 text:** "OUTSIDE shipment 142-S; requires PA-1" (all six); the
unrecorded-edge and PA-1 notes (069 line 46, 071 line 45, 073 line 46); A22's
"frozen under plan R12.4" sentence (not carried into 072). "Owned files" became
"Production files edited" or "file edited" (U6). Other removed text: the root-test
`#[path]` harness lines (069, 070, 072), `contract_read_server_cli_mcp_parity` and
"per-case maps" (073, M13), and 074's compound-note scope line and AC (now
`142.059-T`'s, plan 9.2).

**Judgment calls (for operator review at PR):**

* `142.074-T` got the label `harness-verification-gated` (plan section 4: docs-route
  tasks are `harness-verification-gated`; `142.059-T` already has that label). Its
  title still says "... and supersession note", and `references` still lists the
  compound doc. Both are frontmatter, which step 6 doesn't rewrite; the body says
  the note belongs to `142.059-T`.
* Route (c) placeholders for 071 and 073 (R13.6 had no rows for them) are written as
  a marker-bearing placeholder dispatch, with scenarios that pass at Step 2 recorded
  as characterization and `R12-ROUTE-C-HALT <task> <tests> F3` as the fallback.
* A20's "recorded pending RED" allowance was dropped from all five code tasks (U3).

**Notes for batch 4 (`142.055-T`, `142.056-T`, `142.057-T`, `142.063-T`):**

* Each of `142.055-T` to `142.057-T` needs the U7 STATUS-RESET line (A16 form) in
  its history section; see `142.054-T` (batch 2) and `tmp/asm/b2/check2.ps1` for the
  regex. `PA-6-LIFTED` is step 8's, not step 6's (R-A4).
* "Depends on" must list archived edges: `142.056-T` → `142.055-T`, `142.001-T`;
  `142.057-T` → `142.054-T`, `142.073-T`, `142.001-T`; `142.055-T` → `142.054-T`,
  `142.073-T`; `142.063-T` → `142.062-T`, `142.066-T`, `142.060-T`.
* `142.063-T` alone keeps the `F54 settle:` contract string (R-A4), but its R9.6 "S4"
  wording must become slot terms (Slot-19). The checker's `F54 settle` regex will
  fire on it, so batch 4's wrapper must exempt that string for `142.063-T` only.
* `pending-pa-1` is still on `142.063-T` only (`142.055-T` to `142.057-T` don't have it).
* `142.056-T` (OD-3 B) needs the placeholder marker `Worker: 142.056-T` and HALT
  `R17-F52-NO-PLACEHOLDER`.
* After batch 4, re-check `142.059-T` and the 058 family only in PA5-P (R-A7), not here.
  (Corrected in batch 4: R-A7 gives `142.059-T` its full section 9 content at
  assembly; only `142.058-T` is limited, and the 058a-c family doesn't exist yet.)

### Step 6, content, batch 4 of 4 (Slot-16 to Slot-21, R-A7 limited 058), 2026-10-06: PASS

Run on `chore/stage-142-f-assembly` from HEAD `d738dd26`, backlogit CLI only, no
`backlogit sync` of any kind. Bodies of `142.055-T`, `142.056-T`, `142.057-T`,
`142.063-T` and `142.059-T` were rewritten from scratch drafts spliced in after the
frontmatter (`tmp/asm/b4/*.body.md`, `tmp/asm/b4/splice.py`, gitignored).
`142.058-T` got three edit-tool changes only (R-A7). `142.063-T`: `pending-pa-1`
removed with `backlogit update 142.063-T --labels preflight`, and its title set to
R9.6's "PRE-3F: Add shared activation-settle barrier and positive witness target"
with `backlogit update --title` (both change only that field and `updated_at`).
None of the six had a history section except `142.063-T`; each now holds its
dated step-6 line (U1-exempt), and 055-059 also hold the U7 STATUS-RESET line.

| Slot | Task | Depends on (= `dep list`) | Route | Size, est. | U7 | Check |
|---|---|---|---|---|---|---|
| 16 | `142.055-T` | `142.054-T`, `142.073-T` | (a-1) | S \| medium, 1.5 h | STATUS-RESET (`1dfc1b5b`, `4995d681`) | PASS |
| 17 | `142.056-T` | `142.055-T`, `142.001-T` (archived) | (c), own placeholder (OD-3 B) | M \| medium, 2 h | STATUS-RESET (`5760b948`) | PASS |
| 18 | `142.057-T` | `142.054-T`, `142.073-T`, `142.001-T` (archived) | (a-1) | S \| low, 1.5 h | STATUS-RESET (`e24f5ae2`) | PASS |
| 19 | `142.063-T` | `142.062-T`, `142.066-T` (E2), `142.060-T` (E8) | (c) | S \| medium, 2 h | not in U7's range | PASS |
| (20d) | `142.058-T` | unchanged (17 edges incl. 066, 065 and archived) | — (PA5-P) | — (PA5-P) | STATUS-RESET (`7bd9e504`, `6d216d19`; subtasks `.002`, `.003-ST`) | LIMITED PASS (U1 + U7) |
| 21 | `142.059-T` | `142.058-T`, `142.055-T`, `142.057-T`, `142.025-T`, `142.026-T` (last two archived) | docs | XS \| trivial, 0.5 h | STATUS-RESET (none; no parked commit) | PASS |

**Checks per task** (`tmp/asm/step6-check.ps1` through the batch-4 wrapper
`tmp/asm/b4/check4.ps1`, both gitignored; each task was first run against its
pre-rewrite text as a negative test, and 4-37 checks fired per task): U2 (Shipment
line, every "Depends on" line equal to `dep list`, 4.2 size and estimate), U3, U4
(exact route (a-1), (c) or docs wording), U5, U6 (R12.3 line, no "Owned files"),
U8, U9 (SB-1 quoted; limit (a) 2.25 h for 16 and 18, 3 h for 17 and 19, 0.75 h for
21), U7 STATUS-RESET in A16 form (055-059, same regex as `142.054-T`), the U1 scan,
a history-section check, and the 9.2 strings:

* `142.055-T`: "This task owns the fail-open assertion rewrite in
  `tests/contract/start_launcher_test.rs`" (W9 AC transfer, R11.6 P2-3, as an AC
  too); fail-closed launcher scope only; route (a-1); parked `1dfc1b5b`,
  `4995d681` reusable as its harness records.
* `142.056-T`: one named placeholder with marker `Worker: 142.056-T` in `start.ps1`
  or the test-support seam, placed by the harness-architect at Step 2; the matrix
  (seven-stage table, spaced paths, unowned descendant) fails first against it;
  build-feature replaces it; a Step 2 pass is characterization, not RED; HALT
  `R17-F52-NO-PLACEHOLDER`; parked `5760b948` reference only; no transferred
  fail-open criterion. Negative: the old AC4 and the `start_launcher_test.rs`
  line-80/154 claim are gone.
* `142.057-T`: F53 scope only; parked `e24f5ae2`.
* `142.063-T`: the wholesale R9.6 text (description, AC1-AC8, notes, verification
  1-3), own target `integration_read_server_activation_settle` (PA-7), no F54 file
  edit, F54 barrier evidence from its own IC target only. R-A4: the two
  `F54 settle:` contract strings (AC6, AC7) are the only `F54 settle` text, and the
  wrapper exempts exactly those; "first code task of S4" became "Placement:
  Slot-19 (one-task shipment)", "S4 base" became "the Slot-19 branch base".
  Negative: no "first code task", group base, ownership exception, per-row
  signature or `ensure_daemon` text; the title equals R9.6's.
* `142.059-T`: F55 docs plus the moved "Superseded by 142-F" note (cites this plan
  and `142.055-T`; guardrail 4 replaced by the fail-closed preflight; shared budget
  and exact-child cleanup retained); route docs, `harness-verification-gated`; the
  Shipment line reads Slot-21 and says PA5-P creates it.
* `142.058-T` (R-A7 limited): U1 scan clean, STATUS-RESET present, and the notes
  line "S-20 split, Slot-20 content and E3/E15 deferred to PA5-P (R-A7,
  2026-10-05)" present in the implementation-notes section. U2-U6, U8, U9 and the
  9.2 S-20 content are PA5-P's.

No `R16-CONTENT-CHECK`, no `R16-STALE-TEXT`. Rollup guard after every write:
`142-F` `active` and each task `queued` in the index and the Markdown; no
`R15-ROLLUP-DRIFT`. `backlogit get` body equals the Markdown body exactly
(normalized for CR and trim) for all six.

**Removed U1 text:** the "with PA-1" provenance clause on the PA-2 contract (055,
057); in `142.063-T` the whole old body (PA-1 gate and ownership exception, the
per-row signature and after-edit maps, the pending-RED mapping, S-group wording,
"OUTSIDE shipment 142-S"); in `142.058-T` "sequenced after 142-S" (now "sequenced
after 142-F"). "Owned files" became "Files edited" (055, 056, 057, 059, 063).

**Judgment calls (for operator review at PR):**

* `142.063-T`: AC2's lint sentence reads as amended by R10.3 P2-5 (only `allow`,
  `expect` and `warn` banned), and the notes list the R10.1/R10.3/R11 amendments
  that R9.3 is read with; the R10.1 IC pending-RED allowance is stated as not
  applying (U3). The size stays the 4.2 "S, 2 h", not R9.6's "M". The R9.6 T1-T18
  disposition table stays in the old plan. R9.6's `references` addition (the
  escalation review) is cited in the description instead, because the CLI has no
  references flag.
* `142.056-T`: "Downstream: none" (no task depends on it); the old note "reversed in
  `contract_start_launcher`" is kept, re-attributed to F51's `1dfc1b5b` (R11.9 step 5).
* STATUS-RESET fills: 055-057 and 059 have no subtasks ("Subtasks none ..."); 059's
  commit list is "none: this task has no parked commit, plan 7.3"; 058's base tree
  is "Slot-20d" (its 4.3 slot; the split is PA5-P's).

### Step 6 close-out (all step-6 tasks), 2026-10-06: PASS

`tmp/asm/b4/closeout.ps1` re-ran the step-6 checks over every step-6 task in the
working tree after batch 4: batch 1 through a new wrapper `tmp/asm/b4/check1.ps1`
(the batch-1 requirement set: U2-U6, U8, U9 and the 060/076/077/064 9.2 strings),
batch 2 through `check2.ps1` (with `142.054-T`'s U7), batch 3 through `check3.ps1`
plus the R-A8 check on `142.066-T`, and batch 4 through `check4.ps1`
(`142.058-T` in R-A7 limited mode). Result: **26 of 26 PASS**, with no
`R16-CONTENT-CHECK`, `R16-STALE-TEXT` or `R15-ROLLUP-DRIFT`, and no `pending-pa-1`
label on any `142.0xx-T`. The range `142.054-T` to `142.079-T` holds 26 tasks (the
brief said 27; there is no 27th step-6 task). U7's STATUS-RESET half is present on
`142.054-T` to `142.059-T`; the `PA-6-LIFTED` half is step 8's (R-A4).

### Step 7, part A: slot shipments Slot-01 to Slot-08 (2026-10-06)

The shipments were created with the CLI (`backlogit shipment create --items <task> --priority high`, 1.11.0, with no MCP)
in `queue_position` order. Each description (slot, predecessors from `backlogit dep list`, 4.2 estimate, and the 7.3 T0
location) was written with `backlogit update <S> --section description=...`. `custom_fields.queue_position` was then
hand-written into each Markdown file under `items`. No create was refused, so `R15-SHIPMENT-SHAPE` was not raised. No
shipment edges were added (that's part C), and no `backlogit sync` was run.

| Slot | Shipment | Task | queue_position |
|---|---|---|---|
| 01 | `143-S` | `142.060-T` | 10 |
| 02a | `144-S` | `142.075-T` | 20 |
| 02b | `145-S` | `142.061-T` | 21 |
| 03 | `146-S` | `142.062-T` | 30 |
| 04a | `147-S` | `142.076-T` | 40 |
| 04b | `148-S` | `142.077-T` | 41 |
| 04c | `149-S` | `142.064-T` | 42 |
| 05 | `150-S` | `142.065-T` | 50 |
| 06 | `151-S` | `142.066-T` | 60 |
| 07 | `152-S` | `142.067-T` | 70 |
| 08 | `153-S` | `142.068-T` | 80 |

Verification: for all 11, `backlogit shipment get` shows `queued` with exactly one member, the expected task. The rollup
guard passed: `142-F` is still `active` and all 11 tasks are still `queued`. The task Markdown was unchanged.

The task edges, read before creation, match the planned edges. Slot-01's T0 location is
`parked/142-s-split-0f2cca20:tests/integration/release_archive_smoke_workflow_test.rs` (`a47b8aff`, reference only).

Notes for part B:

* Part B starts at Slot-09a (`142.078-T`, 90), and the next shipment ID should be `154-S`.
* `queue_position` lives only in the Markdown. The index won't have it until run point 1.

### Step 7, part B: slot shipments Slot-09a to Slot-19 (2026-10-06)

Part B used the same method as part A: CLI `backlogit shipment create --items <task> --priority high` in `queue_position`
order, then `backlogit update <S> --section description=...`, and `custom_fields.queue_position` hand-written under
`items`. No create was refused, so there was no `R15-SHIPMENT-SHAPE`. No shipment edges were added (part C), no
`backlogit sync` was run, and Slot-19.k, Slot-20 and Slot-21 were not created (PA5-P).

| Slot | Shipment | Task | queue_position |
|---|---|---|---|
| 09a | `154-S` | `142.078-T` | 90 |
| 09b | `155-S` | `142.079-T` | 91 |
| 09c | `156-S` | `142.054-T` | 92 |
| 10 | `157-S` | `142.069-T` | 100 |
| 11 | `158-S` | `142.070-T` | 110 |
| 12 | `159-S` | `142.071-T` | 120 |
| 13 | `160-S` | `142.072-T` | 130 |
| 14 | `161-S` | `142.073-T` | 140 |
| 15 | `162-S` | `142.074-T` | 150 |
| 16 | `163-S` | `142.055-T` | 160 |
| 17 | `164-S` | `142.056-T` | 170 |
| 18 | `165-S` | `142.057-T` | 180 |
| 19 | `166-S` | `142.063-T` | 190 |

Verification: for all 13, `backlogit shipment get` shows `queued` with exactly one member, the expected task. The rollup
guard passed: `142-F` is still `active` and all 13 tasks are still `queued`. The task Markdown was unchanged.

The predecessors in each description come from `backlogit dep list`, read before creation, and match the section 4
table, including the archived `done` edges on `142.054-T` (12) and on `142.056-T` and `142.057-T` (`142.001-T`). The T0
locations follow 7.3 and each task's own wording: 09a-09c `c269fa79` and `41dd5081` (reference only:
`crates/engram-indexer/src/{lib,preflight}.rs` and `tests/integration/preflight_gate_test.rs`); 16 `1dfc1b5b` and
`4995d681` (reusable, `tests/contract/start_launcher_test.rs`); 17 `5760b948` (reference only,
`tests/contract/start_launcher_failure_test.rs`); 18 `e24f5ae2` (reusable, `tests/contract/start_sh_launcher_test.rs`).
All paths are on `parked/142-s-split-0f2cca20`.

Notes for part C:

* All 24 slot shipments Slot-01 to Slot-19 exist: `143-S` to `166-S`. Part C mirrors the task edges as 5.2 shipment
  edges, using the two maps above.
* The 4.2 sizing table gives `142.071-T` "XS, 1 h", which is above the SG-1 XS bound of 0.5 h. The description copies
  the plan verbatim. The 1 h is still within 2 h, so the verdict stays OK.

### Step 7, part C: mirrored shipment edges among Slot-01 to Slot-19 (2026-10-06): PASS

Following plan 5.2, Stage read `backlogit dep list` on each of the 24 slot tasks. It kept every `blocks` edge X → Y
where both ends are slot tasks and dropped the edges to archived tasks (`142.001-T`, `142.020-T` to `142.053-T` on
`142.054-T`, and `142.001-T` on `142.056-T` and `142.057-T`). No slot task has an edge to `142.058-T`, `142.059-T` or a
PA-5 task. That gives **31 edges**, the planned count. Each edge was added with the 1.11.0 CLI as `backlogit dep add
<ship(X)> <ship(Y)> --type blocks` (`dep add <item-id> <depends-on>`, routed to AddShipmentBlock) and checked with
`backlogit dep list <ship(X)>` straight after. All 31 checks passed. Before the run, no slot shipment had any edge in
either direction.

| Task edge | Shipment edge | Task edge | Shipment edge |
|---|---|---|---|
| 061 → 075 | `145-S` → `144-S` | 071 → 070 | `159-S` → `158-S` |
| 062 → 061 | `146-S` → `145-S` | 071 → 054 | `159-S` → `156-S` |
| 076 → 062 | `147-S` → `146-S` | 072 → 069 | `160-S` → `157-S` |
| 077 → 076 | `148-S` → `147-S` | 073 → 072 | `161-S` → `160-S` |
| 064 → 077 | `149-S` → `148-S` | 073 → 071 | `161-S` → `159-S` |
| 065 → 062 | `150-S` → `146-S` | 074 → 073 | `162-S` → `161-S` |
| 065 → 064 | `150-S` → `149-S` | 055 → 054 | `163-S` → `156-S` |
| 066 → 064 | `151-S` → `149-S` | 055 → 073 | `163-S` → `161-S` |
| 066 → 065 | `151-S` → `150-S` | 056 → 055 | `164-S` → `163-S` |
| 067 → 066 | `152-S` → `151-S` | 057 → 054 | `165-S` → `156-S` |
| 068 → 067 | `153-S` → `152-S` | 057 → 073 | `165-S` → `161-S` |
| 078 → 068 | `154-S` → `153-S` | 063 → 062 | `166-S` → `146-S` |
| 079 → 078 | `155-S` → `154-S` | 063 → 066 | `166-S` → `151-S` |
| 054 → 079 | `156-S` → `155-S` | 063 → 060 | `166-S` → `143-S` |
| 054 → 068 | `156-S` → `153-S` | | |
| 069 → 054 | `157-S` → `156-S` | | |
| 070 → 069 | `158-S` → `157-S` | | |

The arrow means "depends on" (X is blocked by Y), as in `dep list`. Task numbers abbreviate `142.0NN-T`.

* **Cycle check: PASS.** Stage re-read all shipment edges for `143-S` to `166-S` with `dep list` and got 31 `blocks`
  edges, every one inside the set. A Kahn topological sort reached all 24 of 24 nodes, so there's no
  `R15-DAG-CYCLE`. One valid order: 143 144 145 146 147 148 149 150 151 152 166 153 154 155 156 157 158 160 159 161 162
  163 165 164.
* **Files.** 22 shipment files changed (`145-S` to `166-S`; `143-S` and `144-S` have no predecessors), 53 lines added:
  22 `dependencies:` keys and 31 entries. Nothing else changed; `custom_fields.queue_position` is still present in all 24.
* **No sync was run**, step 8 and later weren't touched, and nothing was pushed. Each `dep add` took about 30 s on this
  workspace, which is slow but didn't cause any errors.
* Edges to Slot-19.k to Slot-21 (058, 059 and the PA-5 tasks) are added at PA5-P, once those slots exist.

### Step 8, lift comments (2026-10-06): PASS

Stage appended one history line starting `PA-6-LIFTED 2026-10-04 (OD-6)` to the
Markdown history section of `142.055-T`, `142.056-T`, `142.057-T`, `142.060-T`,
`142.063-T`, `142.058-T` and `142.059-T`. Each line names the task's slot
(16, 17, 18, 01, 19, 20d and 21) and plan 5.4, and says a later re-hold would be a
separate `PA-6-REHOLD <reason>` line. The edits were made with the edit tool, one
line before `<!-- END:history -->` in each file. Existing history lines were kept
as written. Neither the Markdown nor the backlogit logs held any earlier hold
comment for these seven tasks, so nothing else needed keeping. No `backlogit sync`
was run.

* **R-A4 U7 check: PASS.** All seven contain `PA-6-LIFTED 2026-10-04 (OD-6)`, and
  `142.054-T` to `142.059-T` all still contain `STATUS-RESET`. No
  `R16-CONTENT-CHECK <task> U7` was raised.
* **Rollup guard: PASS.** `142-F` is `active` in both `backlogit get` and the
  Markdown. All seven tasks are `queued` in both, and all 26 `142.0xx-T` queue
  files read `queued`.
* **`get` against the Markdown: MATCH.** For the seven tasks and `142-F`,
  `backlogit get --json` gives the same status as the frontmatter and the same body
  as the Markdown, including the new line.
* **Diff:** 7 queue files, 1 line added each.

### Step 9, Constitution Check (2026-10-06): PASS

* **Target.** The old plan is the decomposition plan's `old_plan` frontmatter
  value, `docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md`. It
  has one `## Constitution Check` section (line 6296). The brief's guess,
  `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`, is a
  different plan and was not touched. The target is unambiguous, so there was no
  `R16-CONTENT-CHECK step9 target` halt.
* **Form.** Step 9 says the section is "replaced by section 13", and section 13
  says it "replaces the old plan's section". The decomposition plan is the single
  current statement (section 1), so Stage replaced the old table with a pointer
  rather than a second copy of section 13. The heading stays, and the 16-line table
  (principle rows plus the Revision 6 to 9 rows) became two short paragraphs. They
  name section 13 of the decomposition plan as the current Constitution Check and
  give `09e436be` (on `main`) as the git location of the old table.
* Section 13's heading says "at step 8", but section 8 numbers this step 9.
  Stage applied the replacement at step 9 as the brief directed. The plan is frozen,
  so the heading wasn't changed.
* **Diff:** 7 lines added and 16 removed, in the old plan only. CRLF line endings
  and the final newline were kept.

### Step 10, verify shipments and members (CG-Q), 2026-10-06: PASS

Run on `chore/stage-142-f-assembly` from HEAD `c7ecd2f2`, backlogit CLI 1.11.0
only, with no MCP and no `backlogit sync` of any kind. The scratch script
`tmp/asm/s10/verify.py` (gitignored) did the following for each of the 24 slots,
against the slot map in "Handoff map" below:

* `backlogit shipment get <S>`: `status` is `queued`; `custom_fields.items` is
  exactly `[<task>]`; and `size_composition.members` has exactly one entry, that
  task, with `artifact_type: task`.
* `backlogit get <task> --json`: `artifact_type: task`, `status: queued`,
  `parent_id: 142-F`.
* The shipment Markdown frontmatter: `status: queued`, `custom_fields.items` is
  `[<task>]`, and `custom_fields.queue_position` equals the slot's planned value
  (10 to 190). The task Markdown reads `task` and `queued`.

**Result: 24 of 24 OK, so `R15-SHIPMENT-SHAPE` was not raised.** Every slot has
exactly one member, that member is the slot's mapped task and is `queued`, and
every `queue_position` matches. No other open queue shipment holds a `142.0xx`
task. `queue_position` is still in the Markdown only; the index gets it at run
point 1 (step 11). The step 7 part C edge count was re-read from the Markdown:
31 `dependencies` entries across `143-S` to `166-S`, every one inside the set.

Earlier notes for step 7 and later (all done):

* Step 7 created Slot-01 to Slot-19 only (24 shipments). `142.059-T` already reads
  "Shipment: Slot-21", but Slot-21 (and Slot-19.k, Slot-20a-20d) is created at PA5-P.
* Step 8's `PA-6-LIFTED 2026-10-04 (OD-6)` went into the Markdown history section
  (logs are gitignored) of `142.055-T`, `142.056-T`, `142.057-T`, `142.060-T`,
  `142.063-T`, `142.058-T` and `142.059-T`. The R-A4 U7 check passed after step 8.
* `142.063-T`'s `references` frontmatter still lacks the escalation review (cited in
  the body); `142.074-T`'s title and references are unchanged frontmatter (batch 3).

## Handoff map: slot → shipment → task (the handoff tokens for Ship)

This is the map to publish at step 11, once the merge and run point 1 are done.
Ship claims by shipment ID, in `queue_position` order, subject to the 31 mirrored
`blocks` edges (step 7 part C).

| Slot | Shipment | Task | Unit | queue_position |
|---|---|---|---|---|
| 01 | `143-S` | `142.060-T` | archive verifier | 10 |
| 02a | `144-S` | `142.075-T` | PRE-1a | 20 |
| 02b | `145-S` | `142.061-T` | PRE-1b | 21 |
| 03 | `146-S` | `142.062-T` | PRE-2 | 30 |
| 04a | `147-S` | `142.076-T` | PRE-3a | 40 |
| 04b | `148-S` | `142.077-T` | PRE-3b | 41 |
| 04c | `149-S` | `142.064-T` | PRE-3c | 42 |
| 05 | `150-S` | `142.065-T` | PRE-4b | 50 |
| 06 | `151-S` | `142.066-T` | PRE-4 | 60 |
| 07 | `152-S` | `142.067-T` | PRE-5 | 70 |
| 08 | `153-S` | `142.068-T` | PRE-6 | 80 |
| 09a | `154-S` | `142.078-T` | F50a | 90 |
| 09b | `155-S` | `142.079-T` | F50b | 91 |
| 09c | `156-S` | `142.054-T` | F50c | 92 |
| 10 | `157-S` | `142.069-T` | NEW-1 | 100 |
| 11 | `158-S` | `142.070-T` | NEW-2 | 110 |
| 12 | `159-S` | `142.071-T` | NEW-3 | 120 |
| 13 | `160-S` | `142.072-T` | NEW-4 | 130 |
| 14 | `161-S` | `142.073-T` | NEW-5 | 140 |
| 15 | `162-S` | `142.074-T` | NEW-6 | 150 |
| 16 | `163-S` | `142.055-T` | F51 | 160 |
| 17 | `164-S` | `142.056-T` | F52 | 170 |
| 18 | `165-S` | `142.057-T` | F53 | 180 |
| 19 | `166-S` | `142.063-T` | PRE-3F | 190 |
| 19.k | not created (PA5-P) | PA-5 tasks | PA-5 | 190 + k |
| 20a-20c | not created (PA5-P, R-A7) | `142.058a` to `c` (not created) | F54 split | per PA5-P |
| 20d | not created (PA5-P, R-A7) | `142.058-T` (queued, no shipment) | F54 | per PA5-P |
| 21 | not created (PA5-P) | `142.059-T` (queued, no shipment) | F55 | per PA5-P |

Nothing after Slot-19 can be claimed until the PA5-P PR merges, because no
shipment exists for it.

## PA5-P obligations carried from R-A7 (and plan 7.6)

PA5-P is Stage's PA-5 planning and harvest (plan 7.6). Its staging PR follows
this assembly merge and owes the following. Sources: R-A7 in
`docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md`, and
plan 7.6.

1. **S-20 split** per plan 4.3 under R-A6, measured from `6d216d19`: create
   `142.058a`, `142.058b` and `142.058c` (parent `142-F`) and rewrite
   `142.058-T` to its share. The 400-line bounds count authored and adapted lines
   only (R-A6).
2. **SG-1 and acceptance-map checks** for the 058 family, with the same HALT
   tokens as 4.3 (`R17-SIZE-GATE`, `R17-SPLIT-AC-UNMAPPED`).
3. **Edges E3 and the 142.058 chain of E15** with the 5.1 checks: `058a` → 063
   (E3), 066 and 065; `058b` → `058a`; `058c` → `058b`; `142.058-T` → `058c`.
   Then the E6 edges (`142.058a` → each PA-5 sink), which follow the split.
4. **Section 9 content**: the 9.2 content for `142.058a` to `c` and `142.058-T`,
   plus U2 to U9 for all four. Today `142.058-T` holds only U1, the U7
   STATUS-RESET line, `PA-6-LIFTED` and the R-A7 deferral line (step 6, limited
   mode).
5. **Shipments** Slot-19.k (one per PA-5 task, `queue_position` 190 + k),
   Slot-20a to Slot-20d and Slot-21 (`142.059-T`), each a one-task shipment with
   mirrored edges and `queue_position` (7.6). The PA-5 tasks are re-cut so that
   none edits the F54 file (R-2) and PA5-T4 is split (SG-1).
6. **No claim before merge**: no 142.058-family, `142.059-T` or PA-5 task can be
   claimed until the PA5-P PR merges.
7. **Ship at Slot-20a to Slot-20d** must read R-A6 and R-A7 alongside the task
   text (the "How executors use this" section of the R-A7 record).

## Planned state for the resumed run (reference only; superseded by the steps above)

The 058a-c lines below were deferred to PA5-P by R-A7 and were not applied. The
slot table below predates step 7; the real shipment IDs are in "Handoff map".

Edges after step 5, as planned. `142.058-T` keeps its existing 066 and 065 edges,
because 4.3 copies rather than moves:

* 061a: none. 061 → 061a. 062 → 061. 064a → 062 (R-A2: 062 only). 064b → 064a. 064 → 064b (E1 removes 063).
* 065 → 062, 064 (E10). 066 → 064, 065. 067 → 066. 068 → 067.
* 054a → 068. 054b → 054a. 054 → 054b, 068 (+ archived). 069 → 054. 070 → 069. 071 → 070, 054. 072 → 069. 073 → 072, 071. 074 → 073.
* 055 → 054, 073. 056 → 055 (+ archived). 057 → 054, 073 (+ archived). 063 → 062, 066 (E2), 060 (E8). 060: none (E11).
* 058a → 063 (E3), 066, 065. 058b → 058a. 058c → 058b. 058 → 058c, 066, 065 (+ archived). 059 → 055, 057, 058, 025, 026.
* Planned mirrored shipment edges among Slot-01 to Slot-19: 31.

Slot to `queue_position` (planned before step 7; the shipment IDs now exist, see "Handoff map"):

| Slot | Task | queue_position | Slot | Task | queue_position |
|---|---|---|---|---|---|
| 01 | `142.060-T` | 10 | 09c | `142.054-T` | 92 |
| 02a | `142.061a` (new) | 20 | 10 | `142.069-T` | 100 |
| 02b | `142.061-T` | 21 | 11 | `142.070-T` | 110 |
| 03 | `142.062-T` | 30 | 12 | `142.071-T` | 120 |
| 04a | `142.064a` (new) | 40 | 13 | `142.072-T` | 130 |
| 04b | `142.064b` (new) | 41 | 14 | `142.073-T` | 140 |
| 04c | `142.064-T` | 42 | 15 | `142.074-T` | 150 |
| 05 | `142.065-T` | 50 | 16 | `142.055-T` | 160 |
| 06 | `142.066-T` | 60 | 17 | `142.056-T` | 170 |
| 07 | `142.067-T` | 70 | 18 | `142.057-T` | 180 |
| 08 | `142.068-T` | 80 | 19 | `142.063-T` | 190 |
| 09a | `142.054a` (new) | 90 | | | |
| 09b | `142.054b` (new) | 91 | | | |

That makes 24 shipments.

Notes for step 6 when it resumes:

* backlogit logs are gitignored (`.gitignore` `logs/`), so a `comment add` never
  reaches `main`. U7's STATUS-RESET line (A16 form) and step 8's `PA-6-LIFTED`
  line therefore have to go in each task's Markdown history section to be durable
  and checkable.
* U1 requires removing the `pending-pa-1` label from `142.061-T` to `142.074-T`
  (T16).
* Section 4 says "Docs-route tasks are `harness-verification-gated`", which
  applies to `142.074-T`.
* `142.064-T` already has a 2026-10-04 history note marking its body as superseded.

## Stash

* **Created: none.** Every residual P2/P3 finding that plan section 18 routes to
  the landing staging PR is already in the active stash, so nothing was
  duplicated:
  * `4D864E76`: attempt-13 P2-2, P2-4, P2-5
  * `071E4754`: P2-6, P2-17
  * `7A0ABBCA`: P2-7 to P2-10
  * `89043CF8`: attempt-13 P3s
  * `48AC3309`: attempt-14 Scope P3-1 and the unaddressed P3 bullets
  * `F478484C`: A15-O1
* Also already stashed: `7BF90213`, `5AF5CD66`, `23E287C6` and `F99C705E`.
* `2D683B08` and `429B4886` (from H3) were not triaged; they're separate intake.

## Pre-PR self-review (2026-10-06): PASS, no open P0 or P1

Scope: `git diff origin/main...HEAD` (15 commits before this one, `30a7eae9` to
`c7ecd2f2`; 60 files), from the PS-6 resets through step 9. Scratch script
`tmp/asm/s10/selfreview.py` (gitignored); CLI only; no `backlogit sync`.

| Check | Result |
|---|---|
| Frontmatter parses for every touched `.backlogit` file | PASS: 55 Markdown files (YAML, with `id`, `artifact_type`, `status` and `title` present, and `id` matching the file name) and 1 checkpoint JSON. The three touched `docs/` files also parse. |
| No `.backlogit/.locks/` or `tmp/` content tracked | PASS: `git ls-files` holds neither, and neither appears in the diff. `.backlogit/.locks/` stays untracked and was never staged. |
| U1 stale-text scan (description, acceptance-criteria, implementation-notes and verification; history and comments exempt; the unmarked leading description of the older tasks included) | PASS for all 26 tasks `142.054-T` to `142.079-T`. R-A4 exemption applied to `142.063-T`: 3 `F54 settle:` contract strings, no other `F54 settle`. No `pending-pa-1` label. Two subtask hits, P3-2 below. |
| R-A7: no 058a, 058b or 058c task; no Slot-19.k, Slot-20 or Slot-21 shipment | PASS: no task or subtask title names 058a-c or F54a-c, no `142.08x-T` or `142.09x-T` exists, no shipment title names Slot-19.k, 20 or 21, and the highest shipment ID is `166-S`. |
| `143-S` to `166-S` read `status: queued` | PASS, 24 of 24 (Markdown, and `shipment get` at step 10). |
| `git diff --check` and commit trailers | PASS: no whitespace errors; all 15 commits carry both trailers. |
| Memory doc internal consistency | Findings P2-2 and P3-1, fixed in this commit. |

Findings:

* **P2-1 (fixed): stale active Stage checkpoint in the diff.**
  `.backlogit/checkpoints/checkpoint-20261006-012812.json` (added by `92c18f4f`,
  this session's ID) still read `status: active` with phase
  `asm-halted-step4-size-gate` and a `resume_hint` pointing at step 4. After the
  merge, every Stage start would have found it as a recovery candidate and gone to
  fail-closed operator selection on an out-of-date hint. Fix:
  `backlogit checkpoint resolve checkpoint-20261006-012812.json` (owner-scoped:
  same agent and session; no other checkpoint touched). It now reads
  `status: resolved`. No new checkpoint was written; this memory doc is the
  handoff.
* **P2-2 (fixed): memory doc out of date.** The frontmatter `status` and
  `progress`, the Outcome section ("Steps 4-10 did not run", "do not push"), the
  draft PR summary ("S-02/S-04/S-09/S-20 splits", "rulings R-A1 to R-A5"), the
  step-11 section ("wait until the HALT is resolved") and the planned slot table
  ("shipment IDs are not created") contradicted the completed steps and R-A7.
  All of these were updated, with the old text kept as history where it records
  a decision.
* **P3-1 (fixed):** the frontmatter `rulings` omitted R-A8. Added.
* **P3-2 (open, no action): U1 token in two subtask records.** The
  implementation notes of `142.054.002-ST` and `142.054.003-ST` read "PA-2b
  guidance: F50 completion after PRE-6 (operator-granted 2026-09-27 with PA-1;
  ...)". This is provenance, not PA-1 gating. The subtasks are records under
  `142.054-T` that Stage closes through 10.1 after Slot-09c (step 4 note), so
  they aren't Ship work and weren't in step 6's scope. Rewriting record text isn't
  a mechanical fix, so it was left as written.
* **P3-3 (open, no action): two body layouts.** The 21 pre-existing tasks keep
  the description as unmarked leading body text, while the 5 new tasks
  (`142.075-T` to `142.079-T`) use `<!-- BEGIN:description -->` markers. backlogit
  reads both: the get-vs-Markdown checks at steps 2, 6 and 8 matched. This is
  cosmetic.

Severity counts: P0 0, P1 0, P2 2 (2 fixed), P3 3 (1 fixed, 2 open with no
action needed).

## Commits on `chore/stage-142-f-assembly` (local only; not pushed)

1. `chore(backlog): reset 142-F former 142-S members to queued (PS-6, R-A5)` (`30a7eae9`).
   It contains the 11 queue files.
2. `docs(memory): record 142-F assembly halt at step 4 (R17-SIZE-GATE)` (`92c18f4f`).
   It contains this file and the Stage checkpoint `checkpoint-20261006-012812.json`.
3. `docs(adrs): record 142-F rulings R-A6 and R-A7 for S-20` and
   `docs(docs): log Stage subagent circuit break at 142-F assembly resume`
   (Orchestrator commits, up to `e7832c6c`).
4. `chore(backlog): split 142-F tasks S-02, S-04 and S-09 (assembly step 4, R-A7)`.
   It contains the 5 new queue files, the 3 rewritten parents and this file.
5. `chore(backlog): wire 142-F task edges (assembly step 5, R-A2, R-A7)`.
   It contains the 10 queue files with changed dependencies and this file.
6. `chore(backlog): write section 9 content for Slot-01 to Slot-04c (assembly step 6, batch 1)`.
   It contains the 7 batch-1 queue files and this file.
7. `chore(backlog): write section 9 content for Slot-05 to Slot-09c (assembly step 6, batch 2)`.
   It contains the 7 batch-2 queue files and this file.
8. `docs(adrs): record 142-F ruling R-A8 for 142.066-T CLI name` (Orchestrator, `6818c6e9`).
9. `chore(backlog): write section 9 content for Slot-10 to Slot-15 and apply R-A8 (assembly step 6, batch 3)`.
   It contains `142.066-T`, the 6 batch-3 queue files and this file.
10. `chore(backlog): write section 9 content for Slot-16 to Slot-21 (assembly step 6, batch 4)`.
   It contains the 6 batch-4 queue files and this file; step 6 is complete.
11. Step 7, parts A to C (`b30ec3b8`, `27b817ea`, `1e5702e2`): the 24 slot shipments
   and their 31 mirrored edges.
12. `chore(backlog): append PA-6 lift lines and replace old Constitution Check (assembly steps 8-9)` (`c7ecd2f2`).
   It contains the 7 step-8 queue files, the old plan and this file.
13. `docs(memory): finalize 142-F assembly record (step 10 and self-review)`.
   It contains this file and the resolved Stage checkpoint (P2-1). Step 10 itself
   changed no backlog file.

## Draft PR (final; the Orchestrator pushes and opens it)

Title: `chore(backlog): assemble 142-F one-task slot shipments (Slot-01 to Slot-19)`

```markdown
## Summary
142-F assembly (decomposition plan Revision 18, section 8 steps 1-10, rulings R-A1 to R-A8):
PS-6 resets (with R-A5 subtasks); S-02/S-04/S-09 splits into 142.075-T to 142.079-T
(the S-20 split is deferred to PA5-P by R-A7); edges E1-E15 less E3 and the 058 chain
(R-A2, R-A7); section 9 content for 26 tasks (R-A3, R-A4, R-A8; 142.058-T in R-A7
limited mode); 24 one-task slot shipments 143-S to 166-S (Slot-01 to Slot-19) with 31
mirrored blocks edges and queue_position; PA-6-LIFTED lines; the old Constitution Check
replaced by a pointer to section 13. Step 10 (CG-Q) 24/24. Stage memory:
docs/memory/2026-10-05-stage-142-f-asm-memory.md.

## Post-merge (plan 8 step 11)
Cache rebuild run point 1 (section 11, careful mode), the 5.2 queue-order check,
step 10 again under CG-S; the slot-to-shipment-ID map in Stage memory is the handoff.

## Local Review Readiness
- Reviewed HEAD: c7ecd2f2 (plus the memory finalization commit)
- Outcome: PASS (P0 0, P1 0, P2 2 fixed, P3 3: 1 fixed, 2 no action)
- Blocking findings: none
- Full local build: not applicable — backlog/docs-only
- Follow-ups: PA5-P staging PR (R-A7 obligations in Stage memory)
```

## Post-merge step-11 actions still owed

All of these wait until the assembly PR merges (CG-M: Stage commits only, the
operator merges). Run them on fetched, clean `main`:

1. Cache rebuild run point 1 (section 11, careful mode). Never a plain
   `backlogit sync`.
2. The 5.2 queue-order check (`backlogit queue view --type shipment --status
   queued`): the order must follow `queue_position` 10 to 190 as in "Handoff map".
   A mismatch raises `R16-QUEUE-POSITION`.
3. Step 10 again under CG-S: each of `143-S` to `166-S` has one member, a task,
   `queued`, matching the map. Anything else raises `R15-SHIPMENT-SHAPE <slot>
   <found>`.
4. Publish the slot-to-shipment-ID map ("Handoff map") in Stage memory as the
   handoff tokens for Ship.

Then PA5-P (plan 7.6), with the R-A7 obligations listed above.

## Next step

**Final, 2026-10-06:** section 8 steps 1-10 and the pre-PR self-review are done.
The Orchestrator pushes `chore/stage-142-f-assembly` and opens the PR; the
operator merges it. After the merge, a Stage session runs the step-11 actions
above on fetched, clean `main`, then PA5-P. Keep this worktree off `main` until
the merge, so that no `backlogit sync` can union the queued resets with the
`active` statuses still on `main` (CG-S caution, step 3).

### Next-step history (superseded)

The R17-SIZE-GATE halt is resolved (R-A6, R-A7), and steps 4 and 5 are done
(see "Step 4, resumed" and "Step 5, edges" above). The next Stage invocation
runs section 8 step 6 (section 9 content) on this branch, using the real IDs
above. Do not push or open the PR until steps 6-10 are done.

Update 2026-10-06: step 6 batches 1-3 are done (batch 3 also applied R-A8 to
`142.066-T`). The next Stage invocation runs step 6 batch 4 (`142.055-T`,
`142.056-T`, `142.057-T`, `142.063-T`).

Update 2026-10-06 (later): step 6 batch 4 is done (`142.055-T` to `142.057-T`,
`142.063-T`, `142.059-T`, and `142.058-T` in R-A7 limited mode), and the step-6
close-out passed 26 of 26. The next Stage invocation runs section 8 step 7
(shipments Slot-01 to Slot-19). Do not push or open the PR until steps 7-10 are done.

Update 2026-10-06 (steps 8 and 9): step 7 is done, and steps 8 and 9 passed (see
above). The next Stage invocation runs section 8 step 10 (verify each slot
shipment and member, CG-Q). Do not push or open the PR until step 10 is done.

### Original next step (2026-10-05, superseded)

The operator rules on `R17-SIZE-GATE 142.058a FL-share` (option A, B or C, or
another choice). Then a Stage session resumes on this branch at section 8 step 4,
first re-checking that the 11 resets still read `queued` in both the index and
the Markdown.
