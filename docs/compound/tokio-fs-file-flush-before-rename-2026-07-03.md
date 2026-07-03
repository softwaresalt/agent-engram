---
title: "tokio::fs::File append needs explicit flush().await before any rename/rotation"
description: "An async append helper that writes via tokio::fs::File::write_all and returns without flush().await can drop the just-written line when a later operation renames the file (e.g. size-cap rotation). write_all does not guarantee the bytes have reached the OS; the rename can win the race. Reproduces deterministically on Linux CI, but often passes on Windows in isolation, so it is easy to misdiagnose as a Windows flake."
problem_type: "async_io_data_loss"
category: "language-hazard"
component: "src/services/metrics.rs usage.jsonl emitter"
root_cause: "tokio::fs::File is a state machine over a blocking threadpool with pending write operations; write_all().await returns before the write is guaranteed durable to the OS. Dropping the file without flush() and then renaming it on the next call races the in-flight write."
resolution_type: "code_fix"
date: "2026-07-03"
shipment: "067-S"
---
# tokio::fs::File append needs explicit flush().await before rename/rotation

## Problem

`append_usage_line` appended a JSONL record with:

```rust
file.write_all(&buffer).await?;
Ok(()) // file dropped here, no flush
```

The size-cap rotation path renames `usage.jsonl` -> `usage.1.jsonl` (and shifts
generations) at the *start* of the next append, before writing the new line.
Because `write_all().await` on a `tokio::fs::File` does **not** guarantee the
bytes have landed on the OS (the file is a state machine over spawn_blocking
with its own pending-op buffer), the previous write could still be in flight
when the rename ran — dropping a just-recorded line.

Symptom: the rotation test asserted 4 recorded lines but found 3
(`left: 3, right: 4`). It **passed in isolation on Windows** (different FS
timing) and only failed on **Ubuntu CI**, which made it look like a Windows
flake. It is not — it is a real ordering bug that Linux exposes deterministically.

## Fix

Flush the tokio file before returning so the write is complete before any later
rename can observe (or move) the file:

```rust
file.write_all(&buffer).await?;
file.flush().await?; // drain the pending write to the OS before returning
Ok(())
```

`flush()` comes from `tokio::io::AsyncWriteExt` (already in scope wherever
`write_all` is used). `sync_all()`/fsync is not required for same-process read
correctness or rename ordering — `flush()` is sufficient and minimal.

## Lessons

- Any `tokio::fs::File` write that is followed by a rename, move, read, or
  process handoff MUST be flushed explicitly. Do not rely on drop.
- When a filesystem-timing test fails on Linux CI but passes locally on
  Windows, treat CI as authoritative — do not dismiss it as a "Windows flake."
  The platform that fails deterministically is the one telling the truth.
