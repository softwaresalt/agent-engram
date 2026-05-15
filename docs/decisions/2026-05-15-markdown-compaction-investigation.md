---
title: "Markdown Compaction for Token-Efficient Retrieval"
description: "Investigation of derivative-only markdown compaction across historical docs, closure records, memory files, research notes, and backlog artifacts"
topic: "Evaluate safe markdown compaction without rewriting canonical records"
depth: "standard"
decision_status: "investigated"
source_queue_id: "055-F"
linked_artifacts:
  - "src/services/parsing/markdown.rs"
  - "src/services/ingestion.rs"
  - "src/services/backlog_indexer.rs"
  - "src/services/search.rs"
  - "docs/closure/2026-05-14-037-S-readme-install-ux-closure.md"
  - "docs/memory/2026-05-14/037-s-post-merge-closure-memory.md"
tags:
  - "markdown"
  - "retrieval"
  - "compaction"
  - "memory"
  - "closure"
  - "backlog"
---

## Problem Frame

We want better retrieval and lower context cost from historical markdown without
destructively rewriting canonical project records. The governing constraint from
deliberation `006-D` is derivative-only compaction: the original document stays
authoritative, while any condensed form is secondary and provenance-aware.

This investigation is scoped to markdown classes that matter to agent workflows:

* historical docs and archived records
* closure docs
* session memory docs
* research and decision notes
* backlog artifacts where a derivative is useful

## Current Retrieval Constraints

The current Engram behavior already creates a strong dividing line between
document classes that benefit from structure and classes that lose fidelity when
compressed too aggressively:

* general markdown ingestion uses heading-aware chunking when the document has a
  stable H1-led heading spine
* markdown without a stable heading spine falls back to one file-level record
  with a `fallback_reason`, `lint_summary`, and `suggestions`
* backlog markdown is indexed separately as `BacklogContentRecord` entries, but
  today the full body is stored as one retrieval record rather than per-section
  chunks
* search results already expose `record_kind`, `heading_path`, and fallback
  metadata, so compacted derivatives only help if they preserve or improve that
  structure

Implication: compaction helps most when a source document is verbose, stable,
and already semantically sectional. It helps least when the document is an
active workflow contract whose exact wording, status, dependencies, or audit
trail matter.

## Safe and Risky Document Classes

| Document class | Retrieval value of derivative compaction | Primary risk | Recommendation |
|---|---|---|---|
| Archived historical docs | High | Losing provenance or important timeline details | Safe with provenance-preserving derivatives |
| Closure docs | High | Omitting rollback, monitoring, or review outcomes | Safe if the derivative keeps outcome, risk, and traceability fields |
| Session memory docs | Medium to high after work is complete | Active handoff context can drift while work is still open | Safe only after the referenced task or shipment is closed |
| Research and decision notes | Medium | Compressing away trade-offs, rejected options, or unresolved questions | Conditionally safe; compact only when the note is stable or explicitly marked decided |
| Active backlog artifacts | Low | Hiding acceptance criteria, dependencies, status, or commit traceability | Risky; do not compact as a primary retrieval surface |
| Archived backlog artifacts | Medium | Flattening structured fields into lossy prose | Use structured derivative facets, not prose-only summaries |
| Normative instructions, constitutions, prompts, and policies | Very low | Semantic drift from exact wording | Never compact for authoritative use |

## Why the Matrix Looks This Way

### Historical and closure records are the best first targets

These documents are mostly retrospective. Their highest-value retrieval payload
is usually a small set of stable facts:

* what changed
* why it changed
* what risks mattered
* what follow-up remains
* how to trace the decision back to source material

That maps well to compact derivatives centered on decisions, constraints,
outcomes, and provenance.

### Memory docs are good targets only after closure

Memory files are often verbose because they capture live execution state,
failures, and next steps. That verbosity is useful while work is in flight. Once
the task is done or blocked, the retrieval need changes from "replay the session"
to "recover the conclusion and why it mattered." Post-hoc compaction is useful;
live compaction is risky.

### Research notes need trade-off preservation

Research and deliberation docs usually contain the most value in their option
comparison and rationale, not just the final answer. A derivative that removes
rejected options or unresolved questions becomes misleading. These docs can be
compacted safely only if the derivative preserves:

* chosen direction
* rejected alternatives
* constraints
* open questions

### Backlog artifacts should stay canonical-first

Backlog items are workflow contracts, not merely knowledge records. Their exact
status, hierarchy, labels, dependencies, and acceptance criteria affect
execution. Compressing them into prose harms the very queries agents use to plan
or ship work. For backlog, the safe derivative is a structured search aid, not a
replacement summary.

## Candidate Derivative Formats

| Format | Best for | Expected token reduction | Retrieval usefulness | Risk |
|---|---|---|---|---|
| Decision card | Closure, historical, archived memory | High | High for "what happened and why" queries | Medium |
| Section abstract map | Research notes, long historical docs | Medium to high | Highest fidelity while staying skimmable | Low |
| Outcome timeline | Closure and historical records | High | Strong for chronology and regression forensics | Medium |
| Structured facets sidecar | Archived backlog items | Very high | Excellent for filters and exact retrieval pivots | Low |
| One-paragraph prose summary | Any class | High | Weak for precise retrieval and traceability | High |

## Recommended Derivative Shape by Class

### 1. Decision card

Use for closure docs, archived memory, and other stable retrospective records.

Recommended fields:

* source path
* source hash or commit SHA
* objective
* key decision
* constraints
* outcome
* follow-up
* rollback or failure signal when applicable

This is the best balance between token savings and trustworthy retrieval.

### 2. Section abstract map

Use for research and decision documents with meaningful heading structure.

Shape:

* preserve the source title
* keep one short abstract per top-level heading
* retain rejected options and unresolved questions explicitly

This format aligns well with Engram's heading-aware markdown chunking because it
preserves a stable H1-led spine.

### 3. Structured facets sidecar

Use for archived backlog artifacts only.

Shape:

* artifact ID
* title
* type
* final status
* parent
* dependencies
* acceptance summary
* shipped-by commit or PR

This keeps the derivative queryable without pretending the summary is the source
of truth.

## Model-Assist Options

| Option | Suitability | Guardrails |
|---|---|---|
| Small model drafts first-pass derivatives | Good for closed, structured docs | Require fixed output schema, provenance fields, and human review before promotion |
| Small model extracts section abstracts only | Best fit | Limit to extractive summaries grounded in explicit headings |
| Small model rewrites active workflow artifacts | Poor fit | Do not use |
| Human-authored template with model fill assistance | Good | Keep deterministic headings and require canonical-link backreferences |

## Smaller-Model Assessment

Cheaper models such as Claude Haiku 4.5 or GPT-5.4 Mini are plausible assistants
for derivative generation when the task is narrow and schema-bound:

* extract decisions, objectives, constraints, open questions, and outcomes from a
  closed document
* produce one abstract per existing heading
* emit structured YAML or markdown fields with mandatory provenance

They are a poor fit for:

* deciding whether a subtle requirement in an active backlog item can be dropped
* rewriting normative instructions
* resolving contradictory or ambiguous source material without human review

The practical boundary is extractive compaction versus interpretive rewriting.
Small models are acceptable for the former and unsafe for the latter.

## Guardrails

Any future implementation should require these controls:

1. Canonical-first storage
   * the original markdown remains authoritative
   * derivatives live at separate paths and are clearly marked non-canonical

2. Provenance
   * record source path
   * record source content hash or merge SHA
   * record generation timestamp
   * record generator identity

3. Regeneration discipline
   * derivatives are invalidated when source hash changes
   * active documents do not silently retain stale compacted views

4. Scope limits
   * no destructive rewriting
   * no replacement of backlog workflow state
   * no compaction of constitutions, instructions, prompts, or policy docs for
     authoritative consumption

5. Reviewability
   * derived content must be easy to diff
   * a reviewer must be able to compare derivative claims against the source

## Recommendation

Start with derivative compaction for closed and retrospective markdown only:

1. closure docs
2. archived memory docs
3. stable historical and research records

For those classes, prefer:

* decision cards for outcome-oriented retrieval
* section abstract maps for concept-oriented retrieval

Defer active backlog compaction. If backlog derivatives are explored later, make
them structured facet extracts rather than narrative summaries.

## Non-Goals

* destructive rewriting of canonical markdown
* compaction of active workflow contracts into authoritative summaries
* normative instruction compaction for agent consumption
* replacing existing heading-aware chunking with summary-only ingestion

## Final Judgment

Derivative-only markdown compaction is worth pursuing, but only for stable
historical material where retrieval value comes from distilled decisions and
outcomes rather than exact operational wording. The safest first implementation
target is closure plus archived memory, followed by stable research notes.
Active backlog artifacts and normative agent instructions should remain
canonical-first and uncompacted.
