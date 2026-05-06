---
type: compound-learning
category: security
date: 2026-05-06
feature: 002-F — Backlog Markdown Hydration
severity: high
confidence: confirmed
---

# Windows Drive-Relative Path Traversal: Use `Component::Prefix` Guard

## Problem

`Path::is_absolute()` does **not** catch Windows drive-relative paths like `C:foo`.
On Windows, `Path::new("C:foo").is_absolute()` returns `false`, but
`workspace_root.join("C:foo")` ignores the workspace root and resolves relative to
the drive's current directory — escaping the workspace containment boundary.

A workspace containment check that only tests `is_absolute()` is bypassable on Windows
even with no `..` traversal components.

## Fix

Check for `Component::Prefix(_)` in addition to `is_absolute()`:

```rust
use std::path::Component;

fn is_within_workspace(workspace_root: &Path, candidate: &Path) -> bool {
    // Reject Windows drive-relative paths (e.g. "C:foo") that escape join()
    if candidate.components().any(|c| matches!(c, Component::Prefix(_))) {
        return false;
    }
    let resolved = workspace_root.join(candidate);
    resolved.starts_with(workspace_root)
}
```

`Component::Prefix(_)` only appears in Windows paths with drive letters (e.g. `C:`, `C:\`)
or UNC prefixes (e.g. `\\server\share`). It is safe to reject all such paths when the
intent is to enforce workspace containment — legitimate relative paths never have a prefix.

## Evidence

- `src/services/ingestion.rs` — Round 4 Copilot review comment `PRRT_kwDORJEduc5_3hnQ`
- PR #82, commit `fdd9f1c`

## Applicability

Any path containment check in the codebase that accepts user-supplied or registry-supplied
paths MUST include this guard on Windows. CI runs on Linux where `C:foo` is treated as a
literal relative path (harmless), so tests alone do not catch this on CI.
