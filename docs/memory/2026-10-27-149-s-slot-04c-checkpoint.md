# 149-S Slot-04c checkpoint (142.064-T, PRE-3c)

Branch `feat/149-s-142-f-slot-04c-142-064-t-pre-3c-transient-retry-pre-3-close`, dark mode (P-017).

## Done

* RED harness (commit 6a761628): `an_absent_inventory_file_is_classified_transient` (GREEN precondition:
  absent inventory file is `RejectionClass::Transient` by `classify`) and
  `read_server_retries_a_transient_failure_of_the_same_revision` (RED, `Worker: 142.064-T` timeout);
  `142.076-T` and `142.077-T` cases stayed GREEN; `cargo check --all-targets` passed.
* Build: `TransientRetry` in `src/daemon/lifecycle_policy.rs` (100 ms doubling, 2 s cap backoff; window of
  `layout.activation_deadline` from the revision's FIRST transient failure; permanent rejection resets state;
  newer revision starts a fresh window). Unit test `transient_retry_follows_backoff_window_and_revision`.
* Review fix (adversarial, READY_WITH_FOLLOWUPS): driver tick `delay` is now set to the retry backoff so the
  100 ms schedule is not hidden by the loop's capped delay; pure unit test added; `.min(16)` commented.
* Evidence: three consecutive full-output runs of `integration_read_server_generation_wiring` after the final
  build: 10 passed each, identical per-case maps. All 8 consumer targets match the `142.076-T` RED baseline
  counts (7, 3, 1, 3, 2, 3, 3, 3 passed).
* Gates: fmt, clippy pedantic (no allow), `cargo build --all-targets` PASS.

## Notes

* `cargo dev-test`: first full run passed (exit 0, pre-review-fix). After the review fix the only failure was
  `integration_backlog_hydration::backlog_index_100_items_under_5_seconds`, a wall-clock test (5.0-8.9 s
  against a 5 s limit, host under heavy Defender load); unrelated to this diff. `hcl_indexing_test`
  cold-start and `integration_read_server_lifecycle` each flaked once under load and passed on rerun.
* Subtasks `142.064.001-ST`..`.003-ST` untouched (Stage closes them per plan 10.1).

## Next

Push, open PR with Local Review Readiness, Copilot review loop, merge, post-merge closure.
