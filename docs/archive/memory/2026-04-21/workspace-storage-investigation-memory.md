# Workspace storage investigation memory

- Date: 2026-04-21
- Goal: identify why `d:\GitHub\agent-engram` consumes ~488+ GB.
- Findings:
  - `target/` is the dominant consumer at ~495,002 MB (~483.4 GiB).
  - `target/debug/deps/` alone is ~398,961 MB; `target/debug/incremental/` is ~88,808 MB.
  - `target/debug/deps/` contains 12 copies of `libsurrealdb_core-*.rlib`, averaging ~878 MB each, totaling ~10.29 GB.
  - Largest individual files are all in `target/debug/deps/`, including many `libsurrealdb_core-*.rlib` files (~877-881 MB each) and multiple test binaries (~626-632 MB each).
  - `target-redphase/` adds ~1,874 MB, mostly `target-redphase/debug/deps/` (~1,591 MB).
  - `.engram/` is ~2,231 MB; `.engram/db/` is ~2,086 MB, dominated by embedded SurrealDB commit-log (`clog`) files for multiple workspace/branch namespaces.
  - `.copilot/` is ~1,238 MB; mostly `.copilot/logs/` (~980 MB) and `.copilot/session-state/` (~250 MB).
- Why:
  - Rust debug builds and tests generate very large dependency archives, binaries, debug symbols, and incremental compilation caches.
  - This repo specifically uses heavy Rust dependencies (notably SurrealDB), which produce unusually large `.rlib` outputs.
  - The project intentionally persists local-first workspace index/database state in `.engram/` via embedded SurrealDB.
  - VS Code Copilot stores per-workspace logs and session state in `.copilot/`.
- Suggested cleanup targets:
  - Safe to reclaim most space by deleting `target/` and `target-redphase/` (recreated on next build).
  - Optional cleanup: prune `.copilot/logs/` and stale `.engram/db/*` namespaces if no longer needed.
