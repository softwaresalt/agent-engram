# Phase 6 Memory: US5 — Plugin Installation & Management (T056–T066)

**Spec**: `004-refactor-engram-server-as-plugin`
**Phase**: 6
**Date**: 2026-03-05
**Commit**: cb91672
**Status**: COMPLETE — All 11 tasks done, 20 tests pass (2 ignored)

---

## Task Overview

Phase 6 implements the installer CLI commands: install, update, reinstall, uninstall.
These manage the `.engram/` directory structure and `.vscode/mcp.json` MCP config.

**User Story**: US5 (Plugin Installation & Management)
**Tasks**: T056–T066 (11 total)

---

## Files Modified

| File | Change |
|------|--------|
| `src/installer/templates.rs` | IMPLEMENTED: mcp_json(exe), gitignore_entries() |
| `src/installer/mod.rs` | IMPLEMENTED: install/update/reinstall/uninstall + helpers |
| `src/bin/engram.rs` | UPDATED: pass current_dir() as workspace arg to all installer fns |
| `tests/integration/installer_test.rs` | NEW: 22 tests covering S067-S078 |
| `Cargo.toml` | ADDED: [[test]] for integration_installer |
| `specs/.../tasks.md` | MARKED: T056–T066 as [X] |

---

## Implementation Notes

### Function Signatures
All installer functions now take `workspace: &Path`:
```rust
pub async fn install(workspace: &Path) -> Result<(), EngramError>
pub async fn update(workspace: &Path) -> Result<(), EngramError>
pub async fn reinstall(workspace: &Path) -> Result<(), EngramError>
pub async fn uninstall(workspace: &Path, keep_data: bool) -> Result<(), EngramError>
pub fn is_installed(workspace: &Path) -> bool
pub async fn is_daemon_running(workspace: &Path) -> bool
```

### .engram/ structure created by install:
- `.engram/tasks.md` — minimal frontmatter stub
- `.engram/.version` — "0.1.0"
- `.engram/config.toml` — commented stub
- `.engram/run/` — directory for runtime artifacts (socket/pipe, lockfile)
- `.engram/logs/` — directory for daemon logs
- `.vscode/mcp.json` — MCP stdio config referencing engram binary
- `.gitignore` entries appended if .gitignore exists

### MCP JSON format
```json
{"mcpServers":{"engram":{"type":"stdio","command":"<exe>","args":[]}}}
```
Backslash normalization applied for Windows paths.

### Daemon interaction (uninstall S074)
- `is_daemon_running()` uses `check_health()` via `ipc_endpoint(workspace)`
- Sends `_shutdown` IPC before uninstall if daemon is running
- Waits up to 2s for daemon to stop (polling check_health)

### Tests
- S073/S074 (daemon-running install/uninstall) are `#[ignore]` — require live daemon
- tempfile crate used for temp dirs in tests

---

## Next Steps (Phase 7)

**Phase 7: US6 — Configuration (T067–T073)**

Tasks:
- T067: Unit tests for PluginConfig parsing (tests/unit/plugin_config_test.rs)
- T068: Integration test for config-driven behavior (tests/integration/config_test.rs)
- T069: Implement PluginConfig struct in src/models/config.rs
- T070: Config validation (negative values, unknown fields, clamping)
- T071: Config file loading in src/daemon/mod.rs
- T072: Wire config into daemon subsystems (TTL timeout, watcher debounce, exclusions)
- T073: Verify cargo test passes

Key scenarios: S079-S087
Key constants: idle_timeout_minutes=240, debounce_ms=500, watch_patterns=["**/*"],
  exclude_patterns=[".engram/", ".git/", "node_modules/", "target/", ".env*"]
