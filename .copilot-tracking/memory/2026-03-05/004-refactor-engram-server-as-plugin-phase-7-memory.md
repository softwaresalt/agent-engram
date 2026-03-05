# Phase 7 Memory: US6 — Configuration (T067–T073)

**Spec**: `004-refactor-engram-server-as-plugin`
**Phase**: 7
**Date**: 2026-03-05
**Commit**: e50a895
**Status**: COMPLETE — All 7 tasks done, all tests pass

---

## Files Modified

| File | Change |
|------|--------|
| `src/models/config.rs` | ADDED PluginConfig struct with Default, idle_timeout(), load() |
| `src/models/mod.rs` | ADDED pub use config::PluginConfig |
| `src/daemon/mod.rs` | INSERTED PluginConfig::load() + WatcherConfig from plugin config |
| `tests/unit/plugin_config_test.rs` | NEW: 10 tests S079–S087 |
| `tests/integration/config_test.rs` | NEW: 5 tests S048, S060-S061, S086 |
| `Cargo.toml` | ADDED 2 [[test]] entries |
| `specs/.../tasks.md` | MARKED T067–T073 as [X] |

---

## Design Decisions

- `PluginConfig` is additive — `WorkspaceConfig` and other types in config.rs preserved unchanged
- No `deny_unknown_fields` — unknown TOML keys silently ignored (satisfies S082)
- `idle_timeout_minutes: u64` — negative values are TOML parse error → fallback to defaults (S084)
- Env var `ENGRAM_IDLE_TIMEOUT_MS` still wins over config.toml for test harness compatibility

## Defaults

- `idle_timeout_minutes`: 240 (4 hours)
- `debounce_ms`: 500
- `watch_patterns`: `["**/*"]`
- `exclude_patterns`: `[".engram/", ".git/", "node_modules/", "target/", ".env*"]`
- `log_level`: `"info"`
- `log_format`: `"pretty"`

---

## Next Steps (Phase 8)

**Phase 8: US7 — Polish & Cross-Cutting (T074–T087)**

Tasks span security, error recovery, cross-platform, and final integration:
- T074: Security integration tests (Unix socket perms S097, path traversal S099, IPC injection S101, no secrets S102)
- T075: Error recovery tests (disk full S093-S094, corrupted tasks.md S095)
- T076: Cross-platform path handling (spaces, Unicode, symlinks S031, S076-S077, S091)
- T077: IPC artifact permissions (Unix 0o600 socket, Windows ACL S097-S098)
- T078: Final integration tests passing
- T079-T087: Additional polish tasks per tasks.md

Check `specs/004-refactor-engram-server-as-plugin/tasks.md` for exact T078-T087 scope.
