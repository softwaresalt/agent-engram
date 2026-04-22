---
title: "006-S runtime verification"
date: 2026-04-22
shipment: "006-S"
feature: "029-F"
branch: "release/006-s-daemon-reliability-b1"
verdict: "PASS WITH FOLLOW-UP"
surface: "cli"
mode: "manual"
---

# 006-S runtime verification

## Scope

Verified the daemon runtime surface affected by Shipment B1:

* daemon startup on a fresh Git workspace
* stale `engram.pid` replacement with structured PID metadata
* `.workspace-id` persistence during daemon startup
* clean shutdown after the daemon is reachable

## Environment prechecks

* Verified the expected artifact exists: `target-redphase\debug\engram.exe`
* Used a temporary workspace with a minimal `.git\HEAD`
* Seeded stale runtime state by writing `99999999` to `.engram\run\engram.pid`
* Chose `manual` CLI verification because the changed surface is daemon/shim IPC behavior and no standalone operator-facing smoke command exists for the shim handshake path

## Command and scenario

Executed a PowerShell smoke script that:

* created a temporary Git workspace
* pre-seeded a stale PID file
* launched `target-redphase\debug\engram.exe daemon --workspace <temp-workspace>`
* waited for `.engram\run\engram.pid` and `.engram\.workspace-id`
* parsed the rewritten PID metadata JSON
* confirmed the running daemon PID matched the persisted PID
* stopped the daemon and confirmed process exit

## Expected behavior

* daemon starts successfully on a workspace with stale runtime state
* `engram.pid` is rewritten from legacy numeric contents to structured JSON
* persisted PID matches the live daemon process
* `.workspace-id` is present before shutdown
* daemon can be stopped cleanly

## Observed behavior

The smoke verification passed:

* daemon process started and remained alive until explicit stop
* `engram.pid` was rewritten as `{"pid":18580,"start_time_unix":1776840884}`
* persisted PID matched the launched daemon PID
* `.workspace-id` was created
* daemon exited cleanly after `Stop-Process -Id 18580`

## Evidence

```json
{"exe":"D:\\Source\\GitHub\\agent-engram\\target-redphase\\debug\\engram.exe","workspace":"C:\\Users\\derek.williams\\AppData\\Local\\Temp\\engram-006s-c4734443-2de7-49cb-bd3d-411bb096cbe4","process_id":18580,"pid_file_exists":true,"workspace_id_exists":true,"pid_file_raw":"{\"pid\":18580,\"start_time_unix\":1776840884}","workspace_uuid":"685c43ed-21f7-4c03-a997-1b4b37751d4f","pid_matches_process":true,"start_time_recorded":true,"daemon_alive_before_stop":true,"daemon_exited_after_stop":true}
```

## Verdict

**PASS WITH FOLLOW-UP**

The shipped binary behaved correctly for the daemon startup and runtime-state
recovery path. The version-mismatch respawn lane was still exercised through the
green integration and contract suite rather than a standalone manual shim smoke,
so keep one follow-up note that the daemon/shim handshake path has stronger
automated coverage than manual operator-facing smoke coverage today.

## Follow-up recommendations

* keep the review follow-up on Unix `/tmp` fallback socket creation permissions in closure
* consider adding an operator-facing smoke command for shim/daemon handshake verification in a later shipment
