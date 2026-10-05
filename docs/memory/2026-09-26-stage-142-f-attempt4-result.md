---
type: session-memory
date: 2026-09-26
agent: stage
feature: 142-F
plan: docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md
plan_review_attempt: 4
outcome: PLAN_REVIEW_ADVISORY (attempt 4) — 0 P0, 0 P1, 9 P2, 11 P3; L3-1 closed; STOPPED before harvest
harvested: false
shipment_assembled: false
supersedes: 2026-09-26-stage-142-f-attempt3-fail-escalation.md (for current gate state only)
---

# Stage 142-F: plan-review attempt 4 (ADVISORY)

## Authorization and scope

The operator directed "Resume work based on"
`docs/scratch/2026-09-26-142-s-restart-handoff.md`. Orchestrator relayed
this as route 1 only: verify revision 5, make minimal consistency fixes,
and run ONE review attempt 4.

This session did NOT:

* harvest;
* change backlog items, stash entries, the 142-S manifest, tasks, or edges;
* change source, tests, or config;
* make a commit, push, or PR;
* take any PA-1..PA-5 decision.

The only files changed are the plan and this memory file. HEAD is
`41dd5081`. The handoff's claim that revision 5 does not exist is stale:
revision 5 exists and was reviewed.

## Revision-5 consistency fixes (before the review; plan text only)

Revision 5 added PRE-3F but did not carry it into the rest of the plan.
Stage fixed:

* the dependency graph (`PRE-2 -> PRE-3F -> PRE-3`) and the execution
  order;
* the no-deadlock note, which now says PRE-3F has no edge to
  `142.058-T`;
* the PA-1 row and approval phrase: they add `<PRE-3F>` and name the
  invariant-6 ownership exception, limited by the diff contract;
* invariant 6;
* the PRE lineage (PRE-3F takes `9B7EC1E4`);
* Decisions D5-A, whose retry wording now matches revision 5;
* Constitution rows:
  * row VI adds PRE-3F;
  * the VII/VIII row is split, and row VIII now names the safety modes.
    Revision 5 claimed the CR-03 fix, but the text had not changed;
* the Runtime Verification table, which gains a PRE-3F row and adds the
  F54 evidence to the PRE-3 row.

No unit's behavior, scenarios, or acceptance was changed.

## Verdicts

| Persona | Verdict | P2 | P3 |
|---|---|---|---|
| Rust | ADVISORY | 1 | 6 |
| Scope Boundary | ADVISORY | 3 | 3 |
| Architecture | ADVISORY | 4 | 2 |
| Constitution | ADVISORY | 3 | 3 |
| Learnings | ADVISORY | 2 | 2 |

**The merged gate is ADVISORY.** After dedup there are 0 P0, 0 P1, 9 P2,
and 11 P3.

## Key conclusions

* **L3-1 is closed.** FASP is deterministic on the normal path on Windows
  and Unix, for both of these tests:
  * the positive `published_fixture_generation_activates_at_startup_and_quiesces`;
  * the negative `unknown_ipc_methods_are_refused_without_side_effects`.
* **The negative test is not weakened.** Its body, assertions, snapshot
  helper, and fingerprint stay byte-identical. The barrier runs only in
  `ensure_daemon`, before `binding_before` and `files_before`.
* **Two residual paths remain.** Both are P2, and both fail loudly rather
  than silently:
  * the install-failure path, where `NoActivator` is not terminal after
    PRE-3 (P2-1);
  * barrier self-interference, where `get_workspace_status` →
    `connect_db` runs between the two compared snapshots (P2-2).
* **Not every attempt-3 P2 was absorbed.**
  * The relay half of CR-04 is claimed but missing from the text (P2-3).
  * The `WorkspaceError::Failed` "correction" is false (P2-4): the
    variant is `HydrationError::Failed`, verified at `errors/mod.rs:20-48`.
  * CR-03 is now absorbed, but only through this session's fix.
  * All the other attempt-3 P2s are absorbed.
* **The other P2s:**
  * P2-5: invariant 7 contradicts the `generation` block change.
  * P2-6: PRE-3 sizing, and `install_generation_gate` is not listed.
  * P2-7: deliberation D5-A and the lineage have drifted from the plan.
    This needs Amendment 3 before the PA-1 phrase is presented.
  * P2-8: the failure signature is too coarse for the multi-row RED
    cases.
  * P2-9: PRE-4's quiescence check may see SQLite sidecar mtimes.

The full list is in the plan under `### Attempt 4: ADVISORY`.

## Next required operator decision

The gate is ADVISORY, so the operator decides whether to proceed despite
the P2s. Any continuation needs NEW explicit authorization:

1. **Harvest authorization.** The operator may authorize Stage to harvest
   PRE-1, PRE-2, PRE-3F, PRE-3, PRE-4b, PRE-4, PRE-5, PRE-6, and
   NEW-1..NEW-6 under 142-F, outside 142-S. That session should first:
   * add deliberation Amendment 3 (P2-7);
   * reconcile stash `EFE9190A` and `4628001C` as the Revision 5
     duplicate-scan record plans;
   * write the P2 fixes into task-level acceptance.

   As an alternative, the operator may first authorize a text-only P2 fix
   pass. That pass would get no further review attempt.
2. **Only after harvested IDs exist:** PA-1, using the filled-in phrase,
   which now includes `<PRE-3F>` and the invariant-6 exception.
3. **Separately:** PA-4 and PA-5.

Stage has stopped. There is no active checkpoint, and no backlog change,
so no index sync was needed.
