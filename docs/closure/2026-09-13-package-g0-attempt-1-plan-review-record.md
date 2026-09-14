---
type: plan-review-record
date: 2026-09-13
package: G0
attempt: 1
plan: docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan.md
source: docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
verdict: FAIL
rounds: 1
correction_round_opened: false
harvested: false
backlog_ids: none
shipment: none
pr: none
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
---

# Package G0 attempt 1 — plan-review record

**Verdict: FAIL — terminal at round 1.** Two **architecture-level P1** findings.
The correction budget is restricted to mechanical findings and therefore never
opened. No harvest. **No backlog IDs allocated. No shipment assembled. No PR.**

**Consequence for the program**: per the program decision's gate policy item 2,
**G1 and G2 were NOT reviewed and NOT harvested** — their prerequisite authority
is absent. Their draft plans are preserved unmodified as dependent drafts.

## Gate rule applied

> One bounded mechanical correction + confirmation per package. Any
> architecture-level P0/P1 ⇒ block that package. PASS requires zero P0/P1.
> P2-only ADVISORY is not auto-harvested.

Round 1 produced two architecture-level P1 findings and one mechanical P0. The
correction budget is explicitly restricted to **mechanical** findings, so it did
not open.

## Reviewer panel — cross-model diversity

Seven personas across seven distinct models, all run in round 1.

| Persona | Model | Verdict | Blocking findings |
|---|---|---|---|
| Rust Reviewer | `gpt-5.6-sol` | **FAIL** | 1 P0, 3 P1 (1 architecture-level) |
| Correctness Reviewer | `gpt-5.6-terra` | **FAIL** | 3 P1 (3 architecture-level as raised) |
| Architecture Strategist | `claude-opus-4.8` | **PASS** | none (3 P3) |
| Constitution Reviewer | `grok-4.6` | **PASS** | none (2 P2, 4 P3) |
| Security Reviewer | `claude-sonnet-5` | **PASS** | none (2 P2, 2 P3) |
| Scope Boundary Auditor | `gemini-3.8-flash` | **FAIL** | 1 P1 (mechanical) |
| Maintainability Reviewer | `claude-opus-4.7` | **PASS** | none (3 P2, 1 P3) |

Tally: **3 FAIL / 4 PASS.**

## What this review SETTLES — do not re-derive on the next G0 attempt

These are affirmed and must be inherited, not re-argued.

1. **The decomposition itself is sound.** The Architecture Strategist — the same
   persona that adjudicated v3's findings A and B as architecture-level — rated
   the split topology **PASS** on every probe: seam sufficiency, seam
   non-over-provisioning, topology T3 soundness, the G0/G1 split boundary,
   independent shippability, and layering direction. The three-way split is
   **not** the defect. Do not revisit the split.

2. **v3 finding A (root-symlink fail-open) is CLOSED** for allow-listed roots.
   Adjudicated CLOSED by Architecture; confirmed independently by Security
   (probe 2 PASS) and by Scope (probe 2, `FsCall` justified). The
   lstat-before-canonicalize ordering plus the `FsCall` call-log assertion is a
   genuine *ordering* proof, not a verdict-only proof. **Keep this mechanism
   exactly as designed.**

3. **v3 finding B (non-injectable canonicalization) is CLOSED.** Adjudicated
   CLOSED by Architecture; confirmed by Correctness (probe 2 PASS) and Security.
   The `FakeFs` canonicalization map induces the canonical-escape case with no
   on-disk symlink, no junction, and no Windows elevation. **Keep it.**

4. **The four-method seam is exactly right — neither under- nor
   over-provisioned.** Architecture probe 1 and probe 2 both PASS; Maintainability
   probe 5 PASS ("load-bearing rather than speculative … at four methods").
   Omitting size/mtime/permissions/following-`metadata` is correct. **Do not
   widen or narrow the method set.**

5. **No speculative limits — the bounds question stays closed.** The Security
   Reviewer explicitly declined to request bounds and stated it had no new
   evidence beyond the v3 panel's. Scope probe 4 PASS: zero hidden filters.
   **Do not reopen `max_depth` / `max_filesize` / walker filters.**

6. **The Constitution Check is accurate.** Constitution probe 1 PASS: all I–XI
   use real names and real numbering; none invented, renamed, misnumbered, or
   omitted; VII correctly marked N/A with a reason. The v3 M-3 defect (eight
   fictional principle names) is **not** reproduced. The reviewer verified the
   plan's claim that principle VI's body is about dependency minimality by
   quoting it.

7. **Zero new dependencies is achievable.** Rust probe 8 PASS: every required
   capability exists in `std`; `tempfile` is already a root dependency.

8. **No v3 maintainability defect is reproduced.** Maintainability's summary
   table confirms M-1 (ownerless placeholder), M-3 (name/count errors), M-4
   (self-contradictory scenario convention), M-5 (reachability/inducibility
   conflation), and A-6 (created-by vs modified-by ambiguity) are all **closed**.

9. **Scope discipline is intact.** Scope probes 1, 4, 5, 7, 8 all PASS: no
   speculative public item, no hidden policy limit, no unit padding, no
   cross-boundary leakage into G1/G2 territory, no documentation-as-constraint.

## Required explicit probes — round-1 consensus

| # | Probe (operator-mandated) | Consensus |
|---|---|---|
| 1 | Root lstat-before-canonicalize | **PASS for allow-listed roots** (Architecture, Security, Scope). Correctness raises a *workspace-root* variant — see contested finding C-1 below |
| 2 | Injectable canonicalize | **PASS** (Architecture, Correctness, Security, Maintainability) |
| 3 | Windows determinism | **PASS** (Architecture, Security) with one **mechanical** defect in the extended-length scenario (Rust G0-R6) |
| 4 | No silent filtering | **PASS for the filter class** (Scope 4/4, Security 7, Architecture). Correctness records a surviving **non-filter** duplication path — finding B below |
| 5 | No dead code | **PASS in substance, over-stated in wording** (Rust G0-R5) — mechanical |
| 6 | Test-only topology | **PASS** (Architecture 5, Maintainability 1, Constitution) |

## Terminal findings — ARCHITECTURE-LEVEL

### A — `FsError` cannot represent metadata or canonicalization failure — HIGH CONSENSUS, ARCHITECTURE-LEVEL

* **Raised independently by**: Rust Reviewer (`G0-R1`, P1, architecture-level)
  and Correctness Reviewer (`FS-2`, P1, architecture-level). Two reviewers on
  two different models converged on the identical defect without seeing each
  other's output.
* **Defect**: `RealFs::symlink_metadata` and `RealFs::canonicalize` can fail for
  reasons the nine-variant taxonomy cannot express — permission denial,
  transient I/O error, invalid path, or disappearance between operations. The
  plan names `RootMissing`, `RootNotDirectory`, `RootIsSymlink`, and
  `RootOutOfWorkspace` for the *semantic* outcomes of those two calls, but no
  variant for the calls *failing*. Mapping an `EACCES` on `symlink_metadata` to
  `RootMissing` would be a false statement; mapping a canonicalization failure to
  `RootOutOfWorkspace` would be worse — it would report a containment violation
  that did not occur. `EnumerationFailed` and `ReadFailed` cover only `read_dir`
  and `read_to_string`; two of the four seam methods have no failure channel at
  all.
* **Why it is terminal**: this is **the same defect class that terminated v3**,
  one level up. v3 died because the seam's *method set* was under-provisioned for
  what the resolver asked of it. G0 provisions the method set correctly and then
  under-provisions the *error contract* for what those methods can actually
  return. The plan's own Failure Semantics table claims to be complete and is
  not.
* **Why it is architecture-level, not mechanical**: `FsError` is public, is
  returned directly through the seam, and is the type G1 consumes across the
  package boundary. Adding operation-failure variants changes the public error
  contract, the Failure Semantics table, the two-column reachability matrix, the
  `RealFs` mapping, and requires new `[R]`/`[G]` pairs. It is a contract change,
  not a wording repair. Both raising reviewers classified it this way
  independently.

### B — Corpus identity is undefined; duplicate and nested roots produce a multiset — ARCHITECTURE-LEVEL

* **Raised by**: Correctness Reviewer (`FS-3`, P1, architecture-level).
  Uncontested — no other reviewer probed corpus identity.
* **Defect**: `resolve_corpus` returns a sorted `Vec<ResolvedFile>` with no
  duplicate-root, nested-root, or duplicate-`rel_path` policy. If the allow-list
  repeats a root, or contains both a root and a descendant of that root, the
  descendant's files are emitted **twice**, and `read_corpus` faithfully
  preserves the duplication. The plan calls its output a "corpus" and the
  objective calls it a set, but the type is a multiset.
* **Why it is terminal**: this is a **cross-package contract defect**, not a G0
  internal issue. G1's `exactly_one` assertion is settled by split-deliberation
  Decision 4 as a `match` on the number of matches — `n > 1 ⇒
  AmbiguousMultiplicity`. A duplicated corpus entry produces `n = 2` for a
  contract that genuinely matches once, so **G1 emits a false multiplicity
  failure caused entirely by G0**. The defect is invisible inside G0 and
  detonates in G1. The five roots are currently disjoint, so this is latent
  today — which is exactly why it must be closed before G2 binds real roots and
  before Packages B and C extend the registry.
* **Why it is architecture-level**: it determines the *meaning* of G0's public
  result type at the boundary G1 consumes. The repair is a public invariant —
  either reject overlapping canonical roots with a named error, or deduplicate by
  normalized `rel_path` — plus new error-taxonomy and test surface. Not a
  wording repair.

## Contested finding — adjudicated NOT blocking

### C-1 — Workspace root is canonicalized without a preceding lstat

* **Raised by**: Correctness Reviewer (`FS-1`, P1, architecture-level).
* **Also examined by**: Architecture Strategist (`G0-A2`, **P3**, mechanical) and
  Security Reviewer (`SEC-3`, **P3**, mechanical).
* **Adjudication: DOWNGRADED to P3 ADVISORY.** Two reviewers examined this
  specific question directly and independently reached the same conclusion: the
  workspace root is a **caller-supplied trust anchor**, not attacker-influenced
  corpus content, so canonicalizing it without an lstat is correct rather than a
  fail-open. The Architecture Strategist — the designated adjudicator of the
  mechanical/architecture-level classification — ruled explicitly on it.
  Security added that it is "not exploitable under the stated threat model".
* **The dissent is recorded, not dismissed.** Correctness's concern that
  `symlink_metadata(joined_root)` follows symlinked *intermediate* components is
  factually correct as a mechanism; the panel's majority position is that the
  workspace root is outside G0's trust boundary by construction. **The next
  attempt must document this explicitly** (both Architecture and Security asked
  for exactly one sentence in H6). Leaving it undocumented is what produced the
  disagreement.

## Mechanical findings — recorded, NOT corrected

The correction budget never opened. These are recorded so the next G0 attempt
inherits them rather than rediscovering them.

| ID | Sev | Raised by | Finding |
|---|---|---|---|
| **M-1** | **P0** | Rust (`G0-R2`); Scope (`F-02`, P2); Maintainability (`M-2`, P2) | **Red/green staging contradictions — three independent reviewers.** G0.12 is specified as implementing resolver "steps 4–6", which already includes the `Other` arm, UTF-8 validation, separator normalization, and sorting. Those behaviours are the subject of G0.13 and G0.15(a)(b), which therefore **cannot fail** when introduced. The test-first sequence is broken for four scenarios. Additionally G0.12 and G0.14 both claim ownership of the exhaustive `FileKind` match and UTF-8 validation — two units cannot own one obligation. Repair: scope G0.12 to steps 4–5 plus the File/Dir/Symlink arms only; move `Other`, UTF-8, separator normalization, and sorting to the units whose red tests drive them. |
| **M-2** | **P1** | Scope (`F-01`) | **`read_corpus`'s happy path is asserted nowhere.** G0.15c tests only `ReadFailed` on a disappearing file. An implementation returning unconditional `Err(ReadFailed)` would pass every test in the plan. Repair: add an explicit scenario asserting `read_corpus` returns `Ok` with matching paths and contents on a multi-file tree. |
| **M-3** | **P1** | Rust (`G0-R3`) | **Red-phase stubs are not warning-clean.** `fn f(path: &Path) -> R { todo!() }` leaves `path` unused, and `-Dwarnings` (`.cargo/config.toml`) makes `unused_variables` a hard error — so the `[R]` units would not compile, defeating the plan's own "every `[R]` unit leaves the workspace compiling" rule. Repair: mandate underscore-prefixed stub parameters (`_fs`, `_path`) and require `todo!()` to be the terminal expression; rename in the `[G]` unit. |
| **M-4** | **P1** | Rust (`G0-R4`) | **Root grammar is unvalidated before the first seam call.** Caller-supplied `&str` roots such as `../outside`, an absolute path, or a Windows prefixed path are lexically joined and then lstat'd — touching the filesystem outside the workspace *before* containment is checked. Repair: reject `Prefix`, `RootDir`, and `ParentDir` components purely lexically before any seam call, and add a call-log test proving a malformed root produces **zero** filesystem calls. |
| **M-5** | P2 | Rust (`G0-R6`) | **G0.9c does not prove what it claims.** `Path::starts_with` compares `Component::Prefix` values, and `Prefix::Disk` ≠ `Prefix::VerbatimDisk` — so a mixed `\\?\C:\repo\child` vs `C:\repo` comparison is **false**, contradicting the scenario's expected acceptance. If both sides are `\\?\`-prefixed the scenario is valid but proves only consistent-form handling, which string-prefix comparison would also satisfy. Repair: map both canonical values to extended-length form and relabel; move the component-wise-vs-string-prefix proof to a sibling-name case (`repo-evil` vs `repo`). |
| **M-6** | P2 | Security (`SEC-2`) | **The motivating adversarial fixture is never instantiated.** Deliberation Decision 5 names `.github/agents-evil` vs `.github/agents` as the case justifying component-wise comparison, but no G0.9 scenario uses it. Repair: make G0.9a that exact fixture. Converges with M-5. |
| **M-7** | P2 | Security (`SEC-1`) | **TOCTOU substitution is not covered by `ReadFailed`.** `read_to_string` follows symlinks, so a resolved file **replaced by a symlink** between `resolve_corpus` and `read_corpus` is read successfully with substituted content — the exact "substituted tree" failure G0 exists to prevent, relocated to the read boundary. H6 claims the race "is surfaced as `ReadFailed`", which is proven only for disappearance. Repair: re-invoke `symlink_metadata` immediately before each read and reject a kind change; document that this narrows but cannot eliminate the syscall-level race. |
| **M-8** | P2 | Maintainability (`M-1`) | **`FileKind::Other` has no production mechanism in `FakeFs`.** The builder exposes `dir`, `file`, `symlink`, `unreadable`, `canonical`, `remove` — none yields an `Other` node, so G0.13b is not implementable as written. The deliberation's four node kinds (File/Dir/Symlink/Unreadable) do not map 1:1 onto `FileKind` (File/Dir/Symlink/Other). Repair: add an `other()` builder or publish the explicit node→kind mapping. |
| **M-9** | P2 | Maintainability (`M-3`) | **`#![warn(clippy::pedantic)]` in the crate is decorative.** `cargo dev-test` runs no clippy; root `cargo clippy --all-targets` does not lint a dependency crate's own lints; `-Dwarnings` is a rustc concern, not a clippy one. So no G0 gate enforces pedantic on the crate, yet Constitution Check row I implies it does. Repair: either add `cargo clippy -p agent-contract-fs --all-targets -- -D warnings -D clippy::pedantic` to Verification, or label the attribute aspirational and record that enforcement is owned by G2. |
| **M-10** | P2 | Constitution (`C-2`) | **`cargo audit` is missing from Verification** while the Constitution Check row claims all four quality gates are present in the constitution's order. Repair: insert `cargo audit` after `cargo dev-test`. |
| **M-11** | P2 | Constitution (`C-1`); Correctness (`FS-6`) | **Count error**: Constitution Check row II says "7 pairs"; Work Units says 16 units / 8 pairs. Same class as v3 M-3's arithmetic error. Repair: change to 8. |
| **M-12** | P2 | Correctness (`FS-6`) | **Cross-reference errors in hardening**: H6 cites `ReadFailed (G0.13b)` but the disappearing-file test is G0.15c; H5/H7 say "three manifest additions" but G0.1 adds four entries; H7 says "no dependency-graph change" while R2 correctly records one added local path node. Repair: correct all four. |
| **M-13** | P2 | Rust (`G0-R5`) | **The dead-code claim is over-stated.** `dead_code` is suppressed only for *effectively externally reachable* items. Unused **private struct fields**, unused private helpers, and `pub` items inside a **private module** still fire under `-Dwarnings`. The conclusion (no `allow(dead_code)` needed) survives, but H4's reasoning does not as written. Repair: narrow H4 to effective external visibility and require private state to be introduced only in the unit that uses it. |
| **M-14** | P2 | Correctness (`FS-5`) | **G0.5b packs two outcomes into one scenario** (a successful `File` classification *and* `RootMissing`), violating the plan's own one-outcome-per-test convention; splitting it would make four scenarios, violating the ≤3 cap. Repair: drop the regular-file assertion from G0.5b — G0.5a already proves file classification. |
| **M-15** | P2 | Correctness (`FS-8`) | **H3's grep proof is narrower than its claim.** The Verification grep checks `max_depth`, `max_filesize`, and `WalkBuilder` only; it does not mechanically exclude hidden-file, extension, or ignore-file filtering. Repair: narrow the claim to the three checked constructs, or extend the check. |
| **M-16** | P2 | Correctness (`FS-4`) | **Undefined input/traversal invariants**: empty allow-list; a root canonically equal to the workspace root; `.` as a root; directory cycles / reparse points. Repair: state the accepted root grammar and the cycle policy explicitly. Converges with M-4. |

## Advisory findings carried forward

| ID | Sev | Raised by | Summary |
|---|---|---|---|
| A-1 | P3 | Architecture (`G0-A1`) | `read_corpus` returns `Vec<(String, String)>` — a positional tuple — while resolve returns the named `ResolvedFile`. At the G0→G1 boundary this invites transposition. Prefer a named `ReadFile { rel_path, content }`. |
| A-2 | P3 | Architecture (`G0-A2`); Security (`SEC-3`) | H6 omits the trust-anchor assumption: the workspace root is canonicalized but not symlink-checked. One sentence closes the C-1 dispute permanently. |
| A-3 | P3 | Architecture (`G0-A3`) | H2 should state explicitly that `RealFs` symlink-kind detection rests on the documented `std::fs` lstat contract and is not privilege-testable, backstopped by the second containment layer. |
| A-4 | P3 | Security (`SEC-4`) | Path rendering in `FsError` is unspecified; `RootOutOfWorkspace` may carry a runner-specific absolute path into CI logs. Carried forward from v3 A-10, still open. State the rendering policy. |
| A-5 | P3 | Scope (`F-03`, rated P2); Maintainability (`M-1`) | `EntryNotReadable` / `FileKind::Other` may be speculative: Git cannot track FIFOs, sockets, or device nodes, so `Other` is unreachable against `RealFs` in this corpus. **Note the tension with M-1/M-8**: Correctness and Rust both require an exhaustive match with no wildcard arm. Resolve deliberately — either keep the variant with an explicit `FakeFs` producer, or drop `FileKind::Other` entirely and let the match be exhaustive over three kinds. |
| A-6 | P3 | Constitution (`C-4`) | "tests live in `tests/contract/`, the mandated contract tier" over-reads principle II, whose contract tier is defined as MCP tool response verification. The placement is justified by topology T3, not by II. |
| A-7 | P3 | Constitution (`C-5`) | Width Isolation row calls G0.1 "registration-only and contains no logic"; G0.1 also adds the full type surface and the first real test. Reword. |
| A-8 | P3 | Constitution (`C-3`) | Principle IV row's read claim holds for `resolve_corpus` but not for public `RealFs`, which is an unconstrained `std::fs` adapter. Split the claim. |
| A-9 | P3 | Constitution (`C-6`) | Principle VIII freeze-scope says "the four files listed in G0.1 plus `lib.rs`"; G0.1 lists five files already including `lib.rs`. |
| A-10 | P3 | Maintainability (probe 4) | `resolve_corpus` is likely to trip `clippy::pedantic`'s `too_many_lines` once G2 closes the workspace lint gap. Record in H6 that the single-file layout is not guaranteed to survive G2. |
| A-11 | P3 | Maintainability (probe 6) | The plan does not restate split-Decision 2's **freeze invariant** — G1 adds a new crate and must not edit G0's. An implementing agent may be tempted to "just add one method" to G0 during G1. Restate it in H5/H6. |
| A-12 | P3 | Rust (probe 3) | `FakeFs::calls()` should return a cloned snapshot rather than a `Ref`, so a caller cannot hold an immutable borrow while another seam call records an event. |
| A-13 | P3 | Correctness (`FS-7`) | H1's explanation ("a canonicalize-first refactor will still return `RootIsSymlink`") is inaccurate — canonicalizing first may yield a non-symlink target reported as `Dir`. The call-log assertion still catches the reordering; only the prose is wrong. |

## What the next G0 attempt must settle first

1. **Provision the error contract (finding A).** Decide what `FsError` must
   express for a *failing* `symlink_metadata` and a *failing* `canonicalize` —
   either operation-specific variants or a single
   `FilesystemOperationFailed { operation, path, source_kind }` — and give each a
   red/green pair. Then reconcile the Failure Semantics table, the reachability
   matrix, and the `RealFs` mapping. This is the direct analogue of v3's
   "provision the seam properly", one level up.
2. **Define corpus identity (finding B).** Choose explicitly: reject overlapping
   or repeated canonical roots with a named error, **or** deduplicate by
   normalized `rel_path`. State the invariant in the objective, prove it with a
   repeated-root test and a nested-root test, and say in the G0→G1 contract that
   `rel_path` is unique. G1's `exactly_one` correctness depends on this.
3. **Re-cut the red/green staging (M-1).** Three reviewers independently found
   the G0.12 over-scope. Re-derive unit boundaries so no behaviour is implemented
   before the test that drives it, and so no obligation is claimed by two units.
   The unit count will rise above 16.
4. **Make the red phase actually compile (M-3).** Mandate `_`-prefixed stub
   parameters. This is a precondition of the plan's own red-phase mechanism.
5. **Assert the `read_corpus` happy path (M-2).**
6. **Validate root grammar lexically before any seam call (M-4, M-16).**
7. **Fix the Windows scenario (M-5, M-6)** and use the sibling-name fixture the
   deliberation already names.
8. **Resolve the `FileKind::Other` tension deliberately (A-5 vs M-8).** Keep it
   with a producer, or drop it and make the match exhaustive over three kinds.
   Do not leave it half-specified.
9. **Do not reopen**: the split, the four-method seam, the lstat-before-
   canonicalize mechanism, the `FsCall` call log, the `FakeFs` canonicalization
   map, the no-bounds decision, or the Constitution Check mapping. All are
   affirmed above.

## Explicit non-actions

No implementation code written. No source, test, or configuration file modified
— the plan proposes edits to `Cargo.toml`, `.cargo/test-coverage-manifest.toml`,
`crates/`, and `tests/`; **none were applied**. No backlog item created — **no
IDs allocated**. No shipment assembled. No PR created; PR #396 untouched.
`143.*` left abandoned. Packages A–F not modified or restaged. No stash entry
archived. No dependency change. No change to `.github/workflows/ci.yml`. The v2
and v3 plan-review records were read as evidence and **not modified**. G1 and G2
were **not reviewed** and their draft plans were **not modified**.
