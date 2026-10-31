---
title: "168-S runtime verification and pre-merge releasability"
date: 2026-10-31
shipment_id: "168-S"
feature_id: "142-F"
task_id: "142.081-T"
verification_verdict: PASS_WITH_FOLLOW_UP
releasability: READY_WITH_CONDITIONS
---

## Validator contract and evidence

The workspace profile's command adapters apply to CLI and API surfaces.
The changed surface is read-server admission and dispatch for `query_memory`
and `list_symbols`, not background indexing. These results validate the new
binary and isolated real-generation fixtures, not a deployment of that binary
over the existing daemon.

| Surface | Adapter and probe | Result |
|---|---|---|
| CLI | `target/validation/debug/engram.exe --version` | Exit 0, `0.3.0-rc.1+g6444dfe6-dirty`, built before IC commit |
| API | `cargo test --test integration_generation_pinned_memory_and_symbols -- --nocapture` | Three consecutive GREEN runs, 3/3 each |
| API admission | `cargo test --test integration_read_server_generation_wiring -- --nocapture` | Three consecutive GREEN runs, 13/13 each |
| Consumers | Core, search, pinning-contract, report and lifecycle read-generation targets | GREEN |
| Full contract coverage | `RUST_TEST_THREADS=1 cargo dev-test --no-fail-fast` | Final run exit 0, 2,607 passed, 20 ignored, zero failed |
| Background job | Unchanged; existing daemon not restarted or rebound | Not exercised as a rollout probe |

All cargo commands used `CARGO_TARGET_DIR=target/validation`.
The manifest's stale `engram status` hint is not claimed as executed:
the supported `engram daemon-status` and `engram workspace-status` probes
confirmed PID 43836 reachable and bound, but reporting its previous-branch
generation. No current-source deployment result is inferred from that daemon.
No browser or operator-assisted checkpoint is required for this bounded API
conversion. No missing prerequisite blocks the isolated affected-surface tests.

Fixtures publish and activate an indexed generation with `alpha` and `beta`,
verify actual memory content and two symbols, verify generation provenance,
compare with empty managed-state results, and check unchanged generation
directory listings after both reads.

## Pre-deploy audit and safety

* Formatting, pedantic clippy, all-targets check/build and coverage completeness
  passed; coverage reported 277 targets and zero unmapped targets
* Cargo.lock is unchanged; eight audit advisories remain accepted baseline
  risk under `D74F9442` and `25A1BE9E`
* The first full run exceeded an existing ingestion timing threshold;
  its isolated rerun and final full serial run passed without edits
* No migration, dependency change, flag change or production configuration change
* ProposedAction: convert only the two authorized read handlers and dispatch
  admission; ActionRisk: moderate; operator-approved scope; ActionResult: applied
* No daemon termination, live database editing or destructive cleanup

## Monitoring and rollback

Owner: `softwaresalt`. Validation window: first 30 minutes after feature merge,
including the next main CI result and any subsequent read-server deployment.
This is a handoff, not a claim that deployment observation has completed.

Healthy signal: all admitted reads use their opened generation, preserve
matching generation provenance, return fixture content and create no sidecars.
Monitor generation-wiring and pinned-memory/symbol probes and daemon health
using `docs/log-observation-guide.md`.

A reproducible provenance mismatch, managed-store access on a generation
context, or new sidecar after one isolated confirmation is a rollback trigger.
Use a reviewed source/test-only revert of this slot's feature merge, preserving
backlog archives, immutable closure evidence and unrelated feature work.

## Releasability handoff

`READY_WITH_CONDITIONS`: affected-surface verification passes; deployment
observation remains owned by the maintainer. Merge-only delivery does not
restart the running daemon. Existing follow-up `D74F9442` carries audit risk;
no duplicate or new P-021 capture is needed.
