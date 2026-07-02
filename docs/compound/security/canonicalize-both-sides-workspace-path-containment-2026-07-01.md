---
title: "Workspace path containment: canonicalize both sides, resolve relative paths against the workspace root not CWD"
date: 2026-07-01
category: security
confidence: high
evidence:
  - docs/closure/2026-07-01-052-S-engram-verify-cli-closure.md (PR #185 Review Disposition item 2; commits 9f0bb3d -> 93d670b)
  - src/services/verify.rs::run_verify / contain_path (engram verify Phase 1a)
  - Copilot thread PRRT_kwDORJEduc6NrQsy (Windows verbatim-prefix follow-up)
---

## Problem

A CLI that accepts a user-supplied `<path>` and must keep reads inside a workspace
boundary has two easy-to-miss containment bugs:

1. **Relative path resolved against the process CWD.** If `engram verify foo.md`
   joins `foo.md` onto the current directory instead of the `--workspace` root, a
   file that exists only under the CWD gets read even though it is outside the
   intended workspace. The gate silently validates the wrong file.

2. **`starts_with` containment on non-canonicalized paths.** Comparing a raw
   target path against a raw root lets `..` segments, symlinks, or Windows
   verbatim prefixes (`\\?\`) defeat the check — `root.starts_with` can pass (or
   fail) for paths that canonicalize to a different location.

## Solution

Resolve and canonicalize deterministically, and compare canonical-vs-canonical:

* Canonicalize the workspace root **once** up front; never fall back to
  `std::env::current_dir()` for resolving the target.
* Resolve a relative `<path>` by joining it under the **canonicalized workspace
  root**, not the CWD.
* Canonicalize the **target** when it exists; when it does not exist, join it
  lexically under the already-canonicalized root (so a missing file still yields a
  path anchored inside the boundary).
* Enforce `resolved_target.starts_with(workspace_root)` with **both sides
  canonicalized**. A target that resolves outside → error exit (never a silent
  pass).
* A missing/unreadable target maps to the error exit code regardless of
  extension — check existence/readability **before** any content-type branch.

```rust
// pseudocode of the safe shape
let root = canonicalize(workspace_root)?;            // once
let target = if path.is_absolute() { path } else { root.join(path) };
let resolved = match canonicalize(&target) {
    Ok(p) => p,                                       // exists -> real canonical path
    Err(_) => lexical_join(&root, path),              // missing -> anchored under root
};
if !resolved.starts_with(&root) { return Err(outside_workspace); }
```

## When This Applies

Any tool that gates on a user-supplied path inside a trust boundary: linters,
ingestion/sync watchers, subprocess file arguments. The RED test that proves it:
a file that exists **only** under the CWD must NOT be read when a `--workspace`
distinct from the CWD is supplied (it must resolve under the workspace, be absent
there, and return the error exit).

## Detection

Reviewers flag `current_dir()`-based joins and single-side `starts_with`
containment. On Windows, the verbatim-prefix (`\\?\`) case is the residual gap:
normalize both sides (e.g. via a shared `normalize_canonical`) before the
containment check so a canonicalized target with a `\\?\` prefix still matches a
canonicalized root.
