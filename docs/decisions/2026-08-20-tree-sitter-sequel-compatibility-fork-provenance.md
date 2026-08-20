# Decision: Adopt `softwaresalt/tree-sitter-sql` Immutable Compatibility Fork for CREATE PROCEDURE Support

**Date**: 2026-08-20
**Status**: Adopted
**Feature**: 123-F — Enable SQL CREATE PROCEDURE via approved immutable grammar fork
**Shipment**: 119-S
**Related tasks**: 123.001-T (RED), 123.002-T (Cargo pin), 123.003-T (this document), 123.004-T (doc activation), 123.005-T (verification gates), 123.006-T (runtime closure)
**Superseded task**: 033.005-T (blocked pending upstream release; now superseded — CREATE PROCEDURE is delivered via this immutable fork instead)

## Context

`tree-sitter-sequel` (crates.io `0.3.11`) does not support `CREATE PROCEDURE`
syntax; the statement parses as an `ERROR` node and `agent-engram` graceful
degradation yields zero extracted symbols. Upstream `DerekStride/tree-sitter-sql`
has an open pull request (`#355`) adding `CREATE PROCEDURE` support, but the PR
has not merged and no crates.io release exists that includes it.

Waiting indefinitely for the upstream release blocks CREATE PROCEDURE symbol
extraction. `agent-engram` does not own, administer, or publish the upstream
or fork repositories — fork construction was an external/manual prerequisite,
reviewed and approved through the External Evidence Gate (EG-1) before any
repository-owned implementation work began.

## External Evidence Gate (EG-1) — Satisfied 2026-08-20

EG-1 is a fail-closed claim gate requiring named-operator approval of the
following evidence packet before any Cargo adoption could proceed. All
criteria were independently verified and approved:

| Criterion | Value |
|---|---|
| Approved fork repository | `https://github.com/softwaresalt/tree-sitter-sql` |
| Provenance branch (context only, never a Cargo source) | `compat/create-procedure-generated-artifacts` |
| Exact immutable revision (full 40-char SHA) | `50837582b5ba15c7acff3be7bf585a1082d90528` |
| Reviewed upstream base | `f5480941b00ce267f6fe0fb03a066809ade0bc16` (merge of upstream PR [`DerekStride/tree-sitter-sql#355`](https://github.com/DerekStride/tree-sitter-sql/pull/355)) |
| Generator | tree-sitter CLI `0.26.3`, source tag commit `00d6e1e8ffaa476715356f2626e90f8ef08d2272` |
| License | MIT (upstream Derek Stride copyright retained, unmodified) |
| Fork lifecycle owner / named approver | `softwaresalt` |
| Cross-platform fork CI | [run 32412941837](https://github.com/softwaresalt/tree-sitter-sql/actions/runs/32412941837) — `headSha` = `50837582b5ba15c7acff3be7bf585a1082d90528`; ubuntu-latest, windows-2025, macos-latest all `success` |
| Commit signature | **Not cryptographically signed.** Accepted residual risk (see Risk Acceptance below). |

### Source delta (exactly 10 files vs. upstream base)

No change to grammar source, corpus expectations, `package.json`, or
`package-lock.json` beyond the reviewed upstream base. The delta is
provenance/build tooling plus six regenerated (byte-identical, hash-checked)
generated parser artifacts:

* `.gitignore`
* `COMPATIBILITY.md`
* `scripts/verify-compatibility.mjs`
* `.github/workflows/compatibility.yml`
* `src/grammar.json`
* `src/node-types.json`
* `src/parser.c`
* `src/tree_sitter/alloc.h`
* `src/tree_sitter/array.h`
* `src/tree_sitter/parser.h`

Verified locally against the fork checkout at rev
`50837582b5ba15c7acff3be7bf585a1082d90528`:
`git diff --stat f5480941b00ce267f6fe0fb03a066809ade0bc16 HEAD` reports
exactly these 10 files, all pure additions — matching the approved evidence
packet with no unexplained delta.

### Generated artifact SHA-256 hashes (independently re-verified locally)

| File | SHA-256 |
|---|---|
| `src/parser.c` | `758d1b31fd076b082dac1c5e018eab67dcfadaf1c29532a638880d74c5f807cc` |
| `src/grammar.json` | `23765d76df658b4199284f3522197a144d2ee873f4454a79caefe83aa18d787a` |
| `src/node-types.json` | `37d9fb9875262208dc2822302ed678b08bdcf4c19c436d696bec9158d221b7f1` |
| `src/tree_sitter/alloc.h` | `b29c1c9fb7cc82f58c84b376df1297d6e2737a1d655fd356db0859e3c29c2fea` |
| `src/tree_sitter/array.h` | `5bdf6ed1a78e3409fd443e085ca967a64c188a5d082aaf7f819bccd53a471c94` |
| `src/tree_sitter/parser.h` | `180b893c8734778fd32f372dfbc27bd6ad1cd2221f26150b31256ff6716320d2` |

Hashes were recomputed (`Get-FileHash -Algorithm SHA256`) against the
`cargo`-fetched checkout at `%CARGO_HOME%/git/checkouts/tree-sitter-sql-*/5083758`
and match the approved evidence packet byte-for-byte.

## Decision

Adopt the approved immutable fork as the sole source for `tree-sitter-sequel`:

```toml
tree-sitter-sequel = { git = "https://github.com/softwaresalt/tree-sitter-sql", rev = "50837582b5ba15c7acff3be7bf585a1082d90528" }
```

The full 40-character commit SHA is mandatory. Branch, tag, or floating refs
are prohibited as a Cargo dependency source; `compat/create-procedure-generated-artifacts`
is provenance context only.

## Local Verification Performed (123.002-T / 123.005-T)

Independent of the fork's own CI evidence, the following were verified inside
the `agent-engram` workspace:

1. **Commit reachability** — `git ls-remote` confirms the SHA resolves on the
   fork remote (`refs/heads/compat/create-procedure-generated-artifacts`).
2. **Cargo.lock pin** — the `tree-sitter-sequel` entry's `source` field ends in
   the exact approved SHA:
   `git+https://github.com/softwaresalt/tree-sitter-sql?rev=50837582b5ba15c7acff3be7bf585a1082d90528#50837582b5ba15c7acff3be7bf585a1082d90528`.
3. **`cargo metadata --locked`** — succeeds without modifying the lockfile.
4. **`cargo tree -i tree-sitter-sequel`** — reports a single dependency edge
   (`engram` only), no duplicate or shadow source.
5. **Generated artifact hashes** — recomputed locally; all six match (above).
6. **License** — `LICENSE` in the fork checkout is MIT, Derek Stride copyright,
   unmodified from upstream.
7. **ABI compatibility** — `src/parser.c` declares `LANGUAGE_VERSION 15`,
   compatible with the workspace's `tree-sitter = "0.25"` dependency.
8. **RED → GREEN behavior** — `tests/unit/parsing_test.rs` was updated to
   require CREATE PROCEDURE extraction *before* this pin (123.001-T, observed
   RED against crates.io `0.3.11`), then observed GREEN after adoption
   (16/16 SQL unit tests passing, including a grammar-ABI probe asserting a
   named `create_procedure` node with no `ERROR`, and no regression to
   `CREATE FUNCTION` or malformed-SQL graceful degradation).

## Risk Acceptance

The fork head commit is **not cryptographically signed**. This is accepted as
a residual risk, mitigated by:

* full-SHA pinning (no floating branch/tag dependency),
* independently reproduced generated-artifact hashes (this document),
* independently reproduced source-delta review (this document),
* cross-platform fork CI evidence bound to the exact `headSha`,
* a named, accountable fork lifecycle owner (`softwaresalt`).

## Compromise / Staleness Response

Stop shipment or release immediately, and treat as a rollback trigger, on any
of the following:

* repository-control change at the fork (unexpected force-push, ownership
  transfer, disabled CI),
* any generated-artifact hash mismatch against this document,
* an unexplained generated or source delta beyond the 10 files listed above,
* loss of the referenced fork CI provenance,
* ABI drift against `tree-sitter = "0.25"`,
* a security advisory affecting the fork or its generator toolchain,
* fork inactivity exceeding 90 days without an explicit re-review.

**90-day staleness review**: due no later than **2026-11-18**. The reviewing
agent must re-verify reachability, hashes, and fork activity, and either
reconfirm the pin, re-pin to a newer reviewed fork revision, or execute
retirement.

**Retirement path**: retire this fork dependency once an official
`tree-sitter-sequel` crates.io release provides equivalent `CREATE PROCEDURE`
support and downstream parsing tests continue to pass against it. Retirement
replaces the `git`/`rev` dependency with the official crates.io release and
removes this document's fork-specific verification steps from future gates.

## Rollback Procedure

Revert as a single, atomic release-unit change:

1. Restore `Cargo.toml`: `tree-sitter-sequel = "0.3"` (crates.io).
2. Restore `Cargo.lock` via `cargo update -p tree-sitter-sequel` against the
   crates.io registry source.
3. Restore the zero-symbol / graceful-degradation expectation in
   `tests/unit/parsing_test.rs` for `CREATE PROCEDURE`.
4. Restore the stale-limitation module documentation in
   `src/services/parsing/sql.rs` (see 123.004-T).
5. Rebuild and re-run `cargo dev-test` from the restored, trusted lock.

## Rollback Triggers (release-observability)

* any supported-platform build or ABI failure,
* a SQL parse panic or error regression,
* a `CREATE FUNCTION` regression,
* a missing or duplicate procedure symbol,
* any provenance anomaly identified during the 72-hour observation window
  (123.006-T).
