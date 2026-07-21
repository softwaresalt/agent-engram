---
title: "Python Module-Namespace-Qualified Call Resolution"
date: 2026-07-20
scope: deep
status: draft
origin_stash: FE8B3B2D
related_feature: 094-F
sequence_after: [094-F, 07BFA98E]
---

# Python Module-Namespace-Qualified Call Resolution

> Graduated from stash **FE8B3B2D** (deliberation-spike) so the feasibility
> analysis is durably captured beyond the transient backlog stash. This is the
> "proper fix" for cross-module same-name ambiguity that feature 094-F
> deliberately scoped out. It is its own future feature (deliberation → spike →
> plan), sequenced **after** 094-F (shipped) and after the Spark-lineage spike
> 07BFA98E, per operator ordering. Origin: open PR #277 review thread on
> `src/services/parsing/python.rs:269`.

## Problem Frame

Feature 094-F added bare Python `Calls` edges and language-scoped the shared
cross-file singleton resolver so a Python bare call cannot mis-bind to a
same-named definition in *another language*. It does **not** disambiguate a
bare call when **two same-language (Python) modules** define the same function
name. In that case the singleton resolver correctly fails closed (2+
same-language candidates → drop), so recall is left on the table: a call that a
human can resolve by module namespace is dropped rather than bound.

The proper fix is to resolve Python bare calls through the **module namespace**,
mirroring how engram already resolves Rust calls through canonical module paths.

## Feasibility: YES

Engram already has the core machinery, built for Rust:

- `function_meta.canonical_path` column exists
  (`src/db/cozo_queries.rs`, ~L924-940) but is **Rust-only populated** — it is
  fail-closed empty for Python and other languages
  (`src/services/code_graph.rs`, ~L41-67 / ~L143-160).
- `function_ids_by_canonical_path()` and
  `reresolve_calls_edges_with_canonical_context` already implement
  canonical-path-indexed resolution and run as a full-index post-pass.

Python modules **are** namespaces (`foo/bar.py` → module `foo.bar`), so the
resolution rule is well-defined:

1. Callee defined in the caller's own module → bind to `M.name`.
2. Else imported via `from N import name` → bind to `N.name`.
3. Else ambiguous / dynamic → drop (fail closed).

## Scope Boundary (critical)

- **In scope:** module-level functions and `module.func()` disambiguation — the
  same-name-different-module case.
- **Out of scope (documented non-goal):** instance-method dispatch
  (`self.method()` / `obj.method()`). That requires type inference and is a
  separate, larger problem. 094-F already drops attribute/method calls fail-closed.

## Key Extraction Gap

`src/services/parsing/python.rs` (~L72-141) currently captures only a **flat
`import_path` string**. Namespace resolution needs a **symbol-level
import-binding table** — e.g. `from N import parse as p` must record the binding
`p → N.parse`. This is the Python analogue of Rust's `UseGraph`. Building it is
the main net-new extraction work.

## Fail-Closed Cases (013-D no-false-edge)

Bind nothing (drop) for:

- star imports (`from N import *`),
- relative / package-root ambiguity (`__init__.py`, PEP 420 implicit namespace
  packages),
- re-exports,
- `importlib` / dynamic imports.

## Shape of the Work (new components)

1. **Python import-binding capture** — symbol-level binding table in the Python
   parser (the `UseGraph` analogue).
2. **Python `canonical_path` populator** — populate the existing
   `function_meta.canonical_path` for Python module-level defs (today Rust-only).
3. **Python-aware canonical resolver** — extend
   `reresolve_calls_edges_with_canonical_context` (or a sibling) to resolve
   Python bare calls through the import-binding table + canonical index.

This layers on top of 094-F's U3 language-scoping; it does not replace it.

## Relationship to the Same-File Shadowing Bug (FF7DE872)

The same-file same-name shadowing correctness bug — `find_function_id`
(`src/services/code_graph.rs:2037`) returns the FIRST match by name — is a
special case this feature would subsume. It was deliberately **split out** to
bug **FF7DE872** so it can be fixed independently and sooner, and is **not**
gated behind this deliberation.

## Sequencing

Later staging cycle. After 094-F (shipped) and after Spark-lineage spike
07BFA98E, per operator ordering. Enter via the normal deliberation → spike →
plan → plan-review flow before implementation.
