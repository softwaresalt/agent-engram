---
date: 2026-07-04
agent: Stage
mode: stash-triage + grounding-spike + impl-plan + plan-review + harvest + shipment-assembly
new_feature: 071-F
new_shipment: 071-S
review_gate: 071.001-R
source_stash: FC881353
spike_doc: docs/decisions/2026-07-04-ci-build-skip-required-check-spike.md
plan_doc: docs/exec-plans/2026-07-04-ci-build-skip-non-code-prs-plan.md
scope: .github/workflows/ci.yml
status: reviewed-backlog-ready (queued for Ship to claim)
---

# Stage session memory — 2026-07-04 — 071-S CI build-skip on doc/backlog-only PRs

## Task

Operator-directed Stage run against `agent-engram` (main @ `a3c2c81`, cache clean;
pre-existing ` M .gitignore` drift NOT ours — left untouched). Triage + plan stash
`FC881353` ("Reduce CI build actions to run only on code changes") and harvest into
a **queued shipment** with a feature + task, via spike → impl-plan → plan-review →
harvest, then commit+push to a new branch (**NO PR** — Orchestrator lands it).
Stage produced reviewed structure only — no Ship/code/PR work (the actual `ci.yml`
edit is Ship's job).

## Tool status

- MCP `backlogit` server **transport closed** all session → ran in
  `TOOL_DEGRADED` mode on the registry-declared CLI fallback `C:\Tools\backlogit.exe`
  (v1.3.0). Registry present at `.autoharness/backlog-registry.yaml`.
- `INDEX_SYNC_SKIPPED` per operator override (cache clean; do not run reflexive
  `backlogit sync`). All mutations via atomic CLI + authoritative markdown edits.

## The crux — required-check determination (resolved FIRST)

**`build` is NOT a required status check on `main`.** Evidence:

- `gh api .../branches/main/protection` → 404 (no classic protection).
- One ruleset `PR-Required` (id **12812291**, active). `rules[].type` =
  `deletion, non_fast_forward, pull_request, copilot_code_review, update` —
  **no `required_status_checks`**. `pull_request` = 1 approval + code-owner +
  last-push-approval + thread-resolution; `copilot_code_review` on push.
- **No CODEOWNERS file** → code-owner review can't be satisfied conventionally →
  every merge is `--admin` (bypassing the **review** gate, not a status check).
- Corroboration: PR **#200** (`chore(backlog): archive shipment 070-S`, backlog-only)
  ran a full `build` **3m14s** (20:25:44→20:28:58Z, SUCCESS) yet merged
  `REVIEW_REQUIRED`. The build CheckRun is present but unenforced.

→ The stash's "required-check pending-forever" GOTCHA **does not apply here**. The
observed `BLOCKED`/`REVIEW_REQUIRED` is review-driven. A plain `paths-ignore` is the
minimal safe mechanism. Grounding was conclusive from the API + a live PR, so this
was a **grounding spike** (evidence + decision), not a code experiment.

## Recommended mechanism (in the plan, for Ship)

`paths-ignore` on **both** `on.push` and `on.pull_request` in
`.github/workflows/ci.yml`, patterns: `.backlogit/**`, `docs/**`, `**/*.md`,
`.autoharness/**`. Deliberately NOT ignored (re-arm CI): `**/*.rs`, `Cargo.toml`,
`Cargo.lock`, `*.toml`, `.github/workflows/**`, `scripts/**`, `examples/**`,
`src/**`, `crates/**`. `paths-ignore` all-match semantics guarantee a single code
file re-arms the full fmt→clippy→test→audit, so code-PR coverage cannot be
weakened. **Rejected:** PR-title `if:` guard (fragile/bypassable; no safer on
required checks). **Future-coupling guardrail** (in-file comment + plan snippet):
if `build` is ever promoted to a required status check, switch to a companion
always-passing job with the same check name — NOT implemented now. `release.yml`
(tag-triggered) out of scope.

## What Stage produced (all committed, none executed)

- **Spike/decision** `docs/decisions/2026-07-04-ci-build-skip-required-check-spike.md`
  (type spike, decided) — the required-check investigation + finding + guardrail.
- **Plan** `docs/exec-plans/2026-07-04-ci-build-skip-non-code-prs-plan.md`
  (type plan, reviewed) — grounded ci.yml state, chosen mechanism, path-set
  rationale, rejected alternative, future contingency snippet, Step 5.5 scope
  guard, verification plan, blast radius.
- **Feature 071-F** (`.backlogit/queue/071-F.md`, queued) — harvested from stash
  `FC881353`; goals/dod populated; references plan + spike.
- **Task 071.001-T** (queued) — single-width CI/workflow change: add `paths-ignore`
  to `ci.yml` push + pull_request + coupling comment + verify. acceptance-criteria
  + implementation-notes populated. ~<2h. No dependency edges.
- **Review 071.001-R** (`.backlogit/archive/...`, accepted) — 9 findings, all
  resolved/accepted, zero P0/P1. Disposition **ACCEPTED for harvest**. No
  plan-harden (LOW blast radius, single-family).
- **Shipment 071-S** (`.backlogit/queue/071-S.md`, **queued**) — manifest
  [071-F, 071.001-T]; custom_fields `review_artifact: 071.001-R`, `source_plan`,
  `source_spike`, `source_stash_id: FC881353`; description, manifest, dependency
  order, Step 5.5 guard, follow-on, Ship notes populated. Awaiting Ship to claim.

## Validation

- All 6 artifact frontmatters parsed with `yaml.safe_load` → **ALL_OK**.
- `backlogit doctor` → **43 pre-existing `archived_from_self_ref`** findings (known
  baseline), **0 orphans, 0 duplicate IDs, 0 new findings** referencing any 071
  item. Clean.

## Landmines respected

- **NEVER ran `backlogit sync`** — CLI mutations (`stash harvest`, `add`,
  `shipment create`) + authoritative markdown edits.
- Did **not** stage `.gitignore` (pre-existing operator drift). Harvest moved
  `FC881353` from active `stash.jsonl` (reverted to committed) into
  `.backlogit/archive/stash.jsonl` (harvest record, `harvested_artifact_id: 071-F`).
- **No Ship work:** did **not** edit `.github/workflows/ci.yml`, open a PR, or run
  any build/test — read-only for grounding.

## Handoff

Branch `071S-ci-build-skip` off main `a3c2c81`; artifacts committed (chore backlog
+ docs spike/plan/memory); pushed; **no PR** (Orchestrator lands it). Ship claims
071-S and edits `ci.yml` per 071.001-T (paths-ignore + coupling comment), then runs
the 5-step verification.

## Open questions for the operator

1. **Ruleset coupling (informational):** the design is safe *because* `build` is
   not a required status check. If you intend to make CI a required check on `main`
   later, flag it — `paths-ignore` would then need the companion-job pattern
   (documented) to avoid hanging doc-only PRs.
2. **`.autoharness/**` in the ignore set:** included as non-code harness/staging
   state. If any `.autoharness/**` change should ever gate CI, drop it from the set.
