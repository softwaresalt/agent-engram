---
title: "Adversarial + plan review — Stage stash-followups (091-F Option C block, 092-F atomicity, 088.005-T reconcile)"
type: closure
doc_type: closure
source: "stash 8CCB9CC3, B6DF4AD1, 32DAA85B, 6870ECDF"
date: 2026-07-15
slug: stage-followups-adversarial-review
subject_scope: "stash 8CCB9CC3, B6DF4AD1, 32DAA85B, 6870ECDF → 091-F / 092-F / 086-S dispositions"
reviewers: 1
review_mode: "single-agent Stage session, multi-lens (concurrency / constitution / security / scope-boundary / architecture / rust-safety)"
consensus_note: "F9-equivalent: single-agent findings carry confidence tiers; downstream Ship review + Copilot PR review supply cross-model consensus before any merge of 086-S"
verdict: "PASS — Option C BLOCKED (evidence-based); 092-F/086-S SHIP-WITH-FIXES (all HIGH P1 remediated in-plan); 6870ECDF DEFER-AS-BLOCKED"
gate_blocking: true
---

# Adversarial + Plan Review — Stage stash-followups

Operator mandated adversarial review of **every material technical decision and every
implementation plan before harvest/shipment assembly**. This session has one executable plan
(092-F atomicity) and three disposition decisions (Option C block, consolidation, 6870ECDF defer).
Applied the `adversarial-review` + `plan-review` + `plan-harden` discipline; multi-lens single-agent
per the established 088.001-R pattern (F9-equivalent noted above).

Confidence: **HIGH** = source/evidence-confirmed and decision-gating · **MEDIUM** = probable, fix or
disposition required · **LOW** = advisory.

## 0. Verdict

- **0 P0.** No data-corruption, no daemon-crash, no security bypass, no boundary violation.
- **HIGH-confidence P1** findings D2-a, D3-a, D3-b, D3-d, D4-a — **all remediated in-plan / in-disposition**.
- Option C (091-F) is **BLOCKED** on evidence (explicit block decision, not omission).
- 092-F / shipment 086-S is **SHIP-WITH-FIXES → PASS** for harvest.
- 6870ECDF is **DEFER-AS-BLOCKED** — resolving-in-planning would breach the 081-S mutation boundary.

## D1 — Consolidate 8CCB9CC3 + B6DF4AD1 → one Option C feature (091-F)

- Lens: scope-boundary / architecture.
- Challenge: does merging hide distinct work?
- Evidence: B6DF4AD1 states *"Ties to stash 8CCB9CC3."* 8CCB9CC3 = recall scope; B6DF4AD1 = the
  hardening acceptance for the **same** capability (canonical module/type identity). Req (1) of
  B6DF4AD1 **is** 8CCB9CC3's core requirement.
- **D1-a [P3, LOW]:** folding B6DF4AD1 as prose could bury its 4 discrete hardening requirements.
  **Remediated:** all 4 preserved verbatim as 091-F `acceptance-criteria`; both stash IDs recorded
  in body + source provenance.
- Verdict: **consolidation SOUND.**

## D2 — BLOCK Option C (091-F) rather than plan/queue it

- Lens: architecture-strategist / correctness.
- Challenge: is any sound subset shippable now (avoid over-blocking)?
- Evidence: 088 adversarial review **BLOCK 4/4, F1 P0** (name/spelling qualified resolution creates
  false edges to real fns); absolute no-false-edge invariant **not satisfiable** without canonical
  identity; `Self::`-inherent sound subset **already shipped**; remaining recall requires a **new,
  unproven** use-graph + module-path + re-export-tracing capability; 088-review verdict: invariant
  downgrade is an **explicit operator decision, not a default**.
- **D2-a [P1, HIGH]:** queuing Option C now would either reintroduce the F1 false-edge class
  (invariant violation) or spawn tasks that exceed the ≤2h rule with unsound acceptance.
  **Remediated:** BLOCK; require (1) operator invariant decision + (2) prerequisite spike 091.001-T.
- **D2-b [P2, MEDIUM]:** a blocked feature with no next step is dead backlog.
  **Remediated:** 091.001-T (spike) + deliberation doc make resumption machine-actionable.
- Verdict: **BLOCK SOUND and REQUIRED** (explicit block decision per constitution/unsafe-block rule).

## D3 — 32DAA85B independently executable → 092-F / 092.001-T / shipment 086-S

- Lens: concurrency-reviewer / rust-safety / release-observability / plan-harden.
- Challenge: does the atomic writer introduce deadlock, partial publish, latency, or a vacuous test?
- Evidence (source-verified @ df77584): reader `snapshot_dispatch_context` locks
  `active_workspace` → `workspace_config`; no site holds both in the opposite order.
- **D3-a [P1, HIGH — concurrency]:** if `set_workspace_and_config` acquires config before workspace
  it inverts the reader's order → deadlock risk. **Remediated (mandatory in plan):** acquire
  workspace-then-config; document the global lock order at both hold sites.
- **D3-b [P1, HIGH — correctness]:** the `LimitReached` capacity check must run before publishing
  either value, under both guards, with no partial publish on error. **Remediated in plan.**
- **D3-c [P2, MEDIUM — latency]:** bind path has a 500 ms SLA (029-F WS-6). **Remediated:** no
  `.await` on I/O under the write guards; only trivial moves/clones.
- **D3-d [P1, HIGH — test validity]:** the existing 086.004 atomicity test routes transitions
  through a neutral workspace `N` and asserts only A/B cross-pairs → **cannot** detect the
  writer-side tear (vacuous for this bug). **Remediated (test-first):** non-vacuous A→B/B→A test
  that FAILS on the current two-await writer and PASSES after the fix.
- **D3-e [P3, LOW — constitution]:** no new deps; `#![forbid(unsafe_code)]` intact. **Confirmed.**
- **D3-f [P2, MEDIUM — release-observability]:** runtime-affecting bind path. **Remediated:**
  shipment carries monitoring (WS-6 SLA unchanged; status never reports new-ws/old-config) + rollback
  (revert to two-await; no schema/format change → trivial).
- **D3-g [P2, MEDIUM — blast radius]:** shared publish path → escalate to **plan-harden**.
  **Remediated:** plan-harden section authored (lock order, bounded critical section, no partial
  publish, independent test, rollback, monitoring).
- Verdict: **SHIP-WITH-FIXES → PASS.** All HIGH P1 (D3-a/b/d) encoded into 092.001-T acceptance +
  the exec plan before harvest. Coherent + executable → queue **086-S**.

## D4 — 6870ECDF → blocked reconciliation task (091.002-T), NOT resolved-in-planning

- Lens: scope-boundary-auditor.
- Challenge: could Stage "safely" narrow 088.005-T now?
- Evidence: 088.005-T is a member of the **blocked 081-S / 088-F** release-unit manifest. Operator:
  blocked 081-S is outside mutation scope except **safe informational links or explicit follow-up
  dependencies**; the stash itself defers to *"081-S / Option-C resumption"* and is *"preserved
  verbatim … not re-adjudicated."*
- **D4-a [P1, HIGH — scope]:** editing 088.005-T status/title/acceptance now would mutate the blocked
  release unit **and** re-adjudicate a preserved decision → **boundary violation.**
  **Remediated:** create a **new** blocked follow-up (091.002-T) depending on Option C resumption;
  only **informational** `related_to` links to 088-F / 088.005-T; 088.005-T left untouched.
- **D4-b [P2, MEDIUM]:** both remediation options must survive verbatim. **Remediated:** both (narrow
  vs reopen) recorded in 091.002-T acceptance.
- Verdict: **DEFER-AS-BLOCKED SOUND**; 6870ECDF is a dependency-on-Option-C follow-up, not a
  Stage-resolvable state correction.

## Cross-cutting constraints (verified)

- 083-S / 084-S / 085-S manifests: **untouched** (no `add_to_shipment`, no edits).
- 081-S / 088-F manifest and 088.005-T: **not modified**; only informational links from new artifacts.
- 082-S: shipped/archived; not modified.
- No source/test/config code authored; no build/PR/merge (Stage boundary; P-010 respected).
- New shipment 086-S is queued (not claimed) — Ship owns claim/close.

## Remediation queue (confidence × severity)

| ID | Sev | Conf | Action class | Disposition |
|---|---|---|---|---|
| D2-a | P1 | HIGH | manual | BLOCK Option C + spike gate — DONE |
| D3-a | P1 | HIGH | manual | lock order workspace→config — encoded in 092.001-T + plan |
| D3-b | P1 | HIGH | manual | capacity-check-before-publish — encoded |
| D3-d | P1 | HIGH | gated_auto | non-vacuous A→B/B→A test-first — encoded |
| D4-a | P1 | HIGH | manual | defer as blocked; do not touch 088.005-T — DONE |
| D3-c | P2 | MED | manual | no await under lock — encoded |
| D3-f | P2 | MED | manual | monitoring + rollback in shipment — DONE |
| D3-g | P2 | MED | manual | plan-harden section — DONE |
| D2-b | P2 | MED | manual | resumption artifacts (spike + deliberation) — DONE |
| D4-b | P2 | MED | advisory | both options preserved — DONE |
| D1-a | P3 | LOW | advisory | 4 hardening reqs preserved — DONE |
| D3-e | P3 | LOW | advisory | deps/unsafe clean — confirmed |

**Gate outcome: PASS.** No P0; all HIGH-confidence P1 remediated in-plan or dispositioned before
harvest. Runtime-affecting unit 092-F carries monitoring + rollback expectations for Ship's
release-observability.
