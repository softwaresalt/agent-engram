---
title: "142-F F50 facade prerequisites and launcher-callable preflight command (supervisor entrypoint + engram relay)"
description: "Adds the missing process-boundary interface so start.ps1/start.sh can drive the F50 typed preflight machine through target\\debug\\engram.exe"
source: "docs/decisions/2026-09-26-142-f-launcher-preflight-command-deliberation.md"
parent_plan: "docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md"
feature: "142-F"
shipment_context: "142-S (active; operator-approved to be abandoned and its tasks re-queued in a later session); remaining scope split into queued shipments S1-S4 per Revision 6 (created in a later session); S3 held under PA-6 (Revision 8)"
stash_source: "03AA00A8, 49809128, 9B7EC1E4, 6C5DF765"
follow_up_stash: "86F93068 (blocks F54 GREEN), 5AF5CD66, 23E287C6, F99C705E, 7BF90213"
related_stash: "EFE9190A (overlaps PRE-4 and 86F93068), 4628001C (overlaps 86F93068); see Revision 5 duplicate-scan record"
revision: 15
consolidated_into: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md (Revision 16 is authoritative: one task per shipment in slots Slot-01 to Slot-21, task and shipment blocks edges plus queue_position, 142-S disposition through gated PRs, final per-task content requirements; authoritative for decomposition, landing, assembly, claim gates, closure and cache rebuild)"
---

## Problem Frame

F51 (`142.055-T`) and F53 (`142.057-T`) must launch Copilot only when the F50
machine (`crates/engram-indexer/src/preflight.rs`) reaches `Succeeded`. F50 is
only a library typestate, and no process-level command exposes it:

* `engram.exe preflight` is an unrecognized subcommand.
* `engram-indexer`'s `main.rs` handles env-driven indexing only.

Three structural facts constrain the design:

* The root `engram` package cannot link `engram-indexer`, because that would
  create a package cycle.
* `supervisor_workspace_boundary_test` forbids an indexer bin in the root
  package.
* `cargo dev-test` and root `cargo clippy` cover root-package targets only.

This plan restores the P34 "Shared preflight command", which was lost in the
F-roster renumbering. It uses the narrowest interface the F51/F52/F53 fixtures
already imply.

The command is a **dev-layout launcher command**. The agent archive and the
installer exclude `engram-indexer` (`supervisor_release_artifact_test`,
`supervisor_install_exclusion_test`). `engram preflight` is therefore hidden
from top-level help and documented as dev-launcher-only.

### Revision 3: F50 cannot complete yet (stash `49809128`)

Ship's `2026-09-26-ship-142-s-f50-concrete-verifier-api-block.md` shows the
D3 premise is false: F50 cannot finish `142.054.002-ST` and `003-ST` in its
two owned files. Stage checked this against HEAD `4995d681` (deliberation
Amendment 1, D4) and found four gaps:

* **G1: no build-into-candidate path.**
  * F10's `index_sealed_target` uses a Candidate as the source DISCOVERY
    root, and writes the database to `data_dir/cozo/<branch>/engram.db`.
  * Activation requires `<generation-id>/engram.db`.
  * The parent plan's P11 ("Build and seal a candidate generation") and P12
    ("Publish from the supervisor") were lost in the F-roster renumbering,
    exactly like P34.
* **G2: the supervisor crate cannot compose seal or publish.**
  * `engram-indexer` depends only on `engram` and `tokio`, so it has no
    `sha2`, `chrono`, or `serde_json`.
  * The current-revision reader is private to `publish.rs`.
* **G3 (root blocker, beyond Ship's report): production never serves a
  published generation.** `ReadServerStartupGate`, `admit_read`, and
  background reconciliation exist and are tested only in isolation:
  * In `src/`, no code constructs a `GenerationActivator` or a
    `ReadServerStartupGate`, or calls `set_generation_activator`.
  * `lifecycle_policy::run_read_server_startup` publishes a binding "without
    managed hydration".
  * `request_entry::process_request` calls `tools::dispatch`, which passes
    `None` as the read context (`tools/mod.rs:360`).

  So neither a real daemon nor a CLI or MCP read can report
  `/provenance/generation_id`. This is exactly F54's RED
  `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read`
  ("missing generation provenance"). F54 (`142.058-T`) owns only its test
  file, so it cannot go GREEN without this wiring either.
* **G4: the probes have no parser.** The probes are feasible at the process
  level: the F54 harness already drives `engram daemon`, `engram <cmd> --json`,
  and `engram shim` through `CARGO_BIN_EXE_engram`, and `cli::runner`
  auto-spawns the daemon. But the supervisor crate cannot parse JSON.

Revision 3 therefore adds six prerequisite units, PRE-1 through PRE-6, in
the root `engram` library and daemon. It keeps F50's own completion inside
`preflight.rs` and `preflight_gate_test.rs` as a pure composition of public
`engram::` facades plus `std` and `tokio`. That composition compiles both
under the `#[path]` include and in `engram-indexer`. There are no manifest
changes to `engram-indexer`. F50's six pedantic lints in `preflight.rs`
(for example `Result<(), ()>`) remain F50's in-scope work under the C3
symmetric guard.

### Revision 4: attempt-2 findings resolved (deliberation Amendment 2, D5)

Revision 4 carries a Stage-proposed closure for each of the seven attempt-2
P1s, numbered as in the "Attempt 2: FAIL" record below. The attempt-2 FAIL
verdict stands as history. None of these closures counts as accepted until a
later plan review passes:

| Attempt-2 P1 | Closed by (revision 4) |
|---|---|
| P1-1 pipe EOF hang | "Bounded child I/O" below; PRE-5, PRE-6, NEW-4 |
| P1-2 `serde_json::Value` in a public signature | "No `serde_json`" below; `src/preflight_probe.rs` |
| P1-3 identity and layout | "Git-fixture layout" and its correction below; PRE-2, PRE-3 |
| P1-4 G3 lineage | "G3 lineage" below; deliberation D5, PA-1 |
| P1-5 PRE-3 regresses consumers | "`_health` unchanged" below; PRE-3 "Consumer baseline" |
| P1-6 PRE-4 only labels provenance | "PRE-4b" below; PRE-4b unit; the direct `142.058-T` → PRE-4b edge in PA-1 |
| P1-7 PA-3 not bundled | "PA-3 bundled with PA-1" below; the PA-3 row and the PA-1 approval phrase |

* **Bounded child I/O (P1-1).** Reads are keyed on child exit, never on EOF, with
  grandchild fixtures (PRE-5, PRE-6, NEW-4).
* **No `serde_json` in public signatures (P1-2).** A `ProbeRead` descriptor
  replaces it, and the probe module is top-level `src/preflight_probe.rs`.
* **Git-fixture layout (P1-3).** `read_server_layout` owns `runtime_root` and the
  activation deadline, and PRE-3 REPLACES the daemon's own derivation.
  * *Revision 4 correction (P1 #3, checked against source at HEAD
    `4995d681`):*
    * `read_server_layout` takes `&str`, matching `canonicalize_workspace(&str)`
      (`src/db/workspace.rs:61`). No non-UTF-8 mapping to an invented
      `WorkspaceError` variant is needed.
    * Its identity is exactly what `canonicalize_workspace` returns, with
      no further re-canonicalization. On a primary checkout, the `\\?\`
      prefix is stripped by `normalize_canonical`; on a linked worktree the
      platform spelling is kept.
    * PRE-2's containment check compares like with like, because
      `GenerationStore::new` keeps the `\\?\` spelling.
    * The layout also carries `activation_deadline` and `expected_identity()`.
    * PRE-3 lists the exact lines it removes.
* **G3 lineage (P1-4).** The pre-existing Ship captures `9B7EC1E4` and `6C5DF765`
  were found by the duplicate scan, reconciled with PR #407 thread IDs, and
  decided in D5.
* **`_health` unchanged, with an allowlist (P1-5).** Only
  `GENERATION_PINNED_READS = ["get_workspace_statistics"]` is gated through
  `admit_read`. On a daemon without a generations root, every response is
  unchanged EXCEPT that one pinned read, which is refused with the typed
  `GenerationNotYetActivated` error (fail closed, D5 residual risk). It
  never falls back to managed data (D5-D). The done no-generation consumers
  do not send that method (see PRE-3 "Consumer baseline").
* **PRE-4b (P1-6).** The pinned read serves the opened generation's data, and the
  probes compare a data fingerprint, not only a provenance label.
  * *Attempt-2 P1 #6 closure (checked against source at HEAD):* today
    `get_workspace_statistics` (`src/tools/read.rs:139`) ignores any
    dispatch context. It calls `pinned_read_request_context`, which calls
    `ReadRequestContext::from_managed_state` (`read.rs:99`), and then
    `queries_from_read_context`, which calls
    `connect_db(context.data_dir(), context.branch())` (`read.rs:109`). The
    `tools/mod.rs:476` arm drops `read_context`. PRE-4b is the separate,
    owned unit that changes these two files. PRE-4 owns only
    `process_request`, which hands the admitted context to that arm.
  * *Edge ownership:* PA-1 proposes BOTH edges on `142.058-T` (F54):
    * `142.058-T` → PRE-4. F54 reads through a real CLI and stdio MCP,
      which reach the handler only through `process_request` (PRE-4).
    * `142.058-T` → PRE-4b. This is the direct edge the attempt-2 P1-6 fix
      requires. PRE-4 → PRE-4b already implies it, but the direct edge is
      kept on purpose so that F54's dependency on the data path stays
      explicit in the backlog even if the PRE-4 edge is later re-routed.
      This is not the same case as the dropped PRE-2→`142.054-T` edge: that
      one was a P2 simplification, while this one is part of a P1 fix.
    * Neither edge exists until PA-1 is granted. Stage does not mutate
      `142.058-T` or any other active task before then.
* **PA-3 bundled with PA-1 (P1-7).** PA-3 (ratify D1-A) is no longer only
  "recommended" with PA-1. It is granted only by the single PA-1 approval
  phrase (see "Risky actions"), and cannot be granted on its own. The plan
  infers no pre-approval of PA-1, PA-2, PA-2b, or PA-3.

It also absorbs the attempt-2 P2s listed in the review record. Follow-ups
are captured in stash:

* `86F93068`: the remaining handlers. This BLOCKS F54 GREEN.
* `5AF5CD66`: `_health` alignment.
* `23E287C6`: retention.
* `F99C705E`: incremental seeding.
* `7BF90213`: the `engram-indexer` gate.

### Revision 5: attempt-3 findings resolved (L3-1 plus the attempt-3 P2s)

**Authorization basis.** After attempt 3 opened the circuit, the operator
was shown the options, and Orchestrator recommended path 1: one focused
revision 5 and ONE review attempt 4. The operator then directed "keep
working autonomously until the task is truly finished" and "make good
decisions". Orchestrator relays that as choosing path 1, for
**non-destructive planning only**. It grants nothing for harvest, new
backlog items, the 142-S manifest or its dependencies, code, tests,
config, a PR, or a merge.

The P-013.6 escalation compiled after attempt 3 is still
`ESCALATION_DEGRADED`. The Engram CLI binding is Ready, but engram exposes
no handoff receiver, so the escalation was NOT delivered. Attempt 4 runs on
the operator's direction, not on an escalation outcome.

Source facts below were checked at HEAD `41dd5081`. `src/` is unchanged
since `4995d681`. The F54 test file changed at `6d216d19` (Ship,
`142.058.001-ST`).

**L3-1 closure: the F54 activation-settle protocol (FASP).** L3-1's
hazard has two parts:

* **Timing race.** PRE-3's background activation can write runtime copies
  under the fixture temp root, and can change the
  `get_workspace_status.generation` block that F54's binding fingerprint
  includes. Either change can land inside an F54 side-effect window.
* **Platform divergence (new, found by Stage in this revision).** F54's
  `publish_fixture_generation` (`read_server_cli_mcp_parity_test.rs:476-521`)
  stamps `WorkspaceIdentity::new(workspace_hash(workspace, "main"))`, where
  `workspace` is the `std::fs::canonicalize` spelling. On Windows that
  spelling is verbatim (`\\?\C:\...`). The daemon hashes the
  `canonicalize_workspace` spelling, which `normalize_canonical`
  (`src/db/workspace.rs:21-54`) strips. `workspace_hash` (`:1011`) hashes
  raw bytes. So on Windows, F54's generation would end in a PERMANENT
  `IdentityMismatch` (`activation.rs:545-550`), while on Unix it activates.
  Even a perfect barrier could not name one terminal state for both
  platforms.

FASP closes both parts deterministically:

1. **Identity from the one source (PRE-3F).** The F54 fixture stamps
   branch and workspace identity from PRE-1 `read_server_layout`, so its
   generation activates on every platform. Invariant 9 then holds for test
   fixtures too.
2. **Install-before-ready ordering (PRE-3).** When the generations root
   exists at startup, PRE-3 installs the store, activator, and gate
   synchronously, BEFORE `set_hydration_ready_for_generation`. The install
   does no manifest read, hashing, or copy. `_health` semantics are
   unchanged: readiness never depends on the install or on activation
   succeeding. The existing Release/Acquire `hydration_ready` flag
   (`state.rs:1829-1856`) orders the install before any `ready`
   observation. So once `_health` is `ready`, `generation` is non-null if
   and only if a gate is installed.
3. **Publication-last (PRE-3).** Once the activator publishes
   `active_revision`, the driver makes no filesystem write, and changes no
   `path`/`branch`/`db_path`/`generation` field of `get_workspace_status`.
   It only writes in-memory gate state, emits a stderr `tracing` line (the
   daemon has no file appender: `src/daemon/mod.rs:112-127`), and exits.
4. **Settle barrier (PRE-3F).** `ReadServerFixture::ensure_daemon` ends by
   calling `await_activation_settled`. This is the only place it is
   called, so every F54 side-effect window (unknown method, Control, and
   matrix) starts after the barrier. The barrier returns in exactly two
   cases:
   * `NoActivator`: `generation` is null at a post-ready observation. This
     is deterministic both before PRE-3 (no production code installs an
     activator) and after it (ordering 2).
   * `Active`: `active_revision == published_revision == 1` in two
     consecutive observations 250 ms apart, with an identical full binding
     fingerprint and filesystem snapshot.

   Anything else is re-polled until 75 s pass (the 60 s activation
   deadline plus margin). After that, the barrier fails with
   `F54_BLOCK_MARKER`. It never passes on timeout.
5. **Separate positive and negative tests.**
   * **Positive (new, PRE-3F authors it, PRE-3 turns it GREEN):**
     `published_fixture_generation_activates_at_startup_and_quiesces`.
   * **Negative (existing, preserved):**
     `unknown_ipc_methods_are_refused_without_side_effects`. Its body, its
     refusal assertions (not accepted, code `16_001`), its side-effect
     count, its snapshot helper, and its fingerprint fields are
     byte-identical. Only the shared precondition (`ensure_daemon`) gains
     the barrier, and that call lands BEFORE `binding_before` and
     `files_before` are captured.

   Once a generation can be active, the negative test becomes a STRONGER
   witness of `142.058.003-ST`'s "unknown methods are refused without
   activating a generation": any activation work the unknown method
   triggered would now show up inside the window.
6. **No weakening, anywhere.** FASP uses no `#[ignore]`, no
   `should_panic`, no snapshot path exclusions, no fingerprint field
   removal, and no retry of a failed assertion. Timing is used only to
   confirm stability, never to decide a pass.

*Premise note.* Attempt 3 called the unknown-method test "passing", which
was the status at `4995d681`. Ship's `6d216d19` record
(`docs/memory/2026-09-27-ship-142-058-001-st-structural-completion.md`)
reports "1 passed / 4 failed" for the whole target, with the per-case map
in comments on `142.058-T`. Stage did not rerun tests (P-010). FASP is
correct whichever status the case has at Ship's RED-phase baseline. PRE-3F,
PRE-3, PRE-4b, and PRE-4 therefore require each F54 case to keep BOTH its
status and its failure signature. Checking only "passing cases stay
passing" would miss a RED case whose failure reason activation silently
changed.

**Attempt-3 P2 dispositions:**

| Attempt-3 P2 | Revision 5 disposition |
|---|---|
| Rust / CR-02: Windows `\\?\` vs stripped `starts_with` | PRE-2 uses one comparator, `within_root`: `normalize_canonical(std::fs::canonicalize(x))` on BOTH sides. `normalize_canonical` is `pub(crate)`, and `candidate_build.rs` is in-crate. It covers `VerbatimDisk`, `VerbatimUNC`, and linked worktrees. The F54 identity divergence above is fixed in PRE-3F. |
| Rust: `WorkspaceError::Failed` does not exist | **Confirmed (harvest-session text fix, attempt-4 P2-4).** `WorkspaceError` has NO `Failed` variant (`src/errors/mod.rs:18-43`). "Failed to parse workspace files" is `HydrationError::Failed` (`:45-48`); revision 5's claim that the `WorkspaceError` variant exists was wrong. The design outcome is unchanged: `read_server_layout(&str)` returns `WorkspaceError` and needs no mapping to either variant. PRE-1's text is fixed. |
| Rust (typed error) | `CandidateBuildError` is spelled out as a closed `#[non_exhaustive]` `thiserror` enum (PRE rules). |
| Rust: 2-arg `get_workspace_statistics` caller | PRE-4b keeps the 2-arg `pub` wrapper (managed, byte-identical) and adds `get_workspace_statistics_with_context`. `integration_core_read_generation_pin` (`core_read_generation_pin_test.rs:352`) is unchanged. |
| Rust / A3-1: transient retry | The PRE-3 driver retries a `RejectionClass::Transient` (`activation::classify`) failure of the SAME revision with backoff (100 ms, doubling, 2 s cap), for up to `READ_SERVER_ACTIVATION_DEADLINE` from that revision's first transient failure. A `Permanent` failure is never retried (`5C873386`). |
| Rust / CR-04: probe stderr | PRE-5/PRE-6 spawn with `stderr(Stdio::null())`. Invariant 4 is restated per layer, and NEW-4's relay line gets its own prefix. |
| Rust: refusal returns `accepted:false`, then retries | PRE-5 maps a typed JSON refusal to `ProbeReport { accepted: false, .. }` (never `Exit`). `wait_for_generation` re-probes on `accepted:false` until the deadline. |
| S-1: "no root = unchanged" wording | Fixed. PRE-3 ALONE changes no response. After PRE-4, the one pinned read is refused typed (invariant 7). |
| S-2: duplicate scan missed `EFE9190A`, `4628001C` | Recorded in the duplicate-scan record below, with all four outcomes. |
| CR-01: symlink check omits `runtime_root` | `open_generation_store` also rejects an existing symlinked or reparse-point `runtime_root`, and PRE-2 scenario 2 covers it. |
| CR-03: row VIII safety modes | The Constitution row VII/VIII is rewritten, and the new safety rows are in "Risky actions". |

**Duplicate-scan record (P-021 C5/C6 detection (A), unconditional).**
Stage ran the scan read-only (`backlogit_stash_get`). It made NO stash
edit or archive: the session is non-destructive, and a Stage stash edit
needs a harvest-session authorization.

| Entry | Relation | Outcome | Planned disposition (next authorized Stage session, before harvest) |
|---|---|---|---|
| `EFE9190A` (14 d, "Deferred scope expansion candidate from 139-S": thread the request-entry context into shared dispatch) | Overlaps `6C5DF765` on the dispatch plumbing (PRE-4) and `86F93068` on the per-handler conversion | **Partial overlap, not a duplicate.** It is the earliest capture of the dispatch-threading question. It does not carry the literal `DEFERRED SCOPE EXPANSION` marker, but the deliberation route still applies. Late identifier found: PR #393 (139-S). Several resolved Copilot threads there cite it (`docs/archive/memory/2026-09-12/139-s-ship-session-summary.md:110-113, 251-259`), but the record lists no thread IDs. | Reconcile PR #393 in place. Add an `EFE9190A` note to deliberation D5. At harvest, record PRE-4 as consuming its dispatch-plumbing part, and forward-reference its residual per-handler part to `86F93068`. Archive it at Step 5.6 with both forward refs. |
| `4628001C` (6 d, `DEFERRED SCOPE EXPANSION`: `get_workspace_status`'s code-graph block calls `connect_db` in read-server mode) | Overlaps `86F93068` on the `get_workspace_status` conversion | **Partial overlap, not a duplicate.** It is not consumed by this plan. Late-identifier reconciliation (B), triggered by its `N/A` refs: PR #407 was recovered from the 141-S residual-risk record (`docs/archive/memory/2026-09-23/2026-09-20-ship-141-s-copilot-review-and-ci-infra.md:56, 67`). No review-thread ID cites it, so review-thread `N/A` stands as a truthful record. | Reconcile PR #407 in place. Carry it into the `86F93068` deliberation (PA-5) as that conversion's source. Not archived here. |
| `86F93068`, `9B7EC1E4`, `6C5DF765`, `5AF5CD66` | As recorded in revision 4 | **Clean re-scan.** No new duplicate was found. | Unchanged |

The F54 settle barrier and the pinned-read admission leave `4628001C`'s
`connect_db` path exactly as it is at HEAD: F54 already calls
`get_workspace_status` outside every side-effect window.

### Revision 6: four-shipment split (operator Q1-Q8, 2026-09-27 22:54)

**Authorization basis.** At 2026-09-27 22:54 -07:00 the operator answered
"yes to all" to the questions in
`docs/memory/2026-09-27-stage-142-f-shipment-split.md` and its follow-up:

1. Use the 4-shipment split S1-S4 ("Option A", below).
2. Stage writes this Revision 6 and runs ONE review scoped to it.
3. 142-S is abandoned and its tasks are re-queued (later session).
4. S1-S4 are created as queued shipments after the review passes (later
   session).
5. Git handling belongs to Orchestrator/Ship.
6-8. PA-5: option PA5-C; F54's fixture becomes a real indexed snapshot;
   only the code-graph counts of `get_workspace_status` are served from the
   snapshot. A separate Stage session plans PA-5.

This revision changes **sequencing, shipment membership, and which task
owns the F54-based evidence**. It changes no unit's production files,
functions, behavior, or scenarios, except the PRE-3F test-first posture
(R6.4). Where this section and older text disagree, this section wins.

Source facts were checked read-only at HEAD `41dd5081` on the parked
branch `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs`.
Every harness commit named below exists there. Edges were read through
`backlogit_get_dependencies`; nothing was changed.

#### R6.1 Why the split needs a plan change

`tests/contract/read_server_cli_mcp_parity_test.rs` (F54) exists only on
the parked branch (`7bd9e504`, `6d216d19`). It has 4 failing tests, and the
equivalence case stays RED until PA-5 (stash `86F93068`) lands. Revision 5
used that file as proof for PRE-3F, PRE-3, PRE-4b and PRE-4, and assumed one
final merge. With four shipments that each merge fully GREEN, the F54 file
cannot ship before S4. Everything that edits it or uses it as evidence
must therefore move to S4.

#### R6.2 Shipment table

S1-S4 are provisional labels. backlogit assigns the IDs at creation.

| Shipment | Members (in execution order) | Task count | Blocked by | Parked-branch commits it carries | State of those commits at cherry-pick | Merges fully GREEN because |
|---|---|---|---|---|---|---|
| **S1** read-server plumbing | `142.061-T` PRE-1 → `142.062-T` PRE-2 → `142.064-T` PRE-3 (`.001-ST` → `.002-ST` → `.003-ST`) → `142.065-T` PRE-4b → `142.066-T` PRE-4 → `142.067-T` PRE-5 → `142.068-T` PRE-6 | 7 (+3 ST) | none | `a0ccdc27` (operator edits and checkpoint repairs) and the H1 planning-state commit. **No RED harness commit.** | no test content | Each unit writes its own RED harness and turns it GREEN inside S1. The F54 file is absent, so no F54 case can be RED. |
| **S2** self-check and the `preflight` command | `142.054-T` F50 (`.001-ST` → `.002-ST` → `.003-ST`) → `142.069-T` NEW-1 → `142.070-T` NEW-2 → `142.071-T` NEW-3 → `142.072-T` NEW-4 → `142.073-T` NEW-5 → `142.074-T` NEW-6 | 7 (+3 ST) | S1 | F50 scaffold `c269fa79` + `41dd5081` (clippy fix) | GREEN (the scaffold passes) | F50 completion adds the real-chain case and turns it GREEN; NEW units are test-first and GREEN inside S2. |
| **S3** launch scripts and archive fix | `142.055-T` F51 → `142.056-T` F52 → `142.057-T` F53 → `142.060-T` archive verifier | 4 | S2; **PA-6 hold** (Revision 8, R8.6): not claimed until the operator decides PA-6 again | F51 `1dfc1b5b` + `4995d681` (3 RED); F52 `5760b948` (1 RED); F53 `e24f5ae2` (3 RED); archive `a47b8aff` (2 RED) | 9 RED, all owned by S3 tasks | Each RED harness is owned by an S3 task that turns it GREEN. None uses F54 (R6.5). |
| **S4** parity test, PA-5, docs, closure | `142.063-T` PRE-3F → PA-5 tasks (planned later) → `142.058-T` F54 (`.002-ST`, `.003-ST`; `.001-ST` is done) → `142.059-T` F55 docs → `142-F` last | 3 + PA-5 | S3 | F54 `7bd9e504` + `6d216d19` | 4 RED (incl. the equivalence case) | PRE-3F, PA-5 and `142.058-T` together turn every F54 case GREEN. S4 does not merge until they do. |

*(Revision 9, attempt-7 P3-8: the S3 row's hold is enforced per R9.8.
In the S4 row, the F54 commits are not cherry-picked into `142.063-T`;
`142.058-T` loads the `6d216d19` blob in its own FL commit after PA-5
(R9.5), and F54 is the GREEN placeholder until then. "4 RED" exists only
inside `142.058-T`'s own work (R9.4). S4 order is unchanged.)*

*(Revision 10: `142.058-T`, `142.059-T` and `142-F` move to a new S5
after S4. S4 is `142.063-T` → PA-5. See R10.4 and R10.6.)*

**Inter-shipment blocks.** S2 is blocked by S1, S3 by S2, S4 by S3. The
item edges already enforce most of this: `142.054-T` → `142.068-T`
(S2 on S1); `142.055-T`/`142.057-T` → `142.054-T`, `142.073-T` (S3 on S2);
`142.059-T` → `142.055-T`, `142.057-T` (S4 on S3). `142.063-T` and
`142.058-T` have no item edge into S3. For them the S4-after-S3 order comes
from the shipment ordering and P-001 (one active release), not from an
edge. *(Revision 7/9: E8 now gives `142.063-T` an edge into S3; the PA-6
hold blocks S3 and S4 claims, R9.8.)*

**`142-F` placement.** `142-F` is added only to S4, and last, so the
shipment that closes the feature is the last one. S1-S3 contain no feature
item. Their tasks keep `142-F` as `parent_id`.

**No RED interval on a shipping branch.** The 9 S3 RED tests and the 4 F54
RED tests are cherry-picked only into the shipment that owns them. Under
Revision 5, one branch carried them through S1-S3 work. That interval is
gone, and so is the recurring F51 `WARNING:` payload noise in S1/S2 full
suites (Risks, D1-A).

#### R6.3 PRE-3F moves to S4, and its dependencies change

PRE-3F (`142.063-T`) now runs FIRST in S4, after S1-S3 have merged. It is
no longer before PRE-3.

**Edges (depends-on form: "A → B" means A depends on B):**

| Change | Edge | Reason |
|---|---|---|
| remove | `142.064-T` → `142.063-T` | PRE-3 lands in S1; PRE-3F is in S4. |
| add | `142.063-T` → `142.066-T` | PRE-3F's baseline assumes PRE-3, PRE-4b and PRE-4 are on the branch. PRE-4 depends on PRE-3 and PRE-4b, so this one edge covers all three. |
| add | `142.058-T` → `142.063-T` | F54 completion follows the barrier and the identity fix. |
| add | `142.064-T` → `142.062-T` | Removing `142.064-T` → `142.063-T` would leave PRE-3 with no upstream edge. PRE-3 needs PRE-1/PRE-2 (R6.9 E5). |
| keep | `142.063-T` → `142.062-T` | Now implied by `142.063-T` → `142.066-T`. Kept to avoid churn; the assembly session may drop it. |

**Why "before PRE-3" is no longer needed.** Revision 5 put PRE-3F first
because, if PRE-3 landed first, F54 would be racy "for the whole PRE-3 →
PRE-3F interval". Under this split the F54 file is not on any shipping
branch during that interval. It first appears in S4, in the same change
as PRE-3F (R6.6, L3-1 guard). The race window does not exist.

**Unchanged in PRE-3F.** The identity-from-layout edit, the barrier (all
of 142.063-T's attempt-4 P2-1/P2-2/P3 criteria), the `F54 settle:` stderr
line, the diff contract, the byte-identical negative test, and the
invariant-6 exception (granted by the operator on 2026-09-27; placement
now S4) are unchanged. The diff contract still forbids changing the
fixture DB bytes. PA-5's fixture change comes later, as its own change
(R6.8). *(Revision 9: superseded. The invariant-6 exception is history
and is not exercised (PA-7, R9.1). PRE-3F owns its own target and support
(R9.3); the identity fix, the barrier call and the real-snapshot fixture
change in the F54 file belong to `142.058-T` (R9.5).)*

**Changed in PRE-3F.**

* **Baseline.** On the S4 branch, the un-barriered F54 file must never run
  against a post-PRE-3 daemon, because that is exactly the L3-1 race. So
  the "before-edit" run on S4 is NOT a gate and is not required. The
  comparison baseline **B0** is a per-row F54 map (attempt-4 P2-8 format)
  captured where no activator can be installed. In order of preference:
  1. Ship's recorded `6d216d19` per-case map (comments on `142.058-T`), if
     it is already in per-row format; otherwise
  2. a fresh B0 run of `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture`
     on the parked branch at `41dd5081`, where G3 still holds (no
     production activator). Ship runs it before the H5 branch change, or
     later from the preserved parked ref. It is a verification-only run
     with no commit.

  The expected post-S1 map **B1** is B0 plus ONLY these enumerated changes
  (the union of the lists moved out of `142.064-T` and `142.066-T`, R6.5):
  * (i) rows whose captured text includes a `get_workspace_status` response
    may now show a non-null `generation` block;
  * (ii) the `_shutdown` window may now include the runtime-copy DB
    teardown at exit;
  * (iii) rows whose method is `get_workspace_statistics` may change
    signature (for example, a typed query error against the
    `probe_row`-only fixture DB);
  * (iv) the new positive test is present and GREEN.

  Each observed change is recorded with its new text. Any other difference
  from B0 is a HALT to Stage.
* **Test-first posture:** see R6.4.

#### R6.4 Test-first exception: PRE-3F's positive test lands GREEN

**The exception.**
`published_fixture_generation_activates_at_startup_and_quiesces` lands
after PRE-3, so it is GREEN the first time it runs on the S4 branch.
Revision 5's "RED at step (a) until PRE-3" and its `EXPECTED_PENDING_RED`
mapping (owner PRE-3) no longer apply. The test is a **regression
witness**, not the RED-first driver of new behavior.

**Justification.**

1. **The behavior was already driven test-first in S1.** Install-before-
   ready, activation and single-source identity are production behavior
   owned by PRE-3. PRE-3's own harness (`integration_read_server_generation_wiring`,
   scenario 1: "at the FIRST `ready`, `generation` is non-null";
   `active_revision == 1`) is written RED first and turned GREEN in S1.
   Principle II's intent (no production behavior without a failing test
   first) is met there.
2. **PRE-3F adds no production behavior.** It is tests-only. Its job is to
   make F54 deterministic and to witness the same behavior on F54's own
   fixture.
3. **Keeping it RED-first would cost more than it protects.** It would
   need the F54 file (4 known-RED tests) on S1's branch, which breaks the
   operator's "each shipment merges fully GREEN" rule. Or it would hold
   S1 until PA-5, which is the 3-shipment variant the operator did not
   choose.

**Compensating control (mandatory; `142.063-T` acceptance).** A GREEN-on-
landing test can pass vacuously. Ship must prove it can fail:

* **C1 RED witness against a pre-PRE-3 commit.** Ship takes the S1 commit
  on `main` where `142.062-T` (PRE-2) has landed and `142.064.001-ST` has
  not. Call it `<pre-PRE-3>`; S1 uses merge commits (Principle XI), so it
  stays reachable. In a verification-only, detached checkout of
  `<pre-PRE-3>` (no commit, no push, removed in the same step), Ship
  overlays the S4 PRE-3F version of the F54 test file and its
  `[[test]] name = "contract_read_server_cli_mcp_parity"` stanza. Ship
  then runs only the positive test. It MUST fail at step (a) with
  `F54_RED_MARKER` and "generation not installed before readiness
  (PRE-3)". Ship records the commit SHA, the command, and the failure text
  in the task record.
  * If Ship's worktree/checkout policy (P-001/P-016) does not permit that
    checkout, Ship asks the operator for an equivalent. If none is granted,
    Ship HALTs to Stage. The exception is never accepted without a
    recorded RED witness.
* **C2 structural check (Ship review gate).** Step (a) polls `_health`
  directly, NOT through the barrier, and its assertion runs before
  `await_activation_settled` is called. A step (a) that the barrier could
  mask is a PRE-3F defect.
* **C3 cross-reference.** The task record cites the S1 commit where
  PRE-3's scenario 1 went RED → GREEN, so the test-first chain for this
  behavior is auditable end to end.

#### R6.5 F54 evidence moved out of S1 (exact text)

S1 tasks keep every criterion that does not need the F54 file. That covers
every consumer-baseline target other than F54, the grep for other
`stats` senders, REPLACE evidence, publication-last, install-failure-final,
invariant 7, and clippy. The criteria below are REMOVED from the S1 task
and OWNED by the S4 task named. The quoted text is the current
backlogit acceptance text.

| # | From (S1) | Moved text (verbatim start) | New owner (S4) |
|---|---|---|---|
| M1 | `142.064-T` acceptance | "(attempt-4 P2-1) In the three-run F54 evidence, every PRE-3F barrier return line (`F54 settle: ...`) for a fixture that published a generation is `Active`. Any `NoActivator` return after PRE-3 is a HALT, not a flake." | `142.063-T` |
| M2 | `142.064-T` acceptance | "(attempt-4 P2-8) F54 per-case map against the PRE-3F after-edit map (per-row signature format): each case keeps status AND signature except these enumerated expected changes: (i) … (ii) … (iii) … Each observed change is recorded with its new text; any other change is a HALT." | `142.063-T`, recast as the B0 → B1 comparison (R6.3). Change (i) "RED -> GREEN" becomes (iv) "present and GREEN". |
| M3 | `142.064-T` acceptance | "F54 determinism: three consecutive `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture` runs after the build give identical per-case maps (full output captured, and wall-clock time per run recorded, attempt-4 P3). Any difference is a HALT." | `142.063-T` (three runs after the PRE-3F edit on the S4 branch) |
| M4 | `142.064-T` acceptance, consumer-baseline bullet | only the words "and every F54 case" | dropped from S1. The F54 baseline is B0 in `142.063-T`. |
| M5 | `142.064-T` verification step 3 | "Three identical `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture` per-case maps." | `142.063-T` verification |
| M6 | `142.064.003-ST` description | "Then produce the PRE-3 closure evidence: the full F54 per-case map against the PRE-3F after-edit map with the enumerated expected changes only (attempt-4 P2-8); three consecutive identical `contract_read_server_cli_mcp_parity` runs with full output and wall-clock time recorded, and every `F54 settle:` line `Active` (attempt-4 P2-1)" | `142.063-T` (M1-M3). `.003-ST` keeps "all consumer targets matching the RED-phase baseline". Its title becomes "PRE-3c: Retry transient same-revision activation and record consumer evidence". |
| M7 | `142.065-T` acceptance | "…and F54 per-case status AND per-row signature (attempt-4 P2-8 format) unchanged against the latest recorded map (PRE-3's post-build map when PRE-3 has run; otherwise the PRE-3F after-edit map, attempt-4 P3: the graph has no PRE-3 edge)." | `142.063-T` (PRE-4b expects no F54 change beyond B1 item (iii)). `142.065-T` keeps "Consumer regression targets unchanged against the PRE-3 RED-phase baseline … HALT if any goes RED." |
| M8 | `142.065-T` verification step 3 | the words "+ F54 per-case map" | dropped from S1 (covered by `142.063-T`) |
| M9 | `142.066-T` acceptance | "(attempt-4 P2-8) F54 cases keep status AND per-row signature against the PRE-3 post-build map, except rows whose method is `get_workspace_statistics`; each such change is recorded in the Step 4.3 map with its new text (…)" | `142.063-T` (B1 item (iii)) |
| M10 | `142.066-T` acceptance | "The F54 determinism evidence (three identical consecutive runs, full output, wall-clock recorded) is repeated after the PRE-4 build." | `142.063-T`. One three-run set on the S4 branch covers PRE-3, PRE-4b and PRE-4 together, because all three are present. |
| M11 | `142.066-T` verification step 3 | "Three identical F54 runs after the build." | `142.063-T` verification |
| M12 | (new, S4) | "After the PA-5 fixture and handler changes land, three consecutive identical F54 runs, every `F54 settle:` line `Active`, and every case GREEN." | `142.058-T` (its final gate before `142.059-T`) |

**Kept in S1, restated so it is self-contained (text change at
assembly).** `142.066-T` scenario 2's "reconciliation quiescence" uses "the
PRE-3F two-observation rule". PRE-3F is now in S4, so that criterion must
define the rule inline: status call 1 (F1, G1), filesystem snapshot S1,
sleep 250 ms, snapshot S2, status call 2 (F2, G2). The state is settled
when `G1.active_revision == G1.published_revision == G2.active_revision ==
G2.published_revision == Some(2)`, `S1 == S2`, and `F1 == F2`, compared
separately, with no IPC call between S1 and S2. The warm-up read and the
sidecar HALT rule (attempt-4 P2-9) are unchanged. The check runs inside
PRE-3's harness (`integration_read_server_generation_wiring`), which S1
owns, so it needs no F54 file.

**Plan-text overrides.** Where the PRE-3, PRE-4b and PRE-4 unit sections
and the Runtime Verification rows mention F54 maps, three F54 runs, or
"the PRE-3F after-edit map", read them through M1-M12.

#### R6.6 L3-1 guard in S4

*(Revision 9: superseded by the R9.5 item 2 L3-1 guard; the FL commit of
`142.058-T` carries the real body and the barrier call together.)*

L3-1 stays closed only if the un-barriered F54 file never runs against an
activating daemon. In S4:

* The first S4 change is the cherry-pick of `7bd9e504` + `6d216d19`
  together with the PRE-3F edit. No test gate runs between the cherry-pick
  and the PRE-3F edit. The recommended form is one commit for
  `142.063-T` that contains both the picks and the edit; if Ship keeps them
  as separate commits, it runs no F54 target until the PRE-3F commit is in
  place.
* B0 is captured only where G3 holds (R6.3). It is never re-captured on
  the S4 branch.
* Any F54 run on S4 without the barrier present is a PRE-3F defect.

#### R6.7 `142.060-T` no longer depends on `142.058-T`

Remove the edge `142.060-T` → `142.058-T`. The archive verifier
(`a47b8aff`, `scripts/` read-before-close) does not use the F54 file or any
F54 result. `142.060-T` still depends on `142.054-T` (S2), `142.055-T`,
`142.056-T`, and `142.057-T` (S3). It stays the last code task of S3, and
`142.059-T` (S4) still depends on `142.058-T`.

#### R6.8 PA-5 inputs that S4 depends on (recorded, not planned)

The operator's PA-5 answers are recorded here only as inputs. This session
does no PA-5 planning. A separate Stage session will run deliberation, plan
and review for it.

| Input | Operator answer (2026-09-27 22:54) |
|---|---|
| PA-5 option | **PA5-C** |
| F54 fixture | change F54's fixture to a **real indexed snapshot** |
| `get_workspace_status` scope | serve **only the code-graph counts** of `get_workspace_status` from the snapshot |

**S4 dependencies on PA-5.**

* S4 cannot merge until the PA-5 tasks exist, are harvested into S4 after
  `142.063-T` and before `142.058.002-ST`, and are GREEN. The equivalence
  case needs them.
* The PA-5 tasks do not exist yet, so assembly adds them to S4 later. S4
  may be created with its known members first.

**Checks for the PA-5 planning session (carried from FASP; not
decisions).**

* The PRE-3F barrier polls `get_workspace_status` and treats a null
  `generation` block as `NoActivator`. It needs that method to stay
  accepted on a daemon with no gate.
* F54's binding fingerprint reads that method's `path`, `branch`,
  `db_path` and `generation` fields. A counts-only change must leave them
  unchanged, or the FASP determinism rule must be re-reviewed.
* The fixture change edits `142.058-T`'s file outside PRE-3F's diff
  contract. It needs its own owner (PA-5 task or `142.058-T`), and M12
  re-proves determinism after it. *(Revision 9: the owner is
  `142.058-T` (`.002-ST`), never a PA-5 task; R9.5 item 4.)*
* Stash `4628001C` (the code-graph `connect_db` path) and `86F93068` feed
  that session, as recorded in Revision 5.

#### R6.9 Edge-change list for the assembly session (NOT applied here)

No edge was changed in this session. The assembly session applies these
in order, after 142-S is abandoned (H2/H3) and before S1-S4 are created
(H4). It then re-runs a cycle check over all 142-F edges.

| # | Operation | Item (depends on) | Target |
|---|---|---|---|
| E1 | `backlogit_remove_dependency` | `142.064-T` | `142.063-T` |
| E2 | `backlogit_add_dependency` (`blocks`) | `142.063-T` | `142.066-T` |
| E3 | `backlogit_add_dependency` (`blocks`) | `142.058-T` | `142.063-T` |
| E4 | `backlogit_remove_dependency` | `142.060-T` | `142.058-T` |
| E5 | `backlogit_add_dependency` (`blocks`) | `142.064-T` | `142.062-T` |

**Why E5.** `142.063-T` is currently PRE-3's ONLY recorded upstream edge,
so E1 alone would leave PRE-3 with none. PRE-3 needs PRE-1
`read_server_layout` and PRE-2 `open_generation_store`. E5 restores the
plan's `PRE-2 -> PRE-3` order directly. Apply E5 in the same step as E1,
so the queue never shows PRE-3 as unblocked.

Stage checked E1-E5 read-only against the current edges. After them
there is no path from `142.062-T`, `142.064-T`, `142.065-T` or `142.066-T`
back to `142.063-T`, and none from `142.063-T` back to `142.058-T`, so the
graph stays acyclic.

**Item text changes at assembly (Stage authority, after the review
passes):** M1-M12 and the `142.066-T` inline-rule restatement (R6.5). Also:
* `142.063-T`: add B0/B1, R6.4 C1-C3 and R6.6. Remove "RED at step (a)
  until PRE-3", the `EXPECTED_PENDING_RED` mapping, and "Downstream:
  PRE-3". Set Depends on: PRE-4 (`142.066-T`); Downstream: `142.058-T`.
* `142.064-T` implementation notes: "Depends on: PRE-3F (and therefore
  PRE-2)" becomes "Depends on: PRE-2 (E5)".

#### R6.10 Scope limits of this revision

* No backlog item, edge, shipment, stash entry, source, test, or config
  file was changed. There was no git mutation.
* 142-S is still active. Abandoning it (H2), re-queuing its items (H3),
  creating S1-S4 (H4), and git handling (H5/H6) are later sessions'
  work.
* PA-5 is not planned here.

### Revision 7: attempt-5 P1s and selected P2s resolved

**Authorization basis.** The Orchestrator (on autopilot, 2026-09-27
23:16 -07:00) authorized Revision 7 and ONE review scoped to it (attempt
6), with a STOP and no Revision 8 on any P0/P1. The basis is the
operator's "yes to all" to the split. The directive fixed four points:

* Fix P1-A, P1-B and P1-C.
* For P1-C, use option (a): show the red phase BEFORE
  `harness_status: harness-ready`, with NO test-first exception. Do not
  check out an older commit into a second worktree or branch (P-016), and
  do not rewrite history.
* Absorb the PA-5 edges, the F54 hold, the parked-branch tag, the C1
  worktree conflict, and the PA-4/S3 consequence.
* Keep the same hard bounds: no backlog, edge, shipment, stash, source,
  test, config, or git change.

**Precedence.** Where Revision 7 and Revision 6 disagree, Revision 7
wins. Revision 7 **withdraws** R6.4 (the test-first exception and its
controls C1-C3) and the Revision 6 Constitution "II deviation" text. It
also **replaces** R6.3's B0 sources and R6.6's L3-1 guard. Everything
else in Revision 6 stands.

The source facts were checked read-only on the parked branch at HEAD
`41dd5081`:

* `a0ccdc27` is an ancestor of HEAD.
* The only uncommitted path under code directories is the generated
  `scripts/__pycache__/`. Every other uncommitted file is backlog, memory,
  plan, decision, or agent-harness state.
* `start.ps1` today is a fail-open pre-warm wrapper.
* `142.055-T` requires the fail-open release path to be "removed, not
  merely lengthened".

#### R7.1 P1-A: two more moved criteria (M13, M14), plus M15-M17

These rows extend the R6.5 table. The quoted text is the current
backlogit text.

| # | From | Moved or changed text (verbatim start) | New owner / replacement |
|---|---|---|---|
| M13 | `142.073-T` (NEW-5, S2) acceptance, and the NEW-5 plan acceptance (the bullet naming the F54 CLI map at `read_server_cli_mcp_parity_test.rs:~855-890`) | "`unit_cli_parser`, `integration_cli_e2e`, `contract_cli_tool_catalog_parity`, and `contract_read_server_cli_mcp_parity` are unaffected (per-case maps unchanged)." | Remove `contract_read_server_cli_mcp_parity` and the F54-map sentence from S2. The other three targets stay. **Owner: `142.063-T`.** B1 expects NO F54 change from S2 or S3 (R7.4), so NEW-5's "F54 unaffected" claim is proved by the B0 → B1 comparison in S4. |
| M14 | `142.064.001-ST` (S1) description, RED phase | "the F54 per-case map (against the PRE-3F after-edit map)" | Remove it from S1. The consumer baseline maps, the pinned-read sender grep, and the rest of that sentence stay. **Owner: `142.063-T`** (B0, captured at A0, R7.4). |
| M15 | `142.064-T` (S1): the scope line, the HALT note, and the `.003-ST` pointer | "ST-c transient retry + F54 determinism evidence"; "or any consumer target or F54 case deviates beyond the enumerated changes"; `.003-ST` notes "and PRE-3 acceptance \"F54 determinism evidence\"" | Replace with: "ST-c transient retry + consumer evidence"; "or any consumer target deviates from its RED-phase baseline"; drop the pointer. **Owner of the F54 part: `142.063-T`.** |
| M16 | `142.060-T` (S3) description and verification | "SEQUENCING: final code task of 142-S (blocked by 142.054-T through 142.058-T …). Executable only after explicit operator authorization adds it to active shipment 142-S (plan PA1)."; "This is the final 142-S code task"; "in the 142-S PR residual-risk record" | Replace with: "SEQUENCING: final code task of S3 (blocked by `142.054-T`–`142.057-T`; R6.7 removed the `142.058-T` edge). Executable once S3 is claimed."; "This is the final S3 code task" (the no-pending-red full-suite gate is unchanged); "in the S3 PR residual-risk record". |
| M17 | `142.068-T` (S1) acceptance; PRE-3 scenario 1 and PRE-4 scenario 1 plan text | "the same extraction F54 uses for MCP"; "(through its F54 `cli_arguments` mapping, `--json`)"; "`stats` is the F54 `cli_arguments` mapping of `get_workspace_statistics`" | Say it inline instead: "the `generation_id` and `data_fingerprint` fields of the tool result's `structuredContent`"; "through `engram --workspace <W> --json status`"; "`stats` is the CLI subcommand that sends `get_workspace_statistics`". The S1 branch has no F54 file to point at. |

#### R7.2 P1-B: complete rewrite list for `142.063-T` (applied at assembly)

`AC` = acceptance-criteria bullet in the current backlogit text, in order.
Every item not listed here is kept verbatim.

| # | Current text (verbatim start) | Action | Replacement |
|---|---|---|---|
| T1 | AC1 "PA-1 has been granted (the filled phrase in the plan's Harvest Record) before the first edit; otherwise HALT." | replace | "The invariant-6 exception granted on 2026-09-27 (deliberation, Operator Approval) covers this edit, within the diff contract below. Placement: first code task of S4 (plan Revisions 6-7)." |
| T2 | AC2 "(attempt-4 P2-8) Per-row failure signature. Before editing, Ship runs `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture` … After the edit every pre-existing case has the SAME status and the SAME per-row signature. … later units (PRE-3, PRE-4b, PRE-4) compare against the after-edit map in this format." | replace | Keep only the per-row signature DEFINITION (single-assertion cases: the first assertion line after the marker; multi-row cases: the ordered failing rows as (descriptor/method, surface, expectation prefix up to the first payload value), with no payloads). Then add: "Comparison: after the implementation commit (IC, R7.3), three consecutive full-output runs are compared against B0 (captured at A0, R7.4) under the B1 rule. There is NO run of this target before the harness commit on the S4 branch. Remove the sentence 'later units … compare against the after-edit map'." |
| T3 | AC3, characterization clause "whether snapshot, then `get_workspace_status`, then snapshot is stable at HEAD" | replace | "… is stable on the S4 branch after IC". The stability rule itself is unchanged. |
| T4 | AC5 "… so PRE-3's three-run evidence can show every return variant." | replace | "… so the three-run evidence of this task and of `142.058-T` (M12) can show every return variant." |
| T5 | AC7 (positive test), last sentence "At HEAD it fails at (a); it is mapped `EXPECTED_PENDING_RED` with PRE-3 as owner (Ship Step 4.3)." | replace | "Red phase (R7.3): in the harness commit HC, step (a) PASSES (S1's install-before-ready) and the test fails at step (b) with the stub marker. It is mapped `EXPECTED_PENDING_RED` with owner `142.063-T` and turns GREEN at IC. Its body is byte-identical in HC and IC." |
| T6 | AC8 (diff contract) | extend | Add: "The contract applies to HC and IC together, relative to the `6d216d19` blob. HC adds the cherry-picked file and stanza, the `Settled`/`SETTLE_TIMEOUT` items, `await_activation_settled` with its FINAL signature and a STUB body, its one call site, and the positive test. IC changes ONLY the identity lines (and their imports) in `publish_fixture_generation`, and the barrier BODY. `git diff HC IC -- tests/contract/read_server_cli_mcp_parity_test.rs` shows nothing else." |
| T7 | AC9 "`unknown_ipc_methods_are_refused_without_side_effects` is byte-identical, and the barrier runs before its `binding_before`/`files_before`." | extend | Add: "In HC it fails at `ensure_daemon` with the stub marker (mapped `EXPECTED_PENDING_RED`, owner `142.063-T`). After IC its B0 status and signature return, within B1." |
| T8 | Description "Test scenarios (2): (1) baseline preservation (characterization); (2) positive RED witness." | replace | "Test scenarios (2): (1) red phase at HC (R7.3); (2) GREEN at IC, with three identical runs compared against B0 under B1 (R7.4)." |
| T9 | Description item 3 "… RED at step (a) until PRE-3." | replace | "… RED at step (b) in HC (stub barrier), GREEN at IC." |
| T10 | Description, OWNERSHIP EXCEPTION "…may edit it ONLY after the operator grants PA-1 … Without PA-1 this task must not start." | replace | "…may edit it under the invariant-6 exception granted 2026-09-27, ONLY within the diff contract. `142.058-T` stays the owner." |
| T11 | Notes "Depends on: PRE-2 (…). Downstream: PRE-3. No dependency edge to or from 142.058-T (…)." | replace | "Depends on: `142.066-T` PRE-4 (E2), `142.060-T` (E8), and `142.062-T` (kept, implied). Downstream: the first PA-5 task (E7) and `142.058-T` (E3)." |
| T12 | Notes "Why separate and before PRE-3: the barrier is correct at HEAD …; landing it after PRE-3 would leave F54 racy." | replace | "Why after PRE-4 (Revision 6 R6.3): the F54 file first reaches a shipping branch in this task's HC, which carries the barrier call, so there is no racy interval." |
| T13 | Notes HALT "a pre-existing case changes status or per-row signature" | replace | "any IC run differs from B0 outside B1 (i)-(iv); the three IC runs differ from each other; any `F54 settle:` line is not `Active`; or the HC red phase shows any failure other than the expected stub-marker failures (R7.3)" |
| T14 | Verification 1 "Before/after … per-case maps (per-row signatures) recorded and equal for every pre-existing case." | replace | "B0 (A0) and three IC per-case maps (per-row signatures) recorded, and compared under B1." |
| T15 | Verification 2 "The new positive test fails at step (a) with its named message." | replace | "HC red-phase record (R7.3): SHA, command, full output, OS. The positive test fails at (b) with the stub marker after (a) passed. Every pre-existing case that calls `ensure_daemon` fails there with the stub marker, and every other case keeps its B0 status." |
| T16 | Labels `pending-pa-1` | leave | Label cleanup belongs to the assembly session, as recorded in the 2026-09-27 split memory. |

**Plan-unit text.** The PRE-3F override note (updated in this revision)
supersedes the unit's scenarios 1-2, its "At HEAD, (a) fails, which is the
RED witness" text, and its HALT bullet "A pre-existing case changes status
or signature". They are read through T2, T5, T8 and T13.

#### R7.3 P1-C: PRE-3F's red phase (no test-first exception)

*(Revision 9: this section is withdrawn and replaced by R9.3 (HC/IC in
the new target `integration_read_server_activation_settle`) and R9.5 (the
F54 file, edited only by `142.058-T` after PA-5). The "with the stub
marker" wording is withdrawn everywhere.)*

**Mechanism: a stub-first harness commit on the S4 branch.** PRE-3F is a
tests-only task. Its implementation is the test-support code it owns: the
settle barrier's body and the fixture identity fix. Its harness is the
positive test plus the barrier call site. So the red phase does not need
PRE-3 to be absent; it needs the barrier to be absent. That state exists
naturally on the S4 branch, before PRE-3F's implementation.

1. **HC, the harness commit** (harness-architect, BEFORE
   `harness_status: harness-ready`). It is ONE new commit on the S4
   branch, containing all of the following:
   * the F54 file and its `[[test]] name = "contract_read_server_cli_mcp_parity"`
     stanza, exactly as at `6d216d19`. They are brought in as the combined
     content of `7bd9e504` + `6d216d19` (for example `git cherry-pick
     --no-commit`), and the commit message cites both SHAs. This is a new
     commit; no history is rewritten.
   * `enum Settled { NoActivator, Active }` (deriving `Debug, Clone,
     Copy, PartialEq, Eq`) and `SETTLE_TIMEOUT`.
   * `async fn await_activation_settled(&mut self) -> Result<Settled, String>`
     with its FINAL signature and a STUB body. The body makes no IPC call,
     prints no `F54 settle:` line, and immediately returns
     `Err(format!("{F54_RED_MARKER}: activation-settle barrier not implemented (PRE-3F)"))`.
   * its one call site at `ensure_daemon`'s ready-return point,
     propagated with `?`.
   * the positive test `published_fixture_generation_activates_at_startup_and_quiesces`,
     in its FINAL form.

   HC does NOT contain the identity fix or the real barrier body.
2. **The red-phase record** (before `harness-ready`). Ship or the harness
   architect runs `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture`
   and keeps the full output. The expected result, on every OS:
   * The positive test passes step (a), because S1's install-before-ready
     is on the branch, and FAILS at step (b) with the stub marker.
   * Every pre-existing F54 case that calls `ensure_daemon` FAILS there
     with the stub marker, BEFORE any side-effect window opens. A case
     that never calls `ensure_daemon` (for example, a purely structural
     descriptor check) keeps its B0 status and signature.

   These results are NOT valid RED and are a HALT to Stage:
   * a compile error;
   * any other marker or failure text;
   * a failure in step (a);
   * any case reaching a side-effect window.

   The harness manifest records `Red Phase: CONFIRMED (HC <sha>, stub
   marker)`, plus the command, the full-output location, and the OS. It
   maps the positive test and `unknown_ipc_methods_are_refused_without_side_effects`
   as `EXPECTED_PENDING_RED` with owner `142.063-T`. It maps the four
   known-RED F54 cases as `EXPECTED_PENDING_RED` with owner `142.058-T`,
   as before. Only then is `harness-ready` applied (P-002/P-004).
3. **IC, the implementation commit** (Ship build). It contains the
   identity fix in `publish_fixture_generation` (and its import lines) and
   the real barrier body. After IC:
   * the positive test is GREEN;
   * the unknown-methods test returns to its B0 status;
   * the four known-RED cases return to their B0 signatures within B1
     (R7.4).

   The positive test and every existing test body are byte-identical
   between HC and IC (T6).

**Why this is genuine test-first.**

* The positive test fails in HC for exactly the reason IC removes: the
  barrier is not implemented.
* On Windows, step (b) is also RED until IC's identity fix. Without it,
  the fixture's generation ends in a permanent `IdentityMismatch`, and the
  real barrier would time out. So both parts of IC are test-driven on
  Windows. On Unix, the identity fix changes no behavior (the spellings
  already match).
* Step (a) passes in HC. It is a regression assertion of S1 behavior that
  PRE-3's own harness drove RED-first in S1 (PRE-3 scenario 1). It is not
  new behavior of this task.
* **No test-first exception is needed, and none is claimed.**

**Compliance.**

* One worktree, on the S4 branch only.
* No checkout of any older commit or other branch.
* No second worktree (P-016). No history rewrite: HC and IC are new
  commits.
* No production code, flag, or env switch. A test-only production switch
  was considered and rejected, because it adds production surface only for
  a test.
* Authoring the test on the parked branch was also rejected. PRE-1
  `read_server_layout` is absent there, and the parked branch must stay
  unchanged.
* **C1-C3 are withdrawn.** C1 (the detached pre-PRE-3 checkout) no longer
  exists, so the attempt-5 C1 worktree conflict (P2-4) is resolved. C2's
  point (step (a) polls `_health` directly, before any barrier call) is
  kept as a diff-contract review check. C3 is kept as an optional
  task-record citation.

**L3-1 guard (replaces R6.6).** No F54 run is allowed on any tree that has
the F54 file but lacks the `ensure_daemon` barrier call. HC is a single
commit carrying both, and the cherry-picked commits never appear on the S4
branch as standalone commits. Running the target at HC is safe: the stub
fails closed inside `ensure_daemon`, so no side-effect window ever opens
against an activating daemon. Runs at IC use the real barrier.

#### R7.4 Baseline B0 without any checkout (replaces R6.3's B0 sources)

**A0 (Ship; an assembly-sequence step before H5).** The core worktree is
ALREADY on the parked branch, so no checkout is needed. Ship captures B0
there:

* **When:** after H1 and before H5 (the S1 branch change).
* **Precondition:** `git status --short` shows nothing under `src/`,
  `tests/`, `crates/`, `Cargo.toml`, `Cargo.lock`, `.cargo/`, or
  `build.rs`.
* **Run:** `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture`,
  with full output, in the per-row format (T2).
* **Record:** the OS, the HEAD SHA, and wall-clock time, as a comment on
  `142.063-T`.

There is no activator at that HEAD (G3), so B0 carries no L3-1 race.

**B1 rule** (restated from R6.3, with attempt-5 P2-1 and P3s absorbed):

* An IC run equals B0 on the SAME OS, modulo only these changes:
  * (i) rows whose text includes a `get_workspace_status` response may
    show a non-null `generation` block;
  * (ii) the `_shutdown` window may include the runtime-copy DB teardown
    at exit, identically in all three runs;
  * (iii) `get_workspace_statistics` rows may change signature;
  * (iv) the positive test is present and GREEN.
* B1 is compared per row. A case-level line may change only through an
  allowed row.
* **No change is expected from S2, S3, or unrelated `main` work.** Any
  other difference is a HALT to Stage, with an attribution record: the
  row, the old and new text, and the merged change suspected (for example,
  a new descriptor on `main`).

**If A0 is missed**, no checkout is used to recover it. The comparison
degrades to characterization:

* Stage triages each IC row signature against Ship's `6d216d19` record
  on `142.058-T`.
* The gate becomes: the unknown-methods test has its recorded status,
  the three IC runs are identical, every settle line is `Active`, and
  M12's final all-GREEN result holds.

The degradation is recorded in the task record.

#### R7.5 PA-5 edges and the F54 hold

**Edges (placeholders; applied at the PA-5 harvest, not at assembly):**

* **E6:** `142.058-T` depends on `<last PA-5 task>`.
* **E7:** `<first PA-5 task>` depends on `142.063-T`.

The PA-5 planning session owns the task IDs and their internal order. E6
and E7 are the only constraints Revisions 6-7 put on PA-5 placement. This
replaces R6.8's "harvested into S4 after `142.063-T` and before
`142.058.002-ST`" wording and the S4 order's bracketed note.

**Hold (F54 and S4):**

* S4 MAY be created at assembly with its known members.
* S4 MUST NOT be claimed, and HC (the F54 cherry-pick) MUST NOT be made,
  until three things are true:
  1. the PA-5 tasks exist;
  2. they are in S4's manifest;
  3. E6 and E7 are recorded.
* The Orchestrator enforces the hold. Assembly records it as a comment on
  `142.063-T` and `142.058-T`.

This prevents a RED interval with no owner on the S4 branch (attempt-5
Learnings P2). *(Revision 9: "and HC (the F54 cherry-pick) MUST NOT be
made" is withdrawn, because HC no longer touches F54. The full S4 claim
gate is R9.8.)*

#### R7.6 Assembly-sequence steps added (recorded; NOT done here)

| Step | Owner | Action |
|---|---|---|
| H1 limit | Orchestrator/Ship | The H1 planning commit contains NO file under `src/`, `tests/`, `crates/`, `scripts/` (so `scripts/__pycache__/` is excluded), `Cargo.toml`, `Cargo.lock`, `.cargo/`, `build.rs`, `start.ps1`, or `start.sh`. In the split memo, "harness files" means agent-harness (`.github/`) files, which are still included only if the operator agrees. |
| A0 | Ship | Capture B0 after H1 and before H5 (R7.4). |
| **T0: protected tag** | Orchestrator/Ship | BEFORE H2 (abandoning 142-S), create an annotated tag at the parked-branch tip after H1, named for example `parked/142-s-split-<H1-sha8>`. It must reach `a0ccdc27`, `c269fa79`, `41dd5081`, `1dfc1b5b`, `4995d681`, `5760b948`, `e24f5ae2`, `a47b8aff`, `7bd9e504`, and `6d216d19`. It is never moved or deleted until S4 ships. At the start of each shipment, Ship runs `git merge-base --is-ancestor <sha> <tag>` for each commit that shipment carries, and records the result. |
| Edges | Stage (assembly session) | E5, E1, E2, E3, E4, E8, in that order, with a cycle check after each (attempt-5 P2-3: E1 must precede E2, or a cycle 063→066→064→063 forms). |
| Item text | Stage (assembly session) | M1-M17 (R6.5, R7.1), T1-T16 (R7.2), the `142.066-T` inline rule (R7.7), and the `142.064.003-ST` / `142.066-T` S1 repeatability criterion (R7.8). |
| Closure | Ship | S1-S3 exclude `142-F` and close by the manual safe-close precedent (P-015). `142-F`'s queue file must be byte-identical before and after each close. Before S4 closes, Ship checks that every `142-F` descendant (including re-queued 142-S items, PA-5 tasks, and follow-ups) is done or archived. Any `backlogit shipment ship` call is time-bounded, with manual safe-close as the fallback (the attempt-5 Learnings P2 on cascade ship). |

*(Revision 9: the T0 row is replaced by R9.9 (tag at the A0R commit,
remote push and recovery check at H2). The Item-text row is extended by
R9.10, and T1-T16 are replaced by R9.6.)*

**E8 (added; attempt-5 Architecture P3):** `142.063-T` depends on
`142.060-T`. S4-after-S3 is then enforced by an edge, and not only by
shipment order. HC's `[[test]]` stanza context also follows S3's launcher
stanzas.

#### R7.7 The `142.066-T` inline two-observation rule, fully defined

This replaces R6.5's restatement.

* **What is compared:**
  * **F** is the tuple (`path`, `branch`, `db_path`, `generation`) from
    one `get_workspace_status` response.
  * **G** is that same response's `generation` block.
  * **S** is a filesystem snapshot of the fixture temp root. Every entry's
    relative path, kind, and length is included. `.db` and `.db-*`
    entries are compared by metadata only. A Windows sharing violation
    (error 33) on read is recorded as the entry's metadata, not skipped.
* **One check:** status call 1 (F1, G1), then S1, then sleep 250 ms, then
  S2, then status call 2 (F2, G2). There is no IPC call between S1 and S2.
* **Settled** means `G1.active_revision == G1.published_revision ==
  G2.active_revision == G2.published_revision == Some(2)`, and `S1 == S2`,
  and `F1 == F2`, each compared separately.
* **Polling:** re-poll every 25 ms until a 30 s deadline. An IPC error
  means re-poll. On the deadline, FAIL with the last G1/G2. Never pass on
  timeout.

The warm-up read and the sidecar HALT (attempt-4 P2-9) are unchanged.

#### R7.8 S1 careful-mode repeatability (attempt-5 Constitution P2)

This adds acceptance at assembly. It restores S1 repeat-run evidence
without the F54 file:

* `142.064.003-ST` (closing PRE-3) and `142.066-T` (PRE-4) each require
  three consecutive full-output runs of
  `integration_read_server_generation_wiring` after their build, with
  identical per-case maps.
* The same applies to each named consumer target.
* Any difference is a HALT.

**Risk (added):** a B0 → B1 difference found only in S4 may point to
production code that is already on `main`. PRE-3F cannot fix it, because
it is tests-only. In that case Stage opens a new S4 production task under
P-021, which needs deliberation, and S4 does not merge until that task is
done.

#### R7.9 Operator decisions needed before S3 merge (PA-4 and daily `start.ps1`)

**What changes for you.** Today `start.ps1` is fail-open. It pre-warms
engram and starts Copilot even if the pre-warm fails or times out. S3's
`142.055-T` (F51) replaces that on purpose. The launcher then starts
Copilot ONLY when `engram preflight` reports `Succeeded`. The fail-open
path is "removed, not merely lengthened", which is its acceptance.

This repo's engram runs in managed mode: `.engram/config.toml` has no
`mode` key. In managed mode, preflight fails closed at `DaemonVerified`
by design. **So from the moment S3 merges, `start.ps1` on this repo
prints the failing stage and does NOT start Copilot, every time, until
PA-4 is decided.** `start.sh` (F53) behaves the same way. The only manual
workaround is to start Copilot yourself with the `COPILOT_HOME` and
`ENGRAM_DATA_DIR` settings that `start.ps1` sets. That is not a
supported path.

**The options.** One decision is needed before S3 is CLAIMED, because
option C changes S3's work. The latest it can be made is before S3
MERGES.

| Option | What it means for you | Stage view |
|---|---|---|
| **A. Decide PA-4 = yes before S3 merges** (switch this repo to `mode = "read_server"`) | The launcher works on this repo. But read-server mode makes engram read-only for agents between launches: write and control tools are refused. For example, `index_workspace` and `sync_workspace` are `CapabilityClass::Write` with `read_server_available: false` (`src/tools/capabilities.rs:306-321`). The index is rebuilt by the preflight at every launch, with a cold full index (`F99C705E`). Disk use grows with every launch until retention lands (`23E287C6`). Until PA-5, some reads stay unpinned. | Viable only if you accept those trade-offs for daily work. |
| **B. Hold S3's merge until PA-4 is decided** (default if there is no answer) | Nothing breaks on `main`. S3 can be built and reviewed, and its PR waits. S4 waits behind S3, so 142-F closes later. | Safe default. It costs schedule, not stability. |
| **C. A mode-keyed legacy launcher path until PA-4** (the launcher reads the configured mode: managed runs today's pre-warm and starts Copilot with a loud "preflight skipped: managed mode" line; `read_server` runs strict preflight) | Daily `start.ps1` keeps working with no config change. Strict gating applies once you opt in. | This is a SCOPE CHANGE to `142.055-T`/`142.056-T`/`142.057-T`, whose RED harnesses (`1dfc1b5b`, `4995d681`, `5760b948`, `e24f5ae2`) assert fail-closed behavior. It needs a stash entry, a deliberation (P-021 C6), and a plan and review before S3 is claimed. The fallback is keyed on explicit config, never on a failure or timeout. That keeps it distinct from the fail-open path that F51 removes and NEW-6 supersedes. |

**Stage recommendation.**

* If you want daily `start.ps1` to keep working without adopting read-server
  mode yet, choose **C**, and ask Stage to stash and deliberate it before
  S3 is claimed.
* Otherwise choose **B**, and decide PA-4 on its own merits.
* Stage does not recommend A only to unblock the launcher.

This is recorded as **PA-6** (see the glossary). It is not granted, and no
task was created for it. *(Superseded by R8.6: PA-6 was decided "Hold S3"
on 2026-09-28. "Hold S3" holds the claim, which is Stage's reading and
is stricter than option B's "hold the merge". Enforcement is R9.8.)*

**Decision (Revision 8).** On 2026-09-28 at 16:48 -07:00 the operator
decided "PA-6: Hold S3". S3 is not claimed until the operator decides
PA-6 again. S1 and S2 are unaffected, and S4 stays blocked by S3. See
R8.6.

#### R7.10 Other attempt-5 findings

| Finding | Disposition |
|---|---|
| P2-1 baseline attribution | R7.4 (same-OS B0, no S2/S3 change expected, attribution record). The B0′ idea is not adopted, because it needs an older-commit checkout. |
| P2-2 PA-5 order / claim gate | R7.5 |
| P2-3 edge order | R7.6 |
| P2-4 C1 procedure | Withdrawn with C1 (R7.3) |
| P2-5 careful-mode gap | R7.8 |
| P2-6 inline rule | R7.7 |
| P2-7 leftover `142.064-T` text | M15 |
| P2-8 `142.060-T` text | M16 |
| P2-9 H1 content | R7.6 H1 limit |
| P2-10 interim main after S3 | R7.9 |
| P2-11 closure mechanics | R7.6 Closure |
| P2-12 safety ref | R7.6 T0 |
| P3s absorbed | S4→S3 edge (E8); single mandatory commit (HC); B0 OS; `_shutdown` identical across runs; per-row compare; `Settled` derives, `?`, C2 check; PA-5 placement stated once (R7.5); S1 F54 references (M17); `<pre-PRE-3>` (no longer used) |
| P3s not adopted | The `git grep` at H5 (T0's ancestry checks and HC's single commit cover the need); the loose shipment boundaries (accepted as operator-approved); a full `cargo dev-test` in S4 (it is already Ship's merge gate; M12 relies on it); TTL polling (the barrier ends before every window by construction; kept as a Ship note); deferring mechanics to Ship (noted) |

#### R7.11 Scope limits

No backlog item, edge, shipment, stash entry, source, test, or config file
was changed, and there was no git mutation. E5-E8, M13-M17, T1-T16, A0,
T0, and the H1 limit are all deferred to their owners. PA-5 is not
planned. PA-6 is recorded, not granted. *(Superseded by R8.6: PA-6 =
Hold S3.)*

### Revision 8: attempt-6 P1s, P2-1 to P2-3, and PA-6 = Hold S3

**Authorization basis.** The operator decided on 2026-09-28 at 16:48
-07:00: "1. Run revision 8  2. PA-6: Hold S3". The directive limits this
revision to:

* the two attempt-6 P1s: P1-D, the HC placeholder, and P1-E, test
  ownership;
* attempt-6 P2-1 to P2-3, which touch the same paragraphs;
* the `Cargo.toml` fact about `3f890662`;
* recording PA-6 = Hold S3.

After this revision comes ONE scoped review (attempt 7). If that review
finds any P0/P1, work STOPS: no Revision 9 and no assembly. The hard
bounds are unchanged: no backlog, edge, shipment, stash, source, test,
config, or git change.

**Precedence.** Revision 8 wins over Revisions 6 and 7 wherever they
disagree. Everything else in those revisions stands.

The source facts below were checked read-only on the parked branch at HEAD
`41dd5081`.

#### R8.1 The F54 target already exists on `main` (attempt-6 P2-2)

**Facts.**

* `3f890662` (`142.001-T`, F00) is an ancestor of `main`. It added two
  things:
  * the `Cargo.toml` stanza `[[test]] name = "contract_read_server_cli_mcp_parity"`
    with `path = "tests/contract/read_server_cli_mcp_parity_test.rs"`;
  * an inert placeholder file at that path, whose only test is
    `#[test] fn placeholder_registered() {}`.
* `7bd9e504` and `6d216d19` change ONLY the test file.
* None of the commits S1-S3 carry touches `Cargo.toml` or the F54 file:
  `a0ccdc27`, `c269fa79`, `41dd5081`, `1dfc1b5b`, `4995d681`, `5760b948`,
  `e24f5ae2` and `a47b8aff`.

**Consequences.** *(Revision 9: the HC bullets below, "HC makes NO
`Cargo.toml` change", "HC replaces the placeholder", "HC removes
`placeholder_registered`", and the `Cargo.toml` diff bullet, are withdrawn.
HC adds one `[[test]]` stanza for the new target (R9.3). F54 stays the
placeholder until `142.058-T`'s FL (R9.5). R8.2-R8.5 are replaced by R9.3
and R9.4.)*

* On the S1-S3 branches, `contract_read_server_cli_mcp_parity` is the
  GREEN placeholder. That placeholder starts no daemon, so L3-1 is
  unaffected. R6.1's "exists only on the parked branch" refers to the
  real test body only.
* **HC makes NO `Cargo.toml` change.** It replaces the placeholder file's
  whole content. Before the HC additions (R7.3 item 1), the content is
  byte-identical to the `6d216d19` blob. Ship chooses how to get it there,
  for example a no-commit cherry-pick of both SHAs, or taking the path
  from the T0 tag. The check is `git diff 6d216d19 HC --
  tests/contract/read_server_cli_mcp_parity_test.rs`: it must show only
  the HC additions.
* HC removes `placeholder_registered`. It is not a harness test. Its
  removal is recorded in the red-phase record, and it is not mapped.
* `git diff <S4 base> IC -- Cargo.toml` is empty.
* The PRE-3F unit's "There is no `Cargo.toml` edit" and `142.063-T`'s
  scope line "(…; no `Cargo.toml` edit)" are correct and stay.

**Text withdrawn.**

* R7.3 item 1, "the F54 file and its `[[test]]` … stanza, exactly as at
  `6d216d19`". It now reads: "the F54 test body, replacing the placeholder
  file; content as at `6d216d19`; no `Cargo.toml` change".
* T6, "HC adds the cherry-picked file and stanza". It now reads: "HC
  replaces the placeholder file's content with the `6d216d19` blob".
* R7.6's E8 note, "HC's `[[test]]` stanza context also follows S3's
  launcher stanzas". E8 itself stays, because it still enforces
  S4-after-S3.

#### R8.2 The HC placeholder body, pinned (attempt-6 P1-D)

`.cargo/config.toml` sets `rustflags = ["-Dwarnings"]`, so every rustc
warning is an error. The HC body of `await_activation_settled` is fixed
as follows. It replaces R7.3 item 1's "STUB body" description.

```rust
async fn await_activation_settled(&mut self) -> Result<Settled, String> {
    tokio::task::yield_now().await;
    Err(format!(
        "{F54_RED_MARKER}: activation-settle barrier not implemented (PRE-3F); \
         endpoint={}, timeout={SETTLE_TIMEOUT:?}, variants={:?}",
        self.endpoint,
        [Settled::NoActivator, Settled::Active],
    ))
}
```

Each part of the body clears a specific error:

| Warning or lint that would otherwise fail HC | What in the body clears it |
|---|---|
| `dead_code`: `SETTLE_TIMEOUT` never used | `{SETTLE_TIMEOUT:?}` |
| `dead_code`: variant never constructed | `[Settled::NoActivator, Settled::Active]` constructs both (the positive test also constructs `Active`) |
| `clippy::unused_async` (pedantic) | `tokio::task::yield_now().await` |
| `clippy::unused_self` (pedantic) | `self.endpoint`, the fixture's existing `endpoint: String` field |

The body also has these properties:

* It makes no IPC call and opens no window.
* It prints no `F54 settle:` line.
* It always returns `Err`.

**No lint attribute.** Neither HC nor IC may add `#[allow]`, `#[expect]`,
or `cfg_attr` for these items, or for anything else PRE-3F adds. IC
replaces this body wholesale with the real barrier, so T6's `git diff HC
IC` still shows only three things:

* the identity lines;
* their imports;
* the barrier body.

**Imports.**

* HC keeps `use engram::db::workspace::workspace_hash;` and adds no
  `read_server_layout` import.
* IC removes the first import (if unused) and adds the second.

**Mandatory gates on BOTH commits (T17 below).** At HC, and again at IC,
Ship or the harness architect runs:

1. `cargo check --all-targets`, which must exit 0;
2. `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`, which
   must exit 0.

The SHA, command, and exit code are recorded:

* for HC, in the red-phase record, before `harness-ready`;
* for IC, in the task record.

A failure at HC is NOT a red phase. It is a HALT to Stage. Ship does not
adjust the body or add an attribute on its own.

#### R8.3 Per-test ownership table (attempt-6 P1-E)

This table is authoritative. It covers every test function in the F54
file from HC onward, and each has exactly ONE owning task. B0 is the A0
capture of the `6d216d19` content (R7.4). At `6d216d19` the target
reported 1 passed and 4 failed. `ensure_daemon` line numbers are in the
`6d216d19` file.

| # | Test function | Calls `ensure_daemon`? | B0 | HC | After IC | Owner | Harness mapping |
|---|---|---|---|---|---|---|---|
| 1 | `generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces` | no (sync, pure matrix check) | GREEN | GREEN | GREEN | `142.058-T` (file owner) | none (GREEN regression) |
| 2 | `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` | yes (~1353-1356, first statement after `new()`) | RED | RED, R8.4 text | RED at its B0 signature, within B1 | `142.058-T` | `EXPECTED_PENDING_RED`, owner `142.058-T` |
| 3 | `control_descriptors_are_refused_without_side_effects` | yes (~1453-1456, before any exercise) | RED | RED, R8.4 text | RED at its B0 signature, within B1 | `142.058-T` | `EXPECTED_PENDING_RED`, owner `142.058-T` |
| 4 | `unknown_ipc_methods_are_refused_without_side_effects` | yes (~1484-1487; the helper call at ~1537 is not reached in HC) | RED | RED, R8.4 text | RED at its B0 signature, within B1 | `142.058-T` | `EXPECTED_PENDING_RED`, owner `142.058-T` |
| 5 | `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` | yes (~1572-1575) | RED | RED, R8.4 text | RED at its B0 signature, within B1 (GREEN only after PA-5, M12) | `142.058-T` | `EXPECTED_PENDING_RED`, owner `142.058-T` |
| 6 | `published_fixture_generation_activates_at_startup_and_quiesces` (new) | no (step (a) uses `spawn_daemon` and direct `_health`; step (b) calls `await_activation_settled` directly) | absent | RED at step (b), R8.4 text, after (a) passed | GREEN | **`142.063-T`** | `EXPECTED_PENDING_RED`, owner `142.063-T` |

**Rules.**

* **Ownership is unchanged for rows 2-5.** They stay with `142.058-T`
  throughout. HC changes only their temporary failure text, and IC
  restores their B0 signature. `unknown_ipc_methods_are_refused_without_side_effects`
  has one owner, `142.058-T`, and no mapping to `142.063-T`.
* **`142.063-T` owns only row 6.** It closes when row 6 is GREEN and rows
  1-5 match their "After IC" column. It never closes with a test it owns
  still RED.
* **A0 mismatch.** If A0 shows any of rows 1-5 in a different B0 status,
  Ship HALTs to Stage before HC, and Stage re-derives the table. Ship does
  not re-map tests on its own. One example is `unknown_ipc` being GREEN.
* **Placeholder test.** `placeholder_registered` exists only before HC
  (R8.1). It is not a row.

**Replacements.**

* **R7.3 item 2, mapping sentence.** It now reads: "It maps the tests per
  the R8.3 table."
* **R7.3 item 3, the "After IC" bullets.** They now read:
  * "row 6 is GREEN";
  * "rows 2-5 return to their B0 signatures within B1";
  * "row 1 stays GREEN".
* **T7.** It now reads: "`unknown_ipc_methods_are_refused_without_side_effects`
  is byte-identical, and the barrier runs before its
  `binding_before`/`files_before`. It stays owned by `142.058-T` (R8.3
  row 4). In HC it fails at its first `ensure_daemon` with the R8.4 text.
  After IC it returns to its B0 status and signature within B1."
* **T5.** It stands, and its owner `142.063-T` is correct for row 6.
* **T15.** It now reads: "HC red-phase record, containing:
  * SHA, command, full output, and OS;
  * the R8.2 check and clippy exit codes;
  * each R8.3 row's observed HC status and text, which must match the
    table;
  * the removal of `placeholder_registered`."

#### R8.4 Valid red-phase text (attempt-6 P2-1)

The existing wrappers turn a stub error into a panic. Rows 2-5 do this
through `unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: {error}"))`
(for example ~1356). So the expected HC message for rows 2-5 is:

`F54-BLOCK: F54-RED: activation-settle barrier not implemented (PRE-3F); endpoint=<…>, timeout=75s, variants=[NoActivator, Active]`

Row 6's step-(b) failure message carries `F54_RED_MARKER` (AC7), followed
by the same stub text.

**Rule.** An HC failure is valid RED only when its panic message contains
the exact substring `activation-settle barrier not implemented (PRE-3F)`.

* The `F54-BLOCK: ` prefix on rows 2-5 is allowed.
* The harness manifest classifies these failures as `EXPECTED_PENDING_RED`,
  never as BLOCK or infrastructure.

This replaces:

* R7.3's "NOT valid RED" bullet "any other marker or failure text";
* the words "with the stub marker" in R7.3 item 2 and T15.

#### R8.5 One consistent HALT rule for HC (attempt-6 P2-3)

R7.3's "NOT valid RED" list, T13's HC clause, and T15 had no carve-out for
a case whose status does not change. They are replaced by one rule:
**any row whose observed HC status or text differs from its R8.3 row is a
HALT to Stage.** This covers:

* a compile, check, or clippy failure (R8.2);
* a failure without the R8.4 substring;
* row 6 failing at step (a);
* row 1 failing;
* any row reaching a side-effect window.

T13's HC clause now reads: "or the HC red phase differs from the R8.3
table in any row (status or R8.4 text)". The Constitution "Revision 7"
row's "every pre-existing F54 case fails at `ensure_daemon`" now reads
"every pre-existing case that calls `ensure_daemon` (R8.3 rows 2-5)", as
the Revision 8 Constitution row records.

#### R8.6 PA-6 = Hold S3 (operator, 2026-09-28 16:48)

**Decision.** PA-6 is "Hold S3". This is attempt-6's redefined option B.
S3 is **not claimed** until the operator decides PA-6 again. That later
decision could be:

* option A (PA-4 read-server mode);
* option C (a config-keyed launcher path, through normal intake under
  P-021 C4, with the full scope listed in attempt-6 P3-8);
* releasing the hold.

**Effects.**

* **S1 and S2 are unaffected.** They may be created, claimed, built, and
  merged as planned.
* **S3 may be created at assembly as a queued shipment.** Creating it is
  not claiming it. S3 must not be claimed, and no S3 task may be started,
  while the hold stands.
* **S4 stays blocked by S3,** through E8 and the shipment order, so S4
  waits too. `142-F` does not close until PA-6 is decided again and S3
  ships.
* **Daily `start.ps1`/`start.sh` behavior on `main` does not change**
  while the hold stands, because the fail-closed launchers stay unmerged.

**Enforcement.**

* **The Orchestrator** will not route an S3 claim while the hold stands.
* **Ship** refuses to claim S3 unless a later PA-6 decision is recorded.
* **Dark-mode pre-authorization** cannot claim or merge S3 while it is
  held.
* **The assembly session** records the hold as a comment on the S3
  shipment, on `142.055-T`, and on `142.060-T`. This is a new R7.6
  assembly step. It is not done here. *(Revision 9: extended to every S3
  task, `142.055-T`-`142.057-T` and `142.060-T`, plus the S3 shipment,
  and S4 claims are refused too; R9.8.)*

R7.9 and the R6.2 shipment table are annotated with this decision.

#### R8.7 Text-change summary for assembly (additions to R7.6 "Item text")

| # | Target | Change |
|---|---|---|
| T5 | `142.063-T` AC7 | unchanged from R7.2 (owner `142.063-T` = row 6) |
| T6 | `142.063-T` AC8 | as R7.2, with the R8.1 wording ("replaces the placeholder file's content"; no `Cargo.toml` change) |
| T7 | `142.063-T` AC9 | replaced by R8.3's T7 text |
| T13 | `142.063-T` notes HALT | HC clause replaced by R8.5 |
| T15 | `142.063-T` verification 2 | replaced by R8.3's T15 text |
| **T17** | `142.063-T` AC10 "`cargo check --all-targets` passes, and root clippy (pedantic) is clean." | Replace with: "At BOTH HC and IC: `cargo check --all-targets` exits 0 and `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` exits 0, recorded with SHA and exit code. The HC barrier body is exactly the R8.2 body; no lint attribute in HC or IC. A failure at HC is a HALT, not a red phase." |
| **T18** | `142.063-T` scenarios / verification | Add the R8.3 table as the per-function red-phase contract. |
| PA-6 | S3 shipment, `142.055-T`, `142.060-T` | hold comment (R8.6) |

#### R8.8 Edges and scope limits

* **No edge changes.** The assembly order stays E5, E1, E2, E3, E4, E8,
  with a cycle check after each. E4 must precede E8. E6 and E7 are
  applied at the PA-5 harvest. The PA-6 hold is procedural and has no
  edge.
* No backlog item, edge, shipment, stash entry, source, test, or config
  file was changed, and there was no git mutation. The `git log`/`git
  show` calls above were read-only.
* Assembly is not done here.
* The other attempt-6 P2s (P2-4 to P2-10) and all its P3s stay open as
  recorded in Attempt 6. They are non-blocking.
  *(Revision 9, attempt-7 P3-4: Revision 8 in fact absorbed attempt-6
  P2-6 in part, P3-1 and P3-10, at Stage discretion. R9.11 records which
  further attempt-6 items Revision 9 absorbs.)*

### Revision 9: PA-7 target isolation, attempt-7 P2s, and T0 remote push

**Authorization basis.** On 2026-09-29 at 14:55 -07:00 the operator
decided, verbatim: "approve PA-7, Revision 9, push T0". That means:

1. **PA-7 is approved** with the exact wording in R9.1. This is an
   operator decision, not a Stage grant.
2. Stage writes this Revision 9 and runs **ONE** plan review scoped to it
   (attempt 8). Any P0/P1 STOPS the work: no Revision 10, and no assembly
   in this session.
3. **T0 durability route: push T0 to the remote under a parked name at
   H2**, before H2 completes, with a recovery check recorded (R9.9). This
   is a plan-text requirement. No tag is created or pushed in this
   session. The push runs later, at H2, under the normal gates.

The structural direction comes from the P-013.6 escalation review
`docs/decisions/2026-09-28-142-f-escalation-review.md` ("Minimum coherent
Revision 9 directive", items 1-4, and its "Attempt 7 P2 disposition"
table). The hard bounds are the same as Revisions 6-8. No backlog item,
edge, shipment, stash entry, source, test, `Cargo.toml`, config, or git
state is changed in this session.

**Precedence.** Revision 9 wins over Revisions 6-8 and over every unit,
graph, Constitution and Runtime Verification text wherever they disagree.
In particular it **withdraws** these parts:

* R7.3 (HC/IC on the F54 file, the stub in `ReadServerFixture`, and the
  R7.3 L3-1 guard wording);
* the R8.1 "Consequences" bullets about HC ("HC makes NO `Cargo.toml`
  change", "HC replaces the placeholder", "HC removes
  `placeholder_registered`", and "`git diff <S4 base> IC -- Cargo.toml` is
  empty"). The R8.1 **facts** stand;
* R8.2 (the `&mut self` stub body), R8.3 (the six-row table), R8.4 (the
  F54 red-phase text), and R8.5 (the HC HALT rule), which R9.3 and R9.4
  replace;
* R7.2 T1-T18 as item-text instructions, which R9.6 replaces (the
  disposition of each row is in R9.6);
* every "with the stub marker" wording (attempt-7 P3-1).

It **keeps** these parts unchanged:

* E1-E8 and their apply order (R9.10);
* S1-S4 membership and order;
* PA-6 = Hold S3 (R8.6, made enforceable in R9.8);
* A0 and B0 (R7.4; only the consumer and the record's location change,
  R9.5 and R9.9);
* R7.7, R7.8 and M15-M17.

#### R9.1 PA-7 (operator decision, 2026-09-29 14:55 -07:00)

The operator approved PA-7 exactly as worded in the escalation review:

> I confirm that my 2026-09-27 PA-1 approval retained the invariant-6
> PRE-3F file-edit permission; I withheld only admission of the
> harvested units to the old 142-S. I grant no waiver of Ship Step 4.3,
> P-002, or P-004. For the four-shipment plan, I authorize Stage to
> propose moving PRE-3F's positive test and activation-settle helper
> into a separate test target and support owned by 142.063-T, with
> necessary Cargo registration. The F54 test file, including its
> barrier call, identity change, and real-snapshot fixture change, is
> edited only by 142.058-T after PA-5; the historical cross-task edit
> permission is not to be exercised. Do not mutate items, edges,
> shipments, files, or git until the revised plan passes review and
> the normal owning workflow authorizes execution.
> PA-6 remains Hold S3.

**What it changes.**

* The invariant-6 cross-task edit permission is recorded as history and
  is **not exercised**. No task edits another task's Owned file.
* `142.063-T` owns a new test target, its test support, and one
  `Cargo.toml` stanza (R9.3).
* `142.058-T` is the only editor of
  `tests/contract/read_server_cli_mcp_parity_test.rs`, and only after the
  PA-5 tasks are done (R9.5).
* Ship Step 4.3, P-002 and P-004 apply without any waiver.

#### R9.2 The ownership boundary

Every file that S4's test work touches has exactly one owning task. No
file is edited by two tasks.

| Path | Owner | Edited when | State before the owner edits it |
|---|---|---|---|
| `tests/integration/read_server_activation_settle_test.rs` (new target `integration_read_server_activation_settle`) | `142.063-T` | HC (created), and it does not change at IC | absent |
| `tests/helpers/activation_settle.rs` (new shared test support) | `142.063-T` | HC (created with the pinned stub) and IC (barrier body) | absent |
| root `Cargo.toml`: one new `[[test]]` stanza | `142.063-T` | HC only | no stanza for the new target |
| `tests/contract/read_server_cli_mcp_parity_test.rs` (F54, target `contract_read_server_cli_mcp_parity`) | `142.058-T` | FL (R9.5), then `.002-ST` and `.003-ST` | the GREEN `3f890662` placeholder, byte-identical to the S4 base |

**Consequences.**

* **F54 stays the GREEN placeholder** (`placeholder_registered`) through
  S1-S3, PRE-3F and every PA-5 task. It starts no daemon, so L3-1 cannot
  arise before FL.
* **No task in S4 before `142.058-T` has a RED F54 test.** So no
  `EXPECTED_PENDING_RED` mapping onto `142.058-T` is ever needed, and
  Step 4.3 condition 4 is never tested against its file.
* **No review verdict may rely on a RED F54 file while its owner is
  pending.** A verdict for `142.063-T` or a PA-5 task that needs any F54
  test to be RED is a HALT to Stage.
* **The helper travels to F54 by inclusion, not by editing.** `142.058-T`
  includes `tests/helpers/activation_settle.rs` with `#[path]` and calls
  it. If F54 needs any change to that file, `142.058-T` HALTs to Stage for
  a new task. It does not edit the file.
* **PA-5 tasks edit neither file.** Each PA-5 task's RED harness travels
  with that task and lands in its own harness commit. No PA-5 harness is
  installed before PRE-3F's IC full-suite gate passes. *(Revision 10:
  this sentence is withdrawn. Ship Step 2 harnesses every S4 task up
  front. See R10.1.)*
* **Full-suite GREEN at every S4 gate (escalation item 4; Stage
  completion edit, attempt 8).** `142.063-T` (IC), every PA-5 task, and
  `142.058-T` each close with a Step 4.3 verdict of **PASS** on
  `cargo dev-test --no-fail-fast`. None of them closes with an
  `EXPECTED_PENDING_RED` mapping onto another S4 task. *(Revision 10:
  replaced by the R10.1 gate table. `142.063-T` IC and non-final PA-5
  tasks may close `EXPECTED_PENDING_RED` onto later PA-5 tasks.
  `142.058-T` is in S5.)*

#### R9.3 PRE-3F in its own target (replaces R7.3 and R8.1-R8.5 for PRE-3F)

**Files (all new, except the one `Cargo.toml` stanza).**

* **Target file:** `tests/integration/read_server_activation_settle_test.rs`.
* **Shared support:** `tests/helpers/activation_settle.rs`. It is included
  as `#[path = "../helpers/activation_settle.rs"] mod activation_settle;`,
  following the repo's `#[path]` helper convention (for example
  `tests/contract/mcp_catalog_oracle_test.rs`). It is NOT added to
  `tests/helpers/mod.rs`, so the `helpers_daemon_harness` target is
  unchanged.
* **`Cargo.toml` registration** (the one `Cargo.toml` change in
  `142.063-T`), placed immediately after the
  `contract_read_server_cli_mcp_parity` stanza:

  ```toml
  [[test]]
  name = "integration_read_server_activation_settle"
  path = "tests/integration/read_server_activation_settle_test.rs"
  ```

  The `integration_` prefix keeps the coverage oracle's surface globs
  matching, so the oracle manifest is not edited. The F54 stanza is not
  touched.

**Why the support is a separate file.** The F54 fixture's
`&mut ReadServerFixture` method cannot be imported from the private F54
test crate. A `#[path]` module can be included by both crates without
either one editing the other's file.

**Pinned support API (final from HC).** The six public items are fixed
by name and signature at HC and never change:

* `pub const SETTLE_TIMEOUT: Duration = Duration::from_secs(75);`
* `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Settled { NoActivator, Active }`
* `pub async fn send_ipc(endpoint: &str, method: &str) -> Result<IpcResponse, String>`.
  It sends one direct-IPC request with no params and a private 10 s
  `IPC_TIMEOUT`.
* `pub async fn binding_fingerprint(endpoint: &str) -> Option<Value>`.
  It returns the (`path`, `branch`, `db_path`, `generation`) tuple of one
  `get_workspace_status` response, as F54's
  `workspace_binding_fingerprint` does.
* `pub fn filesystem_snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String>`.
  It is a port of F54's `snapshot_directory` and `is_file_lock_error` at
  `6d216d19`, with the same keys and the same value encoding. Those two
  are private functions of the support file.
* `pub async fn await_activation_settled(endpoint: &str, snapshot_root: &Path, expected_revision: u64) -> Result<Settled, String>`.

Every public item has a doc comment. Every function that returns
`Result` has a `# Errors` section, so no pedantic doc lint can fire. The
three utilities (`send_ipc`, `binding_fingerprint`,
`filesystem_snapshot`) are complete at HC, because the positive test uses
them at HC. Only the barrier body is a stub.

**The pinned HC stub** (replaces R8.2):

```rust
/// Waits until the read server at `endpoint` has settled activation.
///
/// # Errors
///
/// Returns an error when the barrier cannot observe a settled state
/// before [`SETTLE_TIMEOUT`].
pub async fn await_activation_settled(
    endpoint: &str,
    snapshot_root: &Path,
    expected_revision: u64,
) -> Result<Settled, String> {
    tokio::task::yield_now().await;
    Err(format!(
        "activation-settle barrier not implemented (PRE-3F); endpoint={endpoint}, \
         root={}, revision={expected_revision}, timeout={SETTLE_TIMEOUT:?}, variants={:?}",
        snapshot_root.display(),
        [Settled::NoActivator, Settled::Active],
    ))
}
```

| Warning or lint that would otherwise fail HC | What clears it |
|---|---|
| `dead_code`: `SETTLE_TIMEOUT` unused | `{SETTLE_TIMEOUT:?}` |
| `dead_code`: a `Settled` variant never constructed | the `[Settled::NoActivator, Settled::Active]` array |
| `unused_variables`: any parameter | all three are formatted |
| `clippy::unused_async` (pedantic) | `tokio::task::yield_now().await` (`tokio` has `full`) |
| `clippy::missing_errors_doc` (pedantic) | the `# Errors` section |
| `dead_code`: `send_ipc`, `binding_fingerprint`, `filesystem_snapshot` | the positive test calls all three at HC |

There is no `self` parameter, so R8.2's `unused_self` concern does not
apply. The stub makes no IPC call, opens no window, and prints no
`F54 settle:` line.

**Why no includer leaves a dead item.** The IC body calls `send_ipc`,
`binding_fingerprint` and `filesystem_snapshot`, and uses both variants
and `SETTLE_TIMEOUT`. So any crate that calls `await_activation_settled`
uses every item of the support file. That is how F54 includes it at FL
with no lint attribute.

**The new target's own fixture (owned by `142.063-T`; private to the
target file).**

* `struct SettleFixture { temporary_root: TempDir, workspace: PathBuf, endpoint: String, daemon: Option<Child> }`,
  with `new`, `spawn_daemon`, `stop_daemon`, and `impl Drop`.
* Free functions `create_workspace` and `publish_fixture_generation`.
* `new` is a port of F54's `ReadServerFixture::new` at `6d216d19`, with
  the same temp-root overlap check, `.git/HEAD`, `mode = "read_server"`
  config, metrics file, source file and `code_graph::sync_workspace`
  seed. It omits the alternate workspace. So the daemon sees the same
  workspace shape that F54's primary workspace has.
* `publish_fixture_generation` publishes `generation-142`, revision 1,
  over the one-row `probe_row` database. From HC it stamps its identity
  from `read_server_layout` (PRE-1): `BranchIdentity::new(&layout.branch,
  None)` and `WorkspaceIdentity::new(layout.workspace_id.clone())`. It
  never calls `workspace_hash`. A layout error panics with
  `PRE3F-BLOCK`.
* **Process safety (Constitution VII).** The fixture holds the owned
  `std::process::Child` handle and signals only that handle. `stop_daemon`
  sends `_shutdown`, waits up to 3 s with `try_wait`, and then calls
  `kill` and `wait` on the handle. `Drop` kills and reaps only a
  still-running owned child. The fixture has no PID-marker or `sysinfo`
  layer, because it never signals a process by PID.
* Markers: `const PRE3F_RED_MARKER: &str = "PRE3F-RED";` and
  `const PRE3F_BLOCK_MARKER: &str = "PRE3F-BLOCK";`. The new target uses
  no `F54_*` marker.

**The positive test** `#[tokio::test] async fn published_fixture_generation_activates_at_startup_and_quiesces()`
keeps steps (a)-(e) of the PRE-3F unit, using the new fixture:

* **(a)** Spawn, then poll `_health` directly with `send_ipc` every 25 ms
  (30 s deadline). At the FIRST `ready`, `get_workspace_status` must have a
  non-null `generation`. Otherwise it panics with
  `PRE3F-RED: (a) generation not installed before readiness (PRE-3)`.
* **(b)** `activation_settle::await_activation_settled(&fixture.endpoint, fixture.temporary_root.path(), 1)`
  must return `Ok(Settled::Active)`. An `Err` panics with
  `PRE3F-RED: (b) activation did not settle: <error>`.
* **(c)** The same contents checks as before, and the runtime copy file
  exists.
* **(d)** Quiescence through `binding_fingerprint` and
  `filesystem_snapshot`: F1, S1, a 4.5 s wait, S2, F2, with no IPC call
  between S1 and S2.
* **(e)** `stop_daemon`.

**Bounded private helpers (attempt-7 P2-1).** The positive test may be
split into at most **four** private helper functions in the target file,
so that `clippy::too_many_lines` (pedantic) cannot fire. Each helper is
called only by the positive test or by another such helper. They are
written at HC and stay byte-identical at IC. No lint attribute (`#[allow]`,
`#[expect]`, or `cfg_attr` carrying one) is allowed anywhere in the three
files. `#[path]`, `#[cfg]`, `#[derive]` and `#[tokio::test]` are not lint
attributes.

**HC, the harness commit** (harness-architect, BEFORE `harness_status:
harness-ready`). One new commit on the S4 branch. It contains the new
target file in its final form, the support file with the three final
utilities and the pinned stub, and the `Cargo.toml` stanza. Nothing else.

**HC red-phase record** (before `harness-ready`; P-002/P-004). It records
the SHA, the OS, and each command with its exit code:

1. `cargo check --all-targets`: exit 0.
2. `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: exit 0.
3. `cargo lint` (adds `--all-features`) and `cargo fmt-check`: exit 0
   (attempt-7 P3-3).
4. `cargo test --test integration_read_server_activation_settle -- --nocapture`,
   with the full output kept: exactly one test runs, and it fails.
5. `cargo dev-test --no-fail-fast`: non-zero, and its ONLY failure is that
   test. F54's `placeholder_registered` and every other target are GREEN.
   *(Revision 10: restated in R10.1 to also accept the recorded RED tests
   of S4 harness commits already on the branch.)*

**Valid RED (replaces R8.4).** The single failure is valid RED only when
its panic message contains, exactly:

`PRE3F-RED: (b) activation did not settle: activation-settle barrier not implemented (PRE-3F); endpoint=`

followed by the stub's remaining fields
(`root=<…>, revision=1, timeout=75s, variants=[NoActivator, Active]`).
That shows step (a) passed. The manifest records `Red Phase: CONFIRMED
(HC <sha>)`, with the commands, the full-output location and the OS.
This is a red-phase record only. It is not an `EXPECTED_PENDING_RED`
mapping, because IC must be PASS.

**HC HALT rule (replaces R8.5).** Any of the following is a HALT to
Stage, not a red phase:

* any gate in items 1-3 exits non-zero;
* the failing test's message lacks the exact substring above, or it
  fails at (a), (c), (d) or (e);
* the full suite has any other failure, compile error or warning;
* any `PRE3F-BLOCK:` line (including from the `Drop` path) or any
  `WARNING:` line appears in the output (attempt-7 P3-12).

**No-adjust rule (attempt-7 P3-10).** The harness-architect may fix
compile errors in the fixture and test code it authors. But neither Ship
nor the harness-architect skill (whose Step 5.1 says "fix the harness
until it compiles") may change the pinned API, the stub body, the valid
RED text, or add any lint attribute. If the gates cannot pass without
one of those, that is a HALT to Stage.

**IC, the implementation commit** (Ship build). IC replaces only the stub
body of `await_activation_settled` with the real barrier. It may add
private functions used only by that body, and their import lines. The
barrier rules are the existing `142.063-T` rules, applied to the
parameters:

* Poll `get_workspace_status` through `send_ipc` every 25 ms until
  `SETTLE_TIMEOUT`. An IPC error or an unbound socket re-polls.
* A `null` `generation` block gives `Ok(Settled::NoActivator)`.
* `Active` needs two observations 250 ms apart, with no IPC call between
  S1 and S2: `G1.active_revision == G1.published_revision ==
  G2.active_revision == G2.published_revision == Some(expected_revision)`,
  `S1 == S2` over `filesystem_snapshot(snapshot_root)`, and `F1 == F2`
  over `binding_fingerprint(endpoint)`.
* Each `Ok` return prints one stderr line
  `F54 settle: <NoActivator|Active> after <ms> ms`. The historical prefix
  is kept so that M1, M12 and T13's greps stay valid.
* A timeout returns `Err` with the last `generation` block. It never
  returns `Ok` on timeout.

`expected_revision` replaces the hard-coded `Some(1)`, so a later fixture
can use the helper without editing it. Both current callers pass 1.

**IC gates** (Step 4.3 verdict must be **PASS**; *Revision 10: `PASS` or
`EXPECTED_PENDING_RED` onto PA-5 tasks only, R10.1*):

* items 1-3 above, exit 0;
* `cargo dev-test --no-fail-fast` passes with no failure, compile error or
  warning;
* three consecutive
  `cargo test --test integration_read_server_activation_settle -- --nocapture`
  runs, each with full output and wall-clock time recorded. Each run is
  GREEN, has one `F54 settle: Active` line, and has no `PRE3F-BLOCK:` or
  `WARNING:` line;
* `pwsh scripts/test-coverage-oracle.ps1 --mode completeness` exits 0;
* `git diff HC IC` touches only `tests/helpers/activation_settle.rs`, and
  within it only the barrier body, its private functions, and imports;
* `git diff <S4 base> IC -- tests/contract/read_server_cli_mcp_parity_test.rs`
  is empty;
* the characterization record (not a gate): whether snapshot, then
  `get_workspace_status`, then snapshot is stable on the S4 branch.

**Two-hour check.** `142.063-T` is tests-only and changes no production
file. The HC work is ported, reviewed F54 fixture code (about 250-300
lines), three small utilities and a pinned stub. The IC work is one
function body, plus at most two private functions, which is under the
five-function limit. Stage estimates about 75 minutes for HC and 40
minutes for IC, so **it fits the 2-hour limit at size M (was S)**,
complexity medium. If HC needs more than the four helpers, more than two
IC private functions, or any file beyond the three, that is a HALT to
Stage for a replan of the test support. The two-owner overlap is not
restored.

#### R9.4 Per-target ownership table (replaces R8.3)

**Target A: `integration_read_server_activation_settle`** (new; owner
`142.063-T`).

| Test | S4 base | HC | After IC | Through PA-5 and F54 | Mapping |
|---|---|---|---|---|---|
| `published_fixture_generation_activates_at_startup_and_quiesces` | absent | RED at (b), with the R9.3 text, after (a) passed | GREEN | GREEN (a regression is a HALT for the task that caused it) | red-phase record only; no `EXPECTED_PENDING_RED` |

**Target B: `contract_read_server_cli_mcp_parity`** (existing; owner
`142.058-T` only). B0 is the A0 capture of the `6d216d19` content (1
passed, 4 failed).

| Test | B0 | S1-S3, PRE-3F, PA-5 | FL runs (R9.5) | At `142.058-T` close | Mapping |
|---|---|---|---|---|---|
| `placeholder_registered` | n/a | GREEN (the `3f890662` placeholder, byte-identical) | removed at FL, recorded | absent | none |
| 1 `generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces` | GREEN | absent | GREEN | GREEN | none |
| 2 `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` | RED | absent | RED at the B0 signature, within B2 | GREEN | none |
| 3 `control_descriptors_are_refused_without_side_effects` | RED | absent | as row 2 | GREEN | none |
| 4 `unknown_ipc_methods_are_refused_without_side_effects` | RED | absent | as row 2 | GREEN | none |
| 5 `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` | RED | absent | as row 2 | GREEN (M12) | none |

**Rules.**

* No F54 row is RED on any tree before FL, so no task maps a failure to
  `142.058-T`.
* Rows 2-5 are RED only inside `142.058-T`'s own work, between FL and its
  close. `142.058-T` closes with Step 4.3 **PASS**.
* **A0 mismatch.** If A0 shows rows 1-5 with a different B0 status, Ship
  records it and HALTs to Stage before FL. It is not a blocker for S1-S3
  or PRE-3F, because none of them runs F54's real body.

#### R9.5 F54 (`142.058-T`) after PA-5, and moved evidence

*Revision 11 note: item 1 adds check (d2), and `142.058-T` keeps its
label through the reset (R11.6 P2-11, R11.2, R11.8).*

**Order inside `142.058-T`** (after E3 `142.063-T` and E6 `<last PA-5
task>` are done): provenance check → FL → three FL runs (B2) → `.002-ST`
(including the real-snapshot fixture) → `.003-ST` → M12 → Step 4.3 PASS.

**1. Provenance revalidation before FL** (escalation item 2; HALT if it
does not qualify). *(Revision 10: this runs at S5 Step 2, and (b) and (c)
are amended in R10.2.)* Ship records each item:

* (a) T0 reaches `7bd9e504` and `6d216d19`, both locally
  (`merge-base --is-ancestor`) and on the remote (the R9.9 recovery
  check).
* (b) The harness record exists and is unchanged: the `harness-ready`
  label, the `7bd9e504` red-phase record (`not implemented: Worker:
  F54`), the done `.001-ST` (`6d216d19`) and its per-case record, and B0.
* (c) No other task has edited the F54 file on the S4 branch:
  `git log <S4 base>..HEAD -- tests/contract/read_server_cli_mcp_parity_test.rs`
  is empty.
* (d) The `get_retrieval_eval_report` CLI-absence assertion that the
  2026-09-24 Stage amendment asked Ship to remove is absent from the
  `6d216d19` blob. Stage checked this read-only: the blob has only
  `supports(ToolSurface::Cli)` checks, not a `!…contains(&ToolSurface::Cli)`
  assertion. Ship re-greps and records the result.

If any item fails, or if (e) below fails, **the old `harness-ready`
record does not qualify.** Ship HALTs to Stage (P-002/P-004). Ship does
not re-label, re-record, or edit the harness to make it qualify.

**2. FL, the F54 load commit** (one commit, by `142.058-T`). It contains
exactly these changes:

* the F54 file content set to the `6d216d19` blob (the real body);
* `#[path = "../helpers/activation_settle.rs"] mod activation_settle;`;
* exactly one
  `activation_settle::await_activation_settled(&self.endpoint, self.temporary_root.path(), 1).await?;`
  at `ensure_daemon`'s ready-return point, before `return Ok(())`;
* the identity fix in `publish_fixture_generation` (`read_server_layout`
  replaces `workspace_hash`, whose import is removed if unused). The
  database bytes, generation ID, revision, digest, store root and
  producer are unchanged.

Everything else is byte-identical to `6d216d19`, including every test
body. `git diff 6d216d19 FL -- <file>` shows only those lines. FL makes
no `Cargo.toml` change. The L3-1 guard (it replaces R6.6 and R7.3): the
real body never exists on a tree without the barrier call, because FL
adds both in the same commit.

**3. Three FL runs and the B2 rule.** Three consecutive
`cargo test --test contract_read_server_cli_mcp_parity -- --nocapture`
runs, each with full output and wall-clock time recorded. They must give
identical per-case maps (per-row signature format, T2 definition). Every
`F54 settle:` line must be `Active` (M1). They are compared to B0 under
**B2** (it replaces B1 for F54). B2 is B0, on the same OS, modulo only:

* (i) a non-null `generation` block in rows whose text includes a
  `get_workspace_status` response;
* (ii) the runtime-copy teardown in the `_shutdown` window, identical in
  all three runs;
* (iii) signature changes in `get_workspace_statistics` rows;
* (v) **legitimate `main` drift** (attempt-6 P2-5): rows for descriptors
  added or changed after A0, each attributed to a merged change by the
  S4-claim drift ledger (R9.9) and judged against its expected-outcome
  class, not against B0;
* (vi) **PA-5 changes**: row changes that the reviewed PA-5 plan
  enumerates in advance for the `probe_row` fixture, each attributed to a
  PA-5 task ID.

B1 item (iv) (the positive test) no longer applies, because that test is
in target A. The `F54 settle:` stderr lines are excluded from row
signatures (attempt-6 P3-2). Any other difference is a HALT to Stage,
with an attribution record.

* **(e) Red re-confirmation.** The FL runs must show rows 2-5 RED at
  their B2 signatures and row 1 GREEN, on compiling code, before any
  `.002-ST` edit. This re-confirms the red phase on the current branch.
  If no row is RED, or a row is RED for any other reason, provenance
  fails (item 1).
* **B2 is same-OS only** (attempt-6 P2-10). Runs on other OSes, such as
  Linux CI, are gated by M12 (every case GREEN, three identical runs,
  every settle line `Active`). A divergence that appears only there is a
  HALT.
* **If A0 was missed,** R7.4's characterization fallback applies
  unchanged at FL.

**4. The real-snapshot fixture change** belongs to `142.058-T`
(`.002-ST`). It follows the fixture specification in the reviewed PA-5
plan. PA-5 tasks do not edit the F54 file. If the new fixture publishes a
different revision, `.002-ST` changes the call's `expected_revision`
argument in the same commit. That is `142.058-T`'s own file.

**5. Close.** M12, then the Step 4.3 verdict for `142.058-T`: `cargo
dev-test --no-fail-fast` PASS, plus lint and format. There is no
`EXPECTED_PENDING_RED` for `142.058-T`. If Ship takes a per-subtask
verdict at `.002-ST`, a non-green result there is a HALT to Stage. It is
not mapped onto the sibling `.003-ST`.

**6. History is kept.** `.001-ST` stays done and is not reopened. The
`7bd9e504` and `6d216d19` records and B0 stay in the task record. FL and
the later runs add records and replace none.

**Moved evidence (owners change; replaces the owner column of R6.5 and
R7.1 where they disagree).**

| Row | New owner and where it is proved |
|---|---|
| M1 | Split. `142.063-T`: the IC three-run `F54 settle: Active` lines in target A. `142.058-T`: the FL runs and M12 in F54. |
| M2, M7, M9, M13, M14 | `142.058-T`, at FL, under B2 |
| M3, M10 | `142.058-T`: the three FL runs cover PRE-3, PRE-4b and PRE-4 together |
| M5, M6 (F54 part), M11, M15 (F54 part) | `142.058-T` verification |
| M4, M8 | unchanged (dropped from S1). B0's consumer is now `142.058-T`. |
| M12 | `142.058-T`, reworded: "After the PA-5 handler changes and `142.058-T`'s real-snapshot fixture change land, three consecutive identical F54 runs, every `F54 settle:` line `Active`, and every case GREEN." |

#### R9.6 `142.063-T` item text (wholesale replacement at assembly)

At assembly, Stage replaces `142.063-T`'s description, acceptance
criteria, implementation notes and verification with the text below.
The title becomes "PRE-3F: Add shared activation-settle barrier and
positive witness target". The label `pending-pa-1` is removed. The
escalation review is added to `references`.

**Description.**

* Plan unit PRE-3F (plan Revisions 6-9; R9.3 is authoritative).
  Execution posture: test-first (HC red phase, then IC). Domain: tests
  only; no production file changes. Placement: first code task of S4.
* Owned files: `tests/integration/read_server_activation_settle_test.rs`
  (new target `integration_read_server_activation_settle`),
  `tests/helpers/activation_settle.rs` (new shared support), and one new
  `[[test]]` stanza in root `Cargo.toml`. This task does not edit
  `tests/contract/read_server_cli_mcp_parity_test.rs` (PA-7). That file
  stays `142.058-T`'s and stays the GREEN placeholder through this task.
* Scope: (1) the pinned support API and HC stub; (2) the target's own
  fixture with layout identity and handle-only process control; (3) the
  positive test (a)-(e); (4) the real barrier at IC.
* Test scenarios (2): (1) the HC red phase: one failure at (b), with the
  R9.3 text; (2) GREEN at IC: three identical runs, and a PASS full suite.

**Acceptance criteria.**

1. The R9.3 file set and `Cargo.toml` stanza, with no other path changed
   (diff contract).
2. The pinned support API and the pinned HC stub, verbatim. No lint
   attribute in any of the three files.
3. The target's own fixture as specified in R9.3, including the
   identity from `read_server_layout`, and signalling only the owned
   `Child` handle.
4. The positive test (a)-(e) with the R9.3 messages, and at most four
   private helpers, written at HC and unchanged at IC.
5. The HC red-phase record (R9.3 items 1-5, and the valid-RED text),
   recorded before `harness-ready`.
6. The IC barrier rules (R9.3), including re-poll on IPC error, never
   `Ok` on timeout, and one `F54 settle:` line per `Ok` return.
7. The IC gates (R9.3), with Step 4.3 verdict PASS. Three identical
   target runs, each with `F54 settle: Active`.
8. `git diff HC IC` touches only the barrier body, its private functions
   and imports in the support file. The F54 file is unchanged from the
   S4 base.

**Implementation notes.**

* Size: M | Complexity: medium | size_source: agent |
  size_ruleset_version: engram-stage-2h-rule-v1. Recorded as prose.
* Depends on: `142.066-T` PRE-4 (E2), `142.060-T` (E8), and `142.062-T`
  (kept, implied). Downstream: the first PA-5 task (E7) and `142.058-T`
  (E3).
* Why after PRE-4: step (a) witnesses PRE-3's install-before-ready, and
  the fixture needs PRE-1's `read_server_layout`.
* HALT to Stage: any R9.3 HC HALT condition; an IC gate failure that the
  barrier body cannot fix inside the diff contract; the fixture's
  identity cannot be expressed through `read_server_layout` without
  changing the fixture database or generation ID; the stability rule
  cannot be met without an IPC call between compared snapshots; or the
  two-hour bounds in R9.3 are exceeded.
* Learnings to read: `docs/compound/test-probe-resets-idle-ttl-livelock-2026-09-04.md`
  and `docs/compound/concurrency-issues/cozo-sqlite-busy-locked-reopen-panic-catch-unwind-2026-07-15.md`.

**Verification.**

1. The HC red-phase record (SHA, OS, commands, exit codes, full output).
2. The IC record: gates, three target runs, the full-suite PASS, the
   oracle completeness check, and both `git diff` checks.
3. The characterization note (not a gate).

**T1-T18 disposition.**

| Row | Revision 9 disposition |
|---|---|
| T1 (AC1 PA-1) | withdrawn. PA-7 replaces it; the historical permission is not exercised. |
| T2 (per-row signature) | the signature DEFINITION moves to `142.058-T` (V4); the rest is withdrawn |
| T3 (stability characterization) | kept, as R9.6 verification 3 |
| T4 (settle-line variants) | kept, inside AC6 |
| T5 (positive test, red phase) | replaced by AC4-AC5 |
| T6 (F54 diff contract) | replaced by AC1 and AC8 |
| T7 (`unknown_ipc`) | withdrawn from `142.063-T` (F54 row 4 belongs to `142.058-T`, R9.4) |
| T8 (scenarios) | replaced by the Description's test scenarios |
| T9 (item 3 RED at (b)) | replaced by AC4-AC5 |
| T10 (ownership exception) | withdrawn (PA-7) |
| T11 (depends on) | kept, in the notes |
| T12 (why after PRE-4) | replaced by the notes |
| T13 (HALT) | replaced by the notes' HALT list |
| T14, T15 (verification) | replaced by verification 1-2 |
| T16 (label) | the label is removed at assembly |
| T17 (gates at HC and IC) | kept, as R9.3 items 1-3 at both HC and IC |
| T18 (per-test table) | replaced by the R9.4 target-A row |

The queue's AC6 (barrier placement in `ensure_daemon`) moves to
`142.058-T` (V3).

#### R9.7 `142.058-T` item text (additions at assembly, V1-V8)

| # | Target | Change |
|---|---|---|
| V1 | Notes | Add: "Sole editor of `tests/contract/read_server_cli_mcp_parity_test.rs` (PA-7). Edits start only after `142.063-T` (E3) and the last PA-5 task (E6). It includes `tests/helpers/activation_settle.rs`, owned by `142.063-T`, via `#[path]` and never edits it; a needed change there is a HALT to Stage." |
| V2 | Acceptance (new, first) | The provenance revalidation, R9.5 item 1 (a)-(e), recorded before FL. Any failure is a HALT; no re-label. |
| V3 | Acceptance (new) | The FL commit, R9.5 item 2, including the `ensure_daemon` placement from `142.063-T`'s old AC6: exactly once per successful `ensure_daemon` call, at its ready-return point, on every call including after a respawn, and before any caller captures `binding_before` or `files_before`. |
| V4 | Acceptance (new) | Three FL runs compared with B0 under B2 (R9.5 item 3), using the per-row signature definition from T2. Owner of M1-M3, M7, M9, M10, M13 and M14. |
| V5 | `.002-ST` acceptance | Add: "The real-snapshot fixture change specified by the reviewed PA-5 plan, in this file only." |
| V6 | Acceptance (new, last) | M12 (the R9.5 wording) and Step 4.3 PASS. No `EXPECTED_PENDING_RED` for this task or its subtasks. |
| V7 | Notes | Add: "History kept: `.001-ST` done; the `7bd9e504` and `6d216d19` records and B0 stay; later records are added, not substituted." |
| V8 | Verification | Replace with: "1. Provenance record (V2). 2. The FL record and three FL maps with the B2 comparison. 3. The M12 maps. 4. `cargo dev-test --no-fail-fast` PASS, lint and format, then `cargo ci`." |

#### R9.8 PA-6 hold enforcement (attempt-7 P2-2 and P2-3)

PA-6 = Hold S3 is unchanged. It is a **claim gate**, not a status or edge
workaround.

* **Orchestrator.** Queue selection skips S3 while the hold stands. It
  also skips S4, which depends on S3 through E8 and the shipment order.
  It may route an unrelated shipment only under P-001's normal rules.
  This is not a new exception.
* **Ship.** Ship refuses to claim S3, and refuses to claim S4, while the
  hold stands. Ship also refuses to start any S3 task that is
  individually ready. For example, F53 (`142.057-T`) becomes ready when
  S2 ships.
* **Dark mode (P-017).** Any `DARK_MODE_SCOPE` excludes S3 and S4. Dark
  mode can neither claim nor merge them.
* **Release.** The hold is lifted only by a **later** recorded PA-6
  decision (option A, option C through P-021 C4 intake, or a lift), plus a
  P-014 approval that cites that decision. Stage does not lift it.
  *(Revision 10: the P-014 clause is withdrawn; a later PA-6 decision
  alone lifts the hold. Every "S3 and S4" in this section reads "S3, S4
  and S5". See R10.3 and R10.4.)*
* **Assembly (new R7.6 step).** The assembly session records the hold as
  a comment on **every** S3 task (`142.055-T`, `142.056-T`, `142.057-T`,
  `142.060-T`) and on the S3 shipment. It does not rely on one comment.
  The existing S4 hold comments on `142.063-T` and `142.058-T` (R7.5)
  stay.
* **S4 claim gate (R7.5, restated).** S4 is not claimed until all of
  these hold: S3 has shipped; the PA-5 tasks exist and are in S4's
  manifest; E6 and E7 are recorded; the R9.9 remote T0 check passes; and
  the R9.9 drift ledger is recorded. "HC (the F54 cherry-pick) MUST NOT be
  made" is withdrawn, because HC no longer touches F54.

#### R9.9 T0 durability: remote push at H2, and the drift ledger (attempt-7 P2-4)

**A0R, the B0 record commit** (Ship, after A0 and before T0). This also
resolves attempt-6 P2-8.

* Ship writes the B0 per-row map and the complete, untruncated A0 output
  into one markdown file,
  `docs/memory/<A0 date>-ship-142-f-a0-b0.md`, with the output in a fenced
  `text` block.
* Ship commits only that file on the parked branch. The H1 limit applies.
* The A0 precondition adds `git worktree list --porcelain`, which must
  show only the core worktree.
* R7.4's "comment on `142.063-T`" is withdrawn. The assembly session
  records the B0 path as a comment on `142.058-T`.

**T0 (replaces the R7.6 T0 row).** Before H2, create the annotated tag
`parked/142-s-split-<A0R sha8>` at the A0R commit. It must reach the ten
parked SHAs (R7.6), and `git cat-file -e <T0>:<B0 path>` must exit 0.

**H2 push and recovery check (operator route, 2026-09-29).** These steps
run inside H2, before 142-S is abandoned:

1. Ship checks `.github/workflows/` for any tag trigger that matches
   `parked/`. Today only `release.yml` has one, and it matches
   `v[0-9]*.[0-9]*.[0-9]*`. Any match is a HALT.
2. `git push origin refs/tags/parked/142-s-split-<A0R sha8>`: the tag
   only, with no branch, no `--force`, and no `--no-verify` (P-019). If an
   enabled pre-push hook blocks it, that is a HALT to the operator. It is
   not bypassed.
3. **Recovery check:** `git ls-remote origin refs/tags/<T0>` and
   `refs/tags/<T0>^{}`. The tag-object SHA must equal
   `git rev-parse <T0>`, and the peeled SHA must equal
   `git rev-parse <T0>^{commit}`.
4. The H2 session memory records the remote URL, the ref, both SHAs, the
   B0 path, and the command outputs. The assembly session copies the
   location to the S4 shipment and `142.058-T`.
5. Only then does H2 abandon 142-S. If any step fails, H2 stops and 142-S
   is not abandoned. There is no fallback to a bundle without a new
   operator decision.

**While T0 lives.**

* At each shipment claim, Ship re-runs step 3 against the remote, in
  addition to the R7.6 local ancestry checks.
* The tag is never moved, re-pushed with force, or deleted, locally or
  remotely, until S4 ships. Deleting it afterwards is a VII destructive
  action that needs operator approval (attempt-6 P3-12).

**Drift ledger (before the S4 claim; attempt-6 P2-5).** *(Revision 10:
before the S5 claim, against `<S5 base>`; T0 is kept until S5 ships.
See R10.2.)* Ship records
`git log --oneline <A0 HEAD>..<S4 base> -- src/tools/capabilities.rs` as
the ledger that B2 item (v) uses. Each descriptor change in it must name
its merged change (commit and PR). A change that cannot be attributed is a
HALT to Stage before S4 is claimed.

#### R9.10 Edges and the assembly sequence

* **No edge changes.** Assembly applies E5, E1, E2, E3, E4, E8, with a
  cycle check after each, and E4 before E8. E6 and E7 are applied at the
  PA-5 harvest.
* E3 (`142.058-T` depends on `142.063-T`) is still needed: F54 now
  consumes `142.063-T`'s helper.
* **Additions to R7.6** (recorded; NOT done here):
  * A0R and the H2 push and recovery check (R9.9);
  * "Item text" now lists M1-M17 with the R9.5 owners, the `142.063-T`
    wholesale replacement (R9.6), V1-V8 (R9.7), the `142.066-T` inline
    rule (R7.7), and the S1 repeatability criterion (R7.8)
    (attempt-7 P3-6);
  * the hold comments (R9.8);
  * the drift ledger before the S4 claim (R9.9).

#### R9.11 Attempt-7 P3s and other absorbed findings (Stage discretion)

| Finding | Disposition |
|---|---|
| P3-1 "with the stub marker" | withdrawn by R9 precedence; stale sites annotated |
| P3-2 row-6 message | pinned (R9.3 valid-RED text) |
| P3-3 `cargo lint`/`cargo fmt-check` | recorded at HC and IC (R9.3) |
| P3-4 R8.8 absorbed list | annotated in R8.8 |
| P3-5 stale PA-6 text | R7.9 "not granted" and R7.11 "not granted" annotated as superseded by R8.6; "hold claim" is Stage's reading of "Hold S3" |
| P3-6 R7.6 item list | R9.10 |
| P3-7 inexact T-row targets | moot: R9.6 replaces the whole item text |
| P3-8 R6.2 stale lines | annotated (S1-S4 table note and the inter-shipment blocks) |
| P3-9 Dependency Graph note | added (Revision 9 note) |
| P3-10 harness-architect no-adjust | R9.3 no-adjust rule |
| P3-11 PA-6 filed under IX | the Revision 9 Constitution row files it under P-001/P-014/P-017 |
| P3-12 `F54-BLOCK`/`WARNING:` lines | R9.3 HC HALT rule (`PRE3F-BLOCK:` and `WARNING:`) |
| Attempt-6 P2-5, P2-7, P2-8, P2-10, P3-2, P3-12, P3-16 | absorbed (R9.5 B2 (v), R9.3 full-suite record, R9.9 A0R, R9.5 same-OS rule, B2 settle-line exclusion, R9.9 VII note, the Revision 9 Constitution row) |
| Attempt-6 P2-4 (R7.7 snapshot contents) and P2-9 (closure mechanics) | still open; not touched by this revision |
| Stale-site annotations (attempt-8 completion check) | Revision 9 notes added at R6.2, R6.3 "Unchanged in PRE-3F", R6.6, R6.8, R7.3, R7.5, R7.6, R7.9, R7.11, R8.1, R8.6, the PRE-3F unit override, the Dependency Graph, the Constitution Check (Revision 9 row), and the Runtime Verification PRE-3F and `142.058-T` rows |

#### R9.12 Scope limits

* This session changed only this plan, `docs/operator-glossary.md`, and
  the Revision 9 memory file.
* No backlog item, edge, shipment, stash entry, source, test,
  `Cargo.toml` or config file was changed. No tag was created or pushed,
  and there was no other git mutation. The `git show`/`git log` reads
  were read-only.
* Assembly is not done here. PA-5 is not planned here. PA-6 is unchanged.

### Revision 10: attempt-8 findings, S5 split, and Ship walkthrough

**Authorization.** The operator decided on 2026-09-29 at 17:38 -07:00
(recorded in `docs/operator-glossary.md`, "Revision 10 decisions"):

1. **P1-G route = Option A.** Follow Ship's existing Step 4.3 rule. An
   earlier task may close with `EXPECTED_PENDING_RED` while a later
   task's recorded RED tests are present in files the earlier task does
   not touch. Ship's rules are not changed. Revision 10 must include a
   walkthrough of Ship's actual steps.
2. **Split `142.058-T` into S5.** The F54 parity task leaves S4 for a new,
   later release.

Revision 10 plus ONE scoped review is authorized. Revision 10 covers
P1-G, attempt-8 P2-1, the S5 split, the Ship walkthrough, and (at Stage
discretion) attempt-8 P2-2 to P2-10. It changes no backlog item, edge,
shipment, stash entry, source, test, `Cargo.toml` or config file.

#### R10.1 Gate restatement (P1-G, Option A)

*Revision 11 note: G5's two methods are replaced by R11.3's route (a)
forms, as amended by R11.10 (route (c) replaces (a-2)). G1 is tied to
each task's record (R11.6 P2-6). The rest stands.*

**What Ship actually does (the facts this section is built on).**
`.github/agents/_ship.agent.md`:

* Step 2 harnesses **every** in-scope task of the claimed shipment before
  the ready queue exists ("Any gap halts; do not proceed with a partial
  set"). A queued code task without a valid disposition goes to the
  harness-architect. An already-active code task reuses a valid RED
  harness; "any other active-task ambiguity halts".
* Step 4.3 gives a non-final code task one of two verdicts: `PASS`, or
  `EXPECTED_PENDING_RED` when all six conditions hold. Condition 1 needs
  the current task's **targeted** harness GREEN. Condition 3 needs every
  failure mapped by exact test name and marker to a recorded RED harness
  of a **later** task in this shipment. Condition 4 needs every declared
  owned file of each mapped task to match its recorded pre-implementation
  baseline.
* The final code task and the final PR readiness run must be `PASS`.

**Withdrawn (they conflict with Step 2).**

* R9.2, "No PA-5 harness is installed before PRE-3F's IC full-suite gate
  passes."
* R9.2, the bullet "Full-suite GREEN at every S4 gate", and its sentence
  "None of them closes with an `EXPECTED_PENDING_RED` mapping onto another
  S4 task."
* The escalation review's item 3 wording "do not install them ahead of
  PRE-3F's full-suite gate" (as carried by R9.2).

**Replacement gate rule (all shipments S1-S5).**

| Gate | Allowed verdict | Allowed mapping |
|---|---|---|
| Any non-final code task | `PASS` or `EXPECTED_PENDING_RED` | Only onto recorded RED tests of later code tasks **in the same shipment**, meeting all six Step 4.3 conditions |
| The final code task of a shipment | `PASS` only | none (Ship rule) |
| Final PR readiness run | `PASS` only | none (Ship rule) |
| `142.063-T` IC (S4) | `PASS` or `EXPECTED_PENDING_RED` | Only onto PA-5 tasks' RED tests (E7 puts every PA-5 task after `142.063-T`) |
| Each non-final PA-5 task (S4) | `PASS` or `EXPECTED_PENDING_RED` | Only onto later PA-5 tasks' RED tests |
| Last PA-5 task (S4's final code task) | `PASS` only | none |
| `142.058-T` (S5's only code task) | `PASS` only | none |

A mapping onto a task in another shipment is never allowed. No mapping
onto `142.058-T` exists, because F54 stays the GREEN placeholder through
S4 and `142.058-T` is in S5.

**Six execution rules that make the six conditions satisfiable.** These
are directives on how Ship applies its own rules. They add no waiver.

* **G1, one baseline point per shipment.** Ship records the condition-4
  owned-file baseline (content hash and type) for **every** code task in
  the shipment at one point: after the last Step 2 harness commit and
  before the first Step 4.1 claim. This is "before implementation", as
  condition 3 requires. Recording each task's baseline at its own harness
  commit is not allowed, because a later harness commit (for example, a
  `Cargo.toml` stanza) would then break an earlier task's baseline with no
  implementation having begun.
* **G2, implementation commits edit only single-owner files.** After the
  G1 point, a task's implementation commits edit only files that no
  other still-pending task in the shipment declares as owned. Shared
  files are finished in Step 2 harness commits: every `[[test]]` stanza,
  every module registration line (`pub mod x;` / `mod x;` in a parent
  `lib.rs` or `mod.rs`), every re-export line, and every new production
  file as a compiling stub (it may be empty, or hold only the task's
  final public signatures). If an
  implementation needs to edit a file that a pending task declares, that
  is a HALT to Stage. It is not a condition-4 exception.
* **G3, name-scoped targeted harness where a target is shared.** When two
  tasks' RED tests live in one test target, each task's recorded
  `harness_cmd` names that task's exact tests (for example
  `cargo test --test <target> -- --exact <name> ...`). Condition 1 is
  then judged on the task's own tests. The full suite stays unfiltered,
  as Step 4.3 requires.
* **G4, harness commits change no existing behavior.** A harness commit
  adds tests, stubs for new items, and registration lines. It does not
  change the body of an existing production function. So a RED stub can
  only fail its own task's tests. Each harness red-phase record shows the
  full-suite delta of that commit: its own RED tests and nothing else. Any
  other new failure is a HALT to Stage at Step 2.
* **G5, marker independence (found by the R10.5 walkthrough).** Step 4.3
  conditions 3 and 5 need each pending RED test to fail with its
  **recorded** marker, with no changed marker. So a RED test's first
  failure point must be its own task's, both at Step 2 and after every
  earlier task in the shipment is implemented. A RED test must not fail
  first inside another pending task's stub: when that task is
  implemented, the marker changes, and the earlier task's gate cannot
  pass. The harness-architect meets G5 in one of two ways:
  * a task-tagged panic as the first statement of the RED test body (the
    142-S precedent: F51-F54's `not implemented: Worker: F5x` records);
    the task's implementation replaces the body; or
  * a real test whose first unimplemented call is the task's **own**
    stub, when every item it calls before that is already on the base
    tree.

  Where neither works, that is a HALT to Stage at Step 2. `142.063-T` is
  unaffected: its RED fails at step (b) inside its own stub, and no other
  pending S4 task's code runs before that.
* **G6, one Step 4.3 verdict per task.** A task's subtasks run inside that
  task's Step 4.2 and share its harness. Ship's Step 3 derivation keeps
  task artifacts only, so no subtask takes its own verdict. R9.5 item 5's
  "a non-green result at `.002-ST` is a HALT" stays as a guard.

**HC item 5, restated (replaces R9.3 HC red-phase item 5).**
"`cargo dev-test --no-fail-fast`: non-zero. Its failures are exactly the
positive test plus the RED tests recorded by S4 harness commits already
on the branch, each matched by exact test name and marker. F54's
`placeholder_registered` and every other target are GREEN." Ship Step 2
harnesses S4 in dependency order, so HC is normally the first harness
commit and the positive test is the only failure. The HC HALT rule's
third bullet ("the full suite has any other failure") is read with this
restatement.

**IC gates, restated (replaces "Step 4.3 verdict must be PASS" in the
R9.3 IC gates).** The IC Step 4.3 verdict is `PASS` or
`EXPECTED_PENDING_RED` under the table above. The other IC gates are
unchanged: lint, format, three GREEN target-A runs, the oracle, and both
diff checks. Target A holds only `142.063-T`'s test, so condition 1 needs
no name scoping.

**Kept.** The final-task no-pending-red rule. "No review verdict may rely
on a RED F54 file while its owner is pending" (R9.2). The R9.3 HC HALT
and no-adjust rules. R9.4's F54 rows (no F54 row is RED before FL).

#### R10.2 S5 split and attempt-8 P2-1

*Revision 11 note: the assembly step's label removal is withdrawn. The
task is reset with its label kept (R11.2), and a failed FL is
`FL-RED-HALT` (R11.6).*

**S5 membership.** `142.058-T` F54 (`.002-ST` → `.003-ST`; `.001-ST`
stays done) → `142.059-T` F55 docs → `142-F` last.

* **`142.059-T` moves with `142.058-T`.** It has a recorded dependency on
  `142.058-T`. Left in S4, it would be blocked by an S5 task, and S4's
  ready queue could never finish. This is the "anything that must travel
  with it" in the operator's decision.
* **`142-F` moves to S5.** R6.2 places the feature only in the last
  shipment, and S5 is now the last one.
* **S4 becomes** `142.063-T` PRE-3F → PA-5 tasks (E7, then the PA-5
  plan's internal order). It has no docs task and no feature item. Its
  final code task is the last PA-5 task.

**Why S5 removes the P2-1 hazard.** P2-1 was that Ship Step 2, at the S4
claim, might rebuild `142.058-T`'s harness and load the real F54 body
before PRE-3F and PA-5 existed. At the S5 claim, S4 has shipped. So
`142.063-T`'s helper and every PA-5 change are already on `main`. A
harness built at the S5 claim is built on the right tree.

**P2-1 disposition: FL becomes `142.058-T`'s pinned Step 2 harness.**

* **Assembly step (new; Stage authority; recorded, NOT done here).** After
  H2, and before S5 is created, the assembly session:
  1. removes the `harness-ready` label from `142.058-T`;
  2. moves `142.058-T` to `queued` if it is still `active` from 142-S;
  3. adds a comment that keeps the history. The `7bd9e504` red-phase
     record, the done `.001-ST` (`6d216d19`) with its per-case record, and
     the B0 path (R9.9) are historical evidence. They are not the S5
     harness.

  Without this step, Ship Step 2 would find an already-active task whose
  recorded RED harness is not on the tree (F54 is the GREEN placeholder).
  That is "any other active-task ambiguity", which halts. With it,
  `142.058-T` is a queued code task with no valid disposition. Ship then
  invokes the harness-architect under its normal rule.
* **What the harness-architect does at S5 Step 2** (pinned; no
  discretion):
  1. **Provenance check first** (R9.5 item 1, amended below). Any failure
     is a HALT to Stage before any commit.
  2. **FL** is the harness commit. Its content is exactly R9.5 item 2:
     the `6d216d19` blob, the `#[path]` include, the one barrier call, and
     the identity fix. Nothing else. The R9.3 no-adjust rule applies to FL
     too: no change to that content and no lint attribute. A compile error
     that needs any other change is a HALT to Stage.
  3. **Red-phase record** (before `harness-ready`): `cargo check
     --all-targets`, clippy (pedantic), `cargo lint` and `cargo fmt-check`
     all exit 0; the three FL runs under B2 (R9.5 item 3); (e) rows 2-5 RED
     at their B2 signatures and row 1 GREEN; and `cargo dev-test
     --no-fail-fast` whose ONLY failures are F54 rows 2-5.
  4. Only then set `harness-ready`. The `.002-ST` and `.003-ST` work is
     the implementation (Step 4.2).
* **If (e) or any other red-phase item fails after FL is committed.** No
  `harness-ready` label is set. Ship HALTs to Stage with the FL SHA and
  the full outputs. FL stays a local, unpushed commit on the S5 branch.
  Ship does not amend, reset, revert, or push it without an operator
  decision (Constitution VII). This answers attempt-8 P2-1's "what happens
  to FL if (e) fails".

**R9.5 item 1, amended for S5.**

* (a) unchanged (T0 reaches both SHAs, locally and on the remote).
* (b) becomes: the historical records named in the assembly comment above
  exist and are unchanged. The `harness-ready` label is **not** part of
  (b), because assembly removes it on purpose.
* (c) becomes: the F54 file at `<S5 base>` is byte-identical to its
  content at `3f890662` (the GREEN placeholder), and `git log <S5
  base>..HEAD -- tests/contract/read_server_cli_mcp_parity_test.rs` is
  empty before FL. This covers every S1-S4 merge.
* (d) unchanged (the CLI-absence assertion is absent from the blob).
* The order inside `142.058-T` is unchanged: provenance → FL → three FL
  runs → `.002-ST` → `.003-ST` → M12 → Step 4.3 **PASS**. The first three
  steps now run at S5 Step 2.

**Claim gates move with the task.** The R9.9 drift ledger is recorded
before the **S5** claim, as `git log --oneline <A0 HEAD>..<S5 base> --
src/tools/capabilities.rs`. B2 item (v) reads it. The T0 remote check runs
at every claim, S5 included. T0 is kept until **S5** ships (was S4).

#### R10.3 Attempt-8 P2 dispositions (P2-2 to P2-10, Stage discretion)

*Revision 11 note: P2-9 is withdrawn in full (R11.5). The P3 "P-001
impact" is restated in R11.2. The HC bound reads "more than 400 new
lines" (R11.6).*

| P2 | Disposition |
|---|---|
| **P2-2** `Cargo.toml` is not disjoint per file | **Resolved by G1 and G2.** Each task owns only its own `[[test]]` stanza. Stanzas are added only in Step 2 harness commits. No implementation commit in S1-S5 edits `Cargo.toml`; if one must (for example, a new dependency), that is a HALT to Stage. Because the G1 baseline is taken after the last harness commit, `Cargo.toml` matches every pending task's baseline with nothing excluded. |
| **P2-3** `142.063-T` granularity | **Deviation recorded.** HC is harness work done by the harness-architect at Step 2. The 2-hour and five-function limits are applied to the implementation unit (IC: one body plus at most two private functions). **HC bound (new HALT):** more than four private test helpers, more than two support-file functions beyond the six pinned items, more than about 400 new lines across the three files, or more than 2 hours of harness work is a HALT to Stage for a replan. A new Constitution Check row records this Task-granularity deviation at assembly. |
| **P2-4** P-014 release circularity | **Resolved.** The PA-6 hold is lifted by a later recorded PA-6 decision **alone**. P-014 stays S3's normal merge gate at its PR. It does not gate the claim. This replaces R9.8's "plus a P-014 approval that cites that decision". |
| **P2-5** the lint rule bans `#![forbid(unsafe_code)]` | **Resolved.** The banned attributes are `allow`, `expect` and `warn`, whether written directly or through `cfg_attr`. `deny` and `forbid` are allowed, because they only tighten. This applies to the three PRE-3F files and to FL. |
| **P2-6** `ENGRAM_DATA_DIR` is not stripped | **Resolved.** The PRE-3F fixture's daemon `Command` calls `env_remove("ENGRAM_DATA_DIR")`. The in-process `code_graph::sync_workspace` seed resolves its store only from the fixture's temporary paths. If HC finds that the seed reads the ambient variable, that is a HALT to Stage. Neither the test nor the support file calls `std::env::remove_var` or `set_var`. |
| **P2-7** polling may mutate the snapshotted files | **Resolved.** The characterization (snapshot, `get_workspace_status`, snapshot) runs at IC **before** the three target-A runs, and its result is recorded. The barrier makes no IPC call between S1 and S2 (unchanged). If the characterization shows that the status read writes a snapshotted file, that is a HALT to Stage. Paths are never excluded (same rule as `142.066-T`). The fixture pins the daemon idle TTL above `SETTLE_TIMEOUT` plus the (d) window by writing `idle_timeout_minutes` (for example `30`) in the config file it creates (attempt-8 Learnings P3). |
| **P2-8** a refused `_shutdown` in `read_server` mode | **Resolved.** In `stop_daemon`, a refused or unanswered `_shutdown` followed by `kill` and `wait` on the owned handle is the normal path. It prints nothing. Only an OS error from `kill` or `wait` prints `PRE3F-BLOCK:`. |
| **P2-9** full-suite gates with known 142-S flakes | **Resolved, with no waiver.** (1) At each shipment claim, before Step 2, Ship runs `cargo dev-test --no-fail-fast` on the base and keeps the output. A non-GREEN base is a HALT before any harness commit. (2) At a gate, a failure that is not in the mapping is blocking. Ship may re-run that one target alone for **classification only**. The verdict then needs a new complete `cargo dev-test --no-fail-fast` run that meets `PASS` or all six conditions. Both complete outputs are kept. (3) A second unmapped failure of the same test goes to P-021 intake. It is never waived. |
| **P2-10** no bounded retry for single IPC calls in (a) and (d) | **Resolved.** In steps (a) and (d), a single `get_workspace_status` or fingerprint call retries up to 3 times, 100 ms apart, on a transport error or an error whose text names a SQLite busy or locked condition. In (d), retries happen only for F1 (before S1) and F2 (after S2), never between S1 and S2. Exhausted retries panic with the step's existing `PRE3F-RED` text, which is not valid RED for HC (the HC HALT rule applies). |

**Attempt-8 P3s absorbed (Stage discretion).**

* **Hold token and scope.** The hold comment text is the literal token
  `PA-6-HOLD` plus the decision reference. It goes on every S3, S4 and S5
  task, and on the S3, S4 and S5 shipments. A `DARK_MODE_SCOPE` that
  names S3, S4 or S5 is a HALT. It is not narrowed silently.
* **P-001 impact of the hold.** While the hold stands, S1 and S2 can ship,
  and after that the 142-F pipeline stops. The Orchestrator may route
  unrelated shipments under P-001's normal rules (R9.8, unchanged).
* **Windows error 33.** `filesystem_snapshot` treats Windows errors 32 and
  33 as lock errors. A path locked in both S1 and S2 is recorded as
  locked by its metadata. It is not treated as proof of stability by
  itself; its size and modified time must also match.
* **Full outputs.** Every full-suite and targeted run named in this plan
  keeps its complete output in the task record.
* **T0 retirement.** If a later PA-6 decision retires S4 or S5, T0 is kept
  until the operator decides what happens to it (Constitution VII).

All other attempt-8 P3s stay open. None of them blocks assembly.

#### R10.4 Shipments and dependency edges

*Revision 11 note: the claim-gate column is replaced by R11.7's full
gates. The assembly additions are replaced by R11.9. Manifests hold no
subtasks (R11.2). Members, order and edges are unchanged.*

S1-S5 are provisional labels. backlogit assigns the IDs at creation.

| Shipment | Members (execution order) | Blocked by | Claim gate (in addition to P-001 and the T0 remote check at every claim) | Final code task (`PASS` only) |
|---|---|---|---|---|
| **S1** | unchanged (R6.2): `142.061-T` → `142.062-T` → `142.064-T` (3 ST) → `142.065-T` → `142.066-T` → `142.067-T` → `142.068-T` | none | the R10.5 S1 item-text changes are applied | `142.068-T` |
| **S2** | unchanged (R6.2): `142.054-T` (3 ST) → `142.069-T` … `142.073-T` → `142.074-T` (docs) | S1 | the R10.5 S2 item-text changes are applied; `142.054-T` is `queued` with no `harness-ready` label and carries the W6 history comment | `142.073-T` |
| **S3** | unchanged: `142.055-T` → `142.056-T` → `142.057-T` → `142.060-T` | S2, and the PA-6 hold | a later recorded PA-6 decision lifts the hold (R10.3 P2-4); the W9 item-text change is applied | `142.060-T` |
| **S4** | `142.063-T` PRE-3F → PA-5 tasks | S3 (E8, and the shipment order) | S3 shipped; the PA-5 tasks exist and are in S4's manifest; E7 recorded | the last PA-5 task |
| **S5** (new) | `142.058-T` F54 (`.002-ST` → `.003-ST`) → `142.059-T` F55 docs → `142-F` | S4 (E3 and E6) | S4 shipped (including its P-001 closure); E6 recorded; the R9.9 drift ledger recorded against `<S5 base>`; `142.058-T` is `queued` with no `harness-ready` label and carries the history comment (R10.2) | `142.058-T` |

**Edges: no new edge, and none removed beyond E1-E8.**

* Assembly still applies E5, E1, E2, E3, E4, E8, with a cycle check after
  each, and E4 before E8. The PA-5 harvest applies E6 and E7.
* **S5 after S4 comes from item edges.** E3 (`142.058-T` → `142.063-T`)
  and E6 (`142.058-T` → the last PA-5 task) both point from S5 into S4.
  The existing `142.059-T` → `142.058-T` edge keeps F55 after F54 inside
  S5.
* **E4 is now essential.** It removes `142.060-T` → `142.058-T`. Without
  it, an S3 task would depend on an S5 task, and S3 could never finish.
  The cycle check after E4 confirms that no S1-S4 item depends on an S5
  item.
* E6 crosses the S4/S5 boundary. It is still applied at the PA-5 harvest.
  It is required before the S5 claim, not the S4 claim (R9.8's S4 gate is
  split as shown above).

**Assembly additions (recorded; NOT done here).**

1. Create S5 with `items: [142-F]`, then add `142.058-T`, its two open
   subtasks, and `142.059-T` in that order. (Ship Step 0.5 resolves the
   feature through `parent_id`; putting it first satisfies Stage's
   parent-first rule.)
2. S4 is created with `142.063-T` only. The PA-5 harvest adds the PA-5
   tasks. S4 contains no feature item.
3. The R10.2 `142.058-T` label, status and history-comment step, and
   the same step for `142.054-T` (W6). *(Attempt-9 completion edit.)*
4. The hold comments with the `PA-6-HOLD` token on every S3, S4 and S5
   task and shipment (R10.3).
5. Copy the T0 location and the B0 path to the S5 shipment and to
   `142.058-T` (was "the S4 shipment", R9.9 step 4).
6. The R10.5 item-text changes for S1, S2 and S3 (W9). *(Attempt-9
   completion edit.)*

#### R10.5 Ship walkthrough S1 to S5

*Revision 11 note: re-walked in R11.7. The W dispositions are governed
by R11.4's table, and the P2-9 base run is withdrawn (R11.5).*

Stage walked each shipment through `.github/agents/_ship.agent.md`
(read-only): Step 0.5 claim, Step 1 pre-flight, Step 2 harness, Step 3
ready queue, Step 4.1 claim, Step 4.3 gates, and final PR readiness.
The owned files come from the queued item text (read-only). Findings are
W1-W9. Each one has a disposition, and every item-text change is applied
at assembly (Stage authority), not here.

**Common to every shipment.** Step 1 (P-001, `cargo check`) and the
P2-9 base run come before Step 2. Step 2 harnesses in dependency order.
The G1 baseline is taken once, after Step 2. Step 3 keeps task artifacts
only (G6). Each non-final code task closes `PASS` or
`EXPECTED_PENDING_RED` within its own shipment. The final code task and
the PR readiness run are `PASS`.

**S1 (read-server plumbing).**

| # | Step | Finding | Disposition |
|---|---|---|---|
| W1 | 4.3 cond. 4 | `142.061-T` (PRE-1) and `142.062-T` (PRE-2) both declare `src/services/generations/candidate_build.rs`. PRE-1's implementation changes a file that the pending PRE-2 declares, so PRE-1 can't close `EXPECTED_PENDING_RED`, and PRE-2's RED makes `PASS` impossible. | PRE-2's three public functions move to a new file, `src/services/generations/candidate_publish.rs`, owned by PRE-2 alone. The two `pub mod` lines in `generations/mod.rs` land in the PRE-1 and PRE-2 harness commits (G2). PRE-2 keeps `publish.rs` (visibility only). Downstream item text names `candidate_publish::open_generation_store` (and the other two). If PRE-2 needs to edit `candidate_build.rs`, HALT to Stage. |
| W2 | 4.3 cond. 4 | `142.067-T` (PRE-5) and `142.068-T` (PRE-6) both declare `src/preflight_probe.rs`. | PRE-6's `probe_mcp_read` and private `run_mcp_session` move to `src/preflight_probe/mcp.rs`, owned by PRE-6. The lines `mod mcp;` and `pub use mcp::probe_mcp_read;` in `preflight_probe.rs` land in PRE-6's harness commit (G2), after PRE-5's harness commit creates the file. |
| W3 | 4.3 cond. 1 and 4 | PRE-4 (`142.066-T`) extends PRE-3's test file `read_server_generation_wiring_test.rs`. PRE-6 extends PRE-5's `preflight_probe_test.rs`. Under G5's first method, PRE-3's implementation rewrites its own test bodies in a file that the pending PRE-4 declares, which breaks condition 4. It is the same for PRE-5 and PRE-6. | PRE-4 gets its own target: `tests/integration/read_server_request_entry_admission_test.rs`, `[[test]] name = "integration_read_server_request_entry_admission"`. PRE-6 gets `tests/integration/preflight_probe_mcp_test.rs`, `integration_preflight_probe_mcp`. PRE-3's and PRE-5's fixture code that the later task needs goes in `tests/helpers/read_server_generation_fixture.rs` (owned by PRE-3) and `tests/helpers/preflight_probe_fixture.rs` (owned by PRE-5). The later task `#[path]`-includes it **in its implementation**, not its harness, so its RED crate has no unused module. Scenario counts and acceptance criteria are unchanged. Only the target names change. |
| W4 | 4.3 cond. 3 and 5 | Every S1 task after PRE-1 exercises upstream S1 code: PRE-2 builds with PRE-1, and PRE-3, PRE-4b, PRE-4 and PRE-5 publish with PRE-2. A production-stub RED would first fail inside an upstream stub, and its marker would change when that upstream task lands. | G5's first method (a task-tagged panic as the first statement) for every S1 RED test after PRE-1. PRE-1 may use either method. |
| W5 | 4.3 cond. 4 | `142.064-T`'s three subtasks share its files. | G6: one verdict per task. No change needed. |

S1 final code task: `142.068-T`, `PASS`. With W1-W4 applied, every
earlier S1 gate can meet all six conditions. The owned files are
disjoint, `Cargo.toml` is fixed after Step 2, and the markers are
per-task.

**S2 (self-check and `preflight`).**

| # | Step | Finding | Disposition |
|---|---|---|---|
| W6 | 2 | `142.054-T` (F50) is `active` with `harness-ready`. Its parked harness (`c269fa79` + `41dd5081`) is GREEN, and the real-chain case (PA-2b) is RED on no tree. Step 2 finds an active task with no valid RED harness: an "active-task ambiguity", which halts. This is the same class as P2-1. | The same assembly step as for `142.058-T` (R10.2): remove the label, move the task to `queued`, and add a history comment. At S2 Step 2 the harness-architect writes F50's harness: the parked scaffold commits, plus the real-chain RED case (PA-2b), and `crates/engram-indexer/src/preflight.rs` as a compiling stub (G2). If F50's public types can't be final at harness time, HALT to Stage. |
| W7 | 4.3 cond. 4 | `142.069-T` and `142.070-T` each add one line to `crates/engram-indexer/src/lib.rs`. | G2: both `pub mod` lines, with empty module files, land in their own harness commits. Neither implementation edits `lib.rs`. |
| W8 | 2 | The NEW-1, NEW-2 and NEW-4 harnesses `#[path]`-include `preflight.rs`, `preflight_verdict.rs` and `preflight_invocation.rs`, which earlier S2 tasks own. At Step 2 those files don't exist yet, or exist only as stubs. | With G5's first method, a RED harness includes only files that exist after the earlier harness commits. The `#[path]` include lines with their scoped `#[allow(dead_code)]` move into each task's implementation, in its own test file. The included files are not declared by the including task, so an upstream implementation never breaks a downstream baseline. |

`142.071-T`'s crate-local harness is not in `cargo dev-test`. So it never
appears in another task's full suite, and its own gate uses its recorded
`harness_cmd` (unchanged plan deviation). `142.074-T` is docs-only. S2's
final code task is `142.073-T`, `PASS`.

**S3 (launch scripts and archive fix; PA-6 hold).**

| # | Step | Finding | Disposition |
|---|---|---|---|
| W9 | 4.3 cond. 4 | `142.056-T` (F52) declares `tests/contract/start_launcher_test.rs`, which F51 owns. Its implementation rewrites the fail-open assertions there (about lines 80 and 154). Its harness commit `5760b948` did not touch that file. F51's implementation edits that file, so at F51's gate the pending F52's declared file changes, and F51 can't close. | Move the in-place rewrite of those fail-open assertions to `142.055-T` (F51). It owns the file, and fail-closed launching is its scope. F52's owned files narrow to `start_launcher_failure_test.rs`. If a read-only check at assembly shows F52 still needs `start_launcher_test.rs`, HALT to Stage. S3 stays held, so this can be settled before the hold lifts. |

F51, F52 and F53 are `active` with valid parked RED harnesses (test-level
`not implemented: Worker: F5x` markers, which G5 accepts). Step 2 reuses
them. `142.060-T` is `queued` with a valid RED record and is reused. At
F51's gate the pending set is F52 (1), F53 (3) and archive (2), in
disjoint files after W9. S3's final code task is `142.060-T`, `PASS`.

**S4 (PRE-3F and PA-5).**

* Step 2: HC first (dependency order), then each PA-5 harness commit. The
  HC red-phase record follows the restated item 5 (R10.1).
* `142.063-T` IC gate: target A holds only its own test (condition 1).
  Failures are the PA-5 RED tests, each with its own marker (condition
  3). IC edits only `tests/helpers/activation_settle.rs`, which no PA-5
  task declares (condition 4).
* **PA-5 planning inputs (added to R6.8; recorded, not planned):** each
  PA-5 task (i) declares files disjoint from every other S4 task's,
  including `tests/helpers/activation_settle.rs`, target A and the F54
  file; (ii) meets G5, so no PA-5 RED test calls
  `await_activation_settled` or another pending PA-5 stub before its own
  marker; (iii) meets G4, so no harness stub changes a shared handler's
  current behavior, and target A's positive test stays GREEN after
  every PA-5 harness commit and implementation; (iv) adds `Cargo.toml`
  stanzas only in its harness commit.
* S4's final code task is the last PA-5 task, `PASS`. There is no F54
  work in S4.

**S5 (F54, F55 docs, `142-F`).**

* Claim gate: R10.4. Step 2: `142.058-T` goes through the pinned FL
  harness (R10.2), and `142.059-T` is an already-active docs task
  qualified by its recorded verification plan (Ship's existing
  docs-route rule).
* Step 3 order: `142.058-T`, then `142.059-T` (the existing edge).
* `142.058-T` is the only code task, so it is the final one: `PASS` after
  `.002-ST`, `.003-ST` and M12. `142.059-T` passes its recorded gates.
  Final PR readiness is `PASS`. `142-F` closes in S5's Step 6 closure.

**Walkthrough result.** Every S1-S5 gate can legally pass under Ship's
unchanged rules once G1-G6 and W1-W9 are applied at assembly. W1-W4 and
W9 change file ownership or test-target names. They do not change
scope, scenarios, or acceptance intent, and the scoped review should
check them.

#### R10.6 Supersession notes and scope limits

*Revision 11 note: R11.8 supersedes this table where they disagree, and
it replaces the Constitution Check addition below.*

**Superseded text (Revision 10 wins where these disagree).**

| Earlier text | Revision 10 replacement |
|---|---|
| R6.2 S4 row (members, "3 + PA-5", "4 RED"), and "`142-F` is added only to S4" | R10.4: S4 = `142.063-T` → PA-5. S5 = `142.058-T` → `142.059-T` → `142-F`. The feature item is in S5. |
| R6.2 inter-shipment blocks ("S4 by S3" only) | Adds "S5 by S4", enforced by E3 and E6 |
| R7.5 / R9.8 S4 claim gate (PA-5 in manifest, E6 and E7, T0, drift ledger) | Split: S4 needs E7 and the PA-5 tasks; S5 needs E6, the drift ledger and the `142.058-T` reset (R10.4). T0 is checked at every claim. |
| R9.2 "No PA-5 harness is installed before PRE-3F's IC full-suite gate passes" | Withdrawn (R10.1) |
| R9.2 "Full-suite GREEN at every S4 gate" bullet | R10.1 gate table |
| R9.3 HC red-phase item 5 | R10.1 restated item 5 |
| R9.3 "IC gates (Step 4.3 verdict must be **PASS**)" | `PASS` or `EXPECTED_PENDING_RED` onto PA-5 only (R10.1) |
| R9.3 "No lint attribute (`#[allow]`, `#[expect]`, or `cfg_attr` carrying one)" | Only `allow`, `expect` and `warn` are banned; `deny` and `forbid` are allowed (R10.3 P2-5) |
| R9.3 two-hour check | Plus the HC bound and the recorded deviation (R10.3 P2-3) |
| R9.5 item 1 (b) and (c) | R10.2 amended (b) and (c) |
| R9.5 "Order inside `142.058-T`" | The same order. Provenance, FL and the three FL runs run at S5 Step 2 as the pinned harness (R10.2). |
| R9.8 "Release … plus a P-014 approval that cites that decision" | A later PA-6 decision alone (R10.3 P2-4) |
| R9.8 Orchestrator, Ship and dark-mode lines ("S3 and S4") | Read "S3, S4 and S5". A `DARK_MODE_SCOPE` naming any of them is a HALT (R10.3). |
| R9.9 drift ledger "before the S4 claim", `<S4 base>`; T0 kept "until S4 ships"; H2 step 4 "the S4 shipment" | `<S5 base>`, before the S5 claim; T0 until S5 ships; "the S5 shipment" (R10.2, R10.4) |
| R6.2 S1 "Each unit writes its own RED harness and turns it GREEN inside S1" | Unchanged in intent. Earlier S1 tasks may close `EXPECTED_PENDING_RED` under R10.1 with W1-W4 applied. |
| The S1, S2 and S3 item text for the owners and targets named in W1-W3 and W6-W9 | R10.5 dispositions (applied at assembly) |

Inline Revision 10 notes were added at R6.2, R9.2, R9.3 (HC item 5 and
IC gates), R9.5 item 1, R9.8 and R9.9. The Dependency Graph,
Constitution Check and Runtime Verification sections are not annotated
here. This table governs them until assembly.

**Constitution Check addition (recorded here for assembly).** II
Test-first: the RED harness still comes first for every code task (G5
keeps real RED where it is possible). Task granularity: the HC
deviation is recorded (R10.3 P2-3). VII: FL is never reset or pushed
without an operator decision (R10.2).

**Edges.** Unchanged: E5, E1, E2, E3, E4, E8 at assembly; E6 and E7 at
the PA-5 harvest.

**Scope limits.**

* This session changed only this plan, `docs/operator-glossary.md`, and
  the Revision 10 memory file.
* No backlog item, edge, shipment, stash entry, source, test,
  `Cargo.toml` or config file was changed. No tag was created or pushed,
  and there was no git mutation. The `git show` read was read-only.
* Assembly is not done here. PA-5 is not planned here. PA-6 = Hold S3 is
  unchanged, and its reach now includes S5.
* Next step: ONE scoped plan review of Revision 10 and the text it
  touches (operator-authorized). Assembly waits for that review to pass.

### Revision 11: status resets, route (a), layout delegation, and P-021

**Authorization.** The operator decided on 2026-09-29 at 18:14 -07:00,
verbatim: "approve resets, route (a), delegate layout, Revision 11".
These decisions are recorded in `docs/operator-glossary.md` ("Status reset
(queued reset)" and "Revision 11 decisions"). This session writes
Revision 11 only. The scoped plan review runs in a separate session.
Revision 11 changes no backlog item, edge, shipment, stash entry, source,
test, `Cargo.toml`, config file, or git state.

**Precedence.** Where Revision 11 and an earlier revision disagree,
Revision 11 wins. R11.8 lists every superseded item. Short "Revision 11"
notes mark the affected R9 and R10 subsections.

#### R11.1 Decisions recorded (operator, 2026-09-29 18:14 -07:00)

| # | Decision | Where it lands |
|---|---|---|
| 1 | **Status resets approved** for `142.054-T`, `142.055-T`, `142.056-T`, `142.057-T`, `142.058-T` and `142.059-T`: `active` → `queued` at assembly, each with a history comment. Labels are kept unless Ship's rules require removal. Every claim gate adds "every task member is `queued`", and the walkthrough re-walks Ship Step 0.5 item 1a. | R11.2, R11.7, R11.9 |
| 2 | **Route (a)** for G5: harness RED tests fail with the task's own marker. Each task's first build commit swaps in the real test body and records it failing, with its real assertions, against the task's own placeholder before implementation proceeds. No Constitution II exception. Lint is solved without suppression attributes (`#![forbid(unsafe_code)]` stays allowed). | R11.3 |
| 3 | **Delegate layout.** The plan stops pinning new file names, module paths and test-target names. The harness-architect chooses them at claim time under binding constraints (i)-(iv). Operator-approved S3-S5 structure stays, but its internal file names are not pinned where no operator decision needs them. | R11.4 |
| 4 | **Drop the P2-9 flaky-test rule** and its base run. Unexpected failures follow Ship's existing P-021 classification and Step 4.4a. | R11.5 |
| 5 | Fix the remaining attempt-9 P2s only where they would block a Ship gate, and record a disposition for every P2 and P3. | R11.6 |

**Facts checked read-only in this session (2026-09-29).**

* **Statuses** (`.backlogit/queue/*.md`):
  * `active` with `harness-ready`: `142.054-T`, `142.055-T`,
    `142.056-T`, `142.057-T` and `142.058-T`.
  * `active` with `harness-verification-gated`: `142.059-T`.
  * `queued`: `142.060-T` (`harness-ready`), and `142.061-T` to
    `142.074-T` (labels `preflight`, `pending-pa-1`, and plan refs; no
    route label).
  * Also `active`, and not tasks: `142-F`, the 142-S shipment, the
    subtasks `142.054.001-ST` to `142.054.003-ST`, and `142.058.002-ST` and
    `142.058.003-ST`.
* **The parked harness commits are not on `main`.** `git branch -a
  --contains 5760b948` lists only the parked 142-S branch. Those commits
  are reachable only through that branch and T0 (R9.9).
* **`5760b948` touched only
  `tests/contract/start_launcher_failure_test.rs`** (626+/8-).
* **On `main`, `tests/contract/start_launcher_test.rs` still has the
  fail-open tests.** They are `launcher_fails_open_to_copilot_within_one_prewarm_budget`
  (assertion at line 80) and
  `launcher_timeout_does_not_terminate_unowned_descendant` (line 154).
  F51's parked harness commit `1dfc1b5b` (594+/128-) replaced them with
  fail-closed tests.
* **The FL blob strips the variables.** At `6d216d19`, the F54 file's
  three child-process sites (lines ~282, ~810, ~905) each call
  `env_remove` for `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE` and
  `ENGRAM_DIRECT`, and set `ENGRAM_IDLE_TIMEOUT_MS`. The blob has no
  `allow`, `expect` or `cfg_attr` attribute.
* **`idle_timeout_minutes` has a parser.** `PluginConfig` in
  `src/models/config.rs` (~263-275) reads it from `.engram/config.toml`.
* **Lint settings.** `.cargo/config.toml` sets `rustflags =
  ["-Dwarnings"]`, and `cargo dev-test` is `test --all-targets`.
* **`build-feature` forbids test edits.**
  `.github/skills/build-feature/SKILL.md` says "Never modify test files
  (tests are the specification)" (~250), and "Do NOT: Modify the test to
  make it pass" (~211). This matters for decision 2 (R11.3, H-R11-1).

#### R11.2 Status resets and claim-gate additions (attempt-9 P1-1, P2-8, P2-10)

**The six resets (applied at assembly, R11.9 step 3).**

| Task | Today | Moves to | Labels | Shipment |
|---|---|---|---|---|
| `142.054-T` F50 | `active`, `harness-ready` | `queued` | kept | S2 |
| `142.055-T` F51 | `active`, `harness-ready` | `queued` | kept | S3 |
| `142.056-T` F52 | `active`, `harness-ready` | `queued` | kept | S3 |
| `142.057-T` F53 | `active`, `harness-ready` | `queued` | kept | S3 |
| `142.058-T` F54 | `active`, `harness-ready` | `queued` | kept | S5 |
| `142.059-T` F55 docs | `active`, `harness-verification-gated` | `queued` | kept | S5 |

**Labels: Ship's rules require no removal.**

* **Ship requires removal in only one case: a conflicting dual label.**
  This means `harness-ready` together with `harness-verification-gated`
  (Step 2 item 4, and Step 3 item 2). None of the six has one.
* **Code tasks are re-qualified at Step 2.** Step 2 item 4 sends a queued
  code task to the harness-architect when its "supporting evidence" is
  invalid. The recorded evidence of the five code tasks is:
  * the parked commits `c269fa79`, `41dd5081`, `1dfc1b5b`, `4995d681`,
    `5760b948`, `e24f5ae2` and `7bd9e504`;
  * the done subtask `6d216d19`.

  None of it is on any S2, S3 or S5 base tree (R11.1). F50's and F54's
  harnesses are also GREEN, so not RED. The evidence is invalid for the
  shipment. The architect rebuilds, or re-applies the content from T0
  under R11.3, and leaves exactly `harness-ready` (Ship's "repair/rebuild
  only if its manifest is invalid").
* **The docs task is re-qualified the same way.** For `142.059-T`, Ship
  records or supplements the verification plan (Step 2 item 3), and the
  architect re-qualifies `harness-verification-gated`.
* **This supersedes the label removals** in R10.2 assembly step 1
  (`142.058-T`) and in W6 (`142.054-T`).

**History comment (one on each reset task; Stage writes it at assembly).**

```text
STATUS-RESET 2026-09-29 (operator-approved; plan Revision 11, R11.2).
Moved active -> queued at 142-F assembly. Previous shipment: 142-S
(abandoned at H2). The recorded harness evidence (<commit list>) is
historical: it is not on the <S2|S3|S5> base tree and is NOT a valid
Step 2 disposition for that shipment. The <label> label is kept; Ship
Step 2 item 4 re-qualifies it through the harness-architect under R11.3.
Subtasks <list or "none"> are unchanged and are not shipment members.
```

*Revision 12 note:* the comment's first line and its "under R11.3"
clause are replaced by assembly edit A16 (R12.5), which cites R11.10 and
R12.

**Subtasks are not shipment members.**

* **Manifests hold task artifacts only**, plus `142-F` at the head of S5
  (R10.4 addition 1). Subtasks are never members. *(Revision 14: `142-F`
  leaves the S5 manifest, so every manifest is task-only; R14.2, Q5.)*
* **Why.** Ship Step 0.5 item 1a scans a "task-only manifest, per the
  097-S contract". Step 3 filters members by "IDs ending `T`", and an
  `-ST` ID also ends in `T`. So a subtask member could be run as its own
  task, or stop Ship at item 1a.
* **The subtasks still run** inside their parent's Step 4.2 (G6).
* **The five `active` subtasks stay `active`.** They are `142.054.001-ST`
  to `.003-ST`, and `142.058.002-ST` and `.003-ST`. Their reset was not
  approved, and no Ship gate reads a subtask that is not a manifest
  member.
* **Deviation.** Stage's own template adds each subtask after its parent
  (Stage Step 5.5 item 4c). This plan follows Ship's task-only contract
  for the handoff instead. This supersedes the R6.2 "(3 ST)" member
  listing and R10.4 addition 1's "its two open subtasks".

**Claim-gate additions (every S1-S5 claim; the full gates are in R11.7).**

* **CG-Q, all members `queued`.** Every manifest member whose
  `artifact_type` is task is `queued`, read with `backlogit get <id>`
  (the item 1a CLI path). The manifest holds no subtask.
* **CG-M, backlog changes on `main`.** Every Stage backlog change that
  the shipment depends on is committed to `main`: assembly, the PA-5
  harvest, and the hold lift. The worktree is clean, so Step 0.5 item 3a
  can create the branch from `main` (attempt-9 Learnings P3).
  *Revision 12 note:* CG-M is reworded in R12.6. Changes land through a
  merged `chore/stage-142-f-<purpose>` staging PR, never a direct commit
  to `main`. CG-S (a status re-read after any index sync) is added.
* **CG-H, H-R11-1 settled (S1 and S2 only).** H-R11-1 is resolved by a
  recorded operator decision (R11.3).
* **CG-P, PA-5 form (S4).** Every PA-5 task uses route form (a-1), or
  H-R11-1 is resolved (R11.3).

**P-001 impact of the PA-6 hold (restates R10.3 P3 "P-001 impact"; P2-8).**

* After the resets, no 142-F task is `active` between shipments.
* The feature `142-F` itself stays `active`, because it is the release
  unit in flight. So while the hold stands after S2 ships, Ship Step 1's
  P-001 check blocks the claim of **every other** top-level release
  unit.
* R10.3's sentence "The Orchestrator may route unrelated shipments under
  P-001's normal rules" is withdrawn.
* No Ship gate needs `142-F`'s status changed. If the operator wants
  other releases to move during the hold, that needs a separate decision.

**Operator acknowledgment (P2-10).** The operator approved all six resets
on 2026-09-29 at 18:14 -07:00. This includes `142.054-T`, which Revision 10
reset without approval. The escalation review's "HALT, not re-label"
applies to unapproved re-labels. These resets are approved, and they keep
the labels.

#### R11.3 Route (a) RED procedure and lint-safe placeholder rule (attempt-9 P1-4, P2-1, P2-2)

*Superseded by R11.10 (operator, 2026-09-29 18:47 -07:00, route (c)):
(a-2), the swap, L1, L2 and the L4 swap record are replaced, and H-R11-1
is closed. (a-1), (t), L3 and the (a-1)/(t) marker rules still apply.*

**Route (a) replaces the two methods in R10.1 G5.** Each code task's RED
takes exactly one of the three forms below. The harness-architect records
the form in each task's Step 2 harness record.

* **(a-1) Real body at Step 2, failing in the task's own code.**
  * The test's first failure is inside the task's **own** code: its own
    stub, or its own existing script or file, not yet changed. That holds
    at Step 2, and at the gate of every earlier task in the shipment.
  * So no pending in-shipment upstream code runs before the test's own
    failure.
  * The real assertions are seen failing at Step 2, and route (a)'s swap
    is a no-op.
  * **Expected users** (the architect confirms at Step 2):
    * `142.063-T` HC and `142.058-T` FL (their pinned harnesses);
    * the first task of each shipment: `142.061-T`, `142.054-T` and
      `142.055-T`;
    * `142.057-T` and `142.060-T`.
* **(a-2) Placeholder at Step 2, then the swap.**
  * **Step 2.** Each RED test is a placeholder (L1). The harness-architect
    also writes the task's real test bodies, and records them word for
    word, with a content hash, in the Ship-owned harness record. They are
    not compiled yet.
  * **The swap (the task's first Step 4.2 commit).** Every upstream
    in-shipment task is done by then, in dependency order. The recorded
    real bodies replace the placeholders, byte-identical to the recorded
    hash. The task's own placeholder items are added in its own files
    (L3).
  * **The RED record, taken before any implementation edit.**
    `harness_cmd` compiles. Each real test fails at the task's own
    placeholder (`Worker: <task-id> ...`), or at a real assertion against
    the task's own unimplemented code. The run shows the recorded
    expected test count.
  * **Halt rule.** A failure anywhere else is a HALT to Stage.
* **(t) Test-only task** (no production code of its own; in this plan,
  only `142.056-T` F52).
  * **Step 2.** The real body goes in at Step 2. It fails with F52's own
    assertion messages against the pre-F51 `start.ps1`.
  * **After its upstream.** The tests are expected to go GREEN once the
    upstream (`142.055-T`) is implemented, so they are never mapped.
  * **If an F52 test is still RED at F51's gate,** it is an in-scope F51
    failure (the same contract surface, P-021 C1). It goes back to F51's
    build-feature (R11.5).

**Markers (Ship condition 3).** Each marker is exact and names its task.

* For (a-2), the marker is `Worker: <task-id> <test_name>`.
* For (a-1) and (t), the marker is a stable substring, recorded per test,
  of the task's own stub or assertion message. It never includes paths,
  process IDs, timings, or captured output.
* A generic `Worker` substring is not a marker.
* If the architect can't find a stable marker, that is a HALT to Stage
  at Step 2, before any implementation.

**Lint-safe rules** (P2-1, P2-2). There are no `allow`, `expect` or `warn`
attributes, whether written directly or through `cfg_attr`. `deny` and
`forbid` stay allowed.

* **L1, the placeholder test.**
  * It is a synchronous `#[test] fn <final_name>()` with no parameters
    and no return type.
  * Its body is exactly one expression:
    `unimplemented!("Worker: <task-id> <test_name>")`. Nothing follows
    it, so `unreachable_code` can't fire.
  * The commit that adds it adds no `use`, helper, const, module or
    `#[path]` include beside it, so `unused_*` and `dead_code` can't fire.
  * The swap may change the attribute (for example, to `#[tokio::test]
    async fn`). The test name never changes, because it is the mapping
    key.
* **L2, Step 2 production content for (a-2) tasks.** None, except an
  empty module file registered by a LD1 registration line (R11.4). An empty
  module compiles clean.
  * So no Step 2 stub has to deal with unused parameters,
    `clippy::unused_async`, `missing_errors_doc`, `missing_panics_doc` or
    a re-export.
  * A `pub use` or `use` of an item that doesn't exist yet never lands at
    Step 2.
* **L3, stubs** (the Step 2 stubs of (a-1) tasks, and the swap-commit
  placeholders of (a-2) tasks).
  * **Body.** If there are parameters, the body starts with
    `let _ = (<every parameter>);` (for a method, `let _ = self;` is
    included). The last expression is
    `unimplemented!("Worker: <task-id> <item>")`, and nothing follows it.
  * **Async stubs** contain one `std::future::ready(()).await;` before
    the `unimplemented!`.
  * **Public docs** carry `# Errors` and `# Panics` sections.
  * **Only live items.** A stub is created only if it is `pub` on a
    public module path, or called by existing non-test code. So it is
    never `dead_code` in a non-test build. An item that would be dead is
    created only by the implementation that first uses it.
* **L4, lint gates.**
  * **Step 2 record.** It needs `cargo check --all-targets`, `cargo lint`
    and `cargo fmt-check` to exit 0. Every later Step 4.3 lint gate runs
    over a tree that still holds later tasks' Step 2 content.
  * **(a-2) swap record.** It needs only the `harness_cmd` compile and the
    failure evidence. Lint and format apply at the task's own Step 4.3,
    after the placeholders are implemented.

**Constitution II (no exception).** In all three forms, every real test is
seen failing before the task's implementation:

* at Step 2 for (a-1) and (t);
* at the swap for (a-2).

The evidence is kept in the Ship-owned task and run record. Where the swap
is used, the harness-architect writes the real test before implementation
("tests are the specification").

**H-R11-1: a halt that remains. It conflicts with Ship's delegated rules.**

* **The conflict.**
  * The (a-2) swap is a test-file edit inside Step 4.2.
  * Ship Step 4.2 hands Step 4.2 to build-feature only, and build-feature
    forbids that edit ("Never modify test files"; R11.1).
  * Ship does not write code itself (Ship Role), and it runs the
    harness-architect only at Step 2.
  * So the first (a-2) task in a shipment halts at its Step 4.2.
* **Stage can't remove this** without a rule change or a re-split.
* **Affected:**
  * **S1:** every task after `142.061-T`. R10's W4 finding says their
    real bodies pass through upstream S1 code, unless the architect shows
    (a-1) holds.
  * **S2:** every task after `142.054-T` whose real body passes through
    pending S2 code.
  * **S4:** any PA-5 task that uses (a-2).
* **Not affected:** S3 ((a-1) and (t)), S4's HC and S5's FL.
* **Operator options:**
  1. **(Recommended.)** Authorize a narrow harness-rule change to
     build-feature, as a separate Stage-to-Ship change outside 142-F. It
     would allow exactly one pre-implementation commit that replaces the
     current task's own recorded placeholders with the harness-architect's
     recorded real bodies, byte-identical to the recorded hash, and no
     other test edit. R11.3 works unchanged under that rule.
  2. **Re-split S1 and S2** so that no RED test has a pending
     in-shipment upstream on its path, about one shipment per dependency
     layer. Every task then uses (a-1). No rule changes, but there are more
     shipments and PRs.
  3. **Route (b):** a recorded Constitution II deviation. Not chosen on
     2026-09-29.
* **Until the operator decides,** claim gate CG-H holds S1 and S2
  (R11.2). So no shipment reaches the halt mid-run.

#### R11.4 Layout delegation (attempt-9 P1-2, P2-7; Learnings P3)

**Rule.** The plan no longer pins names for **new** files, module paths
or test targets. At each shipment's Step 2, the harness-architect picks
them under the four binding constraints LD1-LD4 below (decision 3's
(i)-(iv)). Any constraint that can't be met is a HALT to Stage at Step 2,
before any implementation.

**No longer pinned (withdrawn names).**

* W1: `src/services/generations/candidate_publish.rs`, and the downstream
  item-text path `candidate_publish::open_generation_store` (and the two
  sibling functions).
* W2: `src/preflight_probe/mcp.rs`, and the `mod mcp;` /
  `pub use mcp::probe_mcp_read;` lines.
* W3: the targets `integration_read_server_request_entry_admission` and
  `integration_preflight_probe_mcp`, their files, and the two
  `tests/helpers/*_fixture.rs` names.
* W7 and W8: the empty-module file names, and where each `#[path]`
  include sits.
* New-file names in the S1 and S2 item text are indicative only.

**Still pinned** (an operator decision or a pinned harness needs them, or
the file already exists):

* FL's file `tests/contract/read_server_cli_mcp_parity_test.rs` (R10.2).
* HC's three files and target A (R9.3).
* S3's existing scripts and tests: `start.ps1`, `start.sh`,
  `tests/contract/start_launcher_test.rs` and
  `tests/contract/start_launcher_failure_test.rs`.
* Existing files named in item text, such as
  `crates/engram-indexer/src/preflight.rs`, which already exists
  (attempt-9 Rust P3). An existing file is extended, not re-created.
* User-visible names: `engram preflight`, `engram-indexer preflight`, and
  every CLI, MCP and IPC name. These are contract, not layout.

**Binding constraints.**

* **LD1 (i), shared lines land at Step 2 only.** Every line in a file that
  more than one task touches lands in a Step 2 harness commit (G2). This
  covers `[[test]]` stanzas, `mod` / `pub mod` registration lines,
  `Cargo.toml` changes and `lib.rs` / `mod.rs` edits.
  * For an (a-2) task, a registration line points at an empty module file
    (L2). For an (a-1) task, it points at a file holding L3 stubs.
  * A `pub use` of an item that doesn't exist yet never lands at Step 2
    (L2). If a re-export is needed, it goes in a file that the task alone
    owns, in its implementation. Otherwise, HALT.
* **LD2 (ii), disjoint ownership and no upstream edit** (P1-2).
  * After the G1 point, an implementation edits only files that no other
    pending task in the shipment declares (G2).
  * A later task never needs to edit an earlier task's file.
  * **Visibility.** Private fields are visible to the defining module and
    its children, not its siblings. When a later task reads data that an
    earlier task defines, the architect does one of two things:
    * the earlier task's Step 2 signatures expose it as a live item
      (L3: `pub` on a public module path, so never `dead_code`); or
    * the later task's code lives in a **child** module of the earlier
      task's module, registered at Step 2 under LD1.
  * A sibling module that reads private fields is never a valid layout.
  * Any upstream-scope check in item text (for example, PRE-2's "only
    `starts_with` in `candidate_build.rs`") is read against the file the
    architect records.
* **LD3 (iii), no shared test file between tasks** (the W3 class).
  * Each task's RED tests live in a test file that it alone owns. A later
    task never extends an earlier task's test file.
  * Fixture code that a later task needs goes in a helper file owned by
    the earlier task. The later task `#[path]`-includes it in its swap
    commit or its implementation, never at Step 2 (L1).
  * If two tasks share a **target**, G3 name-scoping applies.
  * **Expected count (P2-7).** Every `harness_cmd` records its expected
    test count. A run whose output shows any other count, zero included,
    is a HALT.
* **LD4 (iv), scope stays fixed, and the record is authoritative.**
  * Layout choices change no scenario, acceptance criterion, user-visible
    name or behavior.
  * A task's implementation file count must not exceed its item text's
    count. If it must, HALT to Stage.
  * The Step 2 harness record lists, per task: owned files, module
    paths, target names, `harness_cmd`, expected test count, RED form
    (R11.3) and markers.
  * **Condition 4 baseline.** G1 baselines the **union** of the item
    text's declared files and the record's owned files. An item-text file
    the architect no longer uses stays unchanged, so it matches its
    baseline.
  * Ship never edits item text. At assembly, Stage adds one pointer line
    to each S1 and S2 task (R11.9 step 5).
  * *Revision 12 note:* the "union" baseline above is withdrawn. The
    condition-4 baseline is each task's owned files in the Step 2 record,
    and LD2's "declares" means the record (R12.2 O2-O4, after edits
    A1-A5). LD2's visibility rule is tightened, LD4 gets a named checker
    (R12.8, P2-5 and P2-6), and the pointer line is A15 (R12.5).

**What happens to W1-W9.**

| W | Revision 11 status |
|---|---|
| W1, W2, W3 | Findings stand as LD2 and LD3. The pinned names are withdrawn. |
| W4 | Superseded by R11.3 (route (a) forms). |
| W5 | Unchanged (G6). |
| W6 | Superseded by R11.2 (the label is kept, and the task is re-qualified). |
| W7 | Absorbed by LD1. |
| W8 | Absorbed by LD3 and L1. The includes land in the swap or implementation, never at Step 2. |
| W9 | Settled by R11.6 (P2-3). |

#### R11.5 Unexpected failures follow P-021 (attempt-9 P1-3, P2-4)

**Withdrawn in full: R10.3 P2-9.** This covers (1) the claim-time base run
of `cargo dev-test --no-fail-fast`, (2) the single-target
classification re-run, and (3) "a second unmapped failure goes to
P-021 intake". The R10.5 "Common to every shipment" sentence that puts
the P2-9 base run before Step 2 is withdrawn too. Step 1 stays exactly as
Ship defines it (P-001 and `cargo check`).

**What governs instead: Ship's unchanged text.**

* **Step 4.3** (the paragraph after the six conditions): an unexpected
  in-scope failure goes back to build-feature for a fix iteration, after
  P-021 scope classification. An out-of-scope failure follows Step 4.4a,
  and it is not fixed in the current task.
* **Step 4.4a:** P-021 C1 classification when the failure first occurs.
  Then deferred-entry discovery over the active stash, the archived stash
  and the residual-risk records. Then reuse of a confirmed entry, or C2
  capture under the single-write invariant.
* **The verdict** comes only from a complete `cargo dev-test
  --no-fail-fast` run that meets `PASS`, or all six Step 4.3 conditions.
  The plan adds no re-run allowance, no flake path and no waiver.
* **If an out-of-scope failure still stops a gate** after capture, Ship
  halts under its own rules. The plan adds no path around that halt.

**Plan-specific classification notes** (these apply C1; they add no
rule).

* **(t) F52 at F51's gate.** An F52 test still RED at `142.055-T`'s gate
  is in scope for F51, because it is the same fail-closed launcher
  contract surface (R11.3 (t)). It goes back to F51's build-feature. It is
  never mapped, because F52's tests are expected GREEN once F51 lands.
* **A HALT defined by this plan** (G4, HC, FL, LD1-LD4, R11.3) is a HALT to
  Stage. It is not a P-021 finding and not a C2 capture.
* **Out-of-scope 142-S flakes.** Any known 142-S flake that surfaces is
  classified by C1 like any other failure. It is not pre-waived.

**Kept from R10.3.** The P2-10 bounded IPC retry inside the PRE-3F
fixture (test-internal, not a gate rule), and the "Full outputs" rule
(every run named in this plan keeps its complete output).

**Attempt-9 P2-4 is settled** because the base run no longer exists.

#### R11.6 Attempt-9 P2 and P3 dispositions

Decision 5: a P2 or P3 is fixed here only when it would block a Ship gate.
Every P2 and P3 gets a disposition.

**P2 (11).**

| P2 | Blocks a gate? | Disposition |
|---|---|---|
| **1** G5 body fails `-Dwarnings` | yes (L4) | **Resolved** by R11.3 L1: the placeholder is one `unimplemented!` expression, with no item beside it. |
| **2** G2 stubs have no lint recipe | yes (L4) | **Resolved** by R11.3 L2 and L3, and LD1 (no premature `pub use`). |
| **3** W9 moves an AC; premise unclear | yes (cond. 4 at F51) | **Checked read-only (R11.1).** The fail-open tests are on `main` (`start_launcher_test.rs`, assertions at lines 80 and 154). `5760b948` (F52) touched only `start_launcher_failure_test.rs`. F51's parked harness `1dfc1b5b` replaced the fail-open tests with fail-closed ones. So the rewrite is F51 Step 2 harness content, re-applied under R11.3 (a-1). **AC transfer (recorded):** F52's AC4 ("the fail-open assertion is rewritten") moves to `142.055-T`. F51's existing approval surface (RS4/RS5) is unchanged. F52's note "reversed in `contract_start_launcher`" refers to F51's commit, and assembly corrects its attribution. **Corrected sentence:** R10.5's "they do not change scope" becomes "W9 moves one acceptance criterion between two 142-F tasks. The feature's scope is unchanged." Item text: R11.9 step 5. |
| **4** base run has no flake path | yes | **Resolved:** the base run is withdrawn (R11.5). |
| **5** FL instructions live only in the plan | yes (Step 2 could reuse `7bd9e504`) | **Fixed at assembly** (R11.9 step 5). `142.058-T`'s item text gets three points: (1) the S5 harness is exactly FL plus its red-phase record (R10.2); (2) the `7bd9e504` and `6d216d19` records and "red phase CONFIRMED" are historical, and they must not be reused as the S5 disposition; (3) any deviation is a HALT to Stage. V3, V4 and V8 are read as S5 Step 2 harness work. |
| **6** G1 baseline not tied to each task's record | yes (cond. 3 and 4) | **Resolved.** Each task's Step 2 harness record cites the G1 record (its commit SHA and per-file content hashes) for its own owned files (LD4 union). A per-commit test-file hash is allowed as the baseline form. A mismatch between the two records is a HALT. |
| **7** name-scoped `harness_cmd` can pass with zero tests | yes (cond. 1) | **Resolved** by LD3's expected count (R11.4). |
| **8** P-001 impact wrong while tasks are `active` | no | **Restated** in R11.2. |
| **9** Constitution Check stale | no | **Recorded** in R11.8's Constitution Check addition, including the HC rejected alternative. Applied to the Constitution Check section at assembly. |
| **10** resets bypass the operator halt | yes (Step 0.5 item 1a) | **Resolved** by the 2026-09-29 approval (R11.1, R11.2) and R11.8's supersession row for W6. |
| **11** FL may inherit `ENGRAM_DATA_DIR` | yes (FL isolation) | **Checked (R11.1):** the blob strips it at all three sites. **New provenance check (d2)** in R9.5 item 1 at S5 Step 2: FL's F54 file calls `env_remove` for `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE` and `ENGRAM_DIRECT` at every child-process `Command`. If any is missing, HALT before any commit. |

**P3 (17).**

| Source | P3 | Disposition |
|---|---|---|
| Rust | IC "no failure" bullet (~1666) has no inline note | Governed by R10.1 "IC gates, restated". Listed in R11.8. No edit. |
| Rust | W3 helpers need W8's `dead_code` rule | **Fixed (lint blocker).** W8's scoped `#[allow(dead_code)]` is withdrawn, because R11.3 bans `allow`. A `#[path]` include is allowed only when the including crate uses every included item. Prefer the crate's public API to including a production file. Otherwise split the helper, or HALT to Stage. |
| Rust | FL blob attributes versus the P2-5 ban | **Moot.** The blob has no `allow`, `expect` or `cfg_attr` (R11.1). The ban applies to lines a task adds. |
| Rust | strip all four variables | **Fixed.** The PRE-3F fixture strips the same three `ENGRAM_*` variables FL strips. `CARGO_BIN_EXE_engram` is a compile-time `env!` value, not a child variable. |
| Rust | P2-10 `tokio::time::sleep` and exact busy/locked markers | Delegated to the harness-architect (R11.4). An async retry uses async sleep. The busy and locked texts are recorded in the HC harness record. |
| Rust | W6 `preflight.rs` already exists | **Settled** by R11.4's "still pinned" list: it is extended, not re-created. |
| Scope | owned-file counts over 3 (PRE-2, PRE-3, PRE-5, PRE-6) | **Open.** LD4 stops any growth. The scoped review may re-check. No Ship gate reads the count. |
| Scope | "about 400 lines" in the HC bound | **Fixed.** It reads "more than 400 new lines". |
| Scope | W1's downstream item IDs not listed | **Moot.** The W1 names are withdrawn (R11.4). LD2 is read against the harness record. |
| Architecture | FL halt leaves S5 `active` and P-001 blocked | **Fixed.** The halt token is `FL-RED-HALT <FL SHA>`. Operator options: (1) a Stage replan of FL under a new decision; (2) retire S5 under the T0 rule (R10.3); (3) another recorded decision. Until then, P-001 stays blocked, as R11.2 says. |
| Architecture | IC row if PA-5 is empty or docs-only | **Fixed.** If PA-5 has no code task, `142.063-T` is S4's final code task, and its verdict is `PASS` only. |
| Architecture | G6 filters by "T" suffix | **Settled** by R11.2: CG-Q reads `artifact_type`, and manifests hold no subtask. |
| Architecture | `142.059-T` verification plan lacks commands | **Open.** Ship records or supplements it at Step 2 item 3 (Ship-owned). |
| Learnings | `idle_timeout_minutes` parser not found | **Settled.** `PluginConfig` parses it (R11.1). |
| Learnings | no full-suite runtime budget | **Open.** It is not a gate. |
| Learnings | plan-detail churn | **Settled** by R11.4. |
| Learnings | backlog changes must reach `main` | **Settled** by CG-M (R11.2). |

The attempt-8 P3s on the oracle at HC and the ruleset check stay open.

#### R11.7 Ship walkthrough S1 to S5, re-walked (attempt-9 P1-1)

Stage re-walked each shipment through `.github/agents/_ship.agent.md`
(read-only): Step 0.5 items 1a, 1b, 2, 3 and 3a; Step 1; Step 2; Step 3;
Step 4.3; and final PR readiness. This replaces the R10.5 walkthrough
result.

**Item 1a check (all shipments).** Ship filters `custom_fields.items` by
`artifact_type` and scans task artifacts only. So `142-F` at the head of
S5 is excluded, and no false halt occurs. *(Revision 14: `142-F` is no
longer an S5 member; R14.2.)* After the six resets
(R11.9 step 3), every manifest task is `queued`, so item 1a passes. The
five `active` subtasks are not members (R11.2), so item 1a never reads
them.

**Full claim gates.** Every claim also needs P-001, the T0 remote check
(R9.9), CG-Q and CG-M (R11.2).

*Revision 12 note:* every claim also needs CG-S, and CG-M is read as
reworded (R12.6). The S1 and S2 Step 2 rows also need the R12.2
ownership check (O1-O4), R12.3 mechanism (b) for `142.069-T`,
`142.070-T` and `142.072-T`, and the R12.4 frozen-item list (R12.7).

*Revision 13 note:*

* The R13.3 `engram-indexer` gate commands apply at every S2 code task
  and in `142.073-T`'s final criteria.
* PRE-4 (`142.066-T`) is F3 in the R13.6 feasibility pass (Q3 open).
* Subtask closure after the S1, S2 and S5 merges follows R13.4.
* Claim gates are unchanged.

*Revision 14 note:* subtask closure now runs after Ship's Step 6 (R14.2),
and every S2-S5 claim also needs CG-T. `142-F` leaves the S5 manifest, so
S5's Step 0.5 row no longer mentions it, and its Final row reads
"`142-F` closes at CP-S5 (R14.2), not at Ship Step 6". PRE-4
(`142.066-T`) uses the Q3 (ii) placeholder (R14.3).

| Shipment | Blocked by | Extra claim-gate items |
|---|---|---|
| S1 | none | the R11.9 step 5 pointer lines are on S1 tasks (CG-H withdrawn, R11.10) |
| S2 | S1 shipped | `142.054-T` carries the STATUS-RESET comment; pointer lines are on S2 tasks (CG-H withdrawn, R11.10) |
| S3 | S2 shipped | a recorded PA-6 decision lifts the hold (R10.3 P2-4); the W9 AC-transfer item text is applied; `142.055-T` to `142.057-T` carry STATUS-RESET comments |
| S4 | S3 shipped | the PA-5 tasks exist and are in the manifest; E7 recorded; the hold is lifted (CG-P withdrawn; PA-5 input (ii) is the R11.10 ordering rule) |
| S5 | S4 shipped (including its P-001 closure) | E6 recorded; the R9.9 drift ledger against `<S5 base>`; `142.058-T` and `142.059-T` carry STATUS-RESET comments; the FL item text (R11.6 P2-5) is applied |

**S1 (`142.061-T` → … → `142.068-T`).**

| Ship step | Result |
|---|---|
| 0.5 item 1a / 1b / 2 / 3 | All seven tasks `queued`; shipment `queued`; task-only manifest; every task's parent is `142-F`. Pass. |
| 0.5 item 3a | Branch from `main`, and the worktree is clean (CG-M). Pass. |
| 1 | P-001 (no other release unit active), `cargo check`. Pass. |
| 2 | Every task is queued with no disposition, so each goes to the harness-architect in dependency order. `142.061-T` uses (a-1). The rest use route (c) (R11.10): real bodies whose first call is the task's own placeholder, or a named fallback (F1)-(F3). Layout under LD1-LD4. L4 record for each harness commit. |
| 3 | Task artifacts only (G6), in dependency order. |
| 4.3 | Non-final tasks: `PASS`, or `EXPECTED_PENDING_RED` onto later S1 tasks (R10.1). Each later (c) test fails with its own recorded marker until its own implementation. No test file is edited in Step 4.2 (R11.10). |
| Final | `142.068-T` `PASS`; PR readiness `PASS`. |

**S2 (`142.054-T` → `142.069-T` … `142.073-T` → `142.074-T` docs).**

| Ship step | Result |
|---|---|
| 0.5 item 1a to 3a | `142.054-T` is `queued` after its reset; the others are `queued`. Pass (CG-Q, CG-M). |
| 1 | Pass. |
| 2 | `142.054-T`: queued with `harness-ready`, but its evidence (`c269fa79`, `41dd5081`) is not on the S2 base and is GREEN, so it is invalid (R11.2). The architect rebuilds it with (a-1), including the PA-2b real-chain RED case, and extends the existing `preflight.rs`. `142.069-T` to `142.073-T` use route (c) or (a-1) (R11.10). `142.071-T` keeps its recorded crate-local `harness_cmd`, with an expected count (LD3). `142.074-T` is docs-route, qualified by its verification plan. |
| 3 | Task artifacts only, in dependency order. |
| 4.3 | As in S1 (route (c); no Step 4.2 test edit). |
| Final | `142.073-T` `PASS`; `142.074-T` passes its recorded gates; PR readiness `PASS`. |

**S3 (`142.055-T` → `142.056-T` → `142.057-T` → `142.060-T`).**

| Ship step | Result |
|---|---|
| 0.5 item 1a to 3a | F51-F53 are `queued` after their resets; `142.060-T` is `queued`. Pass. |
| 1 | Pass once the hold is lifted. |
| 2 | F51 (a-1): the architect re-applies the content of `1dfc1b5b` from T0 (fail-closed tests replace the fail-open tests in `start_launcher_test.rs`; AC4 transferred). F52 (t): `5760b948` content, RED against the pre-F51 `start.ps1`. F53 (a-1). `142.060-T` (a-1): its evidence is checked against the S3 base; if it is not on the base tree, the architect rebuilds it. |
| 3 | F51, F52, F53, then `142.060-T`. |
| 4.3 | F51: `PASS`, or `EXPECTED_PENDING_RED` onto F53 and `142.060-T` only. F52's tests must be GREEN (R11.5). F52: `PASS`, because it has no implementation of its own. F53: `PASS` or `EXPECTED_PENDING_RED` onto `142.060-T`. Owned files are disjoint after W9. |
| Final | `142.060-T` `PASS`; PR readiness `PASS`. |

**S4 (`142.063-T` → PA-5 tasks).**

| Ship step | Result |
|---|---|
| 0.5 item 1a to 3a | All members `queued` (PA-5 tasks are created `queued`). Pass. |
| 1 | Pass once the hold is lifted. |
| 2 | HC first (pinned, (a-1), HC bound of 400 lines). Then each PA-5 harness under route (c) (R11.10) and the R10.5 S4 planning inputs. |
| 3 | `142.063-T`, then PA-5 in its plan order (E7). |
| 4.3 | IC: `PASS` or `EXPECTED_PENDING_RED` onto PA-5 only. If PA-5 has no code task, IC is `PASS` only (R11.6). |
| Final | The last PA-5 code task `PASS`; PR readiness `PASS`. |

**S5 (`142-F` head; `142.058-T` → `142.059-T`).** *(Revision 14:
`142-F` removed from S5; see the note above the claim-gate table and
R14.2.)*

| Ship step | Result |
|---|---|
| 0.5 item 1a to 3a | `142-F` is excluded by `artifact_type`. `142.058-T` and `142.059-T` are `queued`. Pass. |
| 1 | Pass once the hold is lifted and S4 has shipped. |
| 2 | `142.058-T`: evidence invalid (R11.2), so the pinned FL route runs: provenance (a), (b), (c), (d) and (d2), then FL, then the red-phase record (R10.2). FL uses (a-1). A failure is `FL-RED-HALT` (R11.6). `142.059-T`: docs-route, re-qualified `harness-verification-gated` with its verification plan recorded or supplemented. |
| 3 | `142.058-T`, then `142.059-T`. |
| 4.3 | `142.058-T` is the only code task: `PASS` after `.002-ST`, `.003-ST` and M12. |
| Final | `142.059-T` passes its gates; PR readiness `PASS`; `142-F` closes at Step 6. |

**Walkthrough result.**

* Under Ship's unchanged rules, every S1-S5 gate can legally pass once
  R11.2 to R11.6 and R11.10 are applied at assembly.
* One hold stays by design: the PA-6 hold on S3, S4 and S5. (CG-H and the
  H-R11-1 hold on S1 and S2 are withdrawn by R11.10.)
* While the PA-6 hold stands after S2 ships, P-001 blocks every other
  release unit (R11.2).
* The layout choices (LD1-LD4), W9's AC transfer, and every route (c)
  fallback (F1)-(F3) named at Step 2 are recorded changes for the scoped
  review to check.

#### R11.8 Supersession table, edges and Constitution Check addition

**Superseded text (Revision 11 wins where these disagree).**

| Earlier text | Revision 11 replacement |
|---|---|
| R6.2 S1 "(3 ST)" and S2 "(3 ST)" member listings; R10.4 addition 1 "its two open subtasks" | Task-only manifests. Subtasks are never members, and they run inside their parent's Step 4.2 (R11.2). |
| R10.1 G5, the two methods | R11.3 forms (a-1), (a-2) and (t), with markers and L1-L4. The G5 aim (marker independence) is kept. |
| R10.2 assembly step 1 (remove `harness-ready` from `142.058-T`) | The label is kept. The task is reset and re-qualified at Step 2 (R11.2). |
| R10.2 amended (b), "assembly removes [the label] on purpose" | (b) checks that the historical records are unchanged. The label is kept (R11.2). |
| R9.5 item 1 (a) to (d) | Adds (d2), the FL environment-strip check (R11.6 P2-11). |
| R10.3 P2-3 "more than about 400 new lines" | "more than 400 new lines" (R11.6) |
| R10.3 P2-9, all three parts | Withdrawn. Ship's P-021 path governs (R11.5). |
| R10.3 P3 "P-001 impact of the hold" | R11.2 restatement |
| R10.4 claim-gate column (S1, S2, S3, S5) | R11.7 full claim gates (CG-Q, CG-M, CG-H, CG-P) |
| R10.4 S2 and S5 "`queued` with no `harness-ready` label" | `queued`, label kept, STATUS-RESET comment (R11.2) |
| R10.4 addition 3 (resets of `142.058-T` and `142.054-T` only, labels removed) | Six operator-approved resets, labels kept (R11.2, R11.9 step 3) |
| R10.5 "Common to every shipment": the P2-9 base run before Step 2 | Withdrawn (R11.5) |
| R10.5 W1, W2, W3, W7 and W8 pinned names; W8's scoped `#[allow(dead_code)]` | R11.4 (LD1-LD4). The `allow` is withdrawn (R11.6 Rust P3). |
| R10.5 W4 | R11.3 |
| R10.5 W6 (label removed; reset without approval) | R11.2 (approved 2026-09-29; label kept) |
| R10.5 W9 and "they do not change scope" | R11.6 P2-3 (recorded AC transfer; corrected sentence) |
| R10.5 S3 "F51, F52 and F53 are `active` with valid parked RED harnesses. Step 2 reuses them." and "`142.060-T` … is reused" | They are `queued`, and their evidence is re-qualified against the S3 base (R11.2, R11.7). |
| R10.5 walkthrough result | R11.7 walkthrough result |
| R10.6 Constitution Check addition | The addition below |
| R10.1 IC gates, the "no failure" bullet (~1666) | Unchanged. It is read with R10.1 "IC gates, restated" (R11.6 Rust P3). |

**Edges: unchanged.** Assembly applies E5, E1, E2, E3, E4 and E8, with a
cycle check after each, and E4 before E8. The PA-5 harvest applies E6
and E7. Revision 11 adds and removes no edge. Leaving subtasks out of the
manifests changes no edge, because every edge in E1-E8 is between tasks.

**Constitution Check addition (applied to that section at assembly;
replaces R10.6's addition and settles P2-9).**

* **II Test-first: no exception.** Every real test is seen failing before
  its task's implementation: at Step 2 for (a-1) and (t), and at the swap
  for (a-2) (R11.3). H-R11-1 stays open, and CG-H holds S1 and S2 until
  the operator decides. Route (b) was not chosen.
* **Task granularity: the HC deviation.** HC is the Step 2 harness of
  `142.063-T`, capped by the HC bound (at most 2 hours and 400 lines).
  IC is the Step 4.2 implementation, capped by the 2-hour rule. They are
  separate phases, so neither goes over 2 hours.
  * **Rejected alternative:** making HC its own task. A harness-only task
    has no implementation to take a Step 4.3 verdict, and Ship's Step 2
    must still harness `142.063-T`.
* **Status resets:** six, operator-approved on 2026-09-29, with labels
  kept (R11.2).
* **P-021:** Ship's contract is unchanged (R11.5).
* **P-001:** while the PA-6 hold stands after S2 ships, every other
  release unit is blocked (R11.2).
* **VII:** FL is never amended, reset, reverted or pushed without an
  operator decision. `FL-RED-HALT` names the options (R11.6).
* **Stage template deviation:** subtasks are not manifest members, which
  follows Ship's task-only contract (R11.2).

**Scope limits.**

* This session changed only this plan, `docs/operator-glossary.md` and the
  Revision 11 memory file.
* No backlog item, edge, shipment, stash entry, source, test, `Cargo.toml`
  or config file was changed. No tag, and no git mutation. Every git and
  file read was read-only.
* Assembly is not done here. PA-5 is not planned here. The PA-6 hold is
  unchanged. H-R11-1 is open.
* **Next step:** ONE scoped plan review of Revision 11 and the text it
  touches. Assembly waits for that review to pass.

#### R11.9 Assembly steps (recorded; NOT done here)

This list replaces R10.4's "Assembly additions". Stage runs it in one
later session, in this order, under its own backlog authority. A failed
check is a HALT to the operator. Nothing is repaired automatically.

*Revision 12 note:* R12.7 amends these steps:

* step 1: the gating review is of Revision 12;
* step 2: the post-H2 exemption (R12.8, P2-7);
* step 3: the reset comment uses A16;
* step 5: adds A1-A19, with A15 replacing the pointer line;
* step 9: CG-S applies;
* step 10: the staging PR (CG-M, R12.6), then a re-read under CG-S.

*Revision 14 note:* R14.9 adds step 2a (read-only subtask-move check),
A23-A26 and the revised A11/A22 at step 5, S5 as `items: [142.058-T]`
then `142.059-T` at step 6, and the R14.3 deviation at step 8. Step 10
uses the R14.4 rebuild.

1. **Preconditions.** The scoped review of Revision 11 is `PASS`, or
   `ADVISORY` with operator confirmation. H2 is done (142-S abandoned),
   and T0 exists locally and on the remote (R9.9).
2. **Read-only re-check.** Re-read the statuses and labels in R11.1. If
   any item has changed, HALT before any write.
3. **The six resets** (R11.2). For each of `142.054-T`, `142.055-T`,
   `142.056-T`, `142.057-T`, `142.058-T` and `142.059-T`:
   * set `active` → `queued`;
   * append the STATUS-RESET comment, filled in;
   * keep the labels.

   Leave the five `active` subtasks, `142-F` and the 142-S record as they
   are.
4. **Edges.** Apply E5, E1, E2, E3, E4 and E8, with a cycle check after
   each, and E4 before E8. E6 and E7 wait for the PA-5 harvest.
5. **Item-text edits** (Stage authority).
   * **S1 and S2 tasks:** one pointer line: "Layout delegated (plan
     R11.4): new file, module and target names here are indicative; the
     Step 2 harness record is authoritative for owned files."
   * **`142.055-T` and `142.056-T`:** the W9 AC transfer (F52 AC4 moves to
     F51). F52's owned files narrow to
     `tests/contract/start_launcher_failure_test.rs`. F52's
     "reversed in `contract_start_launcher`" note is re-attributed to F51's
     `1dfc1b5b` (R11.6 P2-3).
   * **`142.058-T`:** the three FL points and the (d2) check (R11.6 P2-5
     and P2-11), plus the T0 location and the B0 path (R10.4 addition 5).
6. **Create the shipments.** Manifests hold tasks only, with no subtasks
   (R11.2).
   * **S1:** `142.061-T` first, then the rest in R10.4 order.
   * **S2:** `142.054-T` first, then the rest in R10.4 order.
   * **S3:** `142.055-T`, `142.056-T`, `142.057-T`, `142.060-T`.
   * **S4:** `142.063-T` only. The PA-5 harvest adds the rest.
   * **S5:** `items: [142-F]`, then `142.058-T` and `142.059-T`.
   * Copy the T0 location and the B0 path to the S5 shipment.
7. **Hold comments.** Add `PA-6-HOLD` plus the decision reference to every
   S3, S4 and S5 task and shipment (R10.3).
8. **Constitution Check.** Replace its stale Revision 9 row with the
   R11.8 addition, with its "II Test-first" bullet replaced by the R11.10
   "Constitution II" paragraph (route (c); H-R11-1 closed, CG-H
   withdrawn).
9. **Verify.** Read back each manifest (`backlogit shipment get`), and
   each member's status (`backlogit get`). Every task member is `queued`
   (CG-Q), and there is no subtask member. Report any discrepancy, and
   HALT.
10. **Land on `main` (CG-M).** Commit the backlog and plan changes to
    `main` with a clean worktree, then sync the index. Record the S1-S5
    IDs in the session memory file as the handoff tokens.

#### R11.10 Route (c) (operator, 2026-09-29 18:47 -07:00)

**Authorization.** Operator, verbatim: "route (c), review Revision 11".
The decision is recorded in the glossary row for H-R11-1. Route (c)
replaces route (a)/(a-2) as the fix for G5 and attempt-9 P1-4. It needs
no change to the harness rules, to build-feature or to Ship. This
subsection writes the rule only. The scoped review runs separately.

**Forms after R11.10.** Each code task's RED takes one of three forms. The
harness-architect records the form in the task's Step 2 harness record.

* **(a-1)** stays as R11.3 defines it: the real body fails first inside
  the task's own code, with no earlier pending code on its path. This is
  the normal form for the first code task of each shipment, and for HC
  and FL (pinned).
* **(c)** replaces (a-2) (below).
* **(t)** stays as R11.3 defines it (only `142.056-T` F52).

**Route (c), the procedure.**

1. **The real body is written at Step 2.** At the claim, the
   harness-architect writes each RED test's **real** body in the task's own
   test file, in the task's Step 2 harness commit.
2. **The first call is the task's own placeholder.** The first call that
   runs is to the task's **own** placeholder item: an L3 stub in the task's
   own files. It panics with the task's marker. The real assertions come
   after it.
3. **Implementation turns it GREEN.** build-feature implements the
   placeholder (and the rest of the task). It never edits the test file
   (`.github/skills/build-feature/SKILL.md` ~211 and ~250). Implementing
   the placeholder is what lets the test reach its real assertions and
   pass.
4. **No test edit after Step 2.** The test file is owned by its task and
   is never edited after its harness commit. So the G1 baseline
   (condition 4) holds, and the recorded marker (conditions 3 and 5)
   stays the same until the task's own implementation.

**The ordering rule (G5 under route (c)).** Before its own placeholder
call, a test runs **nothing** that executes pending in-shipment code of an
earlier task. That covers:

* a function, method, constructor or trait call;
* a fixture helper (including an earlier task's helper file);
* a child process, daemon or binary start whose code includes a pending
  earlier task;
* building the placeholder's arguments.

The arguments must come only from base-tree code, literals, temporary
paths, or other items of the task's own. Naming an earlier task's types,
or importing its items, is allowed, because that runs nothing. After the
placeholder call, the test may use any code. By the time the placeholder
is implemented, every earlier task in the shipment is done (dependency
order).

**The placeholder item.**

* It is part of the task's final public API as the item text describes it,
  with its final signature, and it is `pub` on a public module path (L3
  "only live items").
* It never returns `!`, so code after the call is not `unreachable_code`.
* A production item added **only** so that a test has something to call
  first is not allowed.

**When the ordering is impossible for a test.** The architect names the
test in the Step 2 harness record, with its reason, and applies the first
fallback that works:

* **(F1)** Call a different own-task item first: one from the task's
  final API whose arguments need no earlier pending code. The architect
  records which item.
* **(F2)** The test belongs to the earlier task. Moving it is a Stage
  item-text change, so this is a HALT to Stage with the test named and
  the proposed move. A move is only valid if the test is inside the
  earlier task's recorded scenarios and acceptance criteria. Otherwise, it
  goes to (F3).
* **(F3)** HALT to Stage at Step 2, before any implementation, with the
  test named.

**Tests the architect should check first** (the R10.5 W4 paths; recorded,
not decided):

* PRE-2 (`142.062-T`): its tests need a sealed candidate from PRE-1.
* PRE-3, PRE-4b, PRE-4 and PRE-5 (`142.064-T`, `142.065-T`, `142.066-T`
  and `142.067-T`): their tests need a generation published through
  PRE-2.
* PRE-6 (`142.068-T`): its tests need PRE-5's probe fixture.
* NEW-1, NEW-2 and NEW-4 (`142.069-T`, `142.070-T` and `142.072-T`):
  their tests need F50's `preflight.rs` types and functions.

For each, the architect either finds a first own call that needs no
earlier pending code, or applies (F1)-(F3).

**Markers (Ship condition 3).** For (c), the marker is exactly
`Worker: <task-id> <item>`, taken from the placeholder's `unimplemented!`
text. It is recorded per test, with the exact test name. The (a-1) and
(t) marker rules in R11.3 are unchanged, and a generic `Worker`
substring is still not a marker.

**Lint-safe rules, adapted.** There are no `allow`, `expect` or `warn`
attributes, whether written directly or through `cfg_attr`. `deny`,
`forbid` and `#![forbid(unsafe_code)]` stay allowed.

* **L1 (replaced): the real test body.**
  * Every `use`, helper, const and `#[path]` include in the test file is
    used by a test body, so `unused_*` and `dead_code` can't fire.
  * Code after the placeholder call compiles and is reachable to the
    compiler, because the placeholder returns its final type.
  * The test name, attribute and body are final at Step 2.
* **L2 (replaced): Step 2 production content.** For a (c) task, this is
  the task's own L3 stubs, plus the LD1 registration lines. An empty
  module file stays allowed.
* **L3: unchanged.** It now applies to every Step 2 stub:
  * `let _ = (<params>);`, then `unimplemented!("Worker: <task-id> <item>")`
    last;
  * `std::future::ready(()).await;` in async stubs;
  * `# Errors` and `# Panics` docs;
  * live items only.
* **L4 (adapted).** The Step 2 record needs `cargo check --all-targets`,
  `cargo lint` and `cargo fmt-check` to exit 0, and the recorded
  failures: each (c) test fails with its own marker, and the run shows
  the expected test count (LD3). The (a-2) swap record is withdrawn.
* **LD3 includes.** A later task's `#[path]` include of an earlier task's
  helper file lands in the later task's Step 2 harness commit, not in a
  swap or an implementation. The earlier task's helper is test code, so
  it is final at the earlier task's Step 2. The ordering rule applies to
  the helper's calls.
* *Revision 12 note:* a test never `#[path]`-includes another task's
  production file. Tests of `engram-indexer` modules are crate-local
  under `crates/engram-indexer/tests/` (R12.3). Earlier-task signatures
  that a later test names are frozen (R12.4). The L3 recipe is corrected
  (R12.8, P2-4), and so are "first call" and "after the placeholder"
  (R12.8, P3).

**Constitution II.** Every real test is written before its task's
implementation, and it is seen failing at Step 2: with its own marker for
(c), and at the task's own code or assertion for (a-1) and (t). It is
never edited afterwards. So the assertions that pass at the gate are
exactly the ones written first.

* **Residual, for the scoped review.** Under (c), the real assertions
  run for the first time after the placeholder is implemented. The
  failure seen before implementation is the placeholder's marker.
* The operator chose route (c) as the P1-4 resolution. No Constitution II
  deviation is recorded.

**What R11.10 withdraws or restates.**

| Earlier Revision 11 text | R11.10 replacement |
|---|---|
| R11.1 decision 2 (route (a), the swap) | Route (c). The decision-2 row is kept as history. |
| R11.3 (a-2), the swap, the (a-2) marker, L1, L2, the L3 "swap-commit placeholders", and the L4 "(a-2) swap record" | (c) and the adapted L1-L4 above |
| R11.3 "Constitution II (no exception)", the "at the swap for (a-2)" bullet | The Constitution II paragraph above |
| R11.3 H-R11-1 and its operator options | **Closed.** No Step 4.2 test edit exists under (c), so nothing conflicts with build-feature. |
| R11.2 CG-H | **Withdrawn.** S1 and S2 are no longer held. |
| R11.2 CG-P | **Withdrawn.** R10.5's S4 planning input (ii) is read as the R11.10 ordering rule for every PA-5 RED test. |
| R11.4 LD1 "For an (a-2) task … an empty module file (L2)" | A (c) task's registration line points at its L3 stub file (L2 adapted). |
| R11.4 LD3 "in its swap commit or its implementation, never at Step 2 (L1)"; the W8 row "in the swap or implementation" | At the later task's Step 2 harness commit (above) |
| R11.6 P2-1 and P2-2 "resolved by L1 / L2" | Resolved by the adapted L1-L4 |
| R11.8 rows and the Constitution Check II bullet naming (a-2), the swap, H-R11-1 or CG-H | This subsection. The claim gates are CG-Q and CG-M, plus the R11.7 rows. |

**Unchanged:** G1-G4, G6, LD1-LD4 (except the notes above), R11.5, the
resets, the edges, S1-S5 membership and order, the PA-6 hold, and the
R11.9 steps.

**Scope limits.** This follow-up changed only this plan and the Revision
11 memory file. The Orchestrator recorded the glossary change. No
backlog item, edge, shipment, stash entry, source, test, `Cargo.toml` or
config file was changed, and there was no git mutation. **Next step:**
the operator-authorized scoped review of Revision 11, including R11.10
(not run here).

### Revision 12: attempt-10 P1 fixes

Revision 12 is narrow. It closes attempt-10 P1-1 and P1-2, plus P2-2 and
P2-8 because they would block assembly. It gives every other attempt-10
P2 and P3 a one-line disposition. It doesn't reopen route (c), the
resets, P-021, the edges, S1-S5 membership, or the PA-6 hold. All
item-text edits below are **recorded here and applied at assembly**
(R11.9 step 5). None is applied now.

#### R12.1 Decision recorded (operator, 2026-09-29 20:04 -07:00)

* **Verbatim:** "Revision 12, then review".
* **Context:** plan-review attempt 10 of Revision 11 was `FAIL`, the
  sixth `FAIL` in a row (attempts 5-10). The operator accepted the
  Orchestrator's recommendation: a narrow Revision 12 that closes
  attempt-10 P1-1 and P1-2, plus P2-2 and P2-8, then one fresh-session
  scoped review. Stage doesn't run that review.
* **Still in force:** 18:14 "approve resets, route (a), delegate layout,
  Revision 11" and 18:47 "route (c), review Revision 11". Route (c)
  replaced route (a)/(a-2), with no Ship or harness rule change.
  Revision 12 doesn't change Ship, build-feature or harness-architect
  either.

#### R12.2 P1-1: owned files never include a file another task creates

**General rule (binding for assembly and Step 2).**

* **O1.** A task's Owned files never include a file that another task in
  the feature creates. LD1 registration lines (`mod`, `pub mod`,
  `[[test]]`, `lib.rs` and `mod.rs` one-liners) are shared lines that
  land at Step 2. They aren't ownership.
* **O2.** LD2's "a file another pending task declares" means the file
  named in that task's **Step 2 harness record**, not the item text.
* **O3.** The LD4 condition-4 baseline is **each task's owned files in
  the Step 2 record**. The R11.4 "union of item-text and record files"
  is withdrawn. After the edits below, no item text in the feature
  names a file another task creates, so the record and the item text
  agree on who owns each file.
* **O4.** If the architect finds any item text (after the R12 edits) that
  still names another task's file as owned, it halts to Stage at Step 2
  with `R12-OWNERSHIP-HALT <task> <file>`, before any implementation.
  Stage fixes the item text under its own authority.

Stage checked every 142-F task's Owned files line
(`.backlogit/queue/142.054-T.md` to `142.074-T.md`). The only O1
violations are the two in attempt-10 P1-1. The shared one-line entries
(`crates/engram-indexer/src/lib.rs` in `142.069-T` and `142.070-T`,
`src/services/generations/mod.rs` in `142.061-T`, `src/lib.rs` in
`142.067-T`, `src/cli/commands/mod.rs` in `142.072-T`) are LD1 lines.
The F51/F52 overlap on `start_launcher_test.rs` is settled by W9
(R11.9 step 5).

**Assembly edits A1-A5** (item text; the "from" text is quoted exactly):

| # | Item, line | From | To |
|---|---|---|---|
| A1 | `142.062-T` line 23 | "Scope: three public functions in `candidate_build.rs`:" | "Scope: three public functions in PRE-2's own new file (named in the Step 2 harness record; never `candidate_build.rs`, which `142.061-T` creates and owns):" |
| A2 | `142.062-T` line 28 | "Owned files (production): `src/services/generations/candidate_build.rs`; `src/services/generations/publish.rs` (visibility only)." | "Owned files (production): the one new file named for PRE-2 in the Step 2 harness record (plan R11.4, R12.2); `src/services/generations/publish.rs` (visibility only). Its `mod` line lands at Step 2 (LD1)." |
| A3 | `142.062-T` line 38 | "`within_root` is the ONLY path `starts_with` in `candidate_build.rs` (reviewer check)." | "`within_root` is the ONLY path `starts_with` in PRE-2's recorded file (reviewer check)." The rest of the line is unchanged. |
| A4 | `142.068-T` line 23 | "Scope (owned production file `src/preflight_probe.rs` only):" | "Scope (owned production file: the one new file named for PRE-6 in the Step 2 harness record, with its module path and `pub mod` line recorded under LD1/LD2; never `src/preflight_probe.rs`, which `142.067-T` creates and owns):" |
| A5 | `142.068-T` line 52 | "HALT to Stage if a production file beyond `src/preflight_probe.rs` is needed." | "HALT to Stage if a production file beyond the recorded PRE-6 file is needed." |

With A1-A5, PRE-1 (`142.061-T`) and PRE-5 (`142.067-T`) edit only files
they own. Their changes don't touch the baselines of PRE-2's or PRE-6's
owned files, so the Ship Step 4.3 condition-4 check for the mapped
PRE-2 and PRE-6 tests passes. LD2's visibility rule (R12.8, P2-5) still
governs how PRE-2 and PRE-6 read PRE-1's and PRE-5's data.

#### R12.3 P1-2: no `#[path]` include of another task's production file

**Rule.**

* **P1.** A test never `#[path]`-includes a production file owned by
  another task. The (R10/R11) prescriptions to do so in `142.069-T`,
  `142.070-T` and `142.072-T` are withdrawn.
* **P2.** The Step 2 tests of `142.069-T`, `142.070-T` and `142.072-T`
  don't include their own task's production file either. They use the
  crate's public API instead.
* **P3.** `#[path]` includes of **test helper** files stay allowed under
  LD3 and R11.10.
* **P4.** There are no `allow`, `expect` or `warn` attributes (R11.10
  unchanged).

**Chosen mechanism: (b) crate-local tests inside `engram-indexer`.**

Stage checked this against the actual manifests:

* The root `Cargo.toml` has no `engram-indexer` dependency, so the root
  crate can't see it.
* `crates/engram-indexer/Cargo.toml` line 15 already has
  `engram = { path = "../.." }`. So a test in
  `crates/engram-indexer/tests/` can reach both `engram_indexer::…` and
  `engram::…`.
* Both paths are public: `engram-indexer`'s `src/lib.rs` has
  `pub mod preflight;`, and the root has `pub mod cli;` (`src/lib.rs`
  line 36) and `pub mod commands;` (`src/cli/mod.rs` line 7).
* `engram-indexer` tests are auto-discovered, so they need no `[[test]]`
  stanza. That means no shared `Cargo.toml` line.
* `142.071-T` already uses this pattern (item text line 27, R11.7), and
  so does `crates/engram-indexer/tests/supervisor_boundary_test.rs`.

**Why not (a).** Option (a) would add a root dev-dependency,
`[dev-dependencies] engram-indexer = { path = "crates/engram-indexer" }`.
That creates a dev-dependency cycle (root → `engram-indexer` → root). It
also changes the root `Cargo.toml` for three tasks, and it gains nothing
over (b).

**Effect on `-Dwarnings`.** A crate-local test uses library items
through their public paths. An unused `pub` item in a library is never
`dead_code`. So when F50 or NEW-1 later add items to `preflight.rs` or
`preflight_verdict.rs`, frozen Step 2 tests don't start failing. The
only remaining break is a changed signature, which R12.4 freezes.

**Test that can't use (b).** If a specific test can't be written through
public items named in its task's or an earlier task's item text, it
halts at Step 2 with `R12-TEST-PATH-HALT <task> <test>`, before any
implementation. Giving an item-text function `pub` visibility is allowed,
because it adds no item. Adding a production item only for a test is
still not allowed (R11.10).

**Existing own-file include (F50).**
`tests/integration/preflight_gate_test.rs` lines 5-6 already
`#[path]`-include F50's own `preflight.rs`, with no `allow`. P1 doesn't
cover this, because it's F50's own file and later S2 tasks don't edit
`preflight.rs`. The architect keeps it only if it is lint-clean at F50's
own gate without `allow`. Otherwise:
`R12-TEST-PATH-HALT 142.054-T integration_preflight_gate`.

**Assembly edits A6-A14:**

| # | Item, line | From | To |
|---|---|---|---|
| A6 | `142.069-T` line 28 | the whole "Harness (RED first): `tests/integration/preflight_verdict_test.rs` + `[[test]] name = "integration_preflight_verdict"`; `#[path]`-includes … `mod` declarations." line | "Harness (RED first, crate-local, auto-discovered; plan R12.3): one test file under `crates/engram-indexer/tests/`, owned by this task and named in the Step 2 harness record. It uses only the public paths `engram_indexer::preflight` and `engram_indexer::preflight_verdict`. `harness_cmd`: `cargo test -p engram-indexer --test <recorded name>`, with its expected test count. No `[[test]]` stanza, no `#[path]` include of any production file, and no `allow`/`expect`/`warn` attribute." |
| A7 | `142.069-T` lines 33 and 52 | "`cargo test --test integration_preflight_verdict`" | "`cargo test -p engram-indexer --test <recorded name>`". The rest of each line is unchanged. |
| A8 | `142.069-T` line 47 | "because the file compiles under the root-test `#[path]` include and inside `engram-indexer`." | "because the module's scope (line 22) is `std` and `crate::preflight` only." The deviation stays; only its reason changes. |
| A9 | `142.070-T` line 29 | the whole "Harness (RED first): `tests/integration/preflight_invocation_test.rs` + … scoped `#[allow(dead_code)]`." line | Same form as A6, with the public paths `engram_indexer::preflight`, `engram_indexer::preflight_verdict` and `engram_indexer::preflight_invocation`. |
| A10 | `142.070-T` lines 34 and 54 | "`cargo test --test integration_preflight_invocation`" | "`cargo test -p engram-indexer --test <recorded name>`". |
| A11 | `142.072-T` line 30 | "; `#[path]`-includes `preflight.rs` and `preflight_verdict.rs` (scoped `#[allow(dead_code)]`) to assert the closed stage set equals `stage_name` over all `Failure` variants." | ". The stage-set assertion is in a second test file owned by this task, crate-local under `crates/engram-indexer/tests/` and named in the Step 2 harness record (plan R12.3). It asserts that the relay's closed stage set equals `stage_name` over all `Failure` variants, using only public items named in item text. For example, for every `f`, `engram::cli::commands::preflight::normalize_verdict` accepts the `Failed` line for `stage_name(f)` with `Some(1)`. If equality can't be shown that way, the test halts with `R12-TEST-PATH-HALT 142.072-T <test>`. The test file has no `#[path]` include of a production file and no `allow`/`expect`/`warn` attribute." The assertion itself is unchanged (LD4). |
| A12 | `142.072-T` line 35 | "`cargo test --test contract_cli_preflight_relay` is GREEN, including the inherited-grandchild case with its non-vacuity assertion." | Append: "; and `cargo test -p engram-indexer --test <recorded stage-set test>` is GREEN." |
| A13 | `142.072-T` line 56 | "`cargo test --test contract_cli_preflight_relay -- --nocapture` GREEN." | Append: "; `cargo test -p engram-indexer --test <recorded stage-set test> -- --nocapture` GREEN." |
| A14 | `142.069-T`, `142.070-T`, `142.072-T` | any other `#[path]` or `allow` wording | None found (Stage grep, 2026-09-29). If the architect finds one, R12.3 overrides it (pointer line, R12.5). |

The two `harness_cmd` values for `142.072-T` both go in the Step 2
record, each with its expected count (LD3).

#### R12.4 P2-1: earlier tasks' public signatures used by later tests are frozen

* **Freeze.** When a later task's Step 2 test names an earlier task's
  `pub` item (in the same shipment, or F50's existing `preflight.rs`),
  the item's signature is frozen from the later task's Step 2 harness
  commit. The signature covers the path, name, generics and bounds,
  parameters, return type, and the variants and `pub` fields of a named
  enum or struct.
* **Record.** The later task's Step 2 record lists each frozen item, per
  test.
* **Halt.** If an earlier task's implementation would change a frozen
  item, it halts to Stage **before** committing the change:
  `R12-SIGNATURE-HALT <earlier-task> <item> <later-test>`.
* **Not a change.** Adding new `pub` items, and changing bodies, don't
  break the freeze.
* **No new rule elsewhere.** This is a Step 2 record entry plus an LD4
  halt. It adds nothing to Ship Step 4.3.

#### R12.5 P2-2: route (c) must reach the harness-architect

**Assembly edit A15 (replaces the R11.9 step 5 pointer line; S1 and S2
tasks).** "Layout and harness (plan R11.4, R11.10, R12): new file, module
and target names here are indicative, and the Step 2 harness record is
authoritative for owned files. R11.10 route (c) and R12 override any
harness instruction in this item text. The harness-architect writes
every RED test at claim (Ship Step 2), with the ordering rule, markers,
L1-L4 and (F1)-(F3). build-feature never edits a test file. There is no
`#[path]` include of another task's production file, and no
`allow`/`expect`/`warn` attribute."

**Assembly edit A16 (the R11.2 STATUS-RESET comment).**

* First line becomes: "STATUS-RESET 2026-09-29 (operator-approved; plan
  Revision 11, R11.2; route (c) per R11.10 and R12)."
* "… re-qualifies it through the harness-architect under R11.3." becomes
  "… re-qualifies it through the harness-architect under R11.10 and R12
  (forms (a-1), (c), (t))."

**Assembly edits A17-A19 (subtasks; item text only, statuses
unchanged):**

| # | Item, line | From | To |
|---|---|---|---|
| A17 | `142.064.001-ST` line 13 | "Before any production edit (RED phase for the whole PRE-3 task): author `tests/integration/read_server_generation_wiring_test.rs` + its `[[test]]` stanza, confirm RED, and record" | "RED phase for the whole PRE-3 task (plan R11.10, R12): the harness-architect writes `tests/integration/read_server_generation_wiring_test.rs` (indicative name) and its `[[test]]` stanza at the S1 claim (Ship Step 2), never during Step 4.2. build-feature never edits it. Before any production edit, this subtask confirms the recorded RED and records". The rest of the line is unchanged. |
| A18 | `142.054.002-ST` line 27 | "F50 extends `preflight_gate_test.rs` RED-first, then implements" | "The harness-architect extends `preflight_gate_test.rs` at the S2 claim (Ship Step 2; plan R11.10, R12), and build-feature never edits it. F50 then implements". The rest is unchanged. |
| A19 | `142.054.003-ST` line 28 | same as A18 | same as A18 |

Stage checked the other S1 and S2 subtasks: `142.064.002-ST`,
`142.064.003-ST` and `142.054.001-ST` contain no test-authoring text.

#### R12.6 P2-8: backlog changes land through a staging PR; re-verify after any index sync

**CG-M, reworded (replaces R11.2 CG-M):**

* **Staging branch.** Every Stage backlog or plan change that a
  shipment depends on (assembly, the PA-5 harvest, the hold lift) is
  committed by Stage on a `chore/stage-142-f-<purpose>` branch from
  `main`. For assembly, the branch is `chore/stage-142-f-assembly`.
* **Staging PR.** The change reaches `main` only through a merged staging
  PR, following repo practice (for example #399 and #401).
  `.github/workflows/detect-direct-push.yml` flags direct pushes to
  `main`.
* **Who does what.** Stage commits only. The operator or Orchestrator
  pushes, opens and merges the PR (Stage's role boundary).
* **At claim.** The PR is merged, and the worktree is clean on `main`.

**CG-S, status re-verify after any index sync (new; every claim and
assembly step 10).**

* After **any** `backlogit` index sync (`backlogit_sync_index`,
  `backlogit sync`, or a sync run by another tool), re-read every
  manifest member with `backlogit get <id>`, then re-run CG-Q.
* Every reset task, `142.054-T` to `142.059-T`, must be `queued` before
  its shipment is claimed.
* A reset task back at `active` (the stale-cache union landmine) is a
  HALT: `R12-RESET-REVERTED <ids>`.
* Stage may re-apply that reset once, under the existing R11.2 approval,
  on a staging branch (CG-M), then re-sync and re-read. A second revert
  goes to the operator.
* *Revision 13 note:* every sync here, including the re-apply's
  re-sync, uses the R13.5 cache rebuild (stop stale processes by PID,
  delete `backlogit.db*`, then sync). A revert after a clean rebuild goes
  straight to the operator.
* *Revision 14 note:* the rebuild is R14.4's Windows-safe procedure, and
  this re-apply is one of its three approved run points. CG-T (R14.2)
  joins CG-S at every S2-S5 claim.

#### R12.7 Changes to R11.9 (assembly) and R11.7 (walkthrough)

* **R11.9 step 1:** the precondition is the scoped review of
  **Revision 12**: `PASS`, or `ADVISORY` with operator confirmation.
* **R11.9 step 2:** see R12.8, P2-7 (the post-H2 exemption).
* **R11.9 step 5** adds:
  * A1-A5 (R12.2);
  * A6-A14 (R12.3);
  * A15 in place of the R11.4 pointer line, on every S1 and S2 task;
  * A16 in the STATUS-RESET comment (step 3);
  * A17-A19 (R12.5).
* **R11.9 step 9:** CG-S applies.
* **R11.9 step 10:** it now reads "Land on `main` through a staging PR
  (CG-M as reworded in R12.6)". Stage commits on
  `chore/stage-142-f-assembly`; the operator or Orchestrator pushes,
  opens and merges the PR. After the merge, on a clean `main`, sync the
  index, then re-run step 9's read-back under CG-S. Record the S1-S5 IDs
  in the session memory file as the handoff tokens.
  *Revision 13 note:* "sync the index" means the R13.5 cache rebuild.
  Steps 1, 5, 6 and 8 are also amended (R13.9).
  *Revision 14 note:* the rebuild is now R14.4 (run point 1), and steps
  1, 2a, 5, 6 and 8 are amended again (R14.9).
* **R11.7 claim gates:** every claim also needs CG-S. CG-M is read as
  reworded in R12.6.
* **R11.7 Step 2 rows:** S1 and S2 need the R12.2 O1-O4 ownership check
  and the R12.3 mechanism (b) for `142.069-T`, `142.070-T` and
  `142.072-T`. `142.071-T` is unchanged. The R12.4 frozen-item list goes
  in each later task's record.
* **Unchanged:** S3, S4, S5 membership and order, the edges (E5, E1, E2,
  E3, E4, E8 at assembly; E6 and E7 at the PA-5 harvest), and the PA-6
  hold.

#### R12.8 Remaining attempt-10 P2 and P3 dispositions

| Finding | Disposition |
|---|---|
| P2-3 route (c) feasibility after claim | **Partly fixed.** F2 and F3 now halt with the named token `R12-ROUTE-C-HALT <task> <test> <F2\|F3>`, at Step 2 before any implementation. Operator options: (i) a recorded F2 move by Stage; (ii) a scoped plan revision for that task; (iii) Ship blocks the shipment (existing block path), and Stage returns it for re-planning. The read-only feasibility pass before assembly step 6 is **deferred**: it's new analysis beyond this narrow revision, so it's listed as operator question Q1. |
| P2-4 L3 recipe defects | **Fixed** (overrides R11.10 L3). Use `let _ = x;` for one parameter and `let _ = (a, b);` for two or more. A `const fn` stub panics with `panic!("Worker: <task-id> <item>")`, not `unimplemented!`, and the marker is the panic text. `#[must_use]` is allowed on stubs (it isn't an `allow`/`expect`/`warn` attribute). |
| P2-5 LD2 visibility options | **Fixed** (overrides LD2 "Visibility"). A child module uses `pub mod`. Any private field the child reads must also be read by the earlier task's own code, so there is no "never read" warning at the earlier gate. A new `pub` accessor that isn't in the item text is an item-text change: `R12-LAYOUT-HALT <task> <item>`. |
| P2-6 LD4 file-count enforcer | **Fixed.** The harness-architect is the checker. The Step 2 record lists each task's implementation files (owned, non-LD1) and compares their count with the item text's Owned files count. A larger count is a HALT. During implementation, a file outside the record is an LD2 HALT. |
| P2-7 R11.9 step 2 vs H2 | **Fixed.** Step 2 expects exactly one difference from R11.1: 142-S in its post-H2 (abandoned) state, as H2 recorded it. That difference is exempt. Any other change is a HALT. |
| P2-9 Step 6 and subtasks | *Revision 14 note:* the timing below is replaced by R14.2: Stage closes subtasks after Ship's Step 6 finishes, gated by CG-T at the next claim and by CP-S5 after S5. *Revision 13 note:* replaced by R13.4. Subtasks move to `done` once their parent is `done` (verdict `PASS` or a verified `EXPECTED_PENDING_RED`), through a Stage staging PR after the shipment PR merges. The `PASS` precondition below is withdrawn. **Fixed (manual safe-close; no Ship change).** At each shipment's Step 6, a ship operation that expands onto, or rejects on, a subtask that isn't `done` halts under Ship's existing rules. Affected subtasks: S1 `142.064.001-ST` to `.003-ST`; S2 `142.054.001-ST` to `.003-ST`; S5 `142.058.002-ST` and `.003-ST`. After operator confirmation, Stage moves each such subtask to `done` under its backlog authority, only after its parent task's Step 4.3 verdict is `PASS`, and before the parent is closed. |
| P3 Rust: "first call" | **Fixed.** It means the first call that is awaited or polled. |
| P3 Rust: "after the placeholder, any code" | **Fixed.** It means the task's own code, earlier tasks' code, or base-tree code. |
| P3 Rust: L4 clippy command | **Fixed.** L4 also runs Ship's own Step 4.3 clippy command exactly as `_ship.agent.md` states it, plus `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic` for crate-local tests. |
| P3 Rust: placeholder panics in the test's own process | **Fixed.** A (c) marker counts only when the test's own process panics with it. |
| P3 Rust: `doc_markdown` | **Fixed.** Identifiers in stub docs go in backticks. |
| P3 Scope: F51 "must be rewritten by F52" and F52 Guardrail-4 edits in step 5 | **Deferred** to the S3 hold lift: S3 is PA-6-held, and the S3 claim gate already checks the W9 item text. |
| P3 Scope: R11.6 P3 fixes are Stage's choice | Acknowledged; no change. |
| P3 Scope: glossary rows stale after R11.10 | **Fixed** in `docs/operator-glossary.md` (2026-09-29 20:10). |
| P3 Arch: `142.060-T` stale "142-S" text | **Deferred** to the S3 hold lift (S3 only). |
| P3 Arch: "red phase CONFIRMED" in `142.054-T` to `142.057-T` | **Settled** by A16. The STATUS-RESET comment marks that evidence as historical. |
| P3 Arch: (a-1)/(t) marker wording vs harness-architect Step 5.2 | No change. Ship condition 3 governs. |
| P3 Arch: P-001 block has no operator acknowledgment | Listed as operator question Q2. |
| P3 Arch: stale R11.8 II bullet and scope-limit lines | Settled by R11.9 step 8 and R11.10's table. No new edit. |
| P3 Const: route (c) residual | **Fixed.** Recorded as a standing, operator-accepted limitation (18:47 decision). No Constitution II deviation. |
| P3 Const: "HALT, not re-label" paraphrase | Acknowledged. It's attempt-9's paraphrase, not the escalation review's text. |
| P3 Const: "no 142-F task is `active`" | **Fixed.** Read as "no 142-F task artifact is `active`". The five subtasks stay `active` (R11.2). |
| P3 Const: "not a P-021 finding" scope | **Fixed.** It applies to claim-time and Step 2 halts only. |
| P3 Const: `142.059-T` docs paths on the S5 base | **Deferred** to the S5 claim (drift ledger, R9.9). |
| P3 Const: T0 re-apply content | **Fixed.** A Step 2 re-apply from T0 carries tests and L3 stubs only, never implementation. |
| P3 Const: rejected subtask-member alternative | Already named in the R11.2 "Deviation" bullet. No change. |
| P3 Learnings: plan-detail churn | Acknowledged. R12 adds no new plumbing beyond the P1s. |
| P3 Learnings: missing compound path | No plan-body citation outside the attempt-10 record. No change. |

#### R12.9 Operator questions (not blocking the scoped review)

* **Q1 (P2-3).** Do you accept that PRE-4 (`142.066-T`) may stop with
  `R12-ROUTE-C-HALT` at the S1 claim (Step 2, before any code)? Or do you
  authorize a read-only route (c) feasibility pass before assembly
  step 6?
* **Q2 (P3).** Do you acknowledge that while the PA-6 hold stands after
  S2 ships, `142-F` staying `active` blocks every other release unit
  under P-001 (R11.2)?

**Scope limits.** Revision 12 changed only this plan (this subsection,
the frontmatter, and "Revision 12 note" lines in R11.2, R11.4, R11.7,
R11.9 and R11.10), `docs/operator-glossary.md`, and
`docs/memory/2026-09-29-stage-142-f-rev12-memory.md`. It changed no
backlog item, edge, shipment, stash entry, source, test, `Cargo.toml` or
config file, and made no git mutation. **Next step:** one fresh-session
scoped review of Revision 12, not run by Stage.

### Revision 13: attempt-11 P1 fixes

Revision 13 is narrow. It closes attempt-11 P1-1 to P1-4 and records the
operator's 21:00 decisions, including the Q1 read-only feasibility pass
(R13.6) and the Q2 acknowledgment. It doesn't reopen route (c), the
resets, P-021, the edges, S1-S5 membership, or the PA-6 hold. Every
item-text edit below is **recorded here and applied at assembly**
(R11.9 step 5). None is applied now.

#### R13.1 Decisions recorded (operator, 2026-09-29 21:00 -07:00)

**Verbatim:** "Revision 13 (P1 fixes only), approve the cache rebuild,
then review. Q1: I think Stage should check first with read-only pass.
Q2: Yes. Also, I don't understand why a subtask cannot be marked as done
as a constituent of a parent task. Why would that not be allowed?"

* **D1.** Revision 13 fixes the attempt-11 P1s only. The attempt-11 P2s
  and P3s are deferred (R13.8).
* **D2.** The backlogit cache rebuild is approved (a destructive action;
  R13.5).
* **D3.** Stage runs the Q1 read-only route (c) feasibility pass now
  (R13.6).
* **D4.** Q2 is acknowledged (R13.7).
* **D5.** The operator's view: a subtask is a constituent of its parent
  and is marked `done` with it (R13.4).
* **Then:** one fresh-session scoped review, not run by Stage. The 18:14,
  18:47 and 20:04 decisions stay in force.

#### R13.2 P1-1: `142.072-T` gets a public relay stage list

**Rule.** The relay's closed stage set becomes an item-text constant,
`pub const RELAY_STAGES`, in `src/cli/commands/preflight.rs`.

* `normalize_verdict` uses it as its only stage list.
* The crate-local stage-set test proves two-way set equality between
  `RELAY_STAGES` and `stage_name` over every `Failure` variant.
* The `Failure` variant list is guarded by an exhaustive `match` with no
  wildcard arm, so a new variant fails to compile.
* The constant is frozen under R12.4 from `142.072-T`'s Step 2.

This replaces A11's one-direction `normalize_verdict` check, which
couldn't prove equality (attempt-11 P1-1). Stage checked the evidence:
`142.072-T` line 23 names only "four functions", and line 26 keeps its
closed set internal.

**Assembly edits (the "from" text is quoted exactly):**

| # | Item, line | From | To |
|---|---|---|---|
| A11 (rev. R13) | `142.072-T` line 30 | same "From" as R12.3 A11 | ". The stage-set assertion is in a second test file owned by this task, crate-local under `crates/engram-indexer/tests/` and named in the Step 2 harness record (plan R12.3, R13.2). Its first call is this task's own `normalize_verdict`. It asserts two-way set equality between `RELAY_STAGES` and `stage_name(f)` over every `Failure` variant. The variant list is guarded by a `match` over `Failure` with one arm per variant and no wildcard. It also asserts that `normalize_verdict` returns each non-`Build` stage's `Failed` line unchanged with exit 1. There's no `#[path]` include of a production file, and no `allow`/`expect`/`warn` attribute." This replaces R12.3's A11 "To" text. |
| A21 | `142.072-T` line 23 | "four functions:" | "four functions and one constant:" |
| A22 | `142.072-T`, a new line after line 26 | (none) | "- `pub const RELAY_STAGES: [&str; 7]`: the relay's closed stage set, one entry per stage `stage_name` can return (the seven stages of `142.069-T` lines 23 and 35). `normalize_verdict` accepts a `Failed` line only when its stage is in `RELAY_STAGES`, and it has no other stage list. The constant is frozen under plan R12.4 (R13.2)." |

**Scope check (LD4).** The acceptance statement ("the closed stage set
equals `stage_name` over all `Failure` variants", line 30) is unchanged.
The constant is the item that makes it testable. It is part of the final
API, and it wasn't added only for a test: `normalize_verdict` depends on
it. If the architect finds that `stage_name` can return a stage count
other than seven, the array length follows `stage_name`, and the
architect records it at Step 2. That isn't a halt.

#### R13.3 P1-2: the crate-local `engram-indexer` tests are gated

**Why a gap existed.** Stage re-checked the evidence:

* the root `Cargo.toml` lines 1-3 declare a workspace with no
  `default-members`;
* `cargo dev-test` is `test --all-targets` at the root;
* Ship Step 4.3 and CI run only at the root.

So nothing ran the R12.3 tests after their own `harness_cmd`.

**Rule.**

* **Step 2 record (added to L4).** `cargo check -p engram-indexer
  --all-targets` must exit 0.
* **Every S2 code task's gate.** `cargo test -p engram-indexer
  --all-targets --no-fail-fast` must pass, or each failing test must be
  a later S2 task's recorded pending RED, failing with its recorded
  marker (Ship Step 4.3 `EXPECTED_PENDING_RED`). `cargo clippy -p
  engram-indexer --all-targets -- -D warnings -D clippy::pedantic` must
  be clean. The S2 code tasks are `142.054-T`, `142.069-T` to
  `142.073-T`.
* **Final S2 criteria (`142.073-T`).** Both commands must be fully GREEN
  and clean, with no pending RED.

These live in item text and the Step 2 record, and need no Ship change.

**Assembly edit A20** (insert before each task's
`<!-- END:acceptance-criteria -->` line):

| Item, insert before line | Line inserted |
|---|---|
| `142.054-T` 47, `142.069-T` 39, `142.070-T` 41, `142.071-T` 38 | "- (plan R13.3) `cargo test -p engram-indexer --all-targets --no-fail-fast` passes, or each failing test is a later S2 task's recorded pending RED, failing with its recorded marker." These four already carry the `engram-indexer` clippy line. |
| `142.072-T` 43 | The same line, plus: "- (plan R13.3) `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic` is clean." |
| `142.073-T` 39 | "- (plan R13.3, final S2 criteria) `cargo test -p engram-indexer --all-targets --no-fail-fast` is fully GREEN, and `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic` is clean." |

**A15 amended.** The pointer line reads "(plan R11.4, R11.10, R12, R13)"
instead of "(plan R11.4, R11.10, R12)", so the Step 2 `cargo check` and
R13.2 reach the architect.

**Constitution Check deviation (added at R11.9 step 8).**

* **Principle.** Constitution line 22: tests "MUST … pass via `cargo
  dev-test`".
* **Deviation.** The `142.069-T`, `142.070-T` and `142.072-T` stage-set
  tests live in `crates/engram-indexer/tests/`, which `cargo dev-test`
  and CI don't run.
* **Why.** Option (a), a root dev-dependency on `engram-indexer`, was
  rejected as a dev-dependency cycle (root → `engram-indexer` → root;
  R12.3).
* **Compensation.** The A20 commands at every S2 gate and in the S2 final
  criteria.
* **Residual.** After S2 merges, CI still doesn't run these tests.
  Adding `-p engram-indexer` to CI or `cargo dev-test` is a config
  change outside 142-F. It's recorded here as a follow-up for Stage to
  stash after assembly, and isn't done in this feature.

#### R13.4 P1-3: subtasks are marked `done` with their parent

*Revision 14 note:* the "When", "What Stage does" and "P-001" bullets,
and the "Child expansion and safe-close" paragraph, are replaced by
R14.2 (attempt-12 P1-1, fix (b)). Stage closes subtasks after Ship's
Step 6 finishes, CG-T gates the next claim, and CP-S5 gates `142-F`'s
closure. The Rule below stands. Q4 was answered "no" (R14.1).

**The operator's question, answered plainly.** Nothing in backlogit
forbids marking a subtask `done` as a constituent of its parent, and the
plan now does exactly that. The earlier rule blocked it for three
reasons:

* R12.8 tied subtask closure to the parent's Step 4.3 verdict being
  `PASS`. Non-final tasks, such as `142.064-T` in S1 and `142.054-T` in
  S2, normally end with `EXPECTED_PENDING_RED`, so they never get
  `PASS`.
* Subtasks aren't shipment members (R11.2), so no Ship gate owned them.
* The five subtasks left `active` by the abandoned 142-S had no owner at
  all.

The fix ties closure to the parent being `done`, not to `PASS`.

**Rule.** When a parent task moves to `done` at Ship Step 4.5 (verdict
`PASS`, or a verified `EXPECTED_PENDING_RED`), each of its subtasks moves
to `done`. Each gets a history note citing the parent's gate record and
commit SHA.

**Who makes the move: the Ship Step 4.5 grant isn't clear, so the
fallback applies.**

* `_ship.agent.md` line 38 grants "move tasks to active/done".
* Step 4.5 item 4 moves "the task" (~595).
* The whole file has no occurrence of "subtask" (Stage grep).
* Step 3 selects members by their `T` suffix.

A `subtask` is a separate `artifact_type`, and Ship's contract never
mentions it. So moving subtasks isn't clearly inside Ship's grant, and
Revision 13 doesn't stretch it. The **attempt-11 fallback** applies, with
the precondition above:

* **When.** After the shipment PR merges (Ship Step 6 Merge Confirmation
  Gate passed) and before Ship's Step 6 closure.
* **What Stage does.** Stage runs `backlogit move <subtask-id> --status
  done` for each subtask whose parent is `done` on `main`, and appends
  the note above. It commits on `chore/stage-142-f-subtask-close-<shipment>`
  (CG-M staging PR). The operator or Orchestrator merges it, then Ship
  resumes Step 6. No sync runs after the CLI moves (R13.5).
* **Affected subtasks:**
  * S1: `142.064.001-ST` to `.003-ST`;
  * S2: `142.054.001-ST` to `.003-ST`;
  * S5: `142.058.002-ST` and `.003-ST`.
* **P-001.** At assembly step 6, Stage records the subtask-close PR as a
  required release-closure item in each of the S1, S2 and S5 shipment
  records. Ship's existing Release Closure Completion Gate
  (`_ship.agent.md` ~761-766) then keeps the shipment active until the PR
  merges. This needs no Ship change.

**Child expansion and safe-close.** Ship's default Step 6 safe-close
archives only the manifest's explicit IDs and doesn't call `shipment
ship` (`_ship.agent.md` ~840-859).

* So for S1 and S2, subtasks don't block closure. The PR above gives them
  an owner.
* S5 holds `142-F`, so its close may take the P-015 fully-covered-root
  cascade, which calls `shipment ship`. `shipment ship` expands every
  manifest parent's children
  (`docs/compound/workflow-issues/backlogit-ship-blocked-child-expansion-2026-04-26.md`,
  "Root Cause").
* By then every 142-F subtask is `done`. The S1 and S2 subtasks close
  after their own shipments, and S5's close before its Step 6. So the
  expansion finds no non-`done` child.

This replaces the R12.8 P2-9 rule.

**Operator option (Q4).** Having Ship move subtasks itself at Step 4.5
(literally "together") needs a one-line clarification of Ship's Role
Boundary ("tasks and their subtasks"). That's a harness-rule change,
which Revision 13 doesn't make.

#### R13.5 P1-4: rebuild the cache before any sync

*Revision 14 note:* steps 1-3 and the action record are replaced by
R14.4, which is Windows-safe and limits the approval to three run points.
Step 4 and the rules stand.

**Procedure** (replaces "sync" in R11.9 step 10 and in the CG-S re-apply;
from `docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md`,
"Fix / Workaround", steps 1-3):

1. Stop stale `backlogit` MCP processes **by PID** (`Stop-Process -Id
   <pid>`), never by name.
2. Delete `.backlogit/backlogit.db`, `.backlogit/backlogit.db-wal` and
   `.backlogit/backlogit.db-shm`.
3. Run `backlogit sync`, which rebuilds the cache from the Markdown
   files.
4. Re-read statuses (CG-S). Every reset task, `142.054-T` to
   `142.059-T`, must be `queued`.

**Rules.**

* No reflexive `sync` after CLI mutations (`move`, `update`, `shipment`
  commands).
* Never stage `backlogit.db*`. The three files are gitignored
  (`.gitignore` lines 76-78; Stage checked with `git check-ignore`).
* The CG-S re-apply now uses this procedure too. A revert after a clean
  rebuild isn't a stale-cache effect, so it goes straight to the
  operator.

**Action record.**

| Field | Value |
|---|---|
| ProposedAction | The rebuild above, at R11.9 step 10 and at any CG-S re-apply |
| ActionRisk | destructive (it deletes the local cache files) |
| Approval | operator, 2026-09-29 21:00 -07:00 ("approve the cache rebuild") |
| ActionResult | `approved`, not yet `applied` (it runs at assembly) |
| Rollback | None needed. The Markdown files under `.backlogit/` are the source of truth, and the cache rebuilds from them. If the rebuild fails, re-run step 3. The cache files are never committed. |

#### R13.6 Q1: route (c) feasibility pass (read-only)

Stage read the item text, the current source, and R11.10. There were no
builds.

* **Verdict key:**
  * **feasible:** the test can call its own placeholder first, as
    written.
  * **F1:** a different own item comes first.
  * **F3:** no own item can come first (a halt at Step 2).
* **Earlier pending code for S1:** PRE-1 and later tasks, including any
  daemon or CLI child process built from them.

| Test (task) | Verdict | First own call / reason | Evidence |
|---|---|---|---|
| PRE-2 `142.062-T` sc. 1-3 | F1 | `current_published_revision(&store)`, where `store` is a base-tree `GenerationStore::new(<temp>)`. `open_generation_store` and `publish_sealed_candidate` need a `ReadServerLayout`, which is `#[non_exhaustive]` and comes only from PRE-1's pending `read_server_layout`. | 062 lines 24-26, 35-37; 061 lines 22-23; `src/services/generations/mod.rs` 38; `store.rs` 78 |
| PRE-3 `142.064-T` sc. 1-3 | F1 | `read_server_gate()` (own; `142.064.001-ST`) on a base-tree `AppState::with_mode(..)`, before any publish or daemon spawn. *Revision 14 note:* needs `read_server_gate` to be `pub` (A25, A26; R14.7). | 064 lines 29, 37-39; `142.064.001-ST` line 11; `src/server/state.rs` 1186, 1745; `src/lib.rs` 43 |
| PRE-4b `142.065-T` sc. 1-3 | feasible | own `get_workspace_statistics_with_context(state, None, None)` on a base-tree managed `AppState` (scenario 2's managed baseline), then publish and activate | 065 lines 24, 35-37 |
| **PRE-4 `142.066-T` sc. 1-3** | **F3**; *Revision 14 note:* **feasible under Q3 (ii)** (R14.3) | Its only new item is `pub const GENERATION_PINNED_READS`, and a `const` can't be a panicking placeholder. Its behavior change sits inside the base-tree `process_request`, which runs today. Every scenario needs a spawned daemon or CLI, which runs pending PRE-1 to PRE-4b code. F1 fails (no own callable item). F2 fails (the scenarios are PRE-4's own acceptance criteria). *Revision 14:* the constant lands at Step 2 as `&[]`, and each test first asserts its final value with the marker `Worker: 142.066-T GENERATION_PINNED_READS` (A23, A24). | 066 lines 24-27, 35-38; `src/daemon/request_entry.rs` 26-252 |
| PRE-5 `142.067-T` sc. 1-2 | F1 | `probe_cli_read` with a nonexistent exe path, which returns `Spawn` (scenario 3's assertion); then publish and spawn | 067 lines 24-27, 37-39 |
| PRE-5 `142.067-T` line-42 unit test (*Revision 14*) | F1 | `probe_cli_read` with a nonexistent exe path (`Spawn`) first, then `PREFLIGHT_READ.mcp_tool` in `GENERATION_PINNED_READS`. Compiles only under Q3 (ii) (R14.6). | 067 lines 39, 42; R14.6 |
| PRE-5 `142.067-T` sc. 3 | feasible | own probes against rustc-compiled fixtures | 067 line 39 |
| PRE-6 `142.068-T` sc. 1, 3 | F1 | `probe_mcp_read` with a test-local `ProbeRead` whose `mcp_arguments_json` is invalid: `Malformed`, and nothing spawns (scenario 2's assertion). PRE-5 helpers come only after. | 068 lines 23-24, 36-38 |
| PRE-6 `142.068-T` sc. 2 | feasible | as above | 068 line 37 |
| NEW-1 `142.069-T` sc. 1-3 | feasible | own `run(..)` with a test mock `Verifier` (test code), and own `stage_name` | 069 lines 23-25, 35-36; `Verifier` is `pub` (`tests/integration/preflight_gate_test.rs` line 8) |
| NEW-2 `142.070-T` | feasible | own `parse_invocation` / `deadline_from` / `main_with` with literal args | 070 lines 23-25 |
| NEW-4 `142.072-T`, contract test | feasible | own `normalize_verdict` (pure) or `resolve_supervisor`; the fixtures are rustc-compiled | 072 lines 24-27, 30 |
| NEW-4 `142.072-T`, stage-set test | feasible | own `normalize_verdict` first (R13.2) | R13.2 |

**Result.** One task is F3: PRE-4 (`142.066-T`), as attempt 10
predicted. Every F1 above uses an item already in the task's item text,
so it adds no production item. The item text isn't changed for the F3
(D3: record only). It becomes operator question Q3.

* **Options for Q3:**
  * **(i)** Keep the plan. S1 halts with `R12-ROUTE-C-HALT 142.066-T
    <tests> F3` at Step 2, before any code.
  * **(ii)** An (a-1) variant for PRE-4 only. At Step 2,
    `GENERATION_PINNED_READS` lands as `&[]`, with its final type. Each
    PRE-4 test first asserts `GENERATION_PINNED_READS ==
    ["get_workspace_statistics"]`, so it fails at the task's own item
    before running any pending code. PRE-4's implementation sets the
    value. This departs from R11.10's "final" placeholder: the value is
    a placeholder, the signature isn't.
  * **(iii)** Move PRE-4 into its own shipment, after S1. That changes
    S1-S5 membership.
* **Stage recommends (ii).** It's the narrowest change, and it adds no
  test-only item.

#### R13.7 Q2 acknowledged

The operator answered "Yes" (21:00). While the PA-6 hold stands after S2
ships, `142-F` stays `active`. Under P-001, that blocks the claim of
every other top-level release unit (R11.2). This is recorded as an
accepted, standing condition, and R12.9 Q2 is closed.

#### R13.8 Attempt-11 P2 and P3 dispositions

* **P2-1 to P2-7: deferred** (D1).
  * P2-1 (`142.063-T`'s owned parity file) fires only at the PA-6-held
    S4 claim, and is fixed at the S4 hold lift.
  * P2-2 to P2-6 go to the harness-architect at Step 2. Its halt tokens
    (`R12-TEST-PATH-HALT`, `R12-SIGNATURE-HALT`) still cover them.
  * P2-7 (the registration-file baseline) is deferred. O2 and O3 are
    unchanged.
* **P3s: deferred**, except one. "Record the CG-S re-apply and its
  cause" is resolved by R13.5: the cause is the stale cache, and the
  re-apply now uses the rebuild.

#### R13.9 Assembly and walkthrough changes, and operator questions

*Revision 14 note:* the step 1 review is now of Revision 14; step 6's
release-closure item is withdrawn and S5 drops `142-F` (R14.2); step 10
uses R14.4. Q3 is answered (ii) and Q4 "no" (R14.1). R14.9 lists the
full changes.

* **R11.9 step 1:** the gating review is of **Revision 13**.
* **R11.9 step 5** adds A11 (as revised), A20, A21 and A22, and the A15
  amendment (R13.3).
* **R11.9 step 6:** it records the subtask-close PR as a
  release-closure item on the S1, S2 and S5 shipments (R13.4).
* **R11.9 step 8:** it adds the R13.3 Constitution Check deviation.
* **R11.9 step 10:** it uses the R13.5 rebuild in place of a plain sync,
  then re-reads statuses under CG-S.
* **R11.7:** the S1, S2 and S5 closure uses the R13.4 subtask close.
  Claim gates are unchanged.
* **Operator questions:**
  * **Q3 (R13.6).** How should PRE-4 (`142.066-T`) handle its F3: (i),
    (ii) (recommended) or (iii)?
  * **Q4 (R13.4).** Do you want Ship itself to move subtasks at Step
    4.5? That needs a one-line Ship Role Boundary clarification, which is
    a harness-rule change. Without it, the Stage staging-PR fallback
    stands.

**Scope limits.** Revision 13 changed only this plan (this subsection,
the frontmatter, and "Revision 13 note" lines in R11.7, R12.6, R12.7 and
R12.8), `docs/operator-glossary.md`, and
`docs/memory/2026-09-29-stage-142-f-rev13-memory.md`. The Q1 pass was
read-only. It changed no backlog item, edge, shipment, stash entry,
source, test, `Cargo.toml` or config file, ran no build and no cache
rebuild, and made no git mutation. **Next step:** one fresh-session
scoped review of Revision 13, not run by Stage.

### Revision 14: attempt-12 P1 fix and selected P2s

Revision 14 is narrow. It closes attempt-12 P1-1 with fix (b), applies
the operator's Q3 answer (ii), and fixes attempt-12 P2-3, P2-7, P2-9 and
P2-10. P2-4 and P2-5 are closed only because the P1 fix depends on them
(R14.2). It doesn't reopen route (c), the resets, P-021, the edges, the
PA-6 hold, or S1-S4 membership. One membership change is made to S5:
`142-F` leaves its manifest (R14.2). Every item-text, manifest and
Constitution Check edit below is **recorded here and applied at
assembly** (R11.9). None is applied now.

#### R14.1 Decision recorded (operator, 2026-09-30 17:35 -07:00)

**Verbatim:** "Q4 no (fix b), Q3 (ii), Revision 14: the P1 plus P2-3,
P2-7, P2-9 and P2-10, then review."

* **E1.** Q4 is answered "no". Ship's contract is unchanged, and the
  subtask fallback stays with Stage, re-sequenced as attempt-12 fix (b)
  (R14.2).
* **E2.** Q3 is answered (ii): the empty-constant placeholder for PRE-4
  (`142.066-T`), with attempt 12's four conditions (R14.3).
* **E3.** Revision 14 fixes P1-1, P2-3, P2-7, P2-9 and P2-10. Every other
  attempt-12 P2 and P3 is deferred (R14.8).
* **Then:** one fresh-session scoped review of Revision 14, not run by
  this session. The 18:14, 18:47, 20:04 and 21:00 decisions of
  2026-09-29 stay in force, including the D2 rebuild approval, whose
  scope R14.4 narrows.

#### R14.2 P1-1 (fix (b)): Stage closes subtasks after Ship's Step 6, gated at the next claim

**Replaces** R13.4's "When", "What Stage does" and "P-001" bullets, and
its "Child expansion and safe-close" paragraph. R13.4's Rule (a subtask
moves to `done` once its parent is `done`, by `PASS` or a verified
`EXPECTED_PENDING_RED`) and its history-note content stay.

**Why R13.4's timing can't work** (attempt 12, verified by Stage against
`_ship.agent.md`): the Merge Confirmation Gate ends "Proceed to Step 6.0
only after both checks pass" (line 759). Step 6.0 then runs all closure
on its own `post-merge/<slug>` branch (784-795). The Release Closure
Completion Gate (761-767) checks Ship's own closure work, and nothing in
it reads an outside PR recorded on the shipment. So there's no pause
point. Revision 14 drops the "required release-closure item" wording, and
the claim that Ship's closure gate holds the shipment open for Stage's
PR.

**Timing (per shipment Sn with subtasks: S1, S2, S5).** Stage starts the
subtask close only when all of these hold:

1. Ship has finished Step 6 for Sn: the shipment is `shipped`, Ship's
   post-merge closure PR is merged, the P-020 compaction is done, and the
   Release Closure Completion Gate has passed (no open
   `RELEASE_CLOSURE_INCOMPLETE`).
2. **One worktree (P-016).** Ship's `post-merge/<slug>` branch is merged
   and deleted, locally and on the remote. `git worktree list` shows one
   worktree, it's on `main`, it's pulled, and `git status --short` is
   empty.

Then:

3. **Read.** For each subtask whose parent is a `done` 142-F task, Stage
   reads the subtask's status and its parent's status with `backlogit
   get`, and checks each against the Markdown frontmatter on `main`. A
   mismatch is a HALT to the operator: `R14-CACHE-DIVERGED <id>`.
4. **Move.** On `chore/stage-142-f-subtask-close-<shipment>` (from
   `main`), Stage runs `backlogit move <subtask-id> --status done` for
   each such subtask. Each move gets a history note citing the parent's
   Step 4.3 gate record, the parent's implementation commit SHA and the
   shipment's merge SHA. `--force-gates` and `--gate-base` are never used
   (`backlogit move --help`: both are operator-only). Any refusal (exit
   6, 7 or 8) or rejected transition is a HALT:
   `R14-SUBTASK-MOVE-HALT <subtask> <from>-><to> <exit>`. Expect each
   file to move from `.backlogit/queue/` to `.backlogit/archive/`
   (`.backlogit/registry.yaml` lines 1-8 route `done` to `archive`). No
   sync runs after the moves.
5. **Stage PR (CG-M).** Stage commits only. The operator or Orchestrator
   pushes, opens and merges the PR.
6. **After the merge.** On a pulled, clean `main`, Stage runs the R14.4
   cache rebuild, then re-reads under CG-S and CG-T (below). This is the
   attempt-12 P2-4 re-read, included because the moves change Markdown on
   a branch while the local cache saw the branch state.

The run is a Stage session: the one that opened the PR if it's still
live, or a fresh Stage session started for it.

**Affected subtasks** (unchanged from R13.4): S1 `142.064.001-ST` to
`.003-ST` (now `queued`); S2 `142.054.001-ST` to `.003-ST` (now
`active`); S5 `142.058.002-ST` and `.003-ST` (now `active`). S3 and S4
have none today. A PA-5 subtask harvested into S4 is covered by the same
rule.

**CG-T, subtask closure (new claim gate for S2, S3, S4 and S5).** Before
a 142-F shipment is claimed:

* every subtask (queue and archive) whose parent is a 142-F task with
  status `done` is itself `done`; and
* no `chore/stage-142-f-subtask-close-*` PR is open, and the one for the
  previous shipment (if it had subtasks) is merged, followed by the step
  6 re-read.

The check is cumulative, so S3 and S4 re-confirm the S1 and S2 closes.
It's a Stage-recorded claim-gate row, read by whoever claims (the same
form as CG-Q, CG-M and CG-S), and it needs no Ship change. A failure is a
HALT before the claim: `R14-SUBTASK-OPEN <shipment> <ids>`. S1 has no
previous 142-F shipment; its CG-T check is vacuous but recorded.

**CP-S5, operator checkpoint after S5 (no next shipment to gate).** S5 is
the last 142-F shipment, so CG-T can't hold its subtasks. Instead:

* **`142-F` leaves the S5 manifest.** R11.9 step 6's S5 entry becomes
  `items: [142.058-T]`, then `142.059-T`. That matches S1-S4, which are
  already task-only. Ship's S5 safe-close then archives only
  `142.058-T`, `142.059-T` and the shipment record (`_ship.agent.md`
  846-849), and `142-F` stays `active`.
* **142-F's closure waits for CP-S5.** After S5's Step 6, Stage runs the
  subtask close above for `142.058.002-ST` and `.003-ST`. On the same
  branch (`chore/stage-142-f-subtask-close-S5`), Stage then reads every
  child and grandchild of `142-F` in `.backlogit/queue/` and
  `.backlogit/archive/`. If any isn't `done`, it HALTs:
  `R14-142F-OPEN-CHILDREN <ids>`. Only then does it stop and ask the
  operator to confirm. That confirmation is CP-S5, and Stage records it
  verbatim. After CP-S5, Stage runs `backlogit move 142-F --status done`
  on the same branch, with a history note citing S1-S5 and their merge
  SHAs, and commits. The operator or Orchestrator merges the PR.
* **P-001 holds the line.** Until that PR merges and the step 6 re-read
  shows `142-F` `done`, `142-F` stays `active`, so P-001 blocks every
  other release-unit claim. While CP-S5 is pending, that's
  `R14-142F-CLOSE-PENDING`.
* So `142-F` never closes while one of its subtasks is open.

**P-015 (closes attempt-12 P2-5).** With `142-F` out of S5, no manifest
holds a feature, so every S1-S5 Step 6 uses safe-close. R13.4's cascade
reasoning is withdrawn, and no `shipment ship` call is expected.

**Assembly step 2a (new; read-only, before any write).** Stage confirms
that the moves above are possible, and records the evidence:

1. `.autoharness/config.yaml`'s `lifecycle_hooks.pre_task_completion`
   block is still commented out (lines 148-167 on 2026-09-30). If it's
   enabled with a command that builds or tests (for example `cargo
   dev-test`), a Stage `backlogit move ... --status done` would run a
   build, which is outside Stage's role: HALT
   `R14-SUBTASK-MOVE-UNSUPPORTED completion-gate`.
2. `backlogit move --help` still covers "task/subtask completions"
   (verified 2026-09-30).
3. The transitions `queued`→`done` (the `142.064.*` subtasks),
   `active`→`done` (the `142.054.*` and `142.058.00[23]` subtasks), the
   case where the parent is already in `.backlogit/archive/`, and
   `142-F`'s own `active`→`done` are accepted. The evidence is a
   precedent in `.backlogit/archive/` for each case, or the backlogit
   documentation. If there's no evidence for a case, HALT:
   `R14-SUBTASK-MOVE-UNSUPPORTED <case>`.

**Operator options on any `R14-SUBTASK-MOVE-*` halt:** (i) approve a
one-time probe on a scratch copy of `.backlogit/` (`backlogit --cwd
<temp copy> move ...`), with the copy removed afterwards; (ii) revisit
Q4 (Ship moves subtasks at Step 4.5, as a separate harness chore merged
before the S1 claim); (iii) record the subtasks as `archived` with a
history note instead of `done`.

**P-010 and P-001 (unchanged from R13.4).** Subtasks carry no
`shipment_id` and aren't shipment members, so moving them isn't a claim.
Moving `142-F` to `done` is a backlog-item update inside Stage's grant.
Stage doesn't close or ship any shipment.

#### R14.3 Q3 (ii): PRE-4 (`142.066-T`) gets an empty-constant placeholder

**Rule.**

* **Step 2.** The harness-architect lands `pub const
  GENERATION_PINNED_READS: &[&str] = &[];` in `src/daemon/request_entry.rs`
  (public path: `src/lib.rs` line 38 `pub mod daemon;`,
  `src/daemon/mod.rs` line 14 `pub mod request_entry;`). The type is
  final, and the value is a placeholder. `process_request` doesn't read
  it until PRE-4's implementation.
* **Each PRE-4 test's first statement** is
  `assert_eq!(GENERATION_PINNED_READS, ["get_workspace_statistics"],
  "Worker: 142.066-T GENERATION_PINNED_READS");`. Before PRE-4's
  implementation, each test fails there, with that message as its
  marker (Ship Step 4.3 condition 3, `_ship.agent.md` line 455), and runs
  no pending code. PRE-4's implementation sets the value.
* **Freeze (condition 2).** Under R12.4, only the constant's path, name
  and type (`&[&str]`) are frozen, from the first later test that names
  it (`142.067-T`'s line-42 test, R14.6). The value isn't frozen.
* **Owned-file baseline (condition 3).** `src/daemon/request_entry.rs`
  is `142.066-T`'s owned file. Its Ship Step 4.3 condition-4 baseline
  (G1: one point per shipment, after the last Step 2 harness commit) is
  the blob that includes the `&[]` line. The Step 2 record names this
  edit.
* **No PRE-3 test reads the constant (condition 4).** PRE-3 tests share
  the target file. While the value is `&[]`, a PRE-3 test that read it
  would change meaning. PRE-3's invariant-7 test (`142.064-T` line 42)
  names `get_workspace_statistics` literally. If an S1 test other than
  PRE-4's and `142.067-T`'s line-42 test needs the constant, that's
  `R12-ROUTE-C-HALT <task> <test> F3`.
* **Lint.** `&[&str] == [&str; 1]` is a std `PartialEq`, an empty-slice
  `const` fires no pedantic lint, and `assertions_on_constants` covers
  only `assert!` (attempt 12, confirmed sound).

**Departure from R11.10, recorded.** R11.10 ("The placeholder item",
first bullet) requires
the placeholder item to be "part of the task's final public API ... with
its final signature". Here the signature is final but the value isn't.
**Constitution Check addition (R11.9 step 8):**

* **Principle.** Constitution II (test-first, red then green) and R11.10's
  placeholder rule.
* **Deviation.** `142.066-T`'s placeholder is a `pub const` with its
  final type and a placeholder value (`&[]`), set by its implementation.
* **Why.** A `const` can't panic, and PRE-4 has no other own item that
  can be called first (R13.6, F3).
* **Rejected alternatives.** (i) a certain halt at the S1 claim; (iii)
  a separate S1b shipment of 066, 067 and 068, which changes S1/S2
  membership and adds a P-001 cycle (attempt 12, Q3).

**Assembly edits:**

| # | Item, line | From | To |
|---|---|---|---|
| A23 | `142.066-T` line 25 | "`pub const GENERATION_PINNED_READS: &[&str] = &["get_workspace_statistics"];`" | "`pub const GENERATION_PINNED_READS: &[&str]`, final value `&["get_workspace_statistics"]`. The harness-architect lands it at Step 2 as `&[]` with this final type, and this task's implementation sets the value (plan R14.3, Q3 (ii)). Only its type is frozen (plan R12.4)." The rest of the line is unchanged. |
| A24 | `142.066-T`, insert before line 43 (`<!-- END:acceptance-criteria -->`) | (none) | "- (plan R14.3) Each PRE-4 test's first statement is `assert_eq!(GENERATION_PINNED_READS, ["get_workspace_statistics"], "Worker: 142.066-T GENERATION_PINNED_READS");`. Before this task's implementation it fails with that marker and runs no pending code." |

The A15 pointer line on `142.066-T` (R12.5/R13.3) reads "(plan R11.4,
R11.10, R12, R13, R14)".

#### R14.4 P2-3: a Windows-safe cache rebuild, with a narrower approval

**Replaces** R13.5's procedure steps 1-3. Steps 4 (CG-S re-read) and the
R13.5 rules stay. It follows
`docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md`,
"Fix / Workaround", steps 2a-2c, made safe for Windows.

**Who runs it:** Stage, under D2, in the session that reaches one of the
approved run points below. If Stage halts, the operator decides.

**Approved run points (the whole approval; nothing else is covered):**

1. R11.9 step 10, once, after the assembly staging PR merges;
2. the single CG-S re-apply's re-sync (R12.6);
3. the R14.2 step 6 re-read after each subtask-close PR merges (S1, S2
   and S5).

Any other rebuild needs a new operator approval.

**Procedure:**

0. **Preconditions.** One worktree, on a pulled `main`; `git status
   --short` is empty. Record the run in the session memory file before
   step 1.
1. **Find holders.** Run `Get-CimInstance Win32_Process -Filter "Name
   LIKE 'backlogit%'"` and record each `ProcessId`, `ParentProcessId`
   and `CommandLine`. A process is **in this workspace** when its command
   line contains the workspace root (`C:\Source\GitHub\engram`) or
   `--cwd` naming it, or when it's this session's own backlogit MCP
   server (its parent is this session's agent host, the parent of the
   PowerShell tool process). Where Sysinternals `handle.exe` is
   installed, its list of holders of `.backlogit\backlogit.db` is added.
2. **Stop by PID.** `Stop-Process -Id <pid>` for each in-workspace PID
   only, never by name. Never stop a `backlogit` process that can't be
   tied to this workspace. Once its own MCP server is stopped, Stage uses
   only the `backlogit` CLI until the sync finishes.
3. **Delete.** `Remove-Item -LiteralPath <file> -ErrorAction Stop` for
   each of `.backlogit\backlogit.db`, `-wal` and `-shm` that exists. A
   sharing violation is a HALT: `R14-CACHE-LOCKED <file> <remaining
   backlogit PIDs>`, listing every remaining PID and command line for
   the operator.
4. **Check.** `Test-Path` on all three files must be `False`. Otherwise,
   HALT: `R14-CACHE-NOT-DELETED <files>`.
5. **Sync.** `backlogit sync` must exit 0. Otherwise, HALT.
6. **Check Markdown.** `git status --short -- .backlogit/` must be empty.
   Otherwise, HALT: `R14-SYNC-DIRTY <paths>`. Don't commit, revert or
   re-sync. The operator decides.
7. **Re-read** under CG-S (and CG-T at a run point 3).

**Action record (replaces R13.5's):**

| Field | Value |
|---|---|
| ProposedAction | The procedure above, at the three approved run points only |
| Targets | `.backlogit/backlogit.db`, `.backlogit/backlogit.db-wal`, `.backlogit/backlogit.db-shm` (gitignored, `.gitignore` 76-78), and the in-workspace `backlogit` PIDs recorded in step 1 |
| change_kind | file delete (local cache) and process stop (by PID) |
| ActionRisk | destructive |
| Approval | operator, 2026-09-29 21:00 -07:00 ("approve the cache rebuild"), scoped by R14.4 |
| ActionResult | `approved`, not `applied`. Each run becomes `applied` only when steps 4-7 all pass. The run's time, stopped PIDs and results go in the session memory file. A halted run is `failed` and goes to the operator. |
| Rollback | None needed. The Markdown files are the source of truth, and the cache rebuilds from them. Stopped MCP servers are restarted by their client. |

#### R14.5 P2-7: `RELAY_STAGES` holds the seven full `Failed` lines

`normalize_verdict` returns `(&'static str, i32)` (`142.072-T` line 26).
A list of stage names can't give it a `'static` full `Failed` line
without a second table. So the constant holds the full lines, and
`normalize_verdict` returns the matching element.

* **The line form** is `{"state":"Failed","stage":"<Stage>"}`, from
  `142.069-T` line 35.
* R13.2's two-way equality and its wildcard-free `match` are unchanged.
  The test builds each expected line from `stage_name(f)` in that form,
  and compares the two sets both ways.

**Assembly edits (revise R13.2's A11 and A22; A21 is unchanged):**

| # | Item, line | Change |
|---|---|---|
| A11 (rev. R14) | `142.072-T` line 30 | R13.2's A11 "To" text, with "two-way set equality between `RELAY_STAGES` and `stage_name(f)` over every `Failure` variant" replaced by "two-way set equality between `RELAY_STAGES` and the lines `{"state":"Failed","stage":"<stage_name(f)>"}` built from `stage_name(f)` over every `Failure` variant". The rest is unchanged. |
| A22 (rev. R14) | `142.072-T`, a new line after line 26 | "- `pub const RELAY_STAGES: [&str; 7]`: the relay's closed set of `Failed` lines, one full line `{"state":"Failed","stage":"<Stage>"}` per stage that `stage_name` can return (the seven stages of `142.069-T` lines 23 and 35). For a child's `Failed` line, `normalize_verdict` returns the element it equals byte for byte (after stripping one trailing newline), so its result stays `&'static str`. Its own `Failed`/`Build` result is the `Build` element. It has no other stage or line list. The constant is frozen under plan R12.4 (R13.2, R14.5)." |

R13.2's scope check still applies: if `stage_name` returns a stage count
other than seven, the array length follows it.

#### R14.6 P2-9: `142.067-T` line 42 gets a feasibility row

`142.067-T` line 42: "A harness unit test asserts
`PREFLIGHT_READ.mcp_tool` is in `GENERATION_PINNED_READS`." It compares
constants only, and calls nothing of its own first.

* **Verdict: F1.** The first call is `probe_cli_read` with a nonexistent
  exe path, asserting `Spawn` (the same own item and assertion as the
  R13.6 PRE-5 scenario 1-2 row; `142.067-T` line 39). Then the membership assertion
  runs. Before PRE-5's implementation, the test fails with PRE-5's own
  placeholder marker.
* **No item-text change.** As with every R13.6 F1, the item is already
  in the task's text. The architect records the F1 at Step 2.
* **Q3 (ii) dependency.** The test names `GENERATION_PINNED_READS`, and
  every S1 harness is written at the S1 claim, before any
  implementation. Under (ii), the constant exists at Step 2 (as `&[]`),
  so the test compiles. `142.066-T` runs before `142.067-T`, so by the
  time the membership assertion runs, the value is final. Under (i) or
  (iii), this row would halt (`R12-ROUTE-C-HALT 142.067-T <test> F3`).

#### R14.7 P2-10: `read_server_gate` is `pub`

PRE-3's F1 (R13.6) calls `read_server_gate()` first from an integration
test, which is a separate crate, so the getter must be `pub`. Its
sibling `generation_activator` is `pub(crate)` (`src/server/state.rs`
line 1749), and `set_generation_activator` is `pub` (line 1745), so
"beside `set_generation_activator`" doesn't settle it. No other PRE-3
own item can come first: `install_generation_gate` and
`drive_generation_activation` are private (`142.064-T` line 28).

**Assembly edits:**

| # | Item, line | From | To |
|---|---|---|---|
| A25 | `142.064-T` line 28 | "`set_read_server_gate` / `read_server_gate`)" | "`set_read_server_gate` / `read_server_gate`, both `pub`; `read_server_gate` is called first by PRE-3's integration tests under route (c), so it isn't `pub(crate)` like `generation_activator` (plan R14.7))" |
| A26 | `142.064.001-ST` line 15 | "with `set_read_server_gate` / `read_server_gate` beside `set_generation_activator`." | "with `set_read_server_gate` / `read_server_gate` (both `pub`, plan R14.7) beside `set_generation_activator`." |

`read_server_gate` is on 064's frozen-item list for its later S1 tests
(R12.4).

#### R14.8 Other attempt-12 findings: deferred

* **P2-1** (A20 at S3-S5): deferred; follow-up stash 7BF90213 stands.
* **P2-2** (auditor for A20's pending RED): deferred to the
  harness-architect.
* **P2-4** (re-read after the subtask-close PR): closed by R14.2 step 6
  and R14.4 run point 3.
* **P2-5** (S5 cascade reasoning): closed by R14.2 (no feature in any
  manifest).
* **P2-6** (`safe-close` mode in the installed skill): deferred.
* **P2-8** (no R13.6 rows for `142.071-T`, `142.073-T`): deferred.
* **P2-11** (A22 placement by line number): deferred. A22 (rev. R14) is
  still placed after line 26.
* **P2-12** (subtask transitions): partly closed by R14.2 (assembly step
  2a, no `--force-gates`, queue→archive moves expected); the rest is
  deferred.
* **P3s (all ten):** deferred. The Constitution P3 on both SHAs is
  absorbed by R14.2 step 4.

#### R14.9 Assembly, walkthrough and operator questions

* **R11.9 step 1:** the gating review is of **Revision 14**.
* **R11.9 step 2a (new):** the read-only subtask-move check (R14.2).
* **R11.9 step 5** adds A23-A26 and the revised A11 and A22 (R14.3,
  R14.5, R14.7), and the A15 "R14" pointer.
* **R11.9 step 6:** S5 is `items: [142.058-T]`, then `142.059-T`. Record
  CG-T on S2-S5 and CP-S5 on S5. The R13.4 release-closure item is
  withdrawn.
* **R11.9 step 8:** adds the R14.3 Constitution Check deviation.
* **R11.9 step 10:** the rebuild is R14.4.
* **R11.7:** every claim of S2-S5 also needs CG-T. S5's Final row reads
  "`142-F` closes at CP-S5 (R14.2), not at Ship Step 6". The S5 Step 0.5
  row no longer mentions `142-F`.
* **Operator questions:**
  * **Q5 (confirm, not blocking the review).** R14.2 takes `142-F` out
    of S5 and has Stage move it to `done` after CP-S5. If you'd rather
    keep `142-F` in S5, Ship would archive it before the S5 subtasks
    close, which is the gap attempt 12 found.

**Scope limits.** Revision 14 changed only this plan (this subsection,
the frontmatter, and "Revision 14 note" lines in R11.7, R11.9, R12.6,
R12.7, R12.8, R13.4, R13.5, R13.6 and R13.9), `docs/operator-glossary.md`,
and `docs/memory/2026-09-30-stage-142-f-rev14-memory.md`. Stage ran
read-only checks only (`backlogit move --help`, file reads). It changed
no backlog item, edge, shipment, stash entry, source, test, `Cargo.toml`
or config file, ran no build and no cache rebuild, and made no git
mutation. **Next step:** one fresh-session scoped review of Revision 14.

### Revision 15: one task per shipment, in a separate decomposition plan

**Revision 16 is authoritative.** On 2026-09-30, after review attempt 14
failed, `docs/exec-plans/2026-09-30-142-f-decomposition-plan.md` was
rewritten in place as Revision 16. It replaces the Revision 15 content
summarized below, and its section 9 states each task's final content in
place of this plan's A-, M- and O-series edit catalogs. The direction
quoted below was a relayed summary; the operator's verbatim words are in
that document's section 1.1.

Revision 15 isn't written here. It is
`docs/exec-plans/2026-09-30-142-f-decomposition-plan.md`. It follows the
operator's 2026-09-30 direction: "one task per shipment (D-slots),
blocks-edge DAG, 142-S + branch disposition with ProposedActions,
rebuild run point 3 withdrawn (no Q6)".

* **Precedence.** For shipment shape and order, edges, the pre-assembly
  sequence, the 142-S and branch disposition, assembly steps, claim
  gates, closure, the cache rebuild and HALT tokens, the new document
  wins over Revisions 6-14 here. For unit technical design, route (c)
  mechanics and the exact text of each assembly edit (A-, M-, V-, T-,
  W-series), this plan stays authoritative at the sections the new
  document cites.
* **What it changes.** (1) S1-S5 are replaced by 21 one-task shipments,
  D1-D21, plus D19.1-D19.n for the PA-5 tasks; each manifest holds
  exactly one task. (2) Order comes from the `blocks` edges (E1-E8
  unchanged, E9.k between PA-5 tasks) and the slot number; no order-only
  edge is needed. New claim gate CG-D. (3) Every task is the only code
  task of its shipment, so Step 4.3 and the final readiness run are
  `PASS` only; the R10.1 cross-task `EXPECTED_PENDING_RED` rows and
  W1-W4 are moot. (4) 142-S and the parked branch get ProposedActions
  PS-1 to PS-9 (PS-7 branch deletion and PS-8 T0 deletion are
  destructive and unapproved). (5) R14.4 run point 3 is **withdrawn**;
  after a subtask-close PR Stage re-reads only. There is no Q6. (6)
  CG-T and the `142-F` close are keyed to slots; CP-S5 becomes CP-FINAL.
  (7) Q5 is closed, because a one-task manifest can't hold `142-F`. (8)
  A new D-NOTE edit on every task.
* **Attempt-13 P1-1.** Closed by (5): no rebuild is planned outside run
  points 1 and 2, the recorded scope of the 2026-09-29 21:00 approval.
* **Withdrawn.** Stage's earlier Revision 15 draft of 2026-09-30 (a
  consolidation that kept S1-S5 and left run point 3 `planned` behind a
  Q6) followed the wrong direction and is replaced in full. It changed
  no backlog item.
* **Kept from that draft (guards or wording only).** Attempt-13 P2-11,
  P2-12, P2-13 and `R15-EDIT-ANCHOR`. Every other attempt-13 finding stays
  open, as listed in the new document's section 14.
* **This plan is otherwise frozen.** Later review records that cite the
  new document may be appended to `## Plan Review`.

## Requirements Trace

| Requirement (source) | Unit |
|---|---|
| Verdict shape `{"state":"Succeeded"}` / `{"state":"Failed","stage":"<Stage>"}`, and exit 0 iff `Succeeded` (D2, F4 fixtures) | NEW-1, NEW-4 |
| Strict supervisor argv contract with a single deadline (D2, R11-R13) | NEW-2 |
| Supervisor mode drives the F50 production verifier and dispatches before any async runtime (D2-A) | NEW-3 |
| Relay fails closed on a missing, crashed, hung, or contradictory supervisor, and emits a normalized verdict (R22-R24) | NEW-4 |
| Launcher reaches everything through `target\debug\engram.exe` with no PATH or env override (operator, F51 spec) | NEW-5 |
| Parity map, and supersession of the 118-S fail-open guardrail in compound docs | NEW-6 |
| F50 completes `142.054.002-ST`/`003-ST` in scope before any new unit (D3, as amended by D4) | prerequisite `142.054-T` (not re-planned; under PA-1 it gains an edge on PRE-6, which covers PRE-2 transitively) |
| Build constructs a candidate through the sealed `IndexTarget::Candidate` path; one shared read-server layout (002-ST, G1) | PRE-1 |
| Seal produces the inventory and digests (002-ST, G2) | PRE-1 |
| Publish goes through F08 under the publisher lock; a failure leaves the prior generation intact (002-ST, G2) | PRE-2 |
| The read-server daemon activates the published generation after bind, with single-source identity and `_health` unchanged (F18 production wiring, G3, stash `9B7EC1E4`, D5-A) | PRE-3 |
| F54 side-effect windows cannot race PRE-3 background activation, and the F54 fixture's generation activates on every platform (attempt-3 L3-1; FASP) | PRE-3F (barrier, identity, positive test), with PRE-3 ordering and publication-last |
| The pinned read serves the opened generation's DATA, not a provenance label on managed data (G3, stash `6C5DF765`, D5-A) | PRE-4b |
| Request-entry reconciliation and pinned read context on real reads, for the allowlisted pinned read only; no control endpoint (R48, 003-ST, G3, stash `6C5DF765`) | PRE-4 |
| DaemonVerified, HealthVerified, and CliProbeVerified are observable through the real CLI, with bounded child I/O (003-ST, G4) | PRE-5 |
| McpProbeVerified is observable through the real stdio shim, with provenance and data fingerprint equal to the CLI's (003-ST, G4) | PRE-6 |
| The remaining generation-backed handlers serve the admitted context (needed for F54 GREEN) | **not in this plan**: stash `86F93068` |

## Implementation Units

These rules apply to every unit:

* Size and complexity are recorded as prose, with `size_source: agent` and
  `size_ruleset_version: engram-stage-2h-rule-v1`.
* Each production file that a unit changes counts toward the 2-hour rule. The
  new test file and its single `[[test]]` stanza in root `Cargo.toml` are the
  unit's own test-first harness. They are authored by harness-architect before
  build. This follows the F50 precedent, where `preflight.rs` and
  `preflight_gate_test.rs` are both owned by one task.
* New root targets use the `integration_` or `contract_` prefixes. The
  coverage oracle's `src/` surface globs therefore match them, and
  `--mode completeness` stays satisfied without editing the manifest.

### F50 facade prerequisites (PRE-1 through PRE-6, including PRE-4b)

These rules apply to every PRE unit:

* **Lineage.**
  * PRE-1, PRE-2, PRE-5, and PRE-6 come from stash `49809128` (D4-A).
  * PRE-3 comes from stash `9B7EC1E4`, and PRE-4b and PRE-4 come from
    `6C5DF765` (D5-A, deliberation Amendment 2).
  * PRE-3F (revision 5) is the attempt-3 L3-1 closure for PRE-3, so it
    shares PRE-3's `9B7EC1E4` lineage.
* **Test-first.** Each unit's harness is authored and confirmed RED before
  build. Each harness is a root `integration_` target, so `cargo dev-test`
  and root `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
  gate it.
* **Process discipline.** Every process-level harness, and every production
  probe spawn in PRE-5 and PRE-6, follows these rules:
  * `env_remove` for `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, `ENGRAM_DIRECT`,
    and `CARGO_BIN_EXE_engram`
  * `ENGRAM_IDLE_TIMEOUT_MS=0` in harnesses only
  * exact-child PID markers, with cleanup that stops only the recorded
    child (the F54 `ChildMarker` precedent)
  * spaced temp paths
* **Git fixtures.** Every fixture workspace is a git fixture: a `.git/`
  directory whose `HEAD` is `ref: refs/heads/main`. This is required because
  `canonicalize_workspace` rejects a workspace that has no `.git`.
* **Bounded child I/O (attempt-2 P1 #1).** No read of a child's stdout ever
  waits for EOF. On Windows, a grandchild (for example the daemon that the
  CLI or the shim auto-spawns) inherits the stdout pipe's write handle and
  can hold it open indefinitely, so EOF may never arrive after the exact
  child exits. Completion is therefore keyed on the exact child's EXIT,
  observed with `try_wait()`, and never on pipe EOF. Every child-output read
  (NEW-4, PRE-5, PRE-6) uses this one protocol:
  1. **Capped reader thread.** A dedicated reader thread owns the
     `ChildStdout`, wrapped in `Read::take(cap + 1)` so that no single
     `read` or `read_line` can buffer more than `cap + 1` bytes. It sends
     `Chunk(Vec<u8>)` (or `Line(String)` in PRE-6), `ReadError(io::ErrorKind)`,
     and `Eof` messages over a `std::sync::mpsc` channel. It stops reading and
     returns (dropping its pipe end) after `cap + 1` total bytes, after a read
     error, or at EOF. The channel is unbounded, so the reader never blocks on
     the owner.
  2. **Exit-keyed owner loop.** The owner loops every 20 ms. Each tick it
     first drains ready messages with non-blocking `try_recv()`, adding to a
     running byte total, and then calls `try_wait()` on the exact child. The
     loop ends when `try_wait()` returns `Some(status)`, the running total
     exceeds `cap`, or the deadline passes. It never blocks on the pipe.
  3. **Bounded 250 ms post-exit drain.** After `Some(status)`, the owner
     keeps collecting with `recv_timeout` until the FIRST of: an `Eof` or
     disconnect message (the fast path when no grandchild holds the pipe),
     the total exceeding `cap`, or 250 ms elapsed since the exit was
     observed. It then evaluates what it has. A grandchild still holding the
     pipe only costs this fixed 250 ms and never extends it.
  4. **Never EOF, never join.** The reader's `JoinHandle` is dropped
     (detached) and is NEVER joined. Neither `read_to_end`,
     `read_to_string`, `wait_with_output`, nor `Command::output` is used on
     any piped child in these units. A reviewer or test that finds one of
     them on a piped stdout treats it as a defect.
  5. **Kill and reap on expiry or oversize.** When the deadline passes, or
     the total exceeds `cap` before exit, the owner calls `kill()` and then
     `wait()` on the exact child only (never a process-tree or name-based
     kill). It does not run the post-exit drain on this path.
  6. **Error reporting is preserved, with a fixed precedence.** The typed
     result is decided in this order: spawn failure → `Spawn`; total > `cap`
     at any point → oversize (`Oversize` / relay cause `relay_oversize`);
     deadline expiry → `Deadline` (relay cause `relay_deadline`); a
     `ReadError` message → `Malformed` with a static `stdout_read` cause
     (relay cause `relay_read`); otherwise the captured bytes and the child's
     exit status go to the unit's normal evaluation (PRE-5/PRE-6 `Exit` or
     parse, NEW-4 `normalize_verdict`). A read that ends in the drain window
     with no line is NOT a hang and NOT an I/O error; it is evaluated as
     "no output" (`Malformed`/`Exit` in the probes, `Failed`/`Build` in the
     relay). Error values carry only the kind or cause, never environment
     values or payload bytes (the relay's ≤4 KiB stderr echo in NEW-4 is
     unchanged).
  7. **Bound.** Worst-case wall time is `deadline + kill/reap`, or
     `child exit + 250 ms`. Neither depends on any grandchild's lifetime.
* **Inherited-grandchild fixture (shared by PRE-5, PRE-6, and NEW-4).**
  Each harness proves the protocol with a REAL inherited-grandchild fixture,
  not a simulated one:
  * It is a rustc-compiled fixture exe (cross-platform, `EXE_SUFFIX`). In
    grandchild mode it re-executes `current_exe()` with a `--sleep-ms 60000`
    argument, using `Command::spawn` with stdout left as the DEFAULT
    `Stdio::inherit()`, so the grandchild genuinely inherits the pipe write
    handle. It does not redirect, close, or null the grandchild's stdout.
  * Before printing its verdict or response, the fixture writes the
    grandchild's PID to a marker file in the spaced temp dir. Only after
    that does it print the unit's expected output and exit 0 without waiting
    on the grandchild.
  * The harness uses a deadline far below 60 s (5 s). It asserts all of the
    following:
    * the call returns the expected success within `deadline + 1 s` (an
      EOF-waiting implementation would block for about 60 s and fail);
    * NON-VACUITY: the recorded grandchild PID is still alive at the moment
      the call returns, which proves that the pipe was still held open;
    * the grandchild is then killed by its recorded PID only, from a drop
      guard that also runs on assertion failure (the F54 `ChildMarker`
      precedent), and is not alive afterward.
* **Errors.** Every error is a module-local `thiserror` enum, following the
  `StoreError`/`PublishError`/`ActivationError` precedent. There is no
  `unwrap` or `expect` in library code.
  * *Revision 5 (typed error):* `CandidateBuildError` is one closed
    `#[non_exhaustive]` enum in `candidate_build.rs` with exactly these
    variants: `ReservedGenerationId`, `CandidateNotExclusive`,
    `DatabaseNotFound { count: usize }`, `UncheckpointedSidecar`,
    `RenameBusy`, `ContainmentEscape`, `StoreAbsent`, `ManifestUnreadable`,
    `RevisionOverflow`, `Index(#[source] EngramError)`,
    `Store(#[from] StoreError)`, `Publish(#[from] PublishError)`, and
    `Io { op: &'static str, kind: std::io::ErrorKind }`.
    * No variant carries a path, environment value, or payload bytes.
      `op` is a static label.
    * `read_server_layout` keeps returning `WorkspaceError` (PRE-1).
    * A new variant needs a plan revision. It is not a Ship improvisation.
* **Probe stderr (revision 5, attempt-3 CR-04).** Every production probe
  spawn in PRE-5 and PRE-6 sets `stderr(Stdio::null())`. This has two
  effects:
  * An unread piped stderr cannot fill up and block the child.
  * The CLI's or shim's diagnostics (and those of the daemon they
    auto-spawn, which inherits null) cannot add lines to the supervisor's
    stderr (invariant 4).

  Harness spawns may capture stderr for diagnostics only through the same
  bounded reader discipline.
* **HALT and return to Stage (do not improvise) if a unit would need a
  production file beyond the ones it lists.**

#### PRE-1: Read-server layout, candidate build, and seal facade

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:**
  * `src/services/generations/candidate_build.rs` (new)
  * `src/services/generations/mod.rs` (one line: `pub mod candidate_build;`)
* **Harness:**
  * `tests/integration/candidate_generation_build_test.rs`
  * `[[test]] name = "integration_candidate_generation_build"`
* **Behavior:** four public functions, plus these types and constants.
  * **Layout type.**
    `#[non_exhaustive] pub struct ReadServerLayout { pub workspace: PathBuf, pub branch: String, pub workspace_id: String, pub generations_root: PathBuf, pub runtime_root: PathBuf, pub activation_deadline: Duration }`
    * `#[non_exhaustive]` makes it readable but not constructible outside
      this crate. Every field is set only by `read_server_layout`.
    * `pub const READ_SERVER_ACTIVATION_DEADLINE: Duration = Duration::from_secs(60);`
      matches the existing activation test deadline
      (`read_server_startup_activation_test.rs:37`, `TEST_DEADLINE`). It is
      the ONLY source for `activation_deadline`. No caller passes its own
      literal to `GenerationActivator::new`.
    * `pub fn expected_identity(&self) -> ExpectedIdentity` returns
      `ExpectedIdentity::new(self.branch.clone(), self.workspace_id.clone())`.
      It is a trivial accessor, so the activator's identity (PRE-2, PRE-3,
      F50) is never assembled by hand from separate fields.
  * **`pub fn read_server_layout(workspace: &str) -> Result<ReadServerLayout, WorkspaceError>`**
    (attempt-2 P1 #3)
    * **Input type.** It takes `&str`, exactly like
      `canonicalize_workspace(&str)`. It adds no UTF-8 conversion and no
      error variant of its own. Revision 4's non-UTF-8 →
      `WorkspaceError::Failed` mapping is withdrawn.
      * *Correction (harvest session, attempt-4 P2-4; supersedes the
        revision-5 note):* `WorkspaceError` has NO `Failed` variant
        (`src/errors/mod.rs:18-43`). "Failed to parse workspace files" is
        `HydrationError::Failed { reason }` (`:45-48`). Revision 4 was
        right that the `WorkspaceError` variant does not exist. Neither
        variant is used here: taking `&str` makes a non-UTF-8 mapping
        unnecessary.
      * The daemon already holds a `String`.
      * The NEW-2 parser rejects a non-UTF-8 `--workspace` as a
        `UsageError` (exit 2) before any layout call.
    * **Canonical spelling (path prefix handling).**
      `workspace = canonicalize_workspace(workspace)?`, used EXACTLY as
      returned. It is never re-canonicalized with `std::fs::canonicalize`,
      never re-normalized, and never rebuilt from the input string.
      * It requires `.git` at the root (a directory, or a linked-worktree
        `.git` file).
      * A missing path gives `WorkspaceError::NotFound`. A directory with no
        `.git` gives `WorkspaceError::NotGitRoot`, which reaches MCP as the
        existing `NotAGitRoot` code.
      * On a primary checkout the returned spelling has the Windows `\\?\`
        prefix stripped by `normalize_canonical`. On a linked worktree it
        keeps the platform's canonical spelling. This is the same spelling
        today's daemon hashes, so `workspace_id` does not move.
    * `branch = resolve_git_branch(&workspace)`, falling back to
      `"default"` on error. This is byte-identical to
      `lifecycle_policy.rs:168`, including the `/`→`__` sanitisation and the
      12-character detached-HEAD form.
    * `workspace_id = workspace_hash(&workspace, &branch)`.
    * `generations_root = workspace.join(".engram").join("generations")`.
      It does NOT follow `ENGRAM_DATA_DIR`, because the parent plan fixes
      generations at the workspace `.engram/generations` root. The daemon's
      `data_dir` still comes from `resolve_data_dir`, unchanged.
    * `runtime_root = generations_root.join("runtime")`, the parent plan's
      runtime-copy location. It is absolute by construction, because
      `canonicalize_workspace` only ever returns an absolute path. No
      runtime check or error mapping is added for this. The PRE-1 harness
      asserts `is_absolute()`, and `validate_runtime_root` stays the
      defensive check at open time.
    * `activation_deadline = READ_SERVER_ACTIVATION_DEADLINE`.
    * It performs no writes, and does not create `.engram`, the root, or
      the runtime root.
    * It returns `WorkspaceError`, so `run_read_server_startup` calls it
      with the same `?` conversion its current `canonicalize_workspace(&workspace)?`
      already uses.
    * This is the ONLY place the read-server identity, the generations root,
      the runtime root, and the activation deadline are derived. PRE-3
      REPLACES the daemon's inline derivation with this call
      (invariant 9).
  * **Reserved ID.** `runtime` is the runtime-copy directory beneath the
    generations root. `build_candidate_generation` therefore rejects
    `id == "runtime"` with a typed `ReservedGenerationId` error before
    sealing. `mint_generation_id` can never produce that value, because of
    its `preflight-` prefix.
  * **`pub fn mint_generation_id(now: SystemTime) -> Result<GenerationId, CandidateBuildError>`**
    * pure: no filesystem access
    * format `preflight-<unix_ms>-<pid>`, validated by `GenerationId::new`
    * It is called ONLY by the NEW-3 `production_factory` (the NEW-2
      `make`). Build never mints.
  * **`pub async fn build_candidate_generation(layout: &ReadServerLayout, store: &GenerationStore, id: &GenerationId, config: &CodeGraphConfig) -> Result<BuiltCandidate, CandidateBuildError>`**
    1. `store.seal_candidate(id.clone(), id.as_str())` gives an exclusive
       `IndexTarget::Candidate` at `<root>/<id>`. Its kind is asserted to
       be `Candidate`.
    2. `index_workspace(&layout.workspace, target.path(), &layout.branch, config, true)`
       uses the candidate as the DATA root and the workspace as the source.
    3. It locates EXACTLY ONE `engram.db` under `<candidate>/cozo/`. Zero or
       more than one is a typed error.
    4. **Rename safety:**
       * If a `-wal`, `-shm`, or `-journal` sidecar exists beside it, that
         is a typed `UncheckpointedSidecar` error. The runtime-copy open
         path deletes stale sidecars, so data in the WAL would otherwise be
         silently lost.
       * `std::fs::rename` to `<candidate>/engram.db` retries only on raw OS
         error 32 (sharing violation). It makes at most 20 attempts, 50 ms
         apart, and then fails with a typed `RenameBusy` error.
       * The leftover `<candidate>/cozo/` directory is not part of the
         inventory, and it is left in place.
    5. The active manifest is never touched.
  * **`pub fn seal_candidate_inventory(built: BuiltCandidate) -> Result<SealedCandidate, CandidateBuildError>`**
    * computes the SHA-256 of `<candidate>/engram.db` as 64-character
      lowercase hex, reusing `file_tracker::compute_file_hash`
    * produces exactly one entry,
      `ManifestFileDigest::new("<id>/engram.db", digest)`
  * **Private constructors.** `BuiltCandidate` and `SealedCandidate` have
    private fields, and only these functions construct them.
* **Test scenarios (3):**
  1. **Layout, identity spelling, and minting.**
     * The fixture is a spaced git fixture: `.git/` whose `HEAD` is
       `ref: refs/heads/main`. With `W` as its path, `read_server_layout(W)`
       returns all of the following:
       * `workspace == canonicalize_workspace(W)`, byte for byte;
       * `workspace_id == workspace_hash(&canonicalize_workspace(W), "main")`.
         The oracle is NEVER `std::fs::canonicalize`;
       * on Windows, a `workspace` whose first component is not a verbatim
         (`\\?\`) prefix;
       * `generations_root == workspace/.engram/generations`;
       * a `runtime_root` that `is_absolute()` and equals
         `generations_root/runtime`;
       * `activation_deadline == READ_SERVER_ACTIVATION_DEADLINE`, and an
         `expected_identity()` equal to
         `ExpectedIdentity::new("main", workspace_id)`.
     * The same result comes back whether `W` is passed as given or in its
       `std::fs::canonicalize` spelling (the `\\?\` form on Windows). This
       proves the input spelling does not move the identity.
     * A git fixture whose `.git/` has no `HEAD` gives branch `default`.
     * A spaced directory with no `.git` gives
       `Err(WorkspaceError::NotGitRoot { .. })`, and a missing path gives
       `Err(WorkspaceError::NotFound { .. })`. Neither creates `.engram`.
     * Two `mint_generation_id` calls return valid, distinct IDs, and
       neither equals `runtime`.
     * Every fixture tree is byte-identical afterward.
  2. **Build.** The fixture has one `lib.rs` with two functions.
     * `<root>/<id>/engram.db` exists, and there is no sidecar beside it.
     * The pre-seeded active-manifest bytes are unchanged.
     * A CozoScript count over the relation that
       `CodeGraphQueries::count_functions` reads, run on a `cozo::DbInstance`
       opened on the renamed file, returns 2. This proves the data survived
       the rename.
     * Building again with the same ID fails with a typed error, and the
       first candidate is intact.
     * Building with the ID `runtime` fails with `ReservedGenerationId`,
       and creates no `<root>/runtime` candidate.
  3. **Seal.** The inventory has exactly one entry, `<id>/engram.db`. Its
     digest equals an independent `sha2::Sha256` of the file.
* **Acceptance:**
  * `cargo test --test integration_candidate_generation_build` is GREEN.
  * Root clippy (pedantic) is clean.
* **HALT to Stage** if the rename still fails after the bounded retry
  (a handle is held open in-process), or if a sidecar remains after
  `index_workspace` returns.
* **Depends on:** none (first unit).

#### PRE-2: Generation store opening and candidate publish facade

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:**
  * `src/services/generations/candidate_build.rs`
  * `src/services/generations/publish.rs` (visibility only:
    `read_current_manifest_revision` becomes `pub(crate)`, and its logic is
    unchanged)
* **Harness:**
  * `tests/integration/candidate_generation_publish_test.rs`
  * `[[test]] name = "integration_candidate_generation_publish"`
* **Behavior:** three public functions.
  * **`pub fn open_generation_store(layout: &ReadServerLayout, create: bool) -> Result<GenerationStore, CandidateBuildError>`**
    * If `<workspace>/.engram`, `generations_root`, or `runtime_root` exists,
      it must not be a symlink or reparse point (`symlink_metadata`).
      Otherwise the call fails with a typed `ContainmentEscape` error.
      *(Revision 5, CR-01: `runtime_root` is added, because activation
      writes runtime copies there.)* A per-generation directory
      `runtime/<id>` is created later by the existing F09 open path inside
      an already-checked `runtime_root`. The residual TOCTOU window, where
      the root is swapped after this check, sits at the same trust boundary
      as `.engram` itself, and is recorded under Risks.
    * When `create` is true, it runs `create_dir_all(generations_root)`.
    * `GenerationStore::new(generations_root)` must then give a
      canonical root inside the workspace. Otherwise the result is
      `ContainmentEscape`.
      * **Prefix handling.** `GenerationStore::new` canonicalizes with
        `std::fs::canonicalize` and KEEPS the Windows `\\?\` spelling
        (`store.rs:78-95`). `layout.workspace` has that prefix stripped on a
        primary checkout. `Path::starts_with` compares prefix components,
        so a `VerbatimDisk` prefix never matches a `Disk` prefix, and a
        direct `store.root().starts_with(&layout.workspace)` would reject
        EVERY Windows primary checkout.
      * **One comparator (revision 5, attempt-3 Rust/CR-02).** Containment
        uses ONE private helper, `within_root(child, root) -> Result<bool, CandidateBuildError>`.
        It is `normalize_canonical(std::fs::canonicalize(child)?)`
        `.starts_with(normalize_canonical(std::fs::canonicalize(root)?))`.
        * Both sides get the same two-step treatment, so a verbatim spelling
          can never meet a stripped one. This covers `VerbatimDisk`,
          `VerbatimUNC`, and a linked worktree's platform spelling.
        * `normalize_canonical` is `pub(crate)` (`src/db/workspace.rs:21`),
          and `candidate_build.rs` is in the same crate.
        * No other `starts_with` on a path appears in `candidate_build.rs`.
          A reviewer who finds one treats it as a defect.
      * The store's verbatim root is used only for store operations. It is
        never hashed and never compared with the layout's identity fields.
    * When `create` is false and the root is missing, it fails with a typed
      `StoreAbsent` error and creates nothing.
  * **`pub fn current_published_revision(store: &GenerationStore) -> Result<GenerationRevision, CandidateBuildError>`**
    * delegates to `read_current_manifest_revision(&store.active_manifest_path())`,
      so there is no duplicated reader
    * an absent manifest is revision 0
    * an unreadable or unparsable manifest is a typed error; it never falls
      back to 0
  * **`pub fn publish_sealed_candidate(layout: &ReadServerLayout, store: &GenerationStore, sealed: SealedCandidate, producer: &str) -> Result<GenerationRevision, CandidateBuildError>`**
    * builds the manifest from:
      * revision `current + 1`, using `checked_add`
      * `SUPPORTED_MANIFEST_SCHEMA_VERSION`
      * `BranchIdentity::new(&layout.branch, None)`
      * `WorkspaceIdentity::new(&layout.workspace_id)`
      * the sealed inventory
      * `GenerationProvenance::new(producer, Utc::now())`
    * publishes ONLY through `publish_generation_manifest`, which holds the
      lock and applies the revision guard
    * never retries
* **Test scenarios (3):**
  1. **First publish and activation.**
     * `open_generation_store(layout, true)` on a fresh fixture, then build,
       seal, and publish, returns revision 1.
     * An activator built with `GenerationActivator::new(store, layout.runtime_root.clone(), layout.expected_identity(), layout.activation_deadline)`
       calls `activate_initial` and opens it. No test or production caller
       assembles `ExpectedIdentity` or a deadline literal by hand.
     * On the opened `db()`, the function count equals 2.
     * This proves the layout, digest, identity, self-contained DB, and
       runtime root together, and runs on the spaced git fixture. On
       Windows, it also runs the containment check against the store's
       verbatim root (the prefix-handling rule above).
  2. **Second publish, and store containment.**
     * A second build, seal, and publish returns revision 2, and
       `maybe_activate_newer` switches to it.
     * `open_generation_store(layout, false)` on a fixture with no root
       fails with `StoreAbsent` and creates nothing.
     * A symlinked `.engram` (or a junction on Windows, skipped with a
       logged reason if it cannot be created without privileges) fails with
       `ContainmentEscape`.
     * *(Revision 5, CR-01)* The same holds for a symlinked or junctioned
       `generations/runtime`, and nothing is written through it. On
       Windows, a fixture whose store root is reported in verbatim form
       passes `within_root`, which proves the one-comparator rule.
  3. **Corrupt manifest.** With garbage bytes as the active manifest,
     `current_published_revision` and publish both fail with a typed
     error. The garbage bytes are byte-identical afterward.
* **Acceptance:**
  * `cargo test --test integration_candidate_generation_publish` is GREEN.
  * `integration_generation_publish` (the existing F08 target) is unchanged
    or GREEN.
  * Root clippy (pedantic) is clean.
* **Depends on:** PRE-1.

#### PRE-3F: F54 activation-settle protocol harness (attempt-3 L3-1)

> **Revision 6 override.** PRE-3F now runs FIRST in S4, after PRE-4. Its
> baseline is B0/B1, and its positive test lands GREEN with compensating
> control C1-C3. It owns the F54 evidence M1-M3, M7, M9 and M10, and the L3-1
> guard applies. See R6.3-R6.6. Text below that says "before PRE-3", "RED
> until PRE-3", or "Depends on: PRE-2" is superseded.
>
> **Revision 7 override (wins over Revision 6).** There is NO test-first
> exception, and C1-C3 are withdrawn.
>
> * **Red phase:** the stub-first harness commit HC on the S4 branch. The
>   positive test fails at step (b) with the stub marker, and every
>   pre-existing case that calls `ensure_daemon` fails there. This is
>   recorded before
>   `harness-ready`. IC then adds the identity fix and the real barrier
>   body (R7.3).
> * **Baseline:** B0 is captured at A0 on the parked branch before H5, and
>   the B1 rule applies (R7.4).
> * **Superseded below:** scenario 1 (the before-edit run), scenario 2 (RED
>   at step (a)), "At HEAD, (a) fails", and the HALT bullet "A pre-existing
>   case changes status or signature". Read them through R7.2 T2, T5, T8
>   and T13.
> * **Depends on:** PRE-4 (E2) and `142.060-T` (E8).
>
> **Revision 8 override (wins over Revisions 6-7).**
>
> * **HC placeholder body:** it is pinned in R8.2. `cargo check
>   --all-targets` and pedantic clippy must pass at both HC and IC.
> * **Per-test outcomes and owners:** the R8.3 table. This task owns only
>   the new positive test.
> * **Valid red-phase text:** R8.4.
> * **HALT rule:** R8.5.
> * **`Cargo.toml`:** HC makes no change to it. It replaces the
>   `3f890662` placeholder file (R8.1).
>
> **Revision 9 override (wins over Revisions 6-8; R9.1-R9.6).** PRE-3F
> owns a new target `integration_read_server_activation_settle`
> (`tests/integration/read_server_activation_settle_test.rs`), the shared
> support `tests/helpers/activation_settle.rs`, and ONE new `[[test]]`
> stanza in `Cargo.toml`. It does NOT edit the F54 file, which stays the
> GREEN placeholder until `142.058-T`'s FL (R9.5). Pinned API, stub, own
> fixture, HC red phase (only the positive test RED, at (b)), HALT and
> no-adjust rules, IC barrier and gates: R9.3. Ownership: R9.2 and R9.4.
> The invariant-6 exception is not exercised (PA-7). Every sentence below
> about editing the F54 file, its `ensure_daemon`, its identity lines,
> the cherry-pick, B1, "no `Cargo.toml` edit", or PA-1 as a start
> condition is superseded. Size M, complexity medium.

* **Posture:** characterization-first, then test-first. The unit captures
  the F54 per-case baseline before any edit, and its one new test is RED
  until PRE-3.
* **Size:** S. **Complexity:** medium. **Domain:** tests only. No
  production file is changed.
* **Owned file:** `tests/contract/read_server_cli_mcp_parity_test.rs`
  (the existing `contract_read_server_cli_mcp_parity` target). There is no
  `Cargo.toml` edit.
* **Ownership exception (invariant 6).** This file is `142.058-T`'s
  pending-RED harness. PRE-3F may edit it ONLY under the PA-1 grant, which
  names this exception explicitly, and ONLY within the diff contract
  below. `142.058-T` stays the file's owner and inherits the edit when it
  resumes after PRE-4. Without PA-1, PRE-3F does not exist and PRE-3 does
  not start.
* **Why a separate unit, and why before PRE-3.** PRE-3 is the change that
  makes activation possible. If the barrier landed after it, F54 would be
  racy for the whole PRE-3 → PRE-3F interval. If it were folded into
  PRE-3, that unit would exceed the file budget and mix domains. The
  barrier is correct at HEAD, where no production code installs an
  activator (G3), so it can land first without changing any case.
* **Behavior (the FASP items from the Revision 5 notes):**
  1. **Identity from the layout.** `publish_fixture_generation` computes
     `let layout = read_server_layout(workspace_str)` (PRE-1). It stamps
     `BranchIdentity::new(&layout.branch, None)` and
     `WorkspaceIdentity::new(layout.workspace_id.clone())`, replacing
     `workspace_hash(workspace, FIXTURE_BRANCH)`.
     * The `workspace_hash` import is removed if it becomes unused
       (`-D warnings`). It is not `allow`ed.
     * The fixture's database bytes, generation ID, revision 1, digest,
       store root, and producer are unchanged.
     * A layout error panics with `F54_BLOCK_MARKER`, like the fixture's
       other setup failures.
  2. **Settle barrier.** Add `async fn await_activation_settled(&mut self) -> Result<Settled, String>`
     with `enum Settled { NoActivator, Active }`. It is called exactly
     once, as the last statement of `ensure_daemon` before `Ok(())`, on
     every call, including after a respawn.
     * It polls `get_workspace_status` over direct IPC every 25 ms, until
       `SETTLE_TIMEOUT = Duration::from_secs(75)`.
     * A `null` `generation` block gives `NoActivator`.
     * `active_revision == published_revision == Some(1)` in two
       consecutive observations 250 ms apart, with EQUAL full
       `workspace_binding_fingerprint()` values and EQUAL
       `filesystem_snapshot()` maps, gives `Active`.
     * Any other state is re-polled.
     * On timeout it returns `Err` with `F54_BLOCK_MARKER` and the last
       observed `generation` block. The barrier never returns `Ok` on
       timeout.
  3. **Positive test.** Add
     `#[tokio::test] async fn published_fixture_generation_activates_at_startup_and_quiesces()`:
     * **(a) Install-before-ready.** It spawns the daemon through the
       fixture's own `spawn_daemon`, and polls `_health` directly (NOT
       through the barrier). At the FIRST `ready` observation, it reads
       `get_workspace_status` and requires a non-null `generation`.
     * **(b) Active.** It calls `await_activation_settled`, and requires
       `Active`, not `NoActivator`.
     * **(c) Contents.** `active_revision == published_revision == 1`,
       `branch_divergence.served_branch == "main"`, `diverged == false`,
       and `retained_runtime_copies.count == 1`. The file
       `<workspace>/.engram/generations/runtime/generation-142/engram.db`
       exists.
     * **(d) Quiescence.** It takes the fingerprint and the snapshot, waits
       4.5 s (more than two 2 s driver ticks at the cap), and takes them
       again. The two pairs must be equal.
     * **(e) Cleanup.** It calls `stop_daemon`.
     * Every failure message carries `F54_RED_MARKER`, and (a) names
       "generation not installed before readiness (PRE-3)". At HEAD,
       (a) fails, which is the RED witness.
* **Diff contract (reviewer check during Ship's review gate).** The diff
  contains exactly these changes:
  * the identity lines in `publish_fixture_generation` and its import
    line(s);
  * one call at the end of `ensure_daemon`;
  * the new `Settled` enum, `SETTLE_TIMEOUT`, and
    `await_activation_settled`;
  * the new positive test.

  Everything else is byte-identical, including every existing test body,
  assertion, failure message, refusal-code constant, `EXPECTED_*`
  constant, `snapshot_directory`, `changed_entry_count`,
  `workspace_binding_fingerprint`, `exercise_*` helper, and descriptor
  rule. There is no `#[ignore]`, no `should_panic`, no snapshot path
  exclusion, and no removed fingerprint field.
* **Test scenarios (2):**
  1. **Baseline preservation (characterization).** Before editing, Ship
     runs `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture`
     and records each case's status and failure signature: the first
     assertion line after the marker. After the edit, every pre-existing
     case has the SAME status and the SAME signature.
  2. **Positive RED witness.** The new test fails at step (a), with its
     named message, because no production code installs an activator yet.
     It is mapped as `EXPECTED_PENDING_RED`, with PRE-3 as its owner (Ship
     Step 4.3, Ship's own authority).
* **Acceptance:**
  * Both scenarios hold. The baseline and after-edit maps are recorded in
    the task record.
  * `cargo check --all-targets` passes, and root clippy (pedantic) is clean.
* **HALT to Stage** if either of these happens:
  * A pre-existing case changes status or signature.
  * The fixture's identity cannot be expressed through `read_server_layout`
    without changing the fixture database or generation ID.
* **Depends on:** PRE-2 (so that PRE-1 `read_server_layout` exists).

#### PRE-3: Production read-server startup activation wiring (stash `9B7EC1E4`)

> **Revision 6 override.** PRE-3 is in S1 and depends on PRE-2 (E5), not on
> PRE-3F. Every F54 item in its acceptance (the positive test RED → GREEN,
> the F54 per-case map, and the three F54 runs) moved to `142.063-T` (R6.5
> M1-M6). All other acceptance is unchanged.

* **Posture:** test-first. The risk is RS4/RS5-adjacent, because this unit
  changes read-server daemon startup.
* **Size:** S. **Complexity:** high. Hardening is below: consumer
  regression targets and a HALT rule.
* **Production files:**
  * `src/daemon/lifecycle_policy.rs`: `run_read_server_startup`, plus two
    new private functions, `install_generation_gate` and
    `drive_generation_activation` *(harvest-session text fix, attempt-4
    P2-6: `install_generation_gate` was described below but not listed;
    the harvested task is split into three subtasks, see Harvest Record)*
  * `src/server/state.rs`: one `RwLock<Option<Arc<ReadServerStartupGate>>>`
    slot, with `set_read_server_gate` and `read_server_gate` beside the
    existing `set_generation_activator`
* **Harness:**
  * `tests/integration/read_server_generation_wiring_test.rs`
  * `[[test]] name = "integration_read_server_generation_wiring"`
  * It spawns the real `env!("CARGO_BIN_EXE_engram") daemon --workspace <W>`
    with `.engram/config.toml` set to `mode = "read_server"`.
* **Behavior (D5-A, attempt-2 P1 #3 and #5):**
  * **Single-source identity (REPLACE, not duplicate).**
    `run_read_server_startup` (`lifecycle_policy.rs:159-181` at HEAD
    `4995d681`) currently derives its own identity inline.
    * **Removed:**
      * `validate_workspace_path(&workspace)?`, which today is only a
        redundant `canonicalize_workspace` (`services/connection.rs:43`);
      * `let canonical = canonicalize_workspace(&workspace)?;`
      * `let branch = resolve_git_branch(&canonical).unwrap_or_else(..);`
      * `workspace_id: workspace_hash(&canonical, &branch)`.
    * **Replaced by:** `let layout = read_server_layout(&workspace)?;` as the
      function's FIRST statement.
    * **Rewritten consumers, with the same calls and arguments:**
      * `load_or_create_workspace_id(&layout.workspace)`
      * `resolve_data_dir(&layout.workspace)`
      * `parse_config(&layout.workspace)`
    * **Snapshot:** `workspace_id: layout.workspace_id.clone()`,
      `branch: layout.branch.clone()`, and
      `path: layout.workspace.display().to_string()`.
    * **Imports.** Grep shows `run_read_server_startup` is the ONLY user of
      these four helpers in the file. The `lifecycle_policy.rs:19-20`
      `use` list therefore drops `canonicalize_workspace`,
      `resolve_git_branch`, and `workspace_hash`, and line 30
      (`use crate::services::connection::validate_workspace_path;`) is
      deleted. One import of
      `crate::services::generations::candidate_build::read_server_layout`
      is added. If a leftover import triggers an `unused_imports` warning,
      that is a clippy failure, not something to `allow`.
    * **Behavior.** The error behavior does not change. A missing path is
      still `NotFound`, and a non-git path is still `NotGitRoot`, through
      the same `?` conversion. The snapshot values do not change either:
      the path, branch, and workspace ID are the same values today's inline
      derivation computes. Every other step (the workspace UUID, the data
      dir, the config, publication, and the TTL) is unchanged.
    * **One layout value.** The SAME `layout` value is moved into
      `drive_generation_activation`. The task never calls
      `read_server_layout` again, so the snapshot's `workspace_id` and the
      activator's `ExpectedIdentity` cannot diverge.
  * **`_health` is UNCHANGED (attempt-2 P1 #5).** Hydration-ready is still
    set immediately by the existing
    `set_hydration_ready_for_generation(generation)` call, in the same place
    and order as HEAD. `_health`, `get_daemon_status`, and
    `get_workspace_status` never consult the gate, the activator, or the
    background task. Readiness semantics do not move (deferred to stash
    `5AF5CD66`, which already exists; the attempt-3 duplicate scan found no
    second entry, and `265F99BE` is a different `_health` defect).
  * **Install before readiness (revision 5, FASP ordering).** When
    `generations_root` is a directory at startup (checked with
    `symlink_metadata`), `run_read_server_startup` calls the new private
    `install_generation_gate(&state, &layout).await` AFTER the existing
    workspace publication and BEFORE `set_hydration_ready_for_generation`.
    That call:
    * runs `open_generation_store(&layout, false)`;
    * builds `GenerationActivator::new(store, layout.runtime_root.clone(), layout.expected_identity(), layout.activation_deadline)`
      and then `ReadServerStartupGate::new`;
    * calls `socket_bound()`, which is truthful because the startup driver
      runs after the IPC bind (`ipc_server.rs:441-453` and `:758-770`);
    * installs both through `set_generation_activator(Some(..))` and
      `set_read_server_gate(Some(..))`.

    The install reads no manifest, hashes nothing, copies nothing, and
    never awaits activation. The existing Release store on `hydration_ready`
    therefore orders "gate installed" before any `ready` observation. An
    install error is logged once at `warn`, installs nothing, and leaves
    the background ticks below to retry. `_health` is still set `ready` at
    the same point, whether the install succeeded or failed.
  * **Background activation.** After `set_hydration_ready_for_generation`,
    `run_read_server_startup` spawns (`tokio::spawn`, never awaited)
    `drive_generation_activation(state, layout, shutdown_tx.subscribe())`
    and returns `Ok(())` exactly as today. Startup never waits on
    activation, so bind, `_health`, and the TTL are not delayed. The task
    loops with a bounded backoff: 100 ms, doubling, capped at 2 s. It keeps
    ticking at the cap while nothing is published (no root, or revision 0),
    with no activation attempt and no log per tick. It ends when the
    shutdown signal fires. On each tick:
    1. **Install (late root only).** If no gate is installed yet and
       `generations_root` is a directory, it calls the same
       `install_generation_gate`. This covers a root that appears after
       startup.
    2. **Activate.** If a gate is installed but is not ready, it reads
       `current_published_revision(store)`. It calls
       `gate.run_initial_activation()` only when the revision is non-zero
       and ONE of these holds:
       * the revision differs from the last attempted revision; or
       * *(revision 5, transient retry, attempt-3 Rust/A3-1)* the last
         attempt on this SAME revision failed with an error that
         `services::generations::activation::classify` rates
         `RejectionClass::Transient`, its backoff (100 ms, doubling, 2 s
         cap) has elapsed, and less than
         `layout.activation_deadline` has passed since that revision's
         FIRST transient failure.

       A `Permanent` rejection is never re-attempted for the same revision
       (`5C873386`: a rejected revision is never re-hashed). Once the
       transient window is exhausted, the driver waits for a newer
       revision. The loop exits on success, and later swaps belong to the
       F20 request entry.
    3. **Errors.** Every error is logged once per distinct revision or
       error kind at `warn`, through `tracing`, with no payloads.
  * **Publication-last (revision 5, FASP).** After the activator publishes
    `active_revision` (inside `activate_initial`), the driver does only
    three things: the gate's in-memory context and phase writes (existing
    `run_initial_activation`), one stderr `tracing` line, and loop exit. It
    creates, writes, and removes no file, and it changes no
    `path`/`branch`/`db_path`/`generation` field of `get_workspace_status`
    (the fields F54's binding fingerprint reads). A driver that later adds
    such an effect must publish it BEFORE `active_revision`, or return to
    Stage.
  * **Without a generations root, PRE-3 alone changes no response
    (revision 5 wording, attempt-3 S-1).** If the root never exists, no
    gate or activator is ever installed, and nothing is created on disk.
    After PRE-3 alone, every response is byte-identical to today. The one
    intended change on such a daemon, the typed refusal of the pinned
    `stats` read, arrives with PRE-4 (invariant 7), not with PRE-3.
  * **Managed mode** never reaches this code.
* **Test scenarios (3):**
  1. **Published fixture.**
     * Revision 1 is published with PRE-2 before the daemon spawns.
     * `_health` reaches `ready`. *(Revision 5)* At the FIRST `ready`
       observation, the `get_workspace_status` `generation` block is
       already non-null (install before readiness).
     * The `get_workspace_status` generation observability (through its F54
       `cli_arguments` mapping, `--json`) reports `active_revision == 1`
       within 30 s.
     * The reported `workspace_id` equals `read_server_layout(W).workspace_id`,
       AND equals `workspace_hash(&canonicalize_workspace(W), "main")`. `W`
       is a spaced git fixture. This proves that the REPLACE left the
       daemon's identity at its pre-change value, with no prefix drift.
  2. **No root, then empty root, then corrupt, then valid, then transient.**
     * With no generations root, `_health` reaches `ready` as before. The
       generation observability is null, and `.engram/generations` is NOT
       created.
     * Creating an EMPTY root (published revision 0) causes no activation
       attempt: `last_failed_revision` and `active_revision` stay null for
       at least two capped ticks (4 s), and nothing is written under the
       root.
     * Creating the root and publishing a revision 1 whose inventory digest
       is wrong gives `last_failed_revision == 1`, and `active_revision`
       stays null. It is never re-attempted: the harness observes the same
       state after two more capped ticks.
     * Publishing a valid revision 2 then gives `active_revision == 2`
       within the bound.
     * *(Revision 5, transient retry.)* The harness uses a second daemon on
       a fresh fixture. It publishes a revision 1 whose inventory file is
       ABSENT at spawn, which the RED phase confirms `classify` rates
       `Transient`, and waits for `last_failed_revision == 1`. It then
       writes the file with the digest the manifest names, and requires
       `active_revision == 1` within 10 s, with NO new revision published.
     * At every step, `_health` stays `ready` and `get_daemon_status`
       answers without provenance, so readiness never follows the gate.
  3. **Managed mode and the non-git refusal.**
     * **Managed mode** reaches `ready`, creates no `.engram/generations`
       directory, and has no generation observability.
     * **Non-git refusal.** A read-server `engram daemon --workspace <D>`,
       where `D` is a spaced directory with a `.engram/config.toml` but no
       `.git`, must:
       * exit non-zero, with the same exit status and the same
         `NotGitRoot` error text ("is not a Git repository root") that the
         RED-phase baseline captures at HEAD, on whichever stream HEAD
         reports it;
       * leave `D` byte-identical (no `.engram/.workspace-id`, no
         `generations`).

       This is the same refusal HEAD `4995d681` gives. The RED phase
       captures it as the baseline before any change. If HEAD's refusal
       cannot be observed at the process level (for example, the daemon
       exits before logging), HALT to Stage rather than weakening the
       assertion.
* **Acceptance:**
  * The targeted harness is GREEN.
  * **Consumer regression targets are unchanged (attempt-2 P1 #5).** Each
    keeps its exact pass or pending status:
    * `integration_read_server_startup_activation`
    * `integration_read_server_restart`
      (`tests/integration/read_server_restart_test.rs`)
    * `integration_read_server_lifecycle`
      (`tests/integration/read_server_lifecycle_test.rs`)
    * `integration_doctor_smoke` (`tests/integration/doctor_smoke_test.rs`)
    * `integration_direct_sync_mode`
      (`tests/integration/direct_sync_mode_test.rs`)
    * `integration_doctor_read_pin`, `integration_lifecycle_read_generation_pin`,
      and `unit_app_state_mode`
    * every case in `contract_read_server_cli_mcp_parity` (F54, `142.058-T`),
      against the PRE-3F after-edit map. *(Revision 5.)* Each case keeps
      BOTH its status AND its failure signature. The ONE intended change is
      that PRE-3F's `published_fixture_generation_activates_at_startup_and_quiesces`
      goes RED → GREEN in PRE-3.
  * **F54 determinism evidence (revision 5, L3-1).** Ship runs
    `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture`
    three consecutive times after the PRE-3 build. It records the three
    per-case maps, which must be identical. Any difference is a HALT, not
    a flake to retry.
  * **Consumer baseline (RED phase, before any production edit).** Ship
    runs each target above at HEAD and records its per-case pass/fail map
    in the task record. Ship also records a grep showing that none of the
    first five non-F54 targets sends a `GENERATION_PINNED_READS` method
    (`get_workspace_statistics` / `stats`) to a read-server daemon; at
    Stage's check (HEAD `4995d681`) only F54 does, and F54 publishes its own
    fixture generation. If the grep finds another sender, HALT to Stage
    before building PRE-3.
  * **HALT to Stage if any of those goes RED** relative to the baseline,
    in PRE-3, PRE-4b, or PRE-4. Never edit, skip, `#[ignore]`, or
    re-baseline a consumer test, and never gate `_health` to make one pass.
  * **REPLACE evidence.** In the diff, `run_read_server_startup` contains
    no call to `canonicalize_workspace`, `resolve_git_branch`,
    `workspace_hash`, or `validate_workspace_path`, and exactly one call to
    `read_server_layout`. Ship's code review records this. A derivation
    that duplicates the layout next to it is a PRE-3 defect, not a style
    note.
  * Root clippy (pedantic) is clean, with no `unused_imports` allow.
* **HALT to Stage** in any of these cases:
  * the wiring needs any file other than the two listed;
  * the install before readiness cannot run before
    `set_hydration_ready_for_generation` without reading the manifest;
  * the RED phase shows that an absent inventory file is classified
    `Permanent`. Stage then picks another transient trigger. The scenario
    is never dropped.
* **Depends on:** PRE-3F (and therefore PRE-2).

#### PRE-4b: The pinned statistics read serves the opened generation (stash `6C5DF765`)

> **Revision 6 override.** In S1. The F54 comparison in its acceptance moved
> to `142.063-T` (R6.5 M7-M8).

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:**
  * `src/tools/read.rs`
  * `src/tools/mod.rs` (the one dispatch arm)
* **Harness:**
  * `tests/integration/generation_pinned_statistics_test.rs`
  * `[[test]] name = "integration_generation_pinned_statistics"`
  * It runs in-process, following the `lifecycle_read_generation_pin_test`
    and `read_server_startup_activation_test` setup precedent.
* **Behavior (attempt-2 P1 #6):**
  * **Today (the defect).** `get_workspace_statistics` has no context
    parameter. It always builds its context with
    `pinned_read_request_context` → `ReadRequestContext::from_managed_state`
    (`read.rs:95-104`) and opens `connect_db(context.data_dir(), context.branch())`
    (`read.rs:106-111`). A generation context passed to
    `dispatch_with_read_context` only labels the response.
  * **Signature (revision 5: 2-arg compatibility, attempt-3 Rust P2).**
    * The existing `pub async fn get_workspace_statistics(state, params)` is
      KEPT with its 2-arg signature. Its body becomes a one-line delegation
      to `get_workspace_statistics_with_context(state, params, None)`, so
      its managed behavior is byte-identical.
      `tests/integration/core_read_generation_pin_test.rs:352` calls it as
      `read::get_workspace_statistics(state, None)` and is not edited.
    * New: `pub async fn get_workspace_statistics_with_context(state, params, read_context: Option<&Arc<ReadRequestContext>>)`.
      * `Some(ctx)` is used as-is. On this path the handler never calls
        `pinned_read_request_context` or `ReadRequestContext::from_managed_state`.
      * `None` keeps today's `pinned_read_request_context` managed path,
        byte for byte.
  * **Dispatch arm.** The `dispatch_with_read_context` arm for
    `get_workspace_statistics` (`tools/mod.rs:476`) calls
    `read::get_workspace_statistics_with_context(state.clone(), params, read_context)`.
    Every other arm is unchanged. PRE-4 supplies `Some(&ctx)` from
    `process_request`. The existing in-process tests are the only other
    callers, and they reach the managed path through the 2-arg wrapper or
    a `None` context.
  * **Queries (typed conversion).** `queries_from_read_context` resolves the
    data source in this one shared seam, not in the handler. The
    `ReadRequestContext::generation()` doc forbids handler-level mode
    branching, and this seam keeps to it. A `read.rs` doc comment on the
    seam records why.
    * When `context.generation()` is `Some(g)`, it returns
      `CodeGraphQueries::new(CozoDb { inner: Arc::new(g.opened_generation().db().clone()) })`.
      Stage checked each step against source:
      * `OpenedGeneration::db() -> &cozo::DbInstance` (`src/db/cozo_backend/mod.rs:501`)
      * `cozo::DbInstance` is `#[derive(Clone)]` (cozo 0.7.6 `lib.rs:99`),
        and a clone shares the same storage handles
      * `CozoDb { pub(crate) inner: Arc<cozo::DbInstance> }` (`mod.rs:36-37`)
      * `CodeGraphQueries::new(CozoDb)` (`src/db/cozo_queries.rs:576`)

      So `src/db/` is untouched.
    * Otherwise (a managed context) it uses today's `connect_db`.
    * It never calls `connect_db` on a generation context. Such a call
      would open or bootstrap a new, empty managed-style database under the
      runtime copy's parent directory (`from_generation` sets `data_dir`
      there, in `src/server/state.rs`). That empty database is exactly
      the silent wrong-data path this unit removes.
  * **Embedding and registry fields.** `embedding::status(Some(&cg_queries))`
    uses the same generation-backed queries.
    `registry::load_registry_status(ctx)` already returns `Ok(None)` when
    `root_path()` is `None` (generation contexts), so it needs no change.
  * `query_memory` and every other caller of `queries_from_read_context`
    are unchanged. They pass only managed contexts.
* **Test scenarios (3):**
  1. **Pinned read.**
     * The fixture (one `lib.rs`, two functions) is built and published
       with PRE-1/PRE-2. It is activated through `GenerationActivator` and
       `ReadServerStartupGate::run_initial_activation`.
     * `dispatch_with_read_context(state, "get_workspace_statistics", None, Some(&ctx))`
       returns `code_files == 1`, `functions == 2`, and
       `/provenance/generation_id == <id>`.
  2. **The data is real, not just a label.**
     * The harness binds a managed workspace snapshot for the SAME fixture
       workspace on the same state, and leaves its managed data dir
       unindexed. Then `dispatch(state, "get_workspace_statistics", None)`
       returns the managed result: `code_files == 0` and no provenance.
     * The assertion that must hold is that the pinned `code_files` (1)
       differs from the managed `code_files` (0) for the same workspace.
       If the old handler were kept, both reads would return 0, and the
       test would fail.
     * If a ReadServer-mode state cannot hold a managed snapshot, use a
       managed-mode `AppState` bound to the same workspace path as the
       baseline. HALT to Stage rather than drop or weaken the differing
       assertion.
  3. **No stray databases.**
     * The harness lists the files under `ctx.data_dir()` (the runtime
       copy's parent directory) before and after the pinned read. The two
       listings are equal, so no `connect_db` bootstrap ran on the
       generation context.
     * A `query_memory` dispatch with no context is unchanged.
* **Acceptance:**
  * The targeted harness is GREEN.
  * **Diff evidence (reviewer check during Ship's review gate):**
    * In `get_workspace_statistics`, `pinned_read_request_context` and
      `from_managed_state` are reachable only on the `None` branch.
    * In `queries_from_read_context`, `connect_db` is reachable only when
      `context.generation()` is `None`.
    * No generation-context fallback to managed data exists (D5-D).
  * `integration_read_response_provenance` (or the existing F16/F17
    provenance targets) is unchanged or GREEN.
  * `integration_core_read_generation_pin` is unchanged, and its source
    file is not in the diff (revision 5).
  * The PRE-3 consumer regression targets are unchanged against the PRE-3
    baseline, including F54's per-case status AND signature (revision 5).
    The `None` context keeps today's managed path. HALT if any goes RED.
  * Root clippy (pedantic) is clean.
* **HALT to Stage** in either of these cases:
  * `CodeGraphQueries` cannot be built from the opened generation without
    editing `src/db/`, for example because `CozoDb` gains a field or
    `inner` stops being `pub(crate)`.
  * Scenario 1 returns `code_files == 0` through the cloned `DbInstance`,
    which means the clone does not observe the generation's data.

  Never add a managed-data fallback, and never widen the owned files.
* **Depends on:** PRE-2.

#### PRE-4: Route the pinned read through request-entry admission (stash `6C5DF765`)

> **Revision 6 override.** In S1. The F54 row signatures and the three F54
> runs moved to `142.063-T` (R6.5 M9-M11). Scenario 2's quiescence check
> defines the two-observation rule inline (R6.5), because PRE-3F is now in S4.

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:** `src/daemon/request_entry.rs`.
* **Harness:** it extends `tests/integration/read_server_generation_wiring_test.rs`
  (PRE-3).
* **Behavior (D5-A, attempt-2 P1 #5 and #6):**
  * `pub const GENERATION_PINNED_READS: &[&str] = &["get_workspace_statistics"];`
    lists the methods whose handlers read the admitted context. A method
    joins this list only in the same change that converts its handler
    (`86F93068`).
  * **The pinned path.** In `process_request`'s dispatch arm, a request
    takes the new path only when `state.mode() == ReadServer` AND the
    method is in `GENERATION_PINNED_READS`:
    * With no gate installed, it returns a typed refusal,
      `ActivationError::GenerationNotYetActivated`, through the existing
      `refusal` helper.
    * Otherwise it calls `admit_read(&gate, &request)`, which triggers the
      existing background `maybe_activate_newer`.
    * `Admitted(ctx)` is dispatched with
      `tools::dispatch_with_read_context(state, method, params, Some(&ctx))`.
      This is the only place an admitted context enters dispatch. PRE-4b's
      arm then hands it to the handler's generation-backed queries.
    * `Refused(resp)` returns `resp`.
  * **Every other request is byte-identical to today.** That covers all
    other methods (including `get_daemon_status`, `get_workspace_status`,
    and `_health`) and all of managed mode. `_health` readiness never
    consults the gate (attempt-2 P1 #5).
  * No generation-control endpoint is called or added (R48).
* **Test scenarios (3):**
  1. **Real CLI read.**
     * Against the PRE-3 published fixture,
       `engram --workspace <W> --json stats` returns
       `/provenance/generation_id == <id>` and `code_files == 1`.
     * `stats` is the F54 `cli_arguments` mapping of
       `get_workspace_statistics`.
  2. **Reconciliation at request entry.**
     * While the daemon runs, PRE-2 publishes revision 2 from a fixture that
       adds a second file.
     * Bounded re-probing (50 ms, doubling, capped at 500 ms, with a 30 s
       limit) observes provenance equal to the second ID and
       `code_files == 2`.
     * The harness sends only `stats`, `status`, and `_health`.
     * A static check asserts that no descriptor declares a
       generation-control method.
     * *(Revision 5: reconciliation quiescence, for F54's matrix
       windows.)* The harness waits until the revision-2 state is settled,
       using the same two-observation rule as the PRE-3F barrier. It then
       takes the binding fingerprint and a filesystem snapshot, sends
       three `stats` reads (each triggers request-entry
       `maybe_activate_newer` against the UNCHANGED manifest), waits two
       capped ticks, and takes both again. They are equal. A pinned read
       therefore cannot leak activation side effects into a later F54
       refusal window.
  3. **Regression witness.**
     * A read-server daemon with no generations root refuses `stats` with
       the typed not-yet-activated code.
     * `daemon-status` and `status` are accepted exactly as before, with no
       provenance.
     * A managed-mode `stats` is unchanged and carries no provenance.
* **Acceptance:**
  * The targeted harness is GREEN.
  * `integration_request_entry_activation` and the PRE-3 consumer
    regression targets are unchanged against the PRE-3 baseline. HALT if
    any goes RED.
  * Root clippy (pedantic) is clean.
  * Ship records any F54 case that changes status in the Step 4.3 map.
    F54's equivalence case pins `get_workspace_status`, so it is expected
    to stay RED until `86F93068`.
  * *(Revision 5)* F54 cases keep BOTH their status and their failure
    signature against the PRE-3 map. The one exception is rows whose
    method is `get_workspace_statistics` (the only pinned read). Their
    signature MAY change, and each change is recorded in the Step 4.3 map
    with its new text.
    * Expected consequence: F54's fixture generation database holds only
      a `probe_row` relation (`read_server_cli_mcp_parity_test.rs:483-495`).
      So an admitted `stats` read against it may return a typed query
      error instead of counts.
    * That is F54's own GREEN concern, together with `86F93068`. This plan
      does not change the F54 fixture database.
  * The F54 determinism evidence (three identical consecutive runs) from
    PRE-3 is repeated after the PRE-4 build.
* **Depends on:** PRE-3 and PRE-4b.

#### PRE-5: CLI provenance probe facade

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:**
  * `src/preflight_probe.rs` (new, top level: attempt-2 P1 #2)
  * `src/lib.rs` (one line: `pub mod preflight_probe;`)
* **Harness:**
  * `tests/integration/preflight_probe_test.rs`
  * `[[test]] name = "integration_preflight_probe"`
* **Behavior:** synchronous, using `std::process` and `std::thread` only.
  No async runtime is needed, and the public API exposes no `serde_json`
  types.
  * **Read descriptors.**
    * `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct ProbeRead { pub cli_args: &'static [&'static str], pub mcp_tool: &'static str, pub mcp_arguments_json: &'static str }`
      * Every field is a `std` borrowed static, so F50 (whose crate has no
        `serde_json`) can name and pass the descriptors directly.
    * `pub const PREFLIGHT_READ: ProbeRead` is
      `{ cli_args: &["stats"], mcp_tool: "get_workspace_statistics", mcp_arguments_json: "{}" }`.
    * `pub const HEALTH_READ: ProbeRead` is
      `{ cli_args: &["health"], mcp_tool: "get_health_report", mcp_arguments_json: "{}" }`.
  * **Report type.**
    `pub struct ProbeReport { pub accepted: bool, pub generation_id: Option<String>, pub data_fingerprint: Option<String> }`
    * `data_fingerprint` is `code_files=<n>;functions=<n>;classes=<n>;interfaces=<n>;edges=<n>`,
      built from the result object. That lets the probes compare DATA, not
      just labels.
  * **`pub fn probe_cli_read(engram: &Path, workspace: &Path, read: &ProbeRead, deadline: Instant) -> Result<ProbeReport, ProbeError>`**
    * spawns ONLY the absolute, canonicalized `engram` as
      `--workspace <W> --json <cli_args...>`
    * stdin is null
    * uses the bounded child I/O discipline, with a 1 MiB cap, keyed on the
      CLI child's exit (the daemon it auto-spawns is the real-world
      inherited grandchild)
    * parses the first JSON line of the bytes captured by exit + 250 ms
      drain; it never waits for EOF
  * **`pub fn wait_for_generation(engram: &Path, workspace: &Path, read: &ProbeRead, expected: &str, deadline: Instant) -> Result<ProbeReport, ProbeError>`**
    * re-probes with a bounded backoff (50 ms, doubling, capped at 500 ms)
      until `generation_id == expected` or the deadline passes
    * This is the DaemonVerified observation, and it relies on
      request-entry reconciliation only.
  * **Private helpers:** `run_bounded` (the discipline) and `parse_report`.
  * **`ProbeError`** is `Spawn`, `Deadline`, `Exit`, `Oversize`, `Malformed`,
    or `Mismatch`. `Malformed` carries only a static cause (for example
    `stdout_read` for a reader I/O error). It never contains environment
    values or payload bytes. It is a module `thiserror` enum with no
    `#[from] serde_json::Error` and no `serde_json` payload in any variant.
  * `PREFLIGHT_READ.mcp_tool` must be in `GENERATION_PINNED_READS`. A unit
    test in the harness asserts this.
* **Test scenarios (3):**
  1. **Real daemon.**
     * Against a read-server fixture published with PRE-2 (with PRE-3 and
       PRE-4 wired), `probe_cli_read(PREFLIGHT_READ)` returns the published
       ID and `code_files=1;functions=2;…`.
     * `probe_cli_read(HEALTH_READ)` is `accepted`.
  2. **Mismatch.** `wait_for_generation` with a non-published expected ID
     fails with `Mismatch` or `Deadline` within `deadline + 1 s`. With the
     published ID it returns `Ok`.
  3. **Process discipline, using rustc-compiled fixtures.**
     * A nonexistent exe fails with `Spawn`.
     * A hanging fixture is killed at the deadline, and its PID is not
       alive afterward.
     * The shared **inherited-grandchild fixture** (see the PRE rules)
       prints one valid JSON report line and exits 0. With a 5 s deadline,
       the probe returns the parsed report within `deadline + 1 s`, the
       grandchild PID is asserted alive at return (non-vacuity), and it is
       then killed by its recorded PID.
     * Output over the cap fails with `Oversize` (not `Deadline`), and the
       child is reaped.
* **Acceptance:**
  * The targeted harness is GREEN, including the inherited-grandchild case
    with its non-vacuity assertion.
  * `src/preflight_probe.rs` contains no `read_to_end`, `read_to_string`,
    `wait_with_output`, `.output()`, or reader `join()` on a piped child
    (reviewer check during Ship's review gate).
  * Root clippy (pedantic) is clean.
* **Depends on:** PRE-4.

#### PRE-6: Stdio MCP provenance probe facade

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:** `src/preflight_probe.rs`.
* **Harness:** it extends `tests/integration/preflight_probe_test.rs`.
* **Behavior:**
  `pub fn probe_mcp_read(engram: &Path, workspace: &Path, read: &ProbeRead, deadline: Instant) -> Result<ProbeReport, ProbeError>`,
  plus a private `run_mcp_session`.
  * `mcp_arguments_json` is parsed internally BEFORE the shim is spawned.
    Invalid JSON (or a non-object) is `Malformed` with the static cause
    `mcp_arguments`, and no child is spawned. `serde_json` never appears in
    the signature.
  * It spawns the absolute `engram` as `shim --workspace <W>`, with piped
    stdin and stdout. The reader thread sends LINES over the channel, as in
    the bounded discipline, reading through `take(cap + 1)` (1 MiB cap) so
    one unterminated line cannot grow without bound.
  * Every "await a response line" below is a `recv_timeout` on the channel,
    sliced to at most 20 ms and interleaved with `try_wait()` on the shim.
    It is never a blocking read on the pipe. If the shim exits before the
    awaited line arrives, the 250 ms post-exit drain runs once, and a still
    missing line is `Exit` (or `Malformed` for a non-JSON line).
  * The session steps are:
    1. Write `initialize`, then await its response line, bounded by the
       deadline.
    2. Write `notifications/initialized`, then one `tools/call`.
    3. READ the matching `tools/call` response BEFORE closing stdin. This
       is the `142.060-T` read-before-close lesson.
    4. Drop stdin, then run the `try_wait` loop until the child exits or
       the deadline passes, and kill and reap on expiry. The result is
       decided from the response already read in step 3 and the shim's exit
       status. The probe never waits for stdout EOF and never joins the
       reader, because the daemon that the shim auto-spawns may still hold
       the pipe.
  * It extracts `generation_id` and `data_fingerprint` from the tool
    result's structured content, using the same extraction the F54
    harness uses for MCP.
* **Test scenarios (3):**
  1. **Real daemon.** The MCP read of `PREFLIGHT_READ` returns the same
     generation ID AND the same `data_fingerprint` as `probe_cli_read`.
  2. **Bad output.** A fixture that emits non-JSON, or exits before
     responding, fails with `Malformed` or `Exit`. A test-local `ProbeRead`
     whose `mcp_arguments_json` is invalid fails with `Malformed` and spawns
     nothing.
  3. **Hangs.**
     * A fixture that never answers is killed at the deadline, and its PID
       is not alive afterward.
     * The shared **inherited-grandchild fixture**, in MCP mode, spawns its
       inheriting grandchild, answers `initialize` and the `tools/call`
       with valid JSON-RPC lines, and exits 0 when stdin closes. With a 5 s
       deadline, `probe_mcp_read` returns `Ok` within `deadline + 1 s`, the
       grandchild PID is asserted alive at return (non-vacuity), and it is
       then killed by its recorded PID.
* **Acceptance:**
  * The targeted harness is GREEN, including the MCP inherited-grandchild
    case with its non-vacuity assertion.
  * The same no-EOF/no-join reviewer check as PRE-5 holds for
    `run_mcp_session`.
  * Root clippy (pedantic) is clean.
* **Depends on:** PRE-5.

#### F50 completion after PRE-6 (existing `142.054-T`, not a new unit)

This is guidance only. `142.054-T`'s owned files are unchanged; PA-2b
appends it to the implementation notes of `002-ST` and `003-ST`. F50 extends
`preflight_gate_test.rs` RED-first, then implements a concrete
`ProductionVerifier` in `preflight.rs`. Its inputs are exactly
`{layout, engram executable, deadline}` plus the `expected_generation` that
the NEW-3 factory minted. It OWNS a current-thread tokio runtime and never
runs inside another runtime. The `layout` is the one value produced by the
NEW-3 factory's single `read_server_layout` call. F50 never re-derives the
workspace path, branch, or `workspace_id`, never hashes the invocation's
`--workspace` spelling, and never canonicalizes a path itself. Each stage
maps to the facades like this:

| Stage | Composition |
|---|---|
| Build | `open_generation_store(&layout, true)` (the first write happens here, after the deadline is computed); `GenerationId::new(expected)`; `build_candidate_generation`. It never mints. |
| Seal | `seal_candidate_inventory` |
| Publish | `publish_sealed_candidate(.., "engram-preflight")` |
| DaemonVerified | `wait_for_generation(PREFLIGHT_READ, expected)` |
| HealthVerified | `probe_cli_read(HEALTH_READ).accepted` |
| CliProbeVerified | `probe_cli_read(PREFLIGHT_READ)` with `generation_id == expected` |
| McpProbeVerified | `probe_mcp_read(PREFLIGHT_READ)` with `generation_id == expected` AND `data_fingerprint ==` the CLI fingerprint |

**Real-verifier requirement.** F50's completion evidence MUST include a
real-chain case, not only mocks. It drives `ProductionVerifier` with
`env!("CARGO_BIN_EXE_engram")`:

* A spaced read-server git fixture reaches `Succeeded`.
* A managed-mode fixture fails at `DaemonVerified`.

This is possible because `preflight_gate_test` is a root target that
`#[path]`-includes `preflight.rs`.

The verifier also fixes the six pedantic lints: it replaces
`Result<(), ()>` with a typed `VerifyError`, and adds the `# Errors` and
`#[must_use]` attributes. `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic`
becomes an explicit F50 acceptance criterion (PA-2b). These inputs satisfy
NEW-3's HALT conditions 1 to 4 by construction.

### NEW-1: Supervisor preflight verdict runner

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:**
  * `crates/engram-indexer/src/preflight_verdict.rs` (new)
  * `crates/engram-indexer/src/lib.rs` (one line: `pub mod preflight_verdict;`)
* **Harness:**
  * `tests/integration/preflight_verdict_test.rs`
  * `[[test]] name = "integration_preflight_verdict"`
  * The harness includes `preflight.rs` and `preflight_verdict.rs` through
    `#[path]`, with `#[allow(dead_code)]` placed only on those two `mod`
    declarations.
* **Behavior:**
  * `pub fn run<V: Verifier>(deadline: Instant, expected_generation: String, verifier: &mut V) -> Verdict`
    drives `Preflight::<Build>::start` through all seven `verify` calls.
  * `Verdict::json_line(&self) -> &'static str`-style canonical text, with no
    trailing newline.
  * `Verdict::exit_code(&self) -> u8` returns 0 for `Succeeded` and 1 for
    `Failed`. `pub const EXIT_USAGE: u8 = 2`.
  * `pub const fn stage_name(Failure) -> &'static str` is an exhaustive match.
    The wire name is never derived from `Debug`.
  * The module uses only `std` and `crate::preflight`. It has no other
    `crate::` path and no `engram::` path.
* **Test scenarios (3):**
  1. All-pass mock: the verdict is `Succeeded`, the text is byte-exact
     `{"state":"Succeeded"}`, and the exit code is 0.
  2. Seven-stage failure table: each case yields `Failed` with its exact stage
     name, exit 1, and no later stage calls. A zero-length deadline yields
     `Failed`/`Build`.
  3. `stage_name` is an exhaustive mapping equal to the F52 `PreflightStage`
     set and the F53 constant `McpProbeVerified`.
* **Acceptance:**
  * `cargo test --test integration_preflight_verdict` is GREEN, and the new
    target compiles with zero warnings.
  * `cargo build -p engram-indexer` succeeds.
  * `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic` is clean.
* **Depends on:** `142.054-T`.

### NEW-2: Supervisor invocation contract and dispatch function

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:**
  * `crates/engram-indexer/src/preflight_invocation.rs` (new)
  * `crates/engram-indexer/src/lib.rs` (one line)
* **Harness:**
  * `tests/integration/preflight_invocation_test.rs`
  * `[[test]] name = "integration_preflight_invocation"`
  * `#[path]` includes `preflight.rs`, `preflight_verdict.rs`, and
    `preflight_invocation.rs`, with scoped `#[allow(dead_code)]`.
* **Behavior:**
  * `pub fn parse_invocation(args: impl IntoIterator<Item = OsString>) -> Result<Invocation, UsageError>`
    is strict:
    * `--workspace <PATH>` is required. It must be valid UTF-8; a
      non-UTF-8 value is a `UsageError`, because `read_server_layout` takes
      `&str`. It is checked to be an existing directory.
      * The parser keeps the value as given. It does NOT canonicalize or
        normalize it.
      * Canonicalization and the `.git` requirement belong to
        `read_server_layout` alone (invariant 9), so the supervisor cannot
        produce a second spelling of the identity.
      * A missing `.git` is NOT a usage error. It surfaces from the factory
        as a `SetupError`.
    * `--timeout-ms <u64>` is required.
    * `--engram <PATH>` is required. It must be absolute, canonicalized, and
      an existing regular file.
    * The parser rejects duplicate flags, unknown flags, positional extras,
      and a flag token used as a value.
  * `pub fn deadline_from(now: Instant, timeout_ms: u64) -> Result<Instant, UsageError>`
    uses `checked_add`. Overflow is a `UsageError`.
  * `pub fn main_with<V, F>(args, make: F) -> PreflightOutput` has the bound
    `F: FnOnce(&Invocation, Instant) -> Result<(V, String), SetupError>`. It
    returns `PreflightOutput { stdout: Option<&'static str>, stderr: String, exit_code: u8 }`.
    * A usage error returns `stdout: None` and exit 2.
    * A setup error returns `Failed`/`Build` and exit 1.
    * Otherwise it calls `run`.
    * `make` runs only after the deadline is computed.
      * The contract forbids `make` from mutating the filesystem.
      * `make` returns the verifier and the expected generation ID. In
        production, the ID is minted in `make` by the pure PRE-1
        `mint_generation_id`.
      * Every write happens inside the Build stage (the D3 verifier), which
        never mints.
  * Exactly one structured stderr line is emitted:
    `engram-preflight stage=<Stage|none> cause=<ok|usage|setup|deadline|verifier> elapsed_ms=<n>`.
    It never contains environment values or probe payloads.
* **Test scenarios (3):**
  1. A valid invocation with a spaced workspace parses, and its workspace
     is kept exactly as given. A relative or missing `--engram` is
     rejected.
  2. Strict-parser table, where every case gives exit 2 and no stdout:
     missing flag, duplicate flag, unknown flag, positional extra,
     non-numeric value, flag used as a value, `u64::MAX` overflow, a
     nonexistent or non-directory workspace, and (on Unix, where it can be
     constructed) a non-UTF-8 workspace.
  3. `main_with` with a mock factory:
     * `Succeeded` gives the exact line and exit 0.
     * `SetupError` gives `Failed`/`Build` and exit 1.
     * `--timeout-ms 0` gives `Failed`/`Build`, and the verifier is never
       invoked.
     * The stderr line format is asserted.
* **Acceptance:**
  * The targeted harness is GREEN.
  * The same build and clippy commands as NEW-1 pass.
* **Depends on:** NEW-1.

### NEW-3: `engram-indexer preflight` entrypoint

* **Posture:** test-first.
* **Size:** XS. **Complexity:** medium (it depends on the D3 verifier API).
* **Production files:** `crates/engram-indexer/src/main.rs`.
* **Harness:**
  * `crates/engram-indexer/tests/preflight_entrypoint_test.rs`, crate-local
    and auto-discovered. It uses `env!("CARGO_BIN_EXE_engram-indexer")`.
  * **harness_cmd:** `cargo test -p engram-indexer --test preflight_entrypoint_test`.
* **Behavior:**
  * `main` becomes a sync `fn main() -> ExitCode`.
  * When `env::args_os().nth(1) == "preflight"`, it calls
    `preflight_invocation::main_with(rest, production_factory)`, prints
    `stdout` plus `\n`, writes stderr, and exits. This happens before any
    tokio runtime exists.
  * Otherwise it builds the current-thread runtime explicitly and runs the
    unchanged legacy env path.
  * `production_factory` performs three steps, and none of them writes to
    disk:
    1. `engram::services::generations::candidate_build::read_server_layout(workspace)`,
       where `workspace` is the invocation's validated UTF-8 `&str`. It is
       read-only. Any `WorkspaceError` (including `NotGitRoot` for a
       non-git workspace, and `NotFound`) maps to the typed
       `SetupError::Layout`. That gives `Failed`/`Build` with stderr
       `cause=setup`. The stderr line never includes the path or the error
       text.
    2. `mint_generation_id(SystemTime::now())`.
    3. Constructing the F50 production verifier delivered by
       `142.054.002-ST`/`003-ST`, and moving the one `layout` value into it.
  * It returns `(verifier, id)`. Every child is launched by absolute path
    only.
* **Test scenarios (3):**
  1. **Zero deadline versus non-git setup**, with the same verdict and
     different causes:
     * `preflight --workspace <spaced git fixture> --timeout-ms 0 --engram <abs>`:
       * stdout is exactly `{"state":"Failed","stage":"Build"}\n`
       * exit 1
       * stderr `cause=deadline`
       * the fixture is byte-for-byte unchanged
     * The same invocation against a spaced directory with no `.git`, with a
       non-zero timeout, gives the same stdout and exit 1, stderr
       `cause=setup` (the typed `SetupError::Layout`), and a directory that
       is byte-for-byte unchanged (no `.engram`).
  2. Usage error: exit 2 and empty stdout.
  3. No arguments: the legacy `missing required environment variable`
     failure and exit code are unchanged.
* **Acceptance:**
  * The targeted harness is GREEN.
  * `cargo build -p engram-indexer` succeeds.
  * `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic` is clean.
* **HALT and return to Stage (do not add flags) if the D3 verifier:**
  1. needs inputs that cannot be derived from `{workspace, engram, deadline}`;
  2. has no generation-minting API;
  3. resolves any executable through PATH or the cwd;
  4. needs async work inside an existing runtime, unless it owns its own
     runtime.
* **Depends on:** NEW-2, and `142.054-T` including 002-ST and 003-ST.

### NEW-4: Root relay library

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:**
  * `src/cli/commands/preflight.rs` (new; it is not placed in the IPC-only
    `lifecycle.rs`)
  * `src/cli/commands/mod.rs` (one line)
* **Harness:**
  * `tests/contract/cli_preflight_relay_test.rs`
  * `[[test]] name = "contract_cli_preflight_relay"`
  * It calls `engram::cli::commands::preflight` directly against a
    rustc-compiled fixture supervisor at an explicit temp path. The temp dir
    comes from `tempdir_in(env!("CARGO_MANIFEST_DIR"))` and contains spaces.
    `rustc` is resolved with `EXE_SUFFIX`, so the harness is cross-platform.
  * Every spawn calls `env_remove("ENGRAM_DATA_DIR")`.
  * The harness also `#[path]`-includes `preflight.rs` and
    `preflight_verdict.rs`, with `#[allow(dead_code)]` placed only on those
    two `mod` declarations. That lets it assert that the relay's closed
    stage set equals `stage_name` over all `Failure` variants.
* **Behavior:** four functions.
  * `resolve_supervisor(engram_exe: &Path) -> Result<PathBuf, RelayError>`
    * canonicalizes, takes the parent, and joins `engram-indexer{EXE_SUFFIX}`
    * the result must be a regular file
    * there is no PATH, env, or `C:\Tools` lookup
  * `run_relay(supervisor, workspace, timeout_ms, engram_exe) -> RelayOutcome`
    * spawns `preflight --workspace <W> --timeout-ms <T> --engram <E>` with
      stdin null, stdout piped, and stderr inherited
    * The bound is `T + RELAY_GRACE_MS (2000)`, computed with
      `checked_add`. Overflow gives `Failed`/`Build` with cause
      `relay_deadline`, and nothing is spawned.
    * It uses the PRE-unit **bounded child I/O discipline** (attempt-2
      P1 #1): a reader thread with a 4 KiB + 1 cap, a `try_wait` loop, a
      250 ms post-exit drain, and a detached reader that is never joined.
      The supervisor's own children, such as the auto-spawned daemon, may
      inherit the pipe. The relay therefore keys on the supervisor's EXIT,
      never on EOF, and passes the bytes captured by exit + 250 ms drain,
      plus the exit code, to `normalize_verdict`.
    * On expiry, it calls `kill()` then `wait()`, and returns
      `Failed`/`Build` with cause `relay_deadline`.
    * More than 4 KiB returns `Failed`/`Build` with cause `relay_oversize`,
      after kill and reap if the child is still running. A reader I/O error
      returns `Failed`/`Build` with cause `relay_read`. Both still emit the
      structured stderr line, and the oversize case still echoes at most
      4 KiB of the non-conforming bytes to stderr.
  * `normalize_verdict(stdout: &[u8], exit_code: Option<i32>) -> (&'static str, i32)`
    * `None` means the child was killed or ended by a signal.
    * strips exactly one trailing `\n` or `\r\n`
    * accepts exactly one line from the closed set only
    * returns `Succeeded` with 0 only when the line is `Succeeded` AND
      `exit_code == Some(0)`
    * returns the child's well-formed `Failed` line with 1 only when
      `exit_code` is `Some(n)` with `n != 0`
    * everything else returns `Failed`/`Build` with 1, and the non-conforming
      bytes (≤4 KiB) go to stderr
    * the raw child exit code is never relayed
  * `run_preflight(workspace: &Path, timeout_ms: u64) -> i32`
    * uses `current_exe()`, then `resolve_supervisor`, then `run_relay`
    * prints the one normalized line to stdout and ONE structured stderr
      line with the relay's OWN prefix *(harvest-session text fix,
      attempt-4 P2-3; replaces "the same format as NEW-2")*:
      `engram-preflight-relay stage=<Stage> cause=<ok|missing_sibling|relay_spawn|relay_deadline|relay_oversize|relay_read|relay_verdict> elapsed_ms=<n>`.
      For `cause=missing_sibling` it adds the static field
      `sibling=engram-indexer<EXE_SUFFIX>` (a constant name, never a
      path). The supervisor's inherited `engram-preflight ` line stays
      distinguishable. The NEW-4 harness asserts this format.
    * uses no `?` toward `main`: every error becomes `Failed`/`Build` and 1
* **Test scenarios (3):**
  1. Succeeded/exit-0 fixture:
     * the canonical line and 0 are returned
     * the forwarded argv is exact: spaced `--workspace`, `--timeout-ms`,
       and `--engram` absolute, compared with normalized paths
  2. Normalization table (pure `normalize_verdict` calls):
     * `Failed`/`McpProbeVerified` with `Some(23)` → that line, 1
     * `Succeeded` with `Some(1)` → `Failed`/`Build`, 1
     * `Some(0)` with a malformed line → `Failed`/`Build`, 1
     * `Some(0)` with two lines → `Failed`/`Build`, 1
     * `Succeeded\r\n` with `Some(0)` → `Succeeded`, 0
     * `Succeeded` with `None` (killed) → `Failed`/`Build`, 1
     * a crash with no output (`Some(101)`) → `Failed`/`Build`, 1
  3. Process discipline:
     * A missing sibling returns `Failed`/`Build`, 1, and a stderr cause.
     * A hanging fixture is killed within `T + grace`, its PID is not alive
       afterward, and the result is `Failed`/`Build`, 1.
     * The shared **inherited-grandchild fixture**, in supervisor mode,
       spawns its inheriting grandchild, prints `{"state":"Succeeded"}`, and
       exits 0. With `T` = 3000 ms (so `T + grace` = 5 s), the relay returns
       `Succeeded`, 0 within `T + grace + 1 s`. The grandchild PID is
       asserted alive at return (non-vacuity), and it is then killed by its
       recorded PID.
     * A fixture that prints more than 4 KiB returns `Failed`/`Build`, 1,
       with stderr cause `relay_oversize`, not `relay_deadline`.
* **Acceptance:**
  * `cargo test --test contract_cli_preflight_relay` is GREEN, including
    the inherited-grandchild case with its non-vacuity assertion.
  * `src/cli/commands/preflight.rs` contains no `read_to_end`,
    `read_to_string`, `wait_with_output`, `.output()`, or reader `join()`
    on the supervisor's piped stdout (reviewer check).
  * The new target compiles with zero warnings.
  * Root clippy (pedantic) is clean.
* **Depends on:** NEW-1.

### NEW-5: Hidden `engram preflight` subcommand

* **Posture:** test-first.
* **Size:** S. **Complexity:** medium.
* **Production files:** `src/bin/engram.rs`.
  * It adds `#[command(hide = true)] Preflight { #[arg(id = "preflight_workspace", value_name = "WORKSPACE")] workspace: PathBuf, #[arg(long)] timeout_ms: u64 }`.
    The explicit clap `id` keeps the positional argument from sharing the
    global `workspace` id.
  * `workspace` is POSITIONAL, to avoid colliding with the global
    `--workspace`/`ENGRAM_WORKSPACE`. The command ignores the global
    `--workspace`, `--timeout`, `--json`, `--format`, and `--quiet`.
  * The dispatch arm is `std::process::exit(preflight::run_preflight(&workspace, timeout_ms))`.
* **Harness:**
  * `tests/contract/cli_preflight_command_test.rs`
  * `[[test]] name = "contract_cli_preflight_command"`
  * It copies `env!("CARGO_BIN_EXE_engram")` into a spaced temp dir. An
    infrastructure smoke check (`--version` succeeds) runs before any RED
    assertion. Every spawn calls `env_remove` for `ENGRAM_DATA_DIR` and
    `ENGRAM_DIRECT`. Scenario 2 deliberately sets `ENGRAM_WORKSPACE`.
* **Test scenarios (3):**
  1. Missing sibling, run with `--json --quiet` present:
     * stdout is exactly `{"state":"Failed","stage":"Build"}`
     * exit 1
     * stderr names `engram-indexer`
  2. With the fixture sibling returning `Succeeded`, and `ENGRAM_WORKSPACE`
     pointing at a different directory:
     * the forwarded `--workspace` equals the canonical positional path
     * `--engram` equals the canonical copied executable
     * exit 0
  3. `engram preflight --help` shows `<WORKSPACE>` and `--timeout-ms`.
     Top-level `--help` does not list `preflight`. `Cli::command().debug_assert()`
     passes, so there is no clap id collision.
* **Acceptance:**
  * `cargo test --test contract_cli_preflight_command` is GREEN.
  * `unit_cli_parser`, `integration_cli_e2e`, `contract_cli_tool_catalog_parity`,
    and `contract_read_server_cli_mcp_parity` are unaffected. They are
    open-world and descriptor-driven; the F54 CLI map at
    `read_server_cli_mcp_parity_test.rs:~855-890` matches only declared
    descriptors.
  * **Runtime closure:**
    1. `cargo build --workspace`
    2. `target\debug\engram.exe preflight <scratch> --timeout-ms 0`
    3. Expect `{"state":"Failed","stage":"Build"}` and exit 1, with the
       scratch dir unchanged.
* **Depends on:** NEW-4, NEW-3.

### NEW-6: Parity map and compound supersession note

* **Posture:** docs, harness-verification-gated.
* **Size:** XS. **Complexity:** trivial.
* **Files:**
  * `docs/cli-mcp-parity.md`
    * add a table row: `| - | engram preflight <WORKSPACE> --timeout-ms <MS> | local (dev launcher only) | Hidden; relays to the separately distributed engram-indexer supervisor; one JSON verdict line; exit 0 only on Succeeded; requires cargo build --workspace |`
    * add a matching bullet under "CLI commands without MCP tools"
  * `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`
    * append a "Superseded by 142-F" note: guardrail 4 (fail-open to Copilot)
      is replaced by the fail-closed preflight
* **Acceptance:**
  * The row's flags match NEW-5 `--help` exactly.
  * Table column counts are consistent.
  * The supersession note cites this plan and `142.055-T`.
* **Depends on:** NEW-5.

## Dependency Graph

```text
PRE-1 -> PRE-2 -> PRE-3F -> PRE-3 -> PRE-4 ; PRE-2 -> PRE-4b -> PRE-4 ; PRE-4 -> PRE-5 -> PRE-6
Proposed (PA-1): PRE-6 -> 142.054-T (covers PRE-2 transitively) ; PRE-4 -> 142.058-T (F54) ; PRE-4b -> 142.058-T (F54, direct; P1-6)
142.054-T (F50 incl. 002-ST/003-ST) -> NEW-1 -> NEW-2 -> NEW-3
NEW-1 -> NEW-4 -> NEW-5 ; NEW-3 -> NEW-5 ; NEW-5 -> NEW-6
Proposed (PA-1): NEW-5 -> 142.055-T (F51) ; NEW-5 -> 142.057-T (F53)
```

*(Revision 5 graph above, kept as history. Revision 6 below supersedes
it. The arrows run upstream -> downstream.)*

**Revision 6 graph (R6.3, R6.7, R6.9; edges E1-E5 applied at assembly):**

```text
S1: PRE-1 -> PRE-2 -> PRE-3 (a -> b -> c) -> PRE-4 ; PRE-2 -> PRE-4b -> PRE-4 ; PRE-4 -> PRE-5 -> PRE-6
S2: PRE-6 -> 142.054-T (F50 .001 -> .002 -> .003) -> NEW-1 -> NEW-2 -> NEW-3 ; NEW-1 -> NEW-4 -> NEW-5 ; NEW-3 -> NEW-5 ; NEW-5 -> NEW-6
S3: 142.054-T, NEW-5 -> 142.055-T (F51) -> 142.056-T (F52) ; 142.054-T, NEW-5 -> 142.057-T (F53)
    142.054-T, 142.055-T, 142.056-T, 142.057-T -> 142.060-T (archive)      [142.058-T -> 142.060-T REMOVED]
S4: PRE-4 -> PRE-3F (142.063-T) -> [PA-5 tasks] -> 142.058-T (F54 .002, .003) -> 142.059-T (F55) -> 142-F
    PRE-4, PRE-4b -> 142.058-T (existing) ; 142.055-T, 142.057-T -> 142.059-T (existing)
    [PRE-2 -> PRE-3F kept, now implied]
```

**Execution order per shipment (Revision 6):**

* **S1:** `142.061-T → 142.062-T → 142.064.001-ST → 142.064.002-ST →
  142.064.003-ST (closes 142.064-T) → 142.065-T → 142.066-T → 142.067-T →
  142.068-T`
* **S2:** `142.054.001-ST → 142.054.002-ST → 142.054.003-ST (closes
  142.054-T) → 142.069-T → 142.070-T → 142.071-T → 142.072-T → 142.073-T →
  142.074-T`
* **S3:** `142.055-T → 142.056-T → 142.057-T → 142.060-T`
* **S4:** `142.063-T` (with the `7bd9e504` + `6d216d19` cherry-pick, R6.6)
  `→ [PA-5 tasks, placement fixed by the PA-5 session] → 142.058.002-ST →
  142.058.003-ST (closes 142.058-T, M12) → 142.059-T → 142-F`

`142.064.001-ST` has no recorded upstream edge. It runs first in
`142.064-T`, which E5 blocks on `142.062-T`. The S2 order follows the
recorded edges `142.069-T`/`142.071-T` → `142.054-T`.

**Revision 7 graph changes (R7.5, R7.6):**

* **E8.** `142.060-T -> PRE-3F` (`142.063-T` depends on `142.060-T`).
* **E7.** `PRE-3F -> <first PA-5 task>`.
* **E6.** `<last PA-5 task> -> 142.058-T`.

E6 and E7 are applied at the PA-5 harvest. The S4 order is now:

`142.063-T` (HC → red-phase record → `harness-ready` → IC) → `<PA-5
tasks, internal order owned by the PA-5 session>` → `142.058.002-ST` →
`142.058.003-ST` (closes `142.058-T`, M12) → `142.059-T` → `142-F`.

S4 is on hold until E6/E7 exist (R7.5). The assembly edge order is E5, E1,
E2, E3, E4, E8.

**Revision 9 note (R9.2, R9.5, R9.8, R9.10).** No edge changes; E1-E8
and their order stand. The S4 order is unchanged:
`142.063-T` (HC in its own target → red-phase record → `harness-ready` →
IC) → `<PA-5 tasks>` → `142.058-T` (provenance check → FL → three FL runs
→ `.002-ST` → `.003-ST`, M12) → `142.059-T` → `142-F`. There is no F54
cherry-pick in `142.063-T`; `142.058-T` loads the `6d216d19` blob in its
own FL commit. E3 stays because F54 consumes `142.063-T`'s helper. S3 and
S4 are not claimed while PA-6 = Hold S3 stands (R9.8).

**Execution order inside 142-S after PA-1 (superseded by Revision 6;
kept as history):**

`PRE-1 → PRE-2 → PRE-3F → PRE-3 → PRE-4b → PRE-4 → PRE-5 → PRE-6 → 142.054-T → NEW-1 →
NEW-2 → NEW-3 → NEW-4 → NEW-5 → NEW-6 → 142.055-T → 142.056-T → 142.057-T →
142.058-T → 142.059-T → 142.060-T`

**Edge rules:**

* Edges FROM a new unit (NEW-1 on `142.054-T`, and every edge among new
  units) are recorded at harvest.
* Edges that make an ACTIVE task depend on a new unit are listed in PA-1
  and are NOT recorded without approval:
  * `142.054-T` on PRE-6
  * `142.058-T` on PRE-4 AND on PRE-4b. PRE-4 → PRE-4b already implies
    the PRE-4b edge; the direct edge is kept on purpose, as the attempt-2
    P1-6 fix requires (see the revision 4 "Edge ownership" note).
  * `142.055-T` and `142.057-T` on NEW-5
* The attempt-2 P2 dropped the redundant PRE-2→`142.054-T` edge, because
  PRE-6 already covers PRE-2.

**No deadlock:**

* No PRE unit depends on F50, the command, or a launcher task. PRE-3F
  edits `142.058-T`'s test file but has no dependency edge to or from
  `142.058-T`; the PA-1 ownership exception authorizes that edit, not an
  edge (revision 5 note).
* The graph has no cycles.
* `142.060-T` stays the final code task.
* *(Revision 6.)* The first bullet is superseded. PRE-3F now has an edge
  to `142.058-T` (E3: `142.058-T` depends on `142.063-T`) and depends on
  PRE-4 (E2). No PRE unit in S1 depends on anything in S2-S4.
  `142.060-T` is the final code task of S3. In S4, PA-5 code tasks and
  `142.058-T` follow it. After E1-E5 the graph stays acyclic (R6.9).

**F54 is ordered but not fully unblocked.**

* `142.058-T` gains PRE-4 and PRE-4b edges, so the G3 wiring lands first.
* Its equivalence case pins `get_workspace_status`, which stays unpinned
  (no provenance) until stash `86F93068` is planned and admitted.
* F54 GREEN inside 142-S therefore needs a SEPARATE future approval. This
  plan does not claim it.
* *(Revision 6.)* 142-S is to be abandoned. F54 GREEN is now S4's merge
  condition. The operator approved the PA-5 direction (R6.8), but it still
  needs its own planning session.

## Decisions and Rationale

* **D2-A: supervisor plus relay.** This is the only option that satisfies
  all of the following:
  * F51's exact-`engram.exe` spec
  * a single typestate source
  * no package cycle
  * the separate-supervisor boundary
* **Dev-layout only, hidden.** Distribution contracts keep `engram-indexer`
  out of the agent archive and the installer.
* **Positional workspace on the `engram` side, with clap
  `id = "preflight_workspace"`.** This avoids the global `--workspace` id and
  the `ENGRAM_WORKSPACE` env override. The supervisor keeps a strict
  `--workspace`.
* **Relay-normalized verdict, and exit status is authoritative.** The relay
  prints `Succeeded` only when it will exit 0.
* **Bounded relay wait instead of a job object.**
  * The relay kills and reaps its exact child at `T + grace`, computed with
    `checked_add`.
  * Grandchildren that hold the pipe cannot stall it, because it keys on
    the child's exit, not on EOF.
* **D4-A: root-library facades.** Build, seal, publish, and the probes live
  in the root `engram` library, where `sha2`, `chrono`, and `serde_json`
  already exist and root `dev-test` and clippy gate them. The probes are the
  top-level module `src/preflight_probe.rs` (`pub mod preflight_probe;` in
  `src/lib.rs`), and F50 passes them `ProbeRead` descriptors, never JSON
  values. F50 composes them in its owned file.
* **D5-A: narrow allowlisted generation serving (Amendment 2).**
  * PRE-3 installs the store, activator, and gate only when the generations
    root exists, BEFORE readiness is published (revision 5, FASP). It
    re-attempts when the published revision changes, and (revision 5)
    retries a `Transient` failure of the same revision within the
    activation deadline; a `Permanent` failure is never retried. It leaves
    `_health` unchanged.
  * PRE-4b makes the one pinned read serve the opened generation's data.
  * PRE-4 admits only `GENERATION_PINNED_READS`. On a daemon with no gate
    installed, that one method is refused typed (fail closed), and every
    other method is unchanged.
  * Rejected alternatives:
    * D5-B, gating all generation-backed reads, would regress
      `get_daemon_status` and `get_workspace_status` consumers, and would
      mislabel managed data.
    * D5-C, gating `_health`, would regress every no-generation consumer.
    * D5-D, falling back to managed data, would serve unpinned data
      silently.
* **The layout is a single source (PRE-1).** The daemon (PRE-3), the
  publisher (PRE-2), and F50 (through the NEW-3 factory) all call
  `read_server_layout`. The daemon's inline derivation is removed, not
  duplicated.
  * The identity spelling is exactly what `canonicalize_workspace` returns.
    It is never re-canonicalized.
  * The runtime root and the activation deadline come only from the
    layout, and the activator identity only from `expected_identity()`.
  * The only other canonicalization is PRE-2's like-for-like containment
    comparison against `GenerationStore`'s verbatim root. It is never used
    for identity.
* **Minting in `make`, and writes only in Build.**
  * `mint_generation_id` is pure.
  * `open_generation_store(.., true)`, the first filesystem write, happens
    in the Build stage after the deadline is computed.
* **Split into units.** Each unit changes ≤2 production files, has ≤4
  non-trivial functions and 3 scenarios, and stays within one skill domain.

## Constitution Check

| Principle | Assessment |
|---|---|
| I Safety-first Rust | `forbid(unsafe_code)` is kept. There are no `unwrap`/`expect` calls. Errors are module `thiserror` enums, with no `serde_json` types in public probe signatures. **Deviation:** NEW-1/NEW-2 use std-only `UsageError`/`SetupError` instead of `EngramError`. **Justification:** these files must compile under the root-test `#[path]` include and inside `engram-indexer`, and argv/setup errors are not engram domain errors. **Simpler alternative rejected:** using `EngramError` would add `engram::` paths to the supervisor verdict modules. |
| II Test-first | Every code unit has a RED harness authored before build. Every PRE unit plus NEW-1, NEW-2, NEW-4, and NEW-5 runs in `cargo dev-test`. F50's completion requires a real-chain `ProductionVerifier` case, not only mocks. **Deviation:** NEW-3's harness is crate-local. **Justification:** this follows the F12a precedent, and a root bin is forbidden by `supervisor_workspace_boundary_test`. **Simpler alternative rejected:** a root bin target. Follow-up `7BF90213`. |
| III/IV Workspace isolation / CLI containment | Workspaces are canonicalized once, through `canonicalize_workspace` inside `read_server_layout` (`.git` required). A non-git workspace is a typed `WorkspaceError::NotGitRoot` in the library and the daemon, and a `SetupError::Layout` (`cause=setup`) at the supervisor. Every write stays under `<canonical>/.engram/generations`. `open_generation_store` rejects a symlinked or reparse-point `.engram`, root, or `runtime_root` (harvest-session text fix, attempt-4 P3), and a canonical root outside the workspace. That containment check compares two `std::fs::canonicalize` spellings, so the Windows `\\?\` prefix cannot cause either a false escape or a false pass. The active manifest is replaced only by `publish_generation_manifest`. A corrupt manifest fails closed. |
| V Structured observability | There is one structured stderr line per preflight invocation, and stdout carries only the verdict. PRE-3 logs typed activation errors through `tracing`, once per distinct revision or error kind, with no payloads. `ProbeError` never contains environment values or payload bytes. Generation state stays observable through the existing `get_workspace_status` generation block. |
| VI Single responsibility | Layout, build, seal, publish (PRE-1/2), the F54 settle harness (PRE-3F, tests only), daemon wiring (PRE-3), handler data path (PRE-4b), admission routing (PRE-4), probes (PRE-5/6), runner, contract, entrypoint, relay, CLI surface, and docs are separate units. |
| VII Destructive commands | No deletes. PRE-1 renames only inside its own exclusive candidate. The relay and probes kill only the exact child they spawned, which they track by handle. Harness cleanup stops only recorded PIDs. No unit runs a destructive terminal command; any that becomes necessary needs operator approval (VII) and is a HALT to Stage. |
| VIII Safety modes (revision 5, attempt-3 CR-03) | **Careful mode** for PRE-3 (read-server startup, complexity high) and PRE-4 (dispatch): RED-phase consumer baseline, three identical F54 runs, and HALT rules before and after the build. **Freeze-scope mode** for every unit through its listed files and HALT-on-extra-file rule, and for PRE-3F through its diff contract on F54's harness (invariant 6 exception, granted only by PA-1). **Investigate-first mode** for the PRE-3 transient-trigger and non-git-refusal baselines, and the PRE-1 rename HALT: evidence is captured before any change, and an unexpected result returns to Stage. The operator checkpoints are PA-1 (including the PRE-3F exception) and PA-4 in "Risky actions". |
| IX Git-friendly persistence | Generations and runtime copies live under `.engram/`, which is gitignored (only `.version`, `.workspace-id`, `config.toml`, and `registry.yaml` are tracked). Nothing tracked is written. Retention is deferred to `23E287C6`. |
| XI Merge commits | Not affected; it follows Ship policy. |
| Task granularity | Every unit has ≤2 production files (one-line `mod`/visibility edits included), ≤4 non-trivial functions, and 3 scenarios. The new test file plus its `[[test]]` stanza form the unit's own harness (F50 precedent). |
| **Revision 6 (shipment split)** | **II Test-first, deviation:** PRE-3F's positive test lands GREEN (R6.4). **Justification:** the behavior it witnesses is driven RED-first by PRE-3's own harness in S1; PRE-3F is tests-only; a RED-first placement would force known-RED F54 tests into S1 or hold S1 for PA-5. **Simpler alternative rejected:** the 3-shipment variant (launchers wait for PA-5), which the operator did not choose. **Compensating control:** C1 RED witness at `<pre-PRE-3>` (mandatory; HALT if unavailable), plus C2 and C3. **II, all other units:** unchanged. Each S1-S3 unit still writes a RED harness before building, and each shipment merges fully GREEN (R6.2). **VIII Safety modes:** careful mode for PRE-3/PRE-4 keeps the RED-phase consumer baseline and HALT rules in S1. The three identical F54 runs move to `142.063-T` (M1-M3, M10) and `142.058-T` (M12), so careful-mode evidence for F54 is produced where the F54 file exists. Freeze-scope for PRE-3F keeps its diff contract, and its L3-1 guard (R6.6) forbids running the un-barriered F54 file on S4. **XI Merge commits:** C1 needs `<pre-PRE-3>` to stay reachable on `main`. The S1 merge commit keeps it (no squash). **P-001:** one active release at a time. S1-S4 run strictly in order, and 142-S is abandoned before S1 is claimed (a later session). |
| **Revision 7 (supersedes the Revision 6 II deviation)** | **II Test-first: no deviation.** PRE-3F's red phase is the stub-first harness commit HC. In HC the positive test fails at step (b) and every pre-existing F54 case fails at `ensure_daemon`, all with the stub marker. This is recorded before `harness-ready` (P-002/P-004). IC implements the barrier body and the identity fix, and the test bodies are byte-identical between HC and IC (R7.3). **P-016 / branch policy:** one worktree, S4 branch only, no older-commit checkout, no history rewrite. B0 is captured at A0 in the core worktree, which is already on the parked branch (R7.4). **VII:** no destructive step; C1's cleanup is gone with C1. **VIII careful mode:** S1 regains repeat-run evidence (three identical runs of `integration_read_server_generation_wiring` and the consumer targets, R7.8). **XI:** no longer load-bearing for PRE-3F (C1 withdrawn). **IX/operator impact:** R7.9 records PA-6, the S3-merge decision on the fail-closed launcher in managed mode. |
| **Revision 8 (refines Revision 7)** | **II Test-first / P-002 / P-004:** the red phase is still HC. Every test function has exactly one owner and a stated B0, HC and post-IC outcome (R8.3). Only the new positive test belongs to `142.063-T`. Rows 2-5, the pre-existing cases that call `ensure_daemon`, stay with `142.058-T`. Valid RED requires the R8.4 substring. **I Safety-first Rust:** the HC body is pinned so that `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` pass at both HC and IC, with no lint attribute (R8.2). **R7 row correction:** "every pre-existing F54 case fails at `ensure_daemon`" reads "every pre-existing case that calls `ensure_daemon` (R8.3 rows 2-5)". **IX/operator impact:** PA-6 = Hold S3 (R8.6). S3 is not claimed until the operator decides again, and daily `start.ps1` on `main` is unchanged while the hold stands. |
| **Revision 9 (supersedes the Revision 7-8 II/I/VIII/IX text for PRE-3F and F54)** | **II Test-first / P-002 / P-004, no waiver:** `142.063-T`'s red phase is HC in its own target; only its positive test is RED, at (b), with the R9.3 text, recorded before `harness-ready`; IC makes the full `cargo dev-test --no-fail-fast` suite PASS. No S4 task maps a failure onto `142.058-T`; F54 is the GREEN placeholder until FL, and `142.058-T`, every PA-5 task, and `142.063-T` each close with Step 4.3 PASS (R9.2, R9.4). `142.058-T`'s existing red-phase history is revalidated before FL, with HALT, not re-label, on failure (R9.5). **I Safety-first Rust:** the pinned HC stub compiles under `-D warnings` and `-D clippy::pedantic` with no lint attribute (R9.3). **VII:** the new fixture signals only its owned `Child` handle; the T0 tag is never force-pushed or deleted before S4 ships (R9.3, R9.9). **VIII:** freeze-scope for PRE-3F is its three-file diff contract; the invariant-6 cross-task exception is history and is NOT exercised (PA-7). Careful-mode F54 evidence moves to `142.058-T` (R9.5). **P-001 / P-014 / P-017 (not IX):** PA-6 = Hold S3 is a claim gate on S3 and S4; dark mode excludes them; release needs a later PA-6 decision plus P-014 approval (R9.8). **P-019:** the H2 T0 push uses no `--force` or `--no-verify` (R9.9). |

## Risks and Caveats

* **A copied debug `engram.exe` may need native libraries beside it.** The
  NEW-5 smoke check detects this; if it fails, halt to Stage.
* **NEW-3 is not in the full suite.** This is mitigated by the targeted
  harness and the NEW-5 runtime closure. Follow-up `7BF90213`.
* **The F51 `WARNING:` payload lines recur in every interim full-suite run
  until F51 lands.** Apply D1-A from the complete captured output.
* **A cold full index runs on every session start.** The launcher budget
  `T` must cover it. Follow-up `F99C705E`.
* **Unbounded disk growth.** Every run adds a generation and a runtime copy.
  Follow-up `23E287C6`, which depends on the lease gap `9108DB24`.
* **This workspace runs in managed mode today (PA-4).** Until the operator
  opts in, the finished launcher fails closed at `DaemonVerified`, by
  design.
* **F54 stays RED on its equivalence case until `86F93068`.** This is
  explicit, not hidden.
* **`_health` stays decoupled from the gate (attempt-2 P1 #5).** A
  read-server daemon can report `ready` while the pinned read is still
  refused. This is deliberate, so done no-generation consumers
  (`integration_read_server_restart`, `integration_read_server_lifecycle`,
  `integration_doctor_smoke`, `integration_direct_sync_mode`, F54) keep
  working. F50 correctness does not rely on `_health`: its verifier requires
  the pinned read's provenance and data fingerprint, and any refusal or
  missing provenance fails closed at `DaemonVerified`. Alignment is
  deferred to stash `5AF5CD66` (requires deliberation).
* **A rejected initial revision keeps the pinned read refused** until a
  newer revision is published. Preflight publishes a new one on every run,
  so this recovers without a restart.
* **Detached reader threads** in probe and relay processes can outlive the
  child until a grandchild closes the pipe. They are bounded by the process
  lifetime, because every probe or relay process is short-lived, and their
  memory is bounded by the `cap + 1` read limit. Their late messages go to a
  channel whose receiver is already dropped, so they cannot alter a result
  that was already decided.
* **Identity-spelling drift (attempt-2 P1 #3).** `workspace_hash` hashes the
  path's bytes, so any second spelling of the workspace yields a different
  `workspace_id`. The activator would then reject every manifest as an
  identity mismatch. Examples of a second spelling are a `\\?\` form, an
  input-string form, or a linked-worktree form re-normalized differently.
  * Mitigation: there is one derivation (PRE-1). PRE-1 checks it against
    the `std::fs::canonicalize` input spelling, and PRE-3 checks that the
    daemon value still equals `workspace_hash(&canonicalize_workspace(W), branch)`.
  * Residual: the linked-worktree spelling is not fixture-tested in this
    plan. It relies on `canonicalize_workspace`'s existing worktree tests,
    and the layout passes that spelling through unchanged.
* **`ENGRAM_DATA_DIR` does not move generations.** The generations root is
  fixed at `<workspace>/.engram/generations` (parent plan), even when
  `ENGRAM_DATA_DIR` redirects the managed `data_dir`. The harnesses
  `env_remove` it, and an operator who sets it gets split locations by
  design, not by accident.
* **The `runtime` directory name is reserved** under the generations root.
  `build_candidate_generation` rejects that ID, but a generation published
  by another producer under the ID `runtime` would collide with the runtime
  copies. The retention follow-up `23E287C6` should record it as a
  reserved ID.

## Plan Hardening Signals

* **Public API, schema, or contract change: present.** New hidden CLI
  subcommand, a supervisor argv mode, a launcher verdict contract, and new
  public root facades.
* **Security-sensitive behavior: present (low).** Sibling-binary execution,
  inherited secret-bearing environments, and a symlink containment check.
* **Migration, destructive, or irreversible change: absent.**
* **External integration or operator checkpoint: present.** PA-1, PA-2,
  PA-2b, PA-3, and PA-4.
* **High runtime or rollback risk: present.** The command gates every
  session start, and PRE-3/PRE-4 change read-server startup and dispatch.

Requires plan hardening: yes

## Runtime Verification and Closure

| Unit | Runtime surface | Proof before absorption | Closure artifact |
|---|---|---|---|
| PRE-1 / PRE-2 | library + on-disk generation | targeted harness GREEN on spaced git fixtures; the layout identity equals `canonicalize_workspace`/`workspace_hash` for both input spellings; non-git is `NotGitRoot`; `activate_initial` opens the candidate; the function count survives the rename; the Windows containment check passes against the verbatim store root | Ship task record |
| PRE-3F | F54 contract target (tests only) | *(Revision 9, wins over the Revision 7-8 text in this row: the runtime surface is the new target `integration_read_server_activation_settle`, not F54. HC: `cargo check --all-targets`, pedantic clippy, `cargo lint`, `cargo fmt-check` exit 0; only the positive test fails, at (b), with the R9.3 text; F54's placeholder stays GREEN. IC: full `cargo dev-test --no-fail-fast` PASS, three GREEN target runs each with `F54 settle: Active`, oracle completeness, and the F54 file unchanged from the S4 base (R9.3). Closure artifact: the HC red-phase record and the IC record; B0 and the F54 maps move to `142.058-T`.)* *(Revision 8, withdrawn by Revision 9: the HC and IC per-test outcomes follow the R8.3 table and the R8.4 text. `cargo check --all-targets` and pedantic clippy must exit 0 at both HC and IC (R8.2). HC makes no `Cargo.toml` change (R8.1).)* *(Revision 7: supersedes the Revision 6 text for this row.)* Runs first in S4, after PRE-4 and `142.060-T`. HC red-phase record before `harness-ready`: the positive test passes (a) and fails (b) with the stub marker, and every pre-existing case that calls `ensure_daemon` fails there with the stub marker. After IC: the positive test is GREEN, and three identical full-output runs each equal B0 (A0, same OS) modulo B1 (i)-(iv), with every `F54 settle:` line `Active`. `git diff HC IC` for the F54 file shows only the identity lines, their imports, and the barrier body. | Ship task record with the A0 B0, the HC red-phase record (SHA, command, output, OS), and the three IC maps |
| PRE-3 | real `engram daemon` (read-server) | `active_revision` observed; generation non-null at the first `ready`; the daemon's `workspace_id` equals its pre-change value; the non-git refusal is unchanged; the diff shows the inline derivation removed; no-root and empty-root daemons unchanged with `_health` `ready` throughout; the RED-phase consumer baseline (non-F54 targets) and pinned-read sender grep recorded; every named consumer target matches that baseline. *(Revision 6: the F54 positive test and the three F54 maps moved to `142.063-T`, M1-M6.)* | Ship task record with captured `_health`, generation observability, and consumer baseline map |
| PRE-4b | in-process dispatch | the pinned read returns generation data that differs from managed data; PRE-3 consumer targets match baseline *(Revision 6: F54 comparison moved to `142.063-T`, M7-M8)* | Ship task record |
| PRE-4 | real CLI → daemon | `stats` provenance and `code_files` match the published fixture; on no-root daemons `stats` is refused typed and every other method is unchanged; managed daemons unchanged; PRE-3 consumer targets match baseline; reconciliation quiescence using the inline two-observation rule (R6.5) *(Revision 6: F54 rows and three runs moved to `142.063-T`, M9-M11)* | Ship task record with captured JSON |
| PRE-5 / PRE-6 | real CLI and stdio shim | probe harness GREEN, including the kill/reap case and the inherited-grandchild case (grandchild alive at return); CLI and MCP fingerprints are equal | Ship task record |
| `142.054-T` (F50) | `ProductionVerifier` | real-chain case `Succeeded` on a read-server fixture; managed fixture fails at `DaemonVerified`; indexer clippy clean | Ship task record |
| NEW-1 / NEW-2 | library | targeted harness GREEN; indexer build and clippy | Ship task record |
| NEW-3 | `engram-indexer preflight` | crate-local harness GREEN; a zero-deadline run leaves the scratch dir unchanged | Ship task record with captured stdout and exit code |
| NEW-4 | library | relay harness GREEN, including the hang, oversize, and inherited-grandchild cases (grandchild alive at return) | Ship task record |
| NEW-5 | `engram preflight` | command harness GREEN; `cargo build --workspace` real-chain zero-deadline probe | Ship task record |
| NEW-6 | docs | row matches `--help`; supersession note present | Ship docs verification record |
| `142.058-T` (F54, S4; Revision 6) | real daemon, CLI and stdio MCP through the F54 matrix | after the PA-5 tasks: every F54 case GREEN; three identical consecutive runs; every `F54 settle:` line `Active` (M12). *(Revision 9: first the R9.5 item 1 provenance revalidation (HALT, no re-label), then the FL commit (real body + barrier call + identity fix in one commit), three FL runs vs. B0 under B2 with the R9.9 drift ledger, `.002-ST`'s real-snapshot fixture change, M12, and Step 4.3 PASS on the full suite. Owner of M1 (F54 part), M2-M3, M5-M7, M9-M11, M13-M15 (F54 part), R9.5.)* | Ship task record with the provenance record, B0 path, FL record, the three FL maps with B2 comparison, and the three M12 maps |

## Plan Hardening

**Hardening required: yes.** The triggers are the new CLI and launcher
contract, sibling-binary execution, and the session-start gate.

**Learnings and instructions consulted:**

* `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`.
  Guardrail 4 (fail-open) is explicitly superseded, and NEW-6 annotates it.
  One shared budget and exact-child cleanup are retained.
* `docs/compound/capability-rewrite-must-convert-every-consumer-2026-08-21.md`.
  There is no fallback path, and preflight replaces the legacy pre-warm
  budget rather than adding to it.
* `docs/compound/2026-08-22-cargo-dev-test-alias-must-stay-native.md`.
  Root-only `dev-test` coverage, oracle prefix globs.
* `docs/compound/workflow-issues/claimed-lint-gate-defeated-by-crate-level-allow-2026-09-15.md`.
  Hence `clippy -p engram-indexer`.
* `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`.
  Hence `env_remove`.
* `docs/compound/best-practices/clap-long-vs-name-attribute-2026-05-07.md`
* `docs/compound/test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md`
* `docs/compound/test-failures/truncated-test-output-review-hid-call-site-regression-2026-09-12.md`
* Policies P-021 (C1–C6), P-001/P-016, and P-002, plus Ship Step 4.3 and the
  build-feature quality gates.

**Protected invariants:**

1. Copilot never launches unless preflight exits 0.
2. There is one deadline source, `--timeout-ms`. The relay's bound is derived
   from it (`T + grace`). The launcher's outer kill is later than
   `T + grace + margin`.
3. No PATH, `C:\Tools`, cwd, or env-override resolution occurs in the
   relay→supervisor→verifier chain or in `start.ps1`. F53's executable
   resolution follows its own pending harness.
4. Stdout carries exactly one normalized verdict line. Diagnostics go to
   stderr as structured lines with no env values, paths, or payloads,
   restated per layer *(harvest-session text fix, attempt-4 P2-3)*:
   * **Probes (PRE-5/PRE-6):** every probe child is spawned with
     `stderr(Stdio::null())`, so the CLI, the shim, and the daemon they
     auto-spawn add nothing to the caller's stderr.
   * **Supervisor (NEW-2/NEW-3):** exactly one `engram-preflight ` line.
   * **Relay (NEW-4/NEW-5):** exactly one `engram-preflight-relay ` line,
     plus the supervisor's inherited line when it ran, plus at most the
     ≤4 KiB echo of non-conforming supervisor bytes.
5. The legacy env-driven `engram-indexer` contract is unchanged.
6. No unit edits another task's pending-RED harness. The F51, F52, and F53
   fixtures are untouched. The ONE exception (revision 5) is PRE-3F, which
   edits F54's `tests/contract/read_server_cli_mcp_parity_test.rs` only
   under the PA-1 grant and only within the PRE-3F diff contract.
7. PRE-3, PRE-4b, and PRE-4 leave these unchanged:
   * managed-mode daemon startup and dispatch
   * read-server `_health` semantics (hydration-ready set at startup,
     never gated on activation)
   * every read-server response for a method outside
     `GENERATION_PINNED_READS`, with or without a generations root, EXCEPT
     the `generation` block of `get_workspace_status`, which becomes non-null once PRE-3 installs a
     gate (root present at startup, or a root absent at startup that
     appears later). That block is the intended observability surface,
     not a regression *(harvest-session text fix, attempt-4 P2-5)*

   A method in `GENERATION_PINNED_READS` is never served without an admitted,
   opened generation context. It never falls back to managed data. With no
   gate installed (including no generations root), it is refused with the
   typed `GenerationNotYetActivated` error; this is the ONE intended
   read-server behavior change, and the PRE-3 consumer baseline shows no done
   no-generation consumer sends it.
8. No generation-control, reload, or notify endpoint is added or called
   (R48). Activation happens only at startup (F18) and at request entry
   (F20).
9. The read-server identity, the generations root, the runtime root, and
   the activation deadline are derived in exactly one place, PRE-1
   `read_server_layout`.
   * The daemon's former inline derivation (`validate_workspace_path`,
     `canonicalize_workspace`, `resolve_git_branch`, `workspace_hash`) is
     REMOVED from `run_read_server_startup` and replaced by that one call.
   * Its single `layout` value feeds both the workspace snapshot and the
     activator's `expected_identity()`.
   * The identity spelling is `canonicalize_workspace`'s output, unchanged.
     No caller re-canonicalizes or re-hashes the workspace path for
     identity.
   * A non-git workspace is refused with a typed error before any write.
10. No stdout read in the relay or the probes waits for EOF, and no reader
    thread is joined. Completion is keyed on the exact child's exit
    (`try_wait`), within the deadline, followed by at most a 250 ms drain of
    a reader capped at `cap + 1` bytes. The oversize, deadline, read-error,
    exit, and parse outcomes keep distinct typed results. A real
    inherited-grandchild fixture with a non-vacuity (grandchild still alive)
    assertion proves this in each of PRE-5, PRE-6, and NEW-4.
11. No `serde_json` type appears in a public probe or facade signature that
    F50 consumes. Probe reads are selected only through the public
    `ProbeRead` descriptors (`PREFLIGHT_READ`, `HEALTH_READ`) in
    `engram::preflight_probe`.

**Risky actions:**

| ProposedAction | ActionRisk | Approval |
|---|---|---|
| PA-1: add PRE-1, PRE-2, PRE-3F, PRE-3, PRE-4b, PRE-4, PRE-5, PRE-6, and NEW-1..NEW-6 to active 142-S, in that order after the covering feature. Add edges on active tasks: `142.054-T` on PRE-6; `142.058-T` on PRE-4 and on PRE-4b; `142.055-T` and `142.057-T` on NEW-5. Grant the invariant-6 ownership exception that lets PRE-3F edit `142.058-T`'s pending-RED harness `tests/contract/read_server_cli_mcp_parity_test.rs`, only within the PRE-3F diff contract (revision 5, FASP). This confirms deliberation D2-A, D4-A, and D5-A. D5 (deliberation Amendment 2) is the G3 lineage (production read-server startup/admission wiring plus the pinned data path; stash `9B7EC1E4` → PRE-3F and PRE-3, stash `6C5DF765` → PRE-4b and PRE-4), so granting PA-1 also admits the G3 scope expansion under P-021 C6. The same phrase grants PA-2, PA-2b, and PA-3. | contract (exception to active-manifest immutability, and to invariant 6 for PRE-3F) | **Operator**, using the exact PA-1 approval phrase below, with IDs filled in at harvest. |
| PA-2b: append the "F50 completion after PRE-6" guidance to the implementation notes of `142.054.002-ST` and `142.054.003-ST`, and ADD two acceptance criteria to `142.054-T`: (a) the real-chain `ProductionVerifier` case; (b) `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic` clean | moderate (the acceptance criteria of an active task are tightened) | Operator, bundled with PA-1 |
| PA-4: opt this workspace into `mode = "read_server"` (`.engram/config.toml`) before relying on the F51 launcher | operational (it changes how the daemon serves this repo) | Operator decision. Stage creates no task; Ship or the operator makes the one-line config change only when it is approved. |
| PA-2: append the launcher contract (below) to the implementation notes of `142.055-T` and `142.057-T` | low (guidance for active tasks) | Operator, bundled with PA-1 |
| PA-3: ratify D1-A: "Text inside the panic payload of a mapped pending-RED test is part of that mapped failure, not a Step 4.3 warning." | policy interpretation | Operator, **bundled with PA-1**: it is granted only by the single PA-1 approval phrase, never separately, and never inferred. This keeps it from being re-litigated at every interim gate. Until PA-1 is granted, PA-3 is NOT ratified. Ship still keeps its existing sole authority to accept `EXPECTED_PENDING_RED` under Step 4.3 (deliberation D1). That is Ship's own classification authority under the existing rule, not a PA-3 pre-approval. |
| PA-5 (information, not requested here): F54 (`142.058-T`) GREEN also needs stash `86F93068` to be deliberated, planned, and admitted | scope | A separate future operator approval. Stage does not request it in this session. |
| The relay kills its own child when the child exceeds its bound | low | none (the child is tracked by handle) |

**The PA-1 approval phrase (template; covers PA-1, PA-2, PA-2b, and PA-3):**

> I authorize Stage to add `<PRE-1>`, `<PRE-2>`, `<PRE-3F>`, `<PRE-3>`,
> `<PRE-4b>`, `<PRE-4>`, `<PRE-5>`, `<PRE-6>`, and `<NEW-1>` through
> `<NEW-6>` to active shipment 142-S, in that order after 142-F; to make
> `142.054-T` depend on `<PRE-6>`, `142.058-T` depend on `<PRE-4>` and
> `<PRE-4b>`, and `142.055-T` and `142.057-T` depend on `<NEW-5>`; I grant
> the invariant-6 exception letting `<PRE-3F>` edit `142.058-T`'s harness
> `tests/contract/read_server_cli_mcp_parity_test.rs` only within the
> PRE-3F diff contract; I confirm D2-A, D4-A, and D5-A
> (admitting the G3 scope expansion under P-021 C6); and I grant the bundled
> PA-2, PA-2b, and PA-3 (ratifying D1-A).

* Each `<…>` placeholder is the backlog ID that harvest assigns to that
  unit. The phrase is valid only once every placeholder is filled with a
  harvested ID.
* Stage presents the phrase only after a plan review returns PASS or
  ADVISORY and harvest has run under 142-F. Harvested units stay OUTSIDE
  142-S until the operator grants the phrase.
* A partial, paraphrased, or unfilled phrase grants nothing.
* This template supersedes the draft phrase in
  `docs/memory/2026-09-26-stage-142-f-plan-review-attempt2-checkpoint.md`.
  That draft named a PRE-2 edge on `142.054-T`, which was dropped as a P2,
  and omitted the `142.058-T` → PRE-4 edge.

**The PA-2 launcher contract text:**

1. Invoke `<engram> preflight <WORKSPACE> --timeout-ms <T>`.
   * F51: `<engram>` is exactly `<workspace>\target\debug\engram.exe`.
   * F53: resolve it as its harness specifies.
2. Preflight REPLACES the legacy `sync`/`bind` pre-warm, and there is one
   budget. Set `T` to the outer budget minus (2000 ms relay grace + ≥1000 ms
   margin).
3. PowerShell passes arguments through `ProcessStartInfo.ArgumentList`. Never
   use a joined string, and never use `Start-Process -ArgumentList`. Each
   element is one argument, so paths containing spaces or ending in a
   backslash are preserved.
4. The exit status is authoritative. Launch Copilot only when the exit is 0
   AND no stdout verdict line reports a state other than `Succeeded`. Every
   exception, timeout, missing executable, or non-zero exit blocks Copilot
   and prints stdout and stderr. None of these falls back to a warning.
5. `start.sh` quotes `"$WORKSPACE"`, runs the command outside any pipeline,
   and reads `$?` directly.

**Rollback:**

* Each unit is an isolated commit, and each can be reverted with
  `git revert`.
* PRE-3 is inert on a daemon with no generations root, and PRE-4 changes
  only the pinned read there (typed refusal). Deleting `.engram/generations`
  before daemon start restores today's behavior for every other method.
  Restoring the pre-change `stats` response requires `git revert` of PRE-4.
* No schema is changed.

**Monitoring:**

* The typed-stage line from the launcher.
* The structured stderr `cause=` field. A `Build` failure with
  `cause=missing_sibling` in dev means the build ran without `--workspace`.

**Owner and validation window:** Ship owns execution on the 142-S branch. The
window closes at the final 142-S readiness run, which must be an unequivocal
PASS.

**Unresolved operator decisions:**

* PA-1 must be granted before PRE-1 starts. The PRE units block F50, and
  F50 cannot close without them.
* PA-2, PA-2b, and PA-3 are bundled with PA-1 and are granted only by the
  PA-1 approval phrase. None of PA-1, PA-2, PA-2b, or PA-3 is granted yet,
  and none may be inferred from this plan, a review verdict, or a harvest.
* PA-4 is needed only before the launcher is used for real.
* F50's in-scope completion (D3) is blocked on PRE-6, which covers PRE-2
  transitively. Ship keeps F50 `active`, with no status change, until then.
* F54 GREEN also needs `86F93068` (PA-5), which is a separate future
  decision.

## Harvest Record (2026-09-26, operator "Option A")

**Authorization.** After plan-review attempt 4 returned ADVISORY (0 P0,
0 P1, 9 P2, 11 P3), the operator chose "Option A": Stage harvests the 14
units under 142-F, OUTSIDE 142-S, after adding deliberation Amendment 3,
reconciling stash `EFE9190A` and `4628001C`, and writing the P2 fixes into
task acceptance. No further plan-review attempt was run. Shipment 142-S,
tasks `142.054-T`..`142.060-T` and their subtasks, PA-1..PA-5,
`.engram/config.toml`, source, tests, and config were NOT touched.

**Harvested IDs** (all `queued`, parent `142-F`, labels `preflight`,
`pending-pa-1`; size/complexity recorded as prose):

| Unit | ID | Recorded `blocks` edges (depends on) | Deferred edges (NOT recorded) |
|---|---|---|---|
| PRE-1 | `142.061-T` | none | none |
| PRE-2 | `142.062-T` | `142.061-T` | none |
| PRE-3F | `142.063-T` | `142.062-T` | none (PA-1 grants the invariant-6 edit exception, not an edge) |
| PRE-3 | `142.064-T` | `142.063-T` | none |
| PRE-3a | `142.064.001-ST` | none (first subtask) | none |
| PRE-3b | `142.064.002-ST` | `142.064.001-ST` | none |
| PRE-3c | `142.064.003-ST` | `142.064.002-ST` | none |
| PRE-4b | `142.065-T` | `142.062-T` | none |
| PRE-4 | `142.066-T` | `142.064-T`, `142.065-T` | none |
| PRE-5 | `142.067-T` | `142.066-T` | none |
| PRE-6 | `142.068-T` | `142.067-T` | none |
| NEW-1 | `142.069-T` | none | `142.069-T` → `142.054-T` |
| NEW-2 | `142.070-T` | `142.069-T` | none |
| NEW-3 | `142.071-T` | `142.070-T` | `142.071-T` → `142.054-T` |
| NEW-4 | `142.072-T` | `142.069-T` | none |
| NEW-5 | `142.073-T` | `142.072-T`, `142.071-T` | none |
| NEW-6 | `142.074-T` | `142.073-T` | none |

* 16 edges were recorded: 14 among the tasks, matching the Dependency
  Graph, and 2 among the PRE-3 subtasks.
* **Deferred edges.** The Edge rules say the edges from NEW-1 and NEW-3
  to `142.054-T` are recorded at harvest. The operator's harvest bounds
  forbid any new edge touching an active task, so they were NOT
  recorded. Both tasks carry the dependency in prose. They need an
  explicit operator grant, which the PA-1 template does not cover (see
  the supplemental phrase below).
* The PA-1 edges on active tasks were not recorded either:
  `142.054-T` → PRE-6, `142.058-T` → PRE-4 / PRE-4b, and
  `142.055-T` / `142.057-T` → NEW-5.

**PRE-3 split (attempt-4 P2-6).** PRE-3 touches five functions:
`run_read_server_startup`, `install_generation_gate`,
`drive_generation_activation`, `set_read_server_gate`, and
`read_server_gate`. Its complexity is high. That breaks the <5-function
task bound and forces de-risking. The task keeps the harness and the
gating acceptance. It is split into three sequential subtasks on the
same two files:

* a: identity REPLACE plus the gate slot;
* b: install-before-ready plus the driver;
* c: Transient retry plus the F54 determinism evidence.

Under PA-1, each subtask is added to 142-S right after `142.064-T`.

**P2 dispositions (the task acceptance overrides the older plan text
where the two differ):**

| P2 | Disposition |
|---|---|
| P2-1 `NoActivator` not terminal | `142.064-T` AC and `142.064.002-ST`. If the root exists at startup and the install fails, the failure is final for that daemon, with no late install. This supersedes PRE-3's "leaves the background ticks below to retry" for that case. The three-run F54 evidence requires every barrier return to be `Active`. `142.063-T` AC makes the barrier print its return variant. |
| P2-2 barrier self-interference | `142.063-T` AC. No IPC call is made between two compared filesystem snapshots, and the fingerprints are compared separately. The same rule applies to positive step (d). HEAD's snapshot→status→snapshot stability is recorded as evidence. |
| P2-3 CR-04 relay half | Plan text (NEW-4 `run_preflight`, invariant 4, restated per layer). `142.072-T` AC asserts the `engram-preflight-relay` format. `142.070-T` AC keeps the supervisor prefix distinct. |
| P2-4 `HydrationError::Failed` | Plan text (the Revision 5 table and PRE-1). `142.061-T` AC notes it. |
| P2-5 invariant 7 | Plan text (invariant 7 exempts the `get_workspace_status.generation` block). `142.064-T` AC. |
| P2-6 PRE-3 sizing | Plan text (PRE-3 production files list `install_generation_gate`), plus the split above. |
| P2-7 deliberation drift | Deliberation Amendment 3 (A3.1 D5-A text, A3.2 lineage including PRE-3F). |
| P2-8 coarse failure signature | `142.063-T` AC defines the per-row signature. `142.064-T` AC lists the expected changes. `142.065-T` and `142.066-T` apply the same format. |
| P2-9 SQLite sidecar mtimes | `142.066-T` AC. One warm-up read happens before the "before" snapshot. Any change that remains is characterized and is a HALT. Paths are never excluded. |

**P3 dispositions:**

* `within_root` `map_err`: `142.062-T`.
* Barrier at the ready-return point, and IPC error means re-poll:
  `142.063-T`.
* Learnings citations: `142.063-T` references.
* Dual retry policy, the gate's `error!` repeat, and the note that
  publication-last holds only within the deadline: `142.064-T` notes.
* F54 wall-clock time and full output: `142.064-T` and `142.066-T`.
* PRE-4b baseline without a PRE-3 edge: `142.065-T` AC.
* Constitution III/IV `runtime_root`: plan text.
* Not changed:
  * The PRE-3F dependency stays on PRE-2, as in the graph. The note
    in `142.063-T` says the real need is PRE-1.
  * The positive step (c) runtime internals stay, with a note in
    `142.063-T`.
  * The CR-03 "new safety rows" wording stays as history.

**Stash (P-021 C5/C6; deliberation Amendment 3):**

* `EFE9190A`:
  * PR #393 was reconciled in place. Review-thread `N/A` stands.
  * PRE-4 (`142.066-T`) consumes its dispatch-plumbing part. The
    residual per-handler part is forwarded to `86F93068`.
  * It was archived at Step 5.6 with both forward refs (the archive
    and the forward-ref text were completed by the follow-up Stage
    session on 2026-09-26).
* `4628001C`:
  * PR #407 was reconciled in place. Review-thread `N/A` stands.
  * It is carried to the `86F93068` / PA-5 deliberation.
  * It was NOT archived.

**Shipment.** None was assembled or changed. The 142-S manifest is
unchanged, and the harvested items belong to no shipment.

**Filled PA-1 approval phrase** (valid only verbatim; NOT granted):

> I authorize Stage to add `142.061-T`, `142.062-T`, `142.063-T`, `142.064-T`,
> `142.064.001-ST`, `142.064.002-ST`, `142.064.003-ST`, `142.065-T`,
> `142.066-T`, `142.067-T`, `142.068-T`, and `142.069-T` through
> `142.074-T` to active shipment 142-S, in that order, placing
> `142.061-T` through `142.068-T` (including the three `142.064-T`
> subtasks) before `142.054-T`, and `142.069-T` through `142.074-T` after
> `142.054.003-ST` and before `142.055-T`; to make
> `142.054-T` depend on `142.068-T`, `142.058-T` depend on `142.066-T` and
> `142.065-T`, and `142.055-T` and `142.057-T` depend on `142.073-T`; I grant
> the invariant-6 exception letting `142.063-T` edit `142.058-T`'s harness
> `tests/contract/read_server_cli_mcp_parity_test.rs` only within the
> PRE-3F diff contract; I confirm D2-A, D4-A, and D5-A
> (admitting the G3 scope expansion under P-021 C6); and I grant the bundled
> PA-2, PA-2b, and PA-3 (ratifying D1-A).

* Correction (follow-up Stage session, 2026-09-26): the first fill
  omitted the three PRE-3 subtasks and kept the template's "in that
  order after 142-F". 142-F is not a 142-S manifest item (the manifest
  starts at `142.054-T`), and "after 142-F" would put NEW-1 ahead of
  `142.054-T`, which NEW-1 depends on. The placement now follows the
  "Execution order inside 142-S after PA-1" above. The PA-1 template's
  covered grants are otherwise unchanged. The phrase is still NOT
  granted.

**Supplemental phrase for the deferred NEW-1/NEW-3 edges.** This phrase
is separate from PA-1, which does not grant these edges:

> I authorize Stage to make `142.069-T` and `142.071-T` depend on `142.054-T`.

**Operator decisions (2026-09-27).** Recorded in the deliberation's
"Operator Approval (2026-09-27)" section.

* PA-1 is granted in MODIFIED form: every grant EXCEPT "add the
  harvested units to active 142-S". Instead, the operator asked for the
  remaining launcher-preflight scope to be split into at least 3
  shipments of about 8 tasks each.
* Stage recorded the 5 PA-1 active-task edges and the 2 supplemental
  edges (7 `blocks` edges, cycle-checked). It applied PA-2 to
  `142.055-T` and `142.057-T`, and PA-2b to `142.054-T`,
  `142.054.002-ST` and `142.054.003-ST`.
* PA-3 (D1-A) is ratified. D2-A, D4-A and D5-A are confirmed.
* PA-4 is on hold. PA-5 goes to a separate, later Stage session.
* The shipment split, and the plan amendment it needs, are PROPOSED in
  `docs/memory/2026-09-27-stage-142-f-shipment-split.md` and are NOT yet
  approved.
* The "Execution order inside 142-S after PA-1" above no longer
  describes a single-shipment order. Treat it as the dependency order
  only.

## Plan Review

### Attempt 1: FAIL

Six personas reviewed revision 1: Rust, Scope Boundary, Architecture,
Constitution, Learnings, and Security Lens. They produced 10 distinct P1s
after dedup, plus P2 and P3 findings. The P1s are listed below with where
revision 2 addresses each.

1. Dead-code under `-Dwarnings` in the `#[path]` harness. Addressed by the
   scoped `#[allow(dead_code)]` and the zero-warning acceptance criterion.
2. Clap collision with the global `--workspace`/`ENGRAM_WORKSPACE`, found by
   three personas. Addressed by the positional workspace in NEW-5, the global
   flags being ignored, and `debug_assert`.
3. A hung or orphaned supervisor under exact-child cleanup, found by three
   personas. Addressed by the relay's `T+grace` kill-and-reap and the
   launcher budget rule.
4. The relay's stdout exactness and contradictions. Addressed by
   `normalize_verdict`.
5. Gaps in the relay test matrix. Addressed by the NEW-4 table.
6. NEW-2 v1 was untested code, violating P-002 and Principle II. Addressed by
   moving the logic into the new NEW-2 (`main_with`) and giving NEW-3 a
   crate-local RED harness.
7. The agent archive and installer exclude the supervisor. Addressed by
   making the command dev-layout-only and hidden.
8. Constitution Check missing. Added.
9. The 118-S fail-open guardrail was not superseded. Addressed by NEW-6 and
   the learnings list.
10. Root clippy does not lint the indexer. Addressed by
    `clippy -p engram-indexer`.

The Learnings P1 asked for PA-3 to be bundled with PA-1. PA-3 is now
recommended with PA-1. *(Later note: attempt 2 found this insufficient
(attempt-2 P1-7). Revision 4 bundles PA-3 into the PA-1 approval phrase.
This attempt-1 text is kept as history.)*

P2s and P3s absorbed into revision 2:

* `checked_add` for the deadline
* sync `main` before any runtime
* setup failures map to `Failed`/`Build`, with no disk mutation before the
  deadline
* `--engram` required and absolute
* strict parser
* canonicalized paths and a regular-file check
* `env_remove`
* the new relay module
* explicit `stage_name`
* a structured stderr line
* a secret-free diagnostics rule
* a cross-platform harness
* the NEW-3→NEW-5 edge
* the parity "without MCP" bullet
* the PA-2 quoting and pipefail rules

<!-- plan-review-attempt: 1 -->

### Attempt 2: FAIL

Five personas reviewed revision 3: Rust, Scope Boundary, Architecture,
Constitution, and Learnings. After dedup there are 7 P1s. Revision 4
addresses each one (see the "Revision 4" notes in each unit).

1. **Rust P1-1: pipe EOF hang.** On Windows, an inherited grandchild (the
   auto-spawned daemon) keeps the stdout pipe open. EOF never arrives, so
   the NEW-4, PRE-5, and PRE-6 read loops hang past their deadline. Fix:
   key on child exit (a `try_wait` loop), use a reader thread capped at
   cap+1 bytes, drain for 250 ms after exit, never join or read to EOF, and
   add a grandchild fixture.
2. **Rust P1-2: `serde_json::Value` in a public signature.** `probe_mcp_read`
   exposes `Value` to F50, whose crate has no `serde_json`. Fix: add a
   `ProbeRead { cli_args, mcp_tool, mcp_arguments_json }` descriptor and a
   `pub const PREFLIGHT_READ`. Architecture also asked for the module to
   move to top-level `src/preflight_probe.rs` plus one line in `src/lib.rs`.
3. **Rust, Architecture, and Learnings P1-3: wrong identity and layout.**
   `canonicalize_workspace` requires `.git`, so the non-git fixture is wrong.
   `GenerationActivator::new` also needs an absolute `runtime_root` and a
   deadline. Fix: `read_server_layout` uses `canonicalize_workspace` with a
   git fixture, and a non-git workspace is a typed error. Add `runtime_root`
   and an activation-deadline const. PRE-3 REPLACES the startup derivation
   with the layout instead of duplicating it.
4. **Scope P1: G3 has no lineage.** The G3 wiring has no stash or
   deliberation lineage. Fix: a new stash entry with the C2 fields, and
   deliberation Amendment 2 / D5. PA-1 must name D5.
5. **Scope and Learnings P1: PRE-3 regresses consumers.** Done tests run
   read-server with no generation (`read_server_restart_test`,
   `read_server_lifecycle_test`, `doctor_smoke_test`, `direct_sync_mode_test`,
   and F54). Gating `_health` readiness on activation breaks them. Fix: keep
   `_health` hydration semantics UNCHANGED, and gate only generation-backed
   reads through `admit_read`. Background initial activation runs after
   bind, with bounded retry/backoff while nothing is published. The
   consumers are named as regression targets, with a HALT if any goes RED.
   A follow-up stash is recorded for aligning `_health` with the gate.
6. **Architecture P1: PRE-4 only labels provenance.** The data still comes
   from managed state. Fix: add PRE-4b (owns `src/tools/mod.rs` and
   `src/tools/read.rs`). The pinned read receives the admitted context, and
   its queries run on the opened generation's DB. Assert that the data field
   differs from managed data, and have the probes compare the data field.
   Record a stash for the remaining handlers (F54 lineage). Add an edge from
   `142.058-T` to PRE-4b.
7. **Learnings P1: PA-3 is not bundled.** PA-3 is only "recommended" with
   PA-1. Fix: mark it "bundled with PA-1".

**P2 summary (absorbed in revision 4 where cheap):**

* Mint the ID in `production_factory`, not in Build.
* Scoped `dead_code` allow in NEW-4.
* F50 clippy acceptance in PA-2b.
* Rename safety: WAL/SHM sidecars, retry on error 32, and a symbol query.
* A symlinked `.engram` containment check.
* `open_generation_store(layout, create)`.
* NEW-5 clap `id = "preflight_workspace"`.
* `checked_add` for `T + grace`.
* Probe `env_remove` of `CARGO_BIN_EXE_engram`.
* A retention follow-up stash.
* `normalize_verdict(&[u8], Option<i32>)`.
* Drop the redundant PRE-2→054 edge.
* Constitution rows V, VIII, and IX for the PRE units.
* Bounded probe backoff.

<!-- plan-review-attempt: 2 -->

### Attempt 3: FAIL

Five personas reviewed revision 4: Rust, Scope Boundary, Architecture,
Constitution, and Learnings. Rust, Scope, Architecture, and Constitution
returned ADVISORY. Learnings returned FAIL with one new P1. **The merged gate
is FAIL**, because L3-1 is an open P1. This was the final authorized re-entry
(2 re-entry cycles used), so the circuit is open. No task was harvested, no
shipment was assembled, and the 142-S manifest and its tasks were not touched.

**P1 (open):**

1. **Learnings L3-1: PRE-3 background activation can break F54.** F54's
   fixture publishes a valid generation. PRE-3's background initial
   activation (after bind, with bounded retry/backoff) would therefore
   activate F54's generation. That can flip F54's passing test
   `unknown_ipc_methods_are_refused_without_side_effects`, because the test
   binds fingerprint and filesystem side effects. The effect depends on
   timing, so it would show up as a flaky RED rather than a deterministic
   one. Revision 4 names F54 as a regression target but has no mechanism
   that keeps activation from changing its side-effect assertions.

**P2 summary (non-blocking, carried to the next revision):**

* **Rust:**
  * On Windows, the `starts_with` containment check compares a verbatim
    `\\?\` path with a stripped path.
  * `WorkspaceError::Failed` does not exist; use `NotFound`.
  * `core_read_generation_pin_test` calls the 2-arg
    `get_workspace_statistics`; keep a wrapper plus `_with_context`.
  * Transient retry.
  * Set the probe's stderr to null.
  * A refusal returns `accepted:false`, then retries.
* **Scope:**
  * S-1: the "no root = unchanged" wording contradicts the `stats` refusal.
  * S-2: the duplicate scan missed `EFE9190A` and `4628001C`.
* **Architecture:**
  * A3-1: transient retry (same as the Rust point).
* **Constitution:**
  * CR-01: the symlink containment check omits `runtime_root`.
  * CR-02: the Windows prefix check (same as the Rust `\\?\` point).
  * CR-03: row VIII safety modes.
  * CR-04: the stderr contract.

**Disposition:** HALT. The P-013.6 escalation was compiled and is recorded in
`docs/memory/2026-09-26-stage-142-f-attempt3-fail-escalation.md`. Any further
review attempt, revision, or harvest needs explicit new operator
authorization.

<!-- plan-review-attempt: 3 -->

### Attempt 4: ADVISORY

**Authorization.** The operator directed "Resume work based on"
`docs/scratch/2026-09-26-142-s-restart-handoff.md`. Orchestrator relayed
that as route 1 only: verify revision 5, make minimal consistency fixes,
and run exactly ONE review attempt 4. Nothing was granted for harvest,
backlog or stash changes, the 142-S manifest or its edges, code, tests,
config, commits, or PA-1..PA-5. Revision 5 is the plan revision under
review, at HEAD `41dd5081`.

**Pre-review consistency fixes (Stage, revision 5 text only).** Revision 5
had added PRE-3F but had not carried it through the rest of the plan:

1. Dependency Graph: `PRE-2 -> PRE-3F -> PRE-3`. The execution order now
   includes PRE-3F.
2. No-deadlock note: PRE-3F has no edge to or from `142.058-T`. The PA-1
   exception, not an edge, authorizes its edit.
3. PA-1 row and approval phrase: they add `<PRE-3F>`, name the
   invariant-6 ownership exception with the diff-contract limit, and add
   PRE-3F to the `9B7EC1E4` lineage.
4. Invariant 6 names the PRE-3F exception.
5. PRE lineage rule: PRE-3F shares PRE-3's `9B7EC1E4` lineage.
6. Decisions D5-A: the retry wording now matches revision 5 (install
   before readiness, and Transient same-revision retry).
7. Constitution Check:
   * Row VI adds PRE-3F.
   * The old VII/VIII row is split. Row VIII now actually names the
     safety modes. Revision 5 claimed the CR-03 fix but the row text was
     unchanged.
8. Runtime Verification: a PRE-3F row is added, and the PRE-3 row gains
   the F54 evidence.

No unit's behavior, scenarios, or acceptance was changed.

**Personas (5, same as attempt 3).**

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust | ADVISORY | 0 | 0 | 1 | 6 |
| Scope Boundary | ADVISORY | 0 | 0 | 3 | 3 |
| Architecture | ADVISORY | 0 | 0 | 4 | 2 |
| Constitution | ADVISORY | 0 | 0 | 3 | 3 |
| Learnings | ADVISORY | 0 | 0 | 2 | 2 |

**Merged gate: ADVISORY.** After dedup there are 0 P0, 0 P1, 9 P2, and 11
P3. Plan hardening was required, and it is present. `ProposedAction` /
`ActionRisk` rows exist for PA-1..PA-5.

**Mandatory confirmations.**

* **L3-1 is closed. FASP is deterministic on the normal path, on both
  Windows and Unix.** All five personas agree. Source facts:
  * The side-effect windows open only after `ensure_daemon` returns
    (`read_server_cli_mcp_parity_test.rs:675-697`, `:1537-1542`).
  * Install-before-ready is ordered correctly. The install is an awaited
    RwLock write that completes before the Release store on
    `hydration_ready`. `_health` reads that flag with Acquire
    (`state.rs:1820-1856`), and the status `generation` block reads the
    same activator slot (`lifecycle.rs:1080-1083`).
  * Publication-last holds. `publish_active` is the activator's last step
    (`activation.rs:1188-1204`), and the gate then writes only in-memory
    state (`startup_activation.rs:265-300`).
  * The Windows identity divergence is real. The fixture hashes the
    `\\?\` spelling (`:476-521`), while the daemon hashes the
    `normalize_canonical` spelling (`workspace.rs:21-61`, `:1011`).
    PRE-3F fixes it.
* **The negative test is not weakened.**
  `unknown_ipc_methods_are_refused_without_side_effects` keeps its body,
  its three failure clauses (including code `16_001`), `snapshot_directory`,
  `changed_entry_count`, and its fingerprint fields byte-identical. The
  barrier runs in the shared precondition, before `binding_before` and
  `files_before`, and a timeout blocks rather than passes.
* **The positive test is a genuine RED-first witness.** At HEAD no
  activator is installed, so step (a) fails; PRE-3 turns it GREEN.
* **Two residual paths.** Both are fail-loud, not silent, and neither is
  a P1:
  * the install-failure path (P2-1);
  * barrier self-interference (P2-2).

**Attempt-3 P2 absorption, checked against the current text:**

| Status | Items |
|---|---|
| Absorbed | CR-01, CR-02 (`within_root`), the typed `CandidateBuildError`, the 2-arg wrapper plus `_with_context`, A3-1 transient retry (it composes: `activate_initial_attempt` never consults `RejectionCache::may_attempt`, `activation.rs:962-983`), the refusal re-probe, the probe half of CR-04, S-1, and S-2 |
| Absorbed only by this attempt's consistency fix | CR-03 (row VIII) |
| **NOT genuinely absorbed** | See P2-3 and P2-4 below |

**P0 / P1:** none.

**P2 (record as follow-ups; none blocks harvest):**

1. **P2-1 (all five personas): after PRE-3, `NoActivator` is not
   terminal.**
   * If the startup install fails, a later tick can install and activate
     inside an F54 window. The positive test fails in the same run, so the
     race is not silent.
   * Fix: when the root exists at startup, treat an install failure as
     final for that daemon (no late install), or make it a HALT in F54.
     In the three-run evidence, require every F54 barrier return to be
     `Active`.
2. **P2-2 (Scope, Architecture, Learnings): barrier self-interference.**
   * The barrier calls `get_workspace_status` between the two snapshots
     it compares. Each call runs `connect_db`: `create_dir_all`, the lock
     file, the DB open, and schema bootstrap (`cozo_backend/mod.rs:121-220`,
     stash `4628001C`).
   * `Active` may then never stabilize, which gives a deterministic 75 s
     block at PRE-3 and a HALT.
   * Fix: take snapshot, sleep 250 ms, snapshot, with no IPC call in
     between, and compare fingerprints separately. Alternatively, have
     PRE-3F record that snapshot → status → snapshot is stable at HEAD, and
     HALT if it is not.
3. **P2-3 (Constitution): the CR-04 relay half is not in the text.**
   * The disposition table claims "Invariant 4 is restated per layer, and
     NEW-4's relay line gets its own prefix".
   * But NEW-4 still emits "the same format as NEW-2", and invariant 4 is
     unchanged.
   * Fix: give the relay line its own prefix, restate invariant 4 per
     layer, and assert the stderr format in NEW-4.
4. **P2-4 (Rust, verified by Stage): the `WorkspaceError::Failed`
   "correction" is false.**
   * `WorkspaceError` has no `Failed` variant (`errors/mod.rs:20-43`).
     "Failed to parse workspace files" is `HydrationError::Failed`
     (`:46-48`).
   * The design outcome is right: `&str` input, no mapping. Only the
     wording in the revision-5 table and in PRE-1 is wrong.
5. **P2-5 (Constitution): invariant 7 contradicts PRE-3's intended
   change.**
   * Invariant 7 says responses outside `GENERATION_PINNED_READS` are
     unchanged "with or without a generations root".
   * Yet PRE-3 makes `get_workspace_status.generation` non-null when a
     root exists.
   * Fix: exempt that block.
6. **P2-6 (Scope, Constitution): PRE-3 is at the edge of the 2-hour
   rule.**
   * `install_generation_gate` is not in its production-file and function
     list.
   * Fix: list it, and consider splitting the transient retry into its
     own unit.
7. **P2-7 (Scope): the deliberation has drifted from the plan.**
   * Deliberation D5-A still says PRE-3 retries only on a revision change,
     and its lineage omits PRE-3F.
   * The PA-1 phrase asks the operator to "confirm D5-A".
   * Fix: add deliberation Amendment 3 before the PA-1 phrase is
     presented.
8. **P2-8 (Architecture, plus Stage): the failure signature is too
   coarse.**
   * The signature is "the first assertion line". For the multi-row RED
     cases (matrix, Control), that is only the header.
   * Expected post-PRE-3 row-text changes are not enumerated: the
     `get_workspace_status` responses gain a `generation` block, and the
     `_shutdown` window now includes the runtime DB teardown at exit.
   * Fix: define a per-row signature (descriptor, surface, and expectation
     prefix, excluding payloads), and list the expected changes.
9. **P2-9 (Architecture): PRE-4's reconciliation-quiescence check may
   observe read-side SQLite sidecar mtimes.**
   * Fix: take the "before" snapshot after one warm-up read, or
     characterize the effect.

**P3 (advisory, 11):**

* Dual retry policy: key the driver on the activator-recorded revision,
  and reuse `backoff_delay` (Rust, Architecture).
* The gate's `error!` log repeats on every failed attempt, which
  contradicts "logged once" (Rust).
* `within_root` needs an explicit `map_err` to `Io { op, kind }` (Rust).
* `ensure_daemon` returns from inside its loop, so the barrier belongs at
  the ready-return point. Also, the 2-versus-3 scenario wording and
  PRE-3F's dependency is really PRE-1 (Rust, Scope).
* Publication-last holds only inside the activation deadline, and
  re-activation over an existing runtime copy needs a note. Architecture
  verified the re-copy is atomic (Rust).
* Positive step (c) puts runtime internals into F54's file (Scope).
* PRE-4b's acceptance cites the PRE-3 baseline without a PRE-3 edge
  (Architecture).
* The Constitution III/IV row omits `runtime_root` (Constitution).
* The CR-03 "new safety rows" claim is overstated (Constitution).
* Cite `test-probe-resets-idle-ttl-livelock-2026-09-04.md` and
  `cozo-sqlite-busy-locked-reopen-panic-catch-unwind-2026-07-15.md`, and
  state that an IPC error during the barrier means re-poll (Learnings).
* Record F54 wall-clock time, and capture full test output (Learnings).

**Disposition.** ADVISORY: no P0 or P1, and the circuit stays closed.

* Stage did not revise any unit after the review.
* The P2s are carried to the operator. Each one needs either (a) an
  explicit, authorized text-only fix before harvest, or (b) its fix
  written as task-level acceptance at harvest.
* P2-7 needs deliberation Amendment 3 before the PA-1 phrase is
  presented.

Nothing was harvested, no shipment was assembled or changed, and no
backlog, stash, source, test, or config file was touched. Harvest needs a
separate, explicit operator authorization, and PA-1..PA-5 remain
ungranted.

<!-- plan-review-attempt: 4 -->

### Attempt 5: FAIL

**Authorization and scope.** The operator answered "yes to all" at
2026-09-27 22:54 -07:00. That authorized Revision 6 and ONE review scoped
to Revision 6 only, with a STOP (no revision) on any P0 or P1. Five
personas, the same set as attempts 3-4, reviewed:

* the `### Revision 6` section (R6.1-R6.10);
* its override notes on PRE-3F, PRE-3, PRE-4b and PRE-4;
* the Revision 6 graph and per-shipment execution order;
* the Revision 6 No-deadlock and F54 bullets;
* the Constitution "Revision 6" row;
* the changed Runtime Verification rows.

The personas read the plan, the backlogit task text, and the source at
HEAD `41dd5081`, all read-only.

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust | ADVISORY | 0 | 0 | 4 | 6 |
| Scope Boundary | FAIL | 0 | 2 | 5 | 4 |
| Architecture | ADVISORY | 0 | 0 | 3 | 5 |
| Constitution | FAIL | 0 | 1 | 2 | 5 |
| Learnings | ADVISORY | 0 | 0 | 4 | 3 |

**Merged gate: FAIL.** After dedup there are 0 P0, 3 P1, 12 P2, and 17 P3.
Where personas rated the same issue differently, the more conservative
severity was kept (Rust F1, rated P2, merged into P1-B). As instructed, Stage
did NOT revise the plan after this review.

**What the personas confirmed.**

* **Edges.** E1-E5 are sufficient and acyclic (Architecture, Scope). No
  S1 item depends on S2-S4.
* **L3-1.** It stays closed with PRE-3F after PRE-4, because the F54 file is
  on no S1-S3 branch (Architecture).
* **C1.** The overlay can technically be run at `<pre-PRE-3>`. Every
  import the F54 file needs exists there, and PRE-3F adds only
  `read_server_layout` (Rust).
* **No hidden F54 dependency in S1-S3.** Checked in `Cargo.toml`, the
  coverage manifest, `.cargo/config.toml`, and the launcher and F50
  harness headers (spot-check, Architecture). Scope found the
  task-text exception listed as P1-A.
* **The test-first deviation record** has a deviation, a justification,
  and a rejected alternative (Constitution).
* **No overreach.** No PA-5 planning and no backlog mutation (Scope,
  Constitution).

**P1 (open, blocking assembly):**

1. **P1-A (Scope): F54-dependent criteria are left in S1/S2 tasks and are
   not in M1-M12.**
   * `142.073-T` (NEW-5, S2) acceptance requires
     `contract_read_server_cli_mcp_parity` "unaffected (per-case maps
     unchanged)". The NEW-5 plan section says the same. That target is
     not on the S2 branch.
   * `142.064.001-ST` (S1) says its RED phase records "the F54 per-case
     map (against the PRE-3F after-edit map)". M6 covers only `.003-ST`.
   * Fix: add M13 (drop the F54 criterion from `142.073-T` and NEW-5, and
     have `142.063-T`'s B1 state that nothing is expected from S2/S3) and
     M14 (drop the F54 map from `142.064.001-ST`).
2. **P1-B (Scope P1; Rust F1, P2): the R6.9 text-change list for
   `142.063-T` leaves criteria that contradict B0/B1 and R6.4.**
   These remain in the task text:
   * the "before editing, Ship runs…" before/after equality, which on S4
     would run the un-barriered file, the exact L3-1 race;
   * "later units (PRE-3, PRE-4b, PRE-4) compare against the after-edit
     map";
   * "so PRE-3's three-run evidence can show every return variant";
   * verification steps 1-2 ("fails at step (a)");
   * scenario 2, "positive RED witness";
   * the HALT rule "a pre-existing case changes status or per-row
     signature", which conflicts with B1 (i)-(iii);
   * the notes "Why separate and before PRE-3" and "No dependency edge to
     or from 142.058-T".

   The PRE-3F override note also does not supersede unit scenario 1, the
   HALT rule, or "At HEAD, (a) fails".

   Fix: list each of these with its replacement text in R6.9, and extend
   the override note.
3. **P1-C (Constitution): the P-002/P-004 red-phase gate for `142.063-T` is
   not addressed.**
   * P-004 needs every harness test function to fail with its expected
     markers before `harness-ready` is applied.
   * R6.4 removes the `EXPECTED_PENDING_RED` mapping and makes C1
     acceptance evidence collected after claim, not the red-phase record
     made before claim.
   * The red posture of PRE-3F's other test functions (the byte-identical
     negative test, and the barrier) is not stated.
   * As written, harness-architect cannot apply `harness-ready`, and Ship
     would halt at claim.
   * Fix: state the harness-manifest disposition. Either C1 runs before
     `harness-ready` and is recorded as `Red Phase: CONFIRMED (C1 @
     <pre-PRE-3>)`, or name an operator-granted, test-scoped `skip_policy:
     P-004`. State each PRE-3F test function's red posture.

**P2 (12, non-blocking; carried to the Revision 7 decision):**

1. **Baseline attribution (Scope, Architecture, Rust).** B0 at `41dd5081`
   against B1 on S4 mixes four sources: the PRE-3F edit, S1-S3 production,
   S2/S3 and main drift, and new descriptors (a new CLI descriptor makes
   `cli_arguments` panic). A HALT cannot be traced to a cause. Option 1
   (the `6d216d19` map) is probably not in per-row format.
   Fix: capture B0 (the unedited file) and B0′ (the PRE-3F file) in C1's
   checkout at `<pre-PRE-3>`, and compare B0 → B0′ → B1. Use the same OS
   for all three. State that nothing is expected from S2/S3.
2. **PA-5 order and the S4 claim gate (Architecture, Learnings).** No edge
   holds `142.058-T` behind the PA-5 tasks. If S4 is claimed before PA-5
   is harvested, it reopens a RED interval (4 known-RED tests with no
   owner). Fix: deferred edges for the PA-5 harvest, and no S4 claim or
   F54 cherry-pick until the PA-5 tasks are in S4.
3. **Edge apply order (Architecture).** "In order E1..E5" conflicts with
   "E5 in the same step as E1". Applying E2 before E1 briefly forms a
   cycle. Fix: E5, E1, E2, E3, E4, with a cycle check after each.
4. **C1 procedure and policy (Constitution, Learnings, Rust, Scope).**
   * A worktree breaks P-016; an in-place detached overlay touches
     P-011; "removed in the same step" is destructive (VII).
   * There is no compile-failure path and no admission gate (disk,
     running cargo). The overlay source is not pinned to the `142.063-T`
     commit blob.
   * B0 option 2 has the same checkout issue.
   * Fix: name one operator-pre-approved procedure. For example: from a
     clean tree, `git switch --detach <sha>` in the only worktree, overlay
     exactly two paths, run `cargo check` first, restore exactly those
     paths, and verify the tree is clean and at the S4 HEAD. A compile
     failure or any other failure text is a HALT, not a witness.
5. **Careful-mode gap in S1 (Constitution).** PRE-3/PRE-4 merge without
   the repeat-run evidence, and a late S4 HALT would need a production fix
   on main. Fix: three consecutive identical runs of
   `integration_read_server_generation_wiring` and the named consumer
   targets in S1, plus a Risks bullet naming the owner.
6. **Inline two-observation rule is not self-contained (Rust).** It does
   not define F (`path`, `branch`, `db_path`, `generation`), G, or S (root
   and `.db`/`.db-*` metadata rules, Windows error 33), and has no poll or
   deadline, or never-pass-on-timeout rule.
7. **Leftover F54 text in `142.064-T` (Scope).** The scope line "ST-c …
   + F54 determinism evidence", the HALT note "or any … F54 case
   deviates", and the `.003-ST` pointer to "F54 determinism evidence".
8. **`142.060-T` text (Scope).** It still reads "final 142-S code task",
   "blocked by 142.054-T through 142.058-T", "142-S PR residual-risk
   record", and "PA1".
9. **H1 content is not limited (Scope).** S1's "No RED harness commit"
   needs H1 to carry no test or harness files.
10. **Interim main after S3 (Scope).** The fail-closed launchers reach
    main with PA-4 on hold and the F55 docs in S4, which waits on PA-5.
    The operator should acknowledge this, or get a docs or PA-4 stopgap.
11. **Closure mechanics (Learnings).** `142-F` last in S4 implies a
    cascade `shipment ship`. That path has been non-terminating or
    force-releasing on 142-F's roster before. Fix: S1-S3 use manual
    safe-close; before S4, check that every descendant is done or archived;
    set a timeout.
12. **Parked-branch safety ref (Learnings).** Create an immutable ref or
    tag at `41dd5081` before H2, and check at each shipment that its
    commits are reachable from it.

**P3 (17, advisory):**

1. An edge `142.063-T` → `142.060-T` would enforce S4-after-S3; the
   `[[test]]` stanza context also needs S3 (Architecture).
2. Make the single cherry-pick + PRE-3F commit mandatory (Architecture).
3. Record the OS for B0 (Architecture).
4. Run `git grep read_server_cli_mcp_parity` at H5 (Architecture).
5. Note that the launchers merge before their F55 docs (Architecture).
6. State the PA-5 placement once, as a constraint (Scope, Constitution).
7. S1 text still points at F54's `cli_arguments` and MCP extraction
   (Scope).
8. Annotate stale Risks, Constitution VIII and Requirements Trace text,
   including the F51 `WARNING:` bullet (Scope, Constitution).
9. Define `<pre-PRE-3>` as the PRE-2 GREEN tip with no PRE-3 change in
   `lifecycle_policy.rs`/`state.rs`, record its SHA at S1 closure, and
   check it with `merge-base --is-ancestor` (Constitution, Rust).
10. "Each equal to B1" should read "equals B0 modulo (i)-(iv), and the
    runs equal each other" (Rust).
11. Quote the final (i)-(iv) verbatim in M2 (Rust).
12. Change (ii) (the `_shutdown` teardown) must be identical in all three
    runs (Rust).
13. B1 compares per-row lines; a case-level line may change only through
    an allowed row (Rust).
14. `Settled` derives, `?` propagation, and C2 loop placement in the diff
    contract (Rust).
15. Defer cherry-pick mechanics to Ship/harness-architect rather than
    revising the plan again (Learnings).
16. The S4 full `cargo dev-test` must show every F54 case GREEN, from
    complete output plus the exit code (Learnings).
17. The barrier and step (a) stop polling before any idle-TTL, shutdown or
    endpoint-release window (Learnings).

**Disposition: STOP.** The operator instruction was to stop on any P0 or
P1 without revising, so Revision 6 is NOT revised here.

* The escalation circuit is not tripped. This is the first FAIL after
  ADVISORY attempt 4, not a second consecutive FAIL.
* The P1 fixes are text-only: M13/M14, a complete `142.063-T` text list,
  and a P-004 disposition. A Revision 7 plus one more scoped review needs
  a new operator authorization. P1-C may also need an operator decision:
  the C1 red-phase record before `harness-ready`, or a test-scoped
  `skip_policy: P-004`.

The review changed no backlog item, edge, shipment, stash entry, source,
test, or config file, and made no git mutation. S1-S4 are not created.

<!-- plan-review-attempt: 5 -->

### Attempt 6: FAIL

**Authorization and scope.** The Orchestrator (autopilot, 2026-09-27
23:16 -07:00) authorized Revision 7 and ONE review scoped to it, with a
STOP and no Revision 8 on any P0/P1. The same five personas reviewed:

* `### Revision 7` (R7.1-R7.11);
* the PRE-3F Revision 7 override;
* the "Revision 7 graph changes";
* the Constitution "Revision 7" row;
* the PRE-3F Runtime Verification row.

They read the plan, the backlogit task text (`.backlogit/queue/`), the
source at HEAD `41dd5081`, `docs/compound/` and `docs/memory/`, all
read-only.

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust | FAIL | 0 | 2 | 3 | 3 |
| Scope Boundary | ADVISORY | 0 | 0 | 2 | 7 |
| Architecture | ADVISORY | 0 | 0 | 3 | 5 |
| Constitution | FAIL | 0 | 2 | 4 | 4 |
| Learnings | ADVISORY | 0 | 0 | 4 | 3 |

**Merged gate: FAIL.** After dedup there are 0 P0, 2 P1, 10 P2, and 18
P3. Rust and Constitution found the same two P1s independently. Learnings
rated the second one P2; the more conservative severity was kept.

**What the personas confirmed.**

* **P1-A is closed.** The quoted text in M13-M17 matches the current
  queue text word for word, and every other F54 criterion in S1 maps to
  M1-M12. The S2/S3 tasks have no other F54 reference (Scope).
* **P1-B is closed.** Every attempt-5 item maps to a T row, and the
  override note supersedes scenarios 1-2, "At HEAD, (a) fails" and the
  HALT bullet (Scope).
* **The stub-first approach is a legitimate red phase in principle.** It
  is a tests-only task, the positive test fails for exactly the reason IC
  removes, and the test bodies are byte-identical between HC and IC. One
  worktree, no older-commit checkout, no history rewrite (Constitution).
* **Step (a) passes in HC.** It uses `spawn_daemon` and polls `_health`
  directly, not `ensure_daemon`. Each of the 4 async tests calls
  `ensure_daemon` right after `ReadServerFixture::new()`, so the stub
  fails before any side-effect window. The structural test cannot fail,
  so it stays GREEN in HC (Rust).
* **Edges.** E5, E1, E2, E3, E4, E8 stay acyclic after every step, no S1
  item gains an edge into S2-S4, and S4-after-S3 is now edge-enforced
  (`063→060→055/056/057`, `058→063`, `059→058`). E6/E7 stay acyclic if no
  PA-5 task depends on `142.058-T` (Architecture).
* **B0 attribution for S2.** NEW-5 edits only `src/bin/engram.rs` and
  adds no F19 descriptor. F54 enumerates `all_descriptors()`, so a hidden
  clap subcommand adds no rows (Architecture).
* **Scope.** R7 stayed text-only. No task `updated_at` changed. PA-5 is
  not planned, and PA-6 is recorded, not granted (Scope, Constitution).

**P1 (open, blocking assembly):**

1. **P1-D (Rust R7-1 = Constitution P1-1): HC cannot compile as
   specified.**
   * `.cargo/config.toml` sets `rustflags = ["-Dwarnings"]`.
   * R7.3's stub "immediately returns `Err(...)`" and "makes no IPC
     call". That leaves `SETTLE_TIMEOUT` unused and never constructs
     `Settled::NoActivator`, so both are `dead_code` errors. Derived impls
     do not count as uses.
   * Under the pedantic lint gate, `clippy::unused_async` and
     `clippy::unused_self` also fire.
   * R7.3 lists "a compile error" as a HALT, so HC would HALT against its
     own rule, and P-002/P-004 (`cargo check --all-targets` exit 0) could
     never be met. A lint attribute added in HC and removed in IC would
     break T6.
   * **Proposed fix (text only):** pin the exact stub body so it uses
     every item it declares, inside the body that IC replaces. For
     example: `tokio::task::yield_now().await;` followed by
     `Err(format!("{F54_RED_MARKER}: activation-settle barrier not
     implemented (PRE-3F); endpoint={}, timeout={SETTLE_TIMEOUT:?},
     variants={:?}", self.endpoint, [Settled::NoActivator,
     Settled::Active]))`. State: "no lint attribute in HC or IC; `cargo
     check --all-targets` and root pedantic clippy are clean at BOTH HC
     and IC". Add this to 142.063-T AC10.
2. **P1-E (Rust R7-2 = Constitution P1-2; Learnings L6-2, rated P2): the
   `EXPECTED_PENDING_RED` mapping gives one test two owners.**
   * At `6d216d19` the target shows 1 passed / 4 failed (the
     structural-completion memo). The structural test cannot fail, so the
     four known-RED tests are the matrix, control, `unknown_ipc`, and
     equivalence cases.
   * R7.3 item 2 and T7 map `unknown_ipc_methods_are_refused_without_side_effects`
     to owner `142.063-T`. It is also one of "the four known-RED cases",
     owned by `142.058-T`. R7.3 item 3 then lists them as separate groups,
     which adds up to five.
   * After IC, `unknown_ipc` is still RED at its B0 signature. So
     `142.063-T` would close with a mapped test still RED, which is a
     P-004 / Ship Step 4.3 conflict.
   * P-004 requires a posture for "every test function", and R7.3 states
     none for the structural test.
   * **Proposed fix (text only):** add a per-function table with 6 rows:
     the structural test, the 4 async tests, and the positive test. Each
     row gives the name, whether it calls `ensure_daemon`, its B0 status,
     its HC status and exact expected text, its post-IC status, and
     exactly ONE owner.
     * The positive test is owned by `142.063-T` (RED in HC, GREEN at
       IC).
     * The four known-RED cases, including `unknown_ipc`, are owned by
       `142.058-T`. In HC they carry a temporary stub-text signature, and
       after IC they return to their B0 signature within B1.
     * The structural test is GREEN throughout and not mapped.

     Reword T7 and the IC bullets to match.

**P2 (10, non-blocking):**

1. **The expected failure text trips R7.3's own HALT rule (Rust,
   Architecture, Constitution).** The 4 async tests wrap the error as
   `panic!("{F54_BLOCK_MARKER}: {error}")`, so the HC text reads
   `F54-BLOCK: F54-RED: activation-settle barrier not implemented
   (PRE-3F)`. R7.3 makes "any other marker" a HALT, and Ship Step 4.3 may
   classify `F54-BLOCK` as blocked infrastructure.
   *Fix:* valid RED is a panic containing the exact substring
   `activation-settle barrier not implemented (PRE-3F)`, with the
   `F54-BLOCK` prefix allowed. The manifest classifies these as
   `EXPECTED_PENDING_RED`, not BLOCK.
2. **The `[[test]]` stanza source is misstated (Rust R7-4, Scope P2-1,
   Learnings L6-1; corrected by Stage read-only).**
   * `git log -S` shows that the stanza and an inert placeholder file were
     added by `3f890662` (142.001-T, F00), which is already on `main`.
   * `7bd9e504` and `6d216d19` touch only the test file.
   * So HC makes NO `Cargo.toml` edit. It replaces the placeholder
     content, which removes `placeholder_registered`. The unit's "no
     `Cargo.toml` edit" is correct, and R7.3 item 1, T6 and R7.6 E8's
     stanza note are wrong.
   * This also means the F54 target already exists on the S1-S3 branches,
     as the GREEN placeholder, so R6.1's "exists only on the parked
     branch" means the real body only. L3-1 is unaffected, because the
     placeholder starts no daemon.
   * *Fix:* correct R7.3, T6 and E8 accordingly, and list
     `placeholder_registered`'s removal in the per-function table (P1-E).
3. **The carve-out conflicts with the HALT list (Scope P2-2,
   Constitution P2-4).**
   * R7.3's "NOT valid RED" list and T13 do not except a case that keeps
     its B0 status.
   * The Constitution R7 row says "every pre-existing F54 case fails at
     `ensure_daemon`", with no carve-out.
   * *Fix:* add "…other than a case that keeps its B0 status and
     signature", and say "every case that calls `ensure_daemon`".
4. **R7.7's snapshot S contradicts itself (Rust R7-5).** It lists only
   path, kind and length, yet refers to a content read (`.db` metadata
   only; error 33 "on read"). As written, a same-length rewrite passes.
   *Fix:* non-DB files include their content bytes. `.db`/`.db-*`
   entries use metadata only. Error 33 on a content read records the
   length only. Any other I/O error re-polls until the deadline.
5. **B1 has no path forward for a legitimate `main` change (Architecture
   P2-2).** A descriptor added on `main` after A0 adds rows, and that is a
   HALT with no resolution rule. *Fix:* add allowance (v). Rows for
   descriptors added after A0 are allowed only when a `git log
   <A0>..HEAD -- src/tools/capabilities.rs` record attributes them to a
   merged change. They are judged against their expected-outcome class,
   not against B0.
6. **The PA-6 hold has no enforcer, and option B freezes the queue
   (Architecture P2-3, Constitution P2-3).**
   * "Build S3, PR waits" leaves S3 active. Under P-001 that blocks every
     other shipment and lets the review go stale.
   * A dark-mode merge pre-authorization could merge S3 anyway.
   * *Fix:* redefine B as "do not CLAIM S3 until PA-6 is decided". Add an
     R7.6 hold row on S3, `142.055-T` and `142.060-T`, name the
     Orchestrator as enforcer, and require the P-014 approval to cite the
     PA-6 decision.
7. **The P-004 commands don't match (Constitution P2-1).** P-004 names
   `cargo check --all-targets` exit 0 and `cargo dev-test` non-zero with
   markers. R7.3 cites only the targeted `cargo test`. *Fix:* record
   both. Any `dev-test` failure at HC outside the per-function table is a
   HALT.
8. **A0 durability and P-011 (Constitution P2-2).** A B0 comment in the
   git-tracked `.backlogit/` dirties the parked tree before H5. The
   location of the full output isn't stated, and the T0 tag "after H1"
   may not contain B0. *Fix:* give B0 a durable artifact path, commit it
   before T0 or define how it crosses H5, and add `git worktree list
   --porcelain` to the A0 precondition.
9. **Closure mechanics are still under-specified (Learnings L6-3).**
   `backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`
   gives a 60-90 s budget, CPU/WAL sampling, killing by exact PID only,
   and a partial-state check. Its manual path covers an EXCLUDED feature,
   deletes the queue file (a VII action), and syncs (the union landmine).
   S4 includes `142-F`, which has no documented path. *Fix:* cite the
   procedure. S1-S3 skip `shipment ship` entirely. S4 makes one bounded
   attempt, checks `archived_ids`, records the delete as a ProposedAction,
   and does a clean cache rebuild.
10. **B0 covers a single OS (Learnings L6-4).** A0 runs on Windows only,
    so Linux CI IC runs have no baseline. *Fix:* gate them by
    characterization: positive test GREEN, three identical runs, every
    settle line `Active`, and the RED set per P1-E. A divergence that
    appears only on Linux is a HALT.

**P3 (18, advisory):**

1. Pin HC's imports: keep `workspace_hash`, and do not add
   `read_server_layout` until IC (Rust).
2. Exclude `F54 settle:` stderr from row signatures. B1 applies to
   same-OS Ship runs, and CI is gated by M12. Confirm that no PRE-4b read
   other than `get_workspace_statistics` flips a provenance row (Rust).
3. The 25 ms re-poll starts after the ≥250 ms check ends (Rust).
4. Two harmless leftovers: `142.066-T` "equivalence … RED until
   86F93068" and `142.062-T` "Downstream: PRE-3F, PRE-4b" (Scope).
5. More stale text in `142.063-T`: "characterization-first, then
   test-first", "OUTSIDE 142-S; requires PA-1", and T16's memo citation
   (Scope).
6. R6.9's `142.063-T` bullet ("add C1-C3", "remove the mapping") is not
   explicitly replaced in R7.6 (Scope).
7. M13/M17 have no marker in the NEW-5, PRE-3 or PRE-4 sections, and the
   graph-section hold omits "in S4's manifest" (Scope).
8. Option C's scope list is incomplete. It also touches `142.074-T`
   (NEW-6 supersession), `142.054-T` ("supersedes 118-S Guardrail 4"),
   and `142.059-T` (F55) (Scope).
9. The absorbed P2s beyond the four directive points should be noted as
   Stage discretion (Scope).
10. State that E4 must precede E8, or `063→060→058→063` forms
    (Architecture).
11. The L3-1 guard wording forbids A0. Add "…and containing the S1
    activator" (Architecture).
12. Push T0 (or bundle it), and record its later deletion as a VII action
    (Architecture, Constitution, Learnings).
13. The S4/HC hold is procedural only. Create S4 at the PA-5 harvest, or
    create it without `142.063-T` (Architecture).
14. NEW-5 could prove "the `all_descriptors()` name set is unchanged" in
    S2 (Architecture).
15. Option C is an operator-started pre-claim change. Cite P-021 C4
    (normal intake), not C6, and note that it needs new red phases for
    `1dfc1b5b`, `4995d681`, `5760b948` and `e24f5ae2` (Constitution).
16. The base VIII row ("granted only by PA-1", "three identical F54
    runs") and the PRE-3F body ("Without PA-1 … does not exist") are not
    marked superseded (Constitution).
17. Full-suite reviews need the exit code plus a grep of every `test
    result:`, `FAILED` and `WARNING:` line (Learnings; truncated-output
    compound doc).
18. Option A conflicts with the repo's `sync_workspace`-when-stale
    instruction (Learnings).

**Disposition: STOP.** The directive was to stop on any P0 or P1 with no
Revision 8, so Revision 7 is NOT revised here.

* **Circuit breaker.** Attempts 5 and 6 are two consecutive FAILs, so the
  planning-failure threshold is crossed. The escalation route resolved
  from a fresh read of `.autoharness/config.yaml` is `gpt-6-sol` /
  `openai` / `xhigh`. That differs from the Stage route
  (`claude-opus-5.5` / `anthropic`), so it is not a same-route no-op.
* **Escalation payload** (for operator or asynchronous review, NOT a
  seventh attempt):
  * threshold: 2 consecutive plan-review FAILs (attempts 5-6);
  * failures: P1-D (HC compile under `-Dwarnings`) and P1-E (duplicate
    `unknown_ipc` owner, no per-function table);
  * artifacts: this plan (revision 7) and
    `docs/memory/2026-09-27-stage-142-f-rev7.md`;
  * resumption: an operator-authorized Revision 8 limited to the P1-D and
    P1-E text fixes (plus, at Stage discretion, P2-1 to P2-3, which share
    the same paragraphs), and one scoped review.
* Stage does not re-run the review. Both P1 fixes are text-only, and
  neither needs a new design decision.

The review changed no backlog item, edge, shipment, stash entry, source,
test, or config file, and made no git mutation. S1-S4 are not created.

<!-- plan-review-attempt: 6 -->

### Attempt 7: FAIL

**Authorization and scope.** The operator decided on 2026-09-28 at 16:48
-07:00: "1. Run revision 8  2. PA-6: Hold S3". That allows ONE review
scoped to Revision 8, and it STOPS on any P0/P1, with no Revision 9. The
same five personas reviewed:

* `### Revision 8` (R8.1-R8.8);
* the text it touches: the PRE-3F "Revision 8 override", the Constitution
  "Revision 8" row, the PRE-3F Runtime Verification row, R7.2/T6, R7.3,
  R7.6, R7.9, and the R6.2 S3 row.

Their reads were read-only: the plan, `.backlogit/queue/`, the F54 source,
`.cargo/config.toml`, `Cargo.toml`, `_ship.agent.md` Step 4.3,
`docs/compound/` and `docs/memory/`. Two personas (Rust, Architecture) had
no git access. They could not check the R8.1 SHA facts themselves and
said so. Those facts were verified by Stage read-only in the Revision 8
session.

**Completeness check (before the review).** Stage checked that every
directive item is in Revision 8:

* the P1-D body: R8.2 and T17;
* the P1-E six-row table: R8.3 and T18;
* P2-1 to P2-3: R8.4, R8.1 and R8.5;
* the `3f890662` fact: R8.1;
* PA-6 = Hold S3: R8.6, the R7.9 decision note, the R6.2 S3 row, and the
  frontmatter;
* the Constitution R8 row, the PRE-3F R8 override, and the Runtime
  Verification row.

**No completion edit was needed.**

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust | ADVISORY | 0 | 0 | 1 | 3 |
| Scope Boundary | PASS | 0 | 0 | 0 | 5 |
| Architecture | ADVISORY | 0 | 0 | 3 | 3 |
| Constitution | FAIL | 0 | 1 | 1 | 4 |
| Learnings | PASS | 0 | 0 | 0 | 3 |

**Merged gate: FAIL.** After dedup there are 0 P0, 1 P1, 4 P2, and 12 P3.
Stage checked the P1 evidence against `.github/agents/_ship.agent.md`
lines 452-458 (Step 4.3, conditions 3 and 4) and the PRE-3F unit
(~1746-1753). It is kept as a P1.

**What the personas confirmed.**

* **P1-D is closed (Rust, Constitution, Learnings).**
  * The R8.2 body compiles under `-Dwarnings` and passes pedantic clippy
    with no attribute.
  * `F54_RED_MARKER` is a `&str`, and `ReadServerFixture.endpoint` is a
    `String`.
  * `SETTLE_TIMEOUT` is read, and both variants are constructed.
  * `yield_now` is available, because `tokio` has `full`.
  * `unnecessary_wraps` does not fire on an always-`Err` function, and
    `needless_pass_by_ref_mut` is a nursery lint.
  * There is no `[lints]` table, `clippy.toml`, or crate-level `allow`,
    and the toolchain is pinned to `1.97.0`.
* **P1-E's ownership half is closed (Rust, Scope, Constitution,
  Architecture).**
  * Six rows, each with exactly one owner. `unknown_ipc` belongs only to
    `142.058-T`.
  * The source matches the table: row 1 is sync and pure; rows 2, 4 and 5
    call `ensure_daemon` right after `new()`; row 3 calls it after
    assertions that cannot fail; row 4's second call (~1535) is not
    reached.
  * The B0 status matches the structural-completion memo (1 passed,
    4 failed).
  * No queue text gives `unknown_ipc` to `142.063-T`.
* **The R8.4 message is exact:** `F54-BLOCK: F54-RED: activation-settle
  barrier not implemented (PRE-3F); endpoint=<…>, timeout=75s,
  variants=[NoActivator, Active]` (Rust).
* **P2-1 to P2-3 and the Cargo fact are closed (Scope).** Every withdrawn
  phrase is replaced or covered by precedence. No live contradictory text
  is left for Ship to follow.
* **Edges (Architecture).** E5, E1, E2, E3, E4, E8 stay acyclic, and E4
  precedes E8. There is a direct 058→063 edge, so `142.058-T` is
  downstream of `142.063-T` in S4, and R8.3 needs no edge change.
* **PA-6 (Scope, Constitution, Architecture).**
  * It is recorded as an operator decision with a timestamp, not as a
    Stage grant.
  * Holding the claim keeps S3 inactive, so P-001 holds and S1/S2 go
    ahead.
* **Hard bounds kept (Scope).** No queue `updated_at` from this
  revision. The newest edits, on `142.055-T` and `142.057-T`, are from
  2026-09-27 21:26 -07:00, before the Revision 7 authorization.

**P1 (open, blocking assembly):**

1. **P1-F (Constitution): `142.063-T` still cannot reach a legal
   Step 4.3 verdict at IC.**
   * At IC, rows 2-5 are still RED, so the only verdict Ship can give
     `142.063-T` is `EXPECTED_PENDING_RED`.
   * That verdict needs Step 4.3 condition 3. The failures must map to a
     previously recorded, compiling RED harness of a later task
     (`142.058-T`), with its "owned test-file baseline before
     implementation".
   * It also needs condition 4. `142.058-T`'s declared Owned files,
     including the RED test file, must "still match [their] recorded
     pre-implementation content/type baseline".
   * `142.063-T` rewrites that very file at HC (R8.1) and again at IC. So
     condition 4 fails as written. Step 4.3 says: "Any unmet or
     unreadable condition is blocking; do not waive".
   * PRE-3F's "Ownership exception (invariant 6)" (~1749-1753) says
     `142.058-T` "inherits the edit". But it is scoped to the PA-1 grant,
     and it names no Step 4.3 baseline.
   * R8.3 moved the ownership but did not resolve the Step 4.3 check that
     attempt-6 P1-E cited. Ship would HALT at IC.
   * Condition 4 also asks whether `142.058-T` is "still unstarted", which
     sits awkwardly with the completed `142.058.001-ST` structural work.
   * **Proposed fix (text only).**
     * Add an R8.3 rule and a T18 addition. At HC and again at IC, before
       Step 4.3 runs for `142.063-T`, Ship re-records `142.058-T`'s
       owned-file baseline. The new baseline is that commit's blob of
       `tests/contract/read_server_cli_mcp_parity_test.rs`, recorded
       under the invariant-6 exception, and the expected markers are
       rows 2-5's R8.3 texts or B0 signatures.
     * State how "unstarted" is judged for `142.058-T`, given the
       completed `.001-ST`.
     * Any later pre-058 edit re-records the baseline the same way.
     * Because the invariant-6 exception was PA-1-scoped, the fix may
       need an operator decision. It may be a new PA, or a statement
       that R7's withdrawal of PA-1 carries the exception.

**P2 (4, non-blocking):**

1. **The positive test may fail `clippy::too_many_lines`, with no legal
   fix (Rust).**
   * The PRE-3F spec (~1782-1806) gives row 6 five steps: a poll loop,
     the barrier, six field checks, a file check, and a double snapshot
     around a 4.5 s sleep.
   * After rustfmt that is likely more than 100 lines, and the gate uses
     `-D clippy::pedantic`.
   * R8.2 bans lint attributes, and T6 allows no helpers, so the result
     would be an HC HALT with no allowed fix.
   * *Fix:* allow private helpers that only row 6 or the barrier call.
     They are added in HC and unchanged in IC. Or name them in advance.
2. **The PA-6 hold is procedural only, and nothing defines queue
   selection or dark-mode scope around it (Architecture, Constitution;
   Scope noted that attempt-6 P2-6's P-014 citation was only partly
   adopted).**
   * Once S2 ships, the queued S3 is the next shipment the normal
     selection picks.
   * Claiming S4 would take the P-001 slot while all of S4's tasks are
     edge-blocked.
   * *Fix:*
     * the Orchestrator skips S3 (PA-6) and S4 (behind S3), and unrelated
       shipments may be claimed;
     * any `DARK_MODE_SCOPE` excludes S3;
     * any S3 P-014 approval cites the new PA-6 decision;
     * alternatively, assembly creates only S1 and S2, and creates S3/S4
       when their holds release.
3. **The hold comment misses `142.057-T` and `142.056-T` (Architecture).**
   * F53 depends on `142.054-T` and NEW-5, not on `142.055-T`, so it
     becomes ready when S2 ships, with no hold marker.
   * *Fix:* comment on all four S3 members, and do not route a ready S3
     task while the hold stands.
4. **An open-ended hold lengthens the life of parked-branch artifacts
   (Architecture).**
   * After H2, only the local T0 tag reaches the S3/S4 SHAs, while `main`
     moves away from B0.
   * This makes attempt-6 P2-5, P2-8 and P3-12 more likely to bite.
   * *Fix:* while PA-6 holds, push or bundle T0 before H2 (mandatory),
     and resolve attempt-6 P2-5 as a precondition for claiming S4.

**P3 (12, advisory):**

1. **"With the stub marker" is still in some text:**
   * R7.2 T5, which "stands";
   * the PRE-3F Revision 7 override (~1716);
   * the R7 part of the Runtime Verification row;
   * the R7 Constitution row;
   * R7.3 item 1, "immediately returns".

   Precedence resolves these. *Fix:* inline "[R8.4]" or "[R8.2]" markers
   (Rust, Scope, Learnings, Constitution).
2. **Row 6's exact message is not pinned.** It may read `F54-RED: F54-RED:
   …`. *Fix:* pin it literally, or say the check is substring-only (Rust).
3. **The gate commands differ from `cargo lint` and `cargo fmt-check`.**
   The gates omit `--all-features`. *Fix:* record `cargo lint` and
   `cargo fmt-check` exit codes at HC and IC, or state that the file has
   no feature gates (Rust, Learnings).
4. **R8.8 says "P2-4 to P2-10 and all P3s stay open".** Revision 8 in
   fact absorbed P2-6 (partly), P3-1 and P3-10. *Fix:* record these as
   Stage discretion (Scope).
5. **Earlier PA-6 text still conflicts with R8.6:**
   * R7.9's option B row, "S3 can be built and reviewed, and its PR
     waits";
   * "before S3 MERGES";
   * "not granted" in R7.9 and R7.11.

   *Fix:* annotate each "(superseded by R8.6)", and note that "hold
   claim" is Stage's reading of "Hold S3" (Scope, Architecture,
   Constitution).
6. **R7.6 "Item text" lists only T1-T16.** *Fix:* add T17, T18 and the
   PA-6 hold step (Scope).
7. **Some T-row targets are not exact:**
   * T17 quotes AC10 with a comma that the queue text does not have;
   * queue verification item 3 is not updated for HC plus IC;
   * T18's target section is not named.

   (Scope.)
8. **R6.2 is out of date in two places.** The S1 row says "The F54 file
   is absent", and "Inter-shipment blocks" says S4-after-S3 is "not from
   an edge". *Fix:* annotate both with R8.1 and E8 (Architecture).
9. **The Dependency Graph section has no Revision 8 note.** It should say:
   no edge change; E4 before E8; S3 held with no edge (Architecture).
10. **R8.2's no-adjust rule names only Ship.** *Fix:* also name the
    harness-architect skill, whose Step 5.1 says "fix the harness until
    it compiles", and state that R8.4 replaces its default marker
    (Constitution).
11. **The Constitution R8 row files PA-6 under IX.** It belongs under
    P-001/P-014/P-017, and the row also omits R8.1 and R8.5
    (Constitution).
12. **The RED rule does not cover some output lines.** Drop-path
    `F54-BLOCK:` `eprintln!` lines and runtime `WARNING:` lines in the HC
    output fall outside it. *Fix:* make either one a HALT (Learnings;
    `docs/memory/2026-09-26-ship-142-s-f54-red-f50-gate-blocked.md`).

**Edge list: unchanged.** Assembly applies E5, E1, E2, E3, E4, E8, with
a cycle check after each and E4 before E8. E6 and E7 are applied at the
PA-5 harvest.

**Disposition: STOP.** The directive is to stop on any P0/P1 with no
Revision 9. Revision 8 is NOT revised here, and no assembly is done.

* **Circuit breaker.** Attempts 5, 6 and 7 are three consecutive FAILs.
  The escalation route, read fresh from `.autoharness/config.yaml`
  (lines 98-101), is `gpt-6-sol` / `openai` / `xhigh`. That differs from
  the Stage route (`claude-opus-5.5` / `anthropic` / `high`), so it is not
  a same-route no-op.
* **Escalation payload** (for operator or asynchronous review, NOT an
  eighth attempt):
  * threshold: 3 consecutive plan-review FAILs (attempts 5-7);
  * failure: P1-F, the Step 4.3 condition 3/4 owned-file baseline for
    `142.058-T` after `142.063-T` edits its RED file. It may need an
    operator decision on the invariant-6 exception without PA-1;
  * artifacts: this plan (revision 8) and
    `docs/memory/2026-09-28-stage-142-f-rev8.md`;
  * resumption: an operator-authorized Revision 9 for the P1-F text fix
    (plus, at Stage discretion, P2-1 to P2-3 of this attempt, and P3-1,
    P3-5 and P3-8), then one scoped review.
* Stage does not re-run the review.

The review changed no backlog item, edge, shipment, stash entry, source,
test, or config file, and made no git mutation. S1-S4 are not created.

<!-- plan-review-attempt: 7 -->


### Attempt 8: FAIL

**Authorization.** Operator, 2026-09-29 14:55 -07:00, verbatim: "approve
PA-7, Revision 9, push T0". Scope: a completeness check of Revision 9,
then ONE review scoped to Revision 9 and the text it touches. Any P0/P1
stops the work: no Revision 10 and no assembly. Reviewed lines:
Revision 9 (~1321-2063), the stale-site annotations, the PRE-3F unit
override (~2486-2575), the Dependency Graph note, the Constitution
Revision 9 row, and the Runtime Verification PRE-3F and `142.058-T`
rows.

**Completeness check (before the review).** Revision 9 contained
directive items 1-4 and all four attempt-7 P2 dispositions, with the T0
route set to a remote push at H2. PA-7 is recorded verbatim, with its
timestamp, as an operator decision (R9.1). E1-E8 and their apply order,
S1-S4 membership and order, and PA-6 = Hold S3 are unchanged. The check
found gaps: R9.11 claimed annotations that did not exist, and several
stale sentences had no Revision 9 note. Stage made these minimal
completion edits:

1. Revision 9 notes at R6.2 (the table note and the inter-shipment
   blocks), R6.3 "Unchanged in PRE-3F", R6.6, R6.8 (fixture owner),
   R7.3, R7.5 (the HC cherry-pick clause withdrawn), R7.6 (the T0 and
   item-text rows), R7.9 and R7.11 (PA-6 superseded by R8.6), R8.1
   (Consequences withdrawn), and R8.6 (hold comments extended).
2. A Revision 9 override block in the PRE-3F unit.
3. A Revision 9 note in the Dependency Graph.
4. A Revision 9 row in the Constitution Check.
5. Revision 9 text in the Runtime Verification rows for PRE-3F and
   `142.058-T`.
6. R9.3: "four public items" corrected to "six".
7. R9.2: a "full-suite GREEN at every S4 gate" bullet, for directive
   item 4.
8. R9.11: P3-5 and P3-8 wording corrected, and a row added listing the
   stale-site annotations.

**Personas.**

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust Reviewer | ADVISORY | 0 | 0 | 1 | 9 |
| Scope Boundary Auditor | ADVISORY | 0 | 0 | 1 | 7 |
| Architecture Strategist | FAIL | 0 | 1 | 2 | 7 |
| Constitution Reviewer | FAIL | 0 | 1 | 3 | 7 |
| Learnings Researcher | ADVISORY | 0 | 0 | 5 | 5 |

After deduplication: **1 P1, 10 P2, 31 P3.**

**P1 (Architecture and Constitution agree; Stage checked it against
`.github/agents/_ship.agent.md` Step 2, ~309-322, and Step 3, ~327).**

* **P1-G: the S4 gate contract conflicts with Ship Step 2's up-front
  harnessing.** R9.2 (~1429-1437) and R9.3 require three things:
  * no PA-5 RED harness is installed before `142.063-T`'s IC full-suite
    gate;
  * HC's only full-suite failure is the positive test (R9.3 HC item 5);
  * `142.063-T` IC, every PA-5 task, and `142.058-T` each close with
    Step 4.3 **PASS**, with no `EXPECTED_PENDING_RED` onto another S4
    task.

  Ship Step 2, however, harnesses **every** in-scope task of the claimed
  shipment before the ready queue is built ("Any gap halts; do not
  proceed with a partial set"). Step 3 then begins "Now that all tasks
  are harnessed". So the PA-5 RED harnesses are on the S4 branch at
  `142.063-T`'s HC and IC, and at each earlier PA-5 task's gate. The
  plan's gates cannot pass as written, and PA-7 waives nothing.

  The escalation directive's own item 3 wording ("do not install them
  ahead of PRE-3F's full-suite gate") carries the same conflict. The
  sentence "no `EXPECTED_PENDING_RED` onto another S4 task" came from
  this session's completion edit 7. The underlying conflict predates that
  edit.

  **Proposed fix (for a later, operator-authorized revision; NOT
  applied).**
  * Allow Step 4.3 `EXPECTED_PENDING_RED` at `142.063-T` IC, and at the
    earlier PA-5 gates, only for later PA-5 tasks' RED tests in their own
    disjoint files. All six Step 4.3 conditions are satisfiable, because
    no file is shared and F54 stays the GREEN placeholder, so nothing maps
    onto `142.058-T`.
  * Restate HC item 5 as "the only failures are the positive test and
    previously recorded PA-5 RED tests".
  * Keep the final-task no-pending-red rule.
  * Alternative: obtain an explicit operator or policy-owner decision for
    staggered harness generation in S4.

  Either way, pair the fix with P2-1 below.

**P2 (deduplicated).**

1. **`142.058-T` harness reuse at Ship Step 2** (Architecture, and
   Constitution; R9.5 ~1716-1790). At the S4 claim the tree holds only
   the GREEN placeholder. Step 2.4 ("repair/rebuild only if its manifest
   is invalid") could rebuild the harness and load the F54 body before
   PRE-3F. The (a)-(d) checks run inside the task, not at Step 2. (e)
   runs after FL, and the plan does not say what happens to FL if (e)
   fails.
2. **`Cargo.toml` is not disjoint per file** (Architecture, and Scope
   P3; R9.2 ~1400). PA-5 tasks may add stanzas. It needs a
   stanza-scoped ownership rule, kept out of the condition-4 baselines.
3. **`142.063-T` granularity** (Scope, and Constitution; R9.3
   ~1669-1680, Constitution row). HC writes about 12-16 functions across
   3 files. The two-hour check counts only IC functions, and no
   Task-granularity deviation is recorded. It also needs an HC size or
   time HALT.
4. **P-014 release circularity** (Constitution; R9.8 ~1950). The claim
   hold should lift on a later PA-6 decision. P-014 then gates the S3
   merge.
5. **The "no lint attribute" rule bans `#![forbid(unsafe_code)]`**
   (Rust; ~1573, AC2). It needs a carve-out: only `allow`, `expect` and
   `warn` are banned, whether direct or through `cfg_attr`.
6. **`ENGRAM_DATA_DIR` is not stripped** in the new fixture's daemon
   spawn or seed (Learnings;
   `engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`).
7. **Polling may mutate the snapshotted files** (Learnings;
   `test-probe-resets-idle-ttl-livelock-2026-09-04.md`). The
   characterization is not a gate, so run it before HC, or exclude paths
   that the probe writes.
8. **A refused `_shutdown` in `read_server` mode** (Learnings). The
   `kill`+`wait` fallback is the normal step (e) path, and it must emit
   no marker line.
9. **Zero-tolerance full-suite gates with known 142-S flakes**
   (Learnings). This needs a flake baseline on the S4 base and a
   rerun-in-isolation classification, with no blanket waiver.
10. **Single IPC calls in steps (a) and (d) have no bounded retry** on a
    transient SQLite busy or locked error (Learnings;
    `cozo-sqlite-busy-locked-reopen-panic-catch-unwind-2026-07-15.md`).

**P3 (31, summarized).**

* **Rust (9):**
  * the `# Errors` sections are a convention, not a gate;
  * `filesystem_snapshot` `Err` should re-poll;
  * the FL diff contract omits the `read_server_layout` import and the
    `&Path` conversion, and `workspace_hash` is certainly removed;
  * `engram_binary` is not listed;
  * AC8 does not cap IC at two private functions;
  * the ported snapshot and fingerprint bodies can drift from F54's own
    copies (with Architecture);
  * `stop_daemon`'s 10 s `IPC_TIMEOUT`;
  * `tokio::time::sleep` is required;
  * FL's wall-clock increase is expected.
* **Scope (5):**
  * A0R, the worktree precondition and the per-claim recheck are mild
    scope creep;
  * the glossary edit is a convention;
  * AC7 does not define "identical";
  * M5, M6, M11 and M15 are missing from V4/V8, and M1/M6 have
    conflicting owners in V4 and the Runtime Verification row (with
    Constitution);
  * the queued `142.063-T` text stays stale until assembly.
* **Architecture (6):**
  * PA-5 should confirm that the real-snapshot fixture is quiescent;
  * E6/E7 assume a straight PA-5 chain;
  * the drift ledger's path set is narrow, and `<S4 base>` is not
    pinned;
  * T0's retirement is undefined if PA-6 retires S4;
  * the oracle has no surface for `tests/helpers/`;
  * the stanza has no `required-features`.
* **Constitution (6):**
  * a `DARK_MODE_SCOPE` that includes S3 or S4 should HALT, not narrow
    silently;
  * pin a hold token, and mark the S4 shipment and all S4 tasks;
  * state the P-001 pipeline-wide impact of the hold;
  * widen the workflow-trigger check;
  * pre-check `core.hooksPath` before H2;
  * record the `Result<_, String>` test-only deviation.
* **Learnings (5):**
  * pin the daemon idle TTL;
  * Windows error 33 as well as 32, and a file locked in both snapshots
    is not "stable";
  * run the oracle completeness check at HC;
  * keep the complete full-suite output;
  * check the rulesets before the tag push.

**Edge list: unchanged.** Assembly applies E5, E1, E2, E3, E4, E8, with
a cycle check after each and E4 before E8. E6 and E7 are applied at the
PA-5 harvest. No edge was applied.

**Disposition: STOP.** Attempt 8 is FAIL on P1-G. Under the operator's
bound, there is no Revision 10 and no assembly. S1-S4 are not created,
and no hold comment was written. The P2s and P3s stay open for any later
revision.

* **Circuit breaker.** Attempts 5-8 are four consecutive FAILs. The
  P-013.6 escalation already ran after attempt 7 and produced
  `docs/decisions/2026-09-28-142-f-escalation-review.md`. Stage does not
  start a second escalation in this session, because the operator's
  bound is "Nothing else". The payload for the operator follows.
  * Threshold: 4 consecutive plan-review FAILs.
  * Failure: P1-G, the S4 per-task gate versus Ship Step 2's up-front
    harnessing.
  * Artifacts: this plan (Revision 9 + Attempt 8) and
    `docs/memory/2026-09-29-stage-142-f-rev9.md`.
  * Resumption: an operator decision on the P1-G route (an
    `EXPECTED_PENDING_RED` allowance onto later PA-5 tasks, or staggered
    S4 harnessing). Then an authorized Revision 10 limited to P1-G and
    P2-1 (and, at Stage discretion, P2-2 to P2-10), then one scoped
    review.
* Stage does not re-run the review.

The review changed no backlog item, edge, shipment, stash entry, source,
test, `Cargo.toml`, or config file. It created and pushed no tag, and
made no git mutation.

<!-- plan-review-attempt: 8 -->

### Attempt 9: FAIL

**Authorization.** Operator, 2026-09-29 17:38 -07:00, verbatim: "1. Option
A, Revision 10 with the Ship walkthrough  2. split 142.058-T into S5".
Scope: a completeness check of Revision 10, then ONE review scoped to
Revision 10 (~2081-2521) and the text it touches (the inline notes at
~358, ~1430, ~1437, ~1608, ~1663, ~1732, ~1965 and ~2027). Any P0/P1
stops the work: no Revision 11, and no assembly. Ship contract read:
`.github/agents/_ship.agent.md` Step 0.5 (153-293), Step 2 (309-330),
Step 3 (331-368) and Step 4.3 (442-470).

**Completeness check (before the review): complete, with two gaps.**
Revision 10 contains:

* R10.1: the Option A gate restatement, the gate table, and G1-G6.
* R10.2: the S5 split, and P2-1's FL as the pinned S5 Step 2 harness.
  It also has the `142.058-T` reset from `harness-ready`/`active` to
  `queued` at assembly, and the FL halt path.
* R10.3: dispositions for P2-2 to P2-10. P2-3's "split" is the existing
  HC/IC split, plus a recorded deviation and an HC HALT bound. It also
  allows `deny`/`forbid`, lifts the hold on the PA-6 decision alone, and
  covers `ENGRAM_DATA_DIR` isolation and the `Cargo.toml` stanza rule.
* R10.4 (shipments, edges, assembly), R10.5 (walkthrough S1-S5 with
  W1-W9), and R10.6 (the supersession table).

The inline notes that R10.6 claims were all present. Stage made two
minimal completion edits inside R10.4:

1. The F50 (`142.054-T`, W6) reset was missing from the S2 claim gate
   and from assembly addition 3. Both now include it.
2. The W9 item-text change was missing from the S3 claim gate and from
   assembly addition 6. Addition 6 now reads "S1, S2 and S3".

**Personas.**

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust Reviewer | FAIL | 0 | 1 | 2 | 6 |
| Scope Boundary Auditor | ADVISORY | 0 | 0 | 3 | 4 |
| Architecture Strategist | FAIL | 0 | 1 | 3 | 5 |
| Constitution Reviewer | FAIL | 0 | 3 | 4 | 4 |
| Learnings Researcher | ADVISORY | 0 | 0 | 2 | 7 |

After deduplication: **4 P1, 11 P2, 17 P3.** Stage confirmed P1-1 to
P1-3 against the files: the Ship lines, the `status:` fields in
`.backlogit/queue/142.05*-T.md`, and `142.061-T` line 27.

**P1 (the proposed fixes are for a later, operator-authorized revision;
NOT applied).**

* **P1-1: the S3 and S5 claims halt at Ship Step 0.5 item 1a**
  (Architecture, and Constitution; plan ~2430, ~2459, ~2467; Ship
  167-169). Item 1a runs before the claim and before Step 2. It halts
  with `SHIPMENT_STATE_INCONSISTENT` when a `queued` shipment holds an
  `active` task. Four such tasks exist today:
  * S3's `142.055-T`, `142.056-T` and `142.057-T` are `active`;
  * S5's `142.059-T` is `active`.

  R10.2 and W6 reset only `142.058-T` and `142.054-T`, and the
  walkthrough skipped item 1a. So "every S1-S5 gate can legally pass"
  (~2467) is false. This is the same class of problem as P1-G and P2-1.
  **Fix:** add an assembly step with three parts:
  * move every `active` member of S3 and S5 to `queued`. This also
    covers any other 142-F task left `active` by 142-S;
  * keep the valid RED-harness route labels on F51-F53, so Step 2 reuses
    them through the queued route;
  * add a history comment to each moved task.

  Add "every manifest task is `queued`" to every claim gate. Then re-walk
  Step 0.5 for S1-S5. This also settles P2-8 below.
* **P1-2: W1's sibling module can't read PRE-1's private fields** (Rust;
  Scope P2; plan W1 ~2400; `142.061-T` line 27, "private-field
  `BuiltCandidate` / `SealedCandidate`"). In Rust, private fields are
  visible to the defining module and its children, not to its siblings.
  So `publish_sealed_candidate` in `candidate_publish.rs` can't read the
  sealed inventory or id without editing `candidate_build.rs`. W1 makes
  that edit a certain HALT. **Fix:** do one of these:
  * PRE-1's final signatures, stubbed in its harness commit (G2), include
    `pub(crate)` accessors (`inventory()`, `id()`); or
  * make the PRE-2 file a child module (`candidate_build/publish.rs`,
    registered by `mod publish;` in PRE-2's harness commit, like W2).

  Either way, update PRE-2's "only `starts_with` in `candidate_build.rs`"
  check to name the new file.
* **P1-3: P2-9 changes Ship's handling of unexpected failures**
  (Constitution; Scope and Learnings P2 on the base run; plan R10.3 P2-9
  ~2312; Ship Step 4.3, the paragraph after the conditions, and Step
  4.4a).
  * Ship sends every unexpected failure to P-021 scope classification.
    If it is in scope, it goes back to build-feature. If it is out of
    scope, it goes through Step 4.4a and C2 capture.
  * P2-9 lets a green re-run clear the first unmapped failure, and sends
    only a *second* failure to intake. Option A does not allow changing
    Ship's contract.

  **Fix:** each unmapped failure gets P-021 classification when it first
  occurs. If it is out of scope, capture or reuse a C2 entry (4.4a
  discovery) before the fresh complete run gives the verdict. If it is in
  scope, go back to build-feature. Apply the same rule to the P2-9(1)
  base run.
* **P1-4: G5's panic-first RED has no Constitution II deviation**
  (Constitution; plan G5 ~2177, W4 ~2403, W8 ~2417). A method-1 RED body
  can be only the tagged panic (see P2-1). So the real assertions are
  written together with the production code and are never seen failing.
  W4 and W8 make this the default for most of S1 and S2, and no II
  deviation is recorded. The 142-S precedent (F51-F54) is noted, but that
  precedent was not a shipment-wide default. **Fix:** do one of these:
  * add a G5 step: the first Step 4.2 commit replaces the panic with the
    real body. Its RED against the task's own stub (upstream tasks are
    done by then) is recorded before any production edit; or
  * record a II deviation, with the rejected alternative and operator
    acknowledgment.

**P2 (deduplicated).**

1. **The G5 method-1 body fails lint under `-Dwarnings`** (Rust, and
   Learnings P3; ~2177). Code after the panic trips `unreachable_code`.
   Imports and helpers that nothing uses trip `unused_*` and `dead_code`.
   `.cargo/config.toml` sets `-Dwarnings`. Pin the RED body as the tagged
   panic only, with no unused items in the file.
2. **G2 stubs have no lint recipe** (Rust; ~2155, W2). Some problems:
   * unused parameters;
   * `clippy::unused_async`;
   * pedantic `missing_errors_doc`;
   * `pub use mcp::probe_mcp_read;` needs a real item.

   Adopt R9.3's stub technique for S1 and S2, or allow a scoped `allow`
   that the implementation removes.
3. **W9 moves an acceptance criterion, and its premise may be stale**
   (Scope, Architecture, and Constitution P3; ~2428; `142.055-T`,
   `142.056-T`).
   * F52's AC4 ("the fail-open assertion is rewritten") becomes F51 work.
     F51 carries RS4/RS5 approval.
   * The cited assertions at ~80/~154 were not found.
   * F52's notes say the expectation "is reversed in
     `contract_start_launcher`", which contradicts "`5760b948` did not
     touch that file".

   Check read-only (`git show --stat 5760b948`, and the current file)
   before any change. Record any AC transfer explicitly, and fix the
   "no scope change" sentence (~2467).
4. **The P2-9(1) base run is a new claim step with no flake path**
   (Scope, Learnings, and Constitution P3; ~2312). Make it a plan
   precondition in the R10.4 claim-gate column. Give it the P1-3
   classification path.
5. **The pinned FL instructions live only in the plan** (Architecture;
   R10.2 ~2233-2253). `142.058-T`'s item text still carries `7bd9e504`
   and "red phase CONFIRMED", and V3/V4/V8 describe FL as acceptance
   work. Add an assembly item-text edit with three points:
   * the S5 harness is exactly FL plus its red-phase record;
   * the historical records must not be reused;
   * any deviation is a HALT.
6. **The G1 baseline is not tied to the per-task harness record**
   (Architecture; ~2147; Ship condition 3). Condition 3 needs each mapped
   task's own record to name its owned test-file baseline. Allow the
   per-commit test-file hash, and cross-reference the G1 record from
   each task's record. A mismatch is a HALT.
7. **G3's name-scoped `harness_cmd` can pass with zero tests**
   (Constitution; ~2165). Record the expected test count for each
   `harness_cmd`, and require the output to show exactly that many.
8. **The "P-001 impact of the hold" is wrong while held tasks are
   `active`** (Constitution; R10.3). P-001 blocks other features while
   142-F has `active` tasks. P1-1's reset resolves this. Otherwise,
   restate the impact.
9. **The Constitution Check is stale, and the Revision 10 addition is
   incomplete** (Constitution; ~4232, ~2501, P2-3 ~2306).
   * The Revision 9 row is stale.
   * The addition does not cover the II deviation (P1-4), P-021 (P1-3),
     or the status resets.
   * The HC deviation lacks its rejected alternative (splitting HC into
     its own task), and HC plus IC may go over 2 hours.
10. **The status resets bypass Ship's operator-disposition halt**
    (Constitution; R10.2, W6).
    * The escalation review says "HALT, not re-label".
    * The `142.054-T` reset is outside the operator's decision.
    * R10.6 has no row for that superseded text.

    Record operator acknowledgment for each reset, and add the row.
11. **FL may inherit `ENGRAM_DATA_DIR`** (Learnings; P2-6 ~2310, R10.2).
    The byte-identical `6d216d19` blob is not checked for
    `env_remove`. Add a provenance check (d2): either the blob strips the
    variable, or its absence from the FL and F54 run environment is
    recorded.

**P3 (17, summarized).**

* **Rust (6):**
  * the IC "no failure" bullet (~1666) has no inline note;
  * W3 helpers need W8's `dead_code` rule (with Scope and Learnings);
  * the FL blob's own attributes versus the P2-5 ban (limit the ban to
    added lines);
  * P2-6 should strip all four `ENGRAM_*`/`CARGO_BIN_EXE_engram`
    variables;
  * P2-10 needs `tokio::time::sleep` and exact busy/locked markers (with
    Learnings);
  * W6's `crates/engram-indexer/src/preflight.rs` already exists.
* **Scope (3):**
  * owned-file counts go over 3 for PRE-2, PRE-3, PRE-5 and PRE-6;
  * "about 400 lines" in the HC bound is not a hard number;
  * W1's downstream item IDs are not listed.
* **Architecture (4):**
  * the FL halt leaves S5 `active` and P-001 blocked (name the operator
    options and a halt token; with Constitution);
  * the gate table's IC row if PA-5 is empty or docs-only;
  * G6 should filter by `artifact_type`, not by the "T" suffix;
  * `142.059-T`'s verification plan has no exact commands or pass
    criteria.
* **Learnings (4):**
  * `idle_timeout_minutes` has no parser found in `src/config`;
  * there is no full-suite runtime budget;
  * plan-detail churn: defer plumbing nits to the harness-architect
    (`in-plan-task-granularity-splitting-cascades-review-churn-...`);
  * assembly backlog changes must reach `main` before the S2 and S5
    claims (clean-worktree gate).

The attempt-8 P3s on the oracle at HC and the ruleset check stay open.

**Confirmed sound** (no finding):

* E3, E4, E6 and E7 match "S5 after S4". E4 removes the only S3→S5
  dependency.
* S5 membership is exactly what must travel with `142.058-T`.
* The W6 reset now appears in assembly item 3 and the S2 claim gate
  (completion edit 1).
* W2 and W7 (Rust module rules).
* G1's timing against condition 4's wording.
* The FL halt path meets Constitution VII.
* The P2-4 hold and P-014 split.
* HC restated item 5.

**Edge list: unchanged.** Assembly applies E5, E1, E2, E3, E4, E8, with
a cycle check after each and E4 before E8. The PA-5 harvest applies E6
and E7. No edge was applied.

**Disposition: STOP.** Attempt 9 is FAIL on P1-1 to P1-4. Under the
operator's bound, there is no Revision 11 and no assembly. S1-S5 are not
created, no hold comment was written, and no reset was made.

* **Circuit breaker.** Attempts 5-9 are five consecutive FAILs. The
  P-013.6 escalation ran after attempt 7
  (`docs/decisions/2026-09-28-142-f-escalation-review.md`). Stage does
  not start a second escalation, because this session is bounded to one
  review. The payload for the operator follows.
  * Threshold: 5 consecutive plan-review FAILs.
  * Failure: P1-1 (Ship Step 0.5 item 1a, versus `active` S3 and S5
    tasks), P1-2 (W1 visibility), P1-3 (P2-9 versus P-021), and P1-4
    (G5 versus Constitution II).
  * Artifacts: this plan (Revision 10 + Attempt 9) and
    `docs/memory/2026-09-29-stage-142-f-rev10.md`.
  * Resumption: an operator decision on three points:
    * the status resets (P1-1 and P2-10);
    * G5 route (a) or (b) (P1-4);
    * whether to defer S1/S2 plumbing detail (W1-W4, W8) to the
      harness-architect under HALT-to-Stage rules instead of pinning it
      in the plan (Learnings P3).

    Then an authorized Revision 11 limited to P1-1 to P1-4 (and, at Stage
    discretion, P2-1 to P2-11), then one scoped review.
* Stage does not re-run the review.

The review changed no backlog item, edge, shipment, stash entry, source,
test, `Cargo.toml`, or config file. It created and pushed no tag, and
made no git mutation. The only edits to this plan are the two completion
edits in R10.4 and this record.

<!-- plan-review-attempt: 9 -->

### Attempt 10: FAIL

**Authorization.** Operator, 2026-09-29, verbatim: 18:14 -07:00 "approve
resets, route (a), delegate layout, Revision 11"; 18:47 -07:00 "route
(c), review Revision 11". Scope: a completeness check of Revision 11
(R11.1-R11.10, ~2547-3419) and its "Revision 11 note" lines, then ONE
review scoped to Revision 11 and the text it touches. Any P0/P1 stops the
work: no Revision 12, no assembly. Ship contract read:
`.github/agents/_ship.agent.md` Step 0.5 item 1a (164), Step 2 (309-330),
Step 4.3 (442-465); `build-feature` SKILL 211, 250; `harness-architect`
SKILL Steps 4-6 (~184-266).

**Completeness check (before the review): complete, with two gaps.**
Every attempt-9 P1 has a disposition (P1-1 R11.2; P1-2 R11.4 LD2; P1-3
R11.5; P1-4 R11.10), and every P2-1..P2-11 and P3 has a row in R11.6.
Both operator decisions are recorded with timestamps (R11.1 18:14; R11.10
18:47). The resets cover `142.054-T`..`142.059-T`; CG-Q ("every task
member is `queued`") is in every claim gate (R11.7); R11.9 carries the
resets (step 3) and the FL item text (step 5); edges are unchanged with
apply order E5, E1, E2, E3, E4, E8 (cycle check each, E4 before E8).
Stage made two minimal completion edits:

1. R11.9 step 8 now says the Constitution Check gets the R11.8 addition
   with its "II Test-first" bullet replaced by the R11.10 Constitution II
   paragraph (the R11.8 bullet still named (a-2), H-R11-1 and CG-H).
2. The R10.1 "Revision 11 note" now reads "R11.3's route (a) forms, as
   amended by R11.10 (route (c) replaces (a-2))".

**Personas.**

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust Reviewer | FAIL | 0 | 2 | 4 | 5 |
| Scope Boundary Auditor | FAIL | 0 | 1 | 3 | 4 |
| Architecture Strategist | ADVISORY | 0 | 0 | 4 | 6 |
| Constitution Reviewer | ADVISORY | 0 | 0 | 0 | 9 |
| Learnings Researcher | ADVISORY | 0 | 0 | 6 | 3 |

After deduplication: **2 P1, 9 P2, 20 P3.** Stage confirmed both P1s
against the item text (`.backlogit/queue/142.062-T.md` line 28,
`142.067-T` line 30, `142.068-T` lines 23 and 52, `142.069-T` line 28,
`142.070-T` line 29, `142.072-T` line 30) and `Cargo.toml` (the root
package has no dependency on `engram-indexer`).

**P1 (proposed fixes are for a later, operator-authorized revision; NOT
applied).**

* **P1-1: the LD4 union baseline and LD2 block the S1 gates of PRE-1 and
  PRE-5** (Rust, Scope; plan LD2 ~2885-2900, LD4 ~2915-2927, R11.9 step 5
  ~3226). R11.4 withdraws W1/W2's item-text fixes, and R11.9 step 5 adds
  only a pointer line. So the item text still declares:
  * `142.062-T` (PRE-2) owns `src/services/generations/candidate_build.rs`,
    which `142.061-T` (PRE-1) creates and implements;
  * `142.068-T` (PRE-6) owns `src/preflight_probe.rs` "only", which
    `142.067-T` (PRE-5) creates and implements.

  LD2 forbids an implementation from editing a file that another pending
  task declares, and LD4 baselines the **union** of item-text and record
  files ("an item-text file the architect no longer uses stays
  unchanged" is false when another task owns it). So PRE-1's and PRE-5's
  implementations are either a HALT (LD2) or fail Ship Step 4.3 condition
  4 for the mapped PRE-2/PRE-6 tests. **Fix:** at assembly, narrow the
  Owned files of `142.062-T` and `142.068-T` (as W9 does for F52), and
  re-point PRE-2's "only `starts_with` in `candidate_build.rs`" check;
  or make the LD4 union exclude an item-text file that the record assigns
  to another task, and say LD2's "declares" means the record.
* **P1-2: cross-task `#[path]` includes of production files under the
  `allow` ban** (Rust; R11.6 Rust P3 row ~3013, R11.10 "LD3 includes"
  ~3380-3384). `142.069-T`, `142.070-T` and `142.072-T` still prescribe
  `#[path]` includes of `preflight.rs` (F50), `preflight_verdict.rs` and
  `preflight_invocation.rs` with `#[allow(dead_code)]`, which operator
  decision 2 bans, and R11.9 step 5 does not remove it. Without `allow`,
  the including root test crate must use every included item. Under
  route (c) those tests are frozen at Step 2, but F50 and NEW-1 change
  the included files afterwards: any new or unused item fails
  `-Dwarnings` in the later target, so the earlier gate fails condition 2
  and no one may edit the test. R11.10's rule that an included helper is
  "final at Step 2" covers test helpers only, and R11.6's "prefer the
  crate's public API" has no path, because the root crate does not depend
  on `engram-indexer`. **Fix:** ban cross-task includes of production
  files; test through a public API (for example an LD1 Step 2 root
  `[dev-dependencies] engram-indexer = { path = ... }` line, or tests
  inside `crates/engram-indexer/tests/` with a crate-local `harness_cmd`
  as `142.071-T` does), or HALT at Step 2. Remove the `allow` clauses from
  the three item texts in R11.9 step 5.

**P2 (deduplicated).**

1. **Earlier-task public signatures are not frozen** (Rust, Architecture,
   Learnings P3; ~3310). A later test may name an earlier task's items; a
   changed signature breaks its compile at the earlier gate. Freeze every
   Step 2 `pub` item a later test names; a change is a HALT.
2. **Route (c) never reaches the harness-architect** (Scope,
   Architecture, Learnings; R11.9 step 5 ~3226, STATUS-RESET ~2639). The
   pointer line cites only R11.4; the reset comment cites R11.3; the
   subtask `142.064.001-ST` (line 13) still says Step 4.2 authors the test
   and stanza. Make the pointer and comment cite R11.10 (ordering rule,
   markers, L1-L4, F1-F3, no `allow`) as overriding item-text harness
   instructions, and extend step 5 to the S1/S2 subtasks.
3. **Route (c) feasibility is deferred past the claim** (Rust,
   Architecture; R11.10 ~3330-3345). PRE-4 (`142.066-T`) has no own `pub`
   stub to call first and will likely hit F3; F2/F3 have no halt token or
   operator options (S1/S2 left `active`, P-001 blocked). Add a read-only
   feasibility pass before assembly step 6, and name the token and
   options.
4. **L3 recipe defects** (Rust, Learnings; ~3372). `let _ = (x);` fires
   `unused_parens` for one parameter; `unimplemented!` with a message is
   not usable in a `const fn` (use `panic!`); allow `#[must_use]` for
   `must_use_candidate`.
5. **LD2 visibility options** (Rust, Scope; ~2893-2900). A child module
   that alone reads a private field trips "field is never read" at the
   earlier gate; a new `pub` accessor is API outside the ACs. Require
   `pub mod` and fields read by the earlier task's own code, or treat an
   accessor as an item-text change (HALT).
6. **LD4 file-count rule has no enforcer** (Scope; ~2915). Name the
   Step 2 record as the checker and what it counts.
7. **Assembly re-check versus H2** (Architecture; R11.9 steps 1-2). H2
   changes 142-S, but step 2 halts on any change. State the expected
   post-H2 state and exempt it.
8. **CG-M and the index sync** (Learnings; R11.2 CG-M, R11.9 step 10).
   Backlog state lands through a staging PR (`detect-direct-push.yml`),
   not a direct commit to `main`; a sync over a stale cache can restore
   `active` (`backlogit-sync-cache-union-landmine-2026-07-02.md`). Reword
   CG-M; re-run the step 9 CG-Q read-back after any sync.
9. **Step 6 and the five `active` subtasks** (Learnings, Architecture
   P3; R11.2 ~2645, R11.7 S5). `shipment ship` expands manifest parents'
   children (`backlogit-ship-blocked-child-expansion-2026-04-26.md`), and
   no owner moves the subtasks to `done`. Make G6 close each subtask
   before its parent, and name the manual safe-close for S2/S5.

**P3 (20, summarized).** Rust: "first call" means first awaited/polled;
"after the placeholder, any code" should read own, earlier or base-tree
code; L4 should name Ship's exact default-feature clippy command
(`cargo lint` uses `--all-features`) and `-p engram-indexer` checks; the
placeholder must panic in the test's own process; `doc_markdown`.
Scope: list the F51 "must be rewritten by F52" and F52 Guardrail-4 edits
in step 5; R11.6's P3 fixes are Stage's choice; glossary rows are stale
after R11.10. Architecture: `142.060-T` stale "142-S" text; `142.054-T`
to `142.057-T` item text still says "red phase CONFIRMED"; (a-1)/(t)
assertion markers versus harness-architect Step 5.2 wording (Ship
condition 3 governs); P-001 block lacks operator acknowledgment; stale
R11.8 II bullet and scope-limit lines inline. Constitution: record the
route (c) residual as a standing, operator-accepted limitation; "HALT,
not re-label" is attempt-9's paraphrase, not the escalation review's
text; "no 142-F task is `active`" omits the five subtasks; limit "not a
P-021 finding" to claim/Step 2 halts; check `142.059-T`'s docs paths are
unchanged on the S5 base; T0 re-apply carries tests and stubs only; name
the subtask-member rejected alternative. Learnings: plan-detail churn
(defer further plumbing to the architect); the compound path
`2026-05-07-backlogit-shipment-status-constraints.md` does not exist.

**Confirmed sound** (no finding):

* After the six resets, Ship Step 0.5 item 1a passes for S1-S5; `142-F`
  is excluded by `artifact_type`; no subtask is a member.
* A reset `queued` task keeping `harness-ready` with off-base evidence is
  routed to the harness-architect by Step 2 item 4, which may repair it
  and leave exactly `harness-ready`.
* Route (c) needs no Ship, build-feature or harness-architect change. No
  test is edited after Step 2, and with the ordering rule each later
  test's `Worker:` marker stays stable through every earlier gate
  (conditions 3-5). It matches harness-architect Step 5.2's stub-panic
  RED, and it satisfies Constitution II as written; H-R11-1 is correctly
  closed.
* The W9 F52 AC4 → F51 transfer is a legitimate ownership transfer (F51
  owns `start_launcher_test.rs`; its AC2 already covers removing
  fail-open; RS4/RS5 unchanged).
* R11.5 adds no rule to Ship Step 4.3/4.4a (P-021 governs; no re-run,
  flake path or waiver).
* The PA-6 hold on S3/S4/S5 is enforced (claim gates; S5 via "S4
  shipped"; R11.9 step 7).
* The resets are operator-approved and applied by Stage at assembly, not
  auto-repaired by Ship; VII and the FL rules are unchanged.

**Edge list: unchanged.** Assembly applies E5, E1, E2, E3, E4, E8, with a
cycle check after each and E4 before E8. The PA-5 harvest applies E6 and
E7. No edge was applied.

**Disposition: STOP (FAIL on P1-1 and P1-2).** Under the operator's
bound, there is no Revision 12 and no assembly. S1-S5 are not created;
no reset, hold comment, or item-text edit was made.

* **Circuit breaker.** Attempts 5-10 are six consecutive FAILs. The
  P-013.6 escalation ran after attempt 7
  (`docs/decisions/2026-09-28-142-f-escalation-review.md`). Stage does
  not start another; this returns to the operator.
  * Failure: P1-1 (LD4/LD2 versus the item-text owned files of
    `142.062-T` and `142.068-T`), P1-2 (production-file `#[path]`
    includes under the `allow` ban).
  * Both P1s are narrow assembly item-text/constraint fixes; neither
    reopens route (c), the resets, or P-021.
  * Resumption: an operator decision to authorize a Revision 12 limited
    to P1-1, P1-2 (and, at Stage discretion, P2-1 to P2-9), then one
    scoped review.

The review changed no backlog item, edge, shipment, stash entry, source,
test, `Cargo.toml`, or config file, and made no git mutation. The only
plan edits are the two completion edits above and this record.

<!-- plan-review-attempt: 10 -->

### Attempt 11: FAIL

**Authorization.** Operator, 2026-09-29, verbatim: 18:14 -07:00 "approve
resets, route (a), delegate layout, Revision 11"; 18:47 -07:00 "route
(c), review Revision 11"; 20:04 -07:00 "Revision 12, then review". Scope:
a completeness check of Revision 12 (~3452-3755) and its "Revision 12
note" lines (2656, 2687, 2946, 3069, 3224, 3407), then ONE fresh-session
scoped review. Any P0/P1 stops the work: no Revision 13, no assembly.

**Completeness check: complete; no completion edits.** Attempt-10 P1-1
(R12.2), P1-2 (R12.3), P2-1 (R12.4), P2-2 (R12.5), P2-8 (R12.6) and P2-3
to P2-7, P2-9 and all P3 rows (R12.8) have dispositions. The 20:04
decision is verbatim with its timestamp (R12.1). Frontmatter is
`revision: 12`. A1-A19 cite task-file lines; the Scope persona matched
every quoted "From" text exactly. All six tokens are defined
(`R12-OWNERSHIP-HALT` 3492, `R12-TEST-PATH-HALT` 3567, `R12-SIGNATURE-HALT`
3609, `R12-ROUTE-C-HALT` and `R12-LAYOUT-HALT` in R12.8, and
`R12-RESET-REVERTED` 3672). No stale cross-reference was found.

**Personas.**

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust Reviewer | FAIL | 0 | 1 | 6 | 2 |
| Scope Boundary Auditor | FAIL | 0 | 1 | 0 | 5 |
| Architecture Strategist | FAIL | 0 | 1 | 2 | 3 |
| Constitution Reviewer | FAIL | 0 | 1 | 1 | 3 |
| Learnings Researcher | FAIL (medium confidence) | 0 | 1 | 3 | 4 |

After deduplication and Stage verification: **4 P1, 7 P2, 13 P3.** The
Scope P1 (`142.063-T`) was verified but downgraded to P2: it fires only at
the PA-6-held S4 claim and doesn't block assembly, S1 or S2.

**P1 (verified by Stage; proposed fixes NOT applied).**

* **P1-1: the A11 stage-set test can't prove equality, so `142.072-T` is
  certain to halt at the S2 claim** (Rust; plan A11 ~3589;
  `.backlogit/queue/142.072-T.md` lines 26 and 30). Line 26 names only four
  functions, so the relay's closed stage set isn't public. And for `Build`,
  an accepted `Failed` line gives the same result as the fallback
  (`Failed`/`Build`/1). Accepting `stage_name(f)` through
  `normalize_verdict` shows only one direction (stage_name ⊆ relay set).
  So A11's own `R12-TEST-PATH-HALT 142.072-T` clause will fire.
  *Fix:* at assembly, add an item-text constant to the 072 scope (for
  example `pub const RELAY_STAGES: [&str; N]` in
  `src/cli/commands/preflight.rs`) that `normalize_verdict` must use. The
  crate-local test checks two-way set equality against `stage_name`, using
  an exhaustive, wildcard-free `match` over `Failure`. Keep the
  `normalize_verdict` acceptance check for the non-`Build` stages. Freeze
  the constant under R12.4.
* **P1-2: no gate, final readiness run or CI job runs the crate-local
  tests, and R12 records no deviation** (Constitution P1; Rust, Architecture
  and Learnings P2). The evidence:
  * The root `Cargo.toml` lines 1-3 declare a workspace plus a root
    package, with no `default-members`.
  * `.cargo/config.toml` line 20 is `dev-test = "test --all-targets"`, and
    Ship Step 4.3 (`_ship.agent.md` 447-449) and CI (`ci.yml` 81 and 91)
    run only at the root. So they cover the `engram` package only.
  * Constitution line 22 says all tests "MUST … pass via `cargo dev-test`
    before any code is merged."
  
  After R12.3, the 069, 070 and 072 tests run only through their own
  `harness_cmd`. A later S2 task can break them, or break an R12.4 frozen
  item, and no gate or the S2 PR readiness run will notice. R12.3's
  `-Dwarnings` reasoning (~3556-3560) assumes a gate that never runs them.
  *Fix:* add assembly edit A20, putting these into item text and the Step 2
  record (no Ship change):
  * `cargo test -p engram-indexer --all-targets --no-fail-fast` must run
    at every later S2 task's gate and in the `142.073-T` final acceptance.
    Each failure must be either GREEN or mapped as pending RED.
  * `cargo clippy -p engram-indexer --all-targets -- -D warnings -D
    clippy::pedantic` must be clean at the same points.
  * `cargo check -p engram-indexer --all-targets` must succeed at Step 2.
  * Record a Constitution Check deviation for the crate-local tier
    location, citing the rejected option (a).
* **P1-3: the P2-9 subtask safe-close can't be done for S1 or S2**
  (Architecture; plan R12.8 P2-9 ~3714; walkthrough 3091 and 3102;
  `_ship.agent.md` 452-459). The rule requires the parent's Step 4.3
  verdict to be `PASS`. But `142.064-T` (in the middle of S1) and
  `142.054-T` (first in S2) are non-final tasks, and later root-crate
  route (c) tests are still RED when they run. The walkthrough itself
  expects them to get `EXPECTED_PENDING_RED`. So the precondition never
  holds, and S1 and S2 Step 6 halt with no way forward. The rule also names
  no commit path and no evidence (Constitution P2; Learnings P2-3).
  *Fix:* change the precondition to "the parent is `done` (Ship Step 4.5)
  with a recorded `PASS` or a verified `EXPECTED_PENDING_RED`". Set the
  window to after the shipment PR merges and before Step 6 safe-close.
  Stage makes each move with `backlogit move`, commits it on a
  `chore/stage-142-f-subtask-close-<shipment>` staging PR (CG-M), and adds
  a history note citing the parent's gate record and commit SHA. Ship
  resumes Step 6 after that PR merges.
* **P1-4: assembly step 10 and CG-S call for a plain index sync, which
  `docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md` says
  will bring back stale rows** (Learnings; plan R12.6 ~3664-3674, R12.7
  step 10 ~3688-3693; compound doc lines 35-39, 41-58 and 64-68). The
  operator's `main` cache still holds `142.054-T` to `142.059-T` as
  `active`, from before the resets. A sync on top of that cache is exactly
  the case the doc warns about. The single re-apply that CG-S allows uses
  the same plain sync, so a second revert, and an operator halt at step
  10, is predictable.
  *Fix:* in step 10 and in CG-S's re-apply, replace "sync" with the doc's
  recipe:
  1. Stop stale `backlogit` MCP processes by PID.
  2. Delete `.backlogit/backlogit.db`, `-wal` and `-shm`.
  3. Then run `backlogit sync`.
  
  Also forbid reflexive syncs after CLI mutations, and never stage
  `backlogit.db*` on a staging branch.

**P2 (one line each; proposed, not applied).**

1. `142.063-T` line 25 calls `tests/contract/read_server_cli_mcp_parity_test.rs`
   an "owned file", but `142.058-T` owns it (058 line 53; 063 line 30 is the
   invariant-6/PA-1 exception). R12.2's "only O1 violations" claim is
   incorrect, and O4 will halt S4 at Step 2. Add an O-rule exception for
   invariant-6/PA-1 files and for the W9 overlap, or reword 063 at the S4
   hold lift (Scope, downgraded from P1).
2. `142.070-T` lines 23-25: the tests need `Invocation` workspace access, a
   `SetupError` variant (`Layout` appears only in 071) and `pub` fields on
   `PreflightOutput`. R12.3 allows only `pub` on item-text functions.
   Extend the allowance to `pub` fields and variants of item-text types,
   and freeze them under R12.4 (Rust).
3. The R12.4 freeze list misses derives and trait impls the tests rely on,
   trait method sets the tests implement (`Verifier`), `const`-ness, and
   `#[non_exhaustive]` (Rust).
4. The R12.8 P2-4 stub recipe doesn't fit `engram-indexer`, which has no
   crate-level `allow`s (`crates/engram-indexer/src/lib.rs` 1-3, unlike
   `src/lib.rs` 11-19):
   * `#[must_use]` is required where `must_use_candidate` fires, and
     forbidden on `Result` returns (`double_must_use`).
   * `panic!` stubs need `# Panics`, and `Result` stubs need `# Errors`.
   * A single non-`Copy` parameter needs `let _ = (x,);` to avoid
     `needless_pass_by_value`.
   
   (Rust, Learnings.)
5. Test helpers shared through `#[path]` (P3 of R12.3) produce
   `dead_code` in any including crate that leaves a helper unused, and no
   `allow` is permitted. Require every includer to use every helper item,
   or inline the helpers (Rust, Learnings).
6. The F50 own-file `#[path]` exception (`tests/integration/preflight_gate_test.rs`
   lines 5-6) is lint-clean today but has no exit if F50's additions break
   it. Let F50 move to crate-local (b) instead of halting (Rust).
7. O3 plus Ship condition 4: item text still lists the LD1 registration
   files as owned (067 `src/lib.rs`, 072 `src/cli/commands/mod.rs`, 070 the
   indexer `lib.rs`). State that the baseline includes them as they stand
   after Step 2, and that no task edits a registration file after Step 2
   (Architecture).

**P3 (summarized).**
* Scope: the R12 header understates what it closes (R12.4 and several P2
  "Fixed" rows); A16 is counted under both step 3 and step 5; the
  "Append" wording in A12/A13 yields ".;"; A8 writes a line number into
  item text.
* Architecture: CG-M should also require local `main` to equal
  `origin/main` before Ship runs. `detect-direct-push.yml` only warns.
  A15 should explicitly override the harness-architect skill's
  `unimplemented!` marker and its `tests/integration|contract` placement.
* Rust: R12.3 says "F50 or NEW-1 add to `preflight.rs`", but NEW-1 owns
  only `preflight_verdict.rs`. `142.069-T` line 36 Scenario 3 names the
  F52/F53 (S3) items.
* Constitution: record the CG-S re-apply and its cause (investigate
  first). Give Q2 an "answer before the S2 PR merges" deadline.
* Learnings: L4 should add `--all-features` or note that the crate has no
  features. Merge the staging PR without `--admin`.

**Confirmed sound.**
* R12.3 mechanism (b):
  * `crates/engram-indexer/Cargo.toml` line 15 is a normal `engram`
    dependency, and there is no `autotests = false`.
  * The paths are `pub`: `pub mod preflight;`, `src/lib.rs` 36 and
    `src/cli/mod.rs` 7.
  * `Failure`, `Verifier`, `Preflight` and `StageKind` are `pub`, and
    `Failure` isn't `#[non_exhaustive]`.
  * Option (a) is correctly rejected as a dev-dependency cycle.
* All 19 "From" quotes in A1-A19 match exactly. The A14 grep is correct,
  and `142.064.002-ST`, `.003-ST` and `142.054.001-ST` contain no
  test-authoring text.
* A1-A5 resolve condition 4 for PRE-2 and PRE-6. The claimed LD1 shared
  lines are correct.
* The A15 routing reaches the harness-architect, because the reset tasks
  carry no route label. It matches build-feature 211 and 250. The form
  `harness_cmd` `cargo test -p engram-indexer --test <name>` is valid.
* `panic!("literal")` in a `const fn` is stable, and
  `unused_crate_dependencies` is allowed by default.
* R12.6 CG-M and CG-S don't contradict Ship Step 0.5 or Step 2. The
  CG-S re-apply is covered by the 18:14 "approve resets" decision.
  Q2 describes P-001 working as designed, and leaving it open doesn't
  block assembly.
* R12 stays within the attempt-10 resumption bound ("P2-1 to P2-9 at Stage
  discretion"). Nothing is scope creep.

**Disposition: STOP (FAIL on P1-1 to P1-4).** Under the operator's bound,
there is no Revision 13 and no assembly.
* **Circuit breaker.** Attempts 5-11 are seven consecutive FAILs. This
  returns to the operator.
* All four P1s are narrow text fixes (an item-text constant, an
  acceptance-command addition, a precondition reword, and a sync recipe).
  None reopens route (c), the resets, P-021, the edges, S1-S5 membership,
  or the PA-6 hold.
* Resumption: an operator decision.

The review changed no backlog item, edge, shipment, stash entry, source,
test, `Cargo.toml` or config file, and made no git mutation. The only plan
edit is this record.

<!-- plan-review-attempt: 11 -->

### Attempt 12: FAIL

**Authorization.** Operator, 2026-09-29, verbatim: 18:14 -07:00 "approve
resets, route (a), delegate layout, Revision 11"; 18:47 -07:00 "route
(c), review Revision 11"; 20:04 -07:00 "Revision 12, then review"; 21:00
-07:00 "Revision 13 (P1 fixes only), approve the cache rebuild, then
review. Q1: I think Stage should check first with read-only pass. Q2:
Yes. Also, I don't understand why a subtask cannot be marked as done as a
constituent of a parent task. Why would that not be allowed?" Scope: a
completeness check of Revision 13 (3770-4093) and its "Revision 13 note"
lines (3074, 3684, 3707, 3728), then ONE scoped review. Any P0/P1 stops
the work: no Revision 14, no assembly, no cache rebuild. Q3 and Q4 were
assessed, not decided.

**Completeness check: complete; no completion edits.**
* Attempt-11 P1-1 to P1-4 map to R13.2 to R13.5. P2s and P3s are deferred
  under D1 (R13.8).
* The 21:00 decision is verbatim with its timestamp (R13.1). Frontmatter
  is `revision: 13`.
* A11 (revised), A20, A21, A22 and the A15 amendment cite task-file lines,
  and the Scope and Rust personas matched every cited line and "From"
  text. R13.9 covers steps 1, 5, 6, 8 and 10 and R11.7.
* R13.5 records the rebuild as ProposedAction, ActionRisk `destructive`,
  approval verbatim, ActionResult `approved` (not applied), and rollback.
* No stale cross-reference was found. R13.4's compound citation resolves
  to `docs/compound/workflow-issues/backlogit-ship-blocked-child-expansion-2026-04-26.md`.

**Personas.**

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust Reviewer | ADVISORY | 0 | 0 | 5 | 3 |
| Scope Boundary Auditor | ADVISORY | 0 | 0 | 1 | 4 |
| Architecture Strategist | FAIL | 0 | 1 | 4 | 2 |
| Constitution Reviewer | FAIL | 0 | 1 | 3 | 3 |
| Learnings Researcher (medium-high) | ADVISORY | 0 | 0 | 4 | 3 |

After deduplication and Stage verification: **1 P1, 12 P2, 10 P3.** Stage
downgraded the Constitution P1 to P2 (P2-1): its facts hold, but R13.3
applies attempt 11's proposed fix as written, and records the deviation
in the form the Governance section asks for (principle, justification,
rejected alternative (a); constitution lines 286-292).

**P1 (verified by Stage; proposed fix NOT applied).**

* **P1-1: R13.4's "after the merge, before Ship's Step 6 closure"
  ordering can't be carried out, and "needs no Ship change" is wrong**
  (Architecture; plan ~3918-3937).
  * **No pause point.** The Merge Confirmation Gate ends "Proceed to Step
    6.0 only after both checks pass" (`_ship.agent.md` ~758). Step 6.0
    then goes straight to `git checkout main`, `git pull` and a
    `post-merge/<slug>` branch, and does all closure on it (~778-786).
    Nothing waits for an outside PR, so "Ship resumes Step 6" has no hook.
  * **The gate checks completion at the end.** The Release Closure
    Completion Gate (~761-767) counts Step 6.0 itself as required closure
    (item 1), so it's evaluated after safe-close. It has no step that
    reads a release-closure item recorded on the shipment. It can hold
    P-001 open for the *next* release unit, but it can't order Stage's PR
    before safe-close.
  * **Worktree.** A Stage `chore/` branch can't run in the single worktree
    (P-016) while Ship's post-merge branch holds it.
  * **Effect.** Ship closes first. For S5, safe-close archives `142-F` and
    `142.058-T` while `142.058.002-ST` and `.003-ST` are still `active`,
    which is the same no-owner gap as attempt-11 P1-3.
  * **What holds.** Subtasks carry no `shipment_id`, so reconcile won't
    flag them, and Stage moving non-member subtasks isn't a P-001 claim.

  *Fix (either one):*
  * **(a) Operator answers Q4 "yes".** A separate harness chore adds
    subtask moves to Ship's Role Boundary (line 38), Step 4.5 item 4
    (~601), and the Step 3 member filter. The template is updated upstream
    so regeneration keeps it. It must merge before the S1 claim.
  * **(b) Keep the fallback, but move it to *after* Ship's Step 6 closure
    PR merges.** Stage then runs the subtask-close PR. Enforce it with a
    Stage-owned claim-gate item: the S2 claim needs S1's subtask-close
    merged, and each later claim needs the one before it. After S5, add
    an operator checkpoint before any other release-unit claim. Drop the
    "release-closure item" and "Ship resumes Step 6" wording. At
    assembly, confirm read-only that `backlogit move <subtask> --status
    done` is accepted when the parent is archived and when the subtask
    is `queued` (`142.064.001-ST` to `.003-ST`).

**P2 (one line each; proposed, not applied).**

1. R13.3's residual leaves S3-S5 merges without the crate-local tests (Constitution rated P1; Stage downgraded). Extend the A20 lines to S5's code-task gates and final criteria, and to S3/S4 at the PA-6 hold lift. Record `--workspace` in `dev-test`/CI as a rejected alternative, with its reason. Cite the existing follow-up stash 7BF90213 (see `142.071-T`) instead of adding a new one.
2. A20's "recorded pending RED" allowance has no auditor, because Ship's six conditions (`_ship.agent.md` 452-459) cover only `cargo dev-test` output. Require the six conditions on the `-p engram-indexer` run, save its full `--no-fail-fast` output, and match the exact FAILED list against the recorded names and markers (Rust, Architecture, Constitution, Learnings).
3. The R13.5 rebuild isn't safe as written. Find every process holding `backlogit.db*`, including this session's own MCP server, and stop only PIDs whose command line or cwd is inside this workspace, recording them first. Name who runs the rebuild (Stage, under D2). Check with `Test-Path` that all three files are gone, and halt otherwise. After the sync, `git status --porcelain -- .backlogit/` must be empty. Limit the approval scope to step 10 and the single CG-S re-apply, and add targets and change_kind, plus a rule for when ActionResult becomes `applied` (Architecture, Learnings, Constitution).
4. The subtask-close PR changes `.backlogit/` Markdown on a branch. Require the R13.5 rebuild and a CG-S re-read after it merges and `main` is pulled, before the next Ship read (Architecture, Constitution).
5. R13.4's S5 P-015 cascade reasoning is wrong. The cascade needs every child of `142-F` to be an S5 member (`_ship.agent.md` ~867-876), and S1-S4 tasks aren't, so S5 always uses safe-close. Delete that reasoning, and if a cascade is ever used, apply the `shipment ship` time limit (`backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`) (Architecture, Learnings).
6. The in-repo `.github/skills/shipment-reconcile/SKILL.md` defines only `pre` and `post` modes, with no `safe-close` mode (Ship ~840-843 points to one). Confirm at assembly that the installed skill has it, or record the risk for S1-S5 Step 6 (Architecture).
7. A22 says `normalize_verdict` has "no other stage list", but it returns `(&'static str, i32)` (`142.072-T` line 26). A stage-name array can't give a `'static` full `Failed` line without a second table or a leak. Make the constant hold the seven full `Failed` lines, or reword A22 (Rust).
8. R13.6 has no rows for `142.071-T` (crate-local `CARGO_BIN_EXE` spawn test, line 27, which A20 now runs at the 054, 069 and 070 gates) or `142.073-T` (binary-spawn contract test, line 27). Record (a-1) with a recorded assertion marker, or F3 (Rust).
9. R13.6 misses `142.067-T` line 42: a constants-only unit test (`PREFLIGHT_READ.mcp_tool` in `GENERATION_PINNED_READS`) with no own call first. Add an F1 row (call `probe_cli_read` on a nonexistent exe first). This is required for Q3 (ii) (Rust).
10. PRE-3's F1 needs `read_server_gate` to be `pub`. Its sibling `generation_activator` is `pub(crate)` (`src/server/state.rs` ~1750). State `pub` in the row, and have the architect record it under R12.3 (Rust).
11. A22 is placed by line number only, and it shifts `142.072-T` lines 30, 35, 43 and 56 (A11, A12, A20, A13). State that every cited line refers to the file before step 5, that quoted text takes precedence, and put A22 right after the `normalize_verdict` bullet (Scope, Rust).
12. Subtask moves to `done`: record each current status and confirm the transition is allowed (`queued` → `done` for `142.064.001-ST` to `.003-ST`). Ban `--force-gates`, and send any gate failure to the operator. Expect queue→archive renames in the diff (Learnings, Scope, Architecture).

**P3 (summarized).**
* **Rust:**
  * Give the exhaustive `match` distinct arm bodies (`clippy::match_same_arms`).
  * `RELAY_STAGES` is 072's own item, so it's an own-item Step 2 exception, not an R12.4 freeze.
  * NEW-1's evidence should cite `crates/engram-indexer/src/lib.rs` and `pub trait Verifier`, not a `#[path]` include.
* **Scope:**
  * Use `[&str; N]` (7 expected) so the item text follows `stage_name`.
  * The R13.4 Rule's timing wording contradicts its mechanism.
  * Step 4.5 item 4 is at ~601.
  * The CG-S re-apply now only happens after another tool's sync.
* **Constitution:**
  * Name both SHAs in the subtask history note (the parent's implementation commit and the merge commit).
  * Take a status snapshot before the rebuild.
* **Learnings:**
  * R13.5 matches the doc's steps 2a-2c, not "steps 1-3".
  * Keep `dev-test` a native alias in the CI follow-up.
  * Note that the landmine doc overrides the 012-S post-move sync.
* **Stage:** R13.1 and R13.9 say the review is "not run by Stage", but the Orchestrator assigned attempt 12 to Stage.

**Q3 assessment (operator-pending; not decided).**
* **(i) Keep the plan and halt.** Sound: no code runs before its test, and the operator path is recorded (R12.8 P2-3). The cost: S1 will certainly halt at Step 2 after the claim, with P-001 blocking everything else. It only delays the decision.
* **(ii) Empty-const placeholder (Stage-recommended).** Sound, with conditions:
  * It compiles and lints cleanly. `&[&str] == [&str; 1]` is a std `PartialEq`, no pedantic lint fires on an empty-slice const, and `assertions_on_constants` covers only `assert!`.
  * It meets Constitution II's red-then-green.
  * **Condition 1:** the assertion carries an explicit marker message, such as `assert_eq!(GENERATION_PINNED_READS, ["get_workspace_statistics"], "Worker: 142.066-T GENERATION_PINNED_READS")`. Otherwise Ship condition 3 has no marker to match.
  * **Condition 2:** R12.4 freezes only the type, not the value.
  * **Condition 3:** the departure from R11.10 is recorded in the Constitution Check, and the Step 2 edit is in the owned-file baseline (condition 4).
  * **Condition 4:** P2-9 is fixed first. Also check that no PRE-3 test reads the constant.
* **(iii) Separate shipment.** Unsound as written. `142.067-T` depends on 066, `142.068-T` on 067, and `142.054-T` on 068, so moving only 066 breaks S1's dependency order. It is workable only as S1b = [066, 067, 068] between S1 and S2. That gives PRE-4 a true (a-1) RED with no R11.10 departure, but it changes S1/S2 membership, the claim gates, the step 6 manifests and the walkthrough, and adds one P-001 cycle. It needs re-review.
* **Consensus:** (ii) with its conditions. The fallback is (iii) as S1b.

**Q4 assessment (operator-pending; the fallback is the plan of record).**
* **For:** Architecture recommends "yes". It puts the subtask moves in the same commit as the parent's `done`, removes the staging PR and the P1-1 ordering problem, and keeps S5 from archiving `142-F` over active subtasks. Learnings adds that it avoids needing gate evidence after the merge (`post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`).
* **Against:** Scope and Learnings note that it's a harness-rule change outside 142-F, and more than one line (Role Boundary line 38, Step 4.5 item 4, the Step 3 `T`-suffix filter). It needs its own chore, with the template updated upstream, merged before the S1 claim (P-001).
* **Result:** answering "yes" is the cleaner way to close P1-1. Otherwise, fix (b) is required.

**Confirmed sound.**
* **R13.2:**
  * `Failure` has 7 unit variants and is `pub` and `Copy`, not `#[non_exhaustive]`, so a test can build every variant.
  * "Seven stages" matches 069 lines 23 and 35.
  * Two-way equality plus the wildcard-free `match` proves equality, and `[&str; 7]` avoids `redundant_static_lifetimes`.
  * The constant is real API.
* **Assembly edits:** A11, A20, A21 and A22 cite the right lines, and every "From" text matches. Each A20 target is `<!-- END:acceptance-criteria -->`. 054, 069, 070 and 071 already carry the clippy line, while 072 and 073 don't. The A15 text matches.
* **R13.3:**
  * Its premise holds: root `Cargo.toml` 1-3 has no `default-members`, and `.cargo/config.toml` line 20 is `dev-test`.
  * The Step 2 `cargo check` and the S2 all-GREEN final criteria are correct, and option (a) was rightly rejected as a cycle.
  * `engram-indexer` has no crate-level `allow`s, so its clippy gate can fire.
* **R13.5 order:** stop by PID, delete the three files, then sync. That matches the landmine doc. The bans on reflexive syncs and on staging `backlogit.db*` hold (`.gitignore` 76-78). The approval is verbatim, the ActionResult is `approved` and the risk is `destructive`.
* **R13.6 verdicts:**
  * PRE-2 is F1: `GenerationStore::new` is `pub` and in the base tree.
  * PRE-3 is F1, given P2-10: `AppState::with_mode` is `pub` and synchronous.
  * PRE-4b is feasible: 065 line 24 owns the function.
  * PRE-5 is F1: `Spawn`.
  * PRE-6 is F1: the arguments are parsed before the spawn.
  * NEW-1, NEW-2 and NEW-4 are feasible.
  * PRE-4 is correctly F3, since a `const` can't panic.
* **R13.4 scope:** tying closure to the parent being `done` removes the attempt-11 deadlock. It stays inside P-010 and correctly declines to widen Ship's grant.
* **R13 as a whole:** it stayed P1-only, S1-S5 membership is unchanged, and no P2 was quietly fixed.

**Disposition: STOP (FAIL on P1-1).** Under the operator's bound, there is no Revision 14, no assembly and no cache rebuild.
* **Circuit breaker.** Attempts 5-12 are eight consecutive FAILs, so this returns to the operator.
* P1-1 is narrow. It closes either by answering Q4 "yes" (a separate harness chore) or by re-sequencing R13.4 after Ship's Step 6 (fix (b)). Neither reopens route (c), the resets, P-021, the edges, S1-S5 membership or the PA-6 hold.

The review changed no backlog item, edge, shipment, stash entry, source, test, `Cargo.toml` or config file, ran no build and no cache rebuild, and made no git mutation. The only plan edit is this record.

<!-- plan-review-attempt: 12 -->

### Attempt 13: FAIL

**Authorization.** Operator, verbatim: 2026-09-29 18:14 -07:00 "approve
resets, route (a), delegate layout, Revision 11"; 18:47 "route (c),
review Revision 11"; 20:04 "Revision 12, then review"; 21:00 "Revision 13
(P1 fixes only), approve the cache rebuild, then review. Q1: I think
Stage should check first with read-only pass. Q2: Yes ..."; 2026-09-30
17:35 -07:00 "Q4 no (fix b), Q3 (ii), Revision 14: the P1 plus P2-3,
P2-7, P2-9 and P2-10, then review." Scope: a completeness check of
Revision 14 (now lines ~4130-4525) and its "Revision 14 note" lines, then
ONE scoped review. Any P0/P1 stops the work: no Revision 15, no assembly,
no cache rebuild. Q5 was assessed, not decided.

**Completeness check: complete, with five stale-cross-reference edits.**
* Attempt-12 P1-1 maps to R14.2 (fix (b)). P2-3, P2-7, P2-9 and P2-10
  map to R14.4, R14.5, R14.6 and R14.7. P2-4 and P2-5 are closed as parts
  of fix (b). Every other P2 and all P3s are dispositioned (R14.8).
* The 17:35 decision is verbatim with its timestamp (R14.1). Frontmatter
  is `revision: 14`.
* A23, A24, A25, A26, A11 (rev.), A22 (rev.) and the A15 pointer cite
  task-file lines, and the Rust and Scope personas matched every cited
  line and "From" text. R14.9 covers steps 1, 2a, 5, 6, 8 and 10 and
  R11.7.
* R14.4 records the rebuild with ProposedAction, targets, change_kind,
  ActionRisk `destructive`, approval, ActionResult and rollback. Its
  approval scope is the P1 below.
* **Completion edits (citations only, no content change):**
  1. R14.2 "Why": `_ship.agent.md` "(784-791)" → "(784-795)"; the
     `post-merge/<slug>` branch is created at 792-795.
  2. R14.3: the A15 pointer "(R12.3/R13.3)" → "(R12.5/R13.3)"; A15 is
     defined in R12.5.
  3. R11.2 (~2662): inline "(Revision 14: `142-F` leaves the S5
     manifest ...; R14.2, Q5.)".
  4. R11.7 item 1a check (~3061): inline "(Revision 14: `142-F` is no
     longer an S5 member; R14.2.)".
  5. R11.7 S5 heading (~3143): inline "(Revision 14: `142-F` removed
     from S5 ...)".

**Personas.**

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust Reviewer | ADVISORY | 0 | 0 | 2 | 6 |
| Scope Boundary Auditor | ADVISORY | 0 | 0 | 4 | 7 |
| Architecture Strategist | ADVISORY | 0 | 0 | 4 | 3 |
| Constitution Reviewer | FAIL | 0 | 1 | 6 | 6 |
| Learnings Researcher (medium) | ADVISORY | 0 | 0 | 5 | 3 |

After deduplication and Stage verification: **1 P1, 17 P2, ~20 P3.** The
Scope auditor raised the P1's facts independently at P2. Stage verified
them and keeps them at P1, because the plan records a destructive action
as `approved` for runs that no operator approval covers.

**P1 (verified by Stage; proposed fix NOT applied).**

* **P1-1: R14.4 widens the destructive cache-rebuild approval while
  saying it narrows it** (Constitution VII and strict-safety; Scope).
  * **The approval's recorded scope.** The 21:00 approval ("approve the
    cache rebuild", D2) was recorded against R13.5, whose ProposedAction
    is "The rebuild above, at R11.9 step 10 and at any CG-S re-apply"
    (plan ~4021-4023). Attempt-12 P2-3, which the 17:35 decision selected
    for fixing, says "Limit the approval scope to step 10 and the single
    CG-S re-apply."
  * **What R14.4 does.** It adds run point 3, "the R14.2 step 6 re-read
    after each subtask-close PR merges (S1, S2 and S5)" (~4364-4370):
    three more runs that stop processes and delete files. Its Approval
    row still cites 21:00, "scoped by R14.4" (~4411), its ActionResult
    is `approved` (~4412), and the heading (~4354) and R14.1 "Then" say
    the approval is narrowed. Run point 3 comes from attempt-12 P2-4,
    which the 17:35 decision didn't select.
  * **Rule.** `strict-safety.instructions.md` line 53: "Require operator
    approval before any `ActionRisk: destructive` action. Proceeding
    without approval is a policy violation"; line 43: `planned` means
    "identified but not yet approved". As written, Stage would run three
    unapproved destructive actions believing them approved.
  * *Fix (any one, operator's choice):*
    * **(a)** The operator approves run point 3 explicitly. A Revision 15
      records it verbatim and changes "narrower"/"narrows" to "narrows
      run points 1-2 (one CG-S re-apply) and adds run point 3, approved
      <time>".
    * **(b)** Mark run point 3 `planned`, with an operator confirmation
      at each run (raised at R14.2 step 6), and correct the wording.
    * **(c)** Drop run point 3. R14.2 step 6 then re-reads with
      `backlogit get` against the Markdown on the pulled `main`, and any
      mismatch is `R14-CACHE-DIVERGED` to the operator (this reopens
      attempt-12 P2-4's concern, so record it as a residual risk).

**P2 (one line each; proposed, not applied).**

1. CG-T has no enforcement point: Ship doesn't read it (unlike CG-Q/CG-M), so name Stage as the evaluator at R14.2 step 6, record `CG-T PASS <next shipment>`, insert "Stage subtask-close Sn" into the S1-S5 handoff sequence, and optionally add Sn's subtask IDs as S(n+1) shipment `dependencies` so Ship Step 0.5 item 3a enforces it natively (Architecture, Constitution).
2. R14.2 timing preconditions aren't all observable or owned: Ship Step 6.0 doesn't delete the post-merge branch and Stage can't push, and `RELEASE_CLOSURE_INCOMPLETE` isn't visible to a later session; use artifacts (shipment archived `shipped`, closure PR MERGED and its SHA an ancestor of `origin/main`, P-020 status, one clean worktree) and make branch deletion non-blocking or operator-owned (Architecture).
3. Operator option (iii) (subtasks `archived` instead of `done`) deadlocks CG-T and CP-S5, which require `done`; drop it or amend both gates for exactly those IDs with a recorded deviation (Scope, Constitution).
4. `R14-142F-OPEN-CHILDREN` treats any non-`done` status as open, though the registry routes `accepted`/`rejected`/`archived` to archive too; define "open" as queued/active/blocked/review or still in `queue/`, and give operator exits for that halt and for a pending or declined CP-S5, since the P-001 hold has no other exit (Architecture, Scope, Constitution).
5. Step 2a lacks the case every subtask-bearing Step 6 creates (safe-close archiving a parent whose non-member subtasks stay open), and `.backlogit/archive/` has no precedent for subtask `queued`→`done`, a subtask closed after its parent was archived, or a feature `active`→`done` via `move`; expect 2a to halt, and pre-name option (i) or a `queued`→`active`→`done` path (Architecture, Learnings).
6. Guard against backlogit "child status rollup" (Stage verified `.backlogit/logs/087-F.jsonl` archived→active and `109-F.jsonl` done→blocked, reason "child status rollup"): after each move, confirm the parent and `142-F` keep status and folder and gained no rollup event, else HALT `R14-ROLLUP <id>` (Learnings).
7. R14.4 step 1's "in this workspace" accepts a parent-process match alone, which can stop another workspace's MCP server under the same host; require command line/`--cwd`/`handle.exe` evidence, match the root as a whole path token, HALT `R14-CACHE-HOLDER-AMBIGUOUS <pid>` on parent-only matches, and re-scan before step 5 (Constitution, Architecture, Learnings).
8. The rollback row's "Stopped MCP servers are restarted by their client" has no support; say Stage uses the CLI for the rest of the session and logs P-012 `TOOL_DEGRADED: backlogit MCP` (Learnings, Constitution).
9. ActionResult `failed` isn't a strict-safety state (line 39-48); a halted run is `blocked` (or `abandoned`), records partial effects (PIDs stopped, files deleted), and a CG-S/CG-T mismatch after a clean rebuild is a gate failure, not a rebuild failure (Constitution).
10. Option (i)'s scratch `.backlogit` copy has no location; put it in a gitignored path inside the workspace (Principle IV) and give its creation and deletion an action record (Constitution).
11. The R14.3 Constitution Check names Constitution II as the deviated principle, which a reviewer must treat as a NON-NEGOTIABLE halt; record II as complied with and list the R11.10 departures (non-final value; a `const`, not an L3 stub; marker from the assert message) (Constitution).
12. "The operator or Orchestrator pushes, opens and merges the PR": merging is Ship's grant, not the Orchestrator's; make it the operator (or Ship under P-014), require a merge commit (P-009) and record its SHA (Constitution).
13. R14.3 condition-4 evidence is wrong: `142.064-T` line 42 names `GENERATION_PINNED_READS` and `get_workspace_status`, not `get_workspace_statistics` (Stage verified); correct it and add an item-text edit or Step 2 note that the invariant-7 test lists methods by name and never reads the constant (Scope).
14. PRE-4's `assert_eq!(..., ["get_workspace_statistics"], ...)` pins the exact value, but `142.066-T` line 25 says more methods join later (86F93068), which would force editing PRE-4's tests; use `assert!(GENERATION_PINNED_READS.contains(&"get_workspace_statistics"), "Worker: 142.066-T GENERATION_PINNED_READS")`, which fails on `&[]` with the same marker (Rust).
15. R14.5's exhaustive wildcard-free `match` with identical arm bodies fires `clippy::match_same_arms` under `-D clippy::pedantic` with no allow permitted; require distinct arm bodies (e.g., each variant → its index 0..7, asserted) (Rust; upgrades attempt-12 P3).
16. Q5 is a membership change outside the 17:35 bound but has no assembly gate; require Q5 confirmation before R11.9 step 6 and HALT if declined (Scope).
17. Step 2a item 1 should also check the completion broker's `gate_report_hash` for an empty gate set (`8ac74679…bbea` in every archived subtask log) after Stage's first move, halting on a different hash (Learnings).

**P3 (summarized).**
* **Rust:** use "Q3(ii) cond. N" vs "Ship cond. N" labels in R14.3; name the shared RED test file `tests/integration/read_server_generation_wiring_test.rs` in 066's Step 2 baseline; A25 `set_read_server_gate` either `pub` with `#[doc(hidden)]` like `set_generation_activator` or `pub(crate)`; Step 2 should record a clean pedantic clippy run (possible `unused_async` on the getter placeholder); put the `Build` line at `RELAY_STAGES[0]`; say where the 067 line-42 test lives.
* **Scope:** R14.1 E3 says "every other ... deferred" but P2-4/P2-5/part of P2-12 are closed; P2-12's deferred remainder is unnamed; A15 applies to every S1 and S2 task, not only `142.066-T`; config cite is 149-168; R14.9's scope list omits the new R13.6 row; CG-T scope stated three ways (S1 vacuous vs S2-S5); enumerate archived `142-F` descendants at step 2a.
* **Architecture:** S1's CG-T bullet 1 is checkable now; commit the subtask moves before pausing for CP-S5; on Windows SQLite holds without `FILE_SHARE_DELETE`, so unidentified holders end in `R14-CACHE-LOCKED` (state it).
* **Constitution:** enter Careful mode at R14.4 step 0; bound/re-raise `R14-142F-CLOSE-PENDING`; record the `pub fn` accessor as a rejected alternative; delete the `chore/stage-142-f-subtask-close-*` branch after merge.
* **Learnings:** "subtasks aren't members" is true of S1-S5 only (the active 142-S manifest lists subtasks); after CP-S5 no `142-F` child may leave `done` without a new decision (rollup would reopen `142-F`); stray `.lock` sidecars are information only.

**Q5 assessment (operator-pending; plan of record "yes"): sound, confirm
before assembly step 6.**
* Ship Step 0.5 accepts task-only manifests and finds the feature through
  `parent_id`; S1-S4 are already task-only. With no feature member, the
  P-015 cascade can't apply, so every S1-S5 Step 6 is safe-close, which
  archives only manifest items (`_ship.agent.md` 846) and keeps `142-F`
  (protected covering feature) in the queue. This closes attempt-12's
  gap of `142-F` archived over open `142.058.002-ST`/`.003-ST`.
* Stage moving `142-F` to `done` is a backlog-item update within P-010,
  not a shipment close. Precedent is partial: chore `033-C` was closed by
  `move --status done` outside `shipment ship` (012-S), and eight
  shipments (134-S-141-S) closed by safe-close with `142-F` unchanged; no
  feature has been closed this way, so step 2a's feature case may halt
  (P2-5).
* P-001 while `142-F` stays active pending CP-S5 is real (Ship Step 1
  blocks other active release units) but bounded only by the operator;
  P2-4 adds the exits.

**Confirmed sound.**
* **R14.2:** ordering after Ship's Step 6 removes the nonexistent pause
  point; no `--force-gates`/`--gate-base`; no sync after moves; queue→
  archive renames expected (`registry.yaml` 1-8); history notes cite both
  SHAs; `pre_task_completion` is still commented out
  (`.autoharness/config.yaml` ~148-168); Stage only commits; one worktree
  on clean `main` (P-016); P-015 cascade reasoning withdrawn; S1-S5 cover
  every `142-F` child (S3/S4 held items must ship before S5 can be
  claimed).
* **R14.3:** `&[&str] == [&str; 1]` compiles (`impl PartialEq<[U; N]> for
  &[T]`); no lint fires (`eq_op`, `assertions_on_constants`,
  `const_is_empty` don't apply; must_use/errors/panics docs are allowed in
  `src/lib.rs` 12-15); the panic text carries the marker for Ship
  condition 3; `request_entry.rs` is owned only by 066; public path
  `src/lib.rs` 38 / `src/daemon/mod.rs` 14 is correct; only the type is
  frozen. It is a genuine red-then-green.
* **R14.4:** stop by PID only, `-LiteralPath -ErrorAction Stop`,
  `Test-Path`, sync, clean `git status -- .backlogit/` with no commit or
  revert; matches the landmine doc's steps 2a-2c; targets gitignored
  (`.gitignore` 76-78).
* **R14.5:** `RELAY_STAGES.iter().copied().find(..)` keeps `&'static
  str`; the line form matches `142.069-T` line 35; A11 (rev.)'s replaced
  phrase is verbatim in R13.2's A11.
* **R14.6:** F1 via `probe_cli_read` nonexistent exe → `Spawn` (067 line
  39); 067 depends on 066.
* **R14.7:** `ReadServerStartupGate` is `pub` in a `pub mod`; `state.rs`
  1745 `pub` / 1749 `pub(crate)` as cited; A25/A26 "From" texts are exact
  and unique.
* **Bound:** R14 stayed inside the 17:35 decision except run point 3
  (P1-1) and the flagged Q5; S1-S4 membership, edges, resets, P-021 and
  the PA-6 hold are unchanged.

**Disposition: STOP (FAIL on P1-1).** Under the operator's bound, there is
no Revision 15, no assembly and no cache rebuild.
* **Circuit breaker.** Attempts 5-13 are nine consecutive FAILs; this
  returns to the operator.
* P1-1 is closed by an operator decision on run point 3 ((a), (b) or
  (c)) plus a wording fix; no design change is needed.

The review changed no backlog item, edge, shipment, stash entry, source,
test, `Cargo.toml` or config file, ran no build and no cache rebuild, and
made no git mutation. Plan edits: the five citation fixes above and this
record.

<!-- plan-review-attempt: 13 -->
