---
type: plan-review-record
date: 2026-09-13
package: G
version: v3
plan: docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan-v3.md
source: docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v3.md
verdict: FAIL
rounds: 1
correction_round_opened: false
harvested: false
backlog_ids: none
shipment: none
pr: none
branch: chore/checkpoint-resolution-ordering-restage
---

# Package G v3 — plan-review record

**Verdict: FAIL — terminal.** Two **architecture-level P1** findings, each raised
independently by two reviewers and adjudicated architecture-level by the
Architecture Strategist. No correction round was opened. No harvest. No backlog
IDs allocated. No shipment assembled. No PR.

## Gate rule applied

The operator authorized: *"One bounded correction+confirmation only for
mechanical findings. Any architecture-level P0/P1 => stop. PASS only with zero
P0/P1. P2-only ADVISORY is not auto-harvested."*

Round 1 produced two architecture-level P1 findings. The correction budget is
explicitly restricted to **mechanical** findings, so it never opened. The gate is
terminal at round 1.

## Reviewer panel — cross-model diversity

Seven personas across seven distinct models, all run in round 1.

| Persona | Model | Verdict | Blocking findings |
|---|---|---|---|
| Rust Reviewer | `gpt-5.6-sol` | **FAIL** | 3 P1 (1 architecture-level) |
| Correctness Reviewer | `gpt-5.6-terra` | **FAIL** | 2 P0, 2 P1 (2 architecture-level) |
| Constitution Reviewer | `grok-4.6` | **FAIL** | 2 P1 (both mechanical) |
| Security Reviewer | `claude-sonnet-5` | **PASS** | none (2 P3) |
| Architecture Strategist | `claude-opus-4.8` | **FAIL** | 2 P1 (both architecture-level) |
| Scope Boundary Auditor | `gemini-3.8-flash` | **PASS** | none (1 P3) |
| Maintainability Reviewer | `claude-opus-4.7` | **FAIL** | 2 P1 (both mechanical) |

## What v3 got right — do not re-derive on a future attempt

These are settled by this round and should be inherited, not re-argued.

1. **B1 is resolved for its entire filter class.** Removing
   `max_depth`/`max_filesize` and replacing `ignore::WalkBuilder` with plain
   `std::fs` recursion makes filter-level silent omission **structurally
   unexpressible**, not merely avoided. Architecture called this "the strongest
   possible resolution". Maintainability independently confirmed hand-rolled
   recursion is more maintainable here than disabling `WalkBuilder`'s five
   default filters.
2. **The Security Reviewer — whose own v2 P2 caused B1 — explicitly declined to
   re-request bounds**, and verified that re-requesting them "would repeat the v2
   mistake with no new evidence". Probe "no speculative limits" passed **7/7**.
   The bounds question is closed. Do not reopen it.
3. **The evidence base is sound.** 92 files, all `.md`, max depth 2, max 98,546
   bytes — proving both v2 caps were inert on every real input while being
   capable of deleting the largest, most contract-dense file in the corpus. No
   reviewer disputed the measurements.
4. **G.10's positive no-omission proof is correct** (393,216-byte file, depth-10
   file, hidden file, `.gitignore`d file must all appear). Probe passed
   unanimously.
5. **The layering *direction* of B2 is right.** Keeping `run_registry`
   un-instrumented and pushing fault injection to the layer that owns each fault
   was affirmed by Architecture (P7 PASS) and Constitution (P7 PASS). The defect
   is in the matrix's *labels* and in the seam's *sufficiency*, not in the
   decision to layer.
6. **All prior affirmed v2 constraints were preserved.** No reviewer found a
   violation of, or re-litigated, the single registered `[[test]]` target, typed
   registry, absence of `verify_markdown`/`engram` coupling, zero new
   dependencies, compiling `todo!()` red phases, failure precedence, two-layer
   containment, the `AssertionScope` 3×2 matrix, or the CI `--mode select`
   contract.
7. **Scope discipline passed cleanly.** Zero P0/P1/P2 from the Scope Boundary
   Auditor: no scope creep, no padding, no YAGNI violation, the `FileAccess`
   boundary is load-bearing rather than speculative, and the seed registry stays
   at five minimal entries.

## Required explicit probes — round-1 consensus

| # | Probe | Consensus |
|---|---|---|
| 1 | Exhaustive no-filter discovery | **PASS for the filter class** (6/6 probing) — but Architecture records a surviving **non-filter** omission path (finding A) |
| 2 | Oversized / deep file cannot vanish | **PASS** (unanimous) |
| 3 | F3/F4/F5/F6 reachability and layer mapping | **FAIL** (Rust FAIL, Correctness FAIL, Architecture FAIL; Constitution PASS) |
| 4 | Windows determinism | **FAIL** (Rust FAIL, Correctness FAIL, Architecture PARTIAL FAIL) |
| 5 | No speculative limits | **PASS** (7/7) |
| 6 | All prior affirmed constraints | **PASS** (7/7) |

## Terminal findings — architecture-level

### A — Root-symlink fail-open — HIGH consensus, ARCHITECTURE-LEVEL

* **Raised by**: Correctness (P1), Architecture (P1). **Adjudicated VALID /
  ARCHITECTURE-LEVEL** by the Architecture Strategist.
* **Defect**: `validate_root` (G.9) canonicalizes the joined root, and
  `std::fs::canonicalize` **follows a symlinked root**. G.10/G.11 reject only
  symlink *entries returned during recursion* — the root itself is never
  inspected — and `RealFs::list_dir` (`std::fs::read_dir`) follows a symlinked
  root transparently. Replacing an allow-listed root such as `.github/agents`
  with a symlink pointing elsewhere inside the workspace therefore **passes
  containment and is traversed**, so the real corpus is silently bypassed and
  `prohibited` / `required` contracts pass against a substitute tree.
* **Why it is terminal**: this is a **fail-open of exactly the class v3 exists to
  eliminate**, and it directly contradicts two of the plan's own claims — "symlinks
  are rejected, never silently skipped" (Exhaustive discovery) and "symlinks are
  rejected, so an allow-listed directory cannot be re-pointed" (H6). **No unit
  owns or tests this case.**
* **Why it is architecture-level, not mechanical**: the fix requires exposing
  path-level `symlink_metadata` (lstat) through the `FileAccess` seam and
  rejecting a symlink root **before** canonicalization and traversal. That is a
  change to the boundary contract and to resolver ordering, not a wording repair.

### B — Canonicalization is not injectable → F5 not Windows-deterministic — HIGH consensus, ARCHITECTURE-LEVEL

* **Raised by**: Rust (P1), Architecture (P1). **Adjudicated VALID /
  ARCHITECTURE-LEVEL** by the Architecture Strategist.
* **Defect**: G.8 asserts `validate_root` is "pure path logic; no filesystem
  access, therefore deterministic on every platform", but G.9 requires
  `std::fs::canonicalize`. A path that is lexically contained yet canonically
  escaping can only be produced by a real on-disk symlink or junction — which on
  Windows needs elevation or developer mode, i.e. **precisely the platform trick
  the plan forbids**. `FakeFs` cannot inject `canonicalize` because
  canonicalization is not routed through `FileAccess`.
* **Why it is terminal**: the plan therefore **claims a proof it cannot deliver**
  — the identical defect class as v2's B2, reproduced in v3 at a different layer.
  Architecture: "the matrix's 'F5 proven by resolve unit (G.8, G.12)' is
  partly hollow."
* **Why it is architecture-level**: deterministic proof requires routing
  canonicalization (and path metadata) through the injectable boundary — the same
  seam redesign as finding A.

**Common root cause.** A and B are one structural problem: the two-method
`FileAccess` seam (`list_dir`, `read_to_string`) is **under-provisioned for what
the `resolve` layer asks of it**. It is sufficient for the read layer (F6) and
the discovery-failure slice of F5, but the plan additionally asks it to (a)
reject symlink *roots* and (b) prove canonical-escape containment
deterministically — and neither operation is expressible through those two
methods. Architecture: "As specified, the seam under-delivers its own contract."

## Mechanical findings — recorded, NOT corrected

The correction budget was restricted to mechanical findings and never opened,
because findings A and B are architecture-level. These are recorded so a future
attempt inherits them rather than rediscovering them.

| ID | Sev | Raised by | Finding |
|---|---|---|---|
| M-1 | **P0** | Correctness; Maintainability (P1/M2) | G.1 creates a deliberately failing placeholder `#[test]`, but **no unit owns removing it**. G.24/G.26/G.28/G.30/G.32 only *append* tests to the root file. G.35 sc.1 requires `cargo test --all-targets` green, which is impossible while the placeholder survives. The chain is closed against its own success criterion. |
| M-2 | **P0** | Correctness; Constitution (C-2/P1) | G.6 sc.3 requires `classify` / `DiagnosticCode`, which the type-surface table does not introduce until G.20, and G.6 owns only `registry.rs`. The `[R]` commit therefore **cannot compile**, violating the plan's own "No `[R]` unit may leave the crate non-compiling" rule. It is also cross-stage packing, which SC forbids. Fix: keep G.6 on `RegistryError` only; move code mapping and F3≻F7 to G.20. |
| M-3 | **P1** | Constitution (C-1); Maintainability (M1) | The Constitution Check table maps **invented principle names** (Local-first, Deterministic behaviour, Simplicity, No new dependencies, Buildable commits, Lint discipline, CI integrity, Traceability) instead of the real `.github/instructions/constitution.instructions.md` I–XI (I Safety-First Rust, IV CLI Workspace Containment, VI Single Responsibility, VII Destructive Command Approval, VIII Explicit Safety Modes, IX Git-Friendly Persistence, X Agent Context Efficiency, XI Merge Commit History Preservation). **Eight of eleven rows misalign** — the table checks a fictional constitution. Also: "17 `[R]`/`[G]` pairs" is wrong; there are **16** pairs plus the unpaired G.1 bootstrap. |
| M-4 | **P1** | Rust (G3-03) | Convention **SC is internally inconsistent and the units do not comply with it**. SC forbids packing different `DiagnosticCode` outcomes into one `#[test]`, yet G.20 sc.1 tables several codes and sc.3 covers every code; G.6 sc.3 combines F3 and F7; G.26 sc.1 combines F1 and F2. G.16/G.18 expose the contradiction directly: `exactly_one` pass / fail-zero / fail-many is permitted as one `form`×`scope` scenario but yields **two different codes** (F1 and F8), which the next SC sentence forbids. SC must be disambiguated before units are drafted. |
| M-5 | P1→P2 | Rust (P1, mechanical); Correctness (P1); Architecture (P2, mechanical) | The **reachability matrix conflates runtime reachability with deterministic test inducibility**. F5 is genuinely runtime-reachable through `run_registry` (a nested `list_dir`/metadata failure, or a discovered symlink, under an *allowed* root) and F6 is too (the plan's own text maps delete-between-discovery-and-read to F6). "No — by design" and "No" are false as *reachability* claims; the prevalidation argument covers only F5's containment sub-case. Majority classification: **mechanical** — split the column into "runtime-reachable through `run_registry`" (F5=Yes, F6=Yes) and "deterministically inducible end-to-end" (F5=No, F6=No). |

## Advisory findings carried forward

| ID | Sev | Raised by | Summary |
|---|---|---|---|
| A-1 | P2 | Constitution (C-3) | G.8 sc.3 asserts an escaping empty root yields `OutOfWorkspace` **not** `EmptySet`, but `EmptySet` does not exist until G.12 and emptiness is not observable on the `validate_root` API. The literal assertion will not compile. |
| A-2 | P2 | Constitution (C-4); Scope (F-01, P3) | "35 units, each … touches ≤3 files" is false for G.1, which touches 10 (`Cargo.toml` + nine topology files). v2 recorded this as a justified C6 exception; v3 states the cap as universally met. Scope rates it acceptable (~10 minutes) but the claim must be corrected. |
| A-3 | P2 | Correctness; Architecture (D) | G.10 sc.3 (symlink-entry rejection) does not pin its `FileAccess` implementation. Through `RealFs` with a real on-disk symlink it is non-deterministic on Windows. Pin it to `FakeFs`. |
| A-4 | P2 | Maintainability (M3) | F5 conflates two ontologically distinct conditions — containment violation and discovery I/O failure. A CI reader cannot tell which occurred. Either split F5, or require a discriminator field in `detail`. |
| A-5 | P2 | Architecture (E) | The Failure-Semantics and Verification tables claim F5 is "proven by `resolve` unit only (G.8, G.12)", but neither the canonical-escape nor the root-symlink sub-case is deterministically proven anywhere. Reconcile after A/B. |
| A-6 | P2 | Maintainability (M4) | G.24 and G.26 both list `agent_harness_contracts_test.rs` under "Files owned". Created-by vs modified-by ownership is unstated and will not scale when Packages A–F touch the same file. |
| A-7 | P3 | Maintainability (M5) | G.30 adds a **new consumer** of the archived `serde_yaml` solely to assert the *absence* of two lines. H7's "inherited, not introduced" glosses over the new-consumer fact; a substring assertion would prove the same property. |
| A-8 | P3 | Maintainability (M6) | The report layer (G.20–G.23) has no causal dependency on resolve/read/assert output; serializing it after G.19 is a documentation choice presented as a constraint. |
| A-9 | P3 | Security (S-1) | G.9 should use component-wise `Path::starts_with`, not string-prefix comparison, to avoid spurious rejection of Windows extended-length (`\\?\`) canonical paths. Biases fail-closed, so not a security defect. |
| A-10 | P3 | Security (S-2) | The plan does not state that the offending path carried by F5/F6 is routed through `normalize_path` before rendering. Moot if a single `Diagnostic` funnels through one `render`, but unstated. |

## What a future attempt must settle first

1. **Resolve A and B together by provisioning the seam properly.** They are one
   problem. Decide what `FileAccess` must expose — at minimum path-level
   `symlink_metadata`, and canonicalization if deterministic containment proof is
   to be kept — and reorder `validate_root` so the root's own symlink status is
   checked **before** canonicalization or traversal. Alternatively, drop the
   symlink-root-rejection and deterministic-containment claims; but the H6
   guarantee then has to be rewritten honestly, and the root-symlink fail-open
   must still be closed some other way, because it is a live false-PASS vector.
2. **Do not reopen B1.** The filter-class resolution is affirmed 7/7 and the
   Security Reviewer explicitly declined to re-request bounds. Finding A is
   **adjacent to** B1, not a regression of it: it is a non-filter omission path.
3. **Disambiguate SC before drafting units** (M-4). Decide explicitly whether one
   `form`×`scope` scenario may assert more than one `DiagnosticCode` — the
   `exactly_one` case forces the question — then re-derive unit boundaries.
   Several units will need splitting, raising the count above 35.
4. **Rebuild the Constitution Check against the real I–XI** (M-3), marking
   non-applicable principles N/A with a reason rather than renaming them.
5. **Give the G.1 placeholder an owner** (M-1) and **move `classify` out of G.6**
   (M-2).
6. **Relabel the reachability matrix** as two columns (M-5).

## Explicit non-actions

No implementation code written. No backlog item created — **no IDs allocated**.
No shipment assembled. No PR created; PR #396 untouched. `143.*` left abandoned.
Packages A–F not modified or restaged. No stash entry archived. No `src/` change.
No dependency change. No change to `.github/workflows/ci.yml`,
`.cargo/test-coverage-manifest.toml`, `Cargo.toml`, or any file under `tests/` —
the plan proposes those edits; none were applied. The v2 plan-review record was
read as evidence and **not modified**.
