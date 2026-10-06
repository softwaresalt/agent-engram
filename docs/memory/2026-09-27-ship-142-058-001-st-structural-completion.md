# Ship 142.058.001-ST — Structural Parity Slice

- **Date:** 2026-09-27
- **Branch:** `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs`
- **Commit:** `6d216d1956d41510222d596d9283cf4679792fa1` — `test(142.058.001-ST): separate parity structure and refusal assertions`
- **Scope/status:** `142.058.001-ST` is done; parent `142.058-T` remains active and F54 is not complete.

The test-only change removes the success-allowed Control outcome, adds a standalone structural matrix test, and exercises descriptor-declared Control surfaces as refusals. `_shutdown` expects F38 code `16_001` and no filesystem/workspace-fingerprint side effects; `set_workspace` retains its specific retarget code `16_003`. No production code or other test file was modified.

## Verification

- `cargo test --test contract_read_server_cli_mcp_parity generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces -- --nocapture` — **PASS**, 1/1. The structural test checks each descriptor once and exact declared surface rows.
- `cargo test --test contract_read_server_cli_mcp_parity control_descriptors_are_refused_without_side_effects -- --nocapture` — **RED**, 0/1. `_shutdown`/direct IPC was accepted with `{flush_started: true, status: "shutting_down"}`, no refusal code, and `side_effect_count=1`.
- `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture` — **RED**, 1 passed / 4 failed. The structural test passed; behavior failures were preserved. The exact pending-behavior map is recorded in comments on `142.058.001-ST` and `142.058-T`.
- `cargo fmt --all -- --check` — **PASS**.
- `cargo check --all-targets` — **PASS**.

The parent task remains active. No production implementation, full-suite completion, PR, push, merge, shipment closure, or shipment-manifest change occurred. Other pre-existing workspace changes were left untouched.
