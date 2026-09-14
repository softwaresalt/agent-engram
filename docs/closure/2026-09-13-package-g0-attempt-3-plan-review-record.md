---
type: plan-review-record
date: 2026-09-13
package: G0
attempt: 3
plan: docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan-attempt-3.md
source: docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-3.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
prior_reviews:
  - docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md
  - docs/closure/2026-09-13-package-g0-attempt-2-plan-review-record.md
verdict: FAIL
rounds: 1
correction_round_opened: false
circuit: OPEN
harvested: false
backlog_ids: none
shipment: none
pr: none
branch: chore/checkpoint-resolution-ordering-restage
head: c5a6d4d6
---

# Package G0 attempt 3 — plan-review record

**Verdict: FAIL — terminal at round 1. The G0 circuit is now OPEN.**

Attempt 3 was the **third and final allowed** G0 attempt. Round 1 produced **one
architecture-level P1 cluster with two-model consensus**, plus one
high-consensus P1 cluster that the designated adjudicator ruled mechanical, plus
two single-reviewer mechanical P1s. The correction budget is restricted to
mechanical findings and therefore **did not open**, because an architecture-level
P1 is present.

**No harvest. No backlog IDs allocated. No shipment assembled. No PR.**
**No fourth G0 plan will be authored.**

## Gate rule applied

> One bounded mechanical correction + confirmation per package. Any
> architecture-level P0/P1 ⇒ block that package. PASS requires zero P0/P1.
> P2-only ADVISORY is not auto-harvested. Architecture P0/P1 ⇒ FAIL and open
> the circuit.

## Reviewer panel — cross-model diversity

Seven personas across seven distinct models, all run in round 1, all reviewing
plan revision `c5a6d4d6`. Same panel composition as attempts 1 and 2, so the
verdicts are directly comparable.

| Persona | Model | Verdict | Blocking findings |
|---|---|---|---|
| Rust Reviewer | `gpt-5.6-sol` | **FAIL** | 4 P1 (3 classified architecture-level, 1 mechanical); 3 P2 |
| Correctness Reviewer | `gpt-5.6-terra` | **FAIL** | 3 P1 (3 classified architecture-level); 3 P2 |
| Architecture Strategist | `claude-opus-4.8` | **FAIL** | 1 P1, adjudicated **MECHANICAL**; 1 P3 |
| Constitution Reviewer | `grok-4.6` | **FAIL** | 1 P1 (mechanical); 3 P2, 1 P3 |
| Security Reviewer | `claude-sonnet-5` | **PASS** | none (1 P2, 2 P3) |
| Scope Boundary Auditor | `gemini-3.8-flash` | **PASS** | none (3 P2) |
| Maintainability Reviewer | `claude-opus-4.7` | **PASS** | none (3 P3) |

Tally: **4 FAIL / 3 PASS**. Attempt 1 and attempt 2 both split 3 FAIL / 4 PASS.
The pattern across all three attempts is stable: the security, scope and
maintainability lenses pass, and the plan dies on a claim its mechanism does not
deliver.

## Explicit probe outcomes across the panel

The seven reviewers each answered the same seven mandated probes.

| Probe | PASS | FAIL | Dissenting reviewers |
|---|---|---|---|
| 1 — unforgeable corpus / read API | 7 | 0 | — |
| 2 — TOCTOU revalidation | 5 | 2 | Rust, Correctness |
| 3 — independent fault keys incl. `ReadToString` | 7 | 0 | — |
| 4 — entry lstat operation | 7 | 0 | — |
| 5 — exhaustive enum / match compilation | 4 | 3 | Rust (pass on types, fail on staging), Architecture, Constitution |
| 6 — no premature green | 5 | 2 | Rust, Correctness |
| 7 — prior blockers stay closed | 5 | 2 | Rust, Correctness |

## What this review SETTLES — inherited if G0 is ever restaged

These are **affirmed** and must not be re-argued. They are in addition to the
items settled by the attempt-1 and attempt-2 reviews, all of which remain
settled.

1. **The opaque containment design is correct and airtight (probe 1, 7/7 PASS).**
   `ResolvedFile { rel_path: String }` with no stored path and only a
   `rel_path()` accessor, `ResolvedCorpus { workspace, files }` private with
   `resolve_corpus` as sole constructor, and `read_corpus` accepting only
   `&ResolvedCorpus`, is unforgeable in Rust's privacy model. Security
   specifically searched for any other route to a corpus value — `Default`,
   `Clone`, `Deserialize`, `From`, a public returning fn, a pub field, a builder
   — and found none. Architecture ruled the opaque-authority vs
   transparent-output-DTO split (`ReadFile` keeping public fields) a **coherent
   capability rule**, not an inconsistency. **Keep this design.**
2. **Folding the canonical workspace root into the corpus (Q2/W-2) is right.**
   Architecture: it makes "corpus checked against the wrong workspace root"
   *unrepresentable* rather than merely checkable, which is strictly stronger
   than a separate validated-token parameter. **Do not reintroduce a token.**
3. **The `compile_fail` doctest is a valid proof and its red/green exemption
   holds.** Rust, Security, Constitution and Scope all accepted it. Constitution
   noted precisely what it proves — that struct-literal forgery is rejected —
   and that `resolve_corpus` remains the intended public constructor.
   Maintainability verified the doctest is stable across all 36 commits.
4. **Operation-keyed fault injection is complete and correct (probe 3, 7/7
   PASS).** `(FsOp, PathBuf)` keying, with post-resolve mutation disambiguating
   the two call-site pairs that share a `(site, operation)` discriminant, gives
   all **nine** fallible call sites an independent injector. **`ReadToString` is
   genuinely proven by G0.31(c)** — Rust, Correctness and Maintainability each
   separately confirmed the fixture reaches the read rather than failing at the
   re-lstat, which is the exact trap that killed attempt 2's equivalent test.
   **Attempt-2 finding B is CLOSED.**
5. **Explicit entry lstat is correct (probe 4, 7/7 PASS).**
   `read_dir -> Vec<OsString>` (names only, no kind) plus a resolver that
   explicitly lstats every entry gives call site 5 a real driving operation.
   Architecture ruled it the correct minimal seam contract. Security confirmed
   it eliminates the trusted-enumeration-metadata attack surface.
   **Attempt-2 finding C is CLOSED.**
6. **The `FsError` taxonomy is compilation-complete.** Rust verified every
   derive chain: `FsError: Clone + PartialEq + Eq` holds given
   `std::io::ErrorKind`; `FsOp: Ord` is valid and required for the `BTreeMap`
   key; fieldless-enum `Ord` is sound; `FsFault`'s derives hold; `RefCell` in
   `FakeFs` imposes no undeclared `Sync` requirement. All 13 variants carry an
   `#[error(...)]`. **Attempt-2 finding E is CLOSED.**
7. **The channel-bundling rule is a genuine structural fix.** Architecture and
   Maintainability — the latter being the reviewer who raised attempt-2 finding
   D — both verified it row by row against the nine-call-site table. The two
   secondary staging breaks are closed: the identity pair bundles containment +
   `rel_path` + identity map (D part 2), and Phase 3's deliberate no-sort
   restores a reliably red determinism test (D part 3). **Attempt-2 finding D
   parts 1–3 are CLOSED at the level they were raised.**
8. **`todo!()`-arm staged completion is legitimate, not an anti-pattern.**
   Architecture ruled it *required* by B4 — the only way to keep matches
   exhaustive under `-Dwarnings` while staging. Rust and Maintainability both
   confirmed no `todo!()` arm can make a test pass that should fail.
9. **The security core holds (Security: PASS, no P0/P1).** All three attempt-2
   security findings — SEC-1 (TOCTOU residual), SEC-2 (no containment check),
   SEC-3 (absolute path disclosure) — are closed by mechanism. The path
   rendering policy is implementable, dropping `workspace` from `OutOfWorkspace`
   loses nothing a defender needs, `RootIsWorkspaceRoot` closes a real gap
   without opening one, and no code path implicitly follows a symlink.
10. **Scope discipline is intact (Scope: PASS).** All 16 "what changed" rows
    trace to a named prior finding; none is opportunistic. The 18-pair topology
    is driven by the channel-bundling rule and the 2-hour rule, not padding. No
    G1/G2 leakage. The H10 deferral of the doctest's CI execution to G2 is
    appropriate delegation under the program DAG, not a silent gap.
11. **Mechanical closures confirmed.** M-1 (call-log assertion), M-2 (lockfile
    proof method), M-3 (path rendering lift function), M-4 (H9 restated around
    the lift function and the trait-impl break), M-6, M-7(i) and M-7(ii), M-8,
    and advisories A-1, A-2, A-3 are all closed. Constitution confirmed the
    "What changed from attempt 2" table contains **no false closure**.

## Terminal finding — ARCHITECTURE-LEVEL

### A — Read-time revalidation checks workspace containment, not the authorized file identity — TWO-MODEL CONSENSUS

* **Raised independently by**: Rust Reviewer (`F-2`, P1, architecture-level) and
  Correctness Reviewer (P1, architecture-level). Two reviewers on two different
  models converged on the identical defect without seeing each other's output.
* **Defect**: `read_corpus` reconstructs
  `candidate = corpus.workspace.join(rel_path components)`, re-lstats it,
  canonicalizes it, and then checks only
  `canonical.starts_with(corpus.workspace)`. It **never compares the
  re-canonicalized result against the identity the corpus authorized.** If an
  ancestor directory is replaced by a symlink pointing to a **different location
  still inside the workspace**, the re-lstat reports the leaf as `File`,
  canonicalize succeeds, containment succeeds, and `read_to_string` reads a
  **different file** than the one `resolve_corpus` validated.
* **Why it is terminal**: the substituted target is contained in the workspace
  but may lie entirely **outside the configured allow-listed roots**. The
  allow-list is the boundary G1 depends on, so this is an allow-list escape, not
  merely an identity drift. Plan H6 states that ancestor-directory substitution
  is **closed** — it is closed only for redirects that leave the workspace.
  This is the **fourth consecutive instance of the same defect class that
  terminated package-G v3, G0 attempt 1 and G0 attempt 2: a claimed guarantee
  the mechanism does not deliver.** H6 is the plan's own disclosure section, and
  it over-claims.
* **Why it is architecture-level, not mechanical**: the correct repair requires
  `read_corpus` to assert that the re-canonicalized path equals the
  reconstructed authorized path, and to report drift through a **new, correctly
  named public error variant** (`FileIdentityChanged` or equivalent) — reporting
  an in-workspace identity change as `OutOfWorkspace` would be a second
  inaccurate claim. That changes the public `FsError` taxonomy from 13 to 14
  variants and adds driving scenarios, which changes the public type surface by
  the in-force definition. The documentation-only alternative — narrowing H6 and
  Constitution rows III and IV to disclose an allow-list escape as an accepted
  residual — would weaken the exact guarantee the package exists to provide, and
  was rejected for the same reason the attempt-2 review rejected the
  documentation-only escape for its finding A.
* **Dissent recorded**: Security and Architecture both rated probe 2 PASS.
  Neither constructed the redirect-to-a-contained-location variant; Security
  explicitly searched for unnamed open classes and did not find this one. The
  dissent does not overturn the finding — two reviewers demonstrated the
  concrete trace — but it is recorded because it shows the defect is subtle
  enough to survive a dedicated security lens.

## High-consensus finding — adjudicated MECHANICAL

### B — `ResolvedCorpus::workspace` is staged after its first reader — FOUR-MODEL CONSENSUS

* **Raised independently by**: Rust Reviewer (`F-1`, P1, classified
  architecture-level), Correctness Reviewer (P1, classified architecture-level),
  Architecture Strategist (`F1`, P1, adjudicated **MECHANICAL**), and
  Constitution Reviewer (`C-1`, P1, classified **MECHANICAL**). **Four reviewers
  on four different models.** This is the highest-consensus finding in any of
  the three G0 reviews.
* **Defect**: the compilation-completeness rule part 2 and H4 both state that
  `ResolvedCorpus::workspace` is introduced in **G0.36**, naming the containment
  recheck as its first reader. But **G0.32** already implements
  `candidate = corpus.workspace.join(rel_path components)`, so the true first
  reader is the reconstruction step, one pair earlier. Following the plan
  literally, an implementer either adds the field at G0.36 and G0.32 fails to
  compile, or writes the field from G0.14 and trips
  `dead_code: field 'workspace' is never read` under `-Dwarnings` for nine
  consecutive commits.
* **Why it matters**: it falsifies the plan's central B4 claim — and its
  Constitution row I claim — that the workspace builds warning-clean at **every**
  commit. Plan H11 lists this exact condition as one that "would make this plan
  wrong", and the plan then instantiates it.
* **Adjudication**: the Architecture Strategist holds the adjudicator role for
  ARCHITECTURE-LEVEL vs MECHANICAL classification, inherited from the attempt-2
  review. It ruled **MECHANICAL**, and the Constitution Reviewer independently
  agreed. The repair changes no public type (`ResolvedCorpus`'s final shape is
  unchanged), no pair boundary (still 18 pairs), and no cross-package contract:
  correct "G0.36" to "G0.32" in the rule and in H4, and state that the field is
  materialized at G0.32 with the G0.14 and G0.30 construction sites retrofitted
  in that same green commit. Rust and Correctness classified it
  architecture-level on the belief that the topology must be re-cut; the
  adjudicated repair is narrower and sufficient.
* **Consequence**: as a mechanical P1 it would have been eligible for the single
  correction round. The round never opened, because finding A is
  architecture-level.

## Mechanical findings — recorded, NOT corrected

The correction budget never opened. These are recorded so that any future
authority inherits them rather than rediscovering them.

| ID | Sev | Raised by | Finding |
|---|---|---|---|
| **M-1** | **P1** | Rust (`F-3`) | **`FakeFs` has no declared constructor.** The public surface lists eight mutators and three accessors but no `new()` and no `Default`. `FakeFs`'s fields are private and the behavioural tests live in an **external** root-package test target, so no fixture value can be constructed and G0.1 cannot compile as a usable harness. Repair: declare `impl Default for FakeFs` in G0.1 (and `new()` only alongside `Default`, to avoid `clippy::new_without_default`). Mechanical: a one-row addition to the type-surface table. |
| **M-2** | **P1** | Rust (`F-4`) | **`todo!()` staging is not pedantic-clean as documented.** `_`-prefixed parameters and terminal placement address `unused_variables` and unreachable-code, but not **`clippy::missing_panics_doc`**, which fires on public functions that can panic. Every staged `resolve_corpus` / `read_corpus` / seam-impl body containing `todo!()` needs a temporary `# Panics` section, removed when the last panicking path goes. Without it the explicit `-p` clippy command cannot pass at every staged commit, which is a second falsification of the every-commit claim. |
| **M-3** | P2 | Rust (`F-5`); Correctness | **`Cargo.lock` is omitted from the declared change scope — two independent reviewers.** The Dependency posture correctly states that adding a workspace member plus a path dev-dependency adds a local `[[package]]` record, but the "five files touched" count, the H5 freeze scope and the H7 rollback list all omit `Cargo.lock`. Repair: six files, and add it to the freeze scope and rollback. |
| **M-4** | P2 | Constitution (`C-2`) | **Constitution rows V and VII are swapped.** Row V is labelled Structured Observability and row VII Destructive Command Approval; the constitution and the settled attempt-2 mapping have them the other way round. Names are real; numbers are not. Repair: restore the attempt-2 numbering. |
| **M-5** | P2 | Constitution (`C-3`) | **The observability row over-claims.** "Every failure is a typed `FsError` with a site, an operation and a rendered path" is true only for `Io`. Attempt 2's wording was exact and should be restored: every failure is typed and carries a path; **seam** failures additionally carry site, operation and `ErrorKind`. |
| **M-6** | P2 | Constitution (`C-4`) | **The 2-hour claim for Pair 1 is unqualified.** Pair 1 creates five files, the entire type surface, all `FakeFs` mutators and accessors, and three tests — over the fewer-than-3-files / fewer-than-5-functions heuristics. Attempt 2 declared G0.1 as a justified bootstrap exception; attempt 3 dropped the declaration and asserted blanket compliance. Repair: restore the declared exception. Do **not** split Pair 1 — the bootstrap must compile as a unit. |
| **M-7** | P2 | Correctness | **`RootNotRelative` is a third irreducibly absolute payload class.** The rendering policy names only `Io { site: WorkspaceRoot }` and `OutOfWorkspace` as unrelativizable, but `RootNotRelative { root, .. }` necessarily carries the caller-supplied root spelling before any canonical workspace exists, and for an absolute input that is an absolute display path that never passes through `render`. Repair: name it as the third class, or change its payload policy. |
| **M-8** | P2 | Correctness | **Phase 3's pair iteration order and error-selection rule are unspecified.** Full pairwise comparison without a sort leaves the reported conflict implementation-dependent when several duplicates or overlaps exist. Repair: require ascending configured-index traversal (`i` ascending, `j > i` ascending), equality checked before overlap within a pair, first conflict wins. |
| **M-9** | P2 | Scope (`SBA-03`) | **The secondary unforgeability grep is unsatisfiable.** `'pub fn .*-> *(ResolvedCorpus\|ResolvedFile)'` cannot match `resolve_corpus`, whose return type is `Result<ResolvedCorpus, FsError>`, so the check yields zero matches against an expectation of exactly one. Repair: allow an optional `Result<` in the pattern. |
| **M-10** | P2 | Scope (`SBA-01`, `SBA-02`) | **`FakeNode` is public with public fields although no public function accepts or returns it, and `node_kind()` / `injected_fault()` exist principally to give `FakeFs`'s private fields an early public reader.** The plan asserts they are "not padding"; Scope tested that claim and found their only exercisers are self-referential fixture checks. Repair: make `FakeNode` crate-private and reconsider whether the two accessors survive once the seam methods read the fields. |
| **M-11** | P2 | Rust (`F-6`) | **Public structs have no declared `Debug`.** Derives are specified for the enums, `FsCall`, `FsFault` and `FsError`, but not for `RealFs`, `FakeNode`, `FakeFs`, `ResolvedFile`, `ResolvedCorpus` or `ReadFile`, so the "complete public surface in G0.1" claim is incomplete. Repair: add `Debug` in G0.1, using a redacting manual impl for `ResolvedCorpus` if exposing the absolute workspace root in debug output is undesirable. |
| **M-12** | P2 | Rust (`F-7`) | **Cross-platform `OsString` ordering is overstated.** H2 says byte-wise sorting "is identical on both platforms". `OsString: Ord` is deterministic per platform but its representation differs. Repair: claim per-platform determinism for entry names, and byte-wise semantics only for the UTF-8 `rel_path` sort. |

## Advisory findings carried forward

| ID | Sev | Raised by | Summary |
|---|---|---|---|
| A-1 | P3 | Maintainability (`F1`) | **The local-variable analogue of the private-state rule is undocumented.** A seam call whose *success value* is first read in a later unit cannot be written as `let x = …?;` in the earlier unit — `x` would be bound and unread, tripping `unused_variables` under `-Dwarnings`. Two concrete instances: `canonical_ws` (introduced G0.14, first read G0.16) and `canonical_root` (introduced G0.18, first read G0.20). Repair: one paragraph stating that such a call uses the `?;` statement form until the unit that first reads the value introduces the binding. |
| A-2 | P3 | Maintainability (`F3`) | **`clippy::too_many_lines` is addressed for `src/lib.rs` but not for the test file.** Roughly 50 test functions plus shared fixture builders live in one file under the pedantic surface; a fixture helper that grows across pairs could trip the lint. Repair: extend the named mitigation to test helpers — extract sub-helpers with callers in the same commit, never `#[allow]`. |
| A-3 | P3 | Maintainability (`F2`) | `FsOp` and `FsCall` encode the same four operations twice, so a fifth seam method requires a two-place update. Offset by the ergonomic `FsCall::ReadDir(dir)` match in call-log assertions. No repair required; recorded as an observation. |
| A-4 | P3 | Architecture | H5's G0→G1 contract enumerates only `resolve_corpus`, `read_corpus`, `ResolvedCorpus` and `ReadFile`, but G1 must also use `FileAccess`, `FakeFs` and `RealFs` to construct an unforgeable corpus. Add them to the stated contract. |
| A-5 | P3 | Constitution (`C-5`) | Row I dropped attempt 2's settled `FsError` vs `EngramError` deviation note. Restore it. |
| A-6 | P3 | Security | Two minor hardening-prose items recorded in the security output; neither affects the type surface or the unit topology. |

## Finding NOT upheld

The Correctness Reviewer raised a third architecture-level P1 asserting that
Pair 15 is internally inconsistent — that G0.29(c) needs an ordinary multi-root
`Ok` result which G0.30 does not yet produce. **This is not upheld.** G0.29(c) is
red before G0.30 (the resolver panics at the `todo!()` terminating Phase 4) and
green after it, which is exactly the intended red/green shape. The
Maintainability Reviewer — the persona who raised attempt-2 finding D and holds
the staging authority — walked Pair 15 explicitly and confirmed both fixture
variants panic before G0.30 and produce identical output after it. The finding
rests on a misreading of which unit owns the corpus return.

## Circuit status

**The G0 circuit is OPEN.**

Three consecutive G0 plans have now failed plan review at round 1, each on a
different architecture-level defect, and each of the three shares one root cause:
**the plan asserted a guarantee its mechanism did not deliver.** Attempt 1's
Failure Semantics table claimed completeness it did not have. Attempt 2's
containment claim was forgeable at the type level. Attempt 3's H6 claims
ancestor-directory substitution is closed when it is closed only for redirects
that leave the workspace.

Per the operating gate, **no fourth G0 plan is authored in this session.**
Reopening the circuit requires an explicit operator decision.

**Consequence for the program**: per the program decision's gate policy item 2,
**G1 and G2 remain unreviewed and unharvested** — their prerequisite authority is
still absent. Their draft plans are preserved unmodified as dependent drafts.

## What a future G0 authority would need to settle first

Recorded for completeness, not as an invitation to proceed.

1. **Decide the read-time identity contract (finding A).** Either compare the
   re-canonicalized path against the reconstructed authorized path and add a
   correctly named variant for drift, or re-scope containment to the canonical
   **roots** rather than the workspace, or explicitly accept and disclose an
   allow-list escape in H6 and Constitution rows III and IV. Choose one, justify
   it, and make every claim say exactly what the mechanism delivers.
2. **Fix the `workspace` field staging (finding B)** — the adjudicated repair is
   already written out above and is mechanical.
3. **Fix M-1 and M-2** — the missing `FakeFs` constructor and the
   `missing_panics_doc` obligation. Both falsify the every-commit
   warning-clean claim as written.
4. **Fix the ten remaining P2s and six advisories** listed above.
5. **Do not reopen** anything in the "What this review SETTLES" section above, or
   in the attempt-1 and attempt-2 equivalents. In particular the opaque
   containment design, the corpus-carries-its-workspace decision, the
   `compile_fail` doctest, operation-keyed fault injection, the explicit entry
   lstat, the `FsError` taxonomy, the channel-bundling rule and `todo!()`-arm
   staging are all affirmed.

## Explicit non-actions

No implementation code written. No source, test, or configuration file modified
— the plan proposes edits to `Cargo.toml`, `.cargo/test-coverage-manifest.toml`,
`crates/` and `tests/`; **none were applied**. No backlog item created —
**no IDs allocated**. No shipment assembled. No PR created; PR #396 untouched.
`143.*` left untouched. Packages A–F not modified or restaged. The G split, G1
and G2 were not reopened; G1 and G2 were **not reviewed** and their draft plans
were **not modified**. No stash entry archived. No dependency change. No change
to `.github/workflows/ci.yml`. The attempt-1 and attempt-2 plans, deliberations
and review records were read as evidence and **not modified**. The v2 and v3
package-G records were not touched.
