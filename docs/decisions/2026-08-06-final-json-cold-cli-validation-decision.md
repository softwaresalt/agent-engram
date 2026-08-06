---
title: "Final JSON cold CLI response-frame validation decision"
type: decision
date: 2026-08-06
status: accepted
feature_id: "113-F"
shipment_id: "109-S"
prior_feature: "112-F"
prior_shipment: "108-S"
source_revision: "27d3d3a482cc4beae94ce200893d3d11ceaf908d"
runtime_report: "docs/closure/109-S-2026-08-06-runtime-verification.md"
---

# Final JSON Cold CLI Response-Frame Validation Decision

## Decision

Close the prior final-JSON runtime blocker. The sole new-unit live attempt
produced one exact client/usage/frame chain and complete owned cleanup:

- client response ID `62046B37-cold-1`, disposition `completion`, exit code `0`;
- `index_workspace` usage correlation ID `62046B37`, outcome `success`;
- terminal `response_frame_result` response ID `62046B37-cold-1`, outcome
  `flushed`;
- exact owned PID `18248` dead;
- exact named pipe
  `\\.\pipe\engram-bfdee6ac-bd89-4cd4-a96d-e9d433dace83` unreachable;
- exact owned temp path
  `\\?\C:\Source\GitHub\engram\tmp\cold-cli-correlation-ZJxzha` absent after
  harness return; and
- force-kill not used.

This closes the blocker retained by shipment `108-S`: the final debug-only
JSON capture now has a runtime-proven, JSON-decodable terminal frame carrying
the exact response ID and terminal outcome.

## Provenance and Bounds

| Evidence | Value |
|---|---|
| New release unit | Feature `113-F`, shipment `109-S` |
| Attempt | `1/1`, with no retry |
| Live source revision | `27d3d3a482cc4beae94ce200893d3d11ceaf908d` |
| Binary | `target/debug/engram.exe` |
| Binary SHA-256 | `81034B210CB2EE833CFC726658E9920B9CB7EC4D9F77BBDBBB66059C9C557B14` |
| Platform | Microsoft Windows `10.0.26200`, x64, Windows named pipes |
| Request ID | `62046B37-cold-1` |
| Correlation ID | `62046B37` |
| Corpus SHA-256 | `58275c855655b513a682d3e3954d3c55d60d6634300e6e8f17541893aaa00a25` |
| Request timeout | 1 second |
| Aggregate bound | 300,000 ms, including cleanup |
| Cleanup reserve | 60,000 ms |
| Idle fallback | 20,000 ms |
| Observed CLI elapsed | 8,205 ms |
| Observed aggregate elapsed | 8,354 ms |
| Runtime report | `docs/closure/109-S-2026-08-06-runtime-verification.md` |

Before the live attempt, focused deterministic coverage passed for exact
cardinality, adjacency rejection, JSON-line parsing, typed frame serialization,
and exact owned cleanup. The live test was ignored during that preflight.

## Historical Boundary

Shipment `108-S` remains archived and exhausted at `2/2`. Its historical
attempt table and evidence are unchanged. Shipment `109-S` did not extend,
reinterpret, or retry `108-S`; it owned one distinct new attempt and exhausted
that allowance at `1/1`.

The successful result proves the final JSON response-frame capture only. It
does not authorize or claim:

- a production timeout change;
- daemon startup or shutdown redesign;
- an IPC protocol change;
- S072 or audit work; or
- a retained-test refactor.

## Follow-Up Boundary

No follow-up is required to close the final JSON capture blocker. A future
product decision about moving cold startup inside the user request deadline
would remain separate intake and would require a fresh reviewed Stage cycle.
No such work was planned, modified, or implemented here.

Do not rerun the ignored live scenario for this release unit. Its authorized
allowance is exhausted at `1/1`.

Classification: CORRELATED-COMPLETION
