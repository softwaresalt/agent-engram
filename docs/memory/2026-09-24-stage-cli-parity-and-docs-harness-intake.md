# Stage — CLI Parity Requirement and Docs-Only Harness Contradiction

- **Date**: 2026-09-24
- **Agent**: Stage (normal mode, no dark mode)
- **Branch**: `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs`. This is Ship's feature branch. Stage made no commit.
- **Status**: awaiting operator confirmation of two deliberations and an operator-authored disposition for 142.059-T

## Gates

- Tool gate: the registry was present and the backlog MCP tools were unavailable in-session, so this session ran in `DEGRADED_MODE` on the registered backlogit CLI fallback. Engram CLI was healthy.
- `backlogit sync` returned `INDEX_SYNC_OK`.
- Checkpoints: all 29 listed, with no anomalies and no active Stage-owned checkpoint. Zero candidates, so normal startup.
- Hook poll returned no events.
- Step 1.5 grouping was skipped. Both intake entries are feature/chore-shaped.
- Step 1.8 learnings came back low confidence. The useful hits are recorded in the CLI deliberation.

## Intake

| Stash | Kind / priority | Deliberation |
|---|---|---|
| `E06BABAD` | feature / high | Operator CLI-parity requirement. `035-D`; `docs/decisions/2026-09-24-cli-parity-all-user-facing-tools-deliberation.md` |
| `1AD161B8` | chore / critical | Docs-only Step 2 harness contradiction. `036-D`; `docs/decisions/2026-09-24-docs-only-task-harness-disposition-deliberation.md` |

## Changes to Active 142-S Planning Text

The 142-S manifest was not modified.

- `142.058-T` (F54):
  - Acceptance criterion 6 now says `_health` is internal IPC only and `get_retrieval_eval_report` is exercised on its declared surfaces. It also says no test hard-codes CLI absence and F54 adds no CLI command.
  - An implementation-notes amendment was appended and a Stage comment added.
  - Labels and Ship harness notes are preserved.
- `142.058.002-ST`: the matching acceptance lines were corrected.
- `142.059-T`: a comment was added covering the harness-not-applicable status, the operator-disposition need, and the rule that docs must not call any user-facing tool "MCP-only by design". No text or label change.
- `090.004-T`: a comment proposes archiving it as superseded by `E06BABAD` at harvest.
- `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`, item 10: an amendment note was added saying the CLI requirement now exists.

## Open Gates

1. **Operator: disposition for 142.059-T.** Recommended: a one-time scoped Step 2 disposition that accepts its three recorded verification commands. Stage cannot grant it.
2. **Ship: rework the F54 harness.** Remove the `!surfaces.contains(Cli)` assertion for `get_retrieval_eval_report` in `tests/contract/read_server_cli_mcp_parity_test.rs`, then re-verify the red phase.
3. **Operator: confirm the deliberations.** 035-D recommends Option A (a new covering feature after 142-S). 036-D recommends Option 1 now and Option 2 via the harness template channel. Planning, harden, review, harvest, and shipment assembly have not been run for either.

## Sequencing

1. 142-S, the sole active release unit.
2. The harness-policy amendment, through the template channel, which is not a Ship release unit.
3. The CLI parity feature, as its own shipment after 142-S ships.

## Deferred

The stash entries have not been archived; they are not consumed until harvest. `E53CB236` (a P-021 entry targeting F53 dependency order in 142-S) was not processed this session.
