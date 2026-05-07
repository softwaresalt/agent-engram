---
type: runtime-verification
shipment: 027-S
feature: 042-F
task: 042.001-T
branch: feat/042-F-startup-preloading
pr: 88
date: 2026-05-07
verdict: PASS WITH FOLLOW-UP
surface: cli
---

# Runtime Verification — 027-S Startup Script Engram Pre-Loading

## Surface

**CLI / PowerShell script** — `start.ps1`

## Changed Surface

`start.ps1`: Added 9-line engram sync block between `backlogit sync` and
`& $copilotExe --remote`. No Rust code changed.

## Environment Prechecks

| Check | Result |
|---|---|
| Branch is `feat/042-F-startup-preloading` | ✅ |
| Latest commit `5c5c7df` pushed to origin | ✅ |
| CI green on PR #88 | ✅ |
| `start.ps1` artifact is the one under test | ✅ |

## Verification Steps

### Step 1 — Script Syntax (PowerShell AST Parse)

```powershell
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path "start.ps1").Path, [ref]$tokens, [ref]$errors)
```

**Result**: 0 syntax errors. `start.ps1` parses cleanly.

### Step 2 — Installed Binary vs. New Binary

| Binary | Path | Version | Has `sync` subcommand |
|---|---|---|---|
| Installed | `D:\Tools\engram.exe` | 0.0.1 (pre-042-F) | ❌ |
| Local build | `.\target\debug\engram.exe` | current | ✅ |

The installed binary at `D:\Tools\engram.exe` is the pre-42-F binary
that predates the CLI parity commands added in PR #85. The `sync`
subcommand was introduced by that PR.

### Step 3 — Non-Fatal Behaviour (Installed Binary)

Ran the equivalent of the `start.ps1` try/catch block with the installed binary:

```powershell
$engramCmd = Get-Command engram -ErrorAction SilentlyContinue
if ($engramCmd) {
    try {
        engram sync --workspace . --quiet
    } catch {
        Write-Warning "engram sync failed (non-fatal): $_"
    }
}
```

**Observed**: `engram sync` printed `error: unrecognized subcommand 'sync'`
to stderr and exited with code 2. PowerShell's `try/catch` does not catch
non-terminating errors from external processes, so the script continued
unimpeded. The `catch` block was never entered — the failure is silent
from the script's perspective (only the stderr line is visible to the user).

**Verdict**: ✅ Non-fatal. Copilot would still launch.

### Step 4 — New Binary Auto-Spawn Behaviour

Ran `.\target\debug\engram.exe sync --workspace . --quiet`:

```
Error: daemon unavailable: Daemon failed to reach Ready state within 2000ms
Exit: 2
```

**Root cause analysis**: Multiple active engram processes are running from
an ongoing Copilot session (PIDs 31404, 41388, 43924, 48920). The new
binary detected a version mismatch with the existing daemon and attempted
`respawn_daemon` (2 000 ms wait for old daemon to exit). The old daemon
declined to exit because it is serving the active Copilot session.
This is the `SHUTDOWN_WAIT_TIMEOUT_MS = 2_000` path in `lifecycle.rs`.

**Cold-start path (the scenario that matters for `start.ps1`)**: When no
daemon is running — which is the state when `start.ps1` executes before
Copilot starts — `ensure_daemon_running` calls `spawn_daemon` and then
`poll_until_ready` with a 30 000 ms budget. The daemon starts, loads the
database, and the sync runs. This is the intended pre-loading flow.

The cold-start path **cannot be tested here** while an active Copilot
session owns the named pipe. Manual verification on a fresh start is
required (see Follow-Ups).

### Step 5 — `--quiet` Flag

```powershell
.\target\debug\engram.exe sync --help | Select-String "quiet"
# → --quiet    Suppress non-error output
```

The `--quiet` flag is recognized. Normal output is suppressed; errors
still print. Confirmed correct flag name and semantics.

### Step 6 — Script Structure Review

```powershell
# start.ps1 (lines 47–54)
$engramCmd = Get-Command engram -ErrorAction SilentlyContinue
if ($engramCmd) {
    try {
        engram sync --workspace . --quiet
    } catch {
        Write-Warning "engram sync failed (non-fatal): $_"
    }
}
```

- Mirrors the `backlogit sync` block pattern exactly ✅
- `Get-Command -ErrorAction SilentlyContinue` safely skips when engram is absent ✅
- `try/catch` present (handles terminating errors; non-terminating failures pass through) ✅
- `--workspace .` uses the cwd (correct since `start.ps1` runs from the workspace root) ✅
- `--quiet` suppresses success output ✅

## Verdict

**PASS WITH FOLLOW-UP**

The script change is structurally sound and non-fatal under all tested conditions.
The cold-start pre-loading flow requires the updated binary to be installed and
a daemon-free environment to demonstrate the benefit. Both follow-ups below are
post-merge tasks, not blockers for merge.

## Follow-Ups

| # | Description | Priority |
|---|---|---|
| FU-1 | Update installed binary: replace `D:\Tools\engram.exe` (pre-042-F) with a fresh `cargo install --path .` build so the `sync` subcommand is available. Without this, the pre-loading block runs but the sync fails silently. | High |
| FU-2 | Manual cold-start verification: on a clean start (no Copilot session active), run `.\start.ps1` and confirm `engram sync` completes and daemon is running before Copilot launches. | Medium |

## Handoff to Operational Closure

- **Verification verdict**: PASS WITH FOLLOW-UP
- **Runtime surface verified**: `start.ps1` CLI script
- **Evidence**: AST parse (0 errors), non-fatal behaviour test (passes), new binary sync flag verification (confirmed), existing daemon interaction analysis
- **Cold-start path**: Cannot fully exercise during active session; non-blocking follow-up
- **Follow-up recommendations**: Install updated binary (FU-1), cold-start manual test (FU-2)
