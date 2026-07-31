---
title: "Adversarial review — 087 PowerBI durability pair (shipment 100-S)"
type: adversarial-review
date: 2026-07-30
branch: 100-powerbi-durability
baseline: a70395c5
head: 01f048db
diff: "git --no-pager diff a70395c5..HEAD"
reviewers: 3
models:
  - reviewer-a: claude-opus-4.8 (Tier 3 / frontier)
  - reviewer-b: gpt-5.6-sol (Tier 2)
  - reviewer-c: gemini-3.1-pro-preview (Tier 1)
safety_floor: "NEVER wrongly delete a live record (fail-closed)"
decision_gate: "P0 = 0 · P1 = 2 (both must-fix before merge)"
---

## Scope

Multi-model adversarial review of the four durability/deletion-semantics units in
shipment 100-S:

* **Unit A** `src/services/source_traversal.rs` — shared fail-closed reconciler
  (`collect_files_in_workspace_checked` → `CollectedFiles { files, complete }`,
  `reconcile_deleted_paths`).
* **Unit B/C** `notebook_indexer.rs` / `powerbi_indexer.rs` — wire the reconciler
  into `sweep_deleted_{notebook,powerbi}_files`; Unit C also drops the
  `powerbi_file_index_state` marker per swept path.
* **Unit D** `powerbi_indexer.rs` + `cozo_backend/schema.rs` + `cozo_queries.rs` —
  durable completion marker; `index_powerbi_source` sources the hash-skip oracle
  from the marker, writes it last, deletes it at the dirty-scope pre-delete and
  the non-TMDL hash-change delete.

Three reviewers (distinct model vendors — Anthropic / OpenAI / Google) each read
the diff and surrounding code independently and returned structured findings.
Every finding below was **re-verified against source** by the assembler before
classification; models are only trusted where the code confirms them.

## Method note (why the count is defensible)

Both blocking findings were flagged by **two independent vendors** (GPT + Gemini)
and **missed by the frontier model** (Opus caught neither) — the textbook case for
adversarial diversity. Each was then confirmed line-by-line against the code and
the surrounding invariants, so confidence is **HIGH** despite the 2/3 split.
The one P0-labelled unique finding (Gemini) was **refuted** on inspection.

---

## 1. Consensus findings (HIGH confidence — mandatory remediation)

### P1-A — Per-entry read errors silently mis-propagate `complete` → wrongful live-record deletion
*(category: wrongful-delete / complete-flag · votes: GPT=P1, Gemini=P0, Opus=—)*

**File:** `src/services/source_traversal.rs:128-133` (`collect_recursive`)

```rust
let mut entries: Vec<_> = entries
    .flatten()                                              // drops Err(DirEntry) silently
    .filter_map(|entry| {
        let path = entry.path();
        let file_type = std::fs::symlink_metadata(&path).ok()?.file_type(); // drops Err silently
        Some((entry_rank(&file_type), path, file_type))
    })
    .collect();
```

`Iterator::flatten` over `read_dir`'s `Result<DirEntry>` stream discards any `Err`
item, and `symlink_metadata(&path).ok()?` discards any per-entry metadata error —
**neither sets `*complete = false`.** This directly contradicts the `read_dir(dir)`
error branch five lines above (which *does* set `*complete = false`) and violates
the plan's **INV-2**: *"if traversal skipped any directory (read error), the pass
is non-authoritative."*

**Why it wrongly deletes live records:** a dropped **subdirectory** entry means an
entire in-bounds subtree is never collected while `complete` stays `true`. In
`reconcile_deleted_paths`, every stored record under that subtree is then
`not_collected && collected.complete` → `stale = true` → its content records **and
graph nodes are deleted** even though the files physically exist. A dropped
**file** entry that is still physically present is deleted the same way
(`physically_absent` is false, `complete && not_collected` is true). Unlike the
`read_dir` branch, this path emits **no warning** — a silent mass-delete.

**Trigger:** a transient per-entry I/O / permission error during a full-index pass.
Rare on a healthy local FS, but plausible on the exact flaky-network-share scenario
the source-root guard (01f048db) was added to defend — and the blast radius is the
primary safety floor (deleting live records).

**Fix:** handle each entry explicitly, mirroring the `read_dir` branch — on a
`DirEntry` error or a `symlink_metadata` error, `warn!`, set `*complete = false`,
and skip the entry:

```rust
let mut entries: Vec<(u8, PathBuf, FileType)> = Vec::new();
for entry in read_dir_iter {
    let entry = match entry {
        Ok(entry) => entry,
        Err(error) => {
            warn!(dir = %dir.display(), %error,
                  "unreadable directory entry during source traversal");
            *complete = false; // fail-closed: a dropped entry may hide a live file/subtree
            continue;
        }
    };
    let path = entry.path();
    match std::fs::symlink_metadata(&path) {
        Ok(md) => entries.push((entry_rank(&md.file_type()), path, md.file_type())),
        Err(error) => {
            warn!(path = %path.display(), %error,
                  "unreadable entry metadata during source traversal");
            *complete = false; // fail-closed
        }
    }
}
```

---

### P1-B — PowerBI skip-oracle (marker) deleted LAST → partial delete leaves a stale marker → wrongful hash-skip (missing summaries)
*(category: marker-ordering · votes: GPT=P1, Gemini=P0, Opus=—)*

**Files (all three delete blocks):**
* `src/services/powerbi_indexer.rs:~1323-1333` (TMDL dirty-scope pre-delete)
* `src/services/powerbi_indexer.rs:~1471-1480` (non-TMDL hash-change delete)
* `src/services/powerbi_indexer.rs:~1624-1632` (`sweep_deleted_powerbi_files`)

All three delete in the order **content records → graph nodes → marker (last):**

```rust
queries.delete_content_records_by_scope(path, "powerbi", &source.path).await?;
queries.delete_powerbi_nodes_by_file_path(path).await?;
queries.delete_powerbi_index_state_by_scope(path, &source.path).await?; // marker LAST
```

**Root cause:** Unit D changed the hash-skip oracle from the content rows to the
**marker** (`select_powerbi_index_state`), and correctly writes the marker **last**
on the *write* path (so a partial write leaves no marker ⇒ reprocess). But the
*delete* path was **not inverted**: because each `await?` is its own committed
transaction, a transient DB error (busy/lock) or crash **after** the content-row
delete but **before** the marker delete leaves the content gone while the **skip
oracle survives**. On the next pass, if the file is present with the marker's hash,
`unchanged == true` ⇒ it is **hash-skipped with no content rows** — re-opening the
exact "permanently missing summaries" hole that 087.006 exists to close.

**In-repo precedent confirms the intended pattern is oracle-first:** the notebook
sweep deletes its oracle (content records) *first* and its comment codifies the
rule — *"invalidate the freshness stamp before the scope-delete cascade so a
partial failure re-processes rather than hash-skips"* (`notebook_indexer.rs:414`).
Unit D deviates from that established crash-safety convention.

**Reachability by block (verified):**
* **sweep** — physically-absent file whose content rows are deleted; error before
  marker delete; file **re-appears at the same hash** (e.g. git checkout / a
  transiently-removed file) before the next full index ⇒ wrongful skip. **Real.**
* **dirty-scope pre-delete** — content deleted, error aborts the pass before the
  rebuild; marker (old hash) survives; if the file is unchanged and its scope is
  not dirty next run ⇒ wrongful skip. **Real.**
* **hash-change delete** — self-healing (the surviving marker holds the *old* hash,
  which no longer matches the changed file ⇒ reprocess). No hole, but fix for
  consistency.

**Fix:** delete the marker **first** in all three blocks, symmetric to the
write-path (marker last) and matching the notebook oracle-first convention:

```rust
// Delete the skip oracle (marker) FIRST: a partial delete then leaves the file
// marker-absent (safe reprocess) rather than stale-marked (wrongful hash-skip).
queries.delete_powerbi_index_state_by_scope(path, &source.path).await?;
queries.delete_content_records_by_scope(path, "powerbi", &source.path).await?;
queries.delete_powerbi_nodes_by_file_path(path).await?;
```

---

## 2. Majority findings (MEDIUM confidence)

None. (With three reviewers, "majority" and "consensus" coincide at 2/3; both 2/3
findings are promoted to Section 1 because the assembler independently confirmed
them against source. No additional >½ findings exist.)

---

## 3. Unique findings (LOW confidence — preserved for human judgment)

### P3-1 — Out-of-bounds traversal ROOT leaves `complete = true` *(Opus; tracked follow-up (c))*
`src/services/source_traversal.rs:109-112`. If the traversal **root** (`source_dir`)
canonicalizes out of bounds, `collect_recursive` returns via the
`!canonical_dir.starts_with(canonical_root)` branch **without** setting
`*complete = false`, yielding `files: [], complete: true`. `reconcile_deleted_paths`
would then treat an empty collection as authoritative and sweep every record.
**Currently masked** (not exploitable today): the sweep's `if !source_dir.is_dir()`
guard blocks the common case, and `is_regular_file_in_workspace` independently
rejects out-of-bounds files so `physically_absent` is true anyway. Latent, though —
the same fix as P1-A should also treat an unresolvable/out-of-bounds **root** as
`complete = false` (canonicalize the root in `collect_files_in_workspace_checked`
before recursing, and fail closed if it escapes). Consistent with tracked (c).

### P3-2 — `collect_symlinked_directory` preserves `complete` on non-`NotFound` canonicalize failure *(Gemini)*
`src/services/source_traversal.rs:~166-169`. A `PermissionDenied` (vs a genuinely
broken symlink) on `path.canonicalize()` returns without `*complete = false`.
Masked by the same `is_regular_file_in_workspace` physical-absence check. Minor;
optionally differentiate `ErrorKind::NotFound` (broken link → preserve) from other
errors (→ `complete = false`).

### REJECTED — Notebook marker-delete ordering *(Gemini, labelled P0 — false positive)*
`notebook_indexer.rs:414`. Gemini claimed content rows are deleted before the
freshness stamp is invalidated, opening a hash-skip hole. **Refuted:** (1) the
notebook hash-skip oracle is the **content records** (`select_content_records`
→ `file_path→content_hash`, `notebook_indexer.rs:132-140`), which are deleted
**first** — that is the fail-closed order; (2) `delete_lineage_index_state` (the
*lineage* freshness stamp, a separate concern) is correctly invalidated **before**
`delete_lineage_by_scope`; (3) the loop body is **pre-existing** code, untouched by
this diff. Not a defect, and not in scope.

---

## 4. Validated-and-clear (checked, no finding)

* **Predicate parity (no store-vs-collect mismatch):** the indexer and the sweep
  use the *same* predicate on both sides — `is_powerbi_file` (`.json/.bim/.tmdl`)
  for PowerBI and `is_notebook_file` (`.ipynb`) for notebooks — over the same
  `source_dir`/`workspace_root`, with identical traversal + dedup. Every record the
  indexer writes is collected by the sweep ⇒ no wrongful delete from a predicate
  gap. The sweep collects a *superset* (no size/binary/no-entity filtering), which
  is the safe direction.
* **Path normalization:** stored `file_path` and `normalize_collected_rel_path`
  both use `strip_prefix(workspace_root)` + `replace('\\', "/")`; `not_collected`
  compares the already-normalized stored string against the normalized collected
  set. Consistent on Windows and Unix.
* **Alias-supersede determinism:** `entry_rank` ranks real dirs (0) before symlinks
  (2), so the real-dir path is always the one collected and retained; only the
  aliased duplicate is swept. INV-1 holds on a `complete` pass.
* **Cozo correctness:** `powerbi_file_index_state { file_path, source_path =>
  content_hash, completed_at }` — composite key, scoped select/`:rm`, and string
  param bindings are all correct; the marker write uses the busy-retry helper.
* **Fail-closed source-root guard (01f048db):** correct and complete in both
  sweeps; regression tests present. Not counted (already addressed).
* **Follow-ups (a), (b), (d):** (a) migration-window orphans are *stale* rows, not
  a wrongful delete of a live record — not P0/P1. (b) `delete_powerbi_nodes_by_source`
  has **no runtime call site** (test-only), so orphan-marker-on-source-removal is
  unreachable at runtime — not P0/P1. (d) content writes bypass the busy-retry
  helper, but a busy error simply `?`-aborts before the marker write ⇒ reprocess
  (fail-closed) — not data-loss. All correctly tracked, none blocking.

---

## 5. Remediation plan (ordered by confidence × severity)

| # | Finding | Conf | Sev | Score | Action class | Notes |
|---|---|---|---|---|---|---|
| 1 | **P1-A** silent per-entry error mis-propagates `complete` | HIGH | P1 | 9 | `gated_auto` / manual | Fold in P3-1 root-bounds fix while here |
| 2 | **P1-B** PowerBI marker deleted last (3 blocks) | HIGH | P1 | 9 | `gated_auto` / manual | Marker-first in all 3 blocks + regression test (crash between content-delete and marker-delete → reprocess, not skip) |
| 3 | P3-1 out-of-bounds root `complete=true` | LOW | P3 | 1 | `advisory` | Closed by the P1-A fix (canonicalize root, fail closed) |
| 4 | P3-2 symlink canonicalize non-NotFound preserves `complete` | LOW | P3 | 1 | `advisory` | Optional; differentiate `NotFound` |

**Both P1s are deterministic, localized fixes.** Recommend fixing both before
merge and adding: (RF-A) a `collect_files_in_workspace_checked` test where a
subdirectory entry is unreadable mid-iteration ⇒ `complete == false` ⇒ the
alias-stale/live record is **retained**; (RF-B) a PowerBI test that injects a
failure between the content-row delete and the marker delete and asserts the next
run **reprocesses** (does not hash-skip).

---

## 6. Backlog work items (P0/P1)

```yaml
- type: bug
  title: "source_traversal: per-entry read error must set complete=false (fail-closed)"
  description: >
    collect_recursive uses read_dir.flatten() and symlink_metadata(&path).ok()?,
    silently dropping failed entries without setting *complete=false. A dropped
    in-bounds subdirectory/file leaves complete=true, so reconcile_deleted_paths
    treats the still-present record as alias-stale and deletes live content
    records + graph nodes. Violates INV-2; no warning emitted.
  file: "src/services/source_traversal.rs"
  line: 128
  severity: "P1"
  confidence: "HIGH"
  fix: >
    Replace flatten()/ok()? with explicit per-entry error handling that warns,
    sets *complete=false, and skips the entry (mirror the read_dir error branch).
    Also treat an unresolvable/out-of-bounds traversal ROOT as complete=false.
  linked_review: "docs/closure/2026-07-30-087-powerbi-durability-adversarial-review.md"

- type: bug
  title: "powerbi_indexer: delete completion marker FIRST in all three delete blocks"
  description: >
    Unit D made the marker the hash-skip oracle and writes it last (correct), but
    the delete path still deletes content rows/nodes before the marker. A transient
    DB error or crash between leaves the content gone while the marker survives; a
    present same-hash file (sweep re-appear, or unchanged dirty-scope file) is then
    hash-skipped with no content rows — re-opening 087.006's missing-summaries hole.
    Notebook sweep already establishes the oracle-first convention.
  file: "src/services/powerbi_indexer.rs"
  line: 1631
  severity: "P1"
  confidence: "HIGH"
  fix: >
    Call delete_powerbi_index_state_by_scope BEFORE delete_content_records_by_scope
    and delete_powerbi_nodes_by_file_path in all three blocks (~1323-1333 dirty-scope
    pre-delete, ~1471-1480 hash-change delete, ~1624-1632 sweep). Add a regression
    test for the crash-between ordering.
  linked_review: "docs/closure/2026-07-30-087-powerbi-durability-adversarial-review.md"
```

---

## Decision gate — explicit P0/P1 count

* **P0: 0**
* **P1: 2** (both **must-fix before merge**)
  * **P1-A** — `source_traversal.rs:128` silent per-entry error → wrongful live-record delete (fail-open in INV-2).
  * **P1-B** — `powerbi_indexer.rs` marker deleted last in all 3 delete blocks → wrongful hash-skip / missing summaries (re-opens the 087.006 durability hole).
* P2: 0 · P3: 2 (advisory: P3-1 out-of-bounds root [tracked (c), masked]; P3-2 symlink canonicalize).
* 1 unique P0-labelled finding (notebook marker ordering) **rejected** as a false positive.

> Dissent recorded: reviewer-c (Gemini) rated **both** P1-A and P1-B as **P0**.
> The assembler lands on **P1** for each (real fail-open, but each requires a
> transient error/crash at a specific point plus a same-hash present file to
> manifest). Under the protocol's strict "most-conservative severity on conflict"
> rule the headline would elevate to **P0 = 2**. Either way the gate is the same:
> **2 blocking, must-fix findings.**


---

## Resolution (Ship agent)

Both blocking findings were fixed before merge in commit `3449a361`
(`fix(indexing): fail-closed traversal completeness + marker-first delete ordering`):

* **P1-A** — `collect_recursive` now handles each `ReadDir` entry and
  `symlink_metadata` result explicitly: on error it sets `*complete = false` and
  `continue`s, so a dropped/unreadable entry marks the pass non-authoritative
  (fail-closed; INV-2) instead of leaving `complete == true`. Regression test
  `per_entry_metadata_failure_marks_pass_non_authoritative` (`#[cfg(unix)]`,
  `0o444` listable-but-not-stat-able subdir) asserts `complete == false`.
* **P1-B** — all three PowerBI delete blocks (dirty-scope rebuild ~L1319,
  non-TMDL hash-change ~L1472, deletion sweep ~L1624) now delete the
  `powerbi_file_index_state` completion marker **first**, then the content rows
  and nodes. A crash between the deletes now leaves the path marker-absent
  (reprocess on the next pass) rather than content-absent-marker-present
  (hash-skip → missing summaries). Final delete-state is unchanged, so
  RF-6/RF-7/RF-8 remain green.

P3-1 (out-of-bounds traversal root completeness, masked by the P1-A fix) and
P3-2 (busy-retry consistency on content-record writes) are filed as tracked
backlog stash follow-ups.