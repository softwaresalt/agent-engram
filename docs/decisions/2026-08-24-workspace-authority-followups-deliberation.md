---
title: "Workspace authority follow-up sequencing"
type: deliberation-outcome
doc_type: decision
source: "stashes 1CB366DB and 7B15B447; PR #353 review"
date: 2026-08-24
status: decided
source_stash_ids: ["1CB366DB", "7B15B447"]
backlog_deliberations: ["021-D", "022-D"]
promoted_to:
  - docs/exec-plans/2026-08-24-7b15b447-daemon-key-engram-caproot-plan.md
  - docs/exec-plans/2026-08-24-1cb366db-bind-proof-composition-plan.md
---

# Workspace authority follow-up sequencing

## Problem Frame

PR #353 retained the workspace `CapRoot`, but two composition windows remain: `daemon_key_for_workspace` reopens `.engram` between its probe/read/fallback operations, while `set_workspace_with_probe` separately resolves canonical path, UUID, and branch. The two defects share a principle but cross different module boundaries.

## Research Findings

`src/db/workspace.rs` shows three independent `.engram` opens in `workspace_id_present_via`, `workspace_id_from_metadata`, and `read_pid_file_via`. `src/tools/lifecycle.rs` lines 337-340 independently call `canonicalize_workspace`, `load_or_create_workspace_id`, and `resolve_git_branch`. The compound learning from 121-S states that capability conversion is a consumer-graph problem.

## Options Evaluated

1. One broad feature and shipment: one security review, but mixes internal identity composition with lifecycle binding and exceeds width isolation.
2. Separate ordered release units: first retain one `.engram` authority, then expose and consume one combined bind proof.
3. Separate unordered units: smallest scopes, but permits the combined bind API to rely on an identity path that still reopens `.engram`.

## Decision

Choose option 2. `7B15B447` is the prerequisite release unit; `1CB366DB` follows. This is sequencing by dependency, not a claim that the daemon-key defect has greater business impact than the main bind defect. Each plan preserves its own deterministic RED and precise GREEN edge.

## Constraints

- No public `cap_std` type escapes `src/db/workspace.rs`.
- Every production task has a paired failing test first.
- Both plans require hardening, standard plan review, and operator-requested adversarial multi-model review.
- The current session has no independent reviewer dispatch surface; both harvests therefore fail closed at that final gate.
- Do not assemble dependent shipments until ordering can be represented safely; the current shipment creation surface cannot author arbitrary operator batch metadata.

## References

- Backlog deliberations `021-D`, `022-D`
- `docs/compound/capability-rewrite-must-convert-every-consumer-2026-08-21.md`
- `docs/closure/2026-08-21-568b257c-runtime-verification.md`
- PR #353 review follow-ups
