# Tasks: Refactor Engram Server as Workspace-Local Plugin

**Input**: Design documents from `/specs/004-refactor-engram-server-as-plugin/`
**Prerequisites**: plan.md (required), spec.md (required), SCENARIOS.md, data-model.md, contracts/

**Tests**: Included per constitution Principle III (Test-First Development, NON-NEGOTIABLE). Test tasks reference SCENARIOS.md as the authoritative source.

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

### Phase Mapping

| Tasks Phase | Plan Phase | Description |
|-------------|------------|-------------|
| Phase 1 | Phase 1 | Setup (dependencies, stubs) |
| Phase 1.5 | — | Prerequisites (test migration, test harness) |
| Phase 2 | Phase 1 | Foundational (IPC transport, lockfile) |
| Phase 3 | Phase 2 | US1+US2: Zero-Config + Isolation (MVP) |
| Phase 4 | Phase 3 | US4: File Watching |
| Phase 5 | Phase 4 | US3: Lifecycle Management |
| Phase 6 | Phase 5 | US5: Plugin Installer |
| Phase 7 | Phase 6 | US6: Configuration |
| Phase 8 | Phase 6 | Polish & Cross-Cutting |

### Terminology Mapping

| Spec Term | Implementation Term |
|-----------|--------------------|
| Memory Service | Daemon (`src/daemon/`) |
| Client Interface | Shim (`src/shim/`) |
| Communication Channel | IPC (Unix Domain Socket / Windows Named Pipe) |
| Workspace Plugin | `.engram/` directory + daemon + shim |
| Runtime Artifacts | `run/` dir (PID lock, socket), `logs/` dir |
| Configuration | `.engram/config.toml` → `PluginConfig` struct |

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single project layout: `src/`, `tests/` at repository root
- New modules: `src/shim/`, `src/daemon/`, `src/installer/`
- Tests: `tests/contract/`, `tests/integration/`, `tests/unit/`

---

## Phase 1: Setup

**Purpose**: Add dependencies, restructure binary entrypoint, create module stubs

- [X] T001 Add new dependencies to Cargo.toml: `interprocess = { version = "2", features = ["tokio"] }`, `fd-lock = "4"`, `rmcp = { version = "1.1", features = ["server", "transport-io"] }`, `notify = "9"`, `notify-debouncer-full = "0.7"`
- [X] T002 Remove unused `mcp-sdk = "0.0.3"` dependency from Cargo.toml (replaced by rmcp)
- [X] T003 Restructure src/bin/engram.rs with clap subcommands: `shim` (default), `daemon`, `install`, `update`, `reinstall`, `uninstall` per plan.md architecture
- [X] T004 [P] Create module stubs: src/shim/mod.rs (re-exports), src/shim/transport.rs, src/shim/ipc_client.rs, src/shim/lifecycle.rs
- [X] T005 [P] Create module stubs: src/daemon/mod.rs (re-exports), src/daemon/ipc_server.rs, src/daemon/watcher.rs, src/daemon/debounce.rs, src/daemon/ttl.rs, src/daemon/lockfile.rs
- [X] T006 [P] Create module stubs: src/installer/mod.rs, src/installer/templates.rs
- [X] T007 Add new error variants to src/errors/mod.rs: `IpcConnection`, `DaemonSpawn`, `LockAcquisition`, `WatcherInit`, `ConfigParse`, `InstallError` per data-model.md
- [X] T008 [P] Add error code constants to src/errors/codes.rs: 8xxx range (IPC/daemon) and 9xxx range (installer) per data-model.md
- [X] T009 Verify `cargo check` passes with new dependencies and module stubs

**Checkpoint**: Project compiles with all new dependencies and module structure.

---

## Phase 1.5: Prerequisites

**Purpose**: Constitution amendments and test infrastructure required before feature implementation.

- [X] T088 [US1] Update existing contract and integration test files in tests/ to remove mcp-sdk imports — test setup may reference mcp-sdk types removed in T002; adapt to direct JSON-RPC construction or rmcp types
- [X] T089 [US1] Build process-based test harness for spawning daemon processes in tests/ — helper that starts `engram daemon`, waits for IPC ready, provides cleanup on drop; required by T020-T025 (shim lifecycle tests)

---

## Phase 2: Foundational (IPC Transport & Lockfile)

**Purpose**: Core IPC transport and process locking that MUST be complete before any user story.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests (write first, verify they fail)

- [X] T010 [P] Contract test for IPC JSON-RPC request/response format in tests/contract/ipc_protocol_test.rs — validate request serialization (S014-S016), missing fields (S017-S020), parse errors (S025) per SCENARIOS.md
- [X] T011 [P] Unit test for lockfile acquire/release/stale detection in tests/unit/lockfile_test.rs — validate lock acquisition (S027), stale lock detection (S029), cleanup on release (S032), read-only directory error (S030) per SCENARIOS.md
- [X] T012 [P] Unit test for IpcRequest/IpcResponse/IpcError serialization round-trips in tests/unit/proptest_models.rs — extend existing proptest coverage to new IPC models

### Implementation

- [X] T013 [P] Implement IpcRequest, IpcResponse, IpcError structs with serde in src/daemon/protocol.rs per data-model.md — include JSON-RPC 2.0 validation (jsonrpc field, id echoing). Note: IPC types are transport-layer, not domain models.
- [X] T014 [P] Implement DaemonState and DaemonStatus enum in src/daemon/mod.rs per data-model.md — Starting, Ready, ShuttingDown variants with serde(rename_all = "snake_case")
- [X] T015 Implement daemon lockfile management in src/daemon/lockfile.rs — fd-lock PID file acquire, release, stale detection (process liveness check), cleanup; covers S027-S033
- [X] T016 Implement daemon IPC server (listener + accept loop) in src/daemon/ipc_server.rs — interprocess LocalSocketListener, newline-delimited JSON-RPC framing, dispatch to tools::dispatch(); covers S014-S026
- [X] T017 Implement IPC endpoint naming in src/daemon/ipc_server.rs — Unix: `.engram/run/engram.sock`, Windows: `\\.\pipe\engram-{hash_prefix_16}` per contracts/ipc-protocol.md
- [X] T018 Wire IPC server into daemon startup sequence in src/daemon/mod.rs — hydrate → bind IPC → transition to Ready; covers S034-S036 state transitions
- [X] T019 Verify `cargo test` passes for all Phase 2 tests

**Checkpoint**: Foundation ready — IPC transport functional, lockfile enforced. User story implementation can begin.

---

## Phase 3: User Story 1 + User Story 2 — Zero-Config Workspace Memory & Workspace Isolation (Priority: P1) 🎯 MVP

**Goal**: MCP clients invoke tools via stdio, shim auto-starts daemon, workspace isolation via per-workspace IPC channels. This is the core value proposition.

**Independent Test**: Install plugin in fresh workspace, invoke `set_workspace` from an MCP client via stdio, verify daemon auto-starts and returns valid response. Open two workspaces simultaneously and verify zero cross-contamination.

### Tests for US1 + US2 (write first, verify they fail)

- [X] T020 [P] [US1] Contract test for shim cold start in tests/contract/shim_lifecycle_test.rs — no daemon running, shim spawns daemon, forwards request, returns response (S001); covers S005 cold start <2s
- [X] T021 [P] [US1] Contract test for shim warm start in tests/contract/shim_lifecycle_test.rs — daemon already running, shim connects and forwards (S002)
- [X] T022 [P] [US1] Contract test for shim error forwarding in tests/contract/shim_lifecycle_test.rs — daemon returns error, shim forwards faithfully (S004, S008)
- [X] T023 [P] [US1] Integration test for shim malformed input in tests/integration/shim_error_test.rs — invalid JSON (S006), empty stdin (S007), daemon timeout (S009), daemon crash (S010)
- [X] T024 [P] [US2] Integration test for multi-workspace isolation in tests/integration/multi_workspace_test.rs — two workspaces with separate data, verify no leakage (S088-S089); covers S091 symlink resolution
- [X] T025 [P] [US2] Integration test for concurrent workspace scaling in tests/integration/multi_workspace_test.rs — 20 workspaces running concurrently (S090 boundary test)

### Implementation for US1 + US2

- [X] T026 [US1] Implement shim IPC client in src/shim/ipc_client.rs — connect to daemon via interprocess LocalSocketStream, send JSON-RPC request, read response with timeout; covers S003, S009-S010
- [X] T027 [US1] Implement shim lifecycle in src/shim/lifecycle.rs — daemon health check (_health IPC message), spawn via std::process::Command, exponential backoff wait for ready; covers S001, S012, S013
- [X] T028 [US1] Implement daemon spawn guard in src/shim/lifecycle.rs — acquire lock before spawn, detect existing daemon, connect to existing if running; covers S028-S029
- [X] T029 [US1] Implement rmcp StdioTransport + ServerHandler in src/shim/transport.rs — rmcp ServerHandler trait impl, call_tool forwards to IPC client, tools/list returns compiled-in registry; covers S003, S015-S016
- [X] T030 [US1] Wire shim subcommand in src/bin/engram.rs — default subcommand invokes shim transport, connects stdio to daemon via IPC
- [X] T031 [US1] Wire daemon subcommand in src/bin/engram.rs — starts daemon process with --workspace arg, binds IPC, enters ready state; covers S034-S036
- [X] T032 [US2] Implement workspace-scoped IPC addressing in src/daemon/ipc_server.rs — each workspace gets unique IPC endpoint via SHA-256 hash prefix; covers S089, S091
- [X] T033 [US1] Implement _health IPC handler in src/daemon/ipc_server.rs — returns status, uptime, workspace, active connections per contracts/ipc-protocol.md (S021)
- [X] T034 [US1] Verify `cargo test` passes for all Phase 3 tests

**Checkpoint**: Core MVP functional — MCP clients can invoke tools via stdio, daemon auto-starts, workspaces fully isolated. User Stories 1 & 2 independently testable.

---

## Phase 4: User Story 4 — Real-Time File System Awareness (Priority: P2)

**Goal**: Daemon continuously monitors workspace for file changes and triggers existing indexing pipelines with configurable debounce. File watcher is a thin event source per spec clarification.

**Independent Test**: Start daemon, create/modify/delete a file, verify change reflected in queries within 2 seconds.

### Tests for US4 (write first, verify they fail)

- [ ] T035 [P] [US4] Integration test for file change detection in tests/integration/file_watcher_test.rs — create, modify, delete events (S052-S054); verify WatcherEvent emission
- [ ] T036 [P] [US4] Integration test for debounce behavior in tests/integration/file_watcher_test.rs — rapid saves collapse to single event (S055); verify timing with 500ms default
- [ ] T037 [P] [US4] Integration test for exclusion patterns in tests/integration/file_watcher_test.rs — .engram/, .git/, node_modules/, target/ ignored (S056-S059); custom exclusions (S060)
- [ ] T038 [P] [US4] Integration test for edge cases in tests/integration/file_watcher_test.rs — file rename (S062), large batch creates (S063), symlinks (S065), binary files (S066)

### Implementation for US4

- [ ] T039 [P] [US4] Implement WatcherEvent and WatchEventKind models in src/models/ per data-model.md — Created, Modified, Deleted, Renamed variants
- [ ] T040 [US4] Implement file watcher setup in src/daemon/watcher.rs — notify v9 RecommendedWatcher with exclusion pattern filtering; covers S052-S059, S064
- [ ] T041 [US4] Implement debouncer integration in src/daemon/debounce.rs — notify-debouncer-full with configurable duration (default 500ms); covers S055, S063
- [ ] T042 [US4] Wire debounced events to existing pipelines in src/daemon/debounce.rs — emit WatcherEvent, trigger code_graph and embedding services (thin event source per clarification); covers S052-S054, S062
- [ ] T043 [US4] Handle watcher initialization failure gracefully in src/daemon/watcher.rs — log WatcherInit error, daemon continues without file watching (S064 degraded mode)
- [ ] T044 [US4] Verify `cargo test` passes for all Phase 4 tests

**Checkpoint**: File watching operational — changes reflected in queries within 2 seconds. User Story 4 independently testable.

---

## Phase 5: User Story 3 — Automatic Lifecycle Management (Priority: P2)

**Goal**: Daemon self-manages its lifecycle with idle timeout, graceful shutdown, and crash recovery. Zero resource waste from idle workspaces.

**Independent Test**: Start daemon, wait for idle timeout, verify clean shutdown. Restart and verify data intact. Kill daemon, restart and verify recovery.

### Tests for US3 (write first, verify they fail)

- [ ] T045 [P] [US3] Unit test for TTL timer in tests/unit/ttl_test.rs — expiry triggers shutdown (S045), activity resets timer (S046-S047), zero timeout = run forever (S049), rapid activity (S051)
- [ ] T046 [P] [US3] Integration test for daemon lifecycle in tests/integration/daemon_lifecycle_test.rs — graceful shutdown flushes state (S037), shutdown during request (S038), restart after timeout (S050)
- [ ] T047 [P] [US3] Integration test for crash recovery in tests/integration/daemon_lifecycle_test.rs — SIGKILL recovery (S039-S040), stale lock detection, data rehydration; covers S095-S096

### Implementation for US3

- [ ] T048 [US3] Implement idle TTL timer in src/daemon/ttl.rs — activity timestamp tracking, periodic expiry check (S045), configurable duration; covers S048-S049
- [ ] T049 [US3] Wire TTL reset into IPC request handler in src/daemon/ipc_server.rs — every tool call resets idle timer (S046)
- [ ] T050 [US3] Wire TTL reset into file watcher event handler in src/daemon/watcher.rs — every file event resets idle timer (S047)
- [ ] T051 [US3] Implement graceful shutdown sequence in src/daemon/mod.rs — transition to ShuttingDown, flush state, close IPC listener, remove lock file, remove socket, exit; covers S037
- [ ] T052 [US3] Implement _shutdown IPC handler in src/daemon/ipc_server.rs — trigger graceful shutdown from shim command per contracts/ipc-protocol.md (S022)
- [ ] T053 [US3] Implement crash recovery in src/daemon/lockfile.rs — detect stale lock (fd-lock not held), clean stale socket/pipe, allow fresh daemon start; covers S039-S040, S042
- [ ] T054 [US3] Handle SIGTERM/SIGINT via tokio signal handler in src/daemon/mod.rs — trigger graceful shutdown on signal; covers S038
- [ ] T055 [US3] Verify `cargo test` passes for all Phase 5 tests

**Checkpoint**: Lifecycle management complete — daemon auto-shuts down, recovers from crashes, zero resource waste. User Story 3 independently testable.

---

## Phase 6: User Story 5 — Plugin Installation & Management (Priority: P3)

**Goal**: Simple commands to install, update, reinstall, and uninstall the engram plugin in any workspace. Painless setup and corruption recovery.

**Independent Test**: Run `engram install` in clean workspace, verify `.engram/` created and MCP config generated. Run tool call. Uninstall and verify cleanup.

### Tests for US5 (write first, verify they fail)

- [ ] T056 [P] [US5] Integration test for install command in tests/integration/installer_test.rs — clean workspace (S067), existing installation (S068), path with spaces (S076), Unicode path (S077), read-only FS (S078)
- [ ] T057 [P] [US5] Integration test for update/reinstall/uninstall in tests/integration/installer_test.rs — update preserves data (S069), reinstall after corruption (S070), uninstall with keep-data (S071), full removal (S072)
- [ ] T058 [P] [US5] Integration test for installer with running daemon in tests/integration/installer_test.rs — install while running (S073), uninstall stops daemon first (S074)

### Implementation for US5

- [ ] T059 [US5] Implement install command in src/installer/mod.rs — create `.engram/` structure (tasks.md, .version, config stub, run/, logs/), generate MCP config, health check verification; covers S067, S075
- [ ] T060 [P] [US5] Implement MCP config templates in src/installer/templates.rs — `.vscode/mcp.json` template with correct command path, `.gitignore` entries for runtime artifacts
- [ ] T061 [US5] Implement update command in src/installer/mod.rs — replace runtime artifacts, preserve data files (tasks.md, graph.surql, config.toml); covers S069
- [ ] T062 [US5] Implement reinstall command in src/installer/mod.rs — clean runtime, re-create structure, rehydrate from `.engram/` files; covers S070
- [ ] T063 [US5] Implement uninstall command in src/installer/mod.rs — stop running daemon (_shutdown), remove artifacts, `--keep-data` flag for data preservation; covers S071-S074
- [ ] T064 [US5] Detect existing installation in src/installer/mod.rs — check for `.engram/` directory, running daemon; covers S068, S073
- [ ] T065 [US5] Wire installer subcommands in src/bin/engram.rs — install, update, reinstall, uninstall subcommands invoke installer module
- [ ] T066 [US5] Verify `cargo test` passes for all Phase 6 tests

**Checkpoint**: Plugin installer complete — single-command setup and management. User Story 5 independently testable.

---

## Phase 7: User Story 6 — Configurable Behavior (Priority: P3)

**Goal**: Configuration file in `.engram/config.toml` customizes daemon behavior. Sensible defaults when absent.

**Independent Test**: Create config with custom idle timeout, start daemon, verify custom setting applied.

### Tests for US6 (write first, verify they fail)

- [ ] T067 [P] [US6] Unit test for PluginConfig parsing in tests/unit/plugin_config_test.rs — no config file defaults (S079), valid config (S080-S081), unknown fields ignored (S082), malformed TOML fallback (S083), negative values (S084), boundary values (S085, S087)
- [ ] T068 [P] [US6] Integration test for config-driven behavior in tests/integration/config_test.rs — custom exclusion patterns (S060-S061), custom timeout (S048), runtime config change no-op (S086)

### Implementation for US6

- [ ] T069 [US6] Implement PluginConfig struct with TOML parsing in src/models/config.rs — all fields per data-model.md with Default impl for sensible fallbacks; covers S079-S081
- [ ] T070 [US6] Implement config validation in src/models/config.rs — reject negative values, warn on unknown fields, clamp extreme values; covers S082-S085
- [ ] T071 [US6] Implement config file loading in src/daemon/mod.rs — read `.engram/config.toml`, fall back to defaults on missing or invalid; covers S083
- [ ] T072 [US6] Wire config into daemon subsystems — pass idle_timeout to TTL timer, debounce_ms to watcher, exclusion/watch patterns to watcher; covers S048, S060-S061, S087
- [ ] T073 [US6] Verify `cargo test` passes for all Phase 7 tests

**Checkpoint**: Configuration complete — daemon adapts to project-specific settings. User Story 6 independently testable.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Security hardening, performance validation, cross-platform resilience, documentation

### Tests

- [ ] T074 [P] Integration test for security scenarios in tests/integration/security_test.rs — Unix socket permissions (S097), path traversal rejection (S099), IPC message injection (S101), no secrets in `.engram/` (S102)
- [ ] T075 [P] Integration test for error recovery in tests/integration/recovery_test.rs — disk full during flush (S093-S094), corrupted tasks.md recovery (S095)

### Implementation

- [ ] T076 [P] Implement cross-platform path handling in src/daemon/mod.rs and src/installer/mod.rs — spaces, Unicode, symlink resolution; covers S031, S076-S077, S091, FR-018
- [ ] T077 [P] Implement IPC artifact permissions in src/daemon/ipc_server.rs — Unix `0o600` socket permissions (S097), Windows default ACL (S098); covers FR-016
- [ ] T078 [P] Implement structured logging to `.engram/logs/` in src/daemon/mod.rs — tracing-subscriber file appender, structured spans for all significant operations; covers S044, S103, FR-014
- [ ] T079 Implement atomic flush failure handling in src/services/dehydration.rs — detect disk-full during temp file write, preserve existing `.engram/` files; covers S093-S094
- [ ] T080 Implement workspace-moved detection in src/daemon/mod.rs — periodic check that workspace path still valid, shutdown if moved; covers S092
- [ ] T081 [P] Implement IPC request size validation in src/daemon/ipc_server.rs — reject oversized requests to prevent memory exhaustion; covers S024, S101
- [ ] T082 Performance validation: cold start under 2s benchmark in tests/integration/ — covers SC-003, S005
- [ ] T083 Performance validation: read latency <50ms, write latency <10ms benchmark in tests/integration/ — covers SC-002
- [ ] T084 [P] Large workspace test: 100k+ files background indexing does not block tool calls — covers FR-019, S063
- [ ] T085 [P] Documentation updates: update README.md with new architecture, update docs/ with ADR for rmcp migration
- [ ] T086 Run specs/004-refactor-engram-server-as-plugin/quickstart.md validation end-to-end
- [ ] T087 Final `cargo clippy -- -D warnings` and `cargo test` full pass
- [ ] T090 Verify all module stubs from Phase 1 (T004-T006) are replaced with real implementations — no placeholder code remaining per constitution "No dead code" rule
- [ ] T091 Decide on server/ module (HTTP/SSE): either remove entirely or feature-gate behind `legacy-sse` flag — document decision in ADR
- [ ] T092 [US4] Implement WatcherEvent→service adapter in src/daemon/debounce.rs — bridge WatcherEvent to existing code_graph and embedding service interfaces (existing services don't accept WatcherEvent directly)
- [ ] T093 [US2] Implement Unix socket path overflow fallback in src/daemon/ipc_server.rs — detect path >108 bytes, fall back to /tmp/engram-{hash}.sock with 0o600 permissions; covers S119

**Checkpoint**: All user stories complete, hardened, and validated. Feature ready for review.

---

## Dependencies & Execution Order

### Phase Dependencies

```text
Phase 1: Setup ──────────────────────> Phase 2: Foundational (IPC + Lock)
                                              │
                                              ├──> Phase 3: US1+US2 (Zero-Config + Isolation) 🎯 MVP
                                              │          │
                                              │          ├──> Phase 4: US4 (File Watching)
                                              │          │          │
                                              │          │          └──> Phase 5: US3 (Lifecycle)
                                              │          │
                                              │          └──> Phase 6: US5 (Installer)
                                              │                     │
                                              │                     └──> Phase 7: US6 (Config)
                                              │
                                              └──> Phase 8: Polish (after all desired stories)
```

### User Story Dependencies

- **US1 + US2 (P1)**: Depends on Phase 2 (Foundational). No dependencies on other stories. **This is the MVP.**
- **US4 (P2)**: Depends on Phase 3 (US1+US2) for daemon infrastructure. Independent of US3, US5, US6.
- **US3 (P2)**: Depends on Phase 3 (US1+US2) for daemon infrastructure. Benefits from US4 (watcher events reset TTL) but can be tested independently.
- **US5 (P3)**: Depends on Phase 3 (US1+US2) for daemon lifecycle to test `install` health check. Independent of US3, US4, US6.
- **US6 (P3)**: Depends on Phase 5 (US3) for TTL config wiring and Phase 4 (US4) for watcher config wiring. Can implement config parsing independently.

### Within Each Phase

- Tests MUST be written and FAIL before implementation (constitution Principle III)
- Models before services
- Services before handlers
- Core implementation before integration wiring
- Phase complete before moving to next

### Parallel Opportunities

- Phase 1: T004, T005, T006 (module stubs) can run in parallel; T007, T008 (error variants) can run in parallel
- Phase 2: T010, T011, T012 (tests) can run in parallel; T013, T014 (models) can run in parallel
- Phase 3: T020-T025 (tests) can run in parallel
- Phase 4: T035-T038 (tests) can run in parallel; T039 (model) parallel with test writing
- Phase 5: T045-T047 (tests) can run in parallel
- Phase 6: T056-T058 (tests) can run in parallel; T060 (templates) parallel with T059 (install)
- Phase 7: T067-T068 (tests) can run in parallel
- Phase 8: T074-T075 (tests) parallel; T076, T077, T078, T081, T085 all target different files

---

## Parallel Example: Phase 3 (MVP)

```bash
# Launch all tests for US1+US2 together:
Task T020: "Contract test for shim cold start in tests/contract/shim_lifecycle_test.rs"
Task T021: "Contract test for shim warm start in tests/contract/shim_lifecycle_test.rs"
Task T022: "Contract test for shim error forwarding in tests/contract/shim_lifecycle_test.rs"
Task T023: "Integration test for shim malformed input in tests/integration/shim_error_test.rs"
Task T024: "Integration test for multi-workspace isolation in tests/integration/multi_workspace_test.rs"
Task T025: "Integration test for concurrent workspace scaling in tests/integration/multi_workspace_test.rs"

# Then implement sequentially:
Task T026: IPC client (core transport)
Task T027: Shim lifecycle (spawn + health)
Task T028: Daemon spawn guard (lock-based)
Task T029: rmcp StdioTransport (MCP protocol)
Task T030-T031: Wire subcommands
Task T032-T033: Workspace addressing + health handler
```

---

## Implementation Strategy

### MVP First (Phase 1-3: US1 + US2 Only)

1. Complete Phase 1: Setup (dependencies, module stubs)
2. Complete Phase 2: Foundational (IPC transport, lockfile)
3. Complete Phase 3: US1 + US2 (shim, daemon, MCP stdio)
4. **STOP and VALIDATE**: MCP client can invoke tools via stdio, workspaces isolated
5. This is the minimum viable product — AI assistants gain workspace memory

### Incremental Delivery

1. Phase 1-3 → MVP: Zero-config workspace memory with isolation
2. + Phase 4 → Add: Real-time file watching
3. + Phase 5 → Add: Idle timeout, lifecycle management, crash recovery
4. + Phase 6 → Add: One-command install/update/uninstall
5. + Phase 7 → Add: Configurable behavior via TOML
6. + Phase 8 → Polish: Security, performance, documentation
7. Each phase adds value without breaking previous phases

### Task Summary

| Phase | Story | Tasks | Test Tasks | Impl Tasks |
|-------|-------|-------|------------|------------|
| 1 | Setup | 9 | 0 | 9 |
| 1.5 | Prerequisites | 2 | 0 | 2 |
| 2 | Foundational | 10 | 3 | 7 |
| 3 | US1+US2 (P1) | 15 | 6 | 9 |
| 4 | US4 (P2) | 10 | 4 | 6 |
| 5 | US3 (P2) | 11 | 3 | 8 |
| 6 | US5 (P3) | 11 | 3 | 8 |
| 7 | US6 (P3) | 7 | 2 | 5 |
| 8 | Polish | 18 | 2 | 16 |
| **Total** | | **93** | **23** | **70** |

---

## SCENARIOS.md Coverage Map

Every scenario in SCENARIOS.md is covered by at least one task:

| SCENARIOS.md Section | Scenario IDs | Primary Tasks |
|---|---|---|
| Shim Lifecycle | S001-S013 | T020-T034 |
| IPC Protocol | S014-S026 | T010, T013, T016-T017 |
| Daemon Lockfile | S027-S033 | T011, T015, T028, T053 |
| Daemon Lifecycle | S034-S044 | T018, T031, T046-T047, T051, T054, T078 |
| TTL Management | S045-S051 | T045, T048-T050 |
| File Watcher | S052-S066 | T035-T043 |
| Plugin Installer | S067-S078 | T056-T066 |
| Configuration | S079-S087 | T067-T072 |
| Workspace Isolation | S088-S092 | T024-T025, T032, T076, T080 |
| Error Recovery | S093-S096 | T047, T075, T079 |
| Security | S097-S103 | T074, T077-T078, T081 |
| MCP Tool Compatibility | S104-S108 | T020-T022, T029, T033 |

---

## Notes

- [P] tasks target different files with no dependencies — safe to parallelize
- [Story] labels map tasks to user stories for traceability
- Constitution Principle III (TDD): test tasks precede implementation in every phase
- Constitution Principle I (Safety): no `unsafe`, no `unwrap()`, Result/EngramError throughout
- Constitution Principle VI (Single Binary): all subcommands in single `engram` binary
- Total: 93 tasks, 23 test tasks, 70 implementation tasks
- Commit after each task or logical group per constitution commit discipline

