---
title: "Jupyter notebook source support decided plan"
description: "Decided implementation for notebook content ingestion completed locally as 063-F / 051-S / commit 3acd337"
status: pre-merge
feature: "063-F"
shipment: "051-S"
commit: "3acd337"
source_plan: "docs/archive/plans/2026-05-23-jupyter-notebook-source-support-plan.md"
spike: "docs/decisions/2026-05-22-jupyter-notebook-source-support-spike.md"
closure: "docs/closure/2026-05-23-051-S-notebook-source-support.md"
---

## Decided Architecture

Notebook support is a dedicated content-ingestion path, not a code-graph path.
Each `.ipynb` file stays a single physical source and produces one
`notebook_summary` record plus per-cell derived records for author-written
markdown and code cells.

Stable cell provenance is carried by `chunk_id` and `chunk_index`, with IDs such
as `cell-0001`. We keep resolved language in record content and provenance text
instead of widening the `ContentRecord` schema in v1.

## Implemented Units

1. Register the `notebook` source type and dedicated dispatch
2. Add the fixture matrix and red harness for notebook extraction behavior
3. Implement notebook content-record indexing
4. Apply per-cell language precedence and record shaping
5. Document notebook enablement, runtime verification, and the v1 boundary

## Key Constraints

* Language precedence is `magic > language_info.name > kernelspec.language > unknown`
* The initial magic whitelist is limited to `%sql`, `%%sql`, `%%scala`,
  `%%sparkr`, and `%%python`
* Outputs, execution state, arbitrary magic parsing, notebook graph edges, and
  code-graph symbol extraction stay out of scope for v1
* Malformed notebooks must warn and skip instead of crashing the ingestion run

## Verification Contract

Required validation for this plan was:

* notebook dispatch integration coverage
* notebook extractor unit coverage
* notebook search-ingestion integration coverage
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
* `cargo dev-test`

`cargo audit` remains advisory in this repository because CI marks that step
`continue-on-error: true` and the current findings are pre-existing transitive
issues rather than notebook regressions.

## Remaining Merge Step

The implementation is complete locally on branch
`063-jupyter-notebook-source-support` at commit `3acd337`. The remaining work is
to open or update the PR, merge it, and then run the post-merge shipment
archival step for `051-S`.
