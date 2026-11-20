---
name: harvest
description: "Decomposes a reviewed implementation plan into backlog feature/task/subtask hierarchy"
argument-hint: "plan=docs/exec-plans/{YYYY-MM-DD}-{slug}-plan.md"
input:
  properties:
    plan:
      type: string
      description: "Path to the reviewed implementation plan"
    dry_run:
      type: boolean
      description: "When true, output the planned structure without creating entries"
  required:
    - plan
---

# Harvest Skill

The `harvest` skill turns a reviewed implementation plan into backlog
feature, task, and subtask items. It is the reusable decomposition step
invoked by the stage agent.

This skill does not perform planning or review. It assumes the incoming
plan has already been reviewed or otherwise approved for decomposition.

## Inputs

* `${input:plan}`: (Required) Path to the reviewed implementation plan.
* `${input:dry_run:false}`: (Optional, defaults to `false`) Preview the
  planned hierarchy without creating entries.

## Workflow

When the `backlogit` capability pack is installed, follow `.github/instructions/backlogit.instructions.md`: refresh the live workflow surface with `backlogit_get_metadata_catalog` and inspect type/template/WIT metadata (`backlogit_list_types`, `backlogit_list_templates`, `backlogit_get_wit_metadata`) before creating or mutating artifacts.

### Phase 1: Validate the reviewed plan

1. Read `${input:plan}` in full.
2. Confirm the file exists and represents an implementation plan.
3. Resolve the plan's frontmatter `review_manifest` path (workspace-relative; it must
   resolve inside the workspace root). Read that manifest through the review-manifest
   normalizer contract (`ReadReviewManifest` / `NormalizeReviewManifest`, reference
   implementation in `internal/ops`), which accepts attempt-roster, cycle-record,
   pre-review and live frontmatter manifests. Never read a `decision:` marker from the
   plan body: the manifest is the single review authority.
4. Evaluate the gate against the plan's current frontmatter `revision`
   (`EvaluateHarvestAdmission`) and report exactly one state:
   * `HARVEST_ADMITTED` — the normalized manifest reports `verdict: PASS` at
     `latest_attempt`, and that attempt reviewed the plan's current revision. Proceed.
   * `HARVEST_REFUSED` — no `review_manifest` reference, manifest absent, no latest
     attempt, a non-`PASS` verdict (including `ADVISORY` and `FAIL`), or a `PASS`
     against a superseded or unrecorded plan revision. Halt; the plan is not
     harvest-ready. Recommend `plan-review` (revise and re-review for `ADVISORY`).
   * `MANIFEST_QUARANTINED` — the manifest could not be normalized (empty, truncated,
     malformed, wrong types, unreadable, or outside the workspace). This is a fail
     state, never absence: halt and report the quarantine reason.
5. Interpret the latest attempt's `dispatch_mode` (roster entry, or the manifest's
   `dispatch_mode` extra). `multi-agent`, `single-agent-declared-degradation`,
   and `same-model-declared-degradation` are acceptable only when the review record
   states that every selected persona was covered. Missing, empty, or unavailable
   dispatch modes halt before backlog creation.
6. Confirm the plan has already cleared the review gate according to the admission
   state above.
7. Halt if the plan is missing, unreadable, is not `HARVEST_ADMITTED`, has an
   unacceptable dispatch mode, or is clearly not ready for backlog creation.
   Recommend running `plan-review` first when the review state is unclear.

### Phase 2: Parse the plan structure

Extract the planning data needed for decomposition:

1. Root feature title from frontmatter and top-level headings.
2. Problem frame, requirements trace, decisions, and standards check
   for the root feature description.
3. Task candidates from each implementation unit.
4. Subtask candidates from file lists, acceptance criteria, verification
   steps, and test surfaces inside each implementation unit.
5. Dependency edges from the plan's dependency graph.

Use repository search tools to validate file references or symbols when
the plan mentions existing code locations that need confirmation.

### Phase 3: Build the hierarchy model

Map the plan into the backlog hierarchy:

* one feature representing the whole reviewed plan
* one task per implementation unit
* one or more subtasks per file group, verification slice, or explicit
  execution step inside the unit

Before creating anything, apply backlog shaping rules:

1. Keep tasks small enough to fit a focused implementation session
   (the 2-hour rule).
2. Keep each task within a single skill domain.
3. Require a verifiable exit state for every task and subtask.
4. Preserve plan references so downstream execution can trace work
   back to the plan.

**Size + complexity are mandatory, non-conflated, two-axis metadata
(NON-NEGOTIABLE):** Every emitted task-kind work item MUST be assigned both a
`size` value (effort/volume: `XS`, `S`, `M`, `L`, `XL`) and a `complexity`
value (difficulty/uncertainty: `trivial`, `low`, `medium`, `high`). These are
two independent axes, never a single combined scalar. This section is
self-contained — the rules below (structured-emission capability gate, enum
validation gate, two-axis granularity gate, provenance-completeness rule) are
the complete normative contract for this skill and do not depend on any file
outside this template. When the gsd repository's own
`docs/size-complexity-reference.md` is present in the current workspace (as it
is in the gsd dogfood repository itself), treat it as supplementary
rationale and worked examples only — it is not copied into every installed
target workspace, so it must never be a required read for following the
rules here.

**Structured-emission capability gate:** Whether `size`/`complexity` are
written as structured backlog fields, and in how many calls, depends on the
active backlog registry — check `features.sizing`, and check exactly which
operation (`create_task` vs. `update_task`) declares the `size`/`complexity`/
`size_source`/`size_ruleset_version` params before assuming support or
call-sequencing:

* When the registry advertises `features.sizing: true` (for example,
  `backlogit`), write `size` and `complexity` as structured fields on every
  task-kind work item, validated per the enum gate below — but do not assume
  they can be set in the same call that creates the task, or even in the
  same call as each other. `backlogit` 1.8.0's `create_task` operation
  (`backlogit_create_item` / `backlogit add`) accepts no sizing params at
  all, and its `update_task` operation enforces two mutually exclusive,
  body-preserving mutation seams: `size` (with `size_source`/
  `size_ruleset_version` together) is one seam, `complexity` is a separate
  seam, and neither can be combined with the other or with any other field
  update in one call. The required sequence for a registry with this shape
  is therefore: (1) create the task with no sizing params, (2) a follow-up
  update call setting `size` (+ `size_source`/`size_ruleset_version` when the
  registry defines those provenance params, set together as one call), (3) a
  further, separate update call setting `complexity`. Always inspect the
  registry's actual `params` declarations per operation before assuming a
  different (or more permissive) sequence is safe.
* When the registry does not advertise `features.sizing` (absent or `false` —
  for example, `backlog-md`, whose `create_task`/`update_task` operations
  expose no structured size/complexity params), the backend has no
  structured slot for these values. Preserve both values anyway by embedding
  them as clearly labeled prose in the task description (for example,
  `Size: M | Complexity: medium`), still enum-validated, and flag this
  degradation explicitly in the harvest report so operators know structured
  provenance is unavailable for this backend. Do not skip assigning size and
  complexity, and do not halt task creation, merely because the backend
  lacks structured fields.

**Enum validation gate (fail-closed):** Validate `size` against
`XS|S|M|L|XL` and `complexity` against `trivial|low|medium|high` before
emitting or writing any work item, whether as structured fields or as
prose-embedded values per the capability gate above. Reject and halt on any
other value rather than silently coercing or defaulting it. This validation
is an extension of the existing P-003 granularity gate below, not a separate
check — a task that fails size/complexity validation is exactly as unready
for staging as one missing an acceptance criterion.

**Sizing gate arming and fail-closed contract (SHIP-8; single
`size_ruleset_version` identifier `ah-stage-sizing-v1`):** this contract turns
the size/complexity rules above from advisory prose into an enforced
decomposition gate. It rejects the one malformed task and reports it: it is a
decomposition halt, never a session halt, and other independent items continue.
It applies to newly harvested tasks only (enforce-on-new-only: no retroactive
sweep of existing backlog items) and it never auto-splits a task; it halts and
reports, and the operator or Stage decomposes.

1. **Arming signal (sole authority)**: the advertised registry flag
   `features.sizing` is the sole authoritative signal for whether the
   fail-closed branch is armed. The mere presence of sizing parameters on an
   operation (for example `size`, `size_source`, `size_ruleset_version`, or
   `complexity` on an update operation) never arms the gate on its own and
   must not be used as an alternative or fallback trigger: the registry is the
   declared contract, whereas operation parameters vary by tool build.
2. **Armed (`features.sizing: true`) — fail closed**: for every task, validate
   both `size` and `complexity` against their enums before the create call, so
   a rejection happens before any write. The gate rejects rather than coerces
   or defaults: an out-of-enum, missing, or conflated value halts that task.
   The split triggers are checked conditions, not advice: a `size` implying
   more than 2 hours of human-equivalent effort forces a split regardless of
   `complexity`, and `complexity: high` forces a split or an explicit
   de-risking step regardless of `size`; a task tripping the size trigger
   always halts, and a `complexity: high` task halts unless the explicit
   de-risking step is recorded in its description.
3. **Write sequence (for a registry with the backlogit seam shape, exactly
   three calls; the registry's declared `params` stay authoritative)**: (1) create the task with no
   sizing params; (2) one update call setting `size` together with
   `size_source: agent` and a non-empty `size_ruleset_version`; (3) a separate
   update call setting `complexity`. These seams are mutually exclusive and
   are never combined. If a write fails partway, the halt message must name
   the item ID and the exact remaining call so the state is recoverable and
   can be completed idempotently.
4. **Not armed (`features.sizing` absent or `false`) — degrade loudly**:
   preserve both enum-validated values as clearly labeled prose in the task
   description (still enum-validated before any write) and report the
   degradation explicitly in the harvest report so operators know structured
   provenance is unavailable. The degradation is never silently skipped, and
   it applies even if the operation surface accepts sizing params, because
   only the registry flag arms the gate (item 1). Degradation does not halt task creation.

**Two-axis granularity gate (extends the 2-hour rule, P-003):** The 2-hour
rule from shaping rule 1 above is evaluated on BOTH axes independently, never
on size alone, regardless of backend:

* A `size` estimate implying more than 2 hours of human-equivalent effort
  requires splitting the task, regardless of its `complexity`.
* A `complexity: high` task requires de-risking (via further decomposition,
  a spike, or additional deliberation) or an explicit split before being
  emitted as a single executable task — even when its `size` is small. High
  complexity is never a reason to skip the granularity check just because
  the size looks small.

Do not conflate the two axes: never derive `complexity` from `size` (or vice
versa), and never emit a single blended difficulty/effort scalar in place of
the two distinct fields.

**Parent-first ordering (NON-NEGOTIABLE):** Tasks require a `parent_id`
referencing an existing feature. The root feature MUST be created before any
tasks are created. If the harvest context does not include an existing parent
feature for task-kind work items, create or identify one before proceeding.
Omitting `parent_id` for task-kind artifacts violates the required
parent-child hierarchy (P-003 decomposition integrity) and may be blocked by
the configured backlog registry or policy gates.

When the `backlogit` capability pack is installed and the registry advertises
`features.shipments: true`, the parent feature MUST be added to the shipment
before its child tasks during shipment assembly. This ordering is enforced
downstream by the Ship agent — flag it explicitly in the harvest report so
Ship can assemble the shipment correctly.

### Phase 4: Execute or preview

If `${input:dry_run}` is `true`:

1. Produce the proposed feature, task, subtask, and dependency structure,
   including the proposed `size` and `complexity` value for every task.
2. Do not call backlog mutation tools.

If `${input:dry_run}` is `false`:

1. Query the backlog first to avoid duplicate root features.
   Use `backlogit_list_items` or `backlogit_search_items` to check for existing
   items with matching titles.
2. Create the root feature via `backlogit_create_item` or
   `backlogit add --type {artifact_type} --title {title}`.
3. Create one task per implementation unit under that feature via
   `backlogit_create_item`/`backlogit add --type {artifact_type} --title {title}` (`create_task`). Then set `size`
   and `complexity` per the structured-emission capability gate above: when
   `features.sizing: true`, apply them as one or more follow-up
   `backlogit_update_item`/`backlogit update {{id}}` (`update_task`) calls in whatever
   number and order the registry's declared `params` require (for example,
   `backlogit` requires a separate call for `size`/`size_source`/
   `size_ruleset_version` and another separate call for `complexity`); when
   `features.sizing` is absent or `false`, embed both values as prose in the
   description at creation time instead. Validate both values per the enum
   validation gate above before any write, structured or prose.
   **Harness-surface declaration mandatory at task creation (P-004 / T1 resolver
   contract)**: supply exactly one `harness-surface:*` label on the `create_task`
   call's labels parameter (or in a dedicated labels-only `update_task` call during
   repair), never inside the separate `size` or `complexity` `update_task` calls.
   See the Guardrails section for the assignment rule.
4. Create granular subtasks under each task.
5. Wire dependencies when the backlog tool supports dependency operations
   (check the backlog registry for availability).

### Phase 5: Verify and report

1. Confirm the created hierarchy through backlog read operations.
2. Report the created IDs, counts, and dependency summary.
3. Return the ready backlog as the output of the planning pipeline.
4. Recommend handing the resulting backlog to the build-and-ship
   workflow for harness, build, review, CI, and pull request execution.

## Guardrails

* Do not modify the plan file.
* Do not skip duplicate checks.
* Do not create shipment artifacts from this skill. Shipment assembly
  happens downstream in Stage (Step 5.5) or Ship (fallback path).
* Keep descriptions self-contained enough for the next executor to act
  without reopening the plan for basic context.
* Never emit a task without both `size` and `complexity` set and enum-validated.
* Never combine `size` and `complexity` into one field or derive one axis from
  the other. This rule is fully stated above and does not require any external
  file to be present in the installed workspace.
* **Harness-surface declaration (P-004 / T1 resolver contract)**: every task (`-T`)
  MUST receive exactly one `harness-surface:*` label chosen from its file budget.
  Declare `harness-surface:harness-architect` when any path in the file budget adds
  or modifies production code (for example `internal/`, `cmd/`, `src/`, or the
  target workspace's production source modules). Mixed-budget precedence: if any
  path touches production code, the task MUST declare
  `harness-surface:harness-architect` even when paired with test, fixture, or
  documentation files. Declare `harness-surface:none` only when the file budget
  touches nothing but templates, installed mirrors, documentation, registries or
  manifests, `.gitattributes`, CI workflows, spike findings, or test and fixture
  files. Feature items (`-F`) MUST NEVER carry any `harness-surface:*` label
  (`FEATURE_DECLARES_SURFACE` is a fail-closed error in `gsd harness resolve`).
  The resolver never defaults a missing declaration to `harness-surface:none`:
  omitting the label makes Ship's T1 gate halt with `UNRESOLVED` /
  `DECLARATION_MISSING`. Apply the label through the labels parameter only, never
  combined with the mutually exclusive `size` or `complexity` update calls.
* **Docs-route candidates (P-002 docs-only route)**: when a candidate task's file
  budget qualifies as docs-only (explicit owned paths, prose `.md` under the docs
  root only, no deny-list path), declare `harness-surface:none` on it and say in
  the task body that it is a docs-route candidate. Never apply either route label
  (`harness-ready` or `harness-verification-gated`): only harness-architect applies
  a route label.

## Model Routing

This skill operates at **Tier 2 (Standard)** — backlog decomposition is structured and deterministic.
