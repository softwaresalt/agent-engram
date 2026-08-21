---
type: circuit-breaker
timestamp: 2026-08-02T14:32:59.542-07:00
agent: .Stage
skill: direct
breaker_type: universal
operation: backlogit doctor target validation
attempts: 5
---

# Backlogit doctor target validation circuit break

## Failure Chain

Five checks were launched in one parallel batch before any result returned.

### Attempt 1

Target: `queue/104-S.md`

Result: scope error — path outside workspace storage root.

### Attempt 2

Target: `queue/109-F.md`

Result: scope error — path outside workspace storage root.

### Attempt 3

Target: `archive/109.001-R-plan-review-post-105-pending-sync-generation-and-startup-han.md`

Result: scope error — path outside workspace storage root.

### Attempt 4

Target: `queue/109.001-T.md`

Result: scope error — path outside workspace storage root.

### Attempt 5

Target: `queue/109.013-T.md`

Result: scope error — path outside workspace storage root.

The universal breaker threshold is three attempts; checks four and five were already in flight when the first three failures became observable. The optional operation was stopped after that batch and was not retried.

## Context

- Files involved: restored 104-S, 109-F, 109.001-R, 109.001-T, and 109.013-T backlog artifacts.
- Resolution: abandoned the optional target-doctor operation without retry. Exact indexed ID/status/dependency queries and index sync remain authoritative for this session.
- Suggested next step: use the supported doctor target path convention in a future tooling-only session; do not block Phase 5A recovery on this optional check.
