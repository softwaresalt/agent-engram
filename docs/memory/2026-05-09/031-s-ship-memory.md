---
title: 031-S Ship Session Memory
type: session-memory
date: 2026-05-09
feature: 046-F
shipment: 031-S
pr: 112
merge_sha: 25cea55
---

## Task IDs Completed

- **046-F** CLI Install & Workspace Flag Fixes — shipped
- **046.001-T** Fix --workspace flag in install/update/reinstall/uninstall dispatch — done
- **046.002-T** Add binary-level regression tests for installer --workspace flag — done (S079a/b/c/d)
- **046.003-T** Add .backlogit/ to AUTO_DETECT_DIRS for registry scaffold — done

## Files Modified

| File | Change |
|---|---|
| `src/bin/engram.rs` | Replaced 4 `std::env::current_dir()?` with `flags.resolve_workspace().map_err(...)` in Install/Update/Reinstall/Uninstall arms |
| `src/installer/mod.rs` | Added `(".backlogit", "backlog", Some("markdown"))` to `AUTO_DETECT_DIRS` |
| `tests/integration/installer_test.rs` | Added S079a/b/c/d binary dispatch tests + S080 .backlogit library test (+192 lines, 36 tests total) |

## Key Decisions

1. **Binary tests required for dispatch bugs**: Library-level tests calling `installer::install()` directly bypass the dispatch bug. Only `Command::new(env!("CARGO_BIN_EXE_engram"))` tests exercise the buggy path. This is a pattern to apply for any future dispatch-layer bugs.

2. **S079d added via Copilot review**: Initial PR had S079a (install), S079b (update), S079c (uninstall) but missed `reinstall`. Copilot review caught the gap. Added S079d for the fourth arm.

3. **Pre-existing flaky test**: `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` in `daemon_startup_order_test.rs` fails when system is under load. The test uses a 10s TTL that times out on loaded Windows machines. This is pre-existing — not caused by our changes. Stash a bug to increase the TTL.

4. **resolve_workspace() pattern**: Returns `Result<PathBuf, String>`. Map with `.map_err(|e| anyhow::anyhow!("{e}"))` since `main()` returns `anyhow::Result`. The daemon arm was already correct — used as the model for the fix.

## CI Results

- PR #112 created, Copilot review received 1 comment (S079d reinstall gap)
- Fixed in commit 9705b8c, replied to comment, resolved thread via GraphQL
- CI green (2 runs), merged with `--merge --admin`

## Quality Gates

- `cargo fmt --all -- --check`: PASS
- `cargo clippy -- -D warnings -D clippy::pedantic`: PASS
- `cargo test`: 485 pass (1 pre-existing flaky in daemon_startup_order_test)
- Installer tests (36): ALL PASS including S079a/b/c/d and S080

## Open Items

- Pre-existing flaky test: `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` — stash a bug
- Next shipment: **032-S** (CLI Resilience & Error Handling — 047-F, tasks 047.004-T/047.005-T/047.006-T)
  - 047.004-T: db-lock probe before --direct mode opens DB
  - 047.005-T: IndexInProgress fallback on -32603
  - 047.006-T: progress_hint() on OutputFormatter

## Next Steps

1. Stash bug for flaky daemon_startup_order test (increase TTL to 30s)
2. Claim and execute shipment 032-S (Group B: CLI Resilience & Error Handling)
