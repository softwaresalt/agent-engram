---
title: "Fail-closed teardown: delete the skip-oracle FIRST and mark an incomplete traversal non-authoritative — or a reconcile sweep wrongly deletes live records"
description: "A deletion/reconcile pipeline has two symmetric fail-open traps that both end in wrongful data loss. (1) If the WRITE path publishes a durable hash-skip oracle (a completion marker) LAST for crash-safety, the DELETE path must remove that oracle FIRST — otherwise a crash/DB error between the content delete and the marker delete leaves a stale marker that hash-skips a present same-hash file, permanently dropping its records. (2) A traversal that feeds a `complete`/authoritative flag must set complete=false on ANY per-entry read error; iterator adapters that swallow Result (`read_dir().flatten()`, `symlink_metadata(&p).ok()?`) silently drop failed entries while leaving complete=true, so a reconcile treats a still-present file/subtree as stale and deletes its live content + graph nodes."
problem_type: "logic_error"
category: "best-practices"
component: "src/services/source_traversal.rs"
root_cause: "Unit D made a durable completion marker the hash-skip oracle and correctly wrote it LAST (partial write ⇒ no marker ⇒ reprocess), but left the DELETE path oracle-last (content rows → nodes → marker), so a transient error/crash between committed statements left content gone with the skip-oracle surviving ⇒ wrongful hash-skip. Separately, collect_recursive used read_dir().flatten() and symlink_metadata(&path).ok()? which drop failed entries without setting *complete=false, so a dropped in-bounds subdirectory/file left complete=true and reconcile_deleted_paths deleted the still-present live record as alias-stale."
resolution_type: "code_fix"
severity: "high"
message: "delete_skip_oracle_first_and_fail_closed_traversal_completeness"
file_path: "src/services/powerbi_indexer.rs"
date: "2026-07-31"
feature: "087-F"
shipment: "100-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/304"
  - "docs/closure/2026-07-30-087-powerbi-durability-adversarial-review.md (P1-A per-entry completeness; P1-B marker-first ordering; 3 independent reviewers, both P1s flagged by 2 vendors and missed by the frontier model)"
  - "src/services/source_traversal.rs (collect_recursive: explicit per-entry error handling — on DirEntry err or symlink_metadata err, warn! + *complete=false + continue; mirrors the read_dir error branch; INV-2)"
  - "src/services/powerbi_indexer.rs (all three delete blocks now delete_powerbi_index_state_by_scope FIRST, then delete_content_records_by_scope + delete_powerbi_nodes_by_file_path: dirty-scope pre-delete ~L1319, non-TMDL hash-change ~L1472, sweep_deleted_powerbi_files ~L1624)"
  - "src/services/notebook_indexer.rs:414 (in-repo precedent: invalidate the freshness stamp BEFORE the scope-delete cascade so a partial failure reprocesses rather than hash-skips — the oracle-first convention Unit D initially deviated from)"
  - "fix commit 3449a361 fix(indexing): fail-closed traversal completeness + marker-first delete ordering"
  - "docs/compound/best-practices/certify-completeness-reconcile-fileset-and-sweep-orphans-2026-07-29.md (companion: reconcile the full input set before advancing a marker)"
tags:
  - "indexing"
  - "fail-closed"
  - "deletion-sweep"
  - "hash-skip-oracle"
  - "completion-marker"
  - "crash-safety"
  - "wrongful-delete"
  - "data-loss"
  - "traversal-completeness"
  - "iterator-error-swallow"
  - "toctou"
  - "adversarial-review"
  - "087-F"
  - "100-S"
---

## Context

Shipment 100-S (087 PowerBI durability pair, PR #304) hardened the deletion /
reconcile pipeline: a shared fail-closed reconciler
(`collect_files_in_workspace_checked` → `CollectedFiles { files, complete }`,
`reconcile_deleted_paths`) wired into the notebook and PowerBI deletion sweeps,
plus a durable PowerBI completion marker (`powerbi_file_index_state`) that became
the hash-skip oracle. A 3-model adversarial review (Opus / GPT / Gemini) found
**two** must-fix P1s — both fail-OPEN paths on the safety floor *"never wrongly
delete a live record."* Notably both blocking findings were raised by two
independent vendors and **missed by the frontier model**, and both were
re-verified against source before landing. The safety floor here is data loss, so
these graduate.

## Trap 1 — write-oracle-LAST but delete-oracle-LAST (must be delete-FIRST)

Making a durable marker the hash-skip oracle is only crash-safe if BOTH sides are
ordered around it:

- **Write path (correct):** write the marker **LAST**. A partial write that dies
  before the marker leaves the path marker-absent ⇒ reprocessed next pass.
- **Delete path (the bug):** the three PowerBI delete blocks deleted in the order
  `content rows → graph nodes → marker (LAST)`. Because each `await?` is its own
  committed statement, a transient busy/lock error or crash **after** the content
  delete but **before** the marker delete leaves the content gone while the skip
  oracle **survives**. Next pass, a present file at the marker's hash ⇒
  `unchanged == true` ⇒ **hash-skipped with no content rows** — re-opening the
  exact "permanently missing summaries" hole 087.006 was built to close.

```rust
// WRONG — oracle deleted last: crash-between leaves stale marker ⇒ wrongful hash-skip
queries.delete_content_records_by_scope(path, "powerbi", &source.path).await?;
queries.delete_powerbi_nodes_by_file_path(path).await?;
queries.delete_powerbi_index_state_by_scope(path, &source.path).await?; // LAST

// RIGHT — oracle deleted FIRST: crash-between leaves the path marker-absent ⇒ safe reprocess
queries.delete_powerbi_index_state_by_scope(path, &source.path).await?; // FIRST
queries.delete_content_records_by_scope(path, "powerbi", &source.path).await?;
queries.delete_powerbi_nodes_by_file_path(path).await?;
```

The notebook sweep already codified this: *"invalidate the freshness stamp before
the scope-delete cascade so a partial failure re-processes rather than
hash-skips"* (`notebook_indexer.rs:414`). Unit D deviated from the established
convention; the fix restored it in all three blocks.

> **Rule:** the value that gates "may I skip this?" is the LAST thing you write
> and the FIRST thing you delete. Publish durable state after its payload; retract
> it before its payload. Any teardown that removes the payload before the skip
> oracle is a crash window that hash-skips live data.

## Trap 2 — iterator adapters that swallow `Result` defeat a fail-closed `complete` flag

`collect_recursive` fed a `complete` flag that `reconcile_deleted_paths` trusts as
authoritative: `not_collected && collected.complete ⇒ stale ⇒ delete`. But it
built the entry list with adapters that **silently drop errors**:

```rust
// WRONG — flatten() drops Err(DirEntry); .ok()? drops metadata errors; neither sets complete=false
let mut entries: Vec<_> = read_dir(dir)?
    .flatten()
    .filter_map(|entry| {
        let path = entry.path();
        let ft = std::fs::symlink_metadata(&path).ok()?.file_type();
        Some((entry_rank(&ft), path, ft))
    })
    .collect();
```

A dropped in-bounds **subdirectory** entry means an entire subtree is never
collected while `complete` stays `true`; every stored record under it is then
`not_collected && complete ⇒ stale ⇒ deleted` even though the files physically
exist — a silent mass-delete with no warning. This contradicted the sibling
`read_dir(dir)` error branch (which *does* set `complete=false`) and violated
INV-2 (*"if traversal skipped any directory, the pass is non-authoritative"*).

```rust
// RIGHT — handle each entry explicitly; on any error warn + fail closed + skip
let mut entries: Vec<(u8, PathBuf, FileType)> = Vec::new();
for entry in read_dir(dir)? {
    let entry = match entry {
        Ok(e) => e,
        Err(error) => { warn!(dir=%dir.display(), %error, "unreadable dir entry");
                        *complete = false; continue; }   // fail-closed
    };
    let path = entry.path();
    match std::fs::symlink_metadata(&path) {
        Ok(md) => entries.push((entry_rank(&md.file_type()), path, md.file_type())),
        Err(error) => { warn!(path=%path.display(), %error, "unreadable entry metadata");
                        *complete = false; }              // fail-closed
    }
}
```

> **Rule:** any error that could hide an in-bounds file MUST flip the completeness
> flag the reconciler trusts. Audit `flatten()`, `filter_map(|x| ... .ok()?)`,
> `while let Ok(_)`, and `if let Ok(_)` on a `read_dir`/metadata stream — each is a
> place a `Result` is dropped, and a dropped entry on a delete/reconcile path is a
> wrongful-delete waiting for a transient I/O error to trigger it.

## Meta-learning

Both bugs are the same shape: a **teardown/reconcile path that fails OPEN** — it
proceeds to delete on incomplete or crash-torn state instead of refusing. Design
deletion pipelines so the default under uncertainty is *retain and reprocess*, and
verify both the ordering symmetry (oracle written-last / deleted-first) and the
completeness-flag propagation (every skipped/errored entry ⇒ non-authoritative).
Multi-vendor adversarial review earned its keep here: the frontier model caught
neither P1; two cheaper independent models did.
