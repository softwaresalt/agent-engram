# Plan Hardening — Shipment Safe-Close Contract Reconciliation

- Date: 2026-09-17
- Agent: Stage
- Plan: `docs/exec-plans/2026-09-17-shipment-safe-close-contract-reconciliation-plan.md`
- Gate: P-006 (plan declared `Requires plan hardening: yes`)

## Why hardening applies

This plan rewrites the contract documents that govern **every future shipment
closure** in this workspace. Blast radius is not the diff size — it is that a
defect introduced here silently mis-governs all subsequent Stage/Ship runs, and
the defect class this plan exists to fix (contract text diverging from tool
behavior) is exactly the class a careless edit would reintroduce.

Two structural hazards additionally apply: the plan **modifies the agent
templates of the agents executing it**, and the plan's keystone decision rests on
an assumption three separate stash entries explicitly flag as **unconfirmed**.

## R1 — D2's predicate rests on an unverified assumption (CRITICAL)

**Risk.** D2 redefines eligibility as `archived_status` in {`done`,`shipped`}.
That is a *documentation* change. It only unblocks the chain if backlogit's
**actual** eligibility check does not itself gate on the literal string
`shipped`. All three recurrences (77A4E71C, F35EA0E6, 76153F55) state this is
**unconfirmed** — they record only that claims empirically succeeded, and
explicitly list "because Stage assembles shipments outside that check's code
path" as an unexcluded alternative explanation.

If the tool *does* gate on `shipped`, T3 produces a contract that claims 140-S is
eligible while the tool refuses to claim it — replacing an ambiguity with a
falsehood, and leaving the chain just as blocked.

**Mitigation — MANDATORY new task T0, blocking T3.** Confirm the actual
mechanism before enshrining D2. T0 is a verification spike, not a doc task.
It must determine empirically which of these holds:

- (a) the eligibility check reads `item_deps`/`blocks` edges and an archived
  predecessor satisfies them regardless of `archived_status`;
- (b) the check gates on a literal status string;
- (c) no tool-side check runs at all for shipments and eligibility is
  agent-enforced only.

If (b), **D2 is invalid as written** and the plan halts for re-deliberation —
the fix would then require an upstream tool change (Option 2), which is outside
this workspace. Recorded as an explicit fail-closed branch.

## R2 — `abandoned` must not satisfy a `blocks` edge (CRITICAL)

**Risk.** The workspace contains **five abandoned shipments** (125-S, 126-S,
127-S, 128-S, 129-S). An abandoned shipment is archived. If D2's predicate is
implemented as "archived, with an `archived_status` set", or if the enumeration
is read loosely, an **abandoned predecessor would wrongly satisfy a `blocks`
edge** — allowing a successor to claim work whose prerequisite was never done.

This is a strictly worse failure than the ambiguity being fixed: it converts a
false *block* into a false *unblock*.

**Mitigation.** T3's acceptance criteria must require the predicate to be
written as a **closed allow-list** of exactly {`done`, `shipped`}, with an
explicit statement that `abandoned` — and any status not in the allow-list —
does **not** satisfy a `blocks` edge. A negative test case naming `abandoned`
must appear in the rule text.

## R3 — Self-modifying contract executed by its own subjects (HIGH)

**Risk.** T2 rewrites `_ship.agent.md` Step 6, which is the procedure Ship uses
to close shipments — including **this very shipment**. T4 rewrites
`_stage.agent.md` Step 5.5, which governs the assembly of this shipment. An
agent reading a half-applied contract mid-run can close incorrectly.

**Mitigation.**

- **Atomicity**: all tasks ship in **one** shipment and **one** PR. No partial
  landing of T1-T4 is permitted; a document set that disagrees with itself is
  the defect being fixed.
- **Effective-from rule**: the revised contract takes effect for the **next**
  closure after this shipment merges. This shipment's own closure runs under the
  procedure in effect at claim time. T2 must carry this as an explicit note so
  Ship does not switch procedures mid-run.
- **Ordering**: T1 (the authoritative specification) lands before T2/T3, which
  are restatements of it. Already encoded in the dependency order.

## R4 — Eligibility relaxation misread as weakening P-015 (HIGH)

**Risk.** T3 relaxes an *eligibility* predicate in the same shipment where T6
restates a *closure* prohibition. A future reader may conflate them and conclude
P-015 was softened, re-enabling the cascade this whole feature exists to prevent.

**Mitigation.** Add an explicit **non-goal** to the plan and to T3's rule text:
"This change alters only the dependency-eligibility predicate. It does not
alter, weaken, or create any exception to P-015's prohibition on calling
`backlogit shipment ship` / `backlogit_ship_shipment` for partial-feature
shipments." T6 must land in the same shipment so the prohibition is
simultaneously strengthened.

## R5 — T5 mutates operator-restricted shipments (HIGH)

**Risk.** T5 backfills `141-S -> 139-S`. The operator placed 140-S / 141-S /
142-S off-limits for this session ("do not mutate, claim, implement, build, or
create PRs"). An agent executing T5 without re-reading that constraint would
violate it.

**Mitigation.** T5 carries a **hard precondition gate**: it must not execute
without recorded, explicit operator authorization naming the shipment IDs to be
mutated. Absent authorization, T5 returns `blocked` with the reason — it does
not proceed, and does not block the rest of the shipment (T1-T4, T6-T7 are
independent of it). T5 is therefore placed **last** in execution order and is
individually returnable as blocked.

## R6 — P-015's verify-after-each invariant could be weakened in restatement (MEDIUM)

**Risk.** T1 restates the P-015 protected-set invariant inside a new skill mode.
Restating a safety invariant is a classic site for accidental weakening — for
example, verifying the protected set once at the end instead of after **each**
archived item, which would fail to localize a cascade.

**Mitigation.** T1's acceptance criteria must require the invariant to be
verified **after every single manifest item**, and must require the
`pre-archived` exemption to apply to **manifest items only** with **no**
exemption for protected-set members — matching `workflow-policies.md` verbatim
in effect. A reviewer must diff the restatement against the policy text.

## R7 — Loss of stash provenance on closure (MEDIUM)

**Risk.** Ten stash entries are consumed by this feature. If they are archived
without forward references, the three-times-recurring eligibility question loses
its audit trail, and a fourth recurrence would be re-triaged from scratch.

**Mitigation.** Each consumed stash entry is archived with a forward reference to
the backlog item it became, and the deliberation artifact retains the full
provenance table (already written, §1). 20FDC0A7 and B761AFA7 remain **active**
(not consumed) and must not be archived by this session.

## Hardening-mandated plan amendments

| # | Amendment |
|---|---|
| A1 | **Add T0** — verification spike confirming backlogit's actual eligibility mechanism. Blocks T3. Fail-closed branch: if the check gates on a literal `shipped`, halt for re-deliberation. |
| A2 | T3 predicate written as a closed allow-list {`done`,`shipped`}; `abandoned` explicitly excluded with a named negative case. |
| A3 | T2 carries an effective-from note; all tasks ship atomically in one shipment/PR. |
| A4 | Explicit non-goal added: eligibility relaxation does not weaken P-015. |
| A5 | T5 gated on recorded operator authorization; individually returnable as `blocked`; moved last. |
| A6 | T1 acceptance criteria require verify-after-**each** and the manifest-items-only pre-archived exemption. |
| A7 | Consumed stash entries archived with forward references; 20FDC0A7 and B761AFA7 remain active. |

## Amended task list

| Task | Title | Size | Complexity | Depends on |
|---|---|---|---|---|
| T0 | Verify backlogit shipment dependency-eligibility mechanism | S | high | — |
| T1 | Specify `mode: safe-close` in the shipment-reconcile skill | M | high | T0 |
| T2 | Correct `_ship.agent.md` Step 6 1.b to the achievable sequence | S | medium | T1 |
| T3 | Redefine the dependency-eligibility predicate (allow-list) | S | medium | T0, T1 |
| T4 | Add the shipment-edge derivation rule to Stage assembly | S | medium | T1 |
| T5 | Audit and backfill missing shipment `blocks` edges | S | low | T4 + operator auth |
| T6 | Correct the compound doc's P-015 framing and cascade disposition | S | medium | — |
| T7 | Record the superseded manifest-edit remedy and upstream defect | S | medium | T6 |

## Amended dependency order

```
T0 ──┬─► T1 ──┬─► T2
     │        ├─► T3
     └────────┘   └─► T4 ──► T5 (operator-gated)
T6 ──► T7
```

Execution order: **T0 -> T1 -> T2 -> T3 -> T4 -> T6 -> T7 -> T5**

T5 moved to last so an authorization block cannot stall the rest of the shipment.

## Revised effort

8 tasks x 2 hours = ~16 hours human-equivalent.

## Residual risk accepted

- The upstream backlogit cascade defect (20FDC0A7 / F9767C12) is **not** fixed.
  Mitigation is procedural (P-015 prohibition + manual safe-close), not
  technical. Accepted: the fix is in an external repository outside this
  workspace's boundary.
- Group B (checkpoint continuation, incl. CDBE7B7A) remains unaddressed, so
  checkpoint-related operator pauses persist after this feature ships. Accepted
  and explicitly scoped out; tracked as the next staging candidate.
