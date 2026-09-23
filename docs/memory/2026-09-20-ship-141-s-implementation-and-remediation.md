# Ship session memory — 141-S implementation and review remediation

Date: 2026-09-20
Branch: `feat/141-s-error-transport-response-provenance-lifecycle-policy-and-generation-observability`
Shipment: `141-S` (error transport, response provenance, lifecycle policy, generation observability)
Feature: `142-F`

## Items completed

All 7 manifest tasks implemented via TDD, individually committed, each moved to `done`:

- `142.047-T` — Carry the domain error envelope over IPC (`fd30af4e`)
- `142.048-T` — Remove lossy IPC error conversion (`2ebf70cb`)
- `142.049-T` — Add MCP structured success and error transport (`be2d8e94`)
- `142.050-T` — Add CLI JSON success and error transport (`207cc8db`)
- `142.051-T` — Decorate responses with captured-context provenance (`dcc192b5`)
- `142.052-T` — Enforce read-server lifecycle policy (`047a81a8`)
- `142.053-T` — Report generation observability without deletion (`6da09890`)

## Review gate outcome and remediation

First review round (7 personas) returned a mixed verdict: 4/7 `BLOCKED`, 3/7
`READY_WITH_FOLLOWUPS`, converging on a shared finding that the daemon's real
IPC entry point never wires captured-context/generation-activator plumbing
into production traffic.

Deep investigation (git-blame against baseline `655d19e7`, call-site tracing)
determined:

- The core `process_request` → `dispatch_with_read_context`/`admit_read`
  wiring gap is **pre-existing** (identical at baseline, predates 141-S) and
  touches files none of the 7 tasks own (`request_entry.rs`, `ipc_server.rs`).
- `run_read_server_startup` never installing a production `GenerationActivator`
  requires new infrastructure (a real `GenerationStore`) not receivable/
  constructible at that call site today — a design decision, not a mechanical
  fix.
- `get_workspace_status`'s unconditional `connect_db` in ReadServer mode is
  **pre-existing** (present at baseline with a defending comment).

All three were captured as P-021 C2 deferred stash entries (`6C5DF765`,
`9B7EC1E4`, `4628001C`) rather than fixed, since they fail the P-021 C1
same-contract-surface test.

Four genuinely in-scope, low-risk findings were fixed via a remediation
subagent and committed:

1. `watcher_registration_allowed` threaded through the already-resolved
   `DaemonMode` instead of re-reading from disk (`1f42268a`).
2. `ObservabilityState`/`observability_snapshot` made atomic (single lock
   scope) + `runtime_copies` pruning added (`fd397962`, `bf061adf`).
3. CLI's duplicate `translate_ipc_response` vs `run_tool_dispatch` inline
   logic consolidated into one shared path (`f26f9f7e`).
4. Clarifying doc comments on the two-tier (transport vs domain) error
   envelope shape distinction — no behavior change (`bc1073d1`).

A final scoped Correctness-Reviewer re-review of the updated HEAD
(`4b023489`) returned **READY_WITH_FOLLOWUPS**, confirming all 4 remediation
fixes are correct, no new P0/P1 introduced, and the 3 deferrals are
factually sound. One additional P3 advisory note (MCP `translate_ipc_response`
converts domain errors to `isError: true` `CallToolResult` rather than
protocol-level `ErrorData` — a legitimate, test-covered pattern, but a real
behavioral change worth a release-note callout) is carried as a follow-up
handling item in the PR body, not a new stash capture.

## Quality gates (final, at commit `4b023489`)

- `cargo check --all-targets`: clean
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: clean
- `cargo fmt --all -- --check`: clean
- `cargo dev-test` / full suite: only known pre-existing
  `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`
  fails (confirmed identical on baseline `main`); two additional failures seen
  under `--all-targets` parallel load
  (`services::generations::publish::tests::acquire_succeeds_promptly_once_the_lock_is_released`,
  `contract_shim_stdio_initialize::t4_undecodable_result_is_terminal`) both
  independently re-ran green in isolation — confirmed environment-timing
  flakiness under this workspace's constrained parallel-test resources, not
  regressions.

## Branch state

Clean working tree at commit `4b023489f7184d91dfb6937b189d43871b76a71a`.
Not yet pushed. PR not yet created.

## Explicitly out of scope for this session (per operator instruction)

- Shipment `142-S` remains `queued`/blocked — not touched.
- Stash `0011C2DC` / branch `docs/stage-0011c2dc-content-record-schema` — not
  touched, not merged.

## Next steps

1. Push branch, create PR with Local Review Readiness block.
2. Run/monitor CI, P-018 Copilot-review gate.
3. Wait for explicit operator merge approval (P-014) — never auto-merge.
4. Verify merge-commit-only strategy (P-009) before any merge.
5. Post-merge: `post-merge/141-s-*` closure branch, operational-closure,
   compact-context, backlog index resync, source-artifact cleanup, safe-close
   of shipment `141-S` (P-015 partial-feature — this manifest is task-only,
   not a fully-covered root feature, so safe-close applies, not the cascade
   `ship` operation).
