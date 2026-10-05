# Stage checkpoint: 142-F preflight command and the F50 gate-warning classification

- **Status:** mid-session (context rollover). Resume from "Next action".
- **Role:** Stage, invoked by the Orchestrator for active shipment 142-S. Branch `feat/142-s-...` at HEAD `4995d681`.
- **Stage constraints:** no source, test, or config edits; no build or test runs; no claim, PR, or commit. Artifacts stay uncommitted in the working tree, per Stage precedent `2026-09-25-stage-be626470`.

## Goal and acceptance

The operator asked Stage to find a lawful way for Ship to keep going on 142-S. The session is done when four things are delivered:

1. **The two WARNING lines.** An evidence-backed classification of the two `WARNING:` lines that block F50 (142.054-T) at Step 4.3.
2. **The preflight command.** A determination of whether a launcher-callable F50 preflight command exists. If none exists, a separate reviewed work unit under 142-F, with owned files, a test-first posture, and dependencies.
3. **The approvals needed.** The exact operator approvals required. The 142-S manifest must not be mutated without a fresh approval.
4. **Ship's next step.** How Ship continues now.

## Gates done

- **Tools:**
  - The registry is present. MCP is absent, so the CLI fallbacks are in use.
  - `backlogit sync` succeeded (`INDEX_SYNC_OK`, CLI fallback).
  - The Stage hook poll returned `events:[]` and `derived:[]`, so there was nothing to acknowledge.
  - The unfiltered checkpoint scan returned 34 checkpoints, with 0 anomalies and 0 active Stage checkpoints. This was a normal start.

## Evidence

- **E1: The WARNINGs are payload.** Both `WARNING:` lines (full-suite output lines 4121 and 4124) sit inside the panic payload of the mapped RED test `typed_preflight_failure_is_reported_without_starting_copilot`. That test is `tests/contract/start_launcher_test.rs:547-549`, marker `RED F51-FAIL-CLOSED`, owned by 142.055-T, and it interpolates `{transcript}` into its message.
  - The output contains no rustc or cargo `warning:` line, and `.cargo/config.toml` sets `-Dwarnings`.
  - Ship already classified `error: unrecognized subcommand 'doctor'` (line 2448) inside the F54 payload the same way.
  - The warnings are the legacy fail-open evidence that F51 removes.
- **E2: F50 is incomplete.** Subtasks 142.054.002-ST (Build/Seal/Publish) and 142.054.003-ST (Daemon/Health/CLI/MCP probes) are active and in the manifest, but no concrete verifiers exist.
  - Commit c269fa79 added only the typestate and an abstract `Verifier`. The harness covers only a `RecordingVerifier` mock.
  - Completing these subtasks is IN SCOPE (C3 symmetric guard). F50 cannot close until then, whatever happens with the WARNINGs.
- **E3: No launcher-callable preflight command exists.**
  - `engram.exe preflight` reports "unrecognized subcommand".
  - `engram-indexer` `main.rs` is env-only indexing.
  - Plan P34 ("Shared preflight command", `src/bin/engram-indexer.rs`) was dropped when the roster was renumbered to F50.
  - A package cycle prevents linking `engram` to `engram-indexer`.
  - `cargo dev-test` and root clippy cover only the root package.
- **E4: The launcher fixtures fix the verdict shape.**
  - The F51, F52, and F53 fixtures ignore argv.
  - Their JSON verdicts are `{"state":"Succeeded"}` and `{"state":"Failed","stage":"X"}`.
  - F51's workspace-exe fixture prints plain `Succeeded`, so the exit code must be the authoritative signal.
  - F51 already tests the `target\debug\engram.exe` requirement.

## Artifacts created (uncommitted)

- **Stash `03AA00A8`:** high priority, a DEFERRED SCOPE EXPANSION for the command gap.
  - PR and review thread are N/A.
  - Duplicate scan was clean, and no late identifier was found. Both outcomes still need to be recorded in the deliberation or memory.
- **Stash `5D707465`:** low priority, deferred. It clarifies the policy wording for "warnings".
- **Deliberation:** `docs/decisions/2026-09-26-142-f-launcher-preflight-command-deliberation.md`.
  - It records D1-A (the WARNINGs are payload), D2-A (supervisor plus relay), and D3 (F50 completes 002-ST and 003-ST).
  - Its D2 contract still says `--workspace <PATH>` for `engram`. Plan v2 changed this to a POSITIONAL workspace, so the deliberation needs an amendment note.
- **Plan v2:** `docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md`.
  - It has 6 units, plus a Constitution Check, Plan Hardening (PA-1, PA-2, PA-3), and an appended record of review attempt 1 (FAIL), with `<!-- plan-review-attempt: 1 -->`.
  - The units:
    - NEW-1: verdict runner in `crates/engram-indexer/src/preflight_verdict.rs` plus `lib.rs`.
    - NEW-2: invocation and `main_with` in `crates/engram-indexer/src/preflight_invocation.rs` plus `lib.rs`.
    - NEW-3: `main.rs` entrypoint, with the crate-local test `crates/engram-indexer/tests/preflight_entrypoint_test.rs`.
    - NEW-4: relay library in `src/cli/commands/preflight.rs` plus `mod.rs`.
    - NEW-5: hidden `engram preflight <WORKSPACE> --timeout-ms`, in `src/bin/engram.rs`.
    - NEW-6: docs, in `docs/cli-mcp-parity.md` and the compound 118-S supersession note.
  - Dependencies: `054 → N1 → N2 → N3`, `N1 → N4 → N5`, `N3 → N5 → N6`.

## Review attempt 1 (FAIL)

- There were 10 distinct P1 findings. Plan v2 addresses all of them; see the appended section in the plan.
- Personas used: Rust, Scope, Architecture, Constitution, Learnings, Security.

## Next action

Run plan-review attempt 2 on plan v2 with the Rust, Scope, Constitution, Architecture, and Learnings personas. Ask them to verify that the attempt-1 P1s are resolved and to report any new P0 or P1.

## Remaining work

1. **Review attempt 2.** If it FAILs, the plan is at the maximum of 2 re-entries. Follow the escalation protocol and report the exact blockers.
2. **If PASS or ADVISORY, harvest.**
   - Command: `backlogit add --type task --parent 142-F --status queued`, with a description, the acceptance criteria and implementation-notes sections, size and complexity as prose, and owned files.
   - Record dependencies with `backlogit dep add`. The first new task depends on 142.054-T.
   - Do NOT add the new tasks to 142-S. Do NOT edit 142.055-T or 142.057-T.
3. **Step 5.5.** Log it as blocked pending PA-1. Create no new shipment.
4. **Step 5.6.** Archive `03AA00A8` with `backlogit stash archive`, carrying forward references to the new task IDs.
5. **Follow-up stash.** Add an entry for a workspace-wide gate: `dev-test` and root clippy exclude `engram-indexer` tests and lints.
6. **Deliberation amendment.** Record the positional-workspace change, the clean duplicate scan, and "no late identifier found".
7. **Final memory file and index sync.**
8. **Final answer to the Orchestrator.** It must include:
   - **Decision D1-A.** Ship may apply it now on its own authority. PA-3 is a recommended ratification.
   - **Decision D3.** F50 continues now, test-first, extending its own harness for 002-ST and 003-ST and then implementing verifiers in `preflight.rs`. This needs no approval.
   - **The PA-1 wording**, filled in with the new task IDs: "I authorize Stage to add <IDs> to active shipment 142-S and to make 142.055-T and 142.057-T depend on <NEW-5>."
   - **PA-2:** the launcher contract notes.
   - **The execution order.**

## Stopping criterion

Stop once the tasks are harvested (or the exact blocker is reported), the approvals are listed, and the summary is returned.
