# Compacted: 042-F CLI Parity Session Memory

**Compacted from**: docs/memory/2026-05-06/stage-042-F-cli-parity-memory.md, docs/memory/2026-05-07/042-F-cli-parity-post-merge-memory.md
**Feature**: 042-F | **Shipment**: 026-S | **PR**: #85 | **Merge**: 53b432d

## Outcome: SHIPPED

All 10 tasks completed. 17 CLI subcommands added mirroring all 18 MCP tools.
Three review rounds (7 + 8 comments) + 1 CI fix commit. All resolved.

## Files Modified

src/bin/engram.rs, src/cli/{mod,flags,output,runner,commands/}.rs, src/shim/mod.rs
tests/unit/cli_parser_test.rs, tests/integration/cli_e2e_test.rs
docs/architecture.md, .github/instructions/agent-engram.instructions.md

## Key Decisions

1. pub (not pub(crate)) for CLI — separate binary crate
2. std::io::IsTerminal for TTY — no unsafe
3. shim::run(workspace_override) — set_var is unsafe in Rust 2024
4. Removed workspace from Daemon variant — global flag takes precedence
5. value_parser = [json,text] restricts --format at parse time

## Compound Learnings

- clap-long-vs-name-attribute-2026-05-07.md
- rust-2024-set-var-unsafe-2026-05-07.md
- ci-all-targets-stricter-than-local-2026-05-07.md

## Follow-Up Stash

B59D87CA (start.ps1 integration), D5F04760 (query-graph impl), 1620BAA6 (quiet e2e tests)
