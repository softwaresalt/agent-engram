---
title: "Engram large workspace scale spike memory"
date: 2026-08-26
type: session-memory
---

## Outcome

Investigated whether the current Engram implementation can reliably index and
query more than 5,000 files across up to 10 repositories.

The recommendation is to use one workspace daemon per Git repository and not
claim a reliable 5,000-file or federated multi-repo operating envelope yet.

## Files Modified

* `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md`
* `docs/memory/2026-08-26/engram-large-workspace-scale-spike-memory.md`

## Decisions

* Same-root sessions should share one daemon through the workspace lock and IPC
  endpoint
* Different repo roots intentionally create separate daemons and multiply memory
  and CPU cost
* A parent directory containing independent repos is not a supported federated
  workspace unless it is itself a valid Git root
* Current evidence is insufficient to promise reliable 5,000-file indexing

## Evidence

* Current shim readiness timed out twice
* Two live workspace daemons each used approximately 1.2-1.3 GiB working memory
* Full indexing processes files serially and does not use `parse_concurrency`
* Rust indexing has a 64 MiB aggregate prepass snapshot ceiling
* No checked-in acceptance test indexes 5,000 files and then runs a real query
* A prior 1,382-file auto-reindex incident consumed more than 14 GiB

## Failed Approaches

* Live daemon status could not be retrieved because the shim remained in the
  classified `readiness_timeout` state

## Open Questions

* Exact corpus composition and peak RSS for the proposed 5,000-file workload
* Precise startup stage delaying the installed daemon's readiness
* Whether future multi-repo support should be federated or isolated

## Next Steps

* Build a release-mode 5,000-file scale acceptance test
* Profile startup and indexing memory
* Define and document the supported multi-repo operating model

