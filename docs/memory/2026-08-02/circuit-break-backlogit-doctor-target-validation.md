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

Five parallel target checks returned the same scope error: path outside workspace storage root for queue/... or archive/... targets.

## Context

- Files involved: restored 104-S, 109-F, 109.001-R, 109.001-T, and 109.013-T backlog artifacts.
- Resolution: abandoned the optional target-doctor operation without retry. Exact indexed ID/status/dependency queries and index sync remain authoritative for this session.
- Suggested next step: use the supported doctor target path convention in a future tooling-only session; do not block Phase 5A recovery on this optional check.
