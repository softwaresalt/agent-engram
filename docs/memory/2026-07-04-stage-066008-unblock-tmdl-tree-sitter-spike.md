# Stage session memory — 2026-07-04 — Unblock + re-scope 066.008-T (TMDL tree-sitter safety correction)

## Task

Operator-directed Stage run against `agent-engram` (branch off `main` @ `f73d880`):
correct a **mischaracterized** "unsafe/constitution" blocker on `066.008-T`,
unblock it, and produce a spike that re-evaluates the tree-sitter TMDL path now
that the safety myth is dispelled. No Ship work.

## Key finding (code-verified) — the safety blocker was false

`066.008-T` claimed `#![forbid(unsafe_code)]` prevents tree-sitter consumption in
`powerbi-tmdl-parser`. Refuted against the code:

- The main crate already consumes **ten C-based tree-sitter grammar crates**
  (`Cargo.toml:51-61`) across **eleven safe `set_language` call sites**
  (`src/services/parsing/*.rs`) via `Parser::set_language(&tree_sitter_x::LANGUAGE.into())`.
- **Zero `unsafe`** anywhere in `src/` + `crates/`; the only `unsafe` token is a
  *comment* at `src/cli/output.rs:46`.
- Both `src/lib.rs:10` and `crates/powerbi-tmdl-parser/src/lib.rs:9` carry
  `#![forbid(unsafe_code)]` — and the main crate uses all eleven grammars anyway.
- `#![forbid(unsafe_code)]` forbids unsafe in a crate's **own source**, not its
  dependencies. A grammar crate encapsulates generated FFI/`unsafe` (incl. a
  hand-written `scanner.c`) behind a safe `LANGUAGE` surface.

## Second finding — 068-S already closed the coverage gap (ROI shift)

Post-`068-S`, `crates/powerbi-tmdl-parser/src/lib.rs` is a **1404-line
indentation-aware line/indent parser** (`parse_tmdl_document -> TmdlModel`,
`lib.rs:251`) that already handles block relationships, multiline measures,
partitions, data sources, refs, annotations, and lineage tags — the exact
constructs the 2026-06-12 spike attributed to tree-sitter. So a grammar now buys
**robustness/maintainability**, not coverage. The crate doc (`lib.rs:6-7`)
already declares the tree-sitter swap point behind the same public API, so
integration risk is low; the cost is grammar sourcing + external scanner + ABI.

## Actions taken

1. **Corrected + unblocked `066.008-T`** (markdown-authoritative edits + one CLI
   `move`):
   - `backlogit move 066.008-T --status queued` (blocked -> queued; logs event).
   - Rewrote the body note to remove the unsafe/constitution framing and state
     the verified finding + the real decision axes (grammar sourcing,
     indentation external scanner, ABI, ROI).
   - Retitled: "Evaluate tree-sitter TMDL grammar v1 — grammar sourcing,
     indentation scanner, ROI" (dropped misleading "constitution-compliant").
   - Rewrote the Stage note (dated 2026-07-04) and retained the 2026-07-03 note
     as superseded-for-provenance.
   - Kept `dependencies: [068.003-T]` and `links: related_to 068-F`.
2. **Re-parented `066.008-T`** from archived `066-F` to new umbrella `069-F` via
   **direct markdown `parent_id` edit** — ID **intentionally preserved** (did
   NOT use `backlogit adopt`, which re-IDs) so the follow-on reference in the
   shipped `068-S` manifest stays valid. The ID-prefix/parent mismatch
   (066.008-T under 069-F) is cosmetic and doctor-clean.
3. **Created umbrella feature `069-F`** ("TMDL tree-sitter grammar path —
   evaluation and first slice"): `backlogit add --type feature`, then populated
   description/goals/dod + `links: related_to 066-F, 068-F` +
   `references:` the two spikes.
4. **Authored the spike**
   `docs/decisions/2026-07-04-tmdl-tree-sitter-safe-consumption-correction-spike.md`
   (conclusion **defer** the grammar build, confidence **high**): unblock the
   item, do NOT build a grammar yet, gate any grammar investment behind a cheap
   **differential-evaluation first slice** (safe parser vs. prototype grammar on
   a real corpus), keep it Power BI-scoped behind `parse_tmdl_document`.

## Dispositions

- `066.008-T`: **queued** (was blocked), parent **069-F**, deps `068.003-T`,
  link `related_to 068-F`, note corrected, retitled.
- `069-F`: **queued** umbrella feature (new).
- Spike: written under `docs/decisions/` (sibling-consistent with the
  2026-06-12 TMDL / 2026-06-13 DAX / 2026-06-13 PBIR spikes).

## Recommendation summary

Safety blocker is **retired** (definitive). But because `068-S` already ships a
working safe indent parser covering the high-value constructs, a full TMDL
grammar is **not ROI-positive now**. Recommendation: **defer the grammar build**;
keep `066.008-T` queued as an evaluation gate; run a ~2h differential-evaluation
first slice; only build a grammar (vendored/generated + external indentation
scanner + pinned ABI) if the safe parser demonstrably mis-parses a real corpus.

## Landmines / notes for next agent

- **Do NOT run `backlogit sync`** — unions the stale SQLite cache into markdown
  and resurrects stale states. Markdown under `.backlogit/` is authoritative.
  Cache remains intentionally stale; it is gitignored and not committed.
- `backlogit update` cannot change `parent_id`; `backlogit adopt` re-IDs. To
  re-parent while preserving an ID, edit `parent_id` in markdown directly.
- **docline lint** (`backlogit docs lint`) flags `source`/`doc_type` as missing
  on the new spike — but this is **universal** across the entire
  `docs/decisions/` corpus (zero docs repo-wide carry `doc_type:`); it is a
  pending `backlogit docs migrate`, not a per-file gate. Spike kept
  sibling-consistent.
- Pre-existing ` M .gitignore` working-tree drift is unrelated operator drift —
  **not staged**.
- Stash still holds `F7E89921` (DAX tree-sitter idea) — already has its own
  2026-06-13 spike (defer); left parked.

## Handoff

Committed to branch `066008-tmdl-tree-sitter-spike` (off `main`), pushed, **no
PR** (Orchestrator lands it). Ship/Orchestrator picks up from there.
