# Deliberation — Shipment Safe-Close Contract Reconciliation

- Date: 2026-09-17
- Agent: Stage
- Session: stage-workflow-closure-defects-2026-09-17
- Status: accepted (operator-directed selection)
- Covering feature (proposed): Shipment Safe-Close Contract Reconciliation
- Policy basis: P-021 C5/C6 (deferred-scope-expansion intake), P-015, P-003, P-005

## 1. Intake and provenance

Triaged the active stash (130 entries) for recurring Stage/Ship workflow and
post-merge closure defects that break autonomous continuation.

### Entries included in this deliberation

| Stash ID | Kind | Pri | Age | Marker | Finding |
|---|---|---|---|---|---|
| B9CC92AC | chore | medium | 12d | DEFERRED SCOPE EXPANSION | `_ship.agent.md` Step 6 1.b invokes `shipment-reconcile` `mode: safe-close`, which the installed skill does not implement |
| 77A4E71C | task | medium | 11d | DEFERRED SCOPE EXPANSION | 137-S eligibility vs 135-S `archived_status: done` (PR #384) |
| F35EA0E6 | bug | high | 8d | DEFERRED SCOPE EXPANSION | Same question recurring: 142-S vs 137-S (PR #389) |
| 76153F55 | bug | high | 4d | DEFERRED SCOPE EXPANSION | Same question recurring: 140-S vs 139-S (PR #394) |
| F9767C12 | bug | high | 14d | FOLLOW-UP | Cascade is independent of explicit manifest membership; supersedes the 28C0E138/F9D1C495 remediation |
| 28C0E138 | bug | high | 14d | DEFERRED SCOPE EXPANSION | 133-S manifest includes parent 142-F; cascade risk |
| F9D1C495 | task | high | 14d | FOLLOW-UP | No safe shipment archival path via backlogit 1.10.1 CLI |
| B2E3C372 | bug | high | 11d | DEFERRED SCOPE EXPANSION | Compound doc frames P-015 as a preference; P-015 actually forbids the cascade |
| 982B0B01 | bug | high | 14d | DEFERRED SCOPE EXPANSION | 137-S missing 134-S blocking predecessor |
| 284285B5 | bug | high | 14d | DEFERRED SCOPE EXPANSION | 141-S missing 139-S blocking predecessor |

All ten carry a deferred-scope-expansion or follow-up marker and
`Requires deliberation: true`. This artifact is the P-021 C6 deliberation
gate for all ten; none proceeded to planning without it.

### P-021 C5(A) — duplicate detection (unconditional)

Ran over every entry above regardless of source-ref population.

**Result: CLEAN DUPLICATE SCAN — no duplicates found; no archival performed.**

- `77A4E71C` / `F35EA0E6` / `76153F55` are **recurrences, not duplicates**. Each
  carries distinct source refs (PR #384 / #389 / #394; shipments 135-S / 137-S /
  139-S; threads `PRRT_kwDORJEduc6fuNrf` / `PRRT_kwDORJEduc6gipAc` /
  `PRRT_kwDORJEduc6h1Wrd`) and each explicitly cross-references its predecessors.
  They are three independent observations of one unresolved question and are
  retained separately as recurrence evidence.
- `28C0E138` / `F9D1C495` / `F9767C12` overlap on remediation but are distinct
  findings (cascade risk / no archival path / correction of the proposed fix).
  `F9767C12` **supersedes the recommendation** in the first two without
  duplicating their findings.
- `982B0B01` / `284285B5` describe different shipments and different edges.

No `DISCOVERY-STATUS: AMBIGUOUS` or `LOOKUP-UNAVAILABLE` tokens present.

### P-021 C5(B) — late-identifier reconciliation

Triggered on entries carrying `N/A` source-ref fields.

**Result: no-op. No late identifier found; all recorded `N/A` values stand as
truthful terminal records.**

- `B2E3C372`, `F35EA0E6`, `76153F55`, `B9CC92AC`: `task=N/A` because each is a
  documentation/contract defect with no owning implementation task. PR number and
  review-thread ID are populated in all four. Nothing to recover.
- `F9D1C495`, `F9767C12`: post-merge closure findings sourced from
  `docs/closure/133-S-2026-09-03-post-merge-closure.md`; PR #377/#378 threads
  recorded. Nothing to recover.
- Searched Ship-owned residual-risk records (`docs/closure/`, archive AUDIT
  RATIONALE blocks) keyed on each deferred entry ID. No newly available
  review-thread ID or PR number surfaced for any `N/A` field.

Reconciliation being a no-op is **not** a C3 or C6 shortfall and did not gate
deliberation.

## 2. Problem statement

**The documented shipment safe-close procedure is not executable as written, and
its actual terminal state contradicts the dependency-eligibility rule that
consumes it.**

Three independently verified contradictions:

1. **Dangling skill mode.** `.github/agents/_ship.agent.md` Step 6 item 1.b
   designates `shipment-reconcile` `mode: safe-close` as *authoritative* for
   closure. `.github/skills/shipment-reconcile/SKILL.md` implements only
   `mode: pre` and `mode: post` and contains **zero** occurrences of the string
   `safe-close` (verified 2026-09-17). Ship's canonical closure path is a
   dangling reference.

2. **Impossible CLI transition.** The same Step 6 prose specifies
   `backlogit move <shipment_id> --status shipped` -> verify `status: shipped` ->
   `backlogit archive` -> verify `archived_status: shipped`. The backlogit CLI
   rejects the move step unconditionally (`exit 9`,
   `ErrShipmentShippedRequiresEnvelope`, "shipment must be shipped via
   ShipShipment, not a direct status update"), with no `--force` bypass. This was
   re-confirmed empirically during 133-S, 135-S, 137-S and 139-S closures.

3. **Eligibility predicate mismatch.** `.github/instructions/backlogit.instructions.md`
   (lines ~63-65) states a queued shipment "is only ELIGIBLE for claim once every
   `blocks`-type predecessor it depends on has reached `shipped`". Because of (2),
   the only achievable terminal state for a partial-feature shipment is
   `archived_status: done`. Taken literally, every remaining queued shipment is
   permanently ineligible.

The escape hatch that makes (2) unavoidable is real and not a workaround choice:
`backlogit shipment ship` is **P-015-forbidden** for these shipments.
`F9767C12` establishes by direct source review of backlogit's
`internal/core/shipment_lifecycle.go` that `featureScopeRoots` discovers a
covering feature by walking *up* `parent_id` from **every** manifest item
regardless of explicit membership, and the ship call site then unconditionally
invokes `returnUnreleasedFeatureItems`, which force-requeues and **clears
`parent_id`** on every descendant outside the release scope. Removing `142-F`
from a manifest does **not** prevent this. `20FDC0A7` additionally records that
the same op hangs non-terminating on this feature's roster.

### Why this is the highest-leverage workflow defect

- It has produced an operator pause on **every** closure in the chain:
  133-S, 134-S, 135-S, 136-S, 137-S, 138-S, 139-S.
- The eligibility question alone has recurred **three times verbatim**
  (77A4E71C -> F35EA0E6 -> 76153F55), each time consuming a full review cycle.
- Two further recurrences are structurally guaranteed on the remaining queued
  links (140-S/141-S, 141-S/142-S) if left unresolved.
- It currently sits directly in front of the live queued chain
  140-S -> 141-S -> 142-S.

## 3. Grouping analysis

Groupings considered over the workflow/closure candidate set.

### Group A — Shipment safe-close contract reconciliation (SELECTED)

Entries: B9CC92AC, 77A4E71C, F35EA0E6, 76153F55, F9767C12, 28C0E138,
F9D1C495, B2E3C372, 982B0B01, 284285B5.

Shared artifact surface — all ten defects are in the same four documents
describing the same procedure:

- `.github/skills/shipment-reconcile/SKILL.md`
- `.github/agents/_ship.agent.md` (Step 6 closure)
- `.github/instructions/backlogit.instructions.md` (eligibility rule)
- `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`

Coherence: single root cause (contract/tool divergence at shipment closure),
single blast radius (harness documents only), ships as one PR.
Risk: **low** — no runtime, source, or product code touched.

### Group B — Checkpoint lifecycle and continuation (DEFERRED)

Entries: 4EF24729 (checkpoint resolved only on a merged `post-merge/*` branch
never reaches `main`), AA5698E3 (stale checkpoints + 8 legacy-schema anomalies),
plus the newly captured dark-mode continuation defect (see §6).

Deferred: different contract surface (checkpoint lifecycle and the
Crash-Resumption Protocol, not shipment closure). Real and operator-relevant,
but combining it would double the blast radius without shared root cause.

### Group C — Archive-record and upstream-tool hygiene (DEFERRED)

Entries: B761AFA7 (ten 133-S task records lack `archived_status`/`archived_from`
wrappers), 20FDC0A7 (backlogit `shipment ship` hang).

Deferred: B761AFA7 is data normalization, separable and low priority. 20FDC0A7
is a defect in the **external** backlogit repository and is not actionable in
this workspace; it is retained as the standing justification for the manual
safe-close path and referenced by Group A rather than fixed by it.

### Explicitly excluded

- `83993031` (unscoped `Stop-Process` during runtime verification) — verification
  safety, a different root cause and artifact surface, per operator lane guidance.
- `39049DEE`, `74AAE80F`, `6C9AA7D3` — build/CI lane.
- `9108DB24`, `A67EEA54`, `AF5CE07E`, `265F99BE`, `F0A2A478` — runtime/product lane.
- `90316096` — MCP protocol modernization lane.
- `3A9CBD36` — 135-S documentation sweep; different surface (product docs).

## 4. Options

### Option 1 — Align prose to tool reality (documentation-only)

Implement a real `mode: safe-close`, rewrite Step 6 1.b to the achievable
sequence, and redefine the eligibility predicate to accept the achievable
terminal state.

- Pro: unblocks the live chain immediately; zero external dependency; low risk;
  resolves all three recurrences workspace-wide in one place.
- Con: enshrines a divergence from backlogit's nominal `shipped` lifecycle.

### Option 2 — Fix the upstream tool first

Change `returnUnreleasedFeatureItems` to skip non-explicit-member features and
permit a manifest-only `move --status shipped`.

- Pro: removes the root divergence properly.
- Con: external repository; unbounded latency; **outside Stage's and this
  workspace's boundary**; does not unblock the live chain; leaves the dangling
  `mode: safe-close` reference broken regardless.

### Option 3 — Hybrid: align prose now, track the upstream defect (RECOMMENDED)

Option 1 as the executable contract, plus a durable upstream-defect record so
the tool fix stays tracked without gating this workspace.

- Pro: smallest change that makes the contract executable; resolves the
  recurrence once; preserves the correct long-term fix as tracked work.
- Con: two artifacts to keep in sync until upstream lands.

## 5. Decision

**Option 3 adopted.**

Rationale: (2) and (3) in §2 are *contract* defects, not tool defects — the
prose describes a sequence that has never once succeeded in this workspace,
while the sequence that *has* succeeded seven times is undocumented. Correcting
the contract to describe the procedure that actually works is the minimal,
lowest-risk intervention, and it is the only one available within this
workspace's boundary. The upstream cascade defect is real but is already
mitigated by P-015's prohibition; it is tracked, not gated on.

Normative decisions carried into planning:

- **D1** — The canonical safe-close terminal state for a partial-feature
  shipment is `status: done` -> `archived_status: done`. The `shipped` literal is
  reachable **only** via `ShipShipment`, which P-015 forbids for this shape.
- **D2** — The dependency-eligibility predicate is redefined as: a `blocks`
  predecessor is satisfied when the predecessor shipment record is **archived**
  with `archived_status` in {`done`, `shipped`}. This resolves 77A4E71C,
  F35EA0E6 and 76153F55 as one workspace-wide ruling.
- **D3** — `mode: safe-close` becomes a real, specified mode of
  `shipment-reconcile`, carrying the P-015 protected-set verify-after-each
  invariant. `_ship.agent.md` stays a thin pointer.
- **D4** — Manifest editing is **not** a cascade remedy (per F9767C12).
  28C0E138 and F9D1C495's recommendation is recorded as superseded so it is not
  re-followed.
- **D5** — P-015 is stated as a **prohibition** in all guidance, matching the
  policy text ("MUST NOT call the cascade"). The three logged cascade
  invocations against 135-S get an explicit recorded disposition.
- **D6** — Shipment-level `blocks` edges MUST be derived from member-task
  dependencies at assembly time; missing edges are backfilled and the derivation
  rule is added to Stage's shipment-assembly step.

### Covering feature

`Shipment Safe-Close Contract Reconciliation` — a **new** covering feature.
Explicitly **not** attached to `142-F`, which is an active product feature
(indexer/read-only daemon split) and unrelated to harness workflow contracts.

### Constraints honored

- No mutation of queued shipments 140-S / 141-S / 142-S in this session
  (operator directive). Edge backfill is planned as a task for Ship, not
  executed by Stage.
- Stage authored artifacts and backlog records only; no source, template, or
  config implementation, no build, no shipment claim, no PR lifecycle.

## 6. Newly captured entry

The operator-reported dark-mode/checkpoint-continuation defect ("autonomous
bounded continuation should not require manually repeating an unambiguous active
checkpoint filename") had **no existing active stash entry**. Verified by keyword
scan across all 130 active entries (`auto-pick`, `single candidate`,
`operator selection`, `unambiguous`, `Crash-Resum`, `bounded`, `filename`). The
nearest neighbours — AA5698E3 and 4EF24729 — cover stale checkpoints and
checkpoint-resolution persistence respectively, not selection autonomy.

Captured as a new stash entry under Stage's intake authority and routed to
**Group B (deferred)**, preserving fail-closed behavior for multiple, malformed,
cross-scope, or non-dark candidates.

## 7. Evidence log

- `backlogit stash list` — 130 active entries, 73 carrying the deferred marker.
- `backlogit checkpoint list` — 25 total, 0 active, 0 quarantined, 0 anomalies
  (ZERO-CANDIDATE NORMAL STARTUP; no recovery performed).
- `Select-String .github/skills/shipment-reconcile/SKILL.md 'safe-close'` — 0 matches.
- Skill modes present: `mode: pre`, `mode: post` only.
- `backlogit dep list 141-S` -> `141-S -> 134-S (blocks)`, `141-S -> 138-S (blocks)`.
  **No `139-S` edge** — 284285B5 confirmed live.
- `backlogit dep list 140-S` -> `138-S`, `139-S`. `142-S` -> `137-S`, `140-S`, `141-S`.
- `backlogit shipment list` — 16 shipments; statuses `done` (8), `abandoned` (5),
  `queued` (3). **No shipment carries a `shipped` status**, corroborating D1/D2.
- `.github/policies/workflow-policies.md` P-015 — "It MUST NOT call the cascade
  `backlogit_ship_shipment` for closure", confirming B2E3C372.

## 8. Degraded-path record

- **ENGRAM: degraded.** The engram CLI daemon failed workspace-status readiness
  twice, so indexed symbol/module search was unavailable for this session.
  Fallback used: backlogit structured index and query surfaces
  (`stash list`, `shipment list`, `dep list`, `get`) as the primary evidence
  source, with scoped `Select-String` against four named, pre-identified contract
  files only. No broad unscoped grep was substituted for indexed search.
- **backlogit registry: `features.sizing` absent.** Structured `size`/`complexity`
  fields are unavailable. Both axes are enum-validated and preserved as labeled
  prose in each task description per the harness degradation rule.
