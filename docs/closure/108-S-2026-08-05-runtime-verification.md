---
title: "Shipment 108-S runtime verification"
date: 2026-08-05
shipment_id: "108-S"
feature_id: "112-F"
surface: cli
adapter: command
verdict: BLOCKED
---

## Shipment 108-S Runtime Verification

### Validator contract

- Surface: Windows cold real CLI and auto-spawned named-pipe daemon.
- Adapter: focused ignored command test using `target/debug/engram.exe`.
- Invariants: fixed request and correlation IDs, one owned workspace/PID/pipe,
  five-minute aggregate deadline, no repository-daemon mutation, and verified
  graceful PID/pipe cleanup.
- Live cap: one RED execution and one post-seam execution; no third run.

### Environment prechecks

Both runs used fresh temporary Git workspaces with a frozen tiny corpus. The
harness asserted that no owned PID or reachable endpoint existed before CLI
launch. Repository daemon PID `16084` remained healthy and observation-only.

### Probe outcomes

| Probe | Result |
|---|---|
| Attempt-one RED | Client completion and correlated dispatch present; zero terminal frame records |
| Attempt-two post-seam | Client completion and correlated dispatch present; zero JSON-decodable terminal frame records |
| Exact request ID | `62046B37-cold-1` on both client responses |
| Correlation ID | `62046B37` on both `index_workspace` usage records |
| User timeout | One second |
| Aggregate bound | 8,438 ms and 7,770 ms through PID death and pipe closure |
| Bounded cleanup | PIDs `16360` and `29700` dead; both pipes unreachable |
| Temp workspaces | Removal observed externally after return; not included in sampled elapsed time |
| Force termination | Not used |

Attempt two reached the new response-frame event, but unconditional pretty
tracing rendered it as multi-line text while the bounded parser accepted only
discrete JSON records. The exact response ID and terminal outcome therefore
were not retained. A subsequent non-live change writes the debug capture event
through the JSON tracing subscriber; deterministic production-event tests,
formatting, pedantic Clippy, and release compilation pass, but the hard live
cap prevents another runtime claim.

### Risky action state

- Debug-only workspace-contained capture: implemented; no arbitrary path and no
  release capture behavior.
- Two-run characterization: completed at the exact cap with deterministic
  PID/pipe cleanup. `TempDir` Drop removal was unchecked by the harness and
  observed externally after return.
- Force termination: not invoked.
- Production timeout or IPC redesign: not performed.

### Verdict and handoff

Meaningful CLI verification ran, but exact response-frame correlation remains
unverified because the final JSON-format remediation was applied only after the
second and final live attempt. Operational closure must preserve that
condition; a future live reproduction requires a fresh reviewed Stage intake.

Verdict: BLOCKED
