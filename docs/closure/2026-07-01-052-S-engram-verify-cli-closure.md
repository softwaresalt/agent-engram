---
title: "052-S engram verify structural linter CLI (Phase 1a) — Closure"
type: closure
date: 2026-07-01
feature: 064-F
shipment: 052-S
pr: 185
merge_sha: pending-operator-approval
branch: 064-engram-verify-cli
---

## Summary

Executed shipment `052-S` — Phase 1a of feature `064-F` (Deterministic gates &
telemetry): a local, no-daemon `engram verify <path>` structural-conformance
linter CLI that unblocks the autoharness `pre_task_completion` gate. The command
is architecturally modeled on `engram manifest` (in-process, no daemon, no DB).
PR #185 is open against `main`; **merge is operator-gated and has NOT been
performed** (this closure is prepared pre-merge and awaits approval).

## Tasks Completed

| Task | Title | Status |
|---|---|---|
| 064.001-T | Phase 1a — verify linter core service + result model | done |
| 064.002-T | Phase 1a — `engram verify <path>` CLI subcommand + exit-code/stderr contract | done |
| 064.003-T | Phase 1a — cross-platform path normalization + subprocess integration test | done |

## Pinned Exit-Code Contract (verified at runtime)

| Exit | Meaning |
|---|---|
| `0` | Conformant, or a non-markdown target (nothing to validate in Phase 1a) |
| `1` | Non-conformant — findings written to **stderr** |
| `2` | I/O or usage error (missing/unreadable file, outside-workspace, `..` traversal) |

Malformed frontmatter (delimiters present, YAML invalid) → `1`. Absent
frontmatter (no delimiters) → allowed. A JSON summary envelope is written to
stdout. Scope is per-file (no glob).

## Shipment Reconciliation

* Shipment `052-S` and feature `064-F` set to `active`; tasks `064.001-T`,
  `064.002-T`, `064.003-T` set to `done`.
* Task specs are retained in `.backlogit/queue/` (NOT relocated to
  `.backlogit/archive/`) because backlogit reused these IDs from the previously
  shipped powerbi-tmdl feature (PR #169, closure
  `2026-06-14-064-S-tmdl-parser-closure.md`, merge `1475200d…`). Relocating to
  archive would collide with those existing records on `main`. This is a backlog
  data-integrity issue (two distinct `064-F` features / duplicate `064.00X-T`
  IDs) that Stage/operator should reconcile — out of Ship scope.
* Deferred tasks `064.004-T`, `064.005-T`, `064.006-T` are NOT in this shipment.
  `064.005-T` / `064.006-T` already exist on `main` (older powerbi content) and
  were intentionally excluded from this PR.

## Quality Gates

Run with the exact CI feature set (`--no-default-features --features
cozo-backend,embeddings`):

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | pass (local + CI) |
| `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | pass (local + CI) |
| `cargo test --all-targets` | all 12 verify tests pass; unrelated environmental flakes only (see Failure Signals) |
| `cargo audit` | advisory-only, CI `continue-on-error`; no new advisories (dependency graph inherited from `main`) |

## Review Disposition

An independent Rust review raised two items initially tagged P1; both were
adjudicated against the authoritative plan and downgraded to non-blocking
follow-ups:

1. *Non-markdown should exit 2* — REJECTED. The plan explicitly pins
   non-markdown → exit `0` (§4 task spec, contract scenario, risk table, and
   resolved open-question Q6). Implementation is correct per contract. The minor
   existence-before-extension ordering is captured as a P3 follow-up.
2. *Relative-path containment when `--workspace` ≠ CWD* — DEFERRED to Phase 1b.
   All pinned containment scenarios (reject `..` / outside-workspace absolute)
   are satisfied and tested; the bypass requires a workspace ≠ CWD, outside the
   Phase-1a per-file autoharness usage model; verify is read-only (bounded
   impact). Expanding to canonicalize+join would change missing-file/non-markdown
   semantics and exceed the declared `careful`/freeze-scope without a RED test.

Max review-fix cycles (3) not exceeded (0 code changes required).

## Runtime Verification

Exercised the built `engram verify` binary against fixtures in both PowerShell
and cmd.exe (delayed expansion). All cases pass:

| Case | Path | Exit | Expected |
|---|---|---|---|
| conformant | `tests/fixtures/verify/conformant.md` | 0 | 0 |
| malformed frontmatter | `tests/fixtures/verify/malformed.md` | 1 | 1 |
| absent frontmatter | (no delimiters) | 0 | 0 |
| non-existent md | `…/nope.md` | 2 | 2 |
| non-markdown | `Cargo.toml` | 0 | 0 |
| backslash path (Windows) | `tests\fixtures\verify\conformant.md` | 0 | 0 |
| traversal reject | `..\outside.md` | 2 | 2 |

Findings verified on **stderr**:
`[frontmatter.malformed] …: frontmatter delimiters present but YAML failed to
parse (line 1)`. JSON summary envelope verified on stdout.

## Invariants to Preserve

* Exit-code contract `0`/`1`/`2` is a public contract the autoharness gate
  depends on — do not drift.
* Findings must remain on **stderr** (agent context injection); summary envelope
  on stdout.
* `#![forbid(unsafe_code)]`; `Result<T, EngramError>` with `?`; no
  `unwrap`/`expect` in non-test code.
* Malformed-vs-absent frontmatter distinction must survive changes to
  `frontmatter::parse` (which returns `None` for both).

## Deployment or Rollout Path

Additive new CLI subcommand; no schema, daemon, or migration changes. Available
after the binary is rebuilt/reinstalled. autoharness config wires
`engram verify {file_path}` into `pre_task_completion` on the autoharness side.

## Post-Deploy Checks

* `engram verify <a conformant .md>` → exit 0.
* `engram verify <a malformed-frontmatter .md>` → exit 1 + stderr finding.
* `engram verify <missing path>` → exit 2.

## Healthy Signals

* fmt + clippy green locally and on CI (ubuntu-latest).
* All 12 verify tests (4 unit + 4 contract + 4 integration) green.
* Runtime exit contract holds cross-shell.

## Failure Signals (environmental, not introduced by this change)

* `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` (daemon TTL timing) — the
  test's own comments document it as racy on loaded machines; passes in
  isolation.
* `unit_notebook_extract` transient incremental-linker recompile on Windows —
  passes on isolated recompile.
* `t030_001_{c,swift}_function_indexed_via_ipc` — panic inside the `cozo-0.7.6`
  SQLite storage dependency under concurrent IPC indexing. Pre-existing cozo
  instability (repo pins cozo 0.7.6; "unblock on cozo ≥ 0.8"). `main`'s own CI
  shows ~2/6 recent runs failing on this class of flake. Addressed via CI
  re-run.

## Follow-on Backlog

* Phase 1b: workspace-root canonicalize + join containment for
  `engram verify` (close the `--workspace` ≠ CWD relative-path gap).
* Verify file existence before the non-markdown extension short-circuit (avoid a
  mistyped extension silently passing the gate).
* Backlog ID-reuse reconciliation: distinct `064-F` features and duplicate
  `064.00X-T` task IDs across the powerbi and verify workstreams.
* Deferred Phase 1a+ tasks: `064.004-T`, `064.005-T`, `064.006-T`.
* Pre-existing `otlp-export` feature build breakage (opentelemetry API drift) —
  outside freeze-scope.
