# Stage convention — harvest-provenance preservation on stash archival

- **Date:** 2026-07-29
- **Cycle:** Stage cycle 3
- **Source stash:** D2416925 (task/low) — resolved in-cycle (Stage-domain
  bookkeeping; no Ship shipment).
- **Scope:** Stage process/convention + retroactive provenance reconciliation.
  No engram source code.

## Problem

When Stage harvests a stash entry into a backlog artifact and then archives the
stash entry, `backlogit stash archive <ID>` records a **generic**
`reason: archived` on the archived JSONL row (`.backlogit/archive/stash.jsonl`).
The link between the source stash entry and the artifact it was promoted into is
therefore **dropped** — the promotion provenance is lost. PR #293 review flagged
this on the two Cycle-2 archivals:

- `8DD29746` → harvested into feature **101-F** (shipment 094-S)
- `F97D51DF` → harvested into feature **102-F** (shipment 093-S)

both archived with the generic reason, dropping the harvest link
(`.backlogit/archive/stash.jsonl:130–131`).

Note: this is Stage-owned bookkeeping. `backlogit` is a fixed external tool
(v1.7.0) — Stage does not modify its code, and per operator direction planning
artifacts are left uncommitted (the Orchestrator commits). The remedy is a
**durable provenance record via backlog comments/links + this convention**, not
surgery on the tool's archived JSONL store.

## Convention (going forward)

On harvesting a stash entry into a backlog artifact, Stage MUST, **before**
archiving the stash entry:

1. **Append a harvest-provenance comment** to the produced artifact (feature or
   task) via `backlogit_append_comment`, of the form:
   `Harvested from stash <STASH_ID> via Stage pipeline: <plan doc> → <artifact
   IDs>. Assembled into shipment <SHIP_ID>.`
   (This makes the artifact self-describe its stash origin — the durable
   forward link the archived JSONL cannot carry.)
2. **Add a semantic link** where an artifact-to-artifact target exists (e.g.
   `related_to` / `spike_ref` to a deliberation), since `add_link` operates on
   artifacts, not stash IDs.
3. **Archive the stash entry.** `backlogit stash archive <STASH_ID>` records only
   the generic `reason: archived` — the CLI (v1.7.0) has no `--reason` flag
   (verified 2026-07-31) — so the archived JSONL row cannot carry the promotion
   target. The artifact comment (step 1) is therefore the authoritative forward
   link. If a future `backlogit` release adds `--reason`, pass
   `--reason "harvested -> <artifact ID> (shipment <SHIP_ID>)"` as a secondary
   trail.
4. **Record the mapping in the session memory doc** (`docs/memory/<date>/…`)
   under a "Deliverables / provenance" section, as a secondary durable trail.

Deferred-at-deliberation entries are handled differently and unchanged: they stay
**active** and deliberation-linked (the `deliberation_id` on the stash row already
preserves that link — e.g. 015-D/016-D/017-D), and are NOT archived until
harvested or closed.

## Retroactive reconciliation (this cycle)

- **101-F** already carries a harvest-provenance comment for `8DD29746` (added
  Cycle 2).
- **102-F** — provenance comment for `F97D51DF` added this cycle (was missing).
- Both mappings are additionally recorded here and in the Cycle-3 session memory
  doc as the durable trail.

Provenance table:

| Stash ID | Kind | Promoted to | Shipment | Plan doc |
|---|---|---|---|---|
| 8DD29746 | task | 101-F (+101.001/002/003-T) | 094-S (shipped) | `docs/exec-plans/2026-07-28-versioned-codegraph-revalidation-backfill-plan.md` |
| F97D51DF | task | 102-F (+102.001-T) | 093-S (shipped) | `docs/decisions/2026-07-28-cargo-audit-advisory-triage.md` |

## Disposition

Resolved in-cycle by Stage (backlog bookkeeping + convention). D2416925 was
archived with the tool's generic `reason: archived` on the JSONL row; the
descriptive promotion provenance lives in this convention doc and the Cycle-3
session memory, not in the archived record. No Ship shipment — this is
Stage-domain process work, not build/PR/code.

### Reconciliation (2026-07-31, Stage drain-closeout)

An earlier draft of this Disposition asserted that D2416925 was "archived with a
descriptive reason pointing at this doc." The archive record
(`.backlogit/archive/stash.jsonl`) instead stores the generic `reason: archived`,
because `backlogit stash archive` (v1.7.0) does not accept a `--reason` flag
(verified 2026-07-31). We softened the wording to match what the archive actually
stores rather than rewrite the historical archive row. This confirms the
convention's own fallback: when the CLI cannot carry a descriptive reason, the
artifact comment plus this doc are the authoritative promotion links.
