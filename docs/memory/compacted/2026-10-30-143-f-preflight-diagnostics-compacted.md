---
title: Superseded preflight timeout diagnostics
date: 2026-10-30
feature: 143-F
status: superseded
---

## Final decision

The operator required full index maintenance to run separately from session
startup. Both launchers now omit preflight. See the
[verified final outcome](../2026-10-30/copilot-startup-index-separation-memory.md).

## Preserved diagnostic evidence

* Increasing startup deadlines did not resolve fresh-generation rebuilding:
  three relay deadlines occurred at 299107, 1799136, and 1799091 ms.
* The old forced-build circuit remains open. Its
  [failure chain](../2026-10-29/circuit-break-copilot-preflight.md)
  is preserved separately and was not retried.
* A direct sync against a partially completed candidate succeeded in
  634059 ms, parsing 113 files and hash-skipping 334. That was not evidence
  that fresh-generation preflight succeeded.
* A temporary diagnostic exposed SQLite code 5 contention. Read-only
  progress probes overlapped writers; they were discontinued.
* The Cozo optimization, longer launcher budgets, and executable override
  were removed. The temporary example was removed.
* Cargo discovery required ignoring inaccessible generated probe/cache
  directories. Package exclusions did not fix that discovery failure.
* Eight existing dependency advisories were captured as stash 25A1BE9E.

## Provenance

The full historical diagnostic record is
[archived without dropping evidence](../../archive/memory/2026-10-29-copilot-preflight-timeout-memory.md).
It describes obsolete launcher behavior, not the current implementation.
The latest task memory and unresolved circuit-breaker checkpoint remain in
their original locations. No unrelated memory, plan, or closure was changed.
