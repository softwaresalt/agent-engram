---
type: session-memory
agent: copilot-cli
date: 2026-05-13
topic: documentation-refresh
shipment_id: 036-S
feature_id: 050-F
status: complete
---

# Session memory — documentation refresh

## Outcome

Completed the documentation overhaul requested from stash item `35FC7DF8` and
carried it through Stage and Ship workflow creation before implementing the docs
rewrite directly on branch `docs/036-S-documentation-refresh`.

## Backlog artifacts completed

* `005-D` — Documentation overhaul and focused docs information architecture refresh
* `050-F` — Documentation overhaul and focused docs information architecture refresh
* `050.001-T` — Audit documentation drift and define the target docs information architecture
* `050.002-T` — Rewrite README as a brochure overview with basic installation only
* `050.003-T` — Refresh the architecture reference for the current shim-daemon and storage model
* `050.004-T` — Refresh usage and configuration guides around the current install and indexing flows
* `050.005-T` — Refresh tools and workflow docs for the current MCP and CLI surfaces
* `050.006-T` — Refresh operational docs and repair documentation navigation integrity
* `036-S` — Documentation refresh and focused docs IA overhaul

## Files changed

* `README.md`
* `docs/quickstart.md`
* `docs/configuration.md`
* `docs/mcp-tool-reference.md`
* `docs/architecture.md`
* `docs/troubleshooting.md`
* `docs/log-observation-guide.md`
* `docs/workflows.md`
* backlog artifacts under `.backlogit/archive/`, `.backlogit/stash.jsonl`,
  `.backlogit/archive/stash.jsonl`, `.backlogit/memories.json`, and
  `.backlogit/checkpoints/`

## Key decisions

* Reduced `README.md` to brochure-level positioning plus basic installation only
* Split detailed material into focused docs pages rather than rebuilding another
  monolithic root guide
* Kept `quickstart.md` as the happy-path setup guide
* Added `docs/workflows.md` as the task cookbook for recurring CLI flows
* Documented `legacy-sse` only as a compatibility path and kept shim plus daemon
  over IPC as the primary runtime story
* Documented `engram sync` as the routine refresh path and `engram index` as the
  full rebuild alias

## Drift corrected

* Removed SurrealDB-first framing from the public docs set
* Removed HTTP/SSE-first and `/rpc`-oriented guidance from the primary setup path
* Replaced stale curl-heavy MCP examples with current CLI-oriented workflows
* Updated docs to reflect the current 18-tool default catalog and current CLI
  parity surfaces
* Updated storage and operations docs to reflect branch-aware workspace identity
  and CozoDB-backed runtime state

## Validation and review

* Ran a local markdown link-resolution pass across the rewritten docs set
* Used an independent code-review pass to look for factual inaccuracies in the
  documentation rewrite

## Notes

* The Ship agent completed backlog orchestration but did not apply the docs
  changes, so the implementation was completed directly afterward
* Generated client helper templates still contain compatibility-oriented details,
  so the docs avoid over-promising exact generated helper content

## Next steps

* Stage, commit, and push `docs/036-S-documentation-refresh`
* Open a PR for user review and merge approval
