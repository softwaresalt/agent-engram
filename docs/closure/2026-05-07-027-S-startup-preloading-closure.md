---
type: operational-closure
mode: pre-merge
shipment: 027-S
feature: 042-F
task: 042.001-T
branch: feat/042-F-startup-preloading
pr: 88
date: 2026-05-07
status: READY WITH CONDITIONS
runtime_verification: docs/closure/2026-05-07-027-S-startup-preloading-runtime-verification.md
source_stash_id: B59D87CA
source_deliberation: docs/decisions/2026-05-07-startup-preloading-deliberation.md
---

# Operational Closure — 027-S Startup Script Engram Pre-Loading

## Change Summary

Added `engram sync --workspace . --quiet` to `start.ps1` between the existing
`backlogit sync` block and the `& $copilotExe --remote` launch line. The block
is guarded by `Get-Command engram -ErrorAction SilentlyContinue` and wrapped in
`try/catch`, making the sync step non-fatal. If engram is absent or the sync
fails, Copilot still launches normally.

The `--remote` flag on the Copilot launch line was also preserved per user request
(replacing the prior `@args` pass-through).

**Files changed**: `start.ps1` (+9 lines)  
**Rust code changed**: None  
**CI**: Green on PR #88  

## Invariants to Preserve

1. `start.ps1` must always launch Copilot even if engram is absent or fails.
2. `start.ps1` must not block indefinitely on any optional pre-flight step.
3. The `--remote` flag on the Copilot launch line must remain.
4. The `backlogit sync` block must remain unchanged.

All four invariants are satisfied by the current implementation.

## Pre-Deploy Audit

| Check | Result |
|---|---|
| No Rust changes — no database migration risk | ✅ |
| `start.ps1` syntax: AST parse 0 errors | ✅ |
| Non-fatal guard: `Get-Command + try/catch` present | ✅ |
| `--quiet` suppresses success noise | ✅ |
| `--remote` preserved on `& $copilotExe` | ✅ |
| CI green on PR #88 | ✅ |
| No secrets, credentials, or hardcoded paths | ✅ |

**Condition before merge**: None blocking. The installed binary condition (FU-1)
is post-merge.

## Deployment / Rollout Path

Merge-only. No service deployment. `start.ps1` is a local developer script
that users run manually. Changes take effect the next time a developer runs
`.\start.ps1`.

## Post-Deploy Checks (after merge)

1. Update installed binary: `cargo install --path .` → confirms new binary is
   at the PATH location (`D:\Tools\engram.exe` or wherever `Get-Command engram`
   resolves).
2. On a fresh terminal (no active Copilot session / no daemon running), run
   `.\start.ps1` and confirm:
   - `engram sync` output is suppressed (`--quiet` working)
   - Copilot launches as expected
   - No unexpected error output

## Runtime Verification Summary

**Verdict**: PASS WITH FOLLOW-UP  
**Evidence**: Script parses cleanly; non-fatal behavior confirmed with old binary;
auto-spawn daemon design confirmed via source code; `--quiet` flag semantics confirmed.  
**Full report**: `docs/closure/2026-05-07-027-S-startup-preloading-runtime-verification.md`

## Healthy Signals

- `start.ps1` completes and Copilot opens without error or delay
- First MCP tool call responds within normal latency (no cold-start timeout)
- No `Write-Warning "engram sync failed"` in terminal output (once FU-1 is done)

## Failure Signals

- `start.ps1` hangs indefinitely (would indicate the `try/catch` is not protecting
  against a blocking call — considered impossible given the non-terminating error
  model, but monitor on first use)
- Copilot does not launch after engram sync step

## Monitoring Plan

This is a developer startup script, not a production service. No dashboards or
alerts are applicable. Monitoring = developer observes terminal output on next use.

Manual observation window: next use after merge + binary update (FU-1).

## Rollback Trigger and Procedure

**Trigger**: `start.ps1` hangs or Copilot fails to launch after the change.

**Rollback**:
```bash
git revert --no-edit -m 1 <merge_sha>
```
Or, more surgically, remove the 9-line engram sync block from `start.ps1`
(lines 47–54) and commit.

The change is trivially reversible — it is a 9-line additive block with no
dependencies.

## Validation Window

One development session after binary update (FU-1 complete). Single developer
validates during their next `start.ps1` invocation.

**Owner**: Developer who merges and runs the script.

## Follow-Up Tasks

| ID | Description | Priority | Target |
|---|---|---|---|
| FU-1 | Install updated engram binary (`cargo install --path .`) to make `sync` subcommand available | High | Post-merge |
| FU-2 | Manual cold-start test on a fresh terminal after FU-1 | Medium | Post-merge |

## Source Artifact Traceability

- **Source stash**: `B59D87CA` — archived via `backlogit stash harvest` during
  Stage session. Entry recorded in `.backlogit/archive/stash.jsonl`.
- **Deliberation**: `docs/decisions/2026-05-07-startup-preloading-deliberation.md`
  — lightweight deliberation, no open follow-up items.
- **Plan**: `docs/exec-plans/2026-05-07-startup-preloading-plan.md` — plan-review
  PASS, no P0/P1 findings.
- `custom_fields.source_stash_id` not set on 042-F (known backlogit limitation
  for Stage-harvested items; traceability preserved here).

## Readiness Status

**READY WITH CONDITIONS**

Conditions:
1. PR #88 receives merge approval (awaiting user)
2. FU-1 (binary update) performed by developer before first use

Merge itself is non-blocking. The engram sync block is safe whether or not
the updated binary is installed — it fails non-fatally in all cases.
