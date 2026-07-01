---
title: "Should Engram add a Rust-native tree-sitter parser for TMDL?"
type: spike
date: 2026-06-12
time_box: "2h"
conclusion: "pivot"
confidence: "medium"
linked_parent_work_item: null
stash_id: "59039891"
promoted_to: ["none"]
tags:
  - "powerbi"
  - "tmdl"
  - "tree-sitter"
  - "parsing"
---

## Goal

**Question.** Should agent-engram add a Rust-native tree-sitter parser for
Tabular Model Definition Language (TMDL), and if so, what integration shape is
the safest fit for the current architecture?

## Success Criteria

* We verify where the current TMDL extractor fails against the real fixture in `tmp/`
* We verify whether a usable Rust-native tree-sitter TMDL ecosystem already exists
* We produce a recommendation that is specific enough to drive follow-on implementation

## Scope Constraints

* Read-only spike with no production code, dependency, or backlog item changes
* Investigation grounded in the current repository plus targeted public documentation and repository metadata
* DAX and PBIR are out of scope except where they directly affect TMDL parser feasibility

## Investigation Approach

1. Inspect the current TMDL extractor and compare it with the real Power BI fixture under `tmp/`
2. Inspect prior Power BI and tree-sitter artifacts to avoid repeating earlier work
3. Check official TMDL documentation to confirm the language shape and object model
4. Check public tree-sitter TMDL availability on GitHub and crates.io
5. Synthesize the recommended parser and integration path

## Findings

### What Was Discovered

#### 1. The current TMDL extractor is a narrow line-prefix parser, not a grammar

`src/services/powerbi_tmdl.rs` only recognizes a small set of line starts:
`model`, `table`, `column`, `measure`, `relationship`, `dataSource`,
`dataType:`, and `expression:`. It does not build a real syntax tree or track
indentation-scoped blocks.

That is already enough to explain why the current implementation is fragile
against real TMDL:

* relationship parsing expects an inline `relationship A.B -> C.D` form
* measure parsing only preserves same-line expressions after `=`, or a later
  `expression:` property
* refs, annotations, partitions, data source files, and multiline expression
  bodies are outside the supported subset

This is materially weaker than the actual fixture shape in `tmp/`.

#### 2. The real fixture uses block-form relationships and multiline measure bodies that the current parser does not model

The shipped fixture under
`tmp/ILSOS-VehicleServices.SemanticModel/definition/relationships.tmdl`
declares relationships as named blocks:

```text
relationship FactToTitle
  fromColumn: FactVehicleRegistrations.VehicleTitleKey
  toColumn: DimVehicleTitle.VehicleTitleKey
```

The current parser does not support that form. Its `parse_relationship`
function only accepts text split by `->`.

The table files also include multiline DAX measures and M partitions. For
example, `FactVehicleRegistrations.tmdl` includes measures whose `=` is followed
by an indented expression block and a partition whose `source = ```...```` block
contains M code. The current `parse_measure` helper drops expressions when the
text after `=` is empty on the declaration line, which means real multiline DAX
measure bodies are not preserved.

#### 3. Official TMDL is indentation-based and object-model aligned

Microsoft's TMDL documentation describes the language as:

* text-based and optimized for human readability
* indentation-based, with parent/child structure expressed through whitespace
* folder-based, with `model.tmdl`, `relationships.tmdl`, `expressions.tmdl`,
  `dataSources.tmdl`, and per-table files under `tables/`
* aligned to the Tabular Object Model (TOM), including properties and embedded
  DAX or M expressions

This is a good fit for a grammar-driven parser, but it also raises the core
implementation challenge: a useful tree-sitter grammar needs to handle both
indentation and embedded raw expression content.

#### 4. The public tree-sitter TMDL ecosystem is immature

The external search results were not strong enough to justify adopting an
off-the-shelf Rust crate:

* `tom-jagus/tree-sitter-tmdl` exists as a repository name, but the repository
  is currently empty
* `Srivatsan260/tree-sitter-tmdl` contains a WIP grammar with `grammar.js`,
  `package.json`, tests, and queries, but no crates.io package and no Rust
  binding files in the repository root
* crates.io returns no `tree-sitter-tmdl` crate

The WIP grammar's own README states the critical limitation plainly: it is
line-oriented, does not emit indent/dedent structure, and would need an
external scanner in a later iteration to recover strict hierarchical blocks.

That makes the external ecosystem a useful reference source, not a production
dependency we should consume directly today.

#### 5. A TMDL tree-sitter parser does not fit the current generic code-graph parser contract

Engram's generic tree-sitter parser surface in `src/services/parsing.rs` is
built around code-graph extraction:

* supported `Language` values map to source-code file types such as Rust,
  Python, SQL, and Markdown
* `parse_source` returns `ParseResult { symbols, edges }`
* extracted symbols are normalized to `Function`, `Class`, and `Interface`

TMDL does not naturally fit that shape. The existing Power BI path returns
`PowerBiSemanticModel`, `PowerBiTable`, `PowerBiMeasure`, and
`PowerBiRelationship` objects instead.

The safer architectural fit is therefore a Power BI-specific tree-sitter parser
module that feeds `powerbi_tmdl`-style semantic-model extraction, not a new
generic code-graph language in `Language`.

#### 6. There is a strong precedent for grammar-first spikes, but the TMDL path needs a different integration boundary

The SQL grammar spike and follow-on work show that this repository is willing to
add tree-sitter grammars when the ecosystem is mature and the output maps
cleanly into existing parser contracts. TMDL differs in two ways:

* the public grammar ecosystem is still immature
* the output target is a Power BI semantic-model AST, not generic code symbols

So the right move is not "repeat the SQL pattern exactly." The right move is
"reuse the tree-sitter discipline, but keep the parser inside the Power BI
ingestion boundary."

### What Was Tried and Failed

* Reusing a published crates.io grammar crate. No `tree-sitter-tmdl` crate is currently published
* Reusing `tom-jagus/tree-sitter-tmdl` as an immediate dependency. The repository is empty
* Treating the existing line-prefix parser as sufficient. The real fixture contains relationship, measure, and partition forms that the current parser does not represent faithfully

### Remaining Unknowns

* Whether the first tree-sitter cut should include an external indent/dedent scanner, or whether a statement-level parse plus a second indentation pass is sufficient
* Whether M partitions and multiline DAX bodies should be modeled as opaque raw blocks in v1 or partially tokenized
* Whether the eventual parser should live as a vendored local grammar crate, a generated in-repo parser module, or a git-sourced dependency once the ecosystem matures

## Recommendation

**Conclusion**: pivot  
**Confidence**: medium

We should **pivot** from "find an external Rust-native TMDL tree-sitter crate"
to "build or vendor a local TMDL tree-sitter grammar and integrate it inside
the Power BI ingestion pipeline."

Recommended shape:

1. **Keep the integration Power BI-specific**
   Add a dedicated TMDL tree-sitter module that returns Power BI semantic-model
   objects or an intermediate TMDL AST. Do not add TMDL to the generic
   `Language` enum unless we later decide TMDL should participate in the
   repository-wide code graph as a first-class language
2. **Treat DAX and M as embedded raw blocks in the first cut**
   We do not need a DAX parser to make TMDL materially better. A first version
   can capture measure bodies and partition sources as opaque text spans while
   still using tree-sitter for structure
3. **Use the real fixture as the grammar corpus**
   The fixture already provides the high-value constructs the current parser
   misses: block relationships, `ref` statements, multiline measures,
   partitions, and model-level references
4. **Avoid depending on the current public WIP grammar directly**
   The WIP grammar is useful as research input, but its own README documents the
   missing indentation support, and there is no published Rust crate to consume
5. **Stage the work after baseline PBIP ingestion, not as a prerequisite**
   Tree-sitter TMDL is a parser-hardening path. It should improve and eventually
   replace the fragile line parser, but it should not block straightforward PBIP
   ingestion work that can still progress with targeted extractor improvements

## Next Steps

1. Create a follow-on implementation plan for a local TMDL tree-sitter parser
   scoped to Power BI ingestion, not generic code-graph language support
2. Define the minimum v1 grammar corpus from the existing fixture:
   `model.tmdl`, `relationships.tmdl`, `expressions.tmdl`, and at least one
   complex table file with multiline measures and a partition block
3. Decide the v1 hierarchy strategy:
   external scanner for indentation now, or statement parse plus indentation
   post-pass
4. Keep DAX and M block capture opaque in v1, and treat DAX parsing as a
   separate follow-on feature
5. Add regression tests that prove improvement over the current parser on:
   block relationships, multiline measure bodies, `ref` statements, and
   partition blocks

## References

* `src/services/powerbi_tmdl.rs:42-166`
* `src/services/powerbi_tmdl.rs:219-276`
* `src/services/parsing.rs:29-58`
* `src/services/parsing.rs:107-250`
* `src/services/parsing/sql.rs:1-55`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/model.tmdl:1-39`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/relationships.tmdl:1-33`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/tables/FactVehicleRegistrations.tmdl:54-109`
* `tests/unit/powerbi_extract_tmdl_test.rs:8-24`
* `docs/decisions/2026-05-22-pbip-project-definition-indexer-spike.md`
* `docs/exec-plans/2026-05-22-pbip-project-definition-indexer-plan.md`
* `docs/decisions/2026-04-24-sql-grammar-spike.md`
* `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md`
* <https://learn.microsoft.com/en-us/analysis-services/tmdl/tmdl-overview?view=sql-analysis-services-2025>
* <https://learn.microsoft.com/en-us/analysis-services/tmdl/tmdl-reference-tabular-object?view=sql-analysis-services-2025>
* <https://github.com/Srivatsan260/tree-sitter-tmdl>
