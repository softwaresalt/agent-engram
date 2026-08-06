---
title: "Cold CLI request-ID and response-frame correlation follow-up"
type: spike-findings
date: 2026-08-05
status: BLOCKED
source_revision: ca398bd6
feature_id: "112-F"
shipment_id: "108-S"
stash_id: "62046B37"
prior_feature: "111-F"
prior_shipment: "107-S"
---

## Cold CLI Request-ID and Response-Frame Correlation Follow-up

### Scope and provenance

Shipment `108-S` followed the Windows cold real-CLI blocker retained by
shipment `107-S`. It did not reopen the separately closed persistence result,
change production timeout semantics, redesign daemon startup or IPC, address
S072 or audit work, or refactor the retained 107-S characterization.

- Platform: `Windows_NT`, Windows named pipes.
- Branch: `feat/108-s-cold-cli-correlation`.
- Attempt-one revision: `54bac42a` plus the uncommitted U1 harness.
- Attempt-one binary: `target/debug/engram.exe`.
- Attempt-two revision: `92b47c6ab6495f18b859800259a6038a0d515adf`
  plus the initial uncommitted U2 source seam.
- Attempt-two binary: `target/debug/engram.exe`.
- Attempt-two binary SHA-256:
  `273918FA5BBCEC31FFD20A62AD32C576BDC8D18D292DAFB762E3A3ADC95DB64C`.
- Recorded post-attempt source revision: `ca398bd6`. This included the
  non-live-only JSON capture remediation made after attempt two. Review-cycle
  remediation atop `e45b61c1` directly covers production event serialization;
  neither change has an additional runtime claim.
- Frozen tiny-corpus aggregate SHA-256:
  `58275c855655b513a682d3e3954d3c55d60d6634300e6e8f17541893aaa00a25`.
- JSON-RPC request ID: `62046B37-cold-1`.
- Correlation ID: `62046B37`.
- User request timeout: one second.
- Aggregate supervisor: five minutes including startup, evidence collection,
  graceful shutdown, idle fallback, and PID/endpoint verification. The elapsed
  sample was taken while `TempDir` was still live; its unchecked Drop cleanup
  occurred after return and was observed externally.

### Attempt evidence

The two-attempt cap was exhausted exactly as planned: one RED run before the
source seam and one post-seam run. No third live execution was performed.

| Evidence | Attempt one — RED | Attempt two — post-seam |
|---|---|---|
| CLI start | `2026-08-06T04:16:26.497Z` | `2026-08-06T04:38:17.185Z` |
| CLI finish | `2026-08-06T04:16:34.537Z` | `2026-08-06T04:38:24.613Z` |
| CLI elapsed | 8,040 ms | 7,427 ms |
| Aggregate elapsed through PID death and pipe closure | 8,438 ms | 7,770 ms |
| Client disposition | exit 0; correlated completion | exit 0; correlated completion |
| Client response ID | `62046B37-cold-1` | `62046B37-cold-1` |
| Dispatch evidence | correlation `62046B37`; `index_workspace` completion | correlation `62046B37`; `index_workspace` completion |
| Terminal frame records parsed | 0 | 0 |
| Frame response ID and outcome | unavailable before the seam | unavailable because the pretty trace was not JSON-decodable |
| Owned daemon PID | `16360` | `29700` |
| Owned named pipe | `\\.\pipe\engram-9d8bcb92-ec0c-4b25-85a3-7d87314baaf8` | `\\.\pipe\engram-994b0a45-ad68-44f2-b69f-ef62b2862088` |
| Bounded cleanup | PID dead, pipe unreachable | PID dead, pipe unreachable |
| Temp workspace | Removal observed externally after return | Removal observed externally after return |

Attempt one established the expected RED contract: the harness observed one
client completion and one correlated dispatch record, but no explicit terminal
frame record.

Attempt two exercised the response-frame seam. The client again completed and
the exact dispatch correlation was present. The daemon's unconditional pretty
tracing formatted the frame event as multi-line text, while the bounded harness
accepted only discrete JSON records. It therefore parsed zero frame records
and could not retain the frame response ID or `flushed`, `serialize_error`,
`write_error`, or `flush_error` outcome. This is the concrete remaining
runtime blocker.

After attempt two, the debug-only fixed workspace-local capture was changed to
configure the existing tracing subscriber for JSON. Non-live verification
passed, including the focused deterministic production event/serialization
test, pedantic Clippy, formatting, and release compilation. Because the live
cap was already exhausted, this final format change was not executed against
another cold daemon and does not close the runtime blocker.

### Dispatch, frame, and client timeline

For both attempts, the cold CLI began with no owned live PID or reachable
workspace endpoint. Auto-spawn then created one owned daemon and one named-pipe
endpoint. The CLI took more than seven seconds end to end despite the one-second
request timeout, returned a successful response carrying
`62046B37-cold-1`, and persisted a successful `index_workspace` usage record
with correlation ID `62046B37`.

The terminal frame step cannot be placed into that exact ID chain from retained
runtime evidence. Attempt one had no seam. Attempt two emitted a terminal event
but the capture/parser format mismatch prevented retention of its exact response
ID and outcome. Timestamp or connection adjacency is deliberately insufficient
for filling that gap.

### Cleanup provenance

Both temporary workspaces were owned by the focused harness. Cleanup requested
graceful shutdown and retained a short inherited idle timeout as fallback.
Neither run used force termination. Each exact PID was dead, each exact named
pipe was unreachable before the aggregate elapsed sample. The `TempDir` was
still live at that sample; Drop removal was unchecked by the harness and both
removals were observed externally only after return. Repository daemon PID
`16084` and its workspace binding remained observation-only and healthy.

### Prior result and timeout-contract gate

Shipment `107-S` remains authoritative for persistence: the validated
singleton/calls persistence symptom did not reproduce at its tested revision,
and this shipment produced no contrary persistence evidence.

The static `startup-outside-deadline` finding is corroborated. Both cold CLI
processes completed successfully after more than seven seconds while receiving
`--timeout 1`; this matches the source ordering in which health and auto-spawn
precede the timeout-bounded request. The missing exact terminal frame record
prevents a complete request-ID/frame timeline, but it does not contradict the
startup-boundary observation.

If a future product change is desired, the smallest candidate boundary remains
`src/cli/runner.rs::run_tool_dispatch`: establish an end-to-end deadline before
health and startup, then pass only its remaining budget to the request phase.
That would change user-visible timeout behavior and requires a fresh Stage
intake, reviewed plan, dedicated tests, and separate runtime evidence. No such
fix is included here.

### Traceability

- Stash: `62046B37`.
- Feature: `112-F`.
- Shipment: `108-S`.
- Prior feature and shipment: `111-F` / `107-S`.
- Reviewed plan:
  `docs/exec-plans/2026-08-05-cold-cli-request-frame-correlation-plan.md`.
- Prior decision:
  `docs/decisions/2026-08-04-daemon-index-runtime-root-cause-follow-up.md`.

Classification: BLOCKED
