---
title: Shipment 121-S — capability-rooted workspace identity
date: 2026-08-21
type: session-memory
shipment: 121-S
feature: 125-F
pr: 353
merge_sha: 119230fe849558b35e8889d4ae1e37c4fdda6010
agent: ship
---

## Outcome

Shipment 121-S shipped. PR #353 merged as `119230fe` (merge commit, 2 parents).
Feature 125-F and tasks 125.001-T through 125.008-T archived. Shipments 122-S
and 123-S left queued with batch metadata byte-identical.

## What shipped

`src/db/workspace.rs` admission moved from ~20 independent path-based
resolutions to retained capability roots (`cap_std::fs::Dir`), opened once and
walked one component at a time with no-follow opens. Metadata comes from the
open handle; content is read from that same handle.

Closed: ancestor rename-swap TOCTOU, metadata check/read TOCTOU, symlink /
junction / broader Windows reparse substitution, `.git` file-vs-dir indirection.
Identity persistence and the daemon IPC key now consume the root handle retained
from the authenticity proof.

## Decisions and rationale

* **Adopted the interrupted prior attempt as basis, then re-verified.** A prior
  session had left two unpushed commits and a non-compiling working tree on
  `feat/121-s-capability-rooted-workspace-identity`. Its base `4a16c698` was an
  ancestor of `origin/main`, the work was sound, and the security reasoning was
  good — so it was cherry-picked onto a fresh branch cut from the exact
  authoritative `origin/main` and re-verified end to end rather than rewritten.
  The residue worktree was left untouched.
* **RED was captured, not assumed.** At the RED commit the pre-fix resolver
  returned `Ok(GitMetadata { .. })` for a swapped ancestor and two colocated
  scenarios failed. Only then was the fix applied.
* **Reparse breadth is tested at the policy decision point, not via a fixture.**
  A reparse point with a tag outside `SYMLINK`/`MOUNT_POINT` cannot be created
  without `DeviceIoControl`, which `#![forbid(unsafe_code)]` rules out. The
  breadth claim is asserted against `is_link_or_reparse` directly.
* **Availability was weighted against security on two calls.** The reparse gate
  is scoped to the validated chain, so a reparse ancestor and a junction
  workspace root both stay admitted. The identity publish degrades to a rename
  where the filesystem cannot hard-link rather than refusing the bind.

## Failed approaches

* **Exclusive `create_new` on the final `.workspace-id`.** Publishes an empty
  leaf before the UUID lands, so 32 concurrent cold starts read a not-yet-written
  file. Caught by `concurrent_cold_starts_share_the_atomically_created_workspace_id`.
* **Handle-relative temp-then-`rename`.** `rename` replaces its destination, so
  concurrent first binds diverged onto different identities. Same test caught it.
* **Failing the bind when `hard_link` is unsupported.** This regressed CI:
  `start-launcher-windows` failed three times with elapsed 10.6 s / 22.7 s /
  21.0 s against an 8 s budget, matching the >20 s sequential fallback path,
  because a failed publish made the bind fail. Degrading to a checked rename
  turned the check green again.
* **Windows handle-derived object identity.** `cap-std` exposes volume serial
  and file index only through the unstable `_WindowsByHandle` trait; reaching it
  from a `std` handle needs `unsafe`. The identity proof is therefore Unix-only.

## Review

Standard review escalated to adversarial multi-model review per the plan.
Security specialist (Opus 4.8) plus four cross-model reviewers (Opus 4.8,
GPT-5.6 Sol, Gemini 3.1 Pro, Grok 4.6). Verdict **GATE PASS**, zero
HIGH-confidence P0/P1 consensus findings.

Copilot review then ran eight fix cycles against a documented limit of three.
Each cycle surfaced a genuine residual path-based read, which is itself the
lesson: a "capability-rooted" rewrite is not done when the core resolver is
converted — every consumer of the result is a separate re-resolution.

## Deferred follow-ups

| Stash | Item |
|---|---|
| `06FC0F11` | U5 handle-identity scenario through identity persistence |
| `5DF94427` | fsync the `.engram` directory after publishing the identity leaf |
| `1C2A3CB3` | Windows equivalent of the root canonical-name identity proof |
| `49000348` | Verify cloud-placeholder reparse tags inside `.git` are still admitted |
| `1CB366DB` | Compose canonical path, identity, and branch from one proof at bind sites |
| `7B15B447` | Keep one `.engram` capability alive across the daemon-key decision |

## Next

122-S is the unique earliest queued member of batch
`dark-factory-20260820-870b1aff-568b257c-c2413934-de460a88`, order 3,
predecessors `[120-S, 121-S]` — both now shipped. No active shipment.
