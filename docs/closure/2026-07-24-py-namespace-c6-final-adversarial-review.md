---
title: "Python namespace canonical resolution — final adversarial re-check (C6 + cycle-8 consistency)"
type: adversarial-review
date: 2026-07-24
branch: stage/py-namespace
head: 1d4f9caf5149a5136743441707fe146990abaf40
commit_subject: "docs(091-S): close non-import-rebind axis (C6) + reconcile cycle-8 consistency threads"
scope: docs/backlog-only planning artifact (no compilable code)
verdict: CONVERGED
reviewers: 1 (single-reviewer self-run synthesis applying Tier 1/2/3 lenses over one artifact set; NOT independent reviewer instances per adversarial-review.instructions.md)
---

## Verdict: **CONVERGED**

All four scoped items verify, the 9-vector composition table composes with exactly one
precision-floor-safe outcome per vector, and **C6 (def + non-import module/scope rebind) is
genuinely closed** — not merely re-described. The producer gap that every prior pass missed is
closed by a real, in-scope data source (T5a's own tree-sitter scan over the caller `source`),
and the routing is fail-closed on both indexing arms.

Two **LOW / P3** observations survive; both are precision/wording nits, **same-class-narrow**,
and **neither regresses current `main` behavior nor reopens the C6 false-edge**. They do not
gate merge. No P0/P1/P2 findings → **no blocking backlog items**.

Grounding note: this is a docs/backlog artifact, so the bar is internal consistency against the
real code seams. The cited seams were re-verified on disk at HEAD `1d4f9caf`:
`code_graph.rs:900-902` (full-index `(Some,Some)` → `create_calls_edge`), `1635-1636` (sync
`(Some,Some)` → `create_calls_edge`), `1639` (sync bare `put_staged_call`), `1213-1223` (sync
read seam / C7-3 anchor). **All seams match the plan's claims exactly.**

---

## Item-by-item verification

### 1. C6 non-import-rebind closure (the load-bearing one) — **PASS (HIGH confidence)**

**(a) Does T5a genuinely scan non-import rebind forms itself (not renamed prose over a
nonexistent producer)? — YES.**

The plan explicitly names and closes the producer gap, rather than re-describing it:

- Frozen invariant (L62–82) now closes over **both** the scope axis **and** the binding-form
  axis, naming the RFS as the resolver's model: *"neither a new scope nor a new syntactic rebind
  form can silently reopen a false-edge path."*
- §Design decision (L232–239) states the gap in plain terms — *"any non-import module/scope
  rebind form in the Shared rebind-form set (RFS), which T2/T2b do **not** produce (they walk
  only import nodes), so **T5a detects these forms with its own in-file tree-sitter scan — the
  SAME module/scope rebind-form scan T5c specifies**"* — over the caller file already available
  at the consumer site.
- §T5a (L497–511) is unambiguous: *"The non-import module/scope rebind axis (C6) … has **no
  producer**: T2 and T2b walk only import nodes, so these forms leave the predicate false and
  the arm still mints a false direct `M.parse` edge … T5a owns modifying `code_graph.rs:896-908`
  on BOTH arms so that … a same-file `def` is routed through `python_bare` staging whenever its
  name is contested by … **any non-import rebind form in the RFS that T5a detects with its OWN
  in-file tree-sitter scan.**"*
- Risks root-defect row (L902) and Requirements-Trace (L194–195) repeat the same mechanism.
- `096.005-T` AC (L27–28) and description (L35) carry the identical producer-gap language plus
  the explicit routing-table assertion `def f; f = factory(); f()` → **STAGED as python_bare,
  NO direct `M.f` edge**, also covering `class f`/`del f`/`match`-case capture.

This is the exact axis prior passes missed. Cycle-8 does **not** just rename an enumeration to an
"abstraction": it identifies the missing data source (the non-import forms have no producer) and
supplies one (T5a's own AST scan over data it holds at the seam). **Data-source reality check
performed** (see Finding LOW-1): the scan's input genuinely exists in scope.

**(b) Is the routing fail-closed? — YES.** Prove-the-negative default, stated identically in the
invariant (L75–79), §Design decision (L237–239), and §T5a (L509–511): *"keep the direct edge
ONLY when provably the sole binding across every modeled scope AND with no RFS form present;
otherwise route to staging."* If any competing binding — any form, any scope — cannot be proven
absent, it stages. Presence-detection is order-agnostic and over-approximating (precision-safe).

**(c) Both arms? — YES.** §Design decision L240–241, §T5a L501, Requirements-Trace L194, Risks
L902, cycle-8 record L1338 all state the change lands on **both** `896-908` (full-index) **and**
`1573-1643` (sync). The sync seam's `(Some,Some)` at `1635-1636` and bare `put_staged_call` at
`1639` are the exact loci named — verified on disk.

**Judgement on `def parse(); parse = factory(); parse()`:** T5a's RFS scan detects the
`assignment` form for `parse`, so the same-file `def parse` is routed to `python_bare` staging
instead of `create_calls_edge` (no direct `M.parse`). T5b then resolves the call-site-effective
binding; the assignment rebind after the winning def is caught by T5c's order-aware RFS
invalidation → **DROP**. Composition-trace **C6 = DROP** (L179). **Outcome is DROP, not a false
edge. Closed.**

### 2. T5a ↔ T5c no-drift — **PASS (HIGH confidence)**

- The RFS is defined **once**, in §Design decision (L242–256), with the explicit single-source
  mandate: *"defined once, here … consumed identically by T5a's in-file routing scan and by
  T5c's order-aware invalidation scan. **Neither task may enumerate its own divergent list.**"*
- `096.005-T` (T5a) consumes it order-agnostic: *"the SAME scan T5c runs … the plan
  Design-decision defines the RFS ONCE; T5a routing and T5c invalidation consume the identical
  set so they cannot drift … T5a's use is order-agnostic 'any RFS form present → stage'."*
- `096.007-T` (T5c) consumes it order-aware: *"the SHARED REBIND-FORM SET (RFS), defined ONCE in
  the plan Design-decision and consumed IDENTICALLY by T5a's in-file routing scan (096.005-T,
  C6) so the two scans can NEVER diverge on which forms count — the shared artifact is the
  syntactic FORM SET, not the order logic."*
- The illustrative form lists in both tasks and in §T5a/§T5c are **consistent** with the single
  authoritative RFS (assignment/augmented/`class`/`def`/`del`/`match`-case/`for`/`with-as`/
  `except-as`/walrus/parameter + star marker); no task presents a competing authoritative list.
  The only axis-of-difference is *order semantics* (T5a order-agnostic presence; T5c order-aware
  winner→call window), which is the intended, documented split — **not** a form-set divergence.

**They cannot diverge on which syntactic forms count. No re-enumeration of a divergent list.**

### 3. C8-1 duplicate same-name imports — **PASS (HIGH confidence), internally consistent**

All four surfaces agree; none claims last-binding disambiguation across duplicate imports:

- Plan §Resolution-rule C8-1 blockquote (L154–165): duplicate same-name imports are **not**
  order-disambiguated; T2 fails closed (M1, no binding) → T5b no target → drop; F3 legacy
  fallback also drops (name non-unique); *"last-binding-wins … disambiguates a name bound by an
  in-module `def` and **at most one** import binding (plus rebind markers) … it does **not**
  disambiguate two or more competing imports."* Explicitly *"fail-closed with no regression"* vs
  main; ordered-import-history a documented v1 non-goal.
- Plan §T5b sub-bullet (L582–588): *"duplicate same-name imports → fail closed (C8-1) … The
  call-site ordering above resolves `def`-vs-single-import + rebinds, **not** import-vs-import of
  the same name; the F3 legacy fallback also drops (name non-unique). No regression vs main."*
- `096.002-T` (T2, M1) AC (L25): two from-imports of one name → **no binding**; *"C8-1: duplicate
  same-name imports are NOT disambiguated by source order … T5b's call-site last-binding rule
  resolves def-vs-SINGLE-import + rebinds only."*
- `096.006-T` (T5b) AC (L31) + description (L37): `from a import f; from b import f; f()` → **no
  canonical edge** (T2 no binding per M1; T5b no target; F3 also drops).

**No residual half-state.** T2 erasing the binding and the feature rule promising ordering are
reconciled: the frozen rule now *scopes* last-binding to def-vs-single-import + rebinds. **No
regression to main** — main drops name-only-ambiguous today; C8-1 matches that.

### 4. C8-2 spike-doc consistency — **PASS (HIGH confidence)**

`docs/decisions/2026-07-23-…-spike.md`:

- Rule list (~L37) now carries a clearly-labeled **"⚠️ Refined (C7-4/F3)"** note (L39–45):
  *"DROP means no canonical module-qualified edge; a unique cross-file bare call still keeps its
  legacy name-only edge … Only the canonical layer drops; a non-unique name still fails closed.
  This qualifies every 'DROP' in this spike, including the Fail-closed matrix below … The spike's
  initial 'dropped outright' phrasing is superseded."*
- Fail-closed matrix (~L205) now carries a matching **"⚠️ Refined (C7-4/F3)"** note (L217–227):
  *"DROP here means no canonical (module-qualified) edge; a unique legacy name-only edge may
  remain … the star-import, relative-import, re-export, and `__init__.py`/PEP-420 rows in
  particular fall through to T5b's no-target legacy unique-match … the earlier 'dropped outright'
  reading is superseded."*

**No row still asserts star/relative/re-export are "dropped outright"** unqualified. Notes are
additive, clearly labeled superseded/refined, and match plan + `096-F` post-C7-4.

---

## Composition check — 9-vector Resolution composition trace (plan L167–182)

| # | Vector | Outcome | Owning layer | Precision-floor-safe? |
|---|---|---|---|---|
| C1 | def-only (`def f; f()`) | `M.f` direct fast path | T5a in-file arm | ✅ |
| C2 | import-only (`from N import f; f()`) | `N.f` | T5b | ✅ |
| C3 | def-before-import | `N.f` (last before call) | T5a→staging→T5b | ✅ |
| C4 | import-before-def | `M.f` (last before call) | T5a→staging→T5b | ✅ |
| C5 | def + star-after-def | **DROP** | T5a→staging→T5c | ✅ |
| **C6** | **def + non-import rebind** (`def f; f=g; f()`; class/del/match/for/with/except/walrus/param) | **DROP** | **T5a routing (shared RFS scan) → T5c invalidation (same RFS)** | ✅ **now closes** |
| C7 | def + function-local import | in-body `b.f`; pre-import → fail closed | T5a→staging→T2b/T5b/T5c | ✅ |
| C8 | provable-namespace unbound (unique) | DROP canonical, keep legacy name-only | T5b F3 | ✅ |
| C9 | call-before-later-rebind | `M.f`; fn-body-under-later-rebind → fail closed | T5b C7-1 | ✅ |

All 9 vectors yield exactly one outcome; everything unprovable fails closed. **C6 closes to DROP
(no direct `M.f` edge).** The C6 owning-layer cell names two layers ("T5a routing + T5c
invalidation"), but this is a *pipeline* (identical in kind to C3/C4/C5's "T5a→staging → T5x")
whose **terminal decision owner is T5c**; T5a's role is the routing precondition that prevents
the direct-edge bypass. There is no conflicting-outcome ambiguity — both layers act on the same
DROP side. Not a "two-owner" violation.

---

## Findings (confidence × severity)

### Consensus / HIGH confidence
None adverse. All four items and all 9 vectors verify PASS. The C6 producer gap is genuinely
closed with a real in-scope data source.

### Majority / MEDIUM confidence
None.

### Unique / LOW confidence (advisory — do not gate merge)

**LOW-1 (P3, precision/wording — HIGH confidence it is a real imprecision, LOW severity).**
The plan and `096.005-T` repeatedly assert T5a *"already holds the caller file AST"* at the
consumer site (plan L236, L508, L530–531, L1326–1327, L1343–1344; `096.005-T` L35, L39). Verified
against code: at **both** consumer loops (`code_graph.rs:851` full-index, `:1573` sync) the only
parse artifact in scope is `parse_result: ParseResult`, whose definition in
`src/services/parsing.rs` carries **only** `symbols: Vec<ExtractedSymbol>` and
`edges: Vec<ExtractedEdge>` — **no `tree_sitter::Tree`**. What *is* in scope is the raw
`source: String` (bound before the parse; the parse consumes a `source.clone()` at `609`/`1290`,
so the original `source` survives to the loop). Therefore, to run *"its own in-file tree-sitter
scan,"* T5a must **re-parse the in-scope `source`** (a second in-memory tree-sitter parse), not
read a pre-held tree.
- **Impact:** The load-bearing conclusion — *"no new producer, DB read, task, or DAG edge"* —
  **remains true**: re-parsing an already-in-memory string needs none of those. The C6 closure is
  genuinely achievable with data at the seam. The only inaccuracies are (i) the word "AST"
  (precisely: `source`, re-parsed) and (ii) an **unbudgeted second tree-sitter parse per Python
  file** (CPU, in-memory, no I/O — cheap relative to the existing read/parse, but not free and not
  acknowledged).
- **Regression risk:** None. Does not reopen the C6 false edge; does not touch `main` behavior.
- **Same-class-narrow:** Yes — a wording/cost-accounting nit within the exact seam already owned
  by T5a.
- **Suggested (non-gating) correction:** In `096.005-T` and the plan's C6 prose, replace
  *"already holds the AST"* with *"already holds the caller `source` in scope and re-parses it
  in-memory for the scan (no new producer/DB/task/DAG edge; one extra in-memory tree-sitter parse
  per Python file)."* This preserves the (correct) DAG/producer conclusion while making the data
  source and its cost accurate. Files/lines: plan L236/L508/L530-531/L1326-1327/L1343-1344;
  `.backlogit/queue/096.005-T.md` L35, L39.

**LOW-2 (sub-P3, advisory — MEDIUM confidence, negligible severity).**
The RFS enumerates *"plain assignment (`name = …`)"* and *"augmented assignment"* (plan L247).
Python's **annotated assignment** (`name: T = v`) is a distinct tree-sitter node
(`annotated_assignment` vs `assignment`) that also rebinds `name`. A literal implementation that
matches only `assignment`/`augmented_assignment` node kinds could miss `name: T = factory()`.
- **Impact:** At worst a single missed rebind *form* → a residual false direct edge for
  `def f; f: T = factory(); f()` on the fast path.
- **Regression risk:** **None vs main** — `main` mints the false direct `M.f` edge for *every*
  non-import rebind today (that is the whole C6 defect); the plan strictly improves precision, and
  this sub-form is no worse than status quo.
- **Same-class-narrow:** Yes — a subtype of "assignment" already inside the RFS's intent. Not a
  new axis.
- **Suggested (non-gating):** When implementing the RFS scan, treat `annotated_assignment` (with
  a value) as an assignment form. This is an implementation footnote, not a plan-consistency
  defect; the RFS's *intent* ("assignment") already covers it.

---

## Remediation plan (ordered by confidence × severity)

| Priority | Finding | Class | Action |
|---|---|---|---|
| P3 | LOW-1 (AST-vs-source wording + unbudgeted re-parse) | advisory | Optional doc edit to plan + `096.005-T`; **not required for merge** |
| sub-P3 | LOW-2 (annotated-assignment RFS sub-form) | advisory | Implementation footnote for the T5a/T5c scan; **not required for merge** |

No `safe_auto` / `gated_auto` / `manual` items. No P0/P1 → **no backlog work items created.**

---

## Backlog / issue-queue entries

None. There are no P0 or P1 findings. The two LOW/P3 observations are advisory and may be folded
into the existing tasks at implementation time without a new work item.

---

## Bottom line

**CONVERGED.** The commit `1d4f9caf` closes the C6 non-import-rebind axis with a genuine
mechanism (T5a's own tree-sitter scan over in-scope caller `source`, consuming the single-source
RFS shared with T5c), fail-closed on both indexing arms, and the three internal-consistency
threads (T5a↔T5c no-drift, C8-1 duplicate-import narrowing, C8-2 spike refinement) are all
mutually consistent across plan + `096.002-T` + `096.005-T` + `096.006-T` + `096.007-T` + spike.
The 9-vector composition table composes with one precision-floor-safe outcome per vector and
everything unprovable fails closed. The only surviving items are two LOW/P3 precision notes
(AST-vs-source wording; annotated-assignment sub-form), both same-class-narrow and non-regressing.
**No surviving false-edge hole. Clear to merge.**
