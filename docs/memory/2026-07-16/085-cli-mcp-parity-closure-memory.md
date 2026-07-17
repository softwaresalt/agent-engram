---
type: session-memory
date: 2026-07-16
agent: ship
session: "2c95481b - 085-S CLI<->MCP parity closure"
topic: "085-S CLI<->MCP parity audit/doc/drift-guard merged; post-merge closure"
---

# Session memory - 085-S CLI<->MCP parity closure

## Outcome

Shipment **085-S** (CLI<->MCP full parity: audit, canonical mapping doc & drift guard) merged to
`main` as merge commit **945ece65** via **PR #259**. The shipment delivered documentation plus one
contract test; it changes no runtime behavior, schema, or on-disk format. Closure artifacts and
backlog reconciliation were produced on branch `feat/085-closure` from the `ship-085` worktree.

## Tasks completed or reconciled

* `090.001-T` - Parity audit + canonical MCP<->CLI mapping doc (`docs/cli-mcp-parity.md`);
  supersedes the stale 2026-07-05 gap list. Done, archived.
* `090.002-T` - Parity drift-guard contract test (`tests/contract/lint_dax_cli_parity_test.rs`,
  target `contract_lint_dax_cli_parity`). Done, archived.
* `090.003-T` - Bidirectional doc parity: every catalog tool description references the mapping doc
  and CLI surface. Done, archived.
* `090-F` - feature archived (all shipment-scope tasks done).
* `085-S` - shipment archived.

## Files modified in the shipped feature

* `docs/cli-mcp-parity.md` (new canonical mapping doc)
* `tests/contract/lint_dax_cli_parity_test.rs` (modified — extended the existing DAX-lint contract
  test with the CLI<->MCP parity/drift-guard assertions; the `[[test]]` target
  `contract_lint_dax_cli_parity` already existed, so `Cargo.toml` was unchanged)
* `src/shim/tools_catalog.rs` (`cli_desc!` / `mcp_only_desc!` macros reference the canonical doc URL)
* `src/bin/engram.rs` (`long_about` references the canonical doc URL)
* `.backlogit/archive/090.001-T.md`, `.backlogit/archive/090.002-T.md`,
  `.backlogit/archive/090.003-T.md` (archived on the feature branch), `.backlogit/queue/090.005-T.md`
  (new deferred follow-up)

Closure work touched only documentation and backlog state:

* `docs/closure/2026-07-16-085-cli-mcp-parity-closure.md`
* `docs/memory/2026-07-16/085-cli-mcp-parity-closure-memory.md`
* `.backlogit/archive/085-S.md`, `.backlogit/archive/090-F.md`
* `.backlogit/queue/090.005-T.md` (new deferred follow-up)

## Key decisions

* **Audit-first, not stale-list harvest.** Gate `088.001-R` F5 required rebuilding the gap
  inventory from live surfaces. The audit excludes `create_task`/`update_task` (not real MCP
  tools), identifies genuine MCP-only tools (`get_retrieval_eval_report`, `query_changes`,
  `index_git_history`), and local-only CLI commands (shim/daemon/install/update/reinstall/
  uninstall/manifest/verify/migrate-down).
* **The guard is an oracle, not a mirror.** The contract test derives the mapping from the real
  dispatch `match` (text-parsed via `include_str!`, feature-independent, 23 names) and the
  compiler-checked `all_tools()` catalog (21). `dispatch_table_is_superset_of_catalog` asserts
  catalog subset dispatch so a parser under-count or stale catalog entry fails loudly.
* **Canonical URL in code references.** Repo-relative doc paths are not resolvable for a
  globally-installed binary; CLI help and both catalog macros emit
  `https://github.com/softwaresalt/agent-engram/blob/main/docs/cli-mcp-parity.md`, asserted via
  `DOC_URL`.
* **Uninstall doc is destructive-default.** `engram uninstall` default deletes the entire
  `.engram/`; `--keep-data` preserves `config.toml` and removes only runtime artifacts
  (`src/installer/mod.rs:642-698`).
* **090-F archived with the shipment.** The 085-S manifest lists 090-F; the deferred functional
  gap-closure (090.004-T) and routing-identity registry (090.005-T) remain queued as follow-ons,
  matching the 088-S pattern (archive the feature, keep deferred child tasks queued).

## Adversarial and Copilot cycle summary

Adversarial review ran BEFORE Copilot to minimize Copilot iterations (operator directive):

* Cross-model pre-PR review: `gpt-5.6-sol` (rust), `gemini-3.1-pro-preview` (security),
  `gpt-5.6-terra` (scope) - P1/P2/P3 fixed before PR opened.
* Independent adversarial code review (`gpt-5.6-sol`) pre-Copilot: 2 P2s on the drift guard
  (union-check masks catalog<->dispatch drift; line parser under-counts wrapped `|` arms) - fixed
  in `ee3f035` via the superset oracle.
* Copilot - 5 passes, all resolved:
  * `2561d83a` clean.
  * `ee3f035` 7 findings: T1/T2 uninstall docs + T3/T4/T7 canonical URL fixed in `7d8db7d`; T5
    (routing-identity) + T6 (feature-gated wrapped-arm) deferred to `090.005-T`.
  * `7d8db7d` 1 finding: stray ESC (0x1B) in `090.005-T.md` from a shell escape - fixed in `444feef`.
  * `444feef` clean.
  * Every thread replied-to and resolved via `resolveReviewThread`.

Merge-gate evidence before PR #259 merged: Copilot review bound to HEAD `444feef`, Copilot
de-requested, 0 unresolved review threads, `mergeStateStatus == CLEAN`, CI `build` SUCCESS.

## Verification state

* Formatting, clippy `-D warnings -D clippy::pedantic`, and targeted tests green.
* `contract_lint_dax_cli_parity` 11/11; `contract_tools_catalog` 5/5.
* CI `build` went green at `444feef` after a one-time cozo-connect flake at `7d8db7d`.

## Known external flakes and open operator items

* CI `build` flaked once at `7d8db7d`: a `cozo::storage::sqlite` `connect_db` panic on a tokio
  worker degraded the graph build, tripping
  `real_path_via_run_retrieval_eval_dispatch_matches_ground_truth` (resolved-edge 1 != 3).
  Environmental DB-connect flake, orthogonal to the docs/test-only 085-S diff; green on re-run.
  Candidate for a DB-connect hardening chore.
* PR #248 (081-S) remains open for the operator (their active branch
  `feat/088-rec1-call-resolution`). Option C supersedes its approach; operator owns disposition.

## Deferred follow-ups

* `090.004-T` (blocked) - Close functional MCP<->CLI parity gaps. Re-harvest from the 090.001
  audit into a follow-on shipment; do NOT implement from the stale list. Product-scope decision.
* `090.005-T` (queued) - routing-identity + compiler-checked dispatch registry (Copilot T5/T6).

## Next steps

1. Push this closure branch and open the closure PR.
2. Request/await Copilot review; drive the 4-point merge gate; merge.
3. Queue order after closure: **086-S** (092-F writer-side workspace+config atomicity), one active
   shipment at a time per P-001.
