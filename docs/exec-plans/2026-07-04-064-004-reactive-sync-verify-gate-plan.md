---
date: 2026-07-04
type: impl-plan
task: 064.004-T
feature: 064-F
shipment: 072-S
title: Phase 1b — reactive daemon sync gated on verify conformance
width_domain: daemon event loop
blast_radius: elevated (plan-harden applied)
status: reviewed (072.001-R accepted)
test_first: true
---

# Impl-plan — 064.004-T: reactive markdown sync gated on `verify_markdown`

## 1. Objective

Gate the daemon's **reactive** (file-watch-driven) markdown re-ingestion on
`services::verify::verify_markdown` so that only structurally **conformant**
mutated markdown is ingested into CozoDB content nodes. Non-conformant
mutations are **skipped and logged** — never ingested. The code-file reindex
path is preserved byte-for-byte (no regression).

Acceptance criteria (verbatim from 064.004-T):

- Markdown `ReingestContent` actions are gated on `verify::verify_markdown`;
  non-conformant mutations are skipped and logged (not ingested).
- No regression to the existing code-file reindex path.
- Tests cover: valid md → ingested; invalid md → skipped + logged.

## 2. Grounded current state (read 2026-07-04 @ main 237595b)

| Fact | Evidence |
|---|---|
| `adapt_event` maps `Created/Modified` code files → `ReindexFile`, **everything else → `Skip`**; markdown currently `Skip`s. | `src/daemon/debounce.rs:89-110`, test `markdown_file_modified_skips` @ `:156` |
| `ServiceAction::ReingestContent` is **defined but never produced and never consumed**. | `src/daemon/debounce.rs:50` (def); only other hit is a `match` arm in a unit test `:186` |
| `verify_markdown(rel_path:&str, content:&str) -> Result<VerifyReport,EngramError>` is **only** wired into the CLI `verify` command — **not the daemon**. | `src/services/verify.rs:69`; sole caller `src/cli/commands/verify.rs:89` |
| `VerifyReport { conformant: bool, findings: Vec<VerifyFinding> }`; conformant ⇔ no findings. | `src/services/verify.rs:30-47` |
| The **live** event-consumer loop is inside `run_with_shutdown_v2` (dispatched from `daemon/mod.rs:202`); it only acts on `ReindexFile` via `code_graph::sync_workspace`. | `src/daemon/ipc_server.rs:797` (fn), loop `:1081-1102`; call site `src/daemon/mod.rs:202` |
| A **second, legacy** copy of the same loop exists in `run_with_shutdown` (also `ReindexFile`-only). | `src/daemon/ipc_server.rs:371` (fn), loop `:638-660` |
| `ingest_single_file(file_path, workspace_root, content_type, source_path, max_file_size, glob_filter, queries) -> Result<bool>` already reads the file itself and writes/deletes content records. | `src/services/ingestion.rs:641-700` |
| `RegistryConfig { sources: Vec<ContentSource> }` is loadable via `load_registry(&registry_path)`; existing daemon-side load pattern. | `src/models/registry.rs:100-103`; `src/services/registry.rs:71`; usage `src/tools/lifecycle.rs:133` |
| The loop already has `snapshot.path` (workspace) and `snapshot.data_dir` in scope. | `src/daemon/ipc_server.rs:1105-1112` |

**Key realization (scope-honest):** this is *not* merely "add a gate to an existing
reactive markdown reingest" — there is **no** reactive markdown reingest today
(markdown `Skip`s). The task therefore has two coupled parts: (A) **produce**
`ReingestContent` for markdown mutations, and (B) **consume** it behind the verify
gate. Both are required to satisfy the acceptance criteria. This is real ~2h
feature work in the daemon width domain. See §6 open questions.

## 3. Design

### Part A — classify markdown mutations (`debounce.rs`)

Extend `adapt_event` so `Created | Modified` on a markdown file
(`.md`, `.markdown`) returns `ServiceAction::ReingestContent { path }`.
`Deleted | Renamed` stay `Skip` (same rationale as code: workspace-level sync
cleanly sweeps orphaned content records — see `ingest_single_file`'s delete
branch at `ingestion.rs:668`). Code files remain `ReindexFile`; all other
extensions remain `Skip`. Add a `const MARKDOWN_EXTENSIONS: &[&str]` mirror of
`INDEXED_EXTENSIONS`.

### Part B — gate + ingest in the live v2 loop (`ipc_server.rs`)

In the `run_with_shutdown_v2` consumer loop (`:1081`):

1. During the debounce drain, accumulate `ReingestContent` paths into a
   deduplicated `pending_reingest: BTreeSet<PathBuf>` **in addition to** the
   existing `pending_reindex: bool`. The `ReindexFile`/`sync_workspace` branch
   is **unchanged**.
2. After the quiet window, if `pending_reingest` is non-empty, resolve the
   workspace `RegistryConfig` once via `load_registry` (data_dir-relative
   registry path, per `lifecycle.rs:133`). For each pending markdown path:
   - resolve its owning `ContentSource` by longest path-prefix match against
     `config.sources` (skip `content_type == "code" | "backlog"` and any source
     that does not own the path); if none owns it → `debug!` and skip;
   - read the file text; call `verify_markdown(rel_path, &content)`;
   - **conformant** → `ingest_single_file(path, ws_root, content_type,
     source_path, max_file_size, glob_filter, &queries)`; on `Err`, `warn!` and
     continue (never break the loop);
   - **non-conformant** → `warn!(path, findings = report.findings.len(),
     "skipped non-conformant markdown reingest")` and **do not ingest**.
3. TTL reset semantics are untouched — a markdown event still resets idle TTL
   exactly as today.

### Part B-hardening — extract the gate into a pure, daemon-free helper

To keep tests off the daemon event loop (Windows `run_with_shutdown_v2` SQLite
flake — see §5), extract the decision into a small, injectable async function,
e.g. in `services::ingestion` or a new `services::reactive_sync`:

```text
async fn verify_gated_reingest(
    path, ws_root, source: &ContentSource, max_file_size, glob, queries
) -> ReingestOutcome  // { Ingested, SkippedNonConformant, SkippedUnowned, Error }
```

The loop becomes a thin caller. All acceptance-criteria tests target this
helper directly; the loop wiring is covered by the existing (unchanged)
integration smoke, not by a new daemon-spin test.

## 4. Test-first plan (Constitution II — tests before impl)

**Unit — `debounce.rs` (`#[cfg(test)] mod tests`):**
- `markdown_modified_produces_reingest_content` (`.md`)
- `markdown_created_produces_reingest_content`
- `markdown_dot_markdown_ext_produces_reingest`
- `markdown_deleted_skips`, `markdown_renamed_skips`
- Regression: existing `rust_file_*`, `toml_*`, `no_extension_*`,
  `hidden_file_*` assertions stay **green** and unchanged. Update
  `markdown_file_modified_skips` → now expects `ReingestContent` (the only
  intentional behavior change to an existing test).

**Unit/contract — gate helper (`services`):**
- `valid_markdown_is_ingested`: conformant fixture → outcome `Ingested`, a
  content record is written (assert via test `CodeGraphQueries` / temp DB).
- `malformed_frontmatter_markdown_is_skipped_and_logged`: `frontmatter.malformed`
  fixture → outcome `SkippedNonConformant`, **no** record written.
- `empty_body_markdown_is_skipped`: `body.empty` fixture → `SkippedNonConformant`.
- `unresolved_template_var_markdown_is_skipped`: `{{...}}` fixture → skipped.
- `unowned_path_is_skipped`: path outside all sources → `SkippedUnowned`, no record.
- Assert a `warn`-level event is emitted on the non-conformant path (capture via
  `tracing-test` / `tracing_subscriber` test layer if already a dev-dep;
  otherwise assert on the returned `ReingestOutcome` discriminant, which is the
  authoritative, non-flaky signal).

**No-regression:** existing daemon/code-graph integration tests unchanged;
`sync_workspace` code path is not modified.

## 5. Blast-radius / plan-harden (elevated)

Daemon event loop, **two** loop copies, adjacent to the known Windows-only
`run_with_shutdown_v2` SQLite startup flake.

Mitigations (mandatory):
1. **Do not touch startup/Ready sequencing, watcher-init timeouts, IPC accept
   loop, or PID/lifecycle code.** The change lives entirely inside the
   already-spawned auto-sync task body. The plan must not depend on or perturb
   daemon-startup timing.
2. **Gate logic is a pure, injectable helper** (Part B-hardening) so no new test
   spins the daemon — this is the primary defense against the SQLite flake.
3. **v1 vs v2 (legacy loop):** gate **only** the live v2 path
   (`run_with_shutdown_v2`, dispatched at `daemon/mod.rs:202`). Add an in-file
   comment at the v1 loop (`run_with_shutdown:638`) noting markdown reingest is
   intentionally v2-only and v1 parity is out of scope. **Do not refactor v1.**
   (See open question Q2 — operator may instead want v1 confirmed dead and
   deleted in a *separate* item; not here.)
4. **Additive-only:** reuse the existing `ReingestContent` variant; no enum
   removals; no signature changes to `adapt_event`'s callers beyond the new arm.
5. **Fail-safe logging:** non-conformant → log + skip; verify/ingest `Err` →
   `warn!` + continue. Never `panic!`, never `unwrap`/`expect` (workspace lint
   denies both), never break the receive loop.
6. Safety mode: **freeze-scope** to `src/daemon/debounce.rs`,
   `src/daemon/ipc_server.rs` (v2 loop only), and the new/extended `services`
   gate helper + its tests. No schema, no CLI, no config surface changes.

## 6. Open questions (operator/implementer to resolve)

- **Q1 — content-source resolution.** Resolve a mutated md path to its
  `ContentSource` by longest path-prefix match (recommended, cheap, local), and
  skip+log if unowned. Alternative (heavier, rejected for blast radius): trigger
  a scoped `ingest_all_sources`. Recommend prefix-match.
- **Q2 — the legacy v1 loop.** Confirm `run_with_shutdown` is dead/legacy. If
  live on any platform, gating only v2 leaves a parity gap. Recommend: gate v2,
  comment v1, and file a **separate** "confirm/remove v1 loop" item rather than
  widening this task.
- **Q3 — markdown extension set.** `md` + `markdown` (recommended). `mdx` is not
  ingested today; exclude.

## 7. Definition of Done

- All §4 tests written first and green.
- `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings
  -D clippy::pedantic`; `cargo test --all-targets`; `cargo audit` all green.
- Code-file reindex path byte-identical (diff review confirms only additive
  markdown branch + gate helper).
- Width domain stays daemon-only: **no** CLI, schema, or config changes.
