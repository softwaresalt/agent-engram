---
title: "Is the tree-sitter TMDL path actually blocked on unsafe/constitution — and is it worth building now?"
type: spike
date: 2026-07-04
time_box: "2h"
conclusion: "defer"
confidence: "high"
linked_parent_work_item: "066.008-T"
stash_id: null
promoted_to: ["069-F", "066.008-T"]
tags:
  - "powerbi"
  - "tmdl"
  - "tree-sitter"
  - "parsing"
  - "safety"
  - "correction"
---

## Goal

**Question.** Two parts, because the second only matters once the first is settled:

1. Is the tree-sitter TMDL grammar path genuinely blocked by a
   constitution/`unsafe` decision, as `066.008-T` claimed — or is that a
   mischaracterization?
2. Given the answer to (1) and the current state of the safe parser after
   `068-S`, is building a tree-sitter TMDL grammar worth doing now, and if so,
   what is the smallest honest first slice?

This spike supersedes the safety framing of the prior TMDL tree-sitter spike
(`docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md`, conclusion: pivot) with a
code-verified correction, then re-runs the cost/benefit against the parser that
actually shipped in the interim.

## Success Criteria

* Confirm or refute the "`#![forbid(unsafe_code)]` prevents tree-sitter
  consumption" blocker against the real code, not against assumption.
* Establish whether the previously-missing TMDL constructs are still missing
  after `068-S`.
* Produce a recommendation specific enough to either unblock-and-build or
  unblock-and-defer, with a concrete first slice if warranted.

## Scope Constraints

* Read-only spike: no production code, dependency, grammar, or schema changes.
* Investigation grounded in the current repository at `main` (`f73d880`).
* DAX and PBIR remain out of scope except where they bound TMDL feasibility
  (both already have their own spikes: `2026-06-13-dax-tree-sitter-spike.md`
  conclusion defer, `2026-06-13-pbir-tree-sitter-spike.md` conclusion decline).

## Investigation Approach

1. Enumerate every `unsafe` token and every `#![forbid(unsafe_code)]` site in
   `src/` and `crates/`.
2. Enumerate how the main crate already consumes tree-sitter grammars.
3. Re-read the current `powerbi-tmdl-parser` crate and `powerbi_tmdl` service to
   see which constructs the safe parser now handles versus what the 2026-06-12
   spike said was missing.
4. Identify the real (non-safety) blockers that survive the correction.
5. Re-run cost/benefit and recommend.

## Findings

### 1. The "unsafe/constitution" blocker is a mischaracterization (refuted)

The `066.008-T` blocker read: *"the current crate-level `#![forbid(unsafe_code)]`
prevents direct `tree-sitter` consumption in `powerbi-tmdl-parser`."* The code
says otherwise:

* **The main crate already consumes ten C-based tree-sitter grammar crates**
  (`Cargo.toml:51-61`: rust, python, javascript, typescript, go, c-sharp, c,
  cpp, swift, sequel) across **eleven `set_language` call sites** — all through
  the fully safe binding API `Parser::set_language(&tree_sitter_x::LANGUAGE.into())`:
  * `src/services/parsing/c.rs:32`, `cpp.rs:40`, `csharp.rs:22`,
    `go_lang.rs:22`, `javascript.rs:24`, `python.rs:22`, `rust.rs:22`,
    `sql.rs:35`, `swift.rs:33`, `typescript.rs:26` (TS) and `typescript.rs:60`
    (TSX).
* **There is zero `unsafe` code in the workspace.** The only `unsafe` token
  anywhere in `src/` + `crates/` is a *comment* at `src/cli/output.rs:46`
  (`// ... avoids unsafe + external libc.`).
* **`#![forbid(unsafe_code)]` is present in both** `src/lib.rs:10` and
  `crates/powerbi-tmdl-parser/src/lib.rs:9` — and the main crate consumes all
  eleven grammars anyway.

The reason is mundane: `#![forbid(unsafe_code)]` forbids `unsafe` in *that
crate's own source*, not in its dependencies. A tree-sitter grammar is
auto-generated C (`parser.c`, optionally a hand-written `scanner.c`), but the
Rust binding crate encapsulates all of that FFI/`unsafe` behind a safe
`LANGUAGE` constant and `set_language`. Consuming a `tree-sitter-tmdl` crate
would be the **same pattern already shipped eleven times**, and
`#![forbid(unsafe_code)]` in `powerbi-tmdl-parser` would still hold even if the
grammar required an external `scanner.c` — that C lives inside the grammar crate
and is consumed through a safe surface.

**Conclusion for part 1: the safety blocker does not exist. `066.008-T` is
unblocked.**

### 2. `068-S` already closed the coverage gap the prior spike blamed on tree-sitter

The 2026-06-12 spike justified tree-sitter primarily on *missing coverage*: block
relationships, multiline measure bodies, partitions, data-source files, refs,
annotations, and lineage tags. Since then, `068-S` shipped and the safe parser
absorbed almost all of it. The current `crates/powerbi-tmdl-parser/src/lib.rs` is
a **1404-line indentation-aware line/indent parser** (`parse_tmdl_document ->
TmdlModel`, entry at `lib.rs:251`) with real whitespace scoping:

* `leading_indent_width`, `member_indent`, and `skip_below_indent` track
  indentation-scoped blocks (`lib.rs:177-222`, `287`).
* Pending-state machines finish blocks on dedent for measures, relationships,
  partitions, and data sources (`should_finish_measure_capture`,
  `should_finish_relationship`, `should_finish_partition`,
  `should_finish_data_source`).
* Modeled types now include `TmdlPartition` (`lib.rs:63`), `TmdlAnnotation`
  (`lib.rs:148`), `TmdlRef` (`lib.rs:157`), and `TmdlDataSource` (`lib.rs:128`),
  with opaque capture of partition `source = ```...```` M bodies and deeper
  data-source content.
* The service layer maps these through in `src/services/powerbi_tmdl.rs`:
  `build_table`/partitions (`132-149`), `build_annotation` (`159`), `build_ref`
  (`166`), `build_data_source` (`199`).

In other words, the hand-rolled **safe** parser already implements the
indentation handling and captures the exact constructs the prior spike said
required a grammar — no tree-sitter, no `unsafe`. The marginal value of a grammar
is therefore no longer *coverage*; it is *robustness and maintainability*
(declarative grammar + error recovery versus a 1400-line state machine on edge
cases such as nested member blocks, quoting/escaping, and expression-boundary
ambiguity).

### 3. The integration boundary is already prepared (low integration risk)

The crate's own module doc anticipates the swap
(`crates/powerbi-tmdl-parser/src/lib.rs:6-7`): *"The current implementation is a
fixture-driven line/indent parser. A future tree-sitter-backed implementation
can land behind the same public API."* A grammar backend only needs to produce a
`TmdlModel` from `parse_tmdl_document`; the whole `src/services/powerbi_tmdl.rs`
adapter and downstream Power BI graph/ingestion stay untouched. This confirms the
prior spike's architectural recommendation (keep TMDL inside the Power BI
ingestion boundary, do **not** add it to the generic `Language` enum) and means
the risky part is the grammar itself, not the wiring.

### 4. The real, surviving blockers are cost, not safety

None of these is a constitution problem; all are real engineering cost:

1. **Grammar sourcing.** There is still no mature, published `tree-sitter-tmdl`
   crate (the 2026-06-12 survey found an empty `tom-jagus` repo and a WIP,
   line-oriented `Srivatsan260` grammar with no Rust binding and no indent
   support). We would vendor or generate and then **maintain** a `grammar.js`
   ourselves.
2. **Indentation external scanner.** TMDL is whitespace-scoped, so a faithful
   grammar needs an external scanner emitting indent/dedent — the same class of
   work the safe parser already does by hand, now re-expressed in C.
3. **Grammar ABI / build fragility.** See
   `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md`.
   Adding a grammar means pinning ABI and owning build breakage.
4. **ROI collapse after `068-S`.** Because the safe parser now covers the
   high-value constructs, a grammar buys robustness at the margin, not new
   capability — while adding a vendored C grammar, an external scanner, and ABI
   maintenance to a workspace that currently ships zero `unsafe` and a
   dependency-light parser crate.

## Recommendation

**Conclusion**: defer (the grammar *build*) — after unblocking the item.
**Confidence**: high (the safety refutation is definitive; the ROI/defer call is
well-grounded in the now-shipped 1404-line safe parser).

1. **Unblock `066.008-T` and correct its note** (done in this session): the
   safety/constitution blocker is retired; the item moves `blocked -> queued`
   and is re-scoped to the real decision axes (sourcing, scanner, ABI, ROI).
2. **Do not build a full TMDL grammar yet.** After `068-S` the safe parser
   covers the constructs that justified the grammar, so a full build is not
   ROI-positive on current evidence.
3. **Gate any grammar investment behind a cheap, differential first slice**
   (below). Only commit to grammar sourcing + external scanner + ABI ownership
   if that slice shows the safe parser materially mis-parsing real-world TMDL.
4. **Keep the path Power BI-scoped** and behind the existing
   `parse_tmdl_document` API; never add TMDL to the generic `Language` enum.
5. **Home the track under `069-F`** (new umbrella) so the safe-parser depth
   feature `068-F` stays shipped/closed and the tree-sitter track has an active
   parent.

## Proposed First Slice (decision gate, not a grammar build)

A time-boxed **differential-evaluation harness**, ~2h, single-width, no new
dependency:

* Assemble a real-world TMDL corpus beyond the inline unit fixtures
  (`tests/unit/powerbi_extract_tmdl_test.rs` currently uses small inline
  `S-PTM-xx` snippets): pull representative `model.tmdl`, `relationships.tmdl`,
  and complex table files with multiline measures, partitions, refs,
  annotations, and lineage tags.
* Run the corpus through the current safe `parse_tmdl_document` and record where
  it drops, truncates, or mis-scopes structure — quantify the correctness delta.
* **Decision rule:** if the safe parser's error rate on the corpus is low, close
  `066.008-T` as *decline* (keep hardening the safe parser). If it exhibits
  material, structural mis-parses that are hard to fix incrementally, promote a
  follow-on task under `069-F` to prototype a vendored/generated grammar with an
  external indentation scanner and a pinned ABI, benchmarked against the safe
  parser on the same corpus.

This makes the expensive grammar decision evidence-driven and cheap to reverse.

## Next Steps

1. Keep `066.008-T` queued under `069-F` as the evaluation gate; execute the
   differential-evaluation first slice before any grammar dependency lands.
2. If the gate recommends building: create follow-on tasks under `069-F` for
   (a) grammar sourcing (vendor vs. generate), (b) external indentation scanner,
   (c) ABI pinning + build-fragility mitigation, (d) parity + regression tests
   against the safe parser. Each is its own ~2h, single-width, test-first task.
3. Record the corpus and the differential results alongside this spike so the
   decision is auditable.

## References

* `066.008-T` (this session: blocked -> queued, note corrected, re-parented to `069-F`)
* `069-F` (new umbrella: TMDL tree-sitter grammar path — evaluation and first slice)
* `068-F` / `068-S` (shipped safe-parser depth: partitions, datasource props, refs/annotations/lineage)
* `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md` (prior spike; safety framing superseded here)
* `docs/decisions/2026-06-13-dax-tree-sitter-spike.md`, `docs/decisions/2026-06-13-pbir-tree-sitter-spike.md`
* `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md`
* `Cargo.toml:51-61` (ten tree-sitter grammar crates)
* `src/services/parsing/c.rs:32`, `cpp.rs:40`, `csharp.rs:22`, `go_lang.rs:22`, `javascript.rs:24`, `python.rs:22`, `rust.rs:22`, `sql.rs:35`, `swift.rs:33`, `typescript.rs:26`, `typescript.rs:60` (safe `set_language` call sites)
* `src/lib.rs:10`, `crates/powerbi-tmdl-parser/src/lib.rs:9` (`#![forbid(unsafe_code)]`)
* `src/cli/output.rs:46` (the only `unsafe` token in the workspace — a comment)
* `crates/powerbi-tmdl-parser/src/lib.rs:6-7` (doc: tree-sitter backend behind the same public API), `:251` (`parse_tmdl_document`)
* `src/services/powerbi_tmdl.rs:25` (`extract_tmdl_semantic_model`), `:132-149`, `:159`, `:166`, `:199`
* `tests/unit/powerbi_extract_tmdl_test.rs` (inline `S-PTM-xx` fixtures)
