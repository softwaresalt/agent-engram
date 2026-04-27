---
title: tree-sitter Grammar ABI Constraint and TSX Grammar Dispatch
date: 2026-04-15
updated: 2026-04-27
category: build-errors
tags: [tree-sitter, grammar, abi, tsx, parsing, swift, sql]
---

## Problem

When adding TSX support to the multi-language parser:

1. The original `typescript.rs` module doc stated "TSX files are handled by the
   same parser using the TypeScript grammar (JSX nodes are ignored at Tier 1)."
   This was incorrect — using `LANGUAGE_TYPESCRIPT` for `.tsx` files will
   mis-parse or silently drop JSX nodes.

2. There is a hidden ABI constraint: `tree-sitter 0.24.x` only accepts grammar
   ABI versions 13–14. Grammar crates at version `0.24+` emit ABI 15, which
   fails **at runtime** (not at compile time) with no obvious error message.

## Solution

- Use `tree_sitter_typescript::LANGUAGE_TSX` for `.tsx` files via a separate
  `Language::Tsx` variant and a dedicated `parse_tsx_source()` function.
- Keep `Language::TypeScript` dispatching to `parse_typescript_source()` with
  `LANGUAGE_TYPESCRIPT`.
- Pin most grammar crates to `"0.23"` in `Cargo.toml` — they emit ABI 14 which
  is accepted by both 0.24 and 0.25 runtimes.
- Exception: `tree-sitter-swift = "=0.7.1"` emits ABI 15 and requires the
  0.25 runtime. Pin to exact version to prevent silent upgrades.

## ABI Table (confirmed as of 2026-04-27)

| tree-sitter version | Accepted grammar ABI | Grammar crate version to pin     |
|---------------------|----------------------|----------------------------------|
| 0.24.x              | 13–14                | 0.23.x                           |
| 0.25.x              | 13–15                | 0.23.x (most); `=0.7.1` (swift); `0.3` (sequel) |

**Project baseline**: tree-sitter `0.25` (upgraded from 0.24 in shipment 005-S).

**SQL/sequel**: `tree-sitter-sequel 0.3` emits ABI 15, compatible with tree-sitter 0.25.
Add as `tree-sitter-sequel = "0.3"` in `Cargo.toml`.

**Kotlin blocked**: `tree-sitter-kotlin 0.3.x` targets tree-sitter 0.20–0.22 and
is incompatible with 0.25. `kotlin.rs` is a no-op stub; activate when a
0.25-compatible crate is published on crates.io.

## Related

- `src/services/parsing/typescript.rs` — `parse_tsx_source` using `LANGUAGE_TSX`
- `src/services/parsing/sql.rs` — `parse_sql_source` using tree-sitter-sequel 0.3
- `src/services/parsing.rs` — `Language::Tsx`, `Language::Sql` variants and dispatch
- `Cargo.toml` — grammar crate pins at `"0.23"` (most), `"=0.7.1"` (swift), `"0.3"` (sequel)
- Session: 026-F multi-language parsing (2026-04-15)
- Shipment 005-S (2026-04-21) — tree-sitter upgraded to 0.25; swift, c, cpp parsers added
- Shipment 013-S (2026-04-27) — SQL parser added via tree-sitter-sequel 0.3
