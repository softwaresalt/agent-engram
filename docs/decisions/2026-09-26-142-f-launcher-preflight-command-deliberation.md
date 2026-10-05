---
title: "142-F launcher-callable preflight command and F50 gate-warning classification"
description: "Decides how start.ps1/start.sh reach the F50 typed preflight machine, and how the F51 transcript WARNING lines in the F50 full-suite run are classified under Ship Step 4.3"
topic: "Stash 03AA00A8, 49809128, 9B7EC1E4, 6C5DF765 (DEFERRED SCOPE EXPANSION) plus the 142-S F50 Step 4.3 and concrete-verifier blockers"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md"
  - "docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md"
  - "docs/memory/2026-09-26-ship-142-s-f50-f51-fixture-phase-correction.md"
  - "docs/memory/2026-09-26-ship-142-s-f50-concrete-verifier-api-block.md"
tags:
  - "142-F"
  - "preflight"
  - "launcher"
  - "p-021"
---

> **Status (2026-10-04, PR #410 review): decided; parts superseded.** D1-A, D2-A, D4-A and D5-A stand. Superseded:
> the invariant-6 ownership exception (retired by PA-7, 2026-09-29: PRE-3F owns its own test target, and only
> `142.058-T` and its split family edit the F54 file, after PA-5); PA-1's admission to 142-S and the S1-S5 split (replaced
> by one-task slots; 142-S is abandoned at H3); and the PA-6 hold (lifted 2026-10-04, OD-6). The current authority is
> `docs/exec-plans/2026-09-30-142-f-decomposition-plan.md` Revision 18 (frozen).

## Problem Frame

Active shipment 142-S is stalled at Ship Step 4.3 for F50 (`142.054-T`). Ship
reports three linked problems:

1. The single post-correction `cargo dev-test --no-fail-fast` run at
   `4995d681`'s tree has exactly 12 failures, all mapped to later-task RED
   harnesses. It also contains two `WARNING:` lines. Ship read the Step 4.3
   phrase "no ... warnings" literally and treated those lines as blocking.
2. Neither F50 nor F51 declares a launcher-callable preflight command or
   argument contract. F51 (`142.055-T`) must launch Copilot "only on F50
   `Succeeded`", but F50 is a library typestate.
3. The operator requires `start.ps1` to use the workspace
   `target\debug\engram.exe`, not PATH and not `C:\Tools`.

Constraints:

* P-021 C1/C4: nothing may be added retroactively to F50's active cycle.
* P-001/P-016: no parallel branch and no second active release unit.
* The operator approved only `142.060-T` for addition to the active 142-S
  manifest.
* Stage writes no source, test, or config; runs no build or test; makes no
  claim and creates no PR.

Success means Ship can keep moving now without breaking test-first or
P-021, and every remaining approval is named exactly.

## Research Findings

Evidence was gathered on 2026-09-26 against HEAD `4995d681`.

### F1: the two WARNING lines are part of a mapped RED test's panic payload

In the retained full-suite output (`1790410758567-copilot-tool-output-...txt`),
both `WARNING:` lines (4121 and 4124) sit inside the block
`---- typed_preflight_failure_is_reported_without_starting_copilot stdout ----`.
They follow directly after `panicked at tests\contract\start_launcher_test.rs:547:5:`
and the recorded marker `RED F51-FAIL-CLOSED: failed preflight must return a
failure status; {transcript}`. The assertion at lines 547–549 interpolates the
captured launcher stdout and stderr (`launcher_output`) into its panic message.
The lines are therefore bytes of that mapped failure's message.

Supporting facts:

* A case-insensitive scan finds no rustc, cargo, or clippy `warning:`
  diagnostic anywhere in the run. `.cargo/config.toml` sets
  `rustflags = ["-Dwarnings"]`, so a compiler warning would already have
  been a compile error.
* The only other `warning` matches are test names ending in `... ok` and
  JSON `"warnings":[]` fields inside F54 payloads.
* Ship already accepted `error: unrecognized subcommand 'doctor'` (line 2448)
  as content captured inside the F54 contract-test failure, not as a runner
  error. That is the same situation.
* The content is intrinsic to the RED state. The unmodified launcher runs
  its legacy fail-open `sync`→`bind` fallback, prints those two warnings,
  and exits 0. Exiting 0 is exactly the failure that `RED F51-FAIL-CLOSED`
  detects. F51 removes this path (acceptance criterion: "The previous
  15-second fail-open release path is removed"), and the lines go away with
  it.

### F2: F50 is incomplete against its own subtasks

`142.054.002-ST` covers Build/Seal/Publish against the F07/F08/F10 facade,
including "prior generation still serving after a failed publish".
`142.054.003-ST` covers DaemonVerified/HealthVerified/CliProbe/McpProbe,
including the "older revision does NOT reach DaemonVerified" and
"no control endpoint" assertions. Both subtasks are `active`, and both are
142-S manifest members.

`c269fa79` added only the typestate and an abstract `Verifier` trait
(`preflight.rs` calls itself a "scaffold"). `tests/integration/preflight_gate_test.rs`
exercises only a `RecordingVerifier` mock (2 tests), so no concrete stage
verifier exists yet.

Finishing these subtasks is a same-contract-surface completion of work
already authorized, in F50's own owned files. Under P-021 C3's symmetric
guard it is IN scope and MUST be done. It must not be deferred.

### F3: no launcher-callable preflight contract exists

* `target\debug\engram.exe preflight --help` returns
  `error: unrecognized subcommand 'preflight'`. No preflight command appears
  in `src/bin/engram.rs`'s `Command` enum or in the `--help` list.
* `crates/engram-indexer/src/main.rs` is env-driven index-only
  (`ENGRAM_INDEXER_*`) and has no preflight mode.
* Plan lineage: P34 "Shared preflight command" (`src/bin/engram-indexer.rs`)
  said it "emit[s] one structured verdict", and P35 said "Invoke P34". The
  F-roster folded P34 into F50 but reduced its file list to the library
  module. The process-boundary entrypoint belongs to no unit, so this is a
  decomposition gap.
* The typestate cannot be linked into the root `engram` binary.
  `engram-indexer` depends on `engram`, so the reverse dependency would be a
  package cycle. `supervisor_workspace_boundary_test` also forbids an
  `engram-indexer` bin in the root package.
* `cargo dev-test` (`test --all-targets` at the workspace root) builds and
  runs root-package targets only. No CI job runs `-p engram-indexer` tests.
  A test in `crates/engram-indexer/tests/` would therefore never gate.
  F50's harness works around this by including `preflight.rs` through
  `#[path]`.

### F4: the launcher harnesses already fix the verdict shape and are argv-agnostic

* The F51 (`start_launcher_test.rs`), F52 (`start_launcher_failure_test.rs`),
  and F53 (`start_sh_launcher_test.rs`) fixtures ignore their argv. None of
  them pins a subcommand name.
* F52 and F53 print exactly one line: `{"state":"Succeeded"}` (exit 0) or
  `{"state":"Failed","stage":"<Stage>"}` (exit 23). F51's PATH shim does the
  same.
* F51's workspace-exe fixture prints plain `Succeeded` (exit 0) or
  `Failed at HealthVerified` (exit 23). A launcher that requires JSON parsing
  to accept success can never pass the F51 success test. **Exit code 0 must
  therefore be the authoritative success signal.**
* F51 already asserts the workspace-binary requirement:
  `RED F51-WORKSPACE-ENGRAM`, `RED F51-EXACT-ENGRAM-PATH`,
  `RED F51-NO-PATH-FALLBACK`, `RED F51-NO-TOOLS-FALLBACK`, and
  `RED F51-MISSING-WORKSPACE-BINARY`. The operator's workspace-binary
  request is already in F51 scope, and nothing new is needed for it.

### F5: prior learnings

`docs/compound/` has no entry on pending-RED payload classification or on a
preflight command. The source deliberation
(`2026-09-02-separate-indexer-read-server-deliberation.md`) says: "The session
launcher ... verifies health plus representative reads before launching
Copilot." That confirms a launcher→preflight process boundary was always
intended.

## Options Evaluated

### Decision D1: classifying the WARNING lines

* **D1-A: accounted-for payload (recommended).** Treat the lines as bytes of
  the mapped `142.055-T` failure, accounted for exactly once under condition
  5. The Step 4.3 record enumerates them explicitly, with line numbers and
  the owning test. Nothing is suppressed or edited. This is consistent with
  the `doctor` precedent (F1) and requires no one to edit a different task's
  RED harness.
* **D1-B: edit the F51 fixture so the failed mode does not reach the legacy
  path.** Rejected. From F50's cycle it would edit a different task's
  pending-RED harness, which Step 4.3 and build-feature explicitly forbid. It
  would force yet another baseline re-record. It would also hide the very
  fail-open evidence the test exists to expose.
* **D1-C: implement F51 first.** Rejected. It violates dependency order
  (F51 depends on F50) and the test-first sequence.

### Decision D2: the launcher-callable command host

* **D2-A: supervisor entrypoint plus `engram` relay (recommended).**
  * `engram-indexer preflight` drives the F50 typestate with the production
    verifier and prints the verdict.
  * `engram preflight` (in `target\debug\engram.exe`) spawns its sibling
    `engram-indexer(.exe)` next to `current_exe()`, forwards the arguments
    plus `--engram <self>`, and relays the verdict and exit status. It fails
    closed with `{"state":"Failed","stage":"Build"}` when the sibling is
    missing.
  * Pros: one typestate source; supervisor ownership (the F50 title,
    P34 lineage); satisfies F51's exact-engram.exe assertions and the
    operator's request; no package cycle.
  * Cons: one extra process hop, and a dev build needs
    `cargo build --workspace` (visible fail-closed diagnostic otherwise).
  * Effort: medium.
* **D2-B: compile `preflight.rs` into the root `engram` binary via
  `#[path]`.** Pros: no hop. Cons: supervisor logic moves into the root
  binary through a cross-crate path include in production code. That
  contradicts the separate-supervisor boundary, duplicates compilation
  units, and invites type drift. Effort: low-medium.
* **D2-C: have the launcher call `target\debug\engram-indexer.exe`
  directly.** Rejected. It contradicts F51's executable spec
  (`RED F51-EXACT-ENGRAM-PATH`) and the operator's explicit
  `target\debug\engram.exe` requirement.

## Trade-off Comparison

| Criterion | D2-A relay | D2-B `#[path]` in root | D2-C direct indexer |
|---|---|---|---|
| F51/F52/F53 harness conformance | yes | yes | no (F51 exact-path) |
| Operator `target\debug\engram.exe` requirement | yes | yes | no |
| Single typestate source / supervisor boundary | yes | weakened | yes |
| Gate-visible tests (root `cargo dev-test`) | yes, via `#[path]` and copied-binary relay test | yes | n/a |
| New files | 2 new source, 2 new tests | 1 new source, 1 new test | none (spec violation) |
| Risk | moderate (process relay) | moderate (architecture drift) | blocked |

## Decision

**D1 → D1-A.** The two `WARNING:` lines are part of the mapped
`142.055-T` failure payload. They are not a Step 4.3 condition-2 warning.
Ship remains the sole authority to accept `EXPECTED_PENDING_RED`. Stage's
evidence-backed recommendation is that D1-A is the correct reading of the
existing rule, not a weakening of it. Ship's record must quote both lines,
their output line numbers, the owning test and marker, and the zero-toolchain-
warning scan. The lines recur in every interim full-suite run until F51
lands, so every interim task gate from F50 through F51 applies the same
classification. The wording clarification is captured separately as stash
`5D707465` (low priority, deferred).

**D2 → D2-A**, with this minimal contract:

* Invocation: `engram preflight --workspace <PATH> --timeout-ms <MS>`. Both
  flags are required. `--timeout-ms 0` is valid and deterministically yields
  `Failed` at `Build`.
* Stdout carries exactly one verdict line: `{"state":"Succeeded"}` or
  `{"state":"Failed","stage":"<StageKind>"}`, where `<StageKind>` is one of
  the seven `StageKind` names. Diagnostics go to stderr only.
* Exit status is 0 iff the state is `Succeeded`. It is 1 for a typed
  `Failed` verdict, and 2 for an invocation or usage error (no verdict line).
* Launcher rule for F51 and F53: launch Copilot iff the exit status is 0 AND
  no stdout verdict line reports a non-`Succeeded` state. Otherwise fail
  closed and surface stdout+stderr. Exit status is the authoritative signal
  (F4).

**D3 (in scope, no expansion):** F50 must finish `142.054.002-ST` and
`142.054.003-ST` in its own owned files, test-first. It adds RED assertions
to `tests/integration/preflight_gate_test.rs` for each subtask's acceptance
criteria, then implements the production verifier in
`crates/engram-indexer/src/preflight.rs`. The verifier must stay compilable
under the harness's `#[path]` include, so it may use only `engram::` and
`std` paths.

Operator-confirmation basis: the operator's 2026-09-26 directive asked Stage
to "define minimal separate task ... to create a command that drives the F50
typestate". That confirms direction D2. The host choice D2-A over D2-B is
Stage's recommendation. It stays reversible until the operator approves
adding the harvested tasks to 142-S (PA-1 in the plan), because that
approval is itself the confirmation of D2-A.

## Rejected Alternatives

* D1-B and D1-C are rejected for the reasons stated above.
* D2-B is rejected because of the architecture boundary and the production
  `#[path]` include.
* D2-C is rejected because it violates the F51 spec and the operator
  requirement.
* Folding the command into F50 or F51 is rejected under P-021 C1(b): it is a
  different contract surface, and ambiguity resolves out of scope.
* Opening a second shipment is rejected under P-001/P-016: 142-F is already
  carried by active 142-S.

## Unresolved Questions

* The constructor inputs and generation-minting API of the F50 production
  verifier are not known until D3 lands. The plan's NEW-2 halts back to
  Stage if they need inputs that cannot be derived from
  `{workspace, engram executable, deadline}`.
* It is not yet verified that a copied debug `engram.exe` runs from a temp
  directory (native DLL co-location). The relay harness must smoke-check
  this as infrastructure before asserting RED.

## Risks and Mitigations

* **Relay hides a supervisor crash as success.** The relay exits 0 only when
  the child exits 0 AND the child's stdout is exactly `{"state":"Succeeded"}`.
  Covered by a test scenario.
* **Dev builds lack `engram-indexer.exe`.** Fail-closed typed `Build` failure
  with a stderr diagnostic naming the missing path. Documented by NEW-4.
* **The D1 reading is contested.** Fallback approval PA-3 (a one-sentence
  operator ruling) is defined in the plan. No edit or suppression is ever
  used.
* **D3 exceeds the 2-hour rule.** `142.054.003-ST` in particular may. If so,
  Ship circuit-breaks and returns it to Stage for a split, which also needs
  manifest approval.

## Amendment 1 (2026-09-26): positional workspace, intake records, and D4

### A1.1: D2 contract amendment

Plan revision 2 changed the `engram` side of the D2 contract to a POSITIONAL
workspace: `engram preflight <WORKSPACE> --timeout-ms <MS>`. A `--workspace`
flag would collide with the clap-global `--workspace` / `ENGRAM_WORKSPACE`
override (attempt-1 P1 #2). The supervisor keeps a strict
`preflight --workspace <PATH> --timeout-ms <MS> --engram <ABS>` form because
it has no clap globals. Every other D2 term (verdict line, exit 0 iff
`Succeeded`, exit 1 typed failure, exit 2 usage) is unchanged.

### A1.2: P-021 C5/C6 triage records

| Entry | Duplicate scan (A) | Late-identifier reconciliation (B) |
|---|---|---|
| `03AA00A8` (launcher command) | CLEAN: the 2026-09-26 scan over every active stash entry found no other entry describing the command expansion. `E53CB236` (F53 dependency on F52) is a different surface. | Triggered (PR `N/A`, thread `N/A`). Searched the Ship residual-risk records that cite `03AA00A8` (`2026-09-26-ship-142-s-f50-concrete-verifier-api-block.md`). **No late identifier found**; no PR exists. `N/A` stands as a truthful terminal record. |
| `49809128` (F50 facades) | CLEAN: no other active entry describes the F50 facade or probe gap. `03AA00A8` is the command surface only, so the two are not duplicates. | Triggered (PR `N/A`, thread `N/A`). Captured by Stage this session from Ship's residual-risk record; **no late identifier found**. `N/A` stands. |

### A1.3: new evidence (Ship, after the first deliberation)

Ship's `2026-09-26-ship-142-s-f50-concrete-verifier-api-block.md` shows D3's
premise is false: F50 cannot finish `002-ST`/`003-ST` inside its two owned
files. Stage verified the following against HEAD `4995d681`:

* **Build.** `GenerationStore::seal_candidate` mints an exclusive
  `IndexTarget::Candidate`. But F10's `index_sealed_target` uses the candidate
  as the source DISCOVERY root, and writes the database to a separate
  `data_dir/cozo/<branch>/engram.db`. Activation requires
  `<generation-id>/engram.db` (`GENERATION_DATABASE_FILE_NAME`). No public
  function indexes the workspace INTO a candidate. The parent plan's
  P9/P11 ("explicit candidate data root", "Build and seal a candidate
  generation") and P12 ("Publish from the supervisor") were lost in the
  F-roster renumbering, exactly like P34.
* **Seal.** `SealedInventory::new` takes caller-supplied digests.
  `engram::services::file_tracker::compute_file_hash` is public and returns
  lowercase SHA-256 hex, so a digest helper does exist (Ship's "private"
  finding is partly superseded). But `engram-indexer` depends only on
  `engram` and `tokio`.
* **Publish.** `publish_generation_manifest` is public, but constructing the
  `GenerationManifest` needs `chrono::Utc::now()`, and the next revision
  needs the current manifest. That reader is private (`publish.rs`), and
  `engram-indexer` has neither `chrono` nor `serde_json`.
* **Probes.** They are feasible at the PROCESS level. No in-process seam is
  needed. The F54 harness (`read_server_cli_mcp_parity_test.rs`) already
  does four things with `CARGO_BIN_EXE_engram`:
  * spawns `engram daemon --workspace`
  * reads `/provenance/generation_id` from `engram <cmd> --json`
  * maps `get_health_report` to CLI `health`
  * drives `engram shim --workspace` over stdio

  `cli::runner` auto-spawns the daemon through
  `shim::lifecycle::ensure_daemon_running`. The only blocker is that the
  supervisor crate cannot parse JSON (no `serde_json`).

### D4: where the F50 concrete-verifier support lives

* **D4-A (recommended): root-library facades plus F50 composition in its
  owned file.** Add four prerequisite tasks under 142-F in the root `engram`
  library:
  * `src/services/generations/candidate_build.rs`: build and seal, then
    publish.
  * `src/services/preflight_probe.rs`: the CLI and health probes, then the
    stdio MCP probe.

  Each task is test-first, with a root `integration_` harness that
  `cargo dev-test` and root clippy gate. F50 then completes `002-ST` and
  `003-ST` in `preflight.rs` and `preflight_gate_test.rs` as a pure
  composition of those public `engram::` facades plus `std` and `tokio`.
  That composition compiles both under the `#[path]` include and in
  `engram-indexer`. F50 gains no new files, and `engram-indexer`'s manifest
  is untouched. F50's six pedantic lints in `preflight.rs` (for example
  `Result<(), ()>`) remain F50's in-scope work under the C3 symmetric guard.
* **D4-B: add `sha2`, `chrono`, and `serde_json` to
  `crates/engram-indexer/Cargo.toml`, and put the full composition and
  probes in supervisor modules.** Rejected. Root `dev-test` and root clippy
  do not gate `engram-indexer` modules, apart from what `#[path]` includes.
  It duplicates the private revision and manifest logic. And `preflight.rs`
  would grow far past the 2-hour rule.
* **D4-C: change F10's `index_sealed_target` so the candidate is the data
  root.** Rejected. It reverses a done contract that
  `candidate_indexing_service_test` asserts, which means regression risk in
  a completed task.
* **D4-D: snapshot-copy workspace sources into the candidate, then use F10
  as-is.** Rejected. It copies the whole workspace on every session start
  and invents snapshot rules (ignore handling, size bounds) that no
  requirement covers.

**Decision D4 → D4-A.** This is Stage's recommendation. Operator
confirmation is bundled into the admission approval (PA-1) in plan revision
3, because nothing executes before that approval. D3 is amended: F50 still
finishes `002-ST`/`003-ST` in its owned files, test-first, but only after
the four facade prerequisites land. `142.054-T` therefore gains dependency
edges on them, and that edit is part of PA-1.

**Residual risk (D4).** A cold full index runs on every session start. The
launcher budget `T` must cover it. Seeding a candidate from the active
generation for an incremental build is captured as a follow-up, not added
here.

## Amendment 2 (2026-09-26): G3 lineage, the read-server data path, and D5

Plan review attempt 2 (Scope P1 #4) found that the G3 production-wiring gap
had no stash or deliberation lineage. This amendment supplies it.

### A2.1: P-021 C5/C6 triage records for G3 and the new follow-ups

The duplicate scan (A) ran over all 151 active stash entries **before** any
new capture. It found that G3 was **already captured** by Ship during 141-S,
as two distinct deferred-scope-expansion entries. Stage therefore
reconciled those two entries in place and captured **no** new G3 entry
(anti-duplication).

| Entry | Duplicate scan (A) | Late-identifier reconciliation (B) |
|---|---|---|
| `9B7EC1E4` (startup never constructs/installs a `GenerationActivator`) | CLEAN. `6C5DF765` covers the other half of G3 and is not a duplicate. `1918AFD2` (mode not exposed over IPC) and `265F99BE` (readiness latch after a branch switch) cover different surfaces. | Triggered (PR `N/A`, thread `N/A`). **Recovered** from the Ship residual-risk record `docs/archive/memory/2026-09-23/2026-09-20-ship-141-s-copilot-review-and-ci-infra.md`: PR **#407**, review thread **`PRRT_kwDORJEduc6kM96E`** (`lifecycle_policy.rs:187`). Appended in place under Stage authority; the capture-time `N/A` text is retained. |
| `6C5DF765` (`process_request` never calls `admit_read`, so provenance and admission are unreachable) | CLEAN (the counterpart of `9B7EC1E4`). | Triggered. **Recovered** from the same record: PR **#407**, thread **`PRRT_kwDORJEduc6kM96Y`** (`tools/mod.rs:377`). Appended in place. |
| `86F93068` (new, DEFERRED SCOPE EXPANSION: the remaining read handlers still read managed state; blocks F54 GREEN) | CLEAN. No active entry describes converting handlers to the admitted context. | Triggered (PR `N/A`, thread `N/A`). Captured by Stage this session; **no late identifier found** (there is no PR yet). `N/A` stands as a truthful terminal record. |
| `5AF5CD66` (align read-server `_health` with the gate), `23E287C6` (generation retention/GC), `F99C705E` (incremental candidate seeding), `7BF90213` (a workspace-wide `engram-indexer` test/clippy gate) | CLEAN for each. `265F99BE` is a different `_health` defect. `9108DB24` (runtime-copy lease) is a prerequisite that `23E287C6` cites, not a duplicate. | Not triggered. These are Stage-captured follow-ups, not deferred-scope captures, and they carry no `N/A` source-ref fields. |

`5C873386` is a PR 391 Copilot finding: retrying `run_initial_activation`
from `Failed` re-hashes a permanently rejected revision. It is **not** a
duplicate. It is a constraint that PRE-3 must honour: retry only when the
published revision changes (see D5).

**Re-verification (2026-09-26, narrow Stage pass for attempt-2 Scope P1 #4):**
the duplicate scan (A) was re-run over all 156 active stash entries and the
archive. No G3 entry other than `9B7EC1E4` and `6C5DF765` exists, and no queue
item cites either. Result: CLEAN, no new capture (anti-duplication). Late-ID
reconciliation (B) is idempotent: both entries already carry PR #407 and their
recovered thread IDs, so this pass was a no-op. Plan PA-1 now names D5 and G3
explicitly.

### A2.2: the evidence behind the D5 design (verified at HEAD `4995d681`)

* **The done no-generation consumers issue generation-backed reads.**
  * `read_server_restart_test` calls `daemon-status`.
  * `get_daemon_status` is declared `Read`, `read_server_available`, and
    `DaemonHandler`, so `request_entry::is_generation_backed_read` classifies
    it as generation-backed.
  * `cli::runner` fetches `get_workspace_status` for indexing progress.
  * Routing **every** generation-backed read through `admit_read` would
    therefore refuse these reads on a daemon with no published generation.
    That is exactly the attempt-2 P1 #5 regression.
* **Only one handler reads the database through a request context.**
  * `get_workspace_statistics` and `query_memory` call
    `pinned_read_request_context` and then `queries_from_read_context`,
    which calls `connect_db(context.data_dir(), branch)`.
  * For a generation context, `data_dir()` is the runtime copy's parent
    directory. `connect_db` would open or CREATE `<runtime>/<id>/cozo/<branch>/engram.db`,
    which is an empty database and not the opened generation.
  * So passing an admitted context to today's handler would label empty
    data with a generation ID. This is attempt-2 P1 #6.
* **The opened DB can be served without touching the DB layer.**
  * `OpenedGeneration::db()` returns `&cozo::DbInstance`.
  * `cozo::DbInstance` derives `Clone` (cozo 0.7.6).
  * `CozoDb { pub(crate) inner: Arc<cozo::DbInstance> }` and
    `CodeGraphQueries::new(CozoDb)` are reachable from `src/tools/read.rs`
    (same crate).
  * So `src/db/cozo_backend/mod.rs` does not need to change.
* **The generation root may not exist.**
  * `GenerationStore::new` canonicalizes and requires an existing directory.
  * The activator runtime root must be absolute. The parent plan puts
    runtime copies under `.engram/generations/runtime/`.
* **The startup driver runs after the IPC bind** (`ipc_server.rs:441-453`
  and `:758-770`), so `ReadServerStartupGate::socket_bound()` is truthful
  when called from `run_read_server_startup`.
* **No done read-server test calls `get_workspace_statistics`.**
  * F54 references it only in its CLI mapping (`"stats"`).
  * F54's equivalence case pins `get_workspace_status`.

### D5: G3 production read-server wiring and the data path

**Options:**

* **D5-A (recommended): narrow, allowlisted generation serving.**
  * PRE-3 constructs the store, the activator, and the gate only when
    `<ws>/.engram/generations` exists. It runs initial activation in the
    background after bind, retrying with bounded backoff only when the
    published revision is non-zero and has changed since the last attempt.
    `_health` semantics are unchanged.
  * PRE-4b makes `get_workspace_statistics` consume a passed admitted
    context, and read from the opened generation's `DbInstance`.
  * PRE-4 routes ONLY the methods in `GENERATION_PINNED_READS`
    (`["get_workspace_statistics"]`) through `admit_read` and
    `dispatch_with_read_context`. Every other method keeps today's path
    byte-for-byte.
  * A daemon with no generations root changes in one way only: it refuses
    the allowlisted pinned read (`get_workspace_statistics`) with the typed
    `GenerationNotYetActivated` error (see Residual risks). Every other
    method, including `_health`, `get_daemon_status`, and
    `get_workspace_status`, is unchanged.
  * Other handlers are converted later, under `86F93068`.
* **D5-B: route every generation-backed read through `admit_read`.**
  Rejected. It refuses `get_daemon_status` and `get_workspace_status` on
  no-generation daemons, which regresses done consumers. It also labels
  managed data with generation provenance for every unconverted handler.
* **D5-C: gate `_health` on the startup gate.** Rejected for now. Every
  done no-generation consumer waits for `_health == ready`. This is
  deferred to `5AF5CD66`.
* **D5-D: fall back to managed data when the gate is not ready.** Rejected.
  The pinned read would silently serve unpinned data, and the preflight
  could not tell which it got.

**Decision D5 → D5-A.** This is Stage's recommendation. Operator
confirmation is bundled into PA-1. D5 is the lineage for plan units PRE-3,
PRE-4b, and PRE-4 (stash `9B7EC1E4` and `6C5DF765`).

**Residual risks (D5):**

* F54 (`142.058-T`) still cannot go GREEN inside 142-S. Its equivalence
  read (`get_workspace_status`) stays unpinned until `86F93068` is planned
  and admitted. This is surfaced to the operator, not hidden.
* The daemon may start before the generations root exists. In that case the
  PRE-3 background task re-checks for the root on every bounded backoff
  tick, and only then constructs the store, activator, and gate. Until
  that happens, the pinned read is refused with the typed
  `GenerationNotYetActivated` error (fail closed). The polling cost is one
  `metadata` call per tick, and the task ends at daemon shutdown.

## Amendment 3 (2026-09-26): D5-A reconciled with plan revision 5, and stash `EFE9190A` / `4628001C`

Plan review attempt 4 (ADVISORY, P2-7) found that D5-A and the D5 lineage
had drifted from plan revision 5. The PA-1 approval phrase asks the
operator to "confirm D5-A", so the text being confirmed has to match the
plan. The operator authorized this amendment ("Option A") in the 142-F
harvest session. It changes no option ranking and no decision: D5 is still
D5-A, and operator confirmation is still bundled into PA-1, which is NOT
granted.

### A3.1: D5-A as confirmed by PA-1 (supersedes the D5-A PRE-3 bullet above)

The first D5-A bullet ("PRE-3 constructs ... only when the published
revision is non-zero and has changed since the last attempt") is
superseded by this text, which matches plan revision 5 (PRE-3 and the
Decisions D5-A entry) plus the attempt-4 P2-1 harvest refinement:

* **Install before readiness.** When `<ws>/.engram/generations` is a
  directory at startup, PRE-3 installs the store, activator, and gate
  synchronously (no manifest read, hashing, or copy) AFTER workspace
  publication and BEFORE `set_hydration_ready_for_generation`. `_health`
  semantics are unchanged: readiness never depends on the install or on
  activation succeeding.
* **Background activation after readiness.** A spawned, never-awaited
  driver ticks with bounded backoff (100 ms, doubling, 2 s cap). It calls
  `run_initial_activation` only when the published revision is non-zero
  AND either (a) it differs from the last attempted revision, or (b) the
  last attempt on the SAME revision failed with an error that
  `activation::classify` rates `RejectionClass::Transient`, its backoff
  has elapsed, and less than the layout's activation deadline has passed
  since that revision's first transient failure.
* **Permanent rejections are never retried.** A `Permanent` rejection is
  never re-attempted for the same revision. This keeps the `5C873386`
  constraint recorded in A2.1 ("a rejected revision is never re-hashed");
  A2.1's shorter wording ("retry only when the published revision
  changes") is refined, not reversed, by the Transient rule.
* **Publication-last.** After `active_revision` is published, the driver
  writes no file and changes no `path`/`branch`/`db_path`/`generation`
  field of `get_workspace_status`.
* **Late install.** A root that is ABSENT at startup and appears later is
  installed by a driver tick. *Harvest refinement (attempt-4 P2-1):* when
  the root EXISTS at startup and the install fails, that daemon does not
  late-install. The failure is logged once at `warn` and is final for that
  daemon's lifetime, so a F54 barrier `NoActivator` observation stays
  terminal. This is written into the PRE-3 task acceptance.
* PRE-4b and PRE-4 bullets, the D5-B/C/D rejections, and the residual
  risks are unchanged. The D5 residual-risk bullet about a root appearing
  after startup applies only to a root absent at startup.

### A3.2: D5 lineage (supersedes "D5 is the lineage for plan units PRE-3, PRE-4b, and PRE-4")

D5 is the lineage for plan units **PRE-3F, PRE-3, PRE-4b, and PRE-4**:

* stash `9B7EC1E4` → PRE-3F (the F54 activation-settle protocol, attempt-3
  L3-1 closure, which exists only to make PRE-3 safe for F54) and PRE-3;
* stash `6C5DF765` → PRE-4b and PRE-4.

The PA-1 phrase also names the invariant-6 ownership exception that lets
PRE-3F edit `142.058-T`'s harness within the PRE-3F diff contract.
*(Retired by PA-7, 2026-09-29: the invariant-6 exception is never exercised.
PRE-3F owns its own test target, and only `142.058-T` and its split family edit
the F54 file, after PA-5. See the status note at the top of this record.)*

### A3.3: stash `EFE9190A` note on D5 (P-021 C5/C6)

* **Relation.** `EFE9190A` (139-S deferred scope expansion candidate:
  thread the request-entry `ReadRequestContext` into shared dispatch) is
  the earliest capture of the dispatch-threading question. It partially
  overlaps `6C5DF765` on the dispatch plumbing and `86F93068` on the
  per-handler conversion. It is **not a duplicate** of either.
* **Duplicate scan (A):** partial overlap, no duplicate; no merge and no
  duplicate archive.
* **Late-identifier reconciliation (B):** PR **#393** (139-S) recovered
  from the Ship residual-risk record
  `docs/archive/memory/2026-09-12/139-s-ship-session-summary.md:110-113, 251-259`.
  Several resolved Copilot threads there cite `EFE9190A`, but the record
  lists no thread IDs, so review-thread `N/A` stands as a truthful record.
  Reconciled in place under Stage authority.
* **Disposition under D5.** PRE-4 consumes its dispatch-plumbing part:
  for `GENERATION_PINNED_READS`, `process_request` hands the admitted
  context to `dispatch_with_read_context`, and PRE-4b's arm hands it to the
  handler. The residual per-handler part (every other generation-backed
  handler) is forward-referenced to `86F93068`. The entry is archived at
  harvest Step 5.6 with both forward refs; the harvested PRE-4 ID is
  recorded in the plan's Harvest Record and the harvest memory.

### A3.4: stash `4628001C` (P-021 C5/C6)

* **Duplicate scan (A):** partial overlap with `86F93068` on the
  `get_workspace_status` conversion; not a duplicate.
* **Late-identifier reconciliation (B):** PR **#407** (141-S) recovered
  from `docs/archive/memory/2026-09-23/2026-09-20-ship-141-s-copilot-review-and-ci-infra.md:56, 67`.
  No review-thread ID cites it, so review-thread `N/A` stands. Reconciled
  in place.
* **Disposition.** Not consumed by this plan and NOT archived. It is
  carried into the `86F93068` deliberation (PA-5) as the source of the
  `get_workspace_status` conversion. Attempt-4 P2-2 also cites it: the
  PRE-3F barrier's `get_workspace_status` call runs `connect_db`, which the
  PRE-3F task acceptance handles by never placing an IPC call between two
  compared filesystem snapshots.

### A3.5: recorded outcomes

| Entry | (A) duplicate scan | (B) late identifier | Disposition |
|---|---|---|---|
| `EFE9190A` | partial overlap, no duplicate | PR #393 recovered; thread `N/A` stands | consumed in part by PRE-4; residual → `86F93068`; archived at Step 5.6 |
| `4628001C` | partial overlap, no duplicate | PR #407 recovered; thread `N/A` stands | carried to `86F93068` / PA-5; not archived |

## Operator Approval (2026-09-27)

Recorded by Stage on 2026-09-27. The operator's decisions (21:18 and 21:21
-07:00, routed by the Orchestrator) were: "1. Yes, but 24 tasks is much too
large for a single shipment; this should be decomposed into at least 3
shipments of 8 tasks each." / "2. Yes" / "approve 1 and 2, start planning 4,
hold 3".

* **PA-3 granted: D1-A is ratified.** Text inside the panic payload of a
  mapped pending-RED test is part of that mapped failure, not a Step 4.3
  warning.
* **D2-A, D4-A and D5-A are confirmed.** D5-A admits the G3 scope expansion
  (stash `9B7EC1E4` → PRE-3F/PRE-3, stash `6C5DF765` → PRE-4b/PRE-4) under
  P-021 C6, with the D5-A text as amended by A3.1 and A3.2.
* **Granted with PA-1:** the active-task edges (`142.054-T` on `142.068-T`;
  `142.058-T` on `142.066-T` and `142.065-T`; `142.055-T` and `142.057-T`
  on `142.073-T`), the invariant-6 exception (`142.063-T` may edit
  `tests/contract/read_server_cli_mcp_parity_test.rs` only within the
  PRE-3F diff contract), PA-2 and PA-2b. *(The invariant-6 exception was
  retired by PA-7 on 2026-09-29 and is never exercised.)*
* **Supplemental phrase granted:** `142.069-T` and `142.071-T` depend on
  `142.054-T`.
* **Modified, not granted as written:** PA-1's "add every harvested unit to
  active 142-S". The operator asked instead for the remaining 142-F
  launcher-preflight scope to be split into at least 3 shipments of about 8
  tasks each. The split is proposed in
  `docs/memory/2026-09-27-stage-142-f-shipment-split.md` and needs a
  further operator approval before any shipment is changed.
* **PA-4 is on hold** (no config change). **PA-5** (stash `86F93068`) goes
  to a separate, later Stage session.
