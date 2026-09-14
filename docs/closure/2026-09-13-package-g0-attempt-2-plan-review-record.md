---
type: plan-review-record
date: 2026-09-13
package: G0
attempt: 2
plan: docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan-attempt-2.md
source: docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-2.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
prior_review: docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md
verdict: FAIL
rounds: 1
correction_round_opened: false
harvested: false
backlog_ids: none
shipment: none
pr: none
branch: chore/checkpoint-resolution-ordering-restage
head: 38b3120c
---

# Package G0 attempt 2 — plan-review record

**Verdict: FAIL — terminal at round 1.** Four **architecture-level P1** finding
clusters, three of them raised independently by two or more reviewers on
different models. The correction budget is restricted to mechanical findings and
therefore never opened. No harvest. **No backlog IDs allocated. No shipment
assembled. No PR.**

**Consequence for the program**: per the program decision's gate policy item 2,
**G1 and G2 remain unreviewed and unharvested** — their prerequisite authority is
still absent. Their draft plans are preserved unmodified as dependent drafts.

## Gate rule applied

> One bounded mechanical correction + confirmation per package. Any
> architecture-level P0/P1 ⇒ block that package. PASS requires zero P0/P1.
> P2-only ADVISORY is not auto-harvested.

Round 1 produced four architecture-level P1 clusters and one P1 mechanical
cluster. The correction budget is explicitly restricted to **mechanical**
findings, so it did not open.

## Reviewer panel — cross-model diversity

Seven personas across seven distinct models, all run in round 1, all reviewing
plan revision `38b3120c`.

| Persona | Model | Verdict | Blocking findings |
|---|---|---|---|
| Rust Reviewer | `gpt-5.6-sol` | **FAIL** | 4 P1 (3 architecture-level, 1 mechanical); 4 P2 |
| Correctness Reviewer | `gpt-5.6-terra` | **FAIL** | 3 P1 (3 architecture-level); 3 P2 |
| Architecture Strategist | `claude-opus-4.8` | **PASS** | none (3 P3) |
| Constitution Reviewer | `grok-4.6` | **PASS** | none (1 P2) |
| Security Reviewer | `claude-sonnet-5` | **PASS** | none (3 P2) |
| Scope Boundary Auditor | `gemini-3.8-flash` | **PASS** | none (2 P2) |
| Maintainability Reviewer | `claude-opus-4.7` | **FAIL** | 2 P1 (2 architecture-level); 1 P2, 1 P3 |

Tally: **3 FAIL / 4 PASS** — the same split as attempt 1, but on entirely
different findings. No attempt-1 finding was reproduced.

## What this review SETTLES — do not re-derive on the next G0 attempt

These are affirmed and must be inherited, not re-argued. They are **in addition
to** the nine items settled by the attempt-1 review, all of which remain
settled.

1. **Attempt-1 finding A is CLOSED at the type level.** The Architecture
   Strategist — the designated adjudicator — rated probes 1 and 2 PASS and
   stated the repair is "the correct architectural fix, not a variant-padding
   patch". The site-less `FsFault` / site-bearing `FsError::Io` split with a
   single total lift function `FsError::io(site, fault)` is the right boundary:
   the adapter knows the operation, the resolver knows the site, and an
   implementor of `FileAccess` cannot mislabel a site because it is never handed
   one. Security and Constitution concurred. **Keep this design.** What is
   *not* closed is the ability to *prove* each channel — see finding B below.

2. **Attempt-1 finding B is CLOSED at the contract level.** Architecture probes
   3 and 4 PASS: "reject, not dedupe" is the correct contract choice for a
   boundary G1 counts over, and the repair is at the correct layer. Correctness
   probes 3 and 4 PASS: no input can produce two entries sharing a
   `canonical_path` or a `rel_path`, and comparing *canonical* roots genuinely
   catches the symlinked-intermediate-component case that attempt-1 finding C-1
   described. Scope probe 6 PASS: all five algorithmic additions are driven by a
   named prior finding, none is opportunistic. **Keep the three-layer identity
   design and the `rel_path` / `canonical_path` field split.**

3. **The `thiserror` dependency change is accepted.** Architecture probe 8,
   Constitution probe 7, and Scope probe 7 all PASS. `thiserror = "1"` is already
   a resolved root dependency, so the inherited "zero new external dependencies"
   constraint is satisfied in substance. **Do not revert to the attempt-1
   "no `[dependencies]` section" purity claim.** Only the *proof mechanism* is
   wrong — see M-2 below.

4. **The test tier move is correct.** Constitution probe 5 PASS: `tests/unit/`
   with target `unit_agent_contract_fs` matches the repository's isolated-logic
   tier definition and the `unit_*` / `tests/unit/*_test.rs` naming convention.
   Attempt-1 advisory A-6's over-claim is not reproduced. **Keep it.**

5. **The Constitution Check is accurate.** Constitution probes 1, 2, 4, 6, 8, 9
   all PASS: principles I–XI use real names and real numbering; all four quality
   gates are present in the constitution's order including `cargo audit`; the
   30-unit / 15-pair count is consistent between Work Units and row II; the
   five-file freeze scope matches G0.1. Attempt-1 findings C-1, C-2, C-3, C-4 and
   C-6 are **closed**. **Do not re-derive the principle mapping.**

6. **Lint enforcement is closed.** Constitution probe 3, Maintainability probe 6,
   and Rust probe 6 all PASS. The explicit
   `cargo clippy -p agent-contract-fs --all-targets -- -D warnings -D clippy::pedantic`
   command is valid for a workspace member that is a dev-dependency of the root
   package, and it genuinely closes attempt-1 M-9. **Keep it.**

7. **Scope discipline is intact.** Scope probes 1, 2, 3, 4, 5, 6, 7, 9 all PASS:
   every public item has a real exerciser; `read_corpus`'s happy path is now
   asserted, closing attempt-1 F-01; no hidden policy limit; no unit padding; no
   cross-boundary leakage into G1/G2; the H3 grep claim is correctly narrowed,
   closing attempt-1 M-15. **The surface is not over-corrected.**

8. **Complexity is proportionate.** Maintainability probe 3 PASS: 12 variants,
   the `FsSite` × `FsOp` discriminant, and the four-phase algorithm each carry
   weight. Probe 5 PASS: the seam is still four methods and still load-bearing
   even though `canonicalize` is now called per file. Probes 4, 7, 8, 9 PASS:
   the single test file is bounded, `too_many_lines` is correctly scoped,
   attempt-1 advisories A-10 and A-11 are recorded, and the
   `rel_path` / `canonical_path` naming closes attempt-1 A-1.

9. **The security core holds.** Security probes 1, 2, 5, 6, 7, 9 all PASS: root
   and entry substitution are rejected by mechanism; the
   `.github/agents-evil` vs `.github/agents` fixture is instantiated, closing
   attempt-1 SEC-2; no file can be silently omitted; no scenario needs
   elevation, developer mode, a real symlink, a junction, or a case-insensitive
   volume; per-file canonicalization adds no new symlink-following surface.
   **Do not reopen the privilege-free determinism design.**

10. **Windows case handling is correct.** Rust probe 7 PASS: relying on
    `canonicalize` to deliver platform semantics, with no case folding and no
    `cfg` fork in the resolver, is correct Windows behaviour, and the
    extended-length repair of attempt-1 M-5 is sound. Correctness probe 6 PASS.
    **Keep deliberation Q3 Option W-3 exactly as designed.**

## Terminal findings — ARCHITECTURE-LEVEL

### A — `read_corpus` performs no containment check and its input type is forgeable — HIGH CONSENSUS

* **Raised independently by**: Rust Reviewer (`R-1`, P1, architecture-level),
  Correctness Reviewer (`FS-1`, P1, architecture-level), and Security Reviewer
  (`SEC-2`, P2, architecture-level). **Three reviewers on three different
  models converged on the identical defect without seeing each other's output.**
* **Defect**: `ResolvedFile { pub rel_path: String, pub canonical_path: PathBuf }`
  has fully public fields, and
  `read_corpus<F: FileAccess>(&F, &[ResolvedFile]) -> Result<Vec<ReadFile>, FsError>`
  receives **no workspace root** and performs **no containment re-check**. Its
  only checks are the re-lstat and the read, both keyed purely on
  `canonical_path`. Any caller in the same process can hand-construct a
  `ResolvedFile` whose `canonical_path` points anywhere on disk, and `RealFs`
  will lstat and read it.
* **Why it is terminal**: the plan's Constitution Check row IV states
  unconditionally that "`resolve_corpus`/`read_corpus` touch only canonically
  contained paths". That is not a property of `read_corpus` — it is a property
  of well-behaved callers. Principle IV is marked **NON-NEGOTIABLE** in this
  repository's constitution. This is the same epistemic defect class that
  terminated attempt 1 and v3: **a claimed guarantee that the mechanism does not
  deliver.** Attempt 1 died because the plan's Failure Semantics table claimed
  completeness it did not have; attempt 2 dies because its containment claim is
  forgeable at the type level.
* **Why it is architecture-level, not mechanical**: the mechanistic repair
  changes the public type surface — `ResolvedFile`'s fields become crate-private
  with `resolve_corpus` as the sole constructor (or an opaque `ResolvedCorpus`
  is introduced), and/or `read_corpus` gains a workspace-root parameter and a
  containment re-check. Either changes the type G1 consumes across the package
  boundary. Two of three raising reviewers classified it P1 architecture-level
  independently.
* **Note on the documentation-only escape**: the Security Reviewer offered a
  documentation-only repair (narrow the Constitution IV row, as attempt 2 already
  does for the `RealFs` adapter per attempt-1 advisory A-8). That would be
  mechanical. It is **not** chosen here, because the Rust and Correctness
  reviewers both identified the forgeability as a defect in the *contract*, not
  in the *description of* the contract, and because G1 is the first consumer and
  will construct `ResolvedFile` values in its own test fixtures. The next attempt
  must decide this explicitly and justify the choice.

### B — The failure channels are not independently injectable, so channel completeness is unprovable — HIGH CONSENSUS

* **Raised independently by**: Rust Reviewer (`R-2`, P1, architecture-level) and
  Maintainability Reviewer (`M-1`, P1, architecture-level). Two reviewers, two
  models, convergent. The Correctness Reviewer's `FS-2` is the same root cause
  seen from the algorithm side.
* **Defect**: the declared `FakeFs` builder set names exactly one fault injector,
  `unreadable(kind)`, and G0.5(a) pins its semantics to **`symlink_metadata`
  only**. But six later scenarios lean on it to induce failures in *other*
  operations: G0.13(b) and G0.13(c) need a `canonicalize` failure, G0.21(a)
  needs a `read_dir` failure, G0.23(a) needs an entry `canonicalize` failure,
  and channel 7 (`ReadToString`) needs a read failure that no scenario induces
  at all. The plan never states whether `unreadable` is universal or
  operation-scoped, nor how `unreadable(workspace_root)` in G0.13(c) is
  distinguishable from `unreadable(root)` in G0.11(b) — under the current
  specification both are simply "`PermissionDenied` on `symlink_metadata`".
* **Additionally**, the Rust Reviewer found that channel 7
  (`Io { operation: ReadToString }`) is cited as proven by G0.29(b), but
  G0.29(b) removes the file *before* the read, so the re-lstat fails first and
  `read_to_string` is never reached. **That channel has no proving scenario.**
* **Why it is terminal**: attempt 2's central claim is that finding A is closed
  for *every* fallible seam invocation. The type design is correct (settled item
  1 above), but the *proof* is not constructible from the plan as written. A
  taxonomy that is complete but unprovable is the same failure mode as a
  taxonomy that is claimed complete and is not. This is also a direct recurrence
  of attempt-1 mechanical finding M-8 (`FileKind::Other` had no producer) one
  level up — the Maintainability Reviewer, who raised M-8, identified it as such.
* **Why it is architecture-level**: `FakeFs`'s builder set is a public dev-facing
  surface listed in the plan's Architecture table, and the repair changes it —
  either `unreadable` is redefined as an operation-indexed fault map, or it is
  replaced by per-operation injectors
  (`fail_symlink_metadata` / `fail_canonicalize` / `fail_read_dir` /
  `fail_read_to_string`). Both raising reviewers classified it this way.

### C — The entry `symlink_metadata` channel does not exist during traversal — HIGH CONSENSUS

* **Raised independently by**: Correctness Reviewer (`FS-2`, P1,
  architecture-level) and Rust Reviewer (`R-8`, P2, mechanical). Convergent on
  the defect; they differ on severity.
* **Defect**: Phase 4 explicitly decides from `read_dir`'s lstat-derived
  `DirEntry.kind`; it does **not** call `FileAccess::symlink_metadata` per entry.
  Yet G0.21(c), G0.22, and the Obligation-ownership table assign a traversal-time
  entry-lstat failure to the G0.21/G0.22 pair, and the eight-channel table lists
  channel 5 as proven there. The only unambiguous entry `symlink_metadata`
  invocation in the whole design is in `read_corpus`, which is not implemented
  until G0.30. G0.21(c) therefore cannot be red against G0.20's state and cannot
  be made green by G0.22 without adding an unplanned redundant lstat to Phase 4.
* **Why it is terminal**: the plan is genuinely ambiguous about whether
  `DirEntry.kind` is authoritative or whether every entry must be lstat'd again.
  That ambiguity changes the call log, the syscall count, the channel table, and
  the ownership of a failure channel — an implementing agent would have to invent
  the answer. The Correctness Reviewer classified it architecture-level because
  the repair changes both the channel table and the work-unit topology.
* **Repair options** (the next attempt must pick one explicitly): (i) keep
  `DirEntry.kind` authoritative and move channel 5's proof entirely into the
  `read_corpus` pair, renumbering the channel table; or (ii) require a second
  lstat per entry during traversal, state which kind is authoritative, and update
  Phase 4, the call-order tests, and R5's cost analysis accordingly.

### D — Red/green staging is broken for the failure channels — HIGH CONSENSUS

* **Raised independently by**: Maintainability Reviewer (`M-2`, P1,
  architecture-level) and Correctness Reviewer (`FS-3`, P1, architecture-level),
  with Correctness probe 9 and Maintainability probe 2 both FAIL.
* **Defect, part 1 (failure-channel wiring)**: under `-Dwarnings`, a `[G]` unit
  that introduces a seam call **cannot compile without handling its `Result`**,
  and the natural `?` + `map_err` handling wires the failure channel in the same
  commit. So G0.13(c) (workspace-root canonicalize failure) passes green against
  G0.12's Phase-1 implementation, and G0.21(a)/(c) pass green against G0.20's
  traversal implementation — before the `[G]` unit that supposedly owns them
  ships. The Obligation-ownership table, added specifically to close attempt-1
  M-1, does not solve this because the problem is not *who claims* the obligation
  but *when the compiler forces it to exist*.
* **Defect, part 2 (identity before display)**: G0.23(b) requires
  `DuplicateFileIdentity` to carry `first_rel` and `second_rel`, so G0.24 must
  construct lexical `rel_path` strings to make G0.23 green. But the ownership
  table assigns *all* `rel_path` construction to the later G0.26, whose G0.25
  tests are meant to drive it. Display-path construction is therefore implemented
  before its driving test.
* **Defect, part 3 (already-deterministic sort)**: G0.20 already sorts directory
  entries and G0.18 already sorts roots, so G0.27(a)'s "identical output under
  different insertion order" is already satisfied before G0.28's final sort ships
  — it cannot be reliably red.
* **Why it is terminal**: Principle II is **NON-NEGOTIABLE**. This is a direct
  recurrence of attempt-1 P0 finding M-1's defect class, and the Maintainability
  Reviewer — who raised the attempt-1 version — identified it as such. The repair
  re-cuts the work-unit topology, which is architecture-level by the adjudicated
  definition.

## Terminal finding — MECHANICAL but P1

### E — The declared `FsError` is not compilation-complete

* **Raised by**: Rust Reviewer (`R-4`, P1). Uncontested; no other reviewer probed
  the derive chain.
* **Defect**: `#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]` on
  `FsError` requires those traits on `FileKind`, `FsSite`, and `FsOp`, which the
  plan never states. `thiserror::Error` additionally requires an `#[error(...)]`
  (or `#[error(transparent)]`) attribute on **every** variant; none is specified.
* **Why it matters**: G0.1 requires the complete type surface to compile
  warning-clean before any green implementation. As written, the central type
  cannot be generated from the plan.
* **Repair**: state the derives on all three nested types explicitly (all are
  `Copy`-able fieldless enums except `FileKind`, which is also fieldless), and
  give every `FsError` variant a concrete `#[error(...)]` message consistent with
  the path-rendering policy.

## Mechanical findings — recorded, NOT corrected

The correction budget never opened. These are recorded so the next G0 attempt
inherits them rather than rediscovering them.

| ID | Sev | Raised by | Finding |
|---|---|---|---|
| **M-1** | **P1** | Rust (`R-3`); Scope (`F-01`, P2); Maintainability (`M-3`, P2) | **G0.11(a)'s call-log assertion is unsatisfiable — three independent reviewers.** Phase 1 unconditionally calls `Canonicalize(workspace_root)` **before** Phase 2 lstats the joined root, so the log can never contain zero `Canonicalize(_)` entries. G0.12 success criterion (2) and H1 repeat the same imprecise claim. Repair: assert the precise allowed order — `Canonicalize(workspace_root)` then `SymlinkMetadata(joined_root)` — and assert specifically that `Canonicalize(joined_root)` and every `ReadDir(_)` are absent. Contrast G0.27(c), which already words this correctly ("zero seam calls **after Phase 1**"). |
| **M-2** | **P2** | Rust (`R-5`); Constitution (`C-1`); Scope (`F-02`) | **The `Cargo.lock` proof is unsound and unsatisfiable — three independent reviewers.** Adding a workspace member plus a root path dev-dependency *does* add a local `[[package]]` record and a dependency edge, even though no registry crate is added. `git diff --stat Cargo.lock # expect no change` will fail on a correct implementation, and `--stat` could not distinguish local from external additions anyway. R2 contradicts itself in one paragraph — it admits "the resolved graph gains exactly one **local path** node" and then cites an unchanged lockfile as proof. Affects Dependency posture, G0.1(4), G0.30(4), Verification, Constitution row VI, R2, H7. Repair: keep the true claim (zero new **external/registry** crates) and prove it by inspecting the lockfile diff for entries bearing a registry/git `source`, or by comparing registry package IDs from `cargo metadata --locked`, with `cargo audit` as the backstop. |
| **M-3** | **P2** | Rust (`R-7`); Security (`SEC-3`) | **The path-rendering policy is not implementable by the stated lift function — two independent reviewers.** `FsError::io(site, fault)` receives no workspace root, and `FsFault.path` is populated by the adapter from the absolute path it was handed (all resolver paths are absolute after Phase 1). So 7 of 8 `Io` channels cannot be relativized, and runner-specific absolute paths leak into CI logs — the exact disclosure attempt-1 advisory A-4 / v3 A-10 flagged, which attempt 2 claims to close. Repair: give the lift function the canonical workspace root (`FsError::io(workspace, site, fault)`) and `strip_prefix` it, falling back to absolute only for `WorkspaceRoot`-site faults; add one assertion that an `Io { site: Root }` payload does not contain the absolute test prefix. |
| **M-4** | **P2** | Rust (`R-6`); Architecture (`A-1`, P3) | **H9 overstates the compile-time barrier — two independent reviewers.** Adding a fifth seam method does **not** force a new `FsOp` variant; the new method's `FsFault` can reuse an existing `FsOp` value. Nothing at the type level binds a method to a distinct operation label. The *conclusion* survives via a different mechanism the plan already has — every fault flows through the single total `FsError::io` lift, so a channel always exists, and adding a trait method breaks every `FileAccess` impl at compile time. Repair: restate H9 around the lift function and the trait-impl break; drop "forces a new `FsOp` variant"; describe one-`FsOp`-per-method as a stated convention, not a compiler guarantee. |
| **M-5** | P2 | Security (`SEC-1`) | **The TOCTOU residual is broader and less timing-dependent than H6 discloses.** The re-lstat closes leaf-symlink substitution and disappearance, but gives **zero** protection against (a) an intermediate ancestor directory replaced by a symlink after resolve — lstat does not follow only the *final* component, so a substituted subtree yields a normal `File` kind and a successful read — and (b) a regular file overwritten by another regular file at the same path, where the kind is unchanged and no error fires at all. Neither requires winning a race; both are open for the entire interval between the two calls. H6 calls the residual "the syscall-level race between the re-lstat and the read", which implies a nanosecond window. Repair: name both classes explicitly in H6 and state that neither needs timing precision. No code change is recommended — the four-method seam is settled. |
| **M-6** | P2 | Correctness (`FS-6`) | **G0.18's zero-`ReadDir` assertion proves less than the invariant it is cited for.** It shows traversal does not start for a *conflicting* fixture, but not the stated stronger invariant that Phase 2 completes for **every** root before any traversal on ordinary input. An implementation could validate-and-traverse the first root, then validate the rest, and still pass. Repair: add a call-log scenario with multiple non-overlapping roots asserting that all root lstat/canonicalize calls precede the first `ReadDir`. |
| **M-7** | P2 | Correctness (`FS-5`) | **Two root-grammar cases remain undefined or wrongly claimed.** (i) Component-wise `starts_with` is inclusive, so a root that canonicalizes to *exactly* the workspace root is accepted and traverses the entire workspace — never stated as intended, despite the direct `.` spelling being rejected. (ii) The claim that `C:\foo` and `\\?\C:\foo` are `Component::Prefix` and rejected on every platform is **false on Unix**, where Rust's `Path` parser treats backslashes and drive syntax as ordinary `Normal` characters. Repair: define accept/reject semantics for a root canonically equal to the workspace root, and make the Windows-prefix rejection platform-qualified with `cfg`-specific tests, or add a platform-independent lexical rule if those spellings must be rejected everywhere. |
| **M-8** | P3 | Maintainability (MINOR) | **`FsCall`'s variant set is never enumerated.** Four separate call-log assertions (G0.9, G0.11a, G0.18c, and Verification) depend on pattern-matching `FsCall::SymlinkMetadata(path)` and friends, but the enum is only named, never defined. Repair: show the definition alongside `FsOp` so an implementer does not conflate the two types. |

## Advisory findings carried forward

| ID | Sev | Raised by | Summary |
|---|---|---|---|
| A-1 | P3 | Architecture (`A-2`) | The `rel_path`-uniqueness derivation omits its load-bearing premise. Lexical subtree disjointness does not follow from canonical non-overlap *in general* — it follows here only because root- and entry-symlink rejection make lexical descent identical to canonical descent within any traversed root. Add that clause to the G0→G1 contract. |
| A-2 | P3 | Architecture (`A-3`) | The boundary uniqueness contract is stated for `Vec<ResolvedFile>`, but G1 consumes `Vec<ReadFile>`. Add: "`read_corpus` preserves the `ResolvedFile` order 1:1 into `ReadFile`, so uniqueness and strict increase hold on the returned `Vec<ReadFile>` as well." |
| A-3 | P3 | Rust (probe 3) | Red-phase warning-cleanliness holds for the mandated `_`-prefixed params and terminal `todo!()`, but imports must also be introduced only in the unit that uses them, or `unused_imports` fires under `-Dwarnings`. Add the rule alongside the existing private-state rule in H4. |

## What the next G0 attempt must settle first

1. **Decide `read_corpus`'s trust posture (finding A).** Either make
   `ResolvedFile` unforgeable — private fields, `resolve_corpus` as sole
   constructor, or an opaque `ResolvedCorpus` handle — **or** give `read_corpus`
   a workspace root and a containment re-check, **or** explicitly narrow the
   Constitution IV claim the way attempt 2 already narrows it for the `RealFs`
   adapter. Choose one, justify it, and make the Constitution IV row say exactly
   what the mechanism delivers and nothing more.
2. **Make every failure channel independently injectable (finding B).** Replace
   or redefine `FakeFs::unreadable` so a fault can be targeted at a specific
   `FsOp` on a specific path. Then give **every one of the eight channels** its
   own proving scenario — including `ReadToString`, which currently has none,
   because G0.29(b) fails at the re-lstat before the read is reached.
3. **Decide whether traversal lstats each entry (finding C).** Either
   `DirEntry.kind` is authoritative and channel 5 belongs to the `read_corpus`
   pair, or Phase 4 performs a second lstat per entry. State which, and update
   Phase 4, the channel table, the ownership table, the call-log tests, and R5.
4. **Re-cut red/green staging around the `-Dwarnings` constraint (finding D).**
   A `[G]` unit that introduces a seam call necessarily wires that call's failure
   mapping. So a failure-channel scenario cannot be owned by a *later* pair than
   the one introducing the call. Either bundle each channel with the pair that
   introduces its call, or split the introducing units finely enough that each
   channel has a genuinely red predecessor state. Also fix the two secondary
   staging breaks: `rel_path` construction is needed by G0.24 but owned by G0.26,
   and G0.27(a) is already satisfied before G0.28 ships.
5. **Make `FsError` compilation-complete (finding E).** State the derives on
   `FileKind`, `FsSite`, and `FsOp`, and give every variant an `#[error(...)]`
   message.
6. **Fix the three convergent mechanical defects**: the G0.11(a) call-log
   assertion (M-1), the `Cargo.lock` proof (M-2), and the path-rendering lift
   function (M-3). Each was raised by two or three independent reviewers.
7. **Correct the two overstated claims**: H9's compile-time barrier (M-4) and
   H6's TOCTOU residual (M-5). Both are cases of the plan claiming more than its
   mechanism delivers — the defect class that has now terminated three
   consecutive attempts.
8. **Do not reopen**: anything in the "What this review SETTLES" section above,
   or in the attempt-1 review's equivalent section. In particular, the `FsFault` /
   `FsError::Io` split, the three-layer identity design, the `thiserror`
   dependency, the `tests/unit/` tier, the Constitution mapping, the `-p` clippy
   command, the Windows `canonicalize`-driven case handling, and the
   privilege-free determinism design are all affirmed.

## Explicit non-actions

No implementation code written. No source, test, or configuration file modified
— the plan proposes edits to `Cargo.toml`, `.cargo/test-coverage-manifest.toml`,
`crates/`, and `tests/`; **none were applied**. No backlog item created — **no
IDs allocated**. No shipment assembled. No PR created; PR #396 untouched.
`143.*` left abandoned. Packages A–F not modified or restaged. The G split,
G1, and G2 were not reopened; G1 and G2 were **not reviewed** and their draft
plans were **not modified**. No stash entry archived. No dependency change. No
change to `.github/workflows/ci.yml`. The attempt-1 plan, deliberation, and
review record were read as evidence and **not modified**. The v2 and v3 records
were not touched.
