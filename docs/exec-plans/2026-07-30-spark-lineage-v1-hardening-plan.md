---
title: "Implementation Plan — 097-F Spark lineage v1 hardening (deferred PR #284 review findings)"
date: 2026-07-30
feature: 097-F
shipment: 099-S
source: ".backlogit/queue/097-F.md"
reference_plan: "docs/exec-plans/2026-07-22-spark-notebook-data-lineage-plan.md"
agent: stage
status: reviewed
tasks: [097.001-T, 097.002-T, 097.003-T, 097.004-T, 097.005-T, 097.006-T]
---

# Implementation Plan — 097-F Spark lineage v1 hardening

Hardening follow-ups deferred from PR #284 (feature 095-F "Spark notebook
data-lineage subgraph v1") Copilot fail-closed review cycles 4–5. The P0
findings (V1 crash-safe stamp invalidation, V3 conditional/ternary guarded-write
scope) already shipped in commit `ef977184`. This plan covers the five deferred
P1/nit items (V2, V4, V5, W1, W2) plus the single cross-cutting rollout action
(X1) that propagates the tightened extractor behavior to already-indexed
notebooks. Scope is fixed to those items — no new lineage capability, no
temp-view lineage (still deferred), no schema change.

## Problem Frame

All work sits inside the shipped v1 Spark lineage subgraph. Verified current
source baseline on `main` @ `df01b498` (engram daemon green, session indexed
2026-07-30; line numbers below confirmed by direct read, not the drifted card):

| Item | Real code site (confirmed) | Current behavior | Defect |
|---|---|---|---|
| **V2** | `src/services/parsing/sql.rs` → `normalize_spark_insert` (L425) + `insert_table_prefix_end` (L448) | Byte-scans **raw** source for the `INSERT … OVERWRITE/INTO TABLE` keyword run and rewrites it to `INSERT INTO` **before** tree-sitter parses. Scanner is neither quote- nor comment-aware. | An `INSERT OVERWRITE TABLE` token sequence inside a line/block comment, a single-quoted string literal, or a `"…"` / `` `…` `` quoted identifier is wrongly rewritten, corrupting the SQL and/or minting a spurious normalization. |
| **V4** | `tests/integration/` (new file) exercising `src/services/ingestion.rs::ingest_all_sources` (L60) + `src/models/config.rs::LineageConfig::to_authority_context` (L373) | Lineage edge production is covered only at the unit/write-path tier (`lineage_precision_recall_test.rs`, `notebook_lineage_writepath_test.rs`). No test drives the full `RegistryConfig.lineage → ingest_all_sources → authority-bound edge` path. | No end-to-end integration guard that a configured authority actually yields a persisted `lineage_derives_from` edge through the real ingestion pipeline. |
| **V5** | `src/models/lineage.rs` → `resolve_path` (L211) + `uri_matches_authority` (L289); prefixes originate at `src/models/config.rs::to_authority_context` (L373) | `storage_authorities` allowlist prefixes are trusted **verbatim**. `uri_matches_authority` does a raw `starts_with` + boundary check; a malformed prefix is never validated. | A misconfigured prefix such as `s3://` (empty authority), `://bucket` (empty scheme), or `s3://bucket/some/path` (has a path component) can bind edges for arbitrary URIs, breaking the `scheme://authority`-only trust invariant (AR-01). |
| **W1** | `src/services/parsing/python.rs` → `resolve_cell_candidates` (U2b resolver, the `ReadBind` arms ~L1296–1315) | The first `ReadBind` match arm unconditionally `binding.insert(variable, endpoint)`. A **second** top-level Spark read into an already-bound variable silently **rebinds** it to the new source instead of invalidating. | Per U2b / 095.004-T fail-closed doctrine, any DataFrame reassignment — including a second read into the same binding — must **invalidate** the dataflow chain (emit no edge), not pick the later read. Current code can mint an edge from an ambiguous multi-read binding. |
| **W2** | `src/models/lineage.rs` → `resolve_table` (L167, split at L177) | Validates only `parts.len() == 3`, non-empty parts, and the `::`/`:` delimiter-collision guard. | Multipart-identifier components are **not** validated against the unquoted Spark identifier grammar. A malformed component token (embedded whitespace, quotes, hyphen, leading digit, punctuation) survives and binds a false `table::` endpoint. |
| **X1** (rollout) | `src/models/lineage.rs` → `CURRENT_EXTRACTOR_VERSION` (L38, currently `"1.0.0"`); freshness via `lineage_freshness_token` (L278) | The U4b freshness token = `{CURRENT_EXTRACTOR_VERSION}:{config_fingerprint}` gates re-extraction of unchanged notebooks. | V2/V5/W1/W2 change extractor **output**; without a version bump, already-indexed notebooks retain the old (less-precise / potentially false) lineage until their content changes (C4 stale-lineage trap). |

### Cross-cutting invariant (fail-closed precision floor, 013-D / A5)

Every change tightens output in the **fail-closed direction** — it can only
*drop* edges the extractor should never have emitted, never add new edge classes.
The lineage subgraph's precision floor (0 false edges) is the protected
invariant; recall is allowed to regress on genuinely ambiguous inputs.

## Requirements Trace

| Req (card item) | Implementation action | Verifying test |
|---|---|---|
| V2 quote-aware **and** comment-aware INSERT normalizer | Make the `normalize_spark_insert` scanner skip line comments (`--`), block comments (`/* */`), single-quoted strings (`'…'`), double-quoted identifiers (`"…"`), and backtick identifiers (`` `…` ``) so `INSERT` inside those regions is not rewritten. | Unit fixtures in `sql.rs::tests`: (a) `INSERT OVERWRITE TABLE` in a comment → not rewritten, no edge; (b) same inside a string literal → not rewritten; (c) quoted-identifier regression fixture; (d) a real `INSERT OVERWRITE TABLE` outside any quoted/comment region still normalizes + emits its edge (no recall regression). |
| V4 ingestion integration test | New `tests/integration` test: build a `RegistryConfig` with `lineage` set (`metastore_authority_id` + a catalog authority, or a `storage_authorities` entry), a fixture notebook with a resolvable read→write, run `ingest_all_sources`, assert an authority-bound `lineage_derives_from` edge exists via `select_lineage_edges`. | The new integration test itself (RED harness → GREEN with existing write-path). |
| V5 authority-prefix validation | Add an `is_valid_storage_authority(prefix)` predicate (non-empty scheme, non-empty authority, no path/query/fragment component) and apply it so a malformed prefix never matches in `resolve_path`. | `lineage.rs::tests`: malformed prefixes (`s3://`, `://b`, `s3://b/path`) resolve nothing; a well-formed prefix (`s3://bucket`) still resolves a URI under it. |
| W1 second-read invalidation | In `resolve_cell_candidates`, a `ReadBind` whose `variable` is **already tracked** must invalidate (remove binding + mark session invalidated) instead of rebinding. Also treat an already-bound-then-reread as fail-closed. | `python.rs::tests`: `df=read(a); df=read(b); df.write(t)` → **no** edge; existing single-read happy path + existing rebind/invalidation tests still pass (no regression). |
| W2 multipart-component grammar validation | Add an `is_spark_unquoted_identifier(component)` predicate (`[A-Za-z_][A-Za-z0-9_]*`, non-empty) and reject any 3-part literal with a malformed component in `resolve_table`. | `lineage.rs::tests`: components with whitespace / quote / hyphen / leading-digit / punctuation → `None`; a clean `cat.schema.table` still resolves. |
| X1 rollout / freshness propagation | Bump `CURRENT_EXTRACTOR_VERSION` `"1.0.0"` → `"1.1.0"` exactly once; verify the freshness token change forces re-extraction of a previously-stamped notebook. | `lineage.rs::tests` + an integration assertion that a stale-stamped notebook is re-extracted after the version change (C4 path). |

## Implementation Units

Each unit satisfies the 2-hour rule (<3 files, <5 functions, <4 core test
scenarios), width isolation (single domain), and produces a verifiable atomic
milestone. Execution posture is **test-first** unless noted.

### Unit V2 — Quote/comment-aware Spark INSERT normalizer (domain: sql-parser) — test-first
* **Files:** `src/services/parsing/sql.rs` (single file).
* **Change:** Replace the naive raw-byte scan in `normalize_spark_insert` with a
  region-aware scan that tracks lexer state (in line comment, in block comment,
  in single-quoted string, in `"`-quoted identifier, in `` ` ``-quoted
  identifier) and only attempts the `INSERT … TABLE` rewrite when in normal
  (unquoted, uncommented) state. Keep `insert_table_prefix_end`/`match_ci_word`
  as-is; the fix is in the outer scan loop plus a small state helper.
* **Tests (in `sql.rs::tests`):** comment-INSERT not rewritten; string-INSERT
  not rewritten; quoted-identifier regression fixture; genuine INSERT still
  normalizes (recall preserved). Fewer than 4 core scenarios.
* **Learning applied:** `docs/compound/best-practices/sql-quoted-identifier-resolution-candidates-list-2026-04-29.md`
  (strip/quote-form discipline for SQL identifiers).

### Unit V5 — Storage-authority prefix validation (domain: lineage-model) — test-first
* **Files:** `src/models/lineage.rs` (single file).
* **Change:** Add `fn is_valid_storage_authority(prefix: &str) -> bool` (requires
  `scheme://authority` shape: non-empty scheme before `://`, non-empty authority
  after `://`, and **no** `/`, `?`, or `#` after the authority). Apply it in
  `resolve_path` so a malformed allowlist prefix is skipped (fails closed) before
  `uri_matches_authority` can bind an edge.
* **Tests (in `lineage.rs::tests`):** malformed prefixes resolve nothing;
  well-formed prefix still resolves.
* **Note:** Validation lives at resolve time ("before binding an edge") so it is
  robust regardless of how prefixes entered the context. Do **not** widen scope
  into `config.rs` beyond, at most, an optional pass-through comment.

### Unit W2 — Multipart-identifier grammar validation (domain: lineage-model) — test-first
* **Files:** `src/models/lineage.rs` (single file, distinct function from V5).
* **Change:** Add `fn is_spark_unquoted_identifier(component: &str) -> bool`
  (`^[A-Za-z_][A-Za-z0-9_]*$`, non-empty) and reject any 3-part literal in
  `resolve_table` where any component fails the predicate, before the existing
  `::`/`:` collision guard.
* **Tests (in `lineage.rs::tests`):** malformed components → `None`; clean
  identifier still resolves.
* **Same-file note (build sequencing, NOT a logical dependency):** V5 and W2 both
  edit `src/models/lineage.rs`. Ship builds shipment tasks on one branch
  sequentially, so same-file edits are naturally serialized; no backlog `blocks`
  edge is warranted between them.

### Unit W1 — Second-read-into-same-binding invalidation (domain: python-parser/dataflow) — test-first
* **Files:** `src/services/parsing/python.rs` (single file).
* **Change:** In `resolve_cell_candidates`, before the resolved-`ReadBind`
  `binding.insert`, check whether `variable` is already bound (or already an
  invalidated session); if so, treat the re-read as a rebind of a tracked binding
  and **invalidate** (remove + `invalidated_sessions.insert`) rather than rebind.
  This aligns the resolver with the U2c/U2b F2 doctrine that any reassignment of
  a tracked binding breaks the chain.
* **Tests (in `python.rs::tests`):** second-read-into-same-binding → no edge;
  existing atomic bind→write, per-form rebind, and single-read happy-path tests
  must still pass (no recall regression on the single-binding case).
* **Learning applied:** `docs/compound/best-practices/python-bare-call-edge-extraction-body-scoped-2026-07-20.md`
  (body-scoped, fail-closed Python extraction discipline).

### Unit V4 — Ingestion end-to-end integration test (domain: tests/integration) — characterization/coverage-first
* **Files:** `tests/integration/<new>_test.rs` (single new test file, e.g.
  `lineage_ingestion_e2e_test.rs`).
* **Change:** Construct a `RegistryConfig` whose `lineage` field is populated
  (via `LineageConfig`), point a source at a fixture notebook containing a
  resolvable authority-bound read→write, run `ingest_all_sources`, and assert a
  persisted `lineage_derives_from` edge via `CodeGraphQueries::select_lineage_edges`.
* **Posture:** Coverage-first — the write path already shipped (095-F), so this
  is expected to go GREEN once written; if it reveals a gap, that gap is filed,
  not fixed inline (freeze-scope; see
  `docs/compound/feature-test-flake-fix-in-scope-not-shared-infra-2026-07-11.md`).
* **Width isolation:** Pure test domain — must NOT carry any production-code
  change (this is why V4 is not merged with V5).

### Unit X1 — Extractor-version bump + re-extraction rollout validation (domain: lineage-model/freshness) — test-first, LANDS LAST
* **Files:** `src/models/lineage.rs` (the `CURRENT_EXTRACTOR_VERSION` const) plus
  a freshness re-extraction assertion (unit in `lineage.rs::tests` and/or an
  assertion reusing `notebook_lineage_freshness_test.rs` patterns).
* **Change:** Bump `CURRENT_EXTRACTOR_VERSION` `"1.0.0"` → `"1.1.0"` **exactly
  once**, and assert the resulting freshness-token change re-extracts a
  previously-stamped notebook (C4 path), so V2/V5/W1/W2's tightened behavior
  reaches already-indexed notebooks.
* **Real dependency:** Must land after V2, V5, W2, W1 so the single version bump
  reflects the complete new behavior and there is exactly one edit to the const
  (no same-line conflict, no double bump).

## Dependency Graph

```text
V2 (097.001-T) ─┐
V5 (097.002-T) ─┤
W2 (097.003-T) ─┼──► X1 (097.006-T)   [version bump reflects all behavior changes]
W1 (097.004-T) ─┘
V4 (097.005-T)  (parallel; test-only; no edge — does not change extractor output)
```

* **Real edges (encoded as `blocks`):** `097.006-T` depends on `097.001-T`,
  `097.002-T`, `097.003-T`, `097.004-T`.
* **Deliberately parallel (no edge):** V2, V5, W2, W1 are independent code paths;
  V4 tests already-shipped happy-path behavior and does not depend on the
  hardening fixes. V5↔W2 share a file but are serialized by the single-branch
  build model, not by a logical dependency.
* No cycles.

## Decisions and Rationale

1. **Six tasks, not five.** The five card items are V2/V4/V5/W1/W2. X1 (version
   bump + re-extraction validation) is added because tightening extractor output
   without invalidating the freshness stamp leaves stale/false lineage on
   already-indexed notebooks (C4). A plan-reviewer would flag its absence as a
   P1 gap. X1 is a distinct atomic milestone with a real fan-in dependency, so it
   is its own task rather than an implicit AC.
2. **V5 and W2 stay separate despite sharing a file.** Each has a distinct
   predicate, distinct call site (`resolve_path` vs `resolve_table`), and
   distinct negative-test matrix. Merging would push the combined task past the
   "<4 core test scenarios" heuristic and blur two atomic milestones. Same-file
   contention is handled by sequential single-branch builds, not by a merge.
3. **V4 is not merged with any code task.** Width isolation forbids combining a
   `tests/integration` unit with a production-code unit. The operator's suggested
   "V4+V5" pairing is declined for this reason; V4 and V5 do not share a fixture
   (V4 is a happy-path e2e edge assertion; V5 is a malformed-prefix negative
   unit test).
4. **Validation at resolve time, fail-closed.** V5 validates the prefix in
   `resolve_path` (before binding) rather than mutating config, keeping the trust
   boundary in the resolver where the edge is actually minted.
5. **Version bump is minor (1.0.0 → 1.1.0).** The change is a precision-tightening
   behavior change with no schema break, so a minor bump is the correct freshness
   invalidation signal.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| V2 region-aware scanner mis-handles nested/escaped quotes and drops a genuine INSERT (recall regression). | Keep a positive "genuine INSERT still normalizes" test; model only the flat lexer states Spark-SQL uses; escaped-quote handling per dialect (`''` doubling) covered by a fixture. |
| W1 over-invalidates and breaks the legitimate single-read→write happy path. | Only invalidate when the variable is **already** tracked; the first read still binds. Existing happy-path + atomic-bind tests are the regression guard. |
| V5/W2 reject a previously-accepted-but-actually-valid identifier/authority (recall regression). | Grammar/shape predicates match the documented Spark unquoted-identifier and `scheme://authority` forms exactly; quoted identifiers are out of v1 scope (already dropped). |
| X1 version bump forces a full re-extraction sweep of all indexed notebooks (runtime cost). | This is the intended, bounded U4b behavior; validate the re-extraction path rather than suppress it. Rollout note carried into closure. |
| Same-file (lineage.rs) churn across V5/W2/X1 causes harness/merge friction. | Single-branch sequential build; X1 (const bump) lands last after V5/W2 to avoid double edits to the same region. |

## Plan Hardening Signals (REQUIRED)

* **Public API / schema / contract change:** ABSENT for MCP tool or DB schema.
  PRESENT (soft): the *observable lineage-subgraph output* changes — some edges
  that v1 previously emitted will no longer be emitted (precision-tightening,
  fail-closed direction only).
* **Security / auth / permission / compliance:** ABSENT. (V5 tightens a *trust
  allowlist* shape check, which is precision-hardening, not an auth surface.)
* **Migration / backfill / destructive / irreversible:** PRESENT. X1 bumps the
  extractor version, which forces re-extraction of all already-indexed notebooks
  (backfill/rollout). No destructive data drop; re-extraction is idempotent.
* **External integration / operator checkpoint / external dependency:** ABSENT.
* **High runtime / rollout / rollback risk:** PRESENT (moderate). The
  version-bump-driven re-extraction touches every indexed notebook; rollback is a
  version revert.

**Requires plan hardening: yes**

## Runtime Verification and Closure

* **Changed runtime surfaces:** the indexer's lineage extraction path (background
  ingestion) and the persisted `lineage_derives_from` edge set. No CLI/MCP tool
  signature change; the read surface (`query_graph` traversal, `lineage_read_surface_test.rs`)
  is unchanged.
* **Runtime verification before absorbed:** V4 proves the e2e authority-bound
  edge still emits; X1 proves a version bump re-extracts a stale-stamped notebook;
  the lineage precision/recall test (`lineage_precision_recall_test.rs`) must
  hold its precision floor (0 false edges) after all fixes.
* **Operational closure artifact:** a rollout note in the shipment/closure
  recording the `1.0.0 → 1.1.0` extractor bump, its re-extraction implication,
  and the rollback trigger (revert the const if the precision floor regresses).

---

## Plan Hardening

Hardening was required (freshness/re-extraction rollout + observable
output-behavior change). This section deepens the rollout, verification, and
rollback detail per the `plan-harden` skill.

### Risk triggers and protected invariants
* **Trigger:** extractor-output behavior change + `CURRENT_EXTRACTOR_VERSION` bump
  forcing re-extraction of already-indexed notebooks.
* **Protected invariant (P0):** lineage precision floor — 0 false
  `lineage_derives_from` edges (013-D / A5). Every unit may only *remove* edges in
  the fail-closed direction; none may add a new edge class or relax an existing
  guard.
* **Protected invariant:** the `table::{authority}::{name}` canonical-id
  injectivity guard (the existing `::`/`:` collision check) must remain intact
  when W2 adds component-grammar validation.

### ProposedAction / ActionRisk / ActionResult
| ProposedAction | ActionRisk | Approval | Expected ActionResult |
|---|---|---|---|
| Bump `CURRENT_EXTRACTOR_VERSION` 1.0.0→1.1.0 (X1) | Medium (forces re-extraction sweep of all indexed notebooks) | Not required (bounded, idempotent, established U4b mechanism) | All notebooks re-extracted against tightened extractor on next index; freshness token changes; stale lineage cleared |
| Tighten `normalize_spark_insert` / `resolve_path` / `resolve_table` / `resolve_cell_candidates` (V2/V5/W2/W1) | Low–Medium (recall regression on genuine inputs) | Not required (fail-closed direction; guarded by positive recall tests) | Fewer false edges; genuine edges preserved by positive tests |
| Add V4 e2e integration test | Low (test-only) | Not required | New RED→GREEN coverage guard; no production change |

### Deepened verification
* **Environment precheck:** `cargo dev-test` green baseline on branch before edits;
  engram daemon green (confirmed) for planning-time code-site verification.
* **Target scenarios:** each unit's negative + positive matrix (above); plus the
  cross-unit precision/recall regression test and the freshness re-extraction test.
* **Blocked-path handling:** if V4 reveals an e2e gap (edge NOT produced through
  `ingest_all_sources` despite a valid config), file a follow-up rather than
  fixing shared ingestion infra inline (freeze-scope).

### Rollback
* **Trigger:** post-merge precision floor regression (any false
  `lineage_derives_from` edge) OR a re-extraction sweep failure.
* **Procedure:** revert the `CURRENT_EXTRACTOR_VERSION` bump (restores old
  freshness token → halts forced re-extraction) and revert the offending unit's
  commit. Each unit is an independent commit to keep rollback granular.

### Monitoring / operational closure / operator checkpoints
* **Signal:** lineage precision/recall metric emitted by
  `lineage_precision_recall_test.rs`; re-extraction count on next index.
* **Owner:** Ship (build + closure). **Validation window:** first full re-index
  after merge.
* **Operator checkpoint:** none required (no external integration; bounded local
  re-extraction).

### Learnings and instruction files consulted
* `docs/compound/best-practices/sql-quoted-identifier-resolution-candidates-list-2026-04-29.md` (V2)
* `docs/compound/best-practices/python-bare-call-edge-extraction-body-scoped-2026-07-20.md` (W1)
* `docs/compound/feature-test-flake-fix-in-scope-not-shared-infra-2026-07-11.md` (V4 freeze-scope)
* `.github/instructions/constitution.instructions.md` (Safety-First Rust, Test-First, Task Granularity)
* Reference plan `docs/exec-plans/2026-07-22-spark-notebook-data-lineage-plan.md` (013-D precision floor, C4 freshness, AR-01/AR-02/AR-07/AR-29 doctrine)

### Unresolved operator decisions that still block safe execution
* None. Version-bump policy (minor bump, re-extraction accepted) is decided above.

---

## Constitution Check

* **I. Safety-First Rust:** all edits `Result<_, EngramError>`-propagating, no
  `unwrap`/`expect`, `#![forbid(unsafe_code)]` unaffected; clippy pedantic must
  stay clean. PASS.
* **II. Test-First:** every unit is test-first/characterization-first with a RED
  harness before GREEN. PASS.
* **III/IV. Workspace isolation / CLI containment:** no path or workspace
  boundary change. PASS.
* **Task Granularity:** each unit <3 files, single domain, atomic milestone. PASS.

## Quality Gates (pre-merge, constitutional order)
1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
3. `cargo test --all-targets` (incl. lineage precision/recall floor)
4. `cargo audit`

## v1 Limitations & Deferred Items (unchanged by this plan)
* Temp-view lineage (Unit 5 of 095-F) remains DEFERRED — not in scope.
* Cross-cell `df` propagation remains out of v1.
* Quoted (non-unquoted-grammar) table identifiers remain dropped (fail-closed);
  W2 validates the *unquoted* grammar only.

## References
* Card: `.backlogit/queue/097-F.md`
* Reference plan: `docs/exec-plans/2026-07-22-spark-notebook-data-lineage-plan.md`
* Feature 095-F (shipped v1), PR #284 review cycles 4–5
* Code sites: `src/services/parsing/sql.rs`, `src/models/lineage.rs`,
  `src/services/parsing/python.rs`, `src/models/config.rs`,
  `src/services/ingestion.rs`

---

## Plan Review

**Gate decision: PASS** (no P0/P1 findings; hardening signals present and the
`## Plan Hardening` section is complete with `ProposedAction`/`ActionRisk`
classification, so the strict-safety and hardening gate conditions are
satisfied). Proceed to `harvest`.

Personas run: Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor,
Learnings Researcher (always-on), Architecture Strategist (always triggered),
Security Lens Reviewer (triggered — V5 validates a *trust allowlist* shape).
Agent-Native Parity Reviewer NOT triggered (no MCP-tool/agent-facing surface
change; the lineage read surface is unchanged).

### Findings by severity

**P0 — none.**

**P1 — none.** (The one candidate P1 a reviewer would raise — "extractor-output
behavior change with no freshness invalidation" — is pre-empted by Unit X1, which
bumps `CURRENT_EXTRACTOR_VERSION` and validates re-extraction.)

**P2 (record as backlog-follow-up awareness, non-blocking):**
* **[Rust Reviewer]** V2's region-aware scanner must handle Spark-SQL escape
  conventions — doubled single quotes (`''`) inside string literals and doubled
  backticks inside `` `…` `` identifiers — or it may exit a quoted region early
  and rewrite a `INSERT` that is still quoted. *Disposition:* add an escaped-quote
  fixture to V2's test matrix (already implied by "quoted-identifier regression
  fixture"; made explicit here).
* **[Architecture Strategist]** W1 changes observable dataflow semantics
  (second-read → no edge). Confirm no *other* caller of `resolve_cell_candidates`
  relies on last-write-wins rebind. *Disposition:* single caller (per-cell notebook
  router); the plan's regression tests (atomic-bind, per-form rebind) guard it.

**P3 (advisory):**
* **[Learnings Researcher]** The cited SQL quoted-identifier learning is about
  *reference resolution* (candidates-list), not *pre-parse normalization*. It is
  applied here only for quote-form awareness — correct, but do not import the
  candidates-list resolution pattern into the normalizer (out of scope).
* **[Security Lens Reviewer]** V5 tightens (never loosens) the storage-authority
  trust check; no new trust is granted. Confirm the predicate rejects
  `scheme://authority?query` and `scheme://authority#frag` as well as path
  components. *Disposition:* the plan's predicate already lists `/`, `?`, `#`.
* **[Scope Boundary Auditor]** Unit X1 is a sixth task beyond the five enumerated
  card items. *Disposition:* accepted — X1 is the *rollout mechanism* for the
  in-scope fixes (not new capability); its absence would itself be a P1 gap. No
  scope creep into temp-view lineage, cross-cell propagation, or config.rs
  rewrites was found.

### Learnings applied (Learnings Researcher)
No plan contradiction with any prior resolution. Three compound learnings
surfaced and applied (V2 SQL quoting, W1 Python body-scoped extraction, V4
freeze-scope test discipline).

### Constitution check (Constitution Reviewer)
Principles I–IV and Task Granularity all PASS (see `## Constitution Check`). No
`unwrap`/`expect`, all fail-closed, single-domain units, atomic milestones.

### Runtime verification & closure readiness
Present and specific: V4 (e2e edge), X1 (re-extraction), precision-floor
regression guard, rollback trigger (version revert), owner (Ship), validation
window (first full re-index). No gaps.

### Gate rationale
Hardening required and satisfied; risky actions classified; no P0/P1; P2/P3 items
are test-completeness/advisory and recorded above for the executor. **Ready to
harvest into feature 097-F.**
