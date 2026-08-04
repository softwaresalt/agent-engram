---
title: "104-S post-merge compound refresh"
date: 2026-08-04
shipment: 104-S
feature: 109-F
mode: apply
---

# 104-S Post-Merge Compound Refresh

## Scope

Reviewed workflow learnings implicated by shipment archival and the
single-authority coordinator closure.

## Classifications

| Entry | Classification | Evidence |
|---|---|---|
| `backlogit-ship-blocked-child-expansion-2026-04-26.md` | keep | Its parent-removal workaround was required for `104-S` and succeeded. |
| `backlogit-shipment-ship-force-releases-covering-feature-2026-04-22.md` | keep | Backlogit still mutates scope derived from feature relationships. |
| `ship-shipment-overscoped-manifest-2026-04-20.md` | keep | Pre/post reconciliation and explicit containment remain necessary. |

## Applied Maintenance

No compound entry required rewriting, consolidation, replacement, or archival.
The existing blocked-child-expansion entry exactly described the refusal and
the safe workaround used here. No new compound entry was created.

## Follow-Up

Backlogit still returns excluded blocked children to queued during shipment
closure. The closure report records the restoration, but the behavior remains
an upstream tool advisory rather than new institutional knowledge.
