# Ship checkpoint: 151-S (142-F Slot-06, 142.066-T PRE-4)

* Branch: `feat/151-s-142-f-slot-06-142-066-t-pre-4`. Harness commit `dab4cdcf` (RED), implementation `8c73058d`.
* RED: 3 PRE-4 scenarios failed with `Worker: 142.066-T GENERATION_PINNED_READS`; 10 PRE-3 cases stayed GREEN.
* GREEN: 13/13 in `integration_read_server_generation_wiring`.
* R7.8: three consecutive careful-mode runs of the target, `integration_request_entry_activation` and the 8 PRE-3 consumer targets gave identical per-case maps.
* Gates: fmt, clippy pedantic, build --all-targets, `cargo dev-test` PASS.
* Local review (Correctness, Rust, Security, Architecture): READY_WITH_FOLLOWUPS. The Security P1 (read-server fall-through is not fail-closed for non-pinned methods) is out of scope under P-021 C1 and was captured as stash `65EF8399`.
* Next: PR, Copilot review loop, merge, post-merge closure.
