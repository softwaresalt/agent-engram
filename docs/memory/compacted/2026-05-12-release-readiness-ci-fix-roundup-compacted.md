---
title: "May 12 Release Readiness and CI Fix Roundup"
type: compacted-memory
date: 2026-05-12
shipments:
  - 035-S
prs:
  - 130
feature: 034-C
sources:
  - docs/archive/memory/2026-05-12/daemon-release-readiness-stage-memory.md
  - docs/archive/memory/2026-05-12/pr-130-ci-fix-memory.md
---

## Summary

* Stage captured daemon and CLI release-hardening intake around lock files, PID state, workspace lifecycle, and workflow validation, then created implementation-ready backlog structure for 035-S
* PR #130 was driven green across four commits: startup hydration lock contention, startup sync race, Markdown IPC polling flake, and final Copilot review follow-ups
* The session explicitly recorded degraded-mode tool availability and used backlogit CLI fallbacks successfully

## Key Decisions

* Treat startup lock contention and queued sync as serialization problems, not separate flukes
* Keep degraded capability surfaces visible when backlogit, engram, or intercom tool surfaces are unavailable
* Use targeted integration tests plus PR CI as the authoritative signal for startup fix regressions

## Verification

* cargo fmt, cargo clippy, cargo dev-test, and cargo audit were part of the validation set
* PR #130 CI returned green after the final fix commit

## Open Items

* Archive housekeeping on the release-readiness branch remained pending at session end
* A few stash entries were still awaiting harvest markers