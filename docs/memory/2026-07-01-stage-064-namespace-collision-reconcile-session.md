# Stage session — `064-*` ID-namespace collision reconciliation

- **Date:** 2026-07-01
- **Agent:** Stage
- **Type:** DATA-INTEGRITY reconciliation (backlog data only; no code/git/PR)
- **Decision doc:** `docs/decisions/2026-07-01-064-id-namespace-collision-reconciliation.md`

## What happened

`backlogit sync`/`doctor` reported duplicate IDs across `.backlogit/archive/` and
`.backlogit/queue/` for `064-F` and `064.001–004-T`. Root cause: **two shipments minted
work under the same numbers** —

- PR #169 (merge `1475200d`) shipped the **TMDL parser crate** as `064-F`/`064-S`/`064.001–004-T`
  (archived) with follow-ons `064.005–008-T` still queued.
- PR #185 (merge `f3f7f2f`) later **re-used** `064-F`/`064.001–004-T` for the **`engram verify`**
  feature and merged that to `main`.

`backlogit get 064-F` resolved to the archived TMDL feature (wrong) while the SQL index
collapsed the ID to the verify copy — a split-brain making verify ops unreliable.

## Decision (Option A)

Keep the **verify** family at `064-*` (referenced by immutable merged history: git, PR #185,
merge `f3f7f2f`, `052-S` manifest, closure doc, session memory) and re-ID the **terminal TMDL**
family to the next free namespace `066-*`. Rejected Option B (re-ID verify) as maximizing
divergence from merged history. Option C (native tooling) is impossible: `backlogit update --id`
is immutable, no rename/re-ID command exists; `doctor` only detects, `adopt` only re-parents —
so reconciliation was direct `.backlogit/` markdown surgery + `sync` + `doctor` verification.

## Old → New mapping (executed)

| Old | New | Location |
|---|---|---|
| 064-F | 066-F | archive (feature, TMDL) |
| 064-S | 066-S | archive (shipment) |
| 064.001–004-T | 066.001–004-T | archive (shipped tasks) |
| 064.005–008-T | 066.005–008-T | queue (follow-on tasks, re-parented to 066-F) |

Unchanged: verify `064-F`, `064.001–004-T`, and shipment `052-S`.

## Mutations performed (before → after)

- Renamed 10 files `064-* → 066-*`; edited each `id`, `parent_id`, and `archived_from`
  (`.backlogit/queue/066-*.md`, per repo convention; kept `commit: 1475200d`).
- Added an `id-reconcile` note + `archived_from` to `066-F` (feature previously had none).
- Updated shipment `066-S` items list + title + body to `066-*`.
- Reverse refs: `archive/062.003-T` prose `064-F → 066-F` (informs target); verify `064-F`
  ship-note now records `064.005/006-T` slots as FREE for Phase 2c/2d.
- Appended audit comments to `064-F` (verify) and `066-F` (TMDL).

## Verification (proof collision is gone)

- `backlogit sync` → Indexed 541 artifacts, no collapse warnings.
- `backlogit doctor` → **0** `duplicate_id`, **0** `root_id_collision`; nothing flagged for
  `064`/`066`. Total findings 49 → 43; the 43 remainder are pre-existing, out-of-scope
  `archived_from_self_ref` on unrelated items (`001/031/040/043/047/052/055/061/062-*`).
- `backlogit get 064-F` → verify (active); `backlogit get 066-F` → TMDL (archived).
- Index: one row per ID; `066.005–008-T` parent = `066-F`.

## Deferred to Ship

None. Pure backlog-data fix — no code, branch, or PR required. The `066.005–008-T` TMDL
follow-ons and verify Phase 1b/2c/2d remain normal-pipeline backlog items.

## Follow-up for operator/Ship (NOT done by Stage per instruction)

Working-tree changes (10 renamed `.backlogit` files, edited `062.003-T`/`064-F`, new decision +
memory docs) are **uncommitted** — the operator instructed no git operations this session.
Whoever commits should stage only these `064/066` reconciliation files, leaving unrelated
working-tree drift alone.

## Compound learning

**backlogit ID-namespace collisions across merged shipments:** backlogit permits the same `id`
to exist once in `queue/` and once in `archive/`; nothing blocks number reuse. When it happens,
`get/update/move` (archive-preferring) and the SQL index (queue-preferring) disagree — a
split-brain. There is **no native re-ID** (`update --id` is rejected; `doctor` detects only;
`adopt` re-parents only), so the fix is manual markdown surgery: edit `id`/`parent_id`/
`archived_from`, rename files, then `sync` + `doctor`. **Rule of thumb:** re-ID the *terminal/
archived* side to a fresh namespace and preserve the *active, merged-history-referenced* side,
because immutable git/PR/closure/shipment references make the merged ID effectively load-bearing.
Preserve provenance on the moved side via the retained `commit:` SHA + `archived_from` queue path
+ a body note + a decision doc mapping. **Prevention:** allocate the next feature number from the
global max across BOTH queue and archive (not just the active queue) before minting a shipment.
