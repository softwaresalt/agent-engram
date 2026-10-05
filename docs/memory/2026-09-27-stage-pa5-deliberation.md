---
type: session-memory
date: 2026-09-27
agent: stage
feature: 142-F
decision: PA-5
status: awaiting-operator-decision
---

# Stage session: PA-5 deliberation

## Outcome

* Wrote `docs/decisions/2026-09-27-pa5-read-handler-conversion-deliberation.md`
  (deliberation only; no plan, plan review, or harvest).
* Recommended option: PA5-C (convert by data class, with honest F54
  expectations for non-generation reads). About 7 tasks, all S or M.
* Added the PA5-A/B/C, PA5-T1...T7, generation-backed read, pinned read, and
  `DaemonScopedRead` labels to `docs/operator-glossary.md`.

## Key findings

* The F54 equivalence test needs only `get_workspace_status`. The F54 matrix
  test needs all 15 generation-read rows (16 with `git-graph`) accepted with
  provenance.
* F54's fixture generation DB holds only `probe_row`, so code-graph rows fail
  with SymbolNotFound or query errors even after conversion. The fixture must
  publish a real indexed generation (Q2).
* `GENERATION_PINNED_READS` is not in `src/` yet. `142.066-T` introduces it.
* The metrics, report, lint, and daemon-status reads do not read generation
  data. Labeling them with provenance would be a false claim (Q1).
* `4628001C` is consumed by PA5-T1 under every option.

## Not done (by design)

No backlog, stash, dependency, shipment, source, test, config, or git changes.

## Next steps

Operator answers Q1-Q4. Then Stage runs impl-plan, plan-harden, plan-review,
and harvest into S4.
