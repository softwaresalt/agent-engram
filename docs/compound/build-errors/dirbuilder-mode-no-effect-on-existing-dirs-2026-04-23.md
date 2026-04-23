---
title: "DirBuilder::mode() does not change permissions on pre-existing directories"
date: 2026-04-23
tags: [build-errors, filesystem, permissions, unix, daemon]
confidence: high
evidence: src/daemon/ipc_server.rs — fixed in PR #21 (009-S), commit 32df870
---

# Problem

When hardening socket directory permissions, using:

```rust
DirBuilder::new().mode(0o700).recursive(true).create(&socket_dir)?;
```

…sets `0o700` on the directory **only if it is newly created**. If the
directory already exists, `DirBuilder::mode()` has no effect. The resulting
permissions remain whatever they were when the dir was first created.

This means a re-started daemon with a pre-existing socket dir silently skips
the permission enforcement, leaving the dir potentially world-readable.

# Solution

Verify permissions after the `create()` call regardless of whether the dir
was newly created:

```rust
DirBuilder::new().mode(0o700).recursive(true).create(&socket_dir)?;

// Verify post-create — DirBuilder::mode() has no effect on pre-existing dirs
let meta = std::fs::metadata(&socket_dir)?;
let actual = meta.permissions().mode() & 0o777;
if actual != 0o700 {
    std::fs::set_permissions(&socket_dir, std::fs::Permissions::from_mode(0o700))?;
}
```

Or, as a simpler compensating action: always call `set_permissions` after create:

```rust
DirBuilder::new().mode(0o700).recursive(true).create(&socket_dir)?;
std::fs::set_permissions(&socket_dir, std::fs::Permissions::from_mode(0o700))?;
```

# When This Applies

Any code that uses `DirBuilder::mode()` on a directory that may already exist
(e.g., daemon restart scenarios, temp dirs reused between runs).

# Notes

This is a Unix-only concern (`mode()` is a `std::os::unix::fs::DirBuilderExt`
method). On Windows the concept of Unix-style mode bits does not apply.
