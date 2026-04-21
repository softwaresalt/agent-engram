---
type: stage-checkpoint
date: 2026-04-20
agent: stage
phase: deliberation-complete-awaiting-operator-confirmation
---

# Group B Stage Cycle — Deliberation Checkpoint

## Status

Step 2 (deliberate) complete. Awaiting operator selection of A/B/C before promoting to
plan (Step 3).

## Group B membership (operator-selected)

* `0523404D` — Swift, Kotlin, C, C++ parsers (medium)
* `D715B3EE` — SQL dialects (T-SQL, PL/SQL, PostgreSQL, MySQL, SQLite) (medium)
* `47F34E2C` — Markdown parser (medium)

## Deliberation artifact

`docs/decisions/2026-04-20-group-b-language-coverage-deliberation.md`

## Key finding

Parser layer (`src/services/parsing/*.rs`) is DB-agnostic — confirmed by reading
parsing.rs, code_graph.rs:1102-1117, and spotting `src/services/cozo_validation.rs` already
in flight. Adding tree-sitter parsers does NOT collide with the CozoDB migration.

However, `ExtractedSymbol` enum (`Function`/`Class`/`Interface`) does NOT fit SQL
(tables, views, indexes, stored procs) or Markdown (headings, code blocks, link refs).
This forces an IR-extension decision that DOES touch storage layer — same reason `003-F`
was dropped from this group.

## Options presented

| | Scope | Risk |
|---|---|---|
| A | Composite — all 3 packs, one shipment, includes IR extension | medium-high |
| **B** ⭐ | Split — ship Swift/Kotlin/C/C++ now; defer SQL+Markdown to next cycle | low |
| C | Spike SQL/Markdown grammar+IR fit, then composite | medium |

Recommendation: **Option B**. Same logic that excluded `003-F` from the group applies to
SQL/Markdown IR extension during CozoDB flux.

## If operator selects B

Ship #1 covering feature: "Tree-sitter parser support for Swift, Kotlin, C, and C++"
* 4 sub-epics (one per language)
* ~12-13 atomic tasks (parser + dispatcher + tests per lang, plus shared infra)
* Estimated 16–22h
* `D715B3EE` and `47F34E2C` stay stashed for follow-up cycle

## If operator selects A or C

Plan and harvest expand accordingly; A includes IR extension as Sub-epic 4; C inserts a
~4h grammar reconnaissance spike before planning.

## Prior tree-sitter constraint (HARD)

Per `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md`:
all new grammar crates MUST pin to `0.23.x` (ABI 14). `impl-plan` must verify each new
crate version on crates.io.

## Next action (after operator response)

1. Operator confirms A/B/C
2. Invoke `impl-plan` skill on the deliberation artifact
3. P-006 hardening gate (read `Requires plan hardening:` from plan)
4. `plan-review` skill
5. P-003 validation
6. `harvest` skill
7. `backlogit_create_shipment` with feature ID first; respect scope guard (use only `harvest_ids`)
8. Session summary with shipment_id handoff

## Deferred (end of cycle)

Clean 4 stale stash entries (`A1B2C3D4`, `B2C3D4E5`, `C3D4E5F6`, `D4E5F6A7`) — all
operationalized by 004-S.
