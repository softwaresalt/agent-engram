---
title: "Capability-safe Unix parent-directory durability for workspace identity"
type: decision
doc_type: decision
source: "stash 5DF94427; PR #353 and PR #363 review"
date: 2026-08-24
status: decided
source_stash_id: "5DF94427"
promoted_to:
  - docs/exec-plans/2026-08-24-5df94427-workspace-id-parent-fsync-plan.md
---

# Capability-safe Unix parent-directory durability for workspace identity

## Problem Frame

`publish_new_child_file` fsyncs staging-file content, publishes `.workspace-id`, removes the staging name, and returns without making the parent directory entry durable. A Unix crash can therefore lose the published name and cause a later bind to mint a different UUID. The caller converts `AlreadyExists` into a successful winner read, so parent-sync failure must also block that otherwise-successful path.

## Evidence

Pinned `cap-std` 4.0.2 exposes safe `Dir::reopen_dir(&dir)` and `Dir::into_std_file()`. On Unix, a reopened capability can be converted to `std::fs::File` and `sync_all()` without an ambient path or unsafe code. On Windows, cap-std opens the directory handle for read access while `File::sync_all()` reaches `FlushFileBuffers`, which requires write access; applying the same helper there would make legitimate first bind fail. Current `workspace_id_from_metadata` returns a handle-derived UUID both after `Ok(())` publication and after `AlreadyExists` winner read.

## Decision

On Unix only, add a capability-relative parent-directory sync after staging cleanup. Invoke it when publication returns `Ok(())` or `AlreadyExists`, before either the newly published UUID or the existing winner UUID can be returned. Parent-sync failure takes precedence over both otherwise-successful outcomes. If publication fails for any other reason, retain and return the original publication-derived error without masking it with a sync attempt.

Preserve hard-link-first and checked-rename fallback semantics. Use deterministic protocol-order and failure-injection tests rather than claiming that a unit test reproduces a crash. Preserve current Windows behavior and document its durability residual until a supported Windows primitive is proven.

## Test-First Contract

- New-file path: `file-sync -> publish -> cleanup -> parent-sync -> winner read/return`; injected parent-sync failure returns the sync-derived error.
- Existing-winner path: `file-sync -> AlreadyExists -> cleanup -> parent-sync -> winner-read return`; injected parent-sync failure overrides the valid winner-read outcome.
- Other publication failure: preserve the existing publication error; do not mask it with parent-sync failure.
- Failure injection uses a private safe test seam or capability wrapper; no unsafe code, raw handles, ambient reopen, permission mutation, or power-loss claim.
- Windows test proves that no parent-sync event occurs.

## Constraints

- No ambient reopen of `.engram`.
- No unsafe code and no dependency upgrade.
- RED must prove both successful-path orderings and exact error precedence.
- Windows must not invoke `sync_all()` on the read-only directory handle; parent-entry durability remains a documented residual.
- This is a separate release unit from bind/daemon composition to preserve width.
- Genuine multi-model adversarial review remains mandatory before harvest.

## References

- Stash `5DF94427`
- Deliberation `025-D`
- PR #353 review
- PR #363 thread `discussion_r3848530354`
- `docs/compound/capability-rewrite-must-convert-every-consumer-2026-08-21.md`
- `src/db/workspace.rs`
