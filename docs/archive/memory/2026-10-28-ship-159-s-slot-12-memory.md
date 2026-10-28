---
title: "Ship 159-S (Slot-12, 142.071-T NEW-3) session memory"
date: 2026-10-28
shipment: 159-S
feature: 142-F
---

## Items

* 142.071-T done and archived. Feature PR #449 merged at `ef1df553dc293220052faf343ae5c9a285bf2672` (head `778ebf0d`).
* Harness-first commit (3 tests; scenarios 1 and 2 RED against the `Worker: 142.071-T` placeholder, scenario 3 characterization), implementation `a4d33df8`, review fix (broken-pipe writes) after.

## Decisions

* `production_factory` maps every setup failure (layout, id mint, verifier construction) to `SetupError::Layout`; a distinct variant is deferred to stash `9933B77D` together with relative-workspace canonicalization.
* Output uses `writeln!`/`write_all` with ignored errors so a closed pipe cannot change the verdict exit code.
* `cargo dev-test` fails only the known load-sensitive `backlog_index_100_items_under_5_seconds` (stash 1346BC60 and 41DB6AD0); passes in isolation.
* Four-persona review: no P0/P1; Copilot review `5688834779`: no findings, no threads.

## Next

* `142.072-T` (NEW-4) and `142.073-T` (NEW-5, Slot-14) consume the entrypoint. Stage triages stash `9933B77D`.
