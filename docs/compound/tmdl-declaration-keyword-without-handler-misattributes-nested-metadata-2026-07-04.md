---
title: "An indent-scoped line parser must actively skip blocks it recognizes as declarations but does not model, or nested metadata leaks onto the previous member"
description: "The TMDL parser listed hierarchy/level/role/function in is_declaration_line() (so they end a member's property capture) but had no handler arm for them in handle_declaration(). Those opening lines fell through, leaving last_member/member_indent pointing at the preceding column/measure. The block's own deeper lineageTag:/annotation lines then resolved to that stale member and OVERWROTE its metadata. Triggered by ubiquitous auto date tables. Fix: when an unmodeled declaration keyword opens inside a table, clear member scope and open an indent 'skip window' so deeper metadata lines are dropped rather than misattributed."
problem_type: "parser_scope_leak"
category: "language-hazard"
component: "crates/powerbi-tmdl-parser/src/lib.rs handle_declaration / resolve_metadata_target"
root_cause: "A keyword can be simultaneously (a) a declaration boundary that must terminate the previous member's property scope and (b) unmodeled, with no branch that consumes its body. If the fall-through path does nothing, the parser's 'current member' pointer stays aimed at the previous, unrelated member, and any indent-scoped child lines (lineageTag:, annotation) attach there."
resolution_type: "code_fix"
date: "2026-07-04"
shipment: "068-S"
---
# Recognized-but-unmodeled declaration keywords must actively skip their body

## Problem

The TMDL parser is a line/indent parser. Scope-sensitive metadata lines
(`annotation <name> = <value>` and `lineageTag: <guid>`) attach to whatever
object the parser currently believes it is inside, resolved by indent:

```text
table Date
  column Year
    lineageTag: COLUMN-GUID        <- attaches to column Year (deeper than member)
    annotation Format = "0"

  hierarchy 'Calendar'             <- is_declaration_line() == true, but NO handler arm
    lineageTag: HIERARCHY-GUID     <- BUG: attaches to / overwrites column Year
    annotation IsHidden = true     <- BUG: appended to column Year's annotations
    level Year
      column: Year
```

`is_declaration_line()` included `hierarchy`/`level`/`role`/`function` so that
encountering one would terminate the *previous* member's opaque property capture.
But `handle_declaration()` had no arm for them, so the line fell through to the
final `false`. Crucially it did **not** clear `last_member`/`member_indent`, so
`resolve_metadata_target()` still saw a column member one indent level up and
routed the hierarchy's nested `lineageTag:`/`annotation` onto that column —
silently overwriting the column's real lineage tag and polluting its annotations.

Power BI auto date/time tables emit a `hierarchy` after the columns in nearly
every model, so this fired constantly in real ingestion, not just edge cases.

## Fix

Two coordinated pieces (commit `143490b`):

1. A fall-through arm in `handle_declaration()` (before the final `false`): if
   the line is a recognized declaration keyword we do not model and we are inside
   a table, clear member scope and open/widen an indent "skip window".

```rust
if is_declaration_line(trimmed) {
    enter_unmodeled_member_block(state, indent); // clears last_member/member_indent,
    return true;                                 // sets skip_below_indent = Some(indent)
}
```

2. `resolve_metadata_target()` returns a new `MetadataTarget::Skip` (a no-op in
   the attach helpers) when the line is indented deeper than the open skip
   window, so the hierarchy's own metadata is *dropped* (out of task scope) rather
   than misattributed. `prepare_pending_state()` clears the window on dedent
   (`indent <= opened`).

The result: the preceding column keeps its own `lineageTag`/annotations, and the
hierarchy's nested metadata is discarded cleanly instead of corrupting a sibling.

## Lesson

In an indent/line parser, membership between the "declaration boundary" set and
the "has a handler" set can diverge. Any keyword you recognize as a boundary but
do **not** model must still actively (a) reset the current-member pointer and
(b) suppress its own children — otherwise scope-sensitive child lines silently
attach to the previous, unrelated member. Dropping unmodeled sub-blocks is a
correct, conservative default; leaking them onto a neighbor is a data-corruption
bug that unit fixtures without a trailing sibling block will not catch. Always
include a "member, then unmodeled sibling block with its own metadata" fixture.
