---
title: "Option C — scope/import-aware qualified call resolution & hardening (deferred, blocked)"
doc_type: decision
source: "stash 8CCB9CC3 + B6DF4AD1 (Option C)"
description: "Consolidation of stash 8CCB9CC3 + B6DF4AD1 into a single blocked covering feature (091-F); gated on an explicit operator invariant decision and a prerequisite canonical-identity feasibility spike"
topic: "Recover deferred module::/crate::/Type:: call recall without violating the absolute no-false-edge invariant"
depth: "lightweight"
decision_status: "deferred — blocked pending operator decision + spike"
promoted_to: "backlog"
author: stage
date: 2026-07-15
source_stash_ids:
  - "8CCB9CC3"
  - "B6DF4AD1"
linked_artifacts:
  - "088-F"
  - "081-S"
  - "docs/closure/2026-07-15-088-rec1-call-resolution-adversarial-review.md"
  - "docs/closure/2026-07-15-stage-followups-adversarial-review.md"
  - "PR #248"
---

# Option C — qualified/method call resolution (deferred, blocked)

## Problem frame

081-S / feature 088-F recovered part of the deferred rec1 call-graph recall by shipping the
**sound subset only** (`Self::method()` on inherent impls). The remaining recall —
`module::helper`, `crate::free`, bare `Type::method`, and cross-type associated calls — was
deferred because name/spelling-based resolution reintroduces **false `calls_resolved_singleton`
edges**, violating the **absolute no-false-edge (precision) invariant** stated in deliberation
013-D.

Stash **8CCB9CC3** (the recall scope) and stash **B6DF4AD1** (the hardening requirements that must
land before any qualified/method resolution can ship soundly) describe **one** capability. They are
consolidated here into a single covering feature **091-F: Option C**.

## Decisive evidence (why this is not harvestable now)

1. **088 adversarial review = BLOCK (4/4 reviewers), F1 = P0/HIGH**
   (`docs/closure/2026-07-15-088-rec1-call-resolution-adversarial-review.md`).
   Lowercase-qualified calls (`mem::swap`, `str::parse`, …) collapse to the bare callee and
   mis-resolve to a unique same-named free function — a **new** false edge. The review's verdict is
   explicit: downgrading the precision invariant is *"a design decision the operator must make
   explicitly, not a default."*
2. **081-S halt memory (`session-ship-081S-halt`)** — 10 Copilot rounds + adversarial review proved
   name/spelling qualified resolution is unsound without canonical module/type identity
   (RMeJ0: `impl Widget` uses the impl **source spelling**, not canonical identity, so
   `impl crate::a::Widget` vs an unrelated `b::Widget` mis-resolve). "Operator decision needed."
3. **B6DF4AD1 req (1)** — canonical module/type identity via *per-file use-graph + module-path
   resolution + re-export tracing* is a **new static-analysis capability that does not exist** in the
   indexer today. Its feasibility and scope are unproven; it cannot be decomposed into sound
   ≤2-hour tasks until a spike establishes the approach.

## Options considered

| Option | Description | Precision | Recall | Cost | Verdict |
|---|---|---|---|---|---|
| **A (shipped)** | `Self::`-inherent-only sound subset | Absolute-safe | Partial | Delivered | Status quo baseline |
| **B** | Downgrade invariant to *best-effort, bounded by name collision*; ship name/spelling qualified resolution | Regresses (F1 class returns) | High | Low | **REJECTED as default** — needs explicit operator decision |
| **C (this)** | Build canonical identity (use-graph + module-path + re-export tracing) + unforgeable `Self` marker + generic-parameter normalization + index-format fingerprint/reindex | Absolute-safe | High | High / unproven | **DEFERRED — blocked** |

## Chosen direction

**BLOCK Option C (091-F).** Do **not** queue implementation. Resumption is gated on **two**
prerequisites, made machine-actionable as child artifacts:

1. **Operator design decision** — keep the absolute no-false-edge invariant and fund the full
   Option C build, **or** explicitly downgrade to best-effort precision (Option B). This is a
   deliberate **BLOCK decision**, not an omission.
2. **Prerequisite feasibility spike — 091.001-T** — prove that per-file use-graph + module-path
   resolution + re-export tracing can yield canonical module/type identity for impl-method index
   names *and* call targets, before any decomposition into executable tasks.

## Acceptance gates carried forward (from B6DF4AD1, preserved verbatim as 091-F criteria)

1. Canonical module/type identity for impl-method index names **and** call targets (use-graph +
   module-path resolution + re-export tracing) so `Self::`/`Type::` cannot mis-resolve across
   same-named types or same-type different-spellings.
2. Unforgeable internal `Self` marker (sentinel or typed field) so a source qualifier like
   `Self::Assoc::method()` cannot forge the marker.
3. Consistent generic-parameter normalization for `Type<T>::method` index names.
4. Parser/index-format fingerprint bump or upgrade-triggered reindex so the feature materializes on
   existing DBs (content-hash skip otherwise hides it).

Plus the standing **absolute no-false-edge invariant** (013-D): recall recovery must never create a
false or mis-resolved `calls_resolved_singleton` edge.

## Open questions (for the spike / operator)

- Feasibility and cost of full re-export (`pub use`) tracing across a workspace.
- Reindex/migration cost of an index-format fingerprint bump on large existing DBs.
- Interaction with 084-S durable staged_call provenance (089-F) and the blocked 088-F resolver.
- Whether a partial canonical-identity subset (e.g. in-crate `crate::`/`super::`/`self::` roots only)
  is independently sound and shippable ahead of full re-export tracing.

## Scope / non-actions (Stage boundary)

- Consolidates stash **8CCB9CC3 + B6DF4AD1**; does not modify the blocked **081-S / 088-F** release
  unit manifest — only **informational** `related_to` links are added from the new 091-F artifacts.
- No source/test/schema code authored; Stage planning only.
