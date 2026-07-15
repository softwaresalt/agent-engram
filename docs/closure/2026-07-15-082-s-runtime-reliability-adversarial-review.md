---
title: "Adversarial multi-model review — 082-S runtime reliability & concurrency hardening (086-F)"
type: closure
date: 2026-07-15
slug: 082-s-runtime-reliability-adversarial-review
subject_commit: 9257917bdd30425c7bbc6718388c72779f5555de
subject_base: a6b09258f0fbe8736e37dc712dd10604df41d58e
subject_branch: feat/086-runtime-reliability
scope: src tests Cargo.toml
reviewers: 5
review_models:
  - reviewer-a: claude-opus-4.8 (Tier 3 frontier)
  - reviewer-b: gpt-5.6-sol (Tier 3 frontier)
  - reviewer-c: gemini-3.1-pro-preview (Tier 3 frontier)
  - reviewer-d: claude-sonnet-4.6 (Tier 2)
  - reviewer-e: mai-code-1-flash-picker (Tier 1)
verdict: SHIP-WITH-FIXES
gate_blocking: true
---

# Adversarial Review — Shipment 082-S (`feat/086-runtime-reliability`)

**Date:** 2026-07-15 · **Base:** `a6b0925` (Stage) · **Tip:** `9257917`
**Scope:** `git --no-pager diff a6b0925..HEAD -- src tests Cargo.toml` (8 files, +833/−103)
**Mode:** Multi-model consensus, static analysis only (no cargo — concurrent suite held target lock).

---

## 0. VERDICT — SHIP-WITH-FIXES

- **0 P0** after aggregation (no data-corruption-at-scale, no daemon crash, no security bypass).
- **2 gating P1 (HIGH confidence)** — must remediate or explicitly accept before PR: **F1** (086.001 `:replace` masking → false success on a destructive op) and **F2** (086.002 Err-only retry whose premise is contradicted by the repo's own evidence).
- **2 P2** (cheap / follow-up), plus advisories.
- The **security-critical guard (086.003)** is **SOUND** — no bypass found. Git isolation is **clean**.

**Minimum-to-unblock** in §6.

---

## 1. Reviewer Panel — cross-model diversity proof

| Reviewer | Model | Vendor / Family | Tier | Effort | Independent stance |
|---|---|---|---|---|---|
| **A** | `claude-opus-4.8` | Anthropic | T3 frontier | high | SHIP-WITH-FIXES (086.002 SOUND — missed retry-defeat) |
| **B** | `gpt-5.6-sol` | OpenAI | T3 frontier | high | BLOCK-leaning (raised P0 + 2×P1) |
| **C** | `gemini-3.1-pro-preview` | Google | T3 frontier | high | BLOCK-leaning (raised P0 crash-claim) |
| **D** | `claude-sonnet-4.6` | Anthropic | T2 standard | high | SHIP-WITH-FIXES (2×P2) |
| **E** | `mai-code-1-flash-picker` | Microsoft | T1 fast | medium | SHIP-WITH-FIXES (declined first pass on adversarial-security framing; relaunched neutral) |

Diversity: **4 vendors, 3 tiers**. Reviewers genuinely disagreed on severity **and** on the soundness of 086.002 (A: SOUND; B/C: UNSOUND; D: CONDITIONAL) — exactly the split-brain signal an adversarial panel exists to surface.

**Evidence sub-agents** (`gpt-5.6-terra`, read-only): git-isolation audit · cozo panic-vs-Err evidence hunt · 041.002-T traceability + test-nature. Consensus assembly + source verification performed by the aggregator (this agent).

---

## 2. Decisive evidence (aggregator-verified against source + repo docs)

| Fact | Evidence | Bearing |
|---|---|---|
| cozo pinned **0.7.6** | `Cargo.lock` | Version under scrutiny |
| **panic = unwind** (no `panic="abort"` in any profile) | `Cargo.toml` profiles | A `DbInstance::new` panic does **not** abort the process |
| Open runs in `spawn_blocking`; `JoinError` mapped | `mod.rs:142-189` | A panic is **contained** → daemon survives (refutes C's "crash") |
| Repo doc: **sequential restart** triggers cozo internal `unwrap()` **PANIC**, "rather than propagating an error" | `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md:39-48` | **Contradicts** author's "surfaces as Err" premise for the exact residual 086.002 targets |
| Retry tests inject a **fabricated** `Err("database is locked")` only | `mod.rs:446-501` | Efficacy validated against a stub, **not** real cozo busy behavior |
| `cli_error` returns **2** | `output.rs:111-114` | 086.003 exit-2 confirmed |
| Guard runs **before** lock/connect/mutation | `migrate.rs:75 < 90 < 102 < 116` | Refusal mutates nothing (also test-verified, no-mutation assert) |
| `snapshot_dispatch_context` holds **both** read guards together | `state.rs:245-251` | Read-side tear eliminated |
| Writer takes **two separate** write locks | `state.rs:253-265` + `:361-363` | Narrow writer-side residual remains (documented) |
| No new deps (Cargo.toml = test-target only); `#![forbid(unsafe_code)]` intact | `Cargo.toml:661-665`, `lib.rs:10` | Constitution: deps/unsafe clean |
| **Git isolation PASS** | `a4dd6d3` = `.backlogit`+`docs/closure` only (0 src/tests/crates); no `resolver`/081-feature code in `src`/`tests` diff; base = Stage `a6b0925` (not 081 HEAD `4b68c3f`) | Clean shipment |
| 041.002-T linkage **present-but-weak** | `.backlogit/queue/086.00{1,2}-T.md:21` "Interim SQLITE_BUSY mitigation — relates to blocked 041.002-T"; **no code comment** names it | Traceable in backlog, not in source |

---

## 3. Consensus findings

Confidence: **HIGH** = all/near-all reviewers + source-confirmed · **MEDIUM** = majority/minority + confirmed · **LOW** = single reviewer.
Every P0/P1 was re-verified by the aggregator against the cited source (`Src✓`).

| ID | Change | Sev | Conf | Agree | Location | Verified issue | Remediation |
|---|---|---|---|---|---|---|---|
| **F1** | 086.001 | **P1** | **HIGH** | 5/5 | `schema.rs:181-194,363,389-398` | `classify_script_error` maps any error containing `already/defined/conflicts/existing`→`Ok(())`. This is a `:create` idempotency concept but is now applied to the `:replace` migrate **and** rollback rewrites. On **rollback** the durable marker is written **first** (`:389`); a swallowed `:replace` drop then yields **false success** (CLI prints `resolution_column_dropped:true`) with the column still present, marker=true, and singletons already retracted. **Not self-healing** — next bootstrap short-circuits on column-present (`:345-347`) and never consults the marker. `Src✓` | Split the driver: swallow `AlreadyExists` **only for `:create`**; `:replace` retries **on busy only**, every other error fatal. Optionally assert `calls_edge_has_resolution` shape after the rewrite. |
| **F2** | 086.002 | **P1** | **HIGH** | 3/5 + repo-doc | `mod.rs:175-178,268-287` | Err-only retry (no `catch_unwind`). The serialization lock prevents the **concurrent**-open panic, but the **sequential/cross-process reopen** panic — the exact transient 086.002 targets — is documented in-repo as a cozo internal `unwrap()` **panic**, which unwinds **past** the retry loop (zero retries). Daemon survives (unwind + `spawn_blocking`, verified) so **not a crash/regression**, but the flagship retry is **likely ineffective** for its stated purpose and its efficacy is validated only against a fabricated `Err`. Code comments overclaim ("surfaces as Err", "never an unwrap panic"). `Src✓` | Minimum: correct the comments to match reality + track under 041.002-T. Recommended: wrap `DbInstance::new` in `std::panic::catch_unwind`, classify the payload via `is_retryable_open_error`, retry busy-panics / re-raise others — **or** add a real integration test proving cozo 0.7.6 serialized busy-open returns `Err`. |
| **F3** | 086.001/002 | **P2** | **HIGH** | 5/5 | `schema.rs:228`, `mod.rs:286` | `unreachable!()` on production paths. **Provably unreachable** (final loop iteration always returns) and clippy-pedantic-clean (the lint is `restriction`, not `pedantic`), so **no runtime risk**; but it is a panic macro in production, flagged by all 5 reviewers (E over-rated P0). `Src✓` | Capture the last error and `return Err(map_db_err(...))` after the loop; delete the macro. Trivial. |
| **F4** | 086.004 | **P2** | **MEDIUM** | 2/5 + agent | `state.rs:253-265,361-363` | Read-path tear fixed (both guards held), but the **writer** still publishes workspace then config in **two awaits** — a narrow residual where a status read sees new-workspace/old-config. The new test drives real writers but routes transitions through a neutral workspace `N` and asserts only on A/B cross-pairs, so it **cannot detect** the residual writer-side tear. `Src✓` | Follow-up (non-blocking): add `set_workspace_and_config` holding both write locks; extend the test to exercise real A→B/B→A ordering; state the residual explicitly in-code. |
| **F5** | 086.003 | **P3** | **LOW** | 1/5 | `migrate.rs:66-74` | (B rated P0.) Containment ≠ exclusivity: workspace B pointing `ENGRAM_DATA_DIR` **inside** workspace A is accepted by A's migrate-down while B's daemon may write concurrently. **Downgraded** — requires deliberate misconfig, is **not a regression**, and is **explicitly documented as the deferred 013-D interim** in-code. `Src✓` | No action this PR; ensure 013-D tracks full path-scoped exclusivity. |
| **F6** | cross-cut | **P3** | **LOW** | agent | `schema.rs`/`mod.rs` | 041.002-T durable-fix linkage lives in backlog task files but **no code comment** names it. | Add `// interim SQLITE_BUSY mitigation; durable fix tracked by 041.002-T` at both retry helpers. |
| ~~F7~~ | 086.002 | — | REFUTED | 1/5 | `mod.rs:202,258-259` | C alleged "unchecked arithmetic" (`attempt+1`, `<<`, `%`). **False positive** — shifts capped via `.min(5)`/`.min(4)`, `attempt` ≤ 19, `%` guarded for 0; cannot overflow. | None. |

### Verified-SOUND (positives)

| ID | Area | Conf | Basis |
|---|---|---|---|
| P-A | 086.003 guard — **no false-negative/bypass** | HIGH (4/5) | `canonicalize` resolves symlinks/8.3/case for existing prefix; `normalize_lexical` resolves `..` so post-non-existent-prefix escapes land outside → rejected; `PathBuf::starts_with` is **component-wise** (sibling `<ws>-evil` correctly rejected); verbatim/UNC stripped symmetrically. Only false-positive is intentional (`.engram` symlinked out → fail-closed). Test asserts exit 2 **and** no mutation. `Src✓` |
| P-B | 086.004 read-path atomicity + behavior preservation | HIGH (5/5) | Both read guards held; `NotSet`, stale-detect+`update_workspace`, `connect_db`, `db_path`, and all 9 returned fields preserved. `Src✓` |
| P-C | Isolation / deps / unsafe / jitter / backoff | HIGH | Git isolation PASS; no new deps; unsafe forbidden intact; jitter bounded + div-by-zero guarded (tested); total backoff ≈1.6–2.4 s ≪ 30 s deadline. |

---

## 4. HIGH-confidence P0/P1 that GATE the PR

- **F1 (P1, HIGH, 5/5)** — 086.001 `:replace` masking → false success on destructive `migrate-down` rollback.
- **F2 (P1, HIGH, 3/5 + repo-doc)** — 086.002 Err-only reopen-retry premise contradicted by repo evidence; flagship mechanism likely inert for its target transient.

No P0. F3 (unreachable!) is HIGH-consensus but **not a runtime gate** (provably unreachable) — bundle it as a cheap in-PR cleanup.

---

## 5. Per-change decision soundness (answers to the operator's scrutiny points)

**086.001 — CONDITIONAL (fix F1).** Masking risk on `:replace`: **yes**, on the rollback path (false success + marker/column divergence, not self-healing). Idempotency guards are sufficient to gate *entry* but **not** the `:replace` *outcome*. Retry is bounded (20) and panic-free **except** the `unreachable!()` (F3). AlreadyExists-before-Busy ordering is harmless (busy text `"database is locked"` contains none of the swallow keywords).

**086.002 — CONDITIONAL / UNSOUND-as-claimed (fix or prove F2).** Reasoning is **half right**: the lock prevents the *concurrent* panic, but the *sequential/cross-process* reopen panic (documented in-repo) is **not** prevented and **not** retried. Can cozo panic on busy? **Yes** per `cozodb-sqlite-lock-panic-2026-05-01.md`. Deadlock/livelock from retrying inside the held mutex+fd-lock? **No** — the retry's sole legitimate opener holds the lock, the transient is self-clearing OS lock-lag, and the budget is bounded. Jitter entropy adequate/bounded? **Yes** (RandomState+nanos, `%` guarded, tested). Within 30 s deadline? **Yes** (~2.4 s). Net: **no crash, no regression**, but the claim must be corrected and the mechanism completed or evidenced.

**086.003 — SOUND.** No bypass across symlinks / 8.3 / case / UNC / `..`-escape / trailing-sep / mixed-sep / drive-relative. Guard runs **before** DaemonLock/connect_db/retract/drop (verified + tested no-mutation). Exit **2** confirmed. Fail-closed is truly non-destructive. Only false-positive is intentional. Cross-workspace inside-shared gap (F5) is real but documented/deferred (013-D).

**086.004 — SOUND for its read-path scope.** Wide tear (across the `connect_db` round-trip) removed; all behavior preserved; **no regression**. Writer-side residual (F4) is genuine, narrow, out-of-scope, and should be documented explicitly + tracked.

**Cross-cutting.** SQLITE_BUSY handling consistent across both helpers (both key on `locked`/`busy`). Blast radius on the shared `connect_db` path is **not widened** — 086.002 is additive, bounded, and panic-contained. 041.002-T linkage present-but-weak (F6).

---

## 6. Minimum-to-unblock

1. **F1 (required):** stop swallowing `AlreadyExists` for `:replace`; migrate/rollback retry on **busy only**, all else fatal. (+ optional post-rewrite shape assert.)
2. **F2 (required):** resolve the premise contradiction — **either** add `catch_unwind` payload-classification retry **or** land a real-cozo-behavior test proving `Err`; **and** correct the "surfaces as Err / never an unwrap panic" comments. If shipping the interim as-is, the **absolute** minimum is fixing the misleading comments + a tracked 041.002-T follow-up.
3. **F3 (cheap, recommended in-PR):** delete both `unreachable!()`, return the captured error.

**Non-blocking follow-ups:** F4 (atomic writer + test), F5 (013-D tracking), F6 (code-comment linkage).

---

## 7. Backlog work items (P0/P1)

```yaml
- type: bug
  title: "086.001: :replace migrate/rollback route through AlreadyExists-swallowing retry → false success"
  description: "retry_cozo_script/classify_script_error map errors containing already/defined/conflicts/existing to Ok(()). Applied to the calls_edge :replace migrate+rollback, a genuine rewrite failure is masked; on rollback the durable marker is set first, so a swallowed drop yields false success + marker/column divergence + already-retracted singletons, not self-healing."
  file: "src/db/cozo_backend/schema.rs"
  line: 219
  severity: "P1"
  confidence: "HIGH"
  fix: "Swallow AlreadyExists only for :create; :replace retries on busy only, all else fatal; optionally assert post-rewrite column shape."
  linked_review: "docs/closure/2026-07-15-082-s-runtime-reliability-adversarial-review.md"

- type: bug
  title: "086.002: Err-only reopen-retry cannot absorb cozo's documented sequential-reopen SQLITE_BUSY panic"
  description: "open_db_with_retry retries only Err. Repo doc cozodb-sqlite-lock-panic-2026-05-01.md states the sequential/restart busy triggers cozo 0.7.6's internal unwrap panic (not Err), which unwinds past the retry loop (zero retries). Daemon survives (unwind + spawn_blocking) but the retry is inert for its target transient; efficacy is tested only against a fabricated Err, and comments overclaim 'surfaces as Err'."
  file: "src/db/cozo_backend/mod.rs"
  line: 268
  severity: "P1"
  confidence: "HIGH"
  fix: "Add catch_unwind payload-classification retry OR prove real cozo busy is Err via test; correct the code comments; link 041.002-T."
  linked_review: "docs/closure/2026-07-15-082-s-runtime-reliability-adversarial-review.md"

- type: chore
  title: "086.001/002: remove unreachable!() from bounded-retry loops"
  description: "Provably-unreachable unreachable!() at schema.rs:228 and mod.rs:286 (panic macro in production, flagged 5/5). No runtime risk but replace with an explicit terminal Err return."
  file: "src/db/cozo_backend/mod.rs"
  line: 286
  severity: "P2"
  confidence: "HIGH"
  fix: "Capture last error; return Err(map_db_err(..)) after the loop."
  linked_review: "docs/closure/2026-07-15-082-s-runtime-reliability-adversarial-review.md"
```

---

## 8. Remediation (Ship — post-review, commit `7fbc9ef`)

All gating findings remediated in-PR before opening the PR. `cargo fmt`/`clippy -D pedantic`
clean; +5 tests; 282 lib tests green; migrate/rollback + connect_db + atomicity suites green.

| ID | Sev | Disposition | Evidence |
|---|---|---|---|
| **F1** | P1 | **FIXED** | `retry_cozo_script`/`run_script_retrying` gained an `allow_already_exists` flag: `true` only for `:create` bootstrap + `CREATE_SCHEMA_META`; `false` for the `:replace` migrate/rollback so "already exists" is now **fatal** there (no false success). schema_meta marker `:put` also routed through the busy-tolerant retry. New test `retry_cozo_script_surfaces_already_exists_as_fatal_when_disallowed`. |
| **F2** | P1 | **FIXED** | Added `catch_busy_panic`: wraps `DbInstance::new` in `catch_unwind`, classifies the panic payload via `is_retryable_open_error`, returns a retryable `Err` for a busy panic and **re-raises** any non-busy panic. Wired into `connect_db`; misleading "surfaces as Err" comments corrected to "cozo unwraps internally → PANIC". New tests: converts-busy-panic / passes-through-results / re-raises-non-busy-panic. |
| **F3** | P2 | **FIXED** | Both `unreachable!()` retry-loop guards replaced with a returned `EngramError` (`schema.rs` retry core + `mod.rs open_db_with_retry`). |
| **F6** | P3 | **FIXED** | In-code `// Interim SQLITE_BUSY mitigation — durable fix tracked as 041.002-T` added at the `connect_db` reopen-retry block. |
| **F4** | P2 | **DEFERRED (follow-up)** | Writer-side (`set_workspace` + `set_workspace_config`) two-await residual is out of 086.004's single-width read-path scope. Tracked as a follow-up stash for Stage triage (atomic `set_workspace_and_config` + extended test). Documented in-code that the concurrent writer updates both in two separate awaits. |
| **F5** | P3 | **DEFERRED (013-D)** | Containment ≠ exclusivity (a workspace pointing `ENGRAM_DATA_DIR` inside another) is the deliberately-deferred full cross-workspace exclusivity mechanism recorded in deliberation 013-D; the fail-closed guard is the conservative interim. No regression. |
| F7 | — | REFUTED | No arithmetic overflow (shifts capped, `%` guarded). |

**Verdict after remediation:** the two gating P1s are resolved and the cheap P2/P3s folded in; remaining items are genuinely out-of-scope follow-ups (F4) or already-deferred design (F5). Shipment is PR-ready.
