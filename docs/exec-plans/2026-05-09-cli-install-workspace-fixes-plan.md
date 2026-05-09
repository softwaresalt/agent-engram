---
title: "CLI Install & Workspace Flag Fixes"
type: impl-plan
date: 2026-05-09
status: draft
source_stash:
  - BC9A6B23
  - B9E4F2A1
---

## Problem Frame

Two related defects affect the `engram install` family of CLI commands:

1. **`--workspace` flag ignored** (`BC9A6B23`, high): The `Install`, `Update`,
   `Reinstall`, and `Uninstall` command branches in `src/bin/engram.rs` (lines
   257, 266, 270, 274) hard-code `std::env::current_dir()` instead of using
   `flags.resolve_workspace()`. Every other CLI command resolves workspace
   correctly. Running `engram install --workspace /path/to/ws` silently
   installs into the CWD, not the specified path.

2. **Missing `.backlogit/` auto-detection** (`B9E4F2A1`, medium): The
   `AUTO_DETECT_DIRS` constant in `src/installer/mod.rs` (line 137) lists
   `.backlog` but not `.backlogit`. Workspaces using backlogit must manually
   add a `backlog` source entry to `registry.yaml` after install.

Both bugs share the same code surface (`src/bin/engram.rs` dispatch +
`src/installer/mod.rs` scaffold) and can be fixed together without scope creep.

## Requirements Trace

| Requirement | Implementation |
|---|---|
| `--workspace` flag respected by install | Replace `current_dir()` with `flags.resolve_workspace()` in 4 match arms |
| `--workspace` flag respected by update | Same pattern |
| `--workspace` flag respected by reinstall | Same pattern |
| `--workspace` flag respected by uninstall | Same pattern |
| `.backlogit/` detected during install | Add `(".backlogit", "backlog", Some("markdown"))` to `AUTO_DETECT_DIRS` |
| Existing tests remain green | Run `cargo test` — no existing tests exercise `--workspace` on install |
| New tests cover the fix | Add integration tests for workspace-flag resolution in installer |

## Implementation Units

### Unit 1: Fix `--workspace` flag in install commands

**Scope**: Replace `std::env::current_dir()` with `flags.resolve_workspace()`
in the four installer command arms in `src/bin/engram.rs`.

**Files affected**:

- `src/bin/engram.rs` — 4 match arms (Install, Update, Reinstall, Uninstall)

**Changes**:

```rust
// BEFORE (line 257, and similar at 266, 270, 274):
let workspace = std::env::current_dir()?;

// AFTER:
let workspace = flags
    .resolve_workspace()
    .map_err(|e| anyhow::anyhow!("{e}"))?;
```

Note: `resolve_workspace()` returns `Result<PathBuf, String>`, so the error
must be mapped to `anyhow::Error` to match the function's return type. The
existing `Daemon` arm (line 246) already uses `flags.workspace` directly, so
the pattern is established.

**Tests**: Unit 2 covers test verification.

**Execution posture**: Direct fix — the pattern exists in every other CLI command.

### Unit 2: Add integration tests for workspace-flag resolution

**Scope**: Add tests that verify the install/update/uninstall commands use the
workspace from `--workspace` rather than cwd.

**Files affected**:

- `tests/integration/installer_test.rs` — add new test functions

**Changes**:

Add tests that invoke `installer::install()`, `installer::update()`, and
`installer::uninstall()` with a workspace path different from the temp dir
used as cwd, verifying that `.engram/` is created in the specified workspace,
not in cwd.

These are library-level integration tests calling the public API directly
(same pattern as existing `s067_install_clean_workspace`). No binary
subprocess needed since the fix is in the dispatch layer, not the installer
library.

Also add a CLI-level test using the compiled binary to verify end-to-end:

```rust
// Verify `engram install --workspace <target>` creates .engram/ in <target>
let target = tempfile::tempdir().expect("target dir");
let cwd = tempfile::tempdir().expect("cwd dir");
let output = Command::new(env!("CARGO_BIN_EXE_engram"))
    .arg("install")
    .arg("--workspace")
    .arg(target.path())
    .current_dir(cwd.path())
    .env_remove("ENGRAM_DATA_DIR")
    .output()
    .expect("engram install");
assert!(target.path().join(".engram").is_dir());
assert!(!cwd.path().join(".engram").exists());
```

**Test scenarios** (≤4):

1. `install --workspace <target>` creates `.engram/` in target, not cwd
2. `update --workspace <target>` updates in target (pre-install target first)
3. `uninstall --workspace <target>` removes from target, not cwd

**Execution posture**: Test-first for the CLI binary test; characterization for
the library-level tests (verify current broken behavior, then fix).

### Unit 3: Add `.backlogit/` to auto-detect registry scaffold

**Scope**: Add `.backlogit` to the `AUTO_DETECT_DIRS` constant so that
`generate_default_registry()` includes a `backlog` source entry when the
directory exists.

**Files affected**:

- `src/installer/mod.rs` — add entry to `AUTO_DETECT_DIRS` constant

**Changes**:

```rust
// Add after the existing ".backlog" entry (line 146):
(".backlogit", "backlog", Some("markdown")),
```

Both `.backlog` and `.backlogit` entries should coexist — a workspace may use
either naming convention. The `generate_default_registry` function iterates
all entries and includes only those whose directories exist.

**Tests**:

- Add a test in `tests/integration/installer_test.rs` that creates a workspace
  with a `.backlogit/` directory, runs `install()`, and verifies the generated
  `registry.yaml` contains a `backlog` source entry with `path: .backlogit`.

**Execution posture**: Test-first — write the test, verify it fails (missing
`.backlogit` detection), add the entry, verify green.

## Dependency Graph

```text
Unit 1 (workspace flag fix)
  ↓
Unit 2 (tests for flag fix) — depends on Unit 1
  ↕ (no dependency)
Unit 3 (backlogit auto-detect) — independent of Units 1-2
```

Units 1 and 3 can be implemented in parallel. Unit 2 depends on Unit 1.
Recommended sequence: Unit 1 → Unit 2 → Unit 3 (serial for simplicity).

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Map `resolve_workspace()` error via `anyhow::anyhow!` | The `main()` function returns `anyhow::Result`, and `resolve_workspace()` returns `Result<PathBuf, String>`. Mapping with `anyhow::anyhow!` matches the existing error pattern in the daemon arm. |
| Keep both `.backlog` and `.backlogit` in auto-detect | Workspaces may use either convention. Auto-detection is additive (only triggers when the directory exists), so both entries are safe. |
| Library-level + binary-level tests | Library tests verify the installer functions directly (fast, no subprocess). The binary test verifies the dispatch layer actually passes the resolved workspace (end-to-end confidence). |
| No changes to `resolve_workspace()` itself | The function already works correctly (flag → env → cwd). The bug is that installer dispatch bypasses it entirely. |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| `resolve_workspace()` returns a relative path that confuses the installer | `resolve_workspace()` returns the value from `--workspace` as-is or `current_dir()` (always absolute). If `--workspace` is relative, the installer functions already handle `Path` correctly. Low risk. |
| Existing installer tests break from workspace path changes | Existing tests call `installer::install(workspace, ...)` directly with explicit paths — they never go through the dispatch layer. No impact. |
| `.backlogit` entry produces duplicate source if both `.backlog` and `.backlogit` exist | Both entries would appear in registry.yaml with different paths. The ingestion system handles duplicate source types. Low risk; edge case. |

## Plan Hardening Signals

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | No | Internal dispatch fix; no API change |
| Security, auth, permission, or compliance-sensitive | No | File paths only; workspace isolation already enforced by installer |
| Migration, backfill, destructive data/config, irreversible | No | Registry scaffold is generated fresh on install; no migration |
| External integration, operator checkpoint, external dependency | No | All local filesystem operations |
| High runtime, rollout, or rollback risk | No | Simple dispatch fix + additive constant entry |

**Requires plan hardening: no**

## Runtime Verification and Closure

### Unit 1 & 2: `--workspace` flag fix

- **Runtime surface**: CLI `engram install/update/reinstall/uninstall` commands
- **Verification**: Run `engram install --workspace <path>` manually and confirm
  `.engram/` is created at the specified path, not cwd
- **Closure**: No monitoring needed — deterministic CLI behavior

### Unit 3: `.backlogit/` auto-detect

- **Runtime surface**: Registry scaffold generated by `engram install`
- **Verification**: Run `engram install` in a workspace containing `.backlogit/`
  and verify `registry.yaml` includes `path: .backlogit`
- **Closure**: No monitoring needed — deterministic scaffold generation

## Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | ✅ No unsafe code; errors propagated via Result |
| II. Test-First Development | ✅ Unit 2 and Unit 3 include test-first steps |
| III. Workspace Isolation | ✅ Fix actually improves workspace isolation |
| IV. CLI Containment | ✅ No operations outside workspace root |
| VII. Destructive Approval | N/A — no destructive operations |
| XI. Merge Commit | ✅ Will use merge commit |

## Plan Review

**Gate decision: PASS**

**Reviewed**: 2026-05-09
**Personas**: Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor, Learnings Researcher
**Hardening required**: No — all five hardening signals absent, confirmed by all reviewers.

### Findings

#### P2-01: Library-level tests do not exercise the bug (Rust Reviewer)

Unit 2 proposes both library-level and binary-level tests. The library-level
tests call `installer::install(target_path, ...)` directly — this already
works correctly because the library functions accept `workspace: &Path` as a
parameter. The bug is solely in the dispatch layer (`src/bin/engram.rs`) which
hard-codes `current_dir()` instead of `flags.resolve_workspace()`.

**Recommendation**: Clarify in the implementation that library-level tests are
confirmatory (proving the library always accepted arbitrary paths), and the
binary-level tests (`Command::new(env!("CARGO_BIN_EXE_engram"))`) are the
actual regression tests that verify the dispatch fix. The binary tests are
the critical coverage; the library tests are supplementary.

**Action**: Advisory — does not block harvest. Ship agent should prioritize
binary-level tests as the primary verification.

#### P3-01: Consistent with prior learnings (Learnings Researcher)

The plan's use of `flags.resolve_workspace()` aligns with the compound
learning `rust-2024-set-var-unsafe-2026-05-07.md`, which documents that
workspace resolution should flow through function parameters rather than
environment mutation. The binary test's `.env_remove("ENGRAM_DATA_DIR")` also
aligns with `engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`.
No contradictions found.

### Summary

- 0 P0, 0 P1, 1 P2, 1 P3
- No plan hardening required
- Plan scope is tight (2 production files, 1 test file)
- All constitutional principles satisfied
- Ready for harvest
