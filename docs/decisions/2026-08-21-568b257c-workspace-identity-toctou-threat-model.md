---
title: 568B257C threat model — workspace identity and Git metadata TOCTOU
date: 2026-08-21
type: threat-model
status: resolved
source_stash_id: 568B257C
agent: stage
confidence: high
---

## Scope

`src/db/workspace.rs` — workspace admission (`canonicalize_workspace`),
Git metadata resolution (`resolve_git_metadata`), and workspace identity
persistence (`load_or_create_workspace_id`, `daemon_key_for_workspace`).

Traceability: Shipment 118-S, Feature 122-F, review commit `2ef18c0d`
("fix: harden worktree startup review findings").

## Trust Boundary

Engram runs with the operator's full filesystem permissions. The workspace root
and every ancestor directory are attacker-influenceable in the threat model:
shared build agents, multi-user machines, and any process running as the same
user can create, replace, or relink directories along the path. Git admin
metadata for a linked worktree lives **outside** the workspace root by design,
so the validated region necessarily extends past the containment boundary.

## Current Implementation

`resolve_git_metadata` validates the native worktree backlink chain through a
sequence of independent, **path-based** filesystem calls:

* `path.canonicalize()` for the workspace root
* `std::fs::symlink_metadata(workspace/.git)`
* `read_metadata_file(.git)` — `symlink_metadata` then `read_to_string`
* `canonical_path(admin_candidate)` + `require_plain_directory(admin_candidate)`
* `read_metadata_file(admin_dir/commondir)`
* `require_plain_directory(common_candidate)`, `canonical_path(common_candidate)`
* `require_plain_directory(common_dir/worktrees)`, `require_plain_directory(common_dir/objects)`
* `require_plain_reference_storage(common_dir)`
* `canonical_path(worktrees_candidate)`
* `read_metadata_file(common_dir/HEAD)`
* `read_metadata_file(admin_dir/gitdir)` (backlink)
* `canonical_path(backlink_candidate)`, `canonical_path(git_entry)`
* `read_metadata_file(admin_dir/HEAD)`

Every one of these re-walks the full path from the filesystem root. Roughly
twenty independent resolutions, each an opportunity for the namespace to change
between validation and use.

Capability-rooted, no-follow access already exists in this same file — but only
for `.workspace-id`: `read_workspace_id` uses `cap_fs_ext::FollowSymlinks::No`
and `OFlags::NOFOLLOW`, and `is_workspace_id_link_or_reparse` checks
`FILE_ATTRIBUTE_REPARSE_POINT` directly rather than relying on `is_symlink()`.
The Git metadata chain does not use either technique.

## Threats

### T1 — Parent-directory replacement (check/use TOCTOU) — High

Between any validating call and the subsequent using call, an attacker replaces
an **ancestor** directory of the target with a directory (or link) they control.
Because each call re-resolves from the root, the validated object and the used
object can be different objects. `read_metadata_file` is the sharpest instance:
it calls `symlink_metadata(path)` and then `read_to_string(path)` — two
resolutions of the same path with a window between them. The attacker does not
need to touch the final component; swapping any ancestor is sufficient.

**Impact**: engram admits an attacker-controlled directory as a trusted
workspace, derives workspace identity and the daemon IPC key from it, and
persists `.engram/` state there.

### T2 — Metadata check/read divergence on the final component — High

Same window as T1 but on the leaf: `symlink_metadata` reports a plain file, the
attacker replaces the leaf with a link or a different file, and `read_to_string`
reads the substituted content. This directly subverts the `gitdir:` directive,
the `commondir` content, and the `gitdir` backlink — the three values the entire
worktree-authenticity proof rests on.

### T3 — Validation/use divergence introduced by `canonicalize` — High

`canonical_path` calls `Path::canonicalize`, which **follows** symlinks all the
way down. `require_plain_directory` calls `symlink_metadata`, which does not.
The code therefore validates one resolution semantics and compares against
another, taken at a different instant. The comparison
`normalize_metadata_pointer(admin_text, &admin_candidate) != admin_dir` is only
meaningful if the namespace is stable across both calls, which is exactly what
an attacker controls.

### T4 — Reparse-point classes not covered by `is_symlink()` — Medium

`require_plain_directory`, `require_plain_reference_storage`, and
`read_metadata_file` gate on `metadata.file_type().is_symlink()`. Rust's Windows
`is_symlink()` covers `IO_REPARSE_TAG_SYMLINK` and `IO_REPARSE_TAG_MOUNT_POINT`
(junctions) but not other reparse tags — cloud/placeholder providers, WSL
`LX_SYMLINK`, app-execution links, and container-isolation links among them. The
same file already applies the broader `FILE_ATTRIBUTE_REPARSE_POINT` test for
`.workspace-id` (`is_workspace_id_link_or_reparse`), so the codebase already
concedes that `is_symlink()` alone is insufficient — the Git metadata chain is
simply inconsistent with that conclusion.

### T5 — Identity/IPC-key poisoning — High (consequence of T1–T3)

`load_or_create_workspace_id` calls `resolve_git_metadata` and then operates on
`.engram/` under the returned root. `daemon_key_for_workspace` derives the daemon
discovery key from that identity. A successful T1–T3 attack redirects both the
persisted identity and the IPC endpoint, enabling an attacker to be handed
daemon connections intended for a legitimate workspace.

### T6 — Non-Windows symlink races — Medium

On Linux and macOS the same path-based pattern is exposed to ordinary symlink
swaps in any world-writable or user-writable ancestor. The defect is not
Windows-specific; only T4's reparse taxonomy is.

## Chosen Direction

**Retained, capability-rooted, no-follow directory handles.**

1. Open the workspace root once as a `cap_std::fs::Dir` and retain the handle.
2. Resolve every subsequent component through `openat`-style operations on the
   retained handle with `FollowSymlinks::No` / `OFlags::NOFOLLOW`, one component
   at a time. Never re-resolve a full path string after validation.
3. Derive all metadata from the **open handle** (fstat on the descriptor), not
   from a second path lookup, so check and use address the same object.
4. Read metadata file contents from the already-open, already-validated handle
   rather than reopening by path.
5. Apply the broader `FILE_ATTRIBUTE_REPARSE_POINT` rejection uniformly on
   Windows, matching what `.workspace-id` handling already does.
6. Keep the cross-boundary reach explicit: the linked-worktree admin directory is
   outside the workspace root, so it gets its own retained root handle, opened
   once, with the backlink proof evaluated entirely against retained handles.

`cap-std` and `cap-fs-ext` are already dependencies of this crate and already
used in this file, so no new dependency is introduced.

## Rejected Alternatives

| Alternative | Why rejected |
|---|---|
| Re-validate after use ("check, use, re-check") | Narrows but does not close the window; an attacker can restore the benign state after the read. |
| Refuse all linked worktrees | Regresses Feature 122-F, which exists to support native worktrees. |
| Lock the parent directories | Not portable; Windows and POSIX offer no equivalent guarantee for arbitrary ancestors under attacker control. |
| Rely on `canonicalize` alone | `canonicalize` follows links and is itself a fresh resolution; it is the source of T3, not a mitigation. |

## Acceptance Criteria (adversarial, cross-platform)

1. **Ancestor swap during resolution** — a test harness that replaces an ancestor
   directory of the git admin path between validation and use MUST cause
   admission to fail, on Windows and on Unix.
2. **Leaf swap between check and read** — replacing a metadata file with a link
   after its metadata check MUST cause admission to fail.
3. **Windows junction substitution** — substituting `worktrees`, `objects`, or
   `refs` with a directory junction MUST be rejected.
4. **Windows non-symlink reparse point** — substituting a validated directory
   with a reparse point whose tag is neither `SYMLINK` nor `MOUNT_POINT` MUST be
   rejected.
5. **Unix symlink substitution** — the equivalent symlink swap on Unix MUST be
   rejected.
6. **Handle identity** — the object admitted MUST be provably the same object
   that was validated (verified via handle-derived identity, not path equality).
7. **No regression** — a legitimate primary checkout and a legitimate native
   `git worktree` MUST both still be admitted.

## Traceability

Source stash `568B257C`. Shipment 118-S, Feature 122-F, review commit `2ef18c0d`.
