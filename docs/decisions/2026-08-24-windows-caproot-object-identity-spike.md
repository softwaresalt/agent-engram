---
title: "Safe Windows object identity from a retained CapRoot"
type: spike
date: 2026-08-24
time_box: "2h"
conclusion: proceed
confidence: high
source_stash_id: "1C2A3CB3"
promoted_to:
  - plan
tags: [windows, capability, filesystem, identity, security]
---

# Safe Windows object identity from a retained CapRoot

## Goal

Can safe, stable Rust prove that a Windows canonical workspace name denotes the same filesystem object as the retained `cap_std::fs::Dir`, without unsafe code or unstable APIs?

## Success Criteria

Find a public API already compatible with pinned `cap-std`/`cap-fs-ext` 4.0.2 that derives comparable identity from handle metadata, or record a concrete blocker.

## Scope Constraints

Read-only dependency and repository inspection. No prototype, build, test, source edit, dependency upgrade, or unsafe code.

## Investigation Approach

1. Re-read the 121-S compound learning and closure residual.
2. Inspect pinned `cap-std` and `cap-primitives` metadata APIs.
3. Inspect the already-pinned `cap-fs-ext` public extension API.
4. Compare the result with current `CapRoot::object_identity`.

## Findings

### What Was Discovered

The prior residual overlooked the public `cap_fs_ext::MetadataExt` trait. In `cap-fs-ext` 4.0.2, this safe public trait implements `dev()` and `ino()` for `cap_primitives::fs::Metadata` on Windows by using the crate-private `_WindowsByHandle` bridge internally. `CapRoot::dir.dir_metadata()` already returns handle-derived capability metadata. Engram already depends on `cap-fs-ext = 4.0.2`. No raw-handle borrowing, direct internal trait import, unsafe block, or dependency change is required.

A cross-platform `CapRoot::object_identity` can therefore use `cap_fs_ext::MetadataExt::{dev, ino}` for metadata taken from the retained handle. A second `CapRoot::open_anchor(candidate)` yields the named-object identity for comparison.

### What Was Tried and Failed

Direct `cap_std::fs::MetadataExt` access to Windows volume/file index is cfg-gated, and direct `_WindowsByHandle` use is an internal implementation detail. Borrowing a raw Windows handle would violate `#![forbid(unsafe_code)]`. Those approaches remain rejected because the public `cap-fs-ext` wrapper exists.

### Remaining Unknowns

`cap-fs-ext` documents that ReFS file identifiers can be 128-bit while `ino()` is 64-bit. The implementation plan must not claim collision-free ReFS identity without evidence. Windows runtime verification must record filesystem type; NTFS is the required first gate, with ReFS treated as an explicit residual or separate follow-up.

## Recommendation

**Conclusion: proceed. Confidence: high.** Use the already-pinned public `cap_fs_ext::MetadataExt` over handle-derived metadata, add a deterministic Windows RED policy test proving two different directories are rejected, and keep the no-unsafe gate explicit. Do not upgrade dependencies.

## Next Steps

Promote to the implementation plan. Require hardening and adversarial multi-model review before harvest because this changes a trust-boundary proof.

## References

- `cap-fs-ext-4.0.2/src/metadata_ext.rs`
- `cap-std-4.0.2/src/fs/dir.rs`
- `cap-primitives-4.0.2/src/fs/metadata.rs`
- `src/db/workspace.rs::CapRoot::prove_names_same_object`
- `docs/closure/2026-08-21-568b257c-runtime-verification.md`
