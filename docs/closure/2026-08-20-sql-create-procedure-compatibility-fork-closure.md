# Closure Record: SQL CREATE PROCEDURE via Approved Immutable Grammar Fork

**Date**: 2026-08-20
**Feature**: 123-F
**Shipment**: 119-S
**Related decision**: `docs/decisions/2026-08-20-tree-sitter-sequel-compatibility-fork-provenance.md`
**Task**: 123.006-T — runtime verification, observation, and rollback closure

## Runtime Verification (Release Binary)

The locally built `engram` binary — built via `cargo build --release`,
matching the release workflow's build profile
(`.github/workflows/release.yml`, `cargo build --locked --release --target
... --bin engram`) rather than a debug build — was run against a
representative, isolated SQL workspace containing `CREATE TABLE`,
`CREATE VIEW`, `CREATE FUNCTION`, and `CREATE PROCEDURE` statements, using
the full daemon indexing pipeline (`engram install` → `.engram/config.toml`
with `sql` added to `supported_languages` → `engram index` → `engram
symbols`).

### Fixture

```sql
CREATE TABLE orders (id INT, age INT);
CREATE VIEW active_orders AS SELECT id FROM orders WHERE age < 30;
CREATE FUNCTION total_orders() RETURNS INT AS BEGIN RETURN 0; END;
CREATE PROCEDURE archive_old_orders() BEGIN DELETE FROM orders WHERE age > 365 END;
```

### Result

`engram index` (1 file parsed): `classes_indexed: 2`, `functions_indexed: 2`,
`edges_created: 4`, `errors: []`.

`engram symbols` returned exactly 4 symbols, no duplicates, no missing
entries:

| Symbol | Type | Source |
|---|---|---|
| `orders` | class | `CREATE TABLE` |
| `active_orders` | class | `CREATE VIEW` |
| `total_orders` | function | `CREATE FUNCTION` (unchanged) |
| `archive_old_orders` | function | `CREATE PROCEDURE` (new) |

This confirms, against the actual `--release`-built binary (not only
`cargo test`):

* exactly one `Function` symbol for the procedure, matching the harvested
  contract,
* `CREATE FUNCTION` behavior is unchanged,
* no missing or duplicate procedure symbols,
* no ABI/build mismatch at runtime.

### Malformed-Input Graceful Degradation (Runtime)

A second file containing malformed SQL
(`CREATE PROCEDURE ( BEGIN SELEKT * FRUM;;; )) GARBAGE ~~~`) was added and
re-indexed against the same live daemon. Result: `errors: []`,
`functions_indexed`/`classes_indexed` unaffected by the malformed file (no
symbols extracted from it, no crash). `engram daemon-status` immediately
after showed `overall: green` across all health checks (`pid_liveness`,
`workspace_identity`, `pipe_reachability`, `registry_validity`,
`offline_scan`, `session_resume`, `telemetry_health`), confirming the daemon
process did not panic or degrade.

### Environment Note

SQL is not in the default `code_graph.supported_languages` policy list
(`rust`, `python`, `typescript`, `tsx`, `javascript`, `go`, `csharp`, `hcl`);
it must be explicitly opted in via `.engram/config.toml` in a consuming
workspace, same as before this change. This is pre-existing, unrelated
workspace-policy behavior, not introduced or altered by 123-F.

## Cross-Platform / Supported-Platform Evidence

* **Fork-level CI** (already recorded in the provenance decision): fork
  revision `50837582b5ba15c7acff3be7bf585a1082d90528`, run
  [32412941837](https://github.com/softwaresalt/tree-sitter-sql/actions/runs/32412941837)
  — `ubuntu-latest`, `windows-2025`, `macos-latest` all `success`. This
  validates the grammar crate's own generated C parser in isolation on all
  three platforms, not `agent-engram` linking against it.
* **Repository CI (this PR)**: `ci.yml`'s `build` job (full `fmt` → `clippy`
  → `test` → `audit` sequence, including compiling and testing against the
  new `tree-sitter-sequel` fork pin) runs on `ubuntu-latest` only. The
  `start-launcher-windows` job on `windows-latest` compiles the workspace
  (exercising the native C build on Windows) but only exercises the
  `contract_start_launcher` test subset, not the full SQL parsing test
  suite. The `release.yml` cross-platform matrix (`ubuntu-24.04`,
  `windows-latest`, `macos-latest`) that fully builds and links
  `agent-engram` against this dependency on all three platforms is
  tag-triggered only and does not run on this PR.
* **Known gap**: full-suite `cargo dev-test` against the adopted fork pin is
  therefore verified on Linux (PR CI) and Windows (local, this task) but not
  on macOS pre-merge. This is accepted as a residual gap for this shipment,
  mitigated by the fork's own macOS CI success (above) and the 72-hour
  post-merge observation window (below), which explicitly covers a
  supported-platform build/ABI failure as a rollback trigger. The next
  tagged release will exercise the full macOS build via `release.yml`
  before any release artifact is published.
* **Local (Windows) verification performed in this task**:
  * `cargo fmt --all -- --check` — pass
  * `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — 0 warnings
  * `cargo dev-test` — 653 passed, 0 failed
  * `cargo audit` — 0 new advisories attributable to `tree-sitter-sequel`
  * Runtime binary verification (above) — pass

## Monitoring Plan (Release-Observability)

| Signal | Baseline (crates.io 0.3.11) | Healthy (fork) | Alert / Rollback Threshold |
|---|---|---|---|
| Procedure symbols per representative `CREATE PROCEDURE` statement | 0 (ERROR node, graceful) | exactly 1 `Function` | 0 or >1 for a single statement |
| `CREATE FUNCTION` symbol count | 1 | 1 (unchanged) | any deviation from 1 |
| Parse panics/errors on SQL workspaces | none | none | any panic or hard parse error |
| Build/ABI status across Linux/Windows/macOS | n/a | green | any supported-platform build/ABI failure |

**Dashboard/query**: no dedicated metrics dashboard exists for this
workspace; observation is manual via `engram index` / `engram symbols`
output and CI status, per the pre-deploy audit below.

**Owner**: release maintainer / operator merging the PR.

## Pre-Deploy Audit Checklist

* [x] Rollback procedure documented and actionable (see decision doc and
      below).
* [x] No schema/data migration involved (dependency-source change only).
* [x] No cross-service boundary affected (single-binary daemon).
* [x] Monitoring plan complete (above).
* [x] Feature flag / rollout gate: not applicable — this is a build-time
      dependency change with no runtime feature flag; rollback is a single
      atomic `Cargo.toml`/`Cargo.lock`/test/doc revert (see decision doc).

## Post-Deploy Observation Window

**Window**: 72 hours from merge, owned by the release maintainer / operator
who approves and executes the merge.

During the window, watch for:

* SQL parsing panics or hard errors in daemon logs on any indexed workspace
  containing SQL,
* missing or duplicate `Function` symbols for `CREATE PROCEDURE` statements,
* `CREATE FUNCTION` regressions,
* any supported-platform CI failure on subsequent pushes,
* any fork repository provenance anomaly (force-push, ownership change,
  disabled CI) — see the decision document's compromise-response list.

**At window close**, record the outcome (healthy / degraded / rolled back)
as a follow-up backlog comment on 123-F or 119-S.

## Rollback Trigger

Roll back immediately, per the atomic procedure recorded in
`docs/decisions/2026-08-20-tree-sitter-sequel-compatibility-fork-provenance.md`,
on any of:

* a supported-platform build or ABI failure,
* a SQL parse panic or error regression,
* a `CREATE FUNCTION` regression,
* a missing or duplicate procedure symbol,
* a provenance anomaly (hash mismatch, unexplained delta, lost CI evidence,
  repository-control change) discovered during or after the observation
  window.

## Scope Confirmation

No new extraction arm was added (the pre-existing
`"create_function" | "create_procedure"` match arm in
`src/services/parsing/sql.rs` is the implementation). No fork
creation/administration, no unrelated SQL grammar work, and no dependency
version invention occurred in this shipment. 123-F supersedes archived
033.005-T; 033.005-T's blocked-on-upstream-release condition is resolved by
this approved immutable fork adoption instead.
