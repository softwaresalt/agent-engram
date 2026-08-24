# Stage session — Copilot `server/discover` MCP compatibility

**Date:** 2026-08-23
**Agent:** Stage
**Branch:** `stage/130-engram-copilot-server-discover-20260823`
**Worktree:** `.worktrees/stage-130-copilot-server-discover-20260823`

## Outcome

Assembled queued shipment **124-S** covering feature **130-F**, ready for Ship.
No source code written, no build run, no commit, no PR — per operator constraint.

## Artifacts

| Artifact | Path |
|---|---|
| Investigation (supplied, carried onto branch, not duplicated) | `docs/decisions/2026-08-23-copilot-prerelease-server-discover-mcp-compatibility-spike.md` |
| Implementation plan (hardened) | `docs/exec-plans/2026-08-23-copilot-server-discover-compat-plan.md` |
| Plan review gate | `docs/reviews/2026-08-23-copilot-server-discover-compat-plan-review.md` |

## Backlog

* `001-SP` — spike anchor for the completed investigation (done)
* `130-F` — covering feature (queued)
* `130.001-T` … `130.005-T` — U1 RED, U2 GREEN, U3 integration, U4 regression guard, U5 rollback/docs
* `130.001-R` — plan review gate (done, approved-with-changes)
* `002-SP` — Cozo cold-start profiling spike, **deliberately excluded** from 124-S
* `124-S` — queued shipment, 6 members (130-F + 5 tasks)

## Key decisions

1. **Narrow allowlist.** Review finding F2 cut the interception scope from "any
   id-bearing pre-initialize method" to **exactly `server/discover`**. Everything
   else forwards to rmcp unchanged. This keeps blast radius at the reproduced
   defect and avoids masking genuine client ordering bugs.
2. **`-32601`, not a semantic implementation.** `server/discover` is undocumented
   in the `1.0.81-8` prerelease notes. The GitHub MCP server already proves
   Copilot tolerates a method-not-found refusal in the same run.
3. **Kill-switch named** `ENGRAM_MCP_PREINIT_COMPAT` (default on, `0` disables)
   so rollback needs no redeploy (finding F1).
4. **`id: 0` round-trip asserted explicitly** (finding F4) — zero is the classic
   falsy-id serialization bug and the evidence shows Copilot uses exactly `0`.
5. **Timeout increase rejected**, per the spike. Recorded as out of scope.
6. **Cozo cold start split out** into `002-SP` to preserve width isolation.

## Injection point (for Ship)

`src/shim/transport.rs::run_shim` currently does:

```rust
let transport = rmcp::transport::io::stdio();
let running = rmcp::serve_server(handler, transport).await?;
```

The wrapper is interposed between these two lines. Serve-first work (124-F /
870B1AFF) is already in `main` and is necessary but not sufficient.

## Blockers / handoff constraints

* **`main` cannot receive this work yet.** `.backlogit/archive/stash.jsonl` has a
  stranded `UU` unmerged index entry (no `MERGE_HEAD`) with literal
  `<<<<<<< Updated upstream` / `>>>>>>> Stashed changes` markers from an
  unresolved `git stash pop`. The file is invalid JSONL. Operator must resolve.
* **Pre-existing HEAD defect.** 18 archived artifacts plus
  `.backlogit/queue/030.005-C.md` fail to parse at `HEAD` (missing/oversized
  `title:` frontmatter). `main`'s uncommitted staged edits are exactly the
  repair. `backlogit sync` therefore hard-errors in a worktree built from `HEAD`.
  Not caused by this session; all 10 new artifacts parse and read back cleanly.

## Next steps

1. Operator resolves the `stash.jsonl` conflict and commits the pending
   artifact-title repairs on `main`.
2. Land this stage branch, then Ship claims `124-S`.
3. Triage `002-SP` separately.
