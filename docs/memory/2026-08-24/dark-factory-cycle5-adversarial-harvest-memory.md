---
title: "Dark factory cycle 5 adversarial review and harvest correction memory"
type: session-memory
doc_type: memory
source: "operator-directed PR 363 fail-closed remediation"
date: 2026-08-24
agent: stage
branch: stage/dark-factory-cycle2-20260824-1540
starting_head: d4847e5ae2a1c7d89a1cb5f48fe48d770575dd9b
pull_request: 363
---

# Dark factory cycle 5 adversarial review and harvest correction memory

## Scope and boundaries

Stage reconciled the three adversarial reports against one execution-identity
standard and reversed executable status for the unsupported security/durability
harvest. No production source, test, configuration, build, linter, shipment
claim, shipment close, PR merge, force push, PR metadata mutation, or PR #362
mutation occurred. Shipment `125-S` kept its exact scope and roster.

## Authoritative-evidence decision

No authoritative evidence binds the three rerun responses to the configured
models. The preserved material contains routing configuration and reviewer
self-assertion only; no execution-system response/task IDs or bound provider and
model fields exist. Runtime identity is explicitly recorded as unavailable.
Checked-in frontmatter alone is insufficient, and no requirement change was
recorded.

The initial fail-closed standard remains authoritative. The rerun and final
pass are invalidated, their consensus calculations are withdrawn, and the four
security/durability plans remain failed/unverified.

## Exact backlog dispositions

| Original stash | Active replacement | Feature and tasks | Shipment | Disposition |
|---|---|---|---|---|
| `7B15B447` | `172AE8CE` | `132-F`, `132.001-T`–`132.004-T` | `126-S` | all blocked/non-executable |
| `1CB366DB` | `8C7733CE` | `133-F`, `133.001-T`–`133.004-T` | `127-S` | all blocked/non-executable; still depends on `126-S` |
| `1C2A3CB3` | `721A42F0` | `134-F`, `134.001-T`–`134.003-T` | `128-S` | all blocked/non-executable |
| `5DF94427` | `BD5DD62A` | `135-F`, `135.001-T`–`135.003-T` | `129-S` | all blocked/non-executable |

Backlogit has no native stash restore operation. The original archived records
remain intact with harvest provenance, while supported `stash add` operations
created active replacement entries that name each original ID, blocked feature,
blocked shipment, and exact model-binding blocker. This avoids duplicate IDs
and preserves bidirectional traceability.

`49000348` remains active and separately environment-blocked. `44E573BC`
remains correctly archived to queued feature `131-F` and shipment `125-S`.

## Claim guards

`125-S` is the only queued shipment from this cycle. Review alone never
authorizes claim. Before claim, Ship must fetch `origin/main` and prove all of
the following:

1. Ordering evidence is satisfied: PR #362 merged as `685f62668ac273a41a1f93fc9be2571510decae2`.
2. PR #363 is merged and its exact final reviewed planning head is an ancestor
   of `origin/main`.
3. Review evidence targets that exact final PR #363 head, all review threads are
   resolved, and no review is pending or changes-requested.
4. No competing shipment is active.
5. The exact roster is `131-F` plus `131.001-T` through `131.007-T`, and the strict RED -> dependency GREEN -> provider GREEN -> attachment GREEN -> shutdown GREEN -> runtime VERIFY -> quality VERIFY chain is intact and satisfied.

Shipments `126-S`–`129-S` are blocked and cannot be claimed. If a future valid
review permits requeue, each must also satisfy the five guards above. `127-S`
additionally requires the explicit `127-S -> 126-S` blocks edge and terminal
shipped proof that `126-S` completed through `132.004-T`; review alone never
satisfies that predecessor.

## Review-comment resolution

The PR scope now again matches the existing description: only `125-S` is queued
and the four identity/durability release units are blocked. The rerun standard
is consistent with the initial report. Shipments are not claimable from the
planning branch, and the suppressed handoff finding is corrected by the guard
language above.

## Next steps

1. Validate backlog sync, doctor, statuses, rosters, dependencies, stash
   provenance, documentation structure, and planning-only scope.
2. Commit normally and push the same branch without force.
3. Reply to review ID `5014260193` with the remediation commit and evidence.
4. Resolve only bot threads whose underlying finding is fully addressed.
5. Ship applies any desired PR title/body metadata later; Stage does not mutate
   PR metadata.
