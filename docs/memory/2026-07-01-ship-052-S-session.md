# Ship Session Memory — Shipment 052-S (064-F Phase 1a `engram verify` CLI) — Post-Merge Closure

**Date**: 2026-07-01
**Branch merged**: `064-engram-verify-cli`
**PR**: #185 — merged into `main`
**Merge commit**: `f3f7f2f078e46e2ba8c029392fc17f2859e66b8f`
**Merged by / at**: `softwaresalt` @ 2026-07-01T18:14:02Z
**Status**: SHIPPED / CLOSED (operator merged; Ship executed post-merge closure)

---

## Scope

Post-merge closure only — the operator merged PR #185; Ship did **not** merge.
Shipment `052-S` = Phase 1a of feature `064-F` (Deterministic gates & telemetry):
the local, no-daemon `engram verify <path>` structural-conformance linter CLI.

## Backlog Transitions (before → after)

| Artifact | Before | After | How |
|---|---|---|---|
| 052-S (shipment) | active | **done** (archived) | `backlogit move 052-S --status done` + `update 052-S --commit f3f7f2f…` |
| 064.001-T | done | done (unchanged) | already terminal; retained in queue/ |
| 064.002-T | done | done (unchanged) | already terminal; retained in queue/ |
| 064.003-T | done | done (unchanged) | already terminal; retained in queue/ |
| 064-F (feature) | active | **active** (kept open) | ship-note appended to `queue/064-F.md`; deferred 1b/2c/2d remain |

## Hard-Won Landmine: backlogit ID collision (pre-existing data-integrity issue)

The IDs `064-F` and `064.001-T…064.004-T` are **doubly occupied**:
- `queue/`  → the **verify** feature + tasks (this shipment).
- `archive/` → an older, already-shipped **powerbi-TMDL** feature + tasks
  (PR #169, merge `1475200d…`).

`backlogit get/update/move <id>` resolves these collided IDs to the **archived
TMDL** copy, so any backlogit ID op on `064-F`/`064.00X-T` targets the WRONG
artifact. Consequences for closure:

1. **Did NOT run `backlogit shipment ship 052-S`** — it archives the released
   scope (`064-F` + tasks), which would (a) violate keeping `064-F` active and
   (b) overwrite/collide with the archived TMDL records (destroying another
   owner's artifacts + uncommitted reorg drift).
2. Used a **surgical single-artifact `backlogit move 052-S --status done`**
   instead (`052-S` is a unique ID → no cascade, no collision). Verified all
   eight collided `064` queue+archive files were **byte-identical** (SHA-256)
   before and after the transition.
3. `064-F` / `064.004-T` annotations were written by **direct file edit** to the
   `queue/` copies (not `backlogit update`, which would hit the archived TMDL
   twins).

The collision itself is out of Ship scope — flagged for Stage/operator in the
closure doc's Follow-on Backlog ("Backlog ID-reuse reconciliation").

## Working-Tree Drift (left untouched — other owners)

On checkout to `main`, sibling drift was carried over and deliberately NOT
staged/committed/discarded: `.backlogit/archive/064-F.md` (TMDL reorg + 011-D
link), `.backlogit/memories.json`, `.cursor/mcp.json`,
`.github/copilot-instructions.md`, `start.ps1`, untracked `053-S`/`065-*`/`010-D`
queue files, `.claude/`, exec-plans and other memory files. Local `main` was
fast-forwarded via `git fetch origin main:main` so the checkout changed zero
tracked files (merge tree == branch tip tree), preserving all drift.

## Follow-ups Ensured

- **Finding B** — `body.empty` rule lacks a test (`src/services/verify.rs`,
  Copilot thread `PRRT_kwDORJEduc6NrAdB`).
- **Finding D** — Windows verbatim-prefix (`\\?\`) containment in `contain_path`
  via `normalize_canonical` (Copilot thread `PRRT_kwDORJEduc6NrQsy`).

Both recorded in the closure doc's Follow-on Backlog **and** referenced under the
Phase-1b hardening home `064.004-T` (`queue/064.004-T.md`) rather than duplicated
as new tasks.

## Compound Learning Recorded

`docs/compound/security/canonicalize-both-sides-workspace-path-containment-2026-07-01.md`
— resolve relative `<path>` under the canonicalized workspace root (never CWD)
and canonicalize BOTH sides before a `starts_with` containment check.

## Deliverables

- Closure doc finalized: `docs/closure/2026-07-01-052-S-engram-verify-cli-closure.md`
  (merge_sha, merged_by/at, status: shipped).
- Remote + local `064-engram-verify-cli` branch pruned (fully merged).
