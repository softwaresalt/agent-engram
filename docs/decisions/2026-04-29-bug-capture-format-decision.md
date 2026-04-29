---
title: "Bug Capture Format Decision"
description: "Chosen format and storage location for structured agent-discovered bug capture"
decision_date: 2026-04-29
status: decided
decided_by: ship/031.002.001-T
source_task: 031.002.001-T
shipment: 008-S
---

## Problem

Agent-discovered defects scatter across PR comments, memory files, and session notes with
no structured ingestion surface. This prevents bugs from being tracked, reproduced, or
escalating into the compound learning loop that all future agents consult.

## Options Evaluated

| Option | Description | Pros | Cons |
|---|---|---|---|
| (a) `docs/bugs/{date}-{slug}.md` | Dedicated bugs directory | Clear separation | New tooling surface; learnings-researcher doesn't scan it |
| (b) Backlog `-B` artifact type | Bug as backlog work item | Tracked with status | No `-B` type in installed registry; registry change required |
| (c) `docs/compound/bugs/` with `type: bug` | Bug category in compound library | Learnings-researcher finds it automatically; feeds learn/evolve pipeline | Bug may be unresolved (mitigated with `status` field) |

## Decision

**Option (c): `docs/compound/bugs/{slug}-{YYYY-MM-DD}.md`**

Bugs are stored as compound-category entries under `docs/compound/bugs/`. They use standard
compound frontmatter extended with `type: bug`, `status: open|resolved`, and
`reproduction_steps`. This places discovered defects directly in the path the
learnings-researcher already queries, satisfying the CE learning-loop integration requirement
without introducing new tooling infrastructure.

## Frontmatter Schema

```yaml
---
title: "Short description of the bug"
description: "One-line summary for learnings-researcher indexing"
problem_type: "bug"
type: bug
category: bugs
component: "affected subsystem or file"
status: open          # open | resolved | wont-fix
severity: critical|high|medium|low
symptom: "What the agent observed"
root_cause: "Root cause if known, 'unknown' if not yet determined"
reproduction_steps: |
  1. Step one
  2. Step two
resolution: "How it was fixed, or empty if open"
resolution_type: "code_fix|config_change|workaround|deferred|wont-fix"
discovered_in: "task ID, PR number, or session reference"
references:
  - path/to/related/artifact
created_at: YYYY-MM-DD
---
```

## Learning Loop Integration

1. Bugs are written to `docs/compound/bugs/` immediately when discovered (any session phase).
2. The `observe` skill recognises `source: bug` and records the bug observation.
3. The `learn` skill clusters repeated bug patterns into instincts.
4. The `compound-refresh` skill reviews and updates stale bug entries post-shipment.
5. The learnings-researcher already scans all `docs/compound/` subdirectories — no config change needed.

## Rationale

Choosing option (c) over (a) because the compound library is the canonical knowledge store
already consulted at the start of every research phase. New bugs become visible to future
agents immediately without any registry or tooling change. The `status: open|resolved` field
addresses the concern that compound is normally for "solved" problems — open bugs are valid
institutional knowledge precisely because they warn future agents about known failure modes.

Rejecting option (b) because the installed backlogit registry does not expose a `-B` artifact
type and adding one is out of scope for this task.
