---
title: "Jupyter notebook source support"
type: impl-plan
date: 2026-05-23
status: reviewed
review_verdict: PASS
source_documents:
  - docs/decisions/2026-05-22-jupyter-notebook-source-support-spike.md
related_backlog:
  - 063-F
---

## Problem Frame

Notebook support belongs in content ingestion, not the code graph.

Repo evidence:

* `src/models/content.rs` and `src/services/ingestion.rs` already support multiple retrieval records per file through `record_kind`, `chunk_id`, `chunk_index`, and line metadata
* `src/services/ingestion.rs` already routes dedicated source types such as `backlog` and `powerbi`
* `src/services/code_graph.rs` does not treat `.ipynb` as a language-bearing source file
* `src/tools/read.rs` already exposes `ContentRecord` rows through `query_memory`

The spike for `063-F` recommends:

* dedicated `notebook` content type
* one `.ipynb` container source with notebook summary plus per-cell records
* language precedence: magic > `language_info.name` > `kernelspec.language` > `unknown`
* v1 non-goals: outputs, execution state, arbitrary magic parsing, notebook graph edges, code-graph symbol extraction

## Requirements Trace

| Requirement | Units |
|---|---|
| Add `notebook` as a first-class source type | 1 |
| Route notebooks through a dedicated indexer | 1, 3 |
| Emit notebook summary plus per-cell records | 2, 3 |
| Preserve stable `chunk_id` / `chunk_index` per cell | 2, 3 |
| Resolve language with spike precedence | 2, 4 |
| Exclude outputs and execution state | 2, 3, 4 |
| Reuse existing search / memory surfaces | 3, 4 |
| Document enablement and v1 boundary | 5 |

## Implementation Units

### Unit 1: Register `notebook` and dedicated dispatch

**Files**

* `src/models/registry.rs`
* `src/services/ingestion.rs`
* `tests/integration/notebook_source_dispatch_test.rs`

**Acceptance focus**

* `notebook` is accepted in registry YAML
* notebook sources do not fall through to generic file ingestion or code-graph parsing
* existing `backlog` and `powerbi` routing stays unchanged

### Unit 2: Add fixture matrix and red-phase harness

**Files**

* `tests/fixtures/notebooks/python_markdown.ipynb`
* `tests/fixtures/notebooks/sql_magic.ipynb`
* `tests/fixtures/notebooks/scala_magic.ipynb`
* `tests/fixtures/notebooks/sparkr_magic.ipynb`
* `tests/fixtures/notebooks/metadata_fallback.ipynb`
* `tests/unit/notebook_extract_test.rs`
* `tests/integration/notebook_search_ingestion_test.rs`

**Acceptance focus**

* fixtures pin one summary record plus per-cell derived records
* `chunk_id` values follow stable ordinals such as `cell-0001`
* outputs, attachments, and execution state do not enter indexed content
* precedence cases fail red before implementation
* malformed notebooks are skipped without crashing the run

### Unit 3: Implement notebook content-record indexing

**Files**

* `src/models/notebook.rs`
* `src/services/notebook_extract.rs`
* `src/services/notebook_indexer.rs`
* `tests/integration/notebook_search_ingestion_test.rs`

**Acceptance focus**

* collect `.ipynb` files recursively for `notebook` sources
* emit one `notebook_summary` record per file and one cell record per author-written markdown or code cell
* preserve the real `.ipynb` path for every derived record
* keep IDs stable through `content_record_identity_seed`-style derivation
* sweep deleted notebooks cleanly by source scope

### Unit 4: Implement per-cell language resolution and record shaping

**Files**

* `src/services/notebook_extract.rs`
* `src/services/notebook_indexer.rs`
* `tests/unit/notebook_extract_test.rs`
* `tests/integration/notebook_search_ingestion_test.rs`

**Acceptance focus**

* recognize `%sql` / `%%sql`, `%%scala`, `%%sparkr`, and `%%python`
* apply precedence of magic > `language_info.name` > `kernelspec.language` > `unknown`
* keep markdown and code cells distinct via record kinds
* surface resolved language in record payload / provenance text without widening the `ContentRecord` schema in v1
* keep arbitrary magic parsing and notebook graph edges out of scope

### Unit 5: Document enablement, verification flow, and v1 non-goals

**Files**

* `docs/quickstart.md`
* `docs/ARCHITECTURE.md`
* Ship-owned follow-on closure artifact under `docs/closure/`

**Acceptance focus**

* docs explain `notebook` source setup and `.ipynb` scope
* docs name the fixture-backed verification flow
* docs explicitly defer outputs, execution state, notebook graph edges, and code-graph symbol extraction

## Dependency Graph

```text
1 -> 2 -> 3 -> 4 -> 5
```

## Decisions and Rationale

1. Use a dedicated `notebook` source boundary like `powerbi`, not generic whole-file ingestion
2. Reuse multi-record `ContentRecord` support instead of inventing synthetic notebook-cell files
3. Keep v1 in search / memory ingestion; do not extend the code graph yet
4. Keep resolved cell language in record payload and provenance text, not a new content-record schema field
5. Exclude outputs and execution state to keep the first slice small and retrieval-focused

## Risks and Caveats

* malformed notebook JSON must warn-and-skip rather than break the whole ingestion run
* notebook line numbers are weaker than cell ordinals; `chunk_id` and `chunk_index` should be the primary provenance keys
* notebook scope can expand quickly, so v1 non-goals must stay explicit in tests and docs

## Plan Hardening Signals

* **Public API, schema, or contract change**: YES - additive `notebook` registry content type
* **Security, auth, permission, or compliance-sensitive behavior**: NO
* **Migration, backfill, destructive data/config action, or irreversible step**: NO
* **External integration, operator checkpoint, or external dependency**: NO
* **High runtime, rollout, or rollback risk**: LOW

**Requires plan hardening: no**

## Runtime Verification and Closure

* Unit 1: verify registry parsing and dedicated dispatch
* Unit 3: verify one summary record plus expected cell records are retrievable through `query_memory`
* Unit 4: verify SQL, Scala, SparkR, and metadata-fallback cells return the expected resolved language
* Unit 5: verify docs cross-references and fixture names stay accurate

## Release Observability

Ship should require:

* notebook sync success for the fixture matrix
* expected record counts per notebook fixture
* successful retrieval of known markdown and code cells through `query_memory`
* rollback trigger: notebook indexing breaks unrelated source ingestion or starts indexing output noise instead of author text

## Constitution Check

* stays within Rust ingestion and test surfaces
* preserves test-first delivery by creating the red harness before behavior changes
* keeps discovery workspace-bound
* harvests into `063-F` backlog work instead of ad hoc tracking

## Plan Review

**Gate decision: PASS**

Reviewed through constitution, Rust, scope, and learnings lenses on 2026-05-23.

### Findings

#### P2 - Keep line metadata secondary to cell identity

Treat `chunk_id` and `chunk_index` as the authoritative notebook provenance keys in v1. Only populate line metadata when it is deterministic and cheap.

#### P2 - Add an explicit malformed-notebook assertion

The harness should prove unreadable `.ipynb` files are skipped cleanly instead of crashing or ingesting raw JSON.

#### P3 - Cover both `%sql` and `%%sql`

Keep both SQL spellings in one precedence slice so the first release does not accidentally support only one form.

### Summary

| Severity | Count | Action |
|---|---|---|
| P0 | 0 | - |
| P1 | 0 | - |
| P2 | 2 | carry into harvest |
| P3 | 1 | advisory |

**Gate: PASS** - no blocking findings. Harvest may proceed.
