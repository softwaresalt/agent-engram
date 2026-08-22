---
title: Converting a resolver to capability handles is not done until every consumer is converted
date: 2026-08-21
type: compound-learning
tags: [security, toctou, capability, filesystem, rust, cap-std, review]
source: shipment 121-S, PR #353, stash 568B257C
---

## Problem

Shipment 121-S replaced ~20 path-based filesystem resolutions in engram's
workspace admission gate with retained `cap_std::fs::Dir` capability handles, to
close ancestor-swap and check/read TOCTOU windows.

The core resolver rewrite landed cleanly and passed a four-model adversarial
review with zero HIGH-confidence P0/P1 findings. Review then found **six more
residual path-based reads across eight subsequent cycles**, each in a different
consumer of the resolver's result:

1. `.engram` presence probe opened ambiently, following a reparse leaf.
2. `daemon_key_for_workspace` probed through a handle, dropped it, then
   re-entered the path-based `workspace_key` wrapper.
3. `.workspace-id` was created and published by ambient pathname.
4. The legacy PID fallback read `.engram/run/engram.pid` by pathname.
5. The root anchor was reopened from the canonicalized string.
6. Three call sites in `tools/lifecycle.rs` each resolved the workspace
   independently for canonical path, identity, and branch.

## Lesson

**A capability rewrite is a graph problem, not a function problem.** Converting
the resolver only moves the trust boundary to its callers. Before starting, walk
every consumer of the resolver's output and classify each as:

* *carries the capability* — takes the handle, no re-resolution;
* *re-resolves* — must be converted or explicitly accepted;
* *composes several results* — the composition itself is a window even when each
  call is individually sound.

The third class is the one that survives review longest, because each
participating function looks correct in isolation.

Practical check: grep the module for `canonicalize`, `symlink_metadata`,
`read_to_string`, `open_ambient_dir`, and `std::fs::` **after** the rewrite, and
justify every remaining hit in a comment. If a comment claims an invariant
("only two ambient resolutions"), count them — in this change that comment was
false by a factor of four and had to be corrected during review.

## Durability and availability constrain the publish protocol

Publishing a security-relevant identity file through a capability handle is
narrower than it looks. Three attempts failed:

| Approach | Failure |
|---|---|
| `create_new` on the final name | Publishes an empty leaf before the write lands; concurrent readers see an invalid identity, and a crash leaves it unrecoverable |
| temp + `rename` | `rename` replaces the destination, so concurrent first binds diverge onto different identities |
| Fail the bind when `hard_link` is unsupported | Real CI regression: the launcher pre-warm budget blew from 8 s to 21–22 s because a failed publish made the bind fail and forced the sequential fallback |

What worked: write and `fsync` a uniquely-named staging file through the handle,
`hard_link` it onto the final name (which refuses to replace), then unlink the
staging name — degrading to a **checked** rename only where the filesystem
cannot link.

`cap-std` 4.0.2 has no `renameat2`/`RENAME_NOREPLACE` wrapper and no `sync_all`
on `Dir`, so parent-directory durability is not reachable from a capability
handle at all. Know these gaps before committing to a design.

## Test the policy, not only the fixture

The Windows reparse-breadth claim — reject *all* reparse tags, not the
`SYMLINK`/`MOUNT_POINT` subset `is_symlink()` covers — could not be tested with a
fixture: creating a reparse point with an arbitrary tag needs
`DeviceIoControl`, and the crate forbids `unsafe`. Junction fixtures pass
against the *unfixed* code, so they prove nothing about the breadth.

Assert the policy at its decision point instead: call the predicate with
`is_symlink = false` and `FILE_ATTRIBUTE_REPARSE_POINT` set. That is exactly the
input the old gate admitted and the new gate must reject, and it needs no
privileged fixture.

Corollary: an adversarial fixture that passes against the pre-fix code is a
no-regression test, not a RED test. Run the suite at the RED commit and record
which scenarios actually fail — in 121-S only two of seven did.

## Deterministic TOCTOU testing needs a seam

Timing-race harnesses produce green suites that prove nothing. A `#[cfg(test)]`
`toctou_checkpoint` seam that is a no-op under `cfg(not(test))` lets a
colocated test stage the swap at a named point with no timing dependency.

Two requirements make it honest:

* the tests must live **colocated** in the module, because an external test
  crate links the library built with `cfg(not(test))` and cannot see the seam;
* "checkpoint never fired" must **fail**, not pass. Sharing that arm with the
  prevention outcome means deleting the checkpoint silently turns the security
  test green.
