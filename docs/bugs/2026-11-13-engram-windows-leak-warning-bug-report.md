---
title: "Engram Windows RADAR_PRE_LEAK_64 warning on November 12"
description: "Evidence and investigation handoff for a Windows leak warning attributed to engram.exe."
doc_type: report
date: 2026-11-13
---

## Summary

Windows Error Reporting recorded a `RADAR_PRE_LEAK_64` event naming
`engram.exe` at **2026-11-12T07:39:03.1272403Z**. Investigate potential
unbounded memory retention in Engram and determine which binary, process
role, and workspace generated the warning.

**Status:** suspected memory-retention defect; not reproduced.
**Priority:** requires triage in the Engram workspace.
**Confidence:** high that Windows emitted the warning; insufficient evidence
to establish an Engram allocation defect or its root cause.

This report is an evidence handoff from the backlogit workspace. No Engram
source changes, heap profiling, or controlled reproduction were performed.
All timestamps below are UTC.

## Expected behavior

Engram's memory use should remain bounded relative to indexed workspace size
and active operations. Repeated work and idle periods should not cause
unbounded private-memory growth. Disconnected client shims should terminate
when no longer needed.

## Observed behavior

Windows reported a potential leak associated with `engram.exe`. The recovered
event does not identify the target Engram PID, executable path, command line,
workspace, allocation site, or memory usage.

The event alone does not establish a crash, an allocation failure, or a
specific leaking code path.

## Primary evidence

The event was re-read from the Windows Application log on November 13.

| Field | Recorded value |
| --- | --- |
| Log | Application |
| Provider | Windows Error Reporting |
| Event ID | 1001 |
| Event record ID | 50105 |
| Event time | 2026-11-12T07:39:03.1272403Z |
| Event name | RADAR_PRE_LEAK_64 |
| Fault bucket | 2102764998484433818 |
| Bucket type | 5 |
| P1 | engram.exe |
| P2 | 0.0.0.0 |
| P3 | 10.0.26300.2.0.0 |
| P4 through P10 | Empty |
| Report ID | efc77de4-60a3-42d6-b1fb-90fae4585427 |
| Report status | 268435456 |
| Hashed bucket | 59f6edf9a858fd318d2e85a00cca1b9a |
| Response | Not available |
| Cab ID / Cab GUID | 0 |
| Report archive location | NULL |

`P2` does not provide a usable Engram release identifier. Do not infer the
affected build from a currently installed binary.

The event listed temporary attachments, including `empty.txt`, WER internal
metadata XML, CSV, TXT, and XML files. Their contents and continued existence
were not inspected. No dump, allocation stack, or heap trace was recovered.

### Read-only event retrieval

Run on the original Windows host while the event remains retained:

```powershell
Get-WinEvent -FilterHashtable @{
    LogName = 'Application'
    ProviderName = 'Windows Error Reporting'
    Id = 1001
    StartTime = [datetime]'2026-11-12T07:38:00Z'
    EndTime = [datetime]'2026-11-12T07:40:00Z'
} | Where-Object {
    $_.RecordId -eq 50105
} | Select-Object RecordId, TimeCreated, Message
```

`TimeCreated` may render in the host's local timezone. Convert it with
`$event.TimeCreated.ToUniversalTime().ToString('o')` before recording it.
The record ID is local to this host and log, not a portable identifier.

## Related observations and attribution limits

### Duplicate shims observed on November 13

A later process inspection found **30 Engram processes**: three workspace
daemons and 27 client shims. Of those shims, 25 were children of one long-lived
Copilot process and used the GSD workspace's Engram binary.

The operator requested cleanup. Twenty-four older GSD shims were stopped,
preserving all three daemons and the newest shim for each Copilot session.
Verification found six remaining processes: three daemons and three shims.

This is a separate lifecycle-retention symptom worth evaluating. It does not
prove that shim accumulation caused the November 12 leak warning. The
inspected GSD shims were created after that warning.

### Later Engram memory snapshot

The following private-memory values were sampled on November 13, after cleanup.
They are not measurements of the process that generated the November 12 event.

| Workspace / role | PID at sampling | Private memory |
| --- | --- | --- |
| GSD daemon | 14060 | 1345.4 MiB |
| go-teams daemon | 33468 | 28.8 MiB |
| backlogit daemon | 57032 | 338.4 MiB |
| Three retained shims | 4628, 18404, 54012 | Approximately 5.1 MiB each |

One snapshot cannot establish a leak or compare workspace-normalized memory
requirements. Current daemon sizes must not be used to assign the earlier
warning to a particular workspace.

### System-wide commit exhaustion and later Copilot crash

Windows recorded low-virtual-memory events independently of the Engram warning:

* **November 12, 06:40:46Z:** System event 2004 attributed
  71,701,045,248 bytes, approximately 66.8 GiB, to `svchost.exe` PID 2944.
* **November 13, 17:57:28Z:** System event 2004 attributed approximately
  70.1 GiB to that service host and 5.1 GiB to Copilot PID 10696.
* **November 13, 17:58:00Z:** Application event 1000 recorded Copilot PID
  10696 crashing in `cli-native.node` with exception `0xc0000409`.
* **November 13 investigation:** PID 2944 hosted `TermService`
  (Remote Desktop Services), using approximately 70.6 GiB private memory.
  System commit was approximately 126.8 / 127.9 GiB, or 99%.

These observations make system-wide commit pressure an important confounder.
The later Copilot crash is not evidence that Engram crashed. No explicit
allocator-failure message was found in the inspected failed Copilot process
log. The precise crash mechanism remains unconfirmed.

## Investigation requested in the Engram workspace

1. Establish attribution, if retained evidence permits: identify the November
   12 binary build, target PID, daemon-versus-shim role, workspace, uptime, and
   operations preceding the warning. Preserve any surviving WER artifacts.
2. Measure private bytes / committed memory over time under a controlled
   workload and idle recovery period. Record system commit headroom alongside
   Engram memory so host exhaustion is not mistaken for an application leak.
3. Exercise repeated search, indexing, synchronization, workspace/session
   reconnection, and client disconnect cycles. Compare against an idle
   baseline and use a representative fixed-size workspace.
4. If growth persists, profile allocations and retention. Evaluate boundedness
   of caches, graph/index state, request queues, connection tasks, watcher
   lifetimes, and embedding resources as hypotheses, not established causes.
5. Evaluate stale-shim cleanup separately, including parent exit, stdin EOF,
   broken IPC, reconnects, and shutdown. Determine whether Engram, its client
   host, or their interaction owns the retained-process behavior.

Do not deliberately exhaust memory or restart shared services on the affected
host. Run stress reproduction in an isolated environment with explicit memory
and duration limits.

## Remediation acceptance criteria

* Document the affected build, process role, and reproduction workload, or
  explicitly retain those attribution gaps if historical evidence is lost
* Demonstrate bounded memory across repeated equivalent workloads and idle
  periods, using a justified workspace-specific budget and growth threshold
* Add a regression test for any confirmed retention or lifecycle defect
* Verify abandoned shims exit without disrupting valid clients or daemons if
  the separate shim-retention symptom is confirmed
* Report measurements, test results, and remaining uncertainty; absence of a
  new Windows warning alone is not sufficient proof of remediation

## Handoff boundary

Evaluate and remediate in the Engram workspace. This report does not authorize
changes to Windows services, pagefile settings, other workspaces, or shared
running sessions. The underlying allocation path remains unknown.
