# Decision: Reconcile the `064-*` backlogit ID-namespace collision

- **Date:** 2026-07-01
- **Author:** Stage agent
- **Status:** Accepted & executed (backlog-data reconciliation)
- **Scope:** backlogit workspace data only (`.backlogit/`). No source code, no git branch, no PR.
- **Related:**
  - Verify shipment `052-S` (merge `f3f7f2f078e46e2ba8c029392fc17f2859e66b8f`, PR #185)
  - TMDL shipment (archived `064-S`) merge `1475200dedf983fafbad9a4eb273cc01f69d6d98`, PR #169
  - Closure: `docs/closure/2026-07-01-052-S-engram-verify-cli-closure.md`
  - Spike: `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md`

## 1. Problem

`backlogit sync`/`doctor` report **duplicate IDs** for five artifacts that exist in
**both** `.backlogit/archive/` and `.backlogit/queue/`:

| ID | Archive copy (what it is) | Queue copy (what it is) |
|---|---|---|
| `064-F` | *Rust native tree sitter for TMDL* (feature, `archived`, PR #169) | *Deterministic gates & telemetry — engram Structural Authority* (verify feature, `active`, PR #185) |
| `064.001-T` | *Introduce internal powerbi-tmdl-parser crate boundary* (`archived`) | *Phase 1a — verify linter core service* (`done`) |
| `064.002-T` | *Cover relationship blocks and multiline measure expressions* (`archived`) | *Phase 1a — `engram verify` CLI subcommand* (`done`) |
| `064.003-T` | *Index ref-only model.tmdl shells* (`archived`) | *Phase 1a — cross-platform path normalization* (`done`) |
| `064.004-T` | *Extract top-level TMDL expressions* (`archived`) | *Phase 1b — reactive sync (DEFERRED)* (`queued`) |

Two additional latent conflicts sit in the same namespace:

- **Archived TMDL shipment `064-S`** ("Power BI TMDL parser crate") whose manifest lists
  `064-F, 064.001-T … 064.004-T` (the TMDL items).
- **Live TMDL follow-on tasks** `064.005-T`, `064.006-T`, `064.007-T`, `064.008-T`
  (partitions/M, data sources, refs/annotations, grammar eval) still in the **queue**
  with `parent_id: 064-F`. Because `064-F` also resolves to the *verify* feature, these
  TMDL tasks are effectively mis-parented, and they squat on the `064.005/006-T` slots
  the verify feature reserved for Phase 2c/2d (see the verify feature ship-note).

### Consequence (confirmed)

`backlogit get 064-F` resolves to the **archived TMDL** feature, while the SQL index
collapses the same ID to the **verify** copy — a split-brain. Any
`get`/`update`/`move` against the verify feature is unreliable.

```
$ backlogit get 064-F
id:    064-F
title: Rust native tree sitter for TMDL   <-- WRONG (archived TMDL), verify expected
status: archived
```

## 2. Root cause

Two independent shipments minted work under the **same `064-*` numbers**:

- **PR #169** shipped the TMDL parser crate as `064-F` / `064-S` / `064.001–004-T` and
  archived them; follow-ons `064.005–008-T` stayed queued.
- **PR #185** (Phase 1a `engram verify`) later **re-used** `064-F` / `064.001–004-T` for a
  completely different feature and merged that to `main`.

backlogit keys artifacts by `id` (frontmatter) **and** filename, and permits the same ID
to live once in `queue/` and once in `archive/`. Nothing prevented the number reuse, so
the second minting silently collided with the archived first minting.

## 3. Immutable-history constraint

The **verify** `064-*` IDs are referenced by **immutable merged history**: git commits,
PR #185, merge `f3f7f2f`, the closure doc, the `052-S` shipment manifest, and session
memory. Re-IDing verify would permanently desync backlog data from that history and impose
ongoing cost on the still-open Phase 1b/2c/2d work.

The **archived TMDL** `064-*` IDs are **terminal**: shipped via PR #169, archived, and no
operational backlogit command needs to resolve them by ID anymore. Their provenance is
carried by the immutable `commit: 1475200d…` field on each artifact, independent of the ID.

## 4. Options weighed

### Option A — Re-ID the TMDL family `064-*` → `066-*` (CHOSEN)
Move the entire terminal + follow-on TMDL family to the next free feature namespace
(`066-*`), leaving all of `064-*` to the verify feature.

- **Pro:** Verify keeps `064-*`, staying byte-consistent with merged git/PR/closure/shipment
  history. No divergence on the live, referenced-everywhere work.
- **Pro:** Also resolves the latent conflict — TMDL follow-ons cleanly re-parent to `066-F`,
  freeing `064.005/006-T` for verify Phase 2c/2d exactly as the verify ship-note anticipated.
- **Con:** Mutates archived artifacts and diverges their ID from PR #169's history.
  **Mitigated** by keeping `commit: 1475200d…` on every TMDL item, setting
  `archived_from: .backlogit/queue/066-*.md` per repo convention, and recording the
  old→new mapping in this decision doc + a note in the `066-F` body.

### Option B — Re-ID the verify family `064-*` → `066-*` (REJECTED)
- **Con:** Desyncs active, everywhere-referenced verify work (git, PR #185, merge `f3f7f2f`,
  `052-S` manifest, closure doc, session memory) from immutable merged history, with ongoing
  cost for the open Phase 1b/2c/2d tasks. Maximizes divergence. Rejected.

### Option C — backlogit-native de-duplication (INVESTIGATED → folds into A)
- `backlogit update --id` is **immutable/always rejected** — there is **no** native re-ID/
  rename command.
- `backlogit doctor` **detects** duplicate IDs (and repairs `archived_from`/orphans) but does
  **not** de-duplicate by renaming.
- `backlogit adopt` only **re-parents** an item; it cannot change an item's own ID.

So there is no tool-native re-ID path. Reconciliation must be direct `.backlogit/` markdown
surgery (edit `id`/`parent_id`/refs, rename files) followed by `backlogit sync` +
`doctor` verification. Option C's usable pieces (`adopt` semantics for re-parenting,
`doctor` for verification) are applied **within** Option A's execution.

## 5. Decision

Adopt **Option A**. Re-ID the complete TMDL family `064-*` → `066-*`; the verify feature and
its shipment `052-S` are left untouched at `064-*`.

## 6. Old → New ID mapping

| Old ID | Old file | New ID | New file |
|---|---|---|---|
| `064-F` (TMDL feature) | `archive/064-F.md` | `066-F` | `archive/066-F.md` |
| `064-S` (TMDL shipment) | `archive/064-S.md` | `066-S` | `archive/066-S.md` |
| `064.001-T` (TMDL) | `archive/064.001-T.md` | `066.001-T` | `archive/066.001-T.md` |
| `064.002-T` (TMDL) | `archive/064.002-T.md` | `066.002-T` | `archive/066.002-T.md` |
| `064.003-T` (TMDL) | `archive/064.003-T.md` | `066.003-T` | `archive/066.003-T.md` |
| `064.004-T` (TMDL) | `archive/064.004-T.md` | `066.004-T` | `archive/066.004-T.md` |
| `064.005-T` (TMDL) | `queue/064.005-T.md` | `066.005-T` | `queue/066.005-T.md` |
| `064.006-T` (TMDL) | `queue/064.006-T.md` | `066.006-T` | `queue/066.006-T.md` |
| `064.007-T` (TMDL) | `queue/064.007-T.md` | `066.007-T` | `queue/066.007-T.md` |
| `064.008-T` (TMDL) | `queue/064.008-T.md` | `066.008-T` | `queue/066.008-T.md` |

**Unchanged (verify):** `queue/064-F.md`, `queue/064.001-T.md`, `queue/064.002-T.md`,
`queue/064.003-T.md`, `queue/064.004-T.md`, and shipment `052-S`.

**Reverse-reference updates:**
- `archive/062.003-T.md` body prose `064-F` → `066-F` (the "informs" relationship target).
- verify `queue/064-F.md` ship-note: collision note updated to record resolution (TMDL → `066`).

**Provenance preserved:**
- Every TMDL `066-*` artifact keeps `commit: 1475200d…` (PR #169).
- Each archived TMDL artifact sets `archived_from: .backlogit/queue/066-*.md` (repo convention;
  queue-path restore hint, non-self-referential).
- `066-F` body carries a reconciliation note pointing back to this decision.

`066-*` was confirmed fully free (no files on disk, no index rows, no shipment). `065-*` is the
daemonless-docs feature, so `066` is the next free feature number.

## 7. Verification (post-reconcile)

1. `backlogit sync` — no "collapse to a single indexed row" for `064-*`.
2. `backlogit doctor` — zero `duplicate_id` / `root_id_collision` findings for `064-*`.
3. `backlogit get 064-F` → verify feature; `backlogit get 066-F` → TMDL feature.
4. `066.005–008-T` resolve under parent `066-F`; `064.005/006-T` free for verify Phase 2c/2d.

Pre-existing, out-of-scope `archived_from_self_ref` findings on unrelated items (`001-*`,
`031-*`, `040-*`, `043-*`, `047-*`, `052-*`, `055-*`, `061-*`, `062-F`) are **not** touched by
this reconciliation.

## 8. Deferred to Ship

None required for the data fix. The `066.005–008-T` TMDL follow-ons and the verify Phase
1b/2c/2d tasks remain backlog items to be planned/shipped later through the normal pipeline;
no code or git action is part of this reconciliation.
