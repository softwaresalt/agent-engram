---
date: 2026-07-04
type: adversarial-review
review_id: 072-S-ADV
target_commit: b3babb0ae6c0d95eba57651e538bcfefd11af89d
target_branch: 072-reactive-sync-verify-gate
task: 064.004-T
feature: 064-F
shipment: 072-S
title: Adversarial multi-model review — reactive markdown sync verify gate (DAEMON-EVENT-LOOP)
reviewers: 3
models: [gpt-5.4-mini (Tier 1), claude-sonnet-4.6 (Tier 2), gpt-5.5 (Tier 3)]
output_mode: full
verdict: APPROVE-WITH-FIXES
---

# Adversarial Review — 064.004-T reactive markdown sync verify gate

## Verdict: **APPROVE-WITH-FIXES**

All four hard, gate-blocking constraints (**C1** injectable/pure gate + no daemon-spin
tests, **C3** freeze-scope + cannot affect the Windows startup flake, **C4** fail-safe
no-panic log-and-continue, plus byte-identical code-reindex path / lock-safety /
`#![forbid(unsafe_code)]` / v1-untouched) are **SATISFIED**. No **P0** finding exists:
nothing here crashes the receive loop, corrupts the DB, or breaks the gate in the
common (non-racing) case.

However, three independent reviewers **unanimously** surfaced a cluster of genuine
correctness / robustness / hardening gaps in the new `src/services/reactive_sync.rs`
gate. One (source-type routing) is a real DB-consistency divergence with a trivial fix
and is treated as **P1 (required or explicitly-deferred-with-signoff)**. The remainder
are **P2/P3** — recommend fixing the cheap ones now; the rest may be deferred with
written rationale.

---

## 1. Scope of review

* **Commit** (local, unpushed): `b3babb0` on `072-reactive-sync-verify-gate`.
* **Files changed (5, +697/−12)** — confirmed via `git show --stat b3babb0`:
  `src/daemon/debounce.rs`, `src/daemon/ipc_server.rs`, `src/services/ingestion.rs`,
  `src/services/mod.rs`, `src/services/reactive_sync.rs` (new).
* **Commands used**: `git show b3babb0`, `git diff main..072-reactive-sync-verify-gate`,
  per-file `git diff … -- <path>`, plus direct source inspection of the DB / state /
  registry / verify internals.
* Reviewers were dispatched in parallel, each read the diff + supporting modules
  authoritatively, and returned structured JSON only. Aggregation and adjudication
  (below) were performed by the coordinator with independent verification of the
  highest-impact claims.

---

## 2. Hard-constraint verification (gate-blocking gates — all PASS)

| Constraint | Result | Evidence |
|---|---|---|
| **C1** — gate is pure/injectable; **tests MUST NOT spin the daemon** | ✅ PASS | Tests target `markdown_gate_decision`, `resolve_content_source`, and `verify_gated_reingest` against a temp CozoDB. `setup_db` = `connect_db(TempDir)` (`reactive_sync.rs` test mod ~L400). No test references `run_with_shutdown_v2`. `reingest_pending_markdown` is documented as intentionally un-unit-tested (`reactive_sync.rs:206-219`). |
| **C3** — freeze-scope; no startup/Ready/watcher/IPC-accept edits; cannot affect the Windows `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` SQLite startup flake | ✅ PASS | Commit touches only 5 files; within `ipc_server.rs` only the v2 auto-sync **task body** (`:1089-1188`) + a v1 comment (`:639-642`). No edits to startup, Ready signalling, watcher init, or `accept_loop` ordering. The flake lives in `tests/integration/daemon_startup_order_test.rs:46` (a startup/TTL path that generates **no file events**), so the new code path is never exercised by it. The second claimed flake `c017_03_agents_have_required_subfields` lives in `tests/contract/evaluation_contract_test.rs:150` — **neither test file is in the commit's changed-file set**, so both flakes are pre-existing and untouched. |
| **C3** — reindex `'sync:` block behaviorally identical to `main` except the one documented guard line | ✅ PASS | `git diff` hunk confirms the **only** additions inside the block are the 2-line comment and `if !pending_reindex { break 'sync false; }` (inserted as the first statement). **Zero removed lines**; the `snapshot`/`ws_config` guards, the `sync_workspace(...)` call, its `Ok`/`Err` arms, and the `true`/`false` returns are all unchanged context. No code-file reindex regression. |
| **C4** — fail-safe: no `panic!`/`unwrap()`/`expect()` in production; fallible ops → `Result<_,EngramError>` via `?`; log-and-continue; receive loop never breaks | ✅ PASS | Production paths use `strip_prefix(...).unwrap_or(file_path)` (not `unwrap`), `map_err`, and `?`. `reingest_pending_markdown` returns `()` and `match`es every error (registry load, DB connect, per-file) into `warn!`+continue (`reactive_sync.rs:231-267`). `verify_gated_reingest` returns `Result` matched by the caller. `verify_markdown` is I/O-free and has no OOB slice (`{{` handling is guarded by `find`). All `unwrap`/`expect`/`panic!` occurrences are inside `#[cfg(test)]`. |
| Lock never leaked | ✅ PASS | `try_start_indexing()` (global CAS, `state.rs:365`) is followed unconditionally by `finish_indexing_and_drain_pending_sync(...)` (`ipc_server.rs:1178`) with no `?`/early-return/panic between; `reingest_pending_markdown` is infallible (`-> ()`). |
| Dedup `BTreeSet` correctness | ✅ PASS | `pending_reingest` is created fresh per debounce batch (`:1095`), dedups by path (repeated saves collapse to one gate), deterministic order, and drops only exact-duplicate paths (intended). Distinct events are never merged. |
| v2-only, v1 untouched, additive, `#![forbid(unsafe_code)]` intact | ✅ PASS | v1 loop got a comment only (`:639-642`); no `unsafe`; reuses the existing `ReingestContent` variant. Only one loop runs live (v2 via `daemon/mod.rs:202`) — no double-processing between v1/v2. |

---

## 3. Findings by consensus tier

Confidence: **HIGH** = flagged by all 3 reviewers · **MEDIUM** = majority (2/3) ·
**LOW** = single reviewer (coordinator-verified where noted).
Severity per protocol takes the most conservative reviewer rating.

### 3.1 Consensus findings (confidence HIGH — flagged by all 3 reviewers)

#### C-1 — TOCTOU double-read gate window (MAJOR · P2)
`src/services/reactive_sync.rs:171` + `src/services/ingestion.rs:684`
`verify_gated_reingest` reads the file once via `tokio::fs::read_to_string` for the
verify gate, then `ingest_single_file` **re-reads it from disk** via `std::fs::read`.
Between the two reads the file can change: a document that is *conformant at gate time*
but mutated to *non-conformant* before the ingest read is **written unverified**,
violating the gate's core promise. A file deleted between reads makes
`ingest_single_file` take its delete branch (`ingestion.rs:668-672`, returns `Ok(true)`)
while `verify_gated_reingest` still reports `Ingested`.
*Adjudication:* real gate-bypass, but the window is microseconds and self-correcting —
the subsequent write emits a fresh watch event that re-gates the file. **Not
gate-blocking**, but must be acknowledged.
*Fix:* thread the already-verified content into ingestion (or re-verify the exact bytes
`ingest_single_file` reads) so the gated bytes and ingested bytes are identical; **or**
document as an accepted known limitation with the self-correction rationale.

#### C-2 — Source-type exclusion is incomplete (notebook/powerbi/pbip) (MAJOR · **P1**)
`src/services/reactive_sync.rs:109`
`resolve_content_source` excludes only `content_type == "code" | "backlog"`. A `.md`
under a `notebook` / `powerbi` / `pbip` source directory therefore resolves as owned and
is ingested through the **generic** `ingest_single_file` content path with that source's
`content_type`. But the **startup** path (`ingest_all_sources`, `ingestion.rs:110-159`)
routes those exact `content_type`s to **dedicated indexers** that never call
`ingest_single_file` for markdown. Result: the reactive path writes `content_record`
rows the startup path never creates → DB diverges after a daemon restart.
*Adjudication:* genuine correctness divergence, flagged by all three reviewers, with a
trivial fix. The plan's C2 only reasoned about `code`/`backlog`; Ship self-flagged this
as open risk (a) and it was never ratified. **Treat as required**, or defer only with
explicit operator sign-off + a documented known limitation.
*Fix:* invert to an **allowlist** of content-types that legitimately use the generic
markdown path, **or** extend the exclusion set to `notebook`/`powerbi`/`pbip` (mirror
the routing in `ingest_all_sources`).

#### C-3 — Workspace-containment / path-safety hardening gap (MAJOR · P2)
`src/services/reactive_sync.rs:101-129, 157-168`
`resolve_content_source` matches by a **purely lexical** `starts_with(format!("{src}/"))`
with **no canonicalization** and **no `..` resolution**; an empty/root source path
(`src.is_empty()`) owns *every* path unconditionally; and `verify_gated_reingest` uses
`strip_prefix(workspace_root).unwrap_or(file_path)`, silently retaining an **absolute**
path on failure. It also never checks `source.status`, so a `Missing`/`Error` source
(which `validate_sources` would down-status, e.g. a symlink escape) still matches. Under
a root source + a symlink/junction inside the workspace, `read_to_string` + ingest could
touch a file outside the workspace root.
*Adjudication:* real defense-in-depth gap, but **low real-world exploitability** — watch
events originate from the OS watcher on the trusted, user-owned workspace, and the same
`strip_prefix(...).unwrap_or(file_path)` pattern already exists pre-change in
`ingest_single_file` (`ingestion.rs:650-652`), so this is **not a new vulnerability
class**. **Not gate-blocking** for a local single-user daemon (impact is a local CozoDB
write, no exfiltration).
*Fix:* filter to `ContentSourceStatus::Active` in `resolve_content_source`; canonicalize
`file_path` and reject any target whose canonical path is not under the canonical
workspace root; on `strip_prefix` failure return `SkippedUnowned` instead of falling back
to the absolute path.

#### C-4 — Full read precedes the size gate (oversize/DoS) (MAJOR · P2)
`src/services/reactive_sync.rs:171`
`tokio::fs::read_to_string` loads the entire file into memory and `verify_markdown` scans
all of it **before** the `max_file_size` check, which lives *inside* `ingest_single_file`
(`ingestion.rs:680`) and is reached only after the full read. A large `.md` dropped under
a watched directory (e.g. a generated log) is fully buffered + scanned every watcher
event, then rejected. Not a crash unless OOM, but an unnecessary memory/latency spike on
the daemon hot path.
*Fix:* `tokio::fs::metadata(...).len()` vs `config.max_file_size_bytes` **before**
`read_to_string`; skip early when exceeded.

#### C-5 — Case-sensitive markdown extension match (MINOR · P3)
`src/daemon/debounce.rs:132-135`
`MARKDOWN_EXTENSIONS.contains(&ext)` is case-sensitive, so `README.MD` /
`Guide.Markdown` never emit `ReingestContent` on case-insensitive filesystems
(Windows/macOS). Consistent with the pre-existing case-sensitive `INDEXED_EXTENSIONS`
(`.rs`/`.toml`), so not a regression — but newly observable for markdown.
*Fix:* compare with `eq_ignore_ascii_case` (ideally normalize both extension sets).

### 3.2 Majority findings (confidence MEDIUM — 2/3 reviewers)

#### M-1 — Inactive-source status not filtered (MAJOR · P2)
`src/services/reactive_sync.rs:108`
`resolve_content_source` iterates all sources without checking `source.status`, whereas
`ingest_all_sources` skips `Missing`/`Error` sources first (`ingestion.rs:58-66`). Folds
into C-3; called out separately because the one-line `status == Active` guard is the
cheapest single mitigation for both the parity gap and the containment concern.
*Fix:* `if source.status != ContentSourceStatus::Active { continue; }` at the top of the
loop.

### 3.3 Unique findings (confidence LOW — single reviewer)

#### U-1 — Content writes bypass the SQLITE_BUSY retry wrapper (MINOR · P2) — **coordinator-verified real**
`src/db/cozo_queries.rs` (`upsert_content_record` ~L2497, `delete_content_records_by_scope`
~L2725, `select_content_records` ~L2541)
Content-record mutations use bare `self.db.run_script(...)`, **not** the
`run_script_busy_retry_mutable` wrapper that code-graph symbol writes use (e.g.
`upsert_function` at `cozo_queries.rs:973`). Verified independently. A transient
`SQLITE_BUSY` from a concurrent MCP-request DB handle makes a reactive reingest fail →
`warn!` + drop until the next file event. This is a pre-existing property of the content
path now placed on the reactive hot path; log-and-continue keeps it fail-safe, so it does
not violate C4.
*Fix (optional):* route content-record mutations through the existing busy-retry wrapper,
or re-queue the path on transient busy.

#### U-2 — `ingest_single_file` bool return ignored → misleading outcome/logs (MINOR · P3)
`src/services/reactive_sync.rs:190-201`
`ingest_single_file` returns `Ok(false)` for glob-filtered, oversize, binary, or
already-current files (`ingestion.rs:663/681/690/712`). `verify_gated_reingest` ignores
the bool and always returns `ReingestOutcome::Ingested`, so `reingest_pending_markdown`
over-counts `ingested` and emits a misleading "reactive markdown reingest complete" info
log. No data impact — cosmetic/observability.
*Fix:* map `Ok(false)` to a `Skipped*` outcome so counters/logs are accurate.

#### U-3 — Silent drop of the batch when the workspace snapshot is `None` (MINOR · P3)
`src/daemon/ipc_server.rs:1165-1176`
If `snapshot_workspace()` returns `None` inside the reingest block (workspace teardown /
concurrent unbind between the outer CAS and the inner snapshot), the whole
`pending_reingest` batch is discarded with no diagnostic. No lock leak (finish still
runs); consistent with the reindex path's drop-on-contention behavior.
*Fix:* add a `warn!(dropped = pending_reingest.len(), …)` on the `None` branch.

---

## 4. Adjudication of the 5 Ship self-flagged risks

| Ship risk | Verdict | Rationale |
|---|---|---|
| **(a)** exclusion only code+backlog | **Real gap → C-2 (P1)** | notebook/powerbi/pbip use dedicated indexers; reactive path writes divergent `content_record` rows. Fix (allowlist) is trivial. |
| **(b)** double-read TOCTOU | **Real but low-severity → C-1 (P2)** | Narrow race, self-correcting via the next watch event. Acknowledge; fix or defer-with-rationale. |
| **(c)** fresh `connect_db` per batch | **Acceptable** | Sequential *after* the code-graph handle is dropped, inside the `try_start_indexing` critical section; identical lifecycle to `sync_workspace` (`code_graph.rs:640`). Serialized by `DB_OPEN_LOCKS` + advisory file lock. `CozoDb`/`queries` drop at function end — no handle leak. Extra `run_schema_bootstrap` per batch is cost, not correctness. |
| **(d)** case-sensitive `md`/`markdown` | **Real minor gap → C-5 (P3)** | Consistent with pre-existing code-file behavior; cheap `eq_ignore_ascii_case` fix. |
| **(e)** single atomic commit (prod+tests) | **Acceptable / good practice** | Impl + tests landing together is desirable; no finding. |

---

## 5. Concurrency / DB adjudication (angle 2)

* **Not concurrent with code-graph sync.** Within one loop iteration the `'sync:` block
  runs `sync_workspace` (which opens and *drops* its own `connect_db` handle,
  `code_graph.rs:640`) **before** the reingest block opens a fresh `connect_db`. Both run
  inside a single `try_start_indexing` (global CAS) critical section → **strictly
  sequential**, no WAL writer-vs-writer contention *between these two*.
* **vs. MCP request handlers.** Reingest writes can still race concurrent MCP-held DB
  handles → possible `SQLITE_BUSY`; content writes lack the busy-retry wrapper (U-1).
  Fail-safe (log + continue), self-heals on next event.
* **vs. TTL/shutdown.** TTL is reset per *received* event, not during processing — same as
  `sync_workspace`. A long reingest batch does not reset TTL mid-run, but SQLite writes
  are per-`run_script` transactional, so a mid-batch process exit leaves a partial batch
  that the next startup sync reconciles. No corruption.
* **Durability without flush.** Content records persist immediately via `run_script`;
  `should_flush` gates only the code-graph state flush, so reingested content is durable
  even when `should_flush == false`. ✅

---

## 6. Test adequacy (angle 6)

* **29 tests** (15 in `debounce.rs`, 14 in `reactive_sync.rs`) — count confirmed.
* Assertions are **substantive, not tautological**: `valid_markdown_is_ingested` asserts
  `Ingested` **and** `select_content_records(Some("docs"))` non-empty; skip tests assert
  the outcome **and** an empty record set; the malformed test asserts a specific
  `frontmatter.malformed` finding; `resolve_*` cover longest-prefix, code/backlog
  exclusion, and backslash normalization.
* **"No daemon spin-up" claim: TRUE** (C1) — verified above.
* **Two claimed pre-existing flakes are NOT introduced by this change** — verified by
  changed-file set: neither `tests/integration/daemon_startup_order_test.rs` nor
  `tests/contract/evaluation_contract_test.rs` is touched by `b3babb0`; the TTL-expiry
  test exercises no file events and the change adds nothing to startup.
* **Coverage gaps** (observations, not blockers): no test for the outer-guard
  no-regression (only-markdown → `sync_workspace` skipped), the TOCTOU window (C-1),
  case-insensitivity (C-5), oversize precheck (C-4), path traversal / non-Active source
  (C-3/M-1), or `reingest_pending_markdown`'s fail-safe paths (missing registry / bad DB
  connect). The byte-identical `'sync:` diff (not a test) is what defends "no code-reindex
  regression," which is acceptable per plan.

---

## 7. Remediation plan (sorted by priority = confidence × severity)

| # | Finding | Conf | Sev | Priority | P | Action class | Fix |
|---|---|---|---|---|---|---|---|
| C-1 | TOCTOU double-read | HIGH | MAJOR | 9 | P2 | `manual` | Thread verified content into ingest, or defer-with-rationale |
| C-2 | Source-type exclusion (notebook/powerbi/pbip) | HIGH | MAJOR | 9 | **P1** | `gated_auto` | Invert to allowlist / extend exclusion set |
| C-3 | Workspace-containment / path safety | HIGH | MAJOR | 9 | P2 | `manual` | Active-filter + canonicalize + reject strip_prefix failure |
| C-4 | Full read before size gate | HIGH | MAJOR | 9 | P2 | `gated_auto` | `metadata().len()` precheck before `read_to_string` |
| M-1 | Inactive-source not filtered | MED | MAJOR | 6 | P2 | `gated_auto` | `status == Active` guard (also mitigates C-3) |
| C-5 | Case-sensitive md extension | HIGH | MINOR | 6 | P3 | `advisory` | `eq_ignore_ascii_case` |
| U-1 | Content writes bypass busy-retry | MED* | MINOR | 4 | P2 | `advisory` | Route through `run_script_busy_retry_mutable` |
| U-2 | Ignored ingest bool → wrong outcome | LOW | MINOR | 2 | P3 | `advisory` | Map `Ok(false)` → `Skipped*` |
| U-3 | Silent drop on snapshot `None` | LOW | MINOR | 2 | P3 | `advisory` | `warn!` on the `None` branch |

\* U-1 raised from LOW→MEDIUM by independent coordinator verification.

### Required to merge (P1)
* **C-2** — resolve notebook/powerbi/pbip routing, **or** obtain explicit operator
  sign-off to ship as a documented known limitation.

### Strongly recommended (cheap, high-value — fix now or defer with written rationale)
* **M-1 + C-4** — the two-line `status == Active` guard and the metadata size precheck
  together close the cheapest slices of C-3, C-4, and M-1.
* **C-1 / C-3** — acknowledge explicitly (fix or defer-with-rationale).

### Advisory (P3 / optional)
* C-5, U-1, U-2, U-3.

---

## 8. Backlog work items (P0/P1 + top consensus P2s)

```yaml
- type: bug
  title: "reactive_sync: exclude notebook/powerbi/pbip content-types from generic markdown reingest"
  description: "resolve_content_source excludes only code+backlog; a .md under a notebook/powerbi/pbip source is ingested via generic ingest_single_file, creating content_record rows the startup dedicated indexers never create -> DB diverges after restart."
  file: "src/services/reactive_sync.rs"
  line: 109
  severity: "MAJOR"
  confidence: "HIGH"
  priority: "P1"
  fix: "Invert to an allowlist of generic-content content-types, or extend the exclusion set to notebook/powerbi/pbip (mirror ingest_all_sources routing)."
  linked_review: "docs/closure/2026-07-04-reactive-sync-verify-gate-adversarial-review.md"

- type: bug
  title: "reactive_sync: close TOCTOU between verify read and ingest re-read"
  description: "verify_gated_reingest reads content for verify, then ingest_single_file re-reads from disk; a file mutated conformant->non-conformant between reads is ingested unverified; a deleted file is reported Ingested."
  file: "src/services/reactive_sync.rs"
  line: 171
  severity: "MAJOR"
  confidence: "HIGH"
  priority: "P2"
  fix: "Thread the verified content into ingestion (or re-verify the exact bytes ingest reads); otherwise document as an accepted, self-correcting known limitation."
  linked_review: "docs/closure/2026-07-04-reactive-sync-verify-gate-adversarial-review.md"

- type: bug
  title: "reactive_sync: workspace-containment hardening (Active-filter + canonicalize)"
  description: "resolve_content_source uses purely lexical prefix match with no canonicalization/.. resolution; empty source owns everything; strip_prefix fallback retains absolute paths; source.status is never checked. Under a root source + workspace symlink, read/ingest could touch a file outside the workspace root."
  file: "src/services/reactive_sync.rs"
  line: 101
  severity: "MAJOR"
  confidence: "HIGH"
  priority: "P2"
  fix: "Filter to ContentSourceStatus::Active; canonicalize file_path and reject targets not under the canonical workspace root; return SkippedUnowned on strip_prefix failure instead of falling back to the absolute path."
  linked_review: "docs/closure/2026-07-04-reactive-sync-verify-gate-adversarial-review.md"

- type: bug
  title: "reactive_sync: check max_file_size before read_to_string (oversize DoS)"
  description: "tokio::fs::read_to_string loads the full file and verify_markdown scans all of it before the max_file_size check inside ingest_single_file; a large watched .md is fully buffered every watcher event before rejection."
  file: "src/services/reactive_sync.rs"
  line: 171
  severity: "MAJOR"
  confidence: "HIGH"
  priority: "P2"
  fix: "tokio::fs::metadata(...).len() vs config.max_file_size_bytes before read_to_string; skip early when exceeded."
  linked_review: "docs/closure/2026-07-04-reactive-sync-verify-gate-adversarial-review.md"
```

---

## 9. Reviewer raw signal (audit trail)

* **Reviewer-A (Tier 1, gpt-5.4-mini):** TOCTOU, source-routing, workspace-containment,
  case-sensitive ext, oversize read.
* **Reviewer-B (Tier 2, claude-sonnet-4.6):** TOCTOU (+delete mislabel), oversize read,
  workspace-containment (+root-source/strip_prefix detail), source-type exclusion,
  ignored ingest bool, case-sensitive ext, inactive-source-not-filtered, silent drop on
  snapshot `None`.
* **Reviewer-C (Tier 3, gpt-5.5):** path-containment, source-routing, TOCTOU, oversize
  read, case-sensitive ext, SQLITE_BUSY (content writes not retry-wrapped).

Agreement across all three on TOCTOU, source-type exclusion, path-containment, oversize
read, and case-sensitivity is the strongest signal in this report.

---

## 10. Bottom line

**APPROVE-WITH-FIXES.** The change is disciplined: it honors freeze-scope, keeps the
code-reindex path byte-identical, is fail-safe by construction, and cannot perturb the
known Windows startup flake. Ship one required fix (**C-2**, or a signed-off deferral),
land the two cheap hardening one-liners (**M-1** + **C-4**), and explicitly
acknowledge **C-1** and **C-3** (fix or defer-with-rationale). None of the findings is a
P0 and none blocks on constraint grounds.

---

## 11. Ship remediation log (post-review)

Applied on branch `072-reactive-sync-verify-gate` in a follow-up `fix:` commit. All
edits confined to `src/services/reactive_sync.rs` (+ its `#[cfg(test)]` module); C1/C3/C4
kept intact (no daemon-spin tests, freeze-scope preserved, fail-safe log-and-continue).

| ID | Finding | Disposition | Detail |
|----|---------|-------------|--------|
| **R1** | C-2 (P1) source-type allowlist | **APPLIED** | `resolve_content_source` now excludes the full dedicated-indexer set via `const DEDICATED_INDEXER_TYPES = ["code","backlog","notebook","powerbi","pbip"]`, a faithful mirror of `ingest_all_sources` routing. Expressed as exclusion (not an enumerated positive allowlist) so custom/non-built-in generic types the startup path *does* ingest are not wrongly skipped. Tests: `resolve_excludes_dedicated_indexer_sources` (pure), `markdown_under_dedicated_indexer_source_is_not_ingested` (e2e → `SkippedUnowned`, no `content_record`). |
| **R2** | C-3/M-1 (P2) active-source guard | **APPLIED** | `resolve_content_source` skips any source whose `status != ContentSourceStatus::Active` (Missing/Error/Unknown), mirroring the startup Missing/Error skip. Scoped narrowly to the active-status guard per operator directive (no path-containment canonicalization). Test: `resolve_skips_inactive_source` (pure). |
| **R3** | C-4 (P2) size precheck | **APPLIED** | `verify_gated_reingest` runs a `tokio::fs::metadata().len()` precheck against `max_file_size_bytes` before `read_to_string`, returning the new `ReingestOutcome::SkippedOversize` (counted as skipped by the orchestrator) so oversize files are never buffered. Test: `oversize_markdown_is_skipped_by_metadata_precheck` (e2e). |
| **R4** | U-1 (P2) SQLITE_BUSY retry wrapper | **DEFERRED** | Content-record mutations (`upsert_content_record`, `delete_content_records_by_scope` in `db/cozo_queries.rs`) use bare `run_script`, not `run_script_busy_retry_mutable`. These are shared query methods on the code-graph/CLI startup ingest paths; routing them through the busy-retry wrapper is **not** a clean swap in the reingest write path — it broadly changes shared ingestion behaviour and is outside this task's freeze-scope. Per operator directive ("do NOT force it if invasive"), deferred to a backlog item. Fail-safe still holds: a `SQLITE_BUSY` surfaces as `EngramError`, is logged-and-continued, and a later watcher event re-processes the file. |
| **R5** | C-1 (TOCTOU) + C-5 (case-sensitive ext) | **ACCEPTED (known)** | **C-1:** the verify-read and ingest-read are separate reads; a mutation between them is self-correcting because the next watcher event re-processes the path. Threading verified bytes into ingestion would require changing the shared `ingest_single_file` signature (freeze-scope violation). **C-5:** `md`/`markdown` extension matching is case-sensitive, deliberately consistent with the existing `INDEXED_EXTENSIONS` behaviour; changing only the markdown set to case-insensitive would introduce an inconsistency. Both accepted, no code change. |

**Suggested backlog follow-up (from R4):** route reactive/generic content-record writes
through `run_script_busy_retry_mutable` (or make the shared content-record mutations
busy-retry-aware) to align with code-graph write hardening.

**Post-fix gate results (CI feature set `--no-default-features --features
cozo-backend,embeddings`):** `fmt --check` clean; `clippy --all-targets -D warnings -D
clippy::pedantic` clean; `test --all-targets` green except the proven pre-existing
`c017_03_agents_have_required_subfields` cross-test-pollution flake (passes in isolation;
its test file is not in this diff). The `reactive_sync` module is 18/18 green (14 prior +
4 new).
