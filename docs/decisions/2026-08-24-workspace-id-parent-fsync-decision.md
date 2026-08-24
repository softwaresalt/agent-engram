---
title: "Capability-safe parent-directory durability for workspace identity"
type: decision
doc_type: decision
source: "stash 5DF94427; PR #353 review"
date: 2026-08-24
status: decided
source_stash_id: "5DF94427"
promoted_to:
  - docs/exec-plans/2026-08-24-5df94427-workspace-id-parent-fsync-plan.md
---

# Capability-safe parent-directory durability for workspace identity

## Problem Frame

`publish_new_child_file` fsyncs staging-file content, publishes `.workspace-id`, removes the staging name, and returns without making the parent directory entry durable. A Unix crash can therefore lose the published name and cause a later bind to mint a different UUID.

## Evidence

Pinned `cap-std` 4.0.2 exposes safe `Dir::reopen_dir(&dir)` and `Dir::into_std_file()`. A reopened capability can be converted to `std::fs::File` and `sync_all()` without an ambient path or unsafe code. The compound learning for 121-S explicitly corrects the earlier PR comment that this was unreachable.

## Decision

Add a capability-relative parent-directory sync after final publish and staging cleanup, before reporting success. Preserve the hard-link-first and checked-rename fallback semantics. Use deterministic protocol-order tests rather than pretending a crash can be reproduced reliably in a unit test.

## Constraints

- No ambient reopen of `.engram`.
- No unsafe code and no dependency upgrade.
- RED must prove publish/cleanup/parent-sync/return ordering and error propagation.
- Windows support must be verified rather than assumed; the Unix durability guarantee is mandatory.
- This is a separate release unit from bind/daemon composition to preserve width.

## References

- Stash `5DF94427`
- PR #353 review
- `docs/compound/capability-rewrite-must-convert-every-consumer-2026-08-21.md`
