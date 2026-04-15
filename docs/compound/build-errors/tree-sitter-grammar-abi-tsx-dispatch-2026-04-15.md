---
title: tree-sitter Grammar ABI Constraint and TSX Grammar Dispatch
date: 2026-04-15
category: build-errors
tags: [tree-sitter, grammar, abi, tsx, parsing]
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
- Pin all grammar crates to `"0.23"` in `Cargo.toml`. Do not upgrade grammar
  crates past `0.23` while the `tree-sitter` dependency stays at `0.24.x`.

## ABI Table

| tree-sitter version | Accepted grammar ABI | Grammar crate version to pin |
|---------------------|----------------------|-------------------------------|
| 0.24.x              | 13–14                | 0.23.x                        |
| 0.25.x+             | TBD (check release)  | match tree-sitter upgrade     |

## Related

- `src/services/parsing/typescript.rs` — `parse_tsx_source` using `LANGUAGE_TSX`
- `src/services/parsing.rs` — `Language::Tsx` variant and dispatch
- `Cargo.toml` — grammar crate pins at `"0.23"`
- Session: 026-F multi-language parsing (2026-04-15)
