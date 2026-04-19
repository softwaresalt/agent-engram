---
type: stage-session
date: 2026-04-19
agent: stage
session_focus: "spike — CozoDB + Datalog migration feasibility"
stash_processed: ["23F4C476"]
artifacts_produced:
  - "docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md"
deferred_entries:
  - id: "1330B629"
    reason: "Harness workflow policy — out of session scope"
  - id: "0523404D"
    reason: "Tree-sitter language expansion — referenced as roadmap context but separately tracked"
  - id: "D715B3EE"
    reason: "SQL dialect grammar coverage — referenced as roadmap context"
  - id: "47F34E2C"
    reason: "Markdown grammar coverage — referenced as roadmap context"
---

## Session Summary

Stash entry `23F4C476` ("Spike: research refactoring the Engram server from
SurrealDB to CozoDB ... including switching to Datalog ... and extracting
patterns from CIE") was routed to the **spike** skill (investigative shape,
not deliberation). Treated as a deep research spike per operator instruction.

## Pipeline State

* **Triage:** Single entry processed; classified as investigative → spike,
  not deliberate.
* **Grouping:** N/A (single-entry session, no contextual grouping needed).
* **Spike skill:** Findings artifact written.
* **Deliberation:** Not applicable — spike produces findings, not decisions.
* **Implementation plan:** **Not generated this session.** Blocked on CIE
  reference resolution (see Decisions §1). Findings artifact is marked
  `promoted_to: ["plan"]` to indicate intended next step once the gap is
  closed.
* **Plan-harden / plan-review / harvest / shipment:** N/A — gated on plan
  generation.

## Decisions and Rationale

1. **Routed as spike, not deliberation.** The entry text uses the word
   "spike" and is investigative ("research", "extracting patterns") rather
   than option-comparison. Per Stage agent rules, investigative entries
   route to the spike skill, which produces a findings artifact in
   `docs/decisions/` (not a backlog item per se).
2. **Confidence: medium, not high.** The capability fit and migration shape
   are well-grounded in directly-verified code reads and documented CozoDB
   behavior. The CIE reference is unresolved (no external network in this
   session), and that pattern set will materially shape the Datalog rule
   library design. Honesty about the gap is more useful than false
   precision.
3. **Recommendation: proceed (phased), not pivot/defer/abandon.** The
   architectural fit is strong; the JSONL ground truth derisks parallel-DB
   migration; the queries that hurt most under SurrealQL get
   structurally-better Datalog forms.
4. **Did not invoke impl-plan in this session.** Phase 0 of the proposed
   migration includes a benchmark spike and CIE resolution — these should
   precede plan generation, not follow it. Operator should decide whether
   to (a) resolve CIE and run impl-plan now, or (b) split phase 0 out as
   its own follow-up spike.

## Failed Approaches Considered

* **SurrealQL → Datalog translator layer.** Rejected — translation hides
  the expressivity gain that motivates the move.
* **SQLite + Rust Datalog crate (crepe/ascent) + separate vector index.**
  Rejected — recreates SurrealDB's multi-component shape; loses the
  single-engine simplicity that is CozoDB's main architectural argument.

## Open Questions for Operator

1. **CIE reference (highest priority).** Repository URL, paper, or internal
   handle. Without this, the planning phase will substitute the §4
   prior-art candidates (Glean, CodeQL, Soufflé, Stack Graphs) as the
   pattern source.
2. **Storage backend preference.** Cozo over RocksDB (more performant, C++
   dep) vs. Cozo over SQLite (pure-Rust path).
3. **Whether to split Phase 0 (benchmark + CIE) into its own preliminary
   spike, or roll it into the migration plan as Phase 0.**

## Expansion Pass (same session, after operator "keep working" instruction)

Findings artifact extended from ~28 KB to ~58 KB with 9 new sections:

* **§7** — Full CozoScript schema (1:1 mirror of `src/db/schema.rs`),
  HNSW DDL with filter expressions, normalized `commit_change` table
  replacing the embedded `commit_node.changes` array, and explicit
  reverse-lookup indexes on every edge relation.
* **§8** — Line-precise side-by-side translations of the two largest
  queries (`hybrid_graph_vector_search` queries.rs:1867–2050 and
  `graph_neighborhood` queries.rs:2067–2192). Net: ~300 LOC of imperative
  SurrealQL composition → ~70 LOC of Cozo binding + two declarative
  Datalog programs.
* **§9** — `CodeGraphBackend` trait abstraction with concrete signatures
  and the `dual_assert!` parallel-DB testing pattern (the load-bearing
  Phase-1 design).
* **§10** — Datalog rule library design (Glean-style language-prefixed
  predicates + CodeQL-style derived view library). Adopted as **default**
  in absence of CIE input.
* **§11** — Per-test acceptance matrix for all 13 DB-pinning tests:
  Pass-through / Re-skin / Replace classification.
* **§12** — Phase-0 benchmark protocol with concrete corpus
  (`.engram/code-graph/nodes.jsonl` ~1800 KB on disk), 4 workloads, and
  pass criteria (recall@10 ≥ 0.95, p95 ≤ 1.5× baseline).
* **§13** — Phased plan: 7 covering chores → ~36 tasks → ~72 hours
  human-equivalent, decomposed at the 2-hour rule. One shipment per
  phase.
* **§14** — Risk register tightened to 10 rows with verified evidence.
* **§15** — Six remaining unknowns, prioritized.

### Verified incidental findings during expansion

1. **JSONL `created_at` duplication is real.** First three lines of
   `.engram/code-graph/edges.jsonl` are identical except for `created_at`.
   Current SurrealDB RELATE path appears to append-rather-than-upsert.
   Cozo's composite primary key on `(from, to)` collapses these for free.
2. **JSONL format is engine-agnostic.** Sampled `nodes.jsonl` /
   `edges.jsonl` show no SurrealDB-specific types. Hydration parser in
   `src/services/hydration.rs` produces plain `String` IDs. Strengthens
   the Phase-1 trait abstraction — JSONL roundtrip already operates at
   the trait surface.
3. **Repo grep for "CIE" / "cozo" / "datalog" / "cozoscript" returned
   no hits** (excluding `target/`, `.git/`, `.engram/`). The CIE
   reference is unambiguously external to this codebase. Recorded.

## Next Steps

* **Operator:** Resolve CIE reference and storage backend preference
  (RocksDB vs. pure-Rust SQLite-backed Cozo).
* **Stage agent (next session):** With CIE resolved, invoke `impl-plan`
  on the findings artifact. Plan will require `plan-harden` (high blast
  radius) and `plan-review` (PASS gate) before harvest.
* **No shipment assembled this session** — premature without a reviewed
  plan.

## Continuity Notes

* The findings artifact at
  `docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md` is
  the authoritative session output and the seed for the next planning
  invocation.
* Stash entry `23F4C476` should remain in the stash until either (a) impl-plan
  is invoked and harvest creates backlog items, or (b) the operator decides
  to convert it to a backlog spike work item with `status: done` and
  `description` linking to the findings artifact.
* No backlog items created; no shipment assembled; no commits made.

---

## Pass 3 — CIE research (2026-04-19, third pass after operator unblock)

**Trigger.** Operator provided the CIE repo URL (`https://github.com/kraklabs/cie`)
to close the §4 gap that was pinning confidence at `medium`.

**Method.** Shallow-cloned `kraklabs/cie` (Go MCP server, AGPL-3.0, KrakLabs,
3.3 MB) to a scratch dir. Read `pkg/ingestion/schema.go` (full),
`pkg/ingestion/datalog.go` (mutation builder, validation, quoting rules),
`pkg/tools/trace.go` (call-graph traversal queries), `pkg/tools/semantic.go`
(HNSW search), and `docs/architecture.md` design-decision sections (Why CozoDB,
Why Vertical Partitioning, Schema v3, Example Datalog Queries).

**Key findings (extracted into spike §16).**

* CIE is the **single most relevant precedent**: same architectural envelope
  (embedded Cozo, Datalog, HNSW, JSON-RPC MCP, tree-sitter, single binary).
* CIE has shipped **3 schema versions**; v3 is **vertically partitioned**
  (metadata / code / embedding split into separate relations). Documented
  10x memory footprint reduction for typical queries. **Engram should adopt
  vertical partitioning from day 1** rather than re-discover this lesson.
* CIE uses **entity-with-stable-id** for edges (`id: String` primary key
  with deterministic construction like `"call:" + caller_id + "|" + callee_id`)
  rather than composite keys. Source comment is explicit: "store as entity with
  stable id to avoid composite-key quirks". Engram's draft schema (§7) used
  composite keys — **switch to entity-with-stable-id**.
* CIE's HNSW query syntax verified: `~rel:idx { keyed_cols | query: q, k, ef,
  bind_distance: var }` then regular Datalog joins for metadata. §8.1's
  `hybrid_graph_vector_search` translation is now directly verified, not
  extrapolated.
* CIE's recursive transitive-closure pattern verified: 3 lines of Datalog
  replace ~120 lines of Rust + single-hop SurrealQL.
* CIE validates embeddings **at ingest** (NaN/Inf, dim consistency, empty-ID
  rejection) — Engram's `gc_corrupted_embeddings` is reactive cleanup that
  should become defense-in-depth backstop after migration.
* CIE supports a **suffix-matching idiom** `(name = X or ends_with(name,
  "." + X))` for cross-namespace symbol resolution. Free correctness
  improvement for Engram tools that take user-supplied symbol names.

**Spike artifact updates.**

* Frontmatter: `confidence` `medium` → `medium-high`; added
  `cie_reference` and `cie_status` keys.
* §4: rewritten to reflect resolved CIE gap; preserves Glean/CodeQL/Soufflé
  /Stack Graphs as orienting context; CIE now load-bearing.
* §6: removed CIE gap from unknowns.
* Recommendation: confidence updated; "Why not high" reframed around
  remaining benchmark + crate-API + dimension unknowns rather than CIE.
* Next Steps / Immediate: marked CIE resolution done; added embedding
  dimension as a deferred decision.
* §10: rewritten as v1-conservative single-namespace shape matching CIE,
  with Glean prefixed-predicate pattern preserved as forward extension.
* §15: removed CIE; renumbered; added crate-API and dimension uncertainties.
* References: `CIE (referenced by operator)` updated to verified link
  with file-and-line citations.
* **NEW §16:** comprehensive evidence section (15 subsections covering
  partitioning, edge IDs, mutation idiom, HNSW, traversal, suffix matching,
  ingest validation, schema versioning, CIE rationale, capability deltas,
  risk register update, recommended schema rewrite, phased plan deltas,
  outstanding follow-ups). Spike grew from ~58 KB to ~83 KB.
* Promotion gate: removed CIE-resolution prerequisite. Spike is **ready for
  `impl-plan` promotion**.

**Files NOT in repo.** CIE clone at scratch dir was a research-only workspace;
not committed. Cleanup deferred to session end.

**Next steps for the impl-plan phase.**

1. `impl-plan` with §16-derived schema (vertical partitioning, entity-with-
   stable-id edges, validation at ingest).
2. `plan-harden` — required because DB migration is high blast radius.
3. `plan-review` PASS gate before harvest.
4. Phase 0 benchmark task (§12 protocol) unblocks remaining unknowns.

---

## Pass 4 — Embedding dimension trade-offs (2026-04-19)

**Trigger.** Operator asked for a focused research pass on whether to bump
embedding dim from 384 to 768 or 1536 alongside the CozoDB migration, given
that ingestion is heaviest at initial hydration.

**Method.** Surveyed the existing embedding code (`src/services/embedding.rs`,
`src/services/code_graph.rs`), the fastembed model catalog (`BGESmallENV15`,
`BGEBaseENV15`, `NomicEmbedTextV15`, `JinaEmbeddingsV2BaseCode`, plus
1536-dim variants), and the public benchmark literature (MTEB Retrieval,
CodeSearchNet / CoIR for code-specific). Cross-checked against CIE's choice
of 768 (nomic) / 1536 (Qodo-Embed-1-1.5B).

**Key findings (extracted into spike §17).**

* **Three independent cost axes** — inference (scales with model param count,
  not output dim), storage (scales with dim), HNSW build/search (scales with
  dim). Inference is the dominant cost on initial hydration; storage and HNSW
  are negligible by comparison.
* **Dim is the wrong knob.** Model **family** matters more than dim for code
  retrieval. A 768-dim code-specialized model (jina-base-code) typically
  beats a 1536-dim general-purpose model on CodeSearchNet by 5–15% MRR.
* **fastembed catalog limits the 1536-dim path.** CIE's specific
  `Qodo-Embed-1-1.5B` (1.5B params, 1536 dim) is NOT in fastembed; using
  it would require swapping to direct `ort` + custom HF download AND
  introduces a 30–50× hydration time penalty on local CPU. **Not viable
  for Engram's default local-CPU UX without a GPU dependency.**
* **Hydration cost ratios (informed estimate, needs Phase-0 confirmation):**
  bge-small (33M params, baseline) → jina-base-code (161M params) ≈ 3–5×
  hydration time; → 1.5B model ≈ 30–50×.
* **Read latency is small at any of these dims.** HNSW kNN at 100K vectors
  completes in single-digit milliseconds at all three dims.
* **Tiered embedding (Tier 1 / Tier 2)** in `code_graph.rs` already
  bounds per-symbol embed cost regardless of source size, so total hydration
  scales with symbol count, not LOC.

**Recommendation: unbundle the two changes.**

* Ship CozoDB migration at 384 / `bge-small-en-v1.5`. Don't combine two
  high-blast-radius changes.
* Add Phase-0 micro-benchmark (small fixture corpus, ~30 query golden set)
  comparing 384 baseline vs. 768 code-specialized (`jina-base-code`,
  `nomic-text-v1.5`). Output informs a post-migration model-swap proposal.
* Make `EMBEDDING_DIM` and `EMBEDDING_MODEL` parameterized constants,
  bake into `SCHEMA_VERSION` so a future swap is re-hydration only, not
  a code change.
* The 1.5B / 1536-dim path is a future GPU-only opt-in, NOT part of this
  migration sequence.

**Spike artifact updates.**

* **NEW §17:** comprehensive 10-subsection embedding-dim trade-off analysis
  (cost axes, fastembed catalog, quality benchmarks, hydration math,
  recommendation, schema readiness, risk register additions, what is
  deliberately not decided, closure on §15 unknown #4). Spike grew from
  ~88 KB to ~100 KB.
* §15 unknown #4 updated — now researched, decision recorded.
* Recommendation "Why not high confidence" embedding-dimension item updated.
* Next Steps "Confirm embedding dimension" updated to point at §17.

**Confidence.** Stays at `medium-high`. The model-dimension choice is no
longer a deferred unknown; it is a researched decision with a Phase-0
verification step. Remaining distance to `high` is now just the Phase-0
benchmark (HNSW perf at 384, model-swap micro-benchmark, storage backend)
and the Rust `cozo` crate API prototype.

**Spike status.** All known operator-raised questions addressed. **Ready for
`impl-plan` promotion.** Final size: ~100 KB across 17 sections + 5
references / promotion notes.
