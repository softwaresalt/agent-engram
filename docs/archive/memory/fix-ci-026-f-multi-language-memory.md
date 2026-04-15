---
session: 40a6ffbd-2a99-4db4-9c2d-ad963001973e
date: 2026-04-15
task: Fix CI failures and Copilot review comments on PR #14 (026-F)
status: complete
commit: eec0cce
---

## Summary

CI was failing on PR #14 (`026-F` multi-language parsing + branch-aware code graph storage) with one
test failure and 5 Copilot review comments requiring fixes.

## Outcome

CI green ✅ on run #24435628637. All 5 review threads replied to and resolved.

## Files Modified

- `tests/integration/code_graph_test.rs` — renamed `index_workspace_skips_non_rust_files` →
  `index_workspace_skips_unsupported_files`; updated assertion from `files_parsed == 1` to
  `files_parsed == 2` (Python now parsed by multi-language support)
- `src/services/parsing.rs` — added `Language::Tsx` variant; fixed `TryFrom<&str>` empty
  file_path by returning `ParseFailed` instead of `UnsupportedLanguage`; added Tsx dispatch
- `src/services/parsing/typescript.rs` — added `parse_tsx_source()` using `LANGUAGE_TSX`;
  updated module doc; restored accidentally-removed `root: Node<'_>` parameter
- `src/services/code_graph.rs` — added `"tsx" => "tsx"` and `"js" | "jsx" => "javascript"` in
  `language_from_path`
- `src/models/config.rs` — added `"tsx"` to `default_supported_languages()`
- `.github/agents/ship.agent.md` — fixed duplicate step numbering in Step 6 (2→3 cascade);
  fixed `docs/research/` → `docs/decisions/` for graduated design decisions
- `.github/skills/harvest/SKILL.md` — removed `or N/A` placeholder from Step 4.2

## Key Decisions

- **TSX needs separate grammar**: `tree_sitter_typescript::LANGUAGE_TSX` for `.tsx`,
  `LANGUAGE_TYPESCRIPT` for `.ts`. Using the wrong grammar silently mis-parses JSX nodes.
- **Empty file_path in TryFrom**: Changed to `ParseFailed` variant to avoid misleading
  error messages with empty paths. `UnsupportedLanguage` still constructed at call sites
  with actual path.
- **Local full suite stalled** (~45 min, 0 output): ort-sys compiled but full suite hung.
  Pushed after targeted test passed + fmt/clippy/check passed. CI completed in ~3.5 min.

## Failed Approaches

- Waiting for full local `cargo test` — hit 45-min stall timeout with no output. Stopped
  and pushed to CI instead (circuit breaker applied).

## Learnings

- After `cargo test --test X` runs (which recompiles ort-sys), `cargo test` (full suite)
  on Windows may still stall on certain test binaries or integration test setup. Push to CI
  instead of waiting locally.
- All 5 review threads were marked `is_outdated: true` by GitHub since the reviewed lines
  changed. Replies + GraphQL `resolveReviewThread` still work correctly on outdated threads.

## Next Steps

- PR #14 is ready for merge approval.
- After merge: run ship.agent.md Step 6 post-merge checklist (operational-closure,
  docs evaluation, compound-refresh, compact-context).
