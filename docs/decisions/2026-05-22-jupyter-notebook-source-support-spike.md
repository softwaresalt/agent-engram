---
title: "Jupyter notebook source support spike"
type: spike
date: 2026-05-22
time_box: "2h"
conclusion: "proceed-to-feature"
confidence: "high"
linked_parent_work_item: "063-F"
promoted_to: ["feature"]
tags:
  - "jupyter"
  - "ipynb"
  - "notebook"
  - "search"
  - "memory"
  - "indexing"
  - "multi-language"
---

## Goal

Decide whether `.ipynb` should enter engram as a dedicated JSON-backed source,
and if so, define the smallest repo-aligned source model that answers the open
questions left in `009-D`:

* notebook source model
* language-detection precedence across kernel metadata, `language_info`, and magics
* explicit v1 non-goals
* minimum fixture matrix and 2-hour task slices

## Repo Evidence

The current repository already gives four strong design anchors.

1. `ContentRecord` supports more than one retrieval unit per physical file via
   `record_kind`, `chunk_id`, `chunk_index`, `line_start`, and `line_end`
   (`src/models/content.rs`)
2. ingestion already reserves dedicated branches for special content types such
   as `backlog` and `powerbi` instead of forcing every format through generic
   whole-file ingestion (`src/models/registry.rs`, `src/services/ingestion.rs`)
3. the code graph path derives language from file extension, and `.ipynb` is not
   a mapped language-bearing extension (`src/services/code_graph.rs`)
4. the Power BI path shows the local precedent for a JSON-backed domain source
   with its own collector, extractor, and dispatch tests
   (`src/services/powerbi_indexer.rs`,
   `tests/integration/powerbi_source_dispatch_test.rs`,
   `tests/integration/powerbi_search_ingestion_test.rs`)

Repo search also found no tracked notebook fixtures or notebook-specific source
path today outside stash `52102117` / deliberation `009-D`, so this work starts
from a true gap rather than an in-flight implementation.

## Findings

### 1. Notebook source model

Recommend a new dedicated `notebook` content type.

Treat each `.ipynb` as one JSON-backed container source and emit derived
retrieval records per author-written cell instead of inventing synthetic files.
That matches the repository better than pushing notebooks into the code graph,
because the generic content path already supports multiple records per file.

Recommended v1 record shape:

* one notebook-level summary record for notebook metadata plus a short content
  summary
* one derived record per markdown or code cell
* stable `chunk_id` values such as `cell-0001`, `cell-0002`, ...
* `chunk_index` equal to the cell ordinal
* `record_kind` values that distinguish notebook summary vs cell records
* `file_path` kept at the real `.ipynb` path so all notebook content stays under
  one physical source identity

This keeps notebook support aligned with the existing `ContentRecord` model and
avoids pretending notebook cells are normal source files.

### 2. Language-detection precedence

Recommended precedence for each code cell:

1. recognized cell magic wins
2. `metadata.language_info.name`
3. `metadata.kernelspec.language`
4. fallback to `unknown`

For the v1 whitelist, recognize at least:

* `%%sql` / `%sql` -> `sql`
* `%%scala` -> `scala`
* `%%sparkr` -> `sparkr`
* `%%python` -> `python`

Treat PySpark as `python` in v1 unless a stronger magic overrides it.

Rationale: notebook-level metadata is only a default for the file, while magics
are the only per-cell signal strong enough to justify switching languages inside
one notebook.

### 3. Explicit v1 non-goals

V1 should not attempt to solve the whole notebook ecosystem.

Explicit non-goals:

* outputs, execution counts, display payloads, attachments, and widget state
* arbitrary magic parsing beyond the small explicit whitelist above
* symbol extraction from notebook cells through the code graph pipeline
* cross-cell execution semantics or variable lineage
* notebook-specific graph edges in the first slice
* malformed notebook repair, nbformat migration, or notebook rewriting

This keeps the first delivery focused on author-written content for search and
memory rather than runtime notebook state.

### 4. Minimum fixture matrix

Use the smallest fixture matrix that proves the routing rules.

| Fixture | Coverage | Why it exists |
|---|---|---|
| `python_markdown.ipynb` | markdown + normal Python cells | baseline notebook summary and per-cell chunking |
| `sql_magic.ipynb` | Python default + `%%sql` | proves magic beats notebook default |
| `scala_magic.ipynb` | Python default + `%%scala` | proves Scala override without a Scala notebook default |
| `sparkr_magic.ipynb` | Python default + `%%sparkr` | proves SparkR override |
| `metadata_fallback.ipynb` | no magic, `language_info` differs from kernel | proves `language_info` beats kernel metadata |

A separate PySpark fixture is not required for v1 because the proposed routing
still treats baseline PySpark cells as Python unless a cell-local override is
present.

### 5. 2-hour task slices

These are the smallest credible follow-on slices.

1. **Source registration and dispatch**
   * add `notebook` as a built-in content type
   * add dispatch coverage proving it does not route through `code` or `backlog`
2. **Fixture and extractor harness**
   * add the minimum notebook fixture matrix
   * add tests for notebook summary records, cell chunk IDs, and precedence rules
3. **Notebook content-record indexing**
   * add a dedicated notebook collector/indexer that reads `.ipynb`
   * emit notebook-level and per-cell `ContentRecord` rows, excluding outputs
4. **Magic precedence and language routing**
   * implement the whitelist above and tag cell records with the resolved
     language
5. **Post-index documentation**
   * document the `notebook` source type, supported notebook surfaces, and the
     deliberate v1 non-goals

Each slice stays within one concern and fits the 2-hour rule better than a
single end-to-end notebook feature task.

## Recommendation

Proceed.

The spike evidence is strong enough to promote notebook support into a parent
feature now, which is tracked as `063-F`. The next Stage-side gate should be an
implementation plan and review that turns the slices above into harvested child
tasks and, after that, a Ship-ready shipment.

## Evidence References

* `src/models/content.rs`
* `src/models/registry.rs`
* `src/services/ingestion.rs`
* `src/services/code_graph.rs`
* `src/services/powerbi_indexer.rs`
* `tests/integration/powerbi_source_dispatch_test.rs`
* `tests/integration/powerbi_search_ingestion_test.rs`
* `.backlogit/queue/009-D.md`
* `.backlogit/queue/063-F.md`
