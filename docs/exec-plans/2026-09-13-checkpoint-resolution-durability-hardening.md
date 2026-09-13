---
doc_type: exec-plan-hardening
date: 2026-09-13
revision: 11
scope: defect-2-only
status: under-review
review_verdict: pending
review_verdict_revision: 11
review_attempts: 5
harvest_authorized: false
plan_document: docs/exec-plans/2026-09-13-checkpoint-resolution-durability-plan.md
supersedes: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-hardening.md
supersedes_normative_content_of: revision 10 of this document
policies: [P-001, P-005, P-006, P-009, P-010, P-011, P-012, P-014, P-016, P-018, P-019, P-020, P-022]
contains_proposed_action: true
---

# Checkpoint Resolution Durability — Plan Hardening (revision 11, Defect 2 only)

Hardening applied under P-006 to
`docs/exec-plans/2026-09-13-checkpoint-resolution-durability-plan.md`. The plan
declares `requires_plan_hardening: yes` because it changes merge-adjacent
ordering governed by P-014/P-018, introduces a startup route that could, if
mis-specified, become an unsupervised merge path, and carries a `ProposedAction`
unit that mutates repository settings.

**Review status (frontmatter is authoritative).** `revision: 11`,
`review_verdict: pending`, `review_verdict_revision: 11`, `review_attempts: 5`,
`status: under-review`, `harvest_authorized: false`. **This document authorizes
no harvest.** Revisions 6, 7, 8, 9 and 10 all returned **FAIL** from independent
review; the round-10 panel was unanimous across four cross-model reviewers with
4 P0 and 17 P1 findings, and recommended replacing the durability mechanism
rather than repairing it a third time.

**Revision 11 is the third and final bounded remediation revision in the
operator-authorized outcome cycle.** It carries the architecture-validated
**protected immutable annotated-tag** design. Its own review has **not yet
returned**.

## What revision 11 changes in this document

Revision 11 **retires** the hardenings that existed only to keep the deleted
machinery coherent, and **replaces** them with hardenings against the new
design's actual failure modes. Retiring a hardening is recorded, never silent.

| Prior | Disposition at revision 11 |
|---|---|
| **D9** — the obligation must survive a failed closure | **Retired by deletion.** There is no obligation *record* with a lifecycle to survive; there is a tag whose status is derived. A failed closure leaves the marker `PENDING`, which is the fail-closed signal by construction (new **D9′**). |
| **D10** — a pending locator must never be mistaken for a published one | **Retired by deletion.** The PR-body locator state machine is deleted. There are no locator phases to confuse. A marker either exists on the remote or does not. |
| **D11** — a rewritten branch must not silently drop the resolution commits | **Superseded by D11′.** The marker is a protected tag, unreachable by any branch operation, so a rewrite cannot drop it; the rewrite is *detected* instead by the `C`-ancestor-of-`H` re-check. |
| **D13** — a masked obligation is an undischarged obligation | **Retired by deletion.** Masking was a property of the two-channel record. Ancestry cannot be masked. |
| **D14** — a recovery step an agent cannot perform is not a recovery | **Superseded by D14′.** The working-tree placement procedure (`WP-0`…`WP-7`) is deleted along with the committing recovery branch it served; discovery is now entirely read-only and creates no worktree. The *principle* is preserved and re-aimed at the new protocol. |
| **D15** — the record must survive deletion of the PR body | **Superseded by D15′.** Preserved as a requirement, satisfied structurally rather than by machinery: discovery reads tags and never reads a PR body. |
| **D16** — detectability must rest on a proven protection | **Superseded by D16′.** Preserved as a requirement, re-aimed from the **branch** ruleset (which round 10 found breaks workflows repository-wide) to the **tag** ruleset, with two corrections of substance: the load-bearing rule and the non-restriction of creation. |

`D1`–`D8` and `D12` carry forward with their substance intact; their folds are
re-pointed to the revision-11 units. The full revision-10 text of every hardening
is retained below the line as immutable historical evidence.

## D1 (BLOCKING) — Resolution must not be recursive

**Risk.** Any mitigation that creates a further Git-tracked checkpoint to cover
the residual window must itself be resolved by a further commit, which needs a
further PR once the current one merges. The remedy never terminates.

**Fold.** Plan `RESOLUTION_MARKER` — every checkpoint owned by the unit is
resolved **before** the final gates, in **one** commit, and **no** new
Git-tracked checkpoint is created to cover the residual window. U5 removes
generic, session-end and post-merge resolution outright, so there is no site at
which a recursive checkpoint could be created.

**Verified by** V5, V22.

## D2 (BLOCKING) — The residual window must be covered by something

**Risk.** Because every checkpoint is resolved before merge, the interval between
the last resolution and verified delivery is deliberately checkpoint-free. If
nothing covers it, a crash in that window is indistinguishable from a clean
state — and is strictly worse than the pre-change still-active checkpoint.

**Fold.** Plan `RESOLUTION_MARKER` (the marker is published **in the same atomic
push** as the resolution commit, so the window has an object covering it from
the first instant it exists) and `RESOLUTION_MARKER_DISCOVERY` (a fresh checkout
with zero active checkpoints still finds the marker by scanning tags).

**Verified by** V9, V13.

## D3 (BLOCKING) — The obligation object must not be self-referential

**Risk.** A record that must contain its own commit's SHA is unimplementable: the
SHA is not known until the commit exists, and writing it changes the commit.
Revision 4's locator failed for exactly this reason.

**Fold.** Plan `RESOLUTION_MARKER` payload — the target commit `C` is
**deliberately excluded** from the payload. The annotated tag **object header**
supplies the binding, written by Git at tag-creation time. The object that names
`C` is not the commit, so nothing is self-referential.

**Verified by** V7 (the peel assertion proves the binding exists without the
payload carrying it).

## D4 (BLOCKING) — Evidence must be produced at the final HEAD, not relabelled

**Risk.** Moving a reviewed-HEAD SHA without re-running the review re-points a
stale verdict at a HEAD that, by construction, contains commits the earlier
review never examined. That is a false attestation.

**Fold.** Plan U4 AC2 — the **actual local review** re-runs at the current head
after publication, for **every** unit. U4 AC7 — a HEAD change **voids** the
approval and requires a fresh review and a fresh HEAD-bound approval.

**Verified by** V16, V24.

## D5 (BLOCKING) — The readiness sequence must be satisfiable

**Risk.** §1.9 reads the PR body and requires `Reviewed HEAD == headRefOid`.
Running the gate **before** the resolution commit is unsatisfiable, because the
commit advances HEAD past whatever the body recorded. The live Ship file gates
readiness (items 7b/7c) **before** four mutating items and the push.

**Fold.** Plan U4 AC2 installs the total order: ordinary mutations → freeze →
publish → **actual review at the current head** → **advisory PR-body
`Reviewed HEAD`** → §1.9 → explicit required-check evaluation → P-018 → HEAD-bound
approval → amended last-mile → expected-head merge. Items **7b and 7c MOVE**
after the push. The PR-body write is metadata and does not advance `headRefOid`
(plan `HEAD_EVIDENCE_RULE`), which is what makes the ordering terminate.

**Verified by** V24, and V5 for the zero-checkpoint path.

## D6 (BLOCKING) — Recovery must not trust stored state

**Risk.** A resumed session that trusts a stored SHA, a stored status, or a
stored branch name acts on a world that may have changed.

**Fold.** Plan `RESOLUTION_MARKER_DISCOVERY` — there **is** no stored status.
Status is computed live from `git merge-base --is-ancestor <C> origin/main`
against a freshly refreshed default branch. Provenance is re-derived live from
the commit→pulls API and re-proved by ancestry against a freshly fetched live
head. U4 AC6's amended last-mile re-fetches every input unconditionally.

**Verified by** V14, V21, V24.

## D7 (BLOCKING) — Discovery must be exhaustive or fail closed

**Risk.** 139-S's shipment was *archived* while its obligation was outstanding. A
status-filtered, truncated, or racy scan misses the motivating case and reports
"clean".

**Fold.** Plan `RESOLUTION_MARKER_DISCOVERY` fixed-point scan — list, **client-side**
exact-prefix filter (so a server-side glob quirk cannot truncate), fetch each
candidate to a **unique** ref, verify listed OID equals fetched OID, then
**re-scan and require a stable `(name, OID)` set** within a bounded retry count
or **halt**. Every validation failure halts; none is skipped.

**Verified by** V11, V12, V15, V26.

## D8 (BLOCKING) — Recovery must not become an auto-merge path

**Risk.** A startup route that can reach a merge is an unsupervised merge path,
whatever it is called.

**Fold.** Plan `RESOLUTION_MARKER_DISCOVERY` — "reaching the recovery path
confers **no merge authority**". U5 AC1 blocks and routes; U7 AC2 routes
exclusively to Ship recovery and does nothing itself; U6 AC6 states Stage gains
no merge authority (P-010).

**Verified by** V14, V23.

## D9′ (BLOCKING) — A failed closure must leave a loud, fail-closed signal

**Risk.** A shipment that cannot reach normal termination must not silently
disappear from the record.

**Fold.** Plan `RESOLUTION_MARKER_DISCOVERY` ancestry rule — if `C` never becomes
an ancestor of `origin/main`, the marker reads **`PENDING`**, permanently, and
`PENDING` **blocks new shipment work** (U5 AC1, U7 AC2). This is the designed
outcome, not a defect (plan RR-7). **No auto-recovery is attempted**, because
auto-recovery would require the cross-run continuation semantics this plan is
forbidden to add.

**Verified by** V15, V17, V20.

## D11′ (BLOCKING) — A rewritten or deleted branch must not silently drop the evidence

**Risk.** A force-push or branch deletion can erase a commit-borne record using
only the author's ordinary push rights. Revision 3's record failed on exactly
this; revision 4's fix (protecting branches) broke every contributor's workflow.

**Fold, in two parts.** **(a) Survival** — the marker is a **tag** in a namespace
protected against update and deletion with no bypass (U0). No branch operation
reaches it: a force-push, a branch deletion, and a PR-body edit all leave it
intact, still peeling to `C`. **(b) Detection** — U4 AC6's amended last-mile
requires **`C` to be an ancestor of `H`**; a rewrite that drops `C` **halts**
before merge.

**Deliberate correction of scope.** Protection covers **tags only**, at
`refs/tags/resolution-obligation/**`. No working branch is covered, so rebase,
`--amend`, `--force-with-lease` and merged-branch cleanup remain fully permitted.

**Verified by** V17, V13.

## D12 (BLOCKING) — A universal policy must not have a procedural hole

**Risk.** A policy binding "no resolution into an already-merged carrier" is not
realized if one of its two named agents' own procedure still permits it.

**Fold.** Plan U6 — Stage's Session-end site and `OWNER-SCOPED RESOLUTION` block
both receive the pre-merge staging-finalization carrier guard, and U5 removes
Ship's generic/session-end/post-merge resolution. Both named agents are covered.

**Correction carried from round 10 (F-15).** Revision 4 realized this by making
`context.pr` / `context.branch` **mandatory at checkpoint creation**, which is
unsatisfiable — a Stage checkpoint legitimately predates the PR that will carry
it. Revision 11 requires **no** future PR identity at creation; the carrier is
**supplied by the caller** at an explicit pre-merge finalization, queried
exactly, and required to be repo-bound, head-bound and **OPEN**. Ambient
derivation is prohibited outright.

**Verified by** V23.

## D14′ (BLOCKING) — A recovery step an agent cannot perform is not a recovery

**Risk.** A recovery procedure that assumes a working-tree state the agent cannot
safely reach is not a recovery; it is a halt with extra words. Round 8 found this
as a defect in this very document, and round 10 found the replacement's fetch
contract unsatisfiable across four surfaces.

**Fold, by removing the need rather than specifying the procedure.** Revision
11's discovery is **entirely read-only**: it creates local refs under
`refs/autoharness/marker-scan/` and **never** switches branches, never commits,
never creates a worktree, and never requires a clean tree. There is **no**
committing recovery branch, so `WP-0`…`WP-7` are deleted rather than repaired.
Every ancestry, `cat-file`, `show` and `log` command names an **explicit
commit-ish**, and `FETCH_HEAD` is **prohibited** as a fetch destination or
assertion target (U2 AC3) — one fetch form, one ancestry target, one surface.

**Verified by** V25 (the `FETCH_HEAD` and `WP-` deletion classification), V26.

## D15′ (BLOCKING) — The evidence must survive deletion of the PR body

**Risk.** If the PR body is the only obligation record, emptying it leaves
startup with zero checkpoints and zero locators, concluding "clean" — strictly
worse than the pre-change still-active checkpoint.

**Fold, structural rather than mechanical.** Plan `RESOLUTION_MARKER_DISCOVERY`
reads **tags**. It never reads a PR body to find a marker, and the PR body is
explicitly demoted: `HEAD_EVIDENCE_RULE` states the `Reviewed HEAD` record is
**advisory metadata for the §1.9 gate**, is **not** a durability mechanism, and
is read by **no** recovery path (U1 AC1). The PR body is consulted only *after*
a `PENDING` marker has already been found, and then only to validate provenance
against payload fields — never to establish that an obligation exists.

**Verified by** V13.

## D16′ (BLOCKING) — Detectability must rest on a proven protection, not an assumption

**Risk.** An immutable marker is only immutable if something enforces it. An
unprotected tag can be moved or deleted by anyone with push access.

**Fold.** Plan U0 (the ruleset) plus `RESOLUTION_MARKER` precondition 4 (the
proof) plus U4 AC6 (the last-mile re-proof). The proof reads the ruleset **by
recorded ID** and asserts `enforcement: active`, `target: tag`, the exact
include, empty exclusions, the update- and deletion-prohibiting rule types,
`bypass_actors: []`, and `current_user_can_bypass: "never"`. Failure, ambiguity
or API unavailability **halts before publication**; drift at the last mile
**halts before merge**.

**Two corrections of substance from round 10.**

1. **The load-bearing rule is *restrict updates*, not `non_fast_forward`.**
   `non_fast_forward` is a branch force-push rule; applied to a tag it would
   reject only *non-fast-forward* movement, so a tag retarget that happens to be
   a fast-forward along the same lineage would still be permitted — precisely
   the mutation immutability must forbid. Reusing the branch rule here would be
   a **silent under-protection**, which is worse than no protection because it
   would pass the proof.
2. **Creation is deliberately NOT restricted.** Ship must publish new markers
   through the ordinary push path **with no bypass**. Restricting creation would
   force Ship to hold bypass rights, contradicting the empty-bypass requirement
   the whole hardening depends on.

**Two residuals, stated rather than claimed away.**

* A repository **administrator** can edit or delete the ruleset. The guarantee is
  exactly as strong as the ruleset. This is narrower than the any-collaborator
  exposure it replaces and is **detected** at the next proof rather than silently
  absorbed (plan RR-3a).
* GitHub exposes an effective-rules endpoint for **branches** but **no per-tag
  equivalent**. The proof is therefore a **configuration read by recorded ruleset
  ID**, asserted field by field — not an effective-rules evaluation. The plan
  makes no stronger claim, and the operator verifies the applied ruleset in the
  UI at apply time as a second witness (plan RR-3b).

**Approval boundary.** U0 is **`ProposedAction`** / **`ActionRisk: high`** /
**`approval_required: true`**, a **manual operator/admin step under P-019**,
**never agent-executed**, **never** satisfied by a dark-mode approval record, and
**not applied by any planning pull request**. Ship is **never** granted
ruleset-write credentials.

**Verified by** V10, V7.

## D17 (BLOCKING) — Publication must be atomic, or it must not happen

**Risk.** Publishing the resolution commit and the marker as two sequential
pushes admits a torn state: a commit with no marker (the obligation is invisible)
or a marker with no commit (the marker attests a commit the remote does not
have). Either is worse than not publishing.

**Fold.** Plan `RESOLUTION_MARKER` publication — **one** `git push --atomic` with
fully-qualified refspecs and **two explicit leases**: the branch lease pinned to
the captured remote OID `R`, and the tag lease with an **empty expected value**
asserting the tag does not exist. **Unsupported `--atomic`, a failed lease, a
rejection, or any ambiguity leaves both refs unchanged and halts** — no partial
retry, no per-ref fallback, no single-ref re-push.

**Empirically validated before this revision was written**, in both rejection
directions, against a real remote: a stale branch lease produced
`HEAD -> main (stale info)` **and** `<tag> (atomic push failed)` with the tag not
created; a competing tag produced `<tag> (stale info)` **and**
`HEAD -> main (atomic push failed)` with the branch not advanced. The plan
records these as validated semantics rather than as assumptions.

**Verified by** V7, V8.

## D18 (BLOCKING) — The tag ref's OID is not the commit

**Risk.** An annotated tag ref resolves to the **tag object** `T`, not to the
commit `C` it targets. A protocol step that treats the tag ref's OID as a commit
will compare the wrong object, and an ancestry test against `T` will be
meaningless. A **lightweight** tag resolves directly to a commit, carries no
message, and therefore carries **no payload at all**.

**Fold.** Plan `RESOLUTION_MARKER` tag-object identity rule and post-publication
verifications 2–4: the remote tag ref must equal **`T`**; the peeled
`refs/tags/<tag>^{}` must equal **`C`**; the object type at `T` must be **`tag`**.
`RESOLUTION_MARKER_DISCOVERY` validation rules 1–2 require the object type `tag`
and **exactly one** level of peeling. A lightweight tag **halts**.

**Empirically confirmed**: after a real atomic publication, `git ls-remote`
reported `<T> refs/tags/<tag>` and `<C> refs/tags/<tag>^{}` as two distinct
lines carrying two distinct OIDs.

**Verified by** V7, V11.

## D19 (BLOCKING) — Idempotent recovery must be proven, never inferred

**Risk.** A lost push response is indistinguishable from a failed push at the
client. If the retry treats an existing tag as "already done" without proving it
is the *same* marker, a competing marker is silently accepted as one's own.

**Fold.** Plan `RESOLUTION_MARKER` idempotence-and-conflict rules — an existing
marker is idempotent recovery **only** when it peels to the same `C` **and** its
payload is byte-identical canonical JSON with a valid digest. Anything else is a
**conflict** that **halts**; it is never deleted, updated, or re-pointed.

**The specific trap, recorded rather than left implicit.** When the remote tag
OID already equals the local `T`, Git classifies the tag refspec as up to date
and does **not** evaluate its lease, so the atomic push proceeds for the branch
alone and reports success. **This must not be read as proof of identity.** The
plan requires the byte-identical canonical proof to be performed in the
preconditions, *before* publication, and explicitly forbids inferring identity
from the push's silence (U1 AC11). This was observed directly during validation.

**Verified by** V9, V12.

## D20 (BLOCKING) — The resolved set must be frozen at publication

**Risk.** A checkpoint appearing after the marker is published makes the marker's
payload incomplete. Amending the marker would violate immutability; publishing a
second marker would break the one-per-primary-key invariant and create two
competing sources of truth.

**Fold.** Plan `RESOLUTION_MARKER` — the finalization freeze, the
**re-enumeration immediately before the commit** (a changed set **restarts
preparation**, so a late checkpoint before publication is absorbed cleanly), and
the after-publication rule that a late or ambiguous checkpoint **halts** without
producing a second tag and without amending the first. U4 AC6's amended
last-mile re-checks the current checkpoint count specifically to detect it.

**Verified by** V18, V6.

## D21 (BLOCKING) — Post-publication remediation must remain possible

**Risk.** Requiring `C == H` at the last mile would make **any** post-publication
review remediation unmergeable, forcing either an abandoned marker or a refusal
to fix a genuine reviewer finding. Revision 4's equality form had exactly this
defect.

**Fold.** Plan `RESOLUTION_MARKER` after-publication rule — review-remediation
commits **may append above `C`**, provided they do **not** alter the resolved
set, and the last-mile requirement is **`C` is an ancestor of `H`**, not `C == H`.
U4 AC7's refresh rules then govern the consequence: a HEAD change **voids** the
approval and requires a fresh review and a fresh HEAD-bound approval.

**Verified by** V16.

## D22 (BLOCKING) — Marker discharge is not release closure

**Risk.** A single "closed" signal that conflates "the resolution landed" with
"the release is closed" will, sooner or later, be read as authority to start the
next shipment. That is a P-001 bypass wearing a different name.

**Fold.** The plan's authorized RQ-7 clarification, stated at three points of
use: `RESOLUTION_MARKER_DISCOVERY`'s status rule (U2 AC6), P-022 (U3 AC4), and
Ship's Merge Confirmation Gate (U5 AC3). All three state that `DISCHARGED` means
**checkpoint-resolution delivery to the protected default branch only**, that it
**does not prove P-001 closure**, and that it **does not authorize the next
shipment**. U3 AC5 names P-001, P-009, P-014, P-018 and P-020 and states that
**none** is weakened, satisfied, or bypassed by any marker state. U7 AC3 states
the same for the Orchestrator's pre-queue route.

**This separation is why revision 11 is smaller without being weaker**, and it is
the one clarification the operator authorized — explicitly **not** permission to
weaken any of the named policies.

**Verified by** V21, V22.

## D23 (BLOCKING) — Zero must be proven, and the zero path must not be a bypass

**Risk.** Two distinct failures. A failed, quarantined or ambiguous enumeration
read as "zero checkpoints" publishes nothing and merges — the silent-loss case. A
zero-checkpoint unit routed down a "pre-existing path unchanged" bypass escapes
the reordered gates entirely.

**Fold.** Plan `RESOLUTION_MARKER` enumeration rules — enumerate with
`consumer_id` **only**, with **no `status`/`agent` API prefilter** (such a filter
silently excludes quarantined records, which are exactly the ones that must be
seen), **inspect malformed and quarantined records first over the full
enumeration**, and treat any failure, malformation, quarantine, ambiguity or
absent tooling as **not zero** — it **halts** (P-012). U4 AC3 then states that a
proven zero omits **exactly three** things — `C`/`T` creation, the atomic
publication, and the post-publication verification — and **executes the identical
review, gate, approval and last-mile tail**. AC3 expressly **prohibits** claiming
any unit runs "the pre-existing path unchanged".

**Verified by** V5, V6, V26.

## D24 (BLOCKING) — Do not claim guarantees the server does not provide

**Risk.** Overstating a guarantee is a correctness defect, not a wording defect:
downstream reasoning built on a false guarantee is unsound.

**Fold.** Three claims are bounded explicitly in the plan. **(a)** Expected-head
merge pinning protects **only the HEAD race**; U4 AC8 forbids claiming
transactional stability for thread, review or check state unless server-enforced.
**(b)** The tag-ruleset proof is a **configuration read**, not an effective-rules
evaluation, because GitHub provides no per-tag effective-rules endpoint (U0 AC7,
RR-3b). **(c)** The protection is **not** admin-proof (U0 AC9, RR-3a).

**Verified by** V10, V19, V24.

## Hardening coverage

| Hardening | Plan fold | Units | Verifications |
|---|---|---|---|
| D1 | `RESOLUTION_MARKER`; removal of generic/session-end/post-merge resolution | U1, U5 | V5, V22 |
| D2 | `RESOLUTION_MARKER` atomic publication; `RESOLUTION_MARKER_DISCOVERY` | U1, U2 | V9, V13 |
| D3 | `RESOLUTION_MARKER` payload (`C` excluded; tag header binds) | U1 | V7 |
| D4 | U4 review re-run and refresh rules | U4 | V16, V24 |
| D5 | U4 total order; `HEAD_EVIDENCE_RULE` | U1, U4 | V24, V5 |
| D6 | `RESOLUTION_MARKER_DISCOVERY` live status and provenance; amended last mile | U2, U4 | V14, V21, V24 |
| D7 | Fixed-point scan; per-marker validation | U2 | V11, V12, V15, V26 |
| D8 | "No merge authority" rule | U2, U5, U6, U7 | V14, V23 |
| D9′ | Ancestry rule; `PENDING` blocks | U2, U5, U7 | V15, V17, V20 |
| D11′ | Tag protection (survival) + `C`-ancestor-of-`H` (detection) | U0, U4 | V17, V13 |
| D12 | Stage carrier guard; Ship removal | U5, U6 | V23 |
| D14′ | Read-only discovery; `FETCH_HEAD` prohibition; explicit commit-ish | U2 | V25, V26 |
| D15′ | Tag-based discovery; PR body demoted to advisory | U1, U2 | V13 |
| D16′ | U0 ruleset; precondition proof; last-mile re-proof | U0, U1, U4 | V10, V7 |
| D17 | Atomic publication with two explicit leases | U1, U4 | V7, V8 |
| D18 | Tag-object identity; peel and type assertions | U1, U2 | V7, V11 |
| D19 | Idempotence-and-conflict proof; no inference from push silence | U1 | V9, V12 |
| D20 | Freeze; re-enumeration before commit; late-checkpoint halt | U1, U4 | V18, V6 |
| D21 | `C` ancestor of `H`, not `C == H`; refresh rules | U1, U4 | V16 |
| D22 | RQ-7 clarification at three points of use | U2, U3, U5, U7 | V21, V22 |
| D23 | Enumeration rules; zero omits exactly three things | U1, U4 | V5, V6, V26 |
| D24 | Bounded claims for expected-head, ruleset proof, admin residual | U0, U4 | V10, V19, V24 |

Every hardening folds into at least one unit **and** at least one verification.
No hardening is satisfied by wording alone.

---

**Everything below this line is immutable historical evidence.** It records the
revision-10 hardening set and the corrections that preceded it. **That normative
content is SUPERSEDED by revision 11** per the disposition table above, and no
fold, unit reference, or coverage row below may be cited as normative for
revision 11.

## Retained hardening history (revision 10 and earlier — SUPERSEDED)

### Superseded prior corrections — do not reimplement

| Prior ref | Superseded content | Why it must not be reimplemented | Replaced by |
|---|---|---|---|
| **H3, required-correction item 2** | *"If the merge does not complete in the same session after resolution, Ship MUST create a **new** checkpoint capturing post-resolution state before yielding."* | **SUPERSEDED — this is the defect, one level down.** A new Git-tracked checkpoint created after the last resolution must itself be resolved, and it can only be resolved by a further commit, which needs a further PR once the current one merges. The remedy is recursive and never terminates. Every checkpoint owned by the unit is now resolved **before** the final gates, and the residual window is covered by live state, not by another checkpoint. | Plan `RESOLUTION_PREFIX` invariant 1; plan `LAST_MILE_RECOVERY`; hardening D1 and D2 below. |
| **H3, required-correction item 3** | The new checkpoint's `resume_hint` must state that resolution already landed. | Superseded as a consequence of the above — there is no new checkpoint. The double-resolution concern it addressed is now handled by `LAST_MILE_RECOVERY`'s live-state classification, which verifies before acting. | Plan `LAST_MILE_RECOVERY` Step 1. |
| **H5** | Predicate drift control across *"five documents"*, mitigated by the T9 drift checker. | **SUPERSEDED AND WITHDRAWN ENTIRELY.** The count was also wrong — the section named four files, not five (PR #396 thread `PRRT_kwDORJEduc6h3aDx`). Both the predicate and its checker belong to Defect 1 and are out of scope. Nothing in revision 6 requires a drift checker, a fixture corpus, a parity runner, or a hook shim. | Not replaced. Out of scope per plan `## Out of scope`. |
| **H4** | Auto-routed resume must re-fetch live state before trusting stored SHAs. | Withdrawn **as a Defect-1 hardening** — there is no auto-routed resume in revision 6. Its *substance* is nonetheless correct and is preserved for the recovery path, where a resumed session likewise must not trust stored SHAs. | Hardening D6 below. |

### D1 (BLOCKING) — Resolution must not be recursive

**Risk.** Any mitigation that creates a further Git-tracked checkpoint to cover
the window after the last resolution reproduces the original defect one level
down, indefinitely.

**Required correction.** The plan must resolve **every** checkpoint owned by the
unit before the final HEAD-bound gates, so all resolutions ride the same merge,
and must cover the remaining window with **live state reconstruction**, never
with another checkpoint. The plan must say so explicitly, so the recursive remedy
cannot be reintroduced by a well-meaning implementer.

**Folds into**: plan `RESOLUTION_PREFIX` invariant 1; U4 acceptance criteria 2 and 6.

**Status**: applied.

### D2 (BLOCKING) — The residual window must be covered by something

**Risk.** Resolving all checkpoints before merge necessarily leaves a
checkpoint-free interval between the last resolution and verified closure. If
nothing covers it, a crash in that interval leaves an open PR carrying resolution
commits that no future session will ever look for — a silent, unrecoverable loss
of the obligation, which is *worse* than the defect being fixed.

**Required correction.** A durable, non-branch, non-commit locator must be
published **before** the first resolution commit, and a session finding zero
active checkpoints must look for it **before** concluding clean startup.

**Folds into**: plan `CLOSURE_LOCATOR` phase 1; plan `LAST_MILE_RECOVERY` entry
rule; U2 acceptance criterion 5; U6 acceptance criterion 1.

**Status**: applied.

### D3 (BLOCKING) — The locator must not be self-referential

**Risk.** A locator that records the resolution commit SHAs *inside a commit*
cannot be written: the final commit's SHA does not exist until that commit is
created, and creating a further commit to hold it advances HEAD again and
displaces the "last commit" property. A locator recorded only on the feature
branch is additionally invisible to the fresh default-branch checkout that must
perform recovery.

**Required correction.** The locator surface must be PR-body metadata — not a
branch, not a commit — and the plan must state outright that no commit is ever
required to record its own SHA.

**Folds into**: plan `CLOSURE_LOCATOR` surface rationale; U2 acceptance criteria
3 and 8.

**Status**: applied.

### D4 (BLOCKING) — Evidence must be produced at the final HEAD, not relabelled

**Risk.** The resolution commits advance HEAD past whatever the earlier review
examined. Moving only the recorded SHA forward re-points a verdict at a HEAD it
never saw. That is a false attestation, and it defeats P-014 §1.9 while appearing
to satisfy it.

**Required correction.** The actual local review must be **re-run** over the
final post-resolution diff, and the plan must forbid re-pointing in terms.
Additionally, the re-run must occur **after push**, so the reviewed state is the
state the remote actually holds.

**Folds into**: plan `RESOLUTION_PREFIX` invariants 2 and 4; U2 acceptance
criteria 4 and 5.

**Status**: applied.

### D5 (BLOCKING) — The readiness sequence must be satisfiable

**Risk.** P-014 §1.9 reads the PR body and requires `Reviewed HEAD ==
headRefOid`. If the gate runs before the body is updated, it is unsatisfiable by
construction once resolution commits have advanced HEAD — so the gate would
either be failed forever or quietly skipped. A quietly skipped merge gate is the
worse outcome.

**Required correction** *(rewritten in revision 9 against a verbatim extract of
the real `_ship.agent.md` Step 5, because every prior round specified this
against a prose description of Step 5 and mismatched it; the extract was
independently re-derived and confirmed accurate by two round-9 personas and is
unchanged in revision 10).* The order must be the named segments of
`RESOLUTION_PREFIX`:

1. **S1** — complete the existing CI/review fix loop (real items 7 *first* and
   7a). These may commit and push, and they run first.
2. **S2** — complete every remaining branch-mutating item: runtime verification
   (real item 7 *second* — the item number is duplicated in the live file),
   operational closure (item 8), follow-up stash writes (item 9), and the
   ordinary push (item 10).
3. **S3** — **prove** current-unit checkpoint enumeration complete.
4. **S3.5** *(conditional, nonzero only; added in revision 10)* — **prove the
   branch protection is live** for the actual `headRefName` read from the PR API
   before anything is published. API unavailable, uncovered branch, ambiguous
   rules, or any bypass capability **halts here**, before publication.
5. **S4** *(conditional, nonzero only)* — publish the phase-1 locator; resolve
   every checkpoint **and** write the `RESOLUTION_OBLIGATION_RECORD` at `OPEN`
   in one commit; push it; **prove remote head == local head**; publish the
   phase-2 locator.
6. **S5** *(every unit)* — at the resulting pushed HEAD: re-run the **actual**
   local review; publish the final locator resolution state if one exists;
   update the PR-body `Reviewed HEAD`; then run §1.9 (real item **7b**, **moved
   here**), an **explicit required-check evaluation**, and P-018 (real item
   **7c**, **moved here**).
7. **S6** — record approval with a pinned `approved_head` (real item 14).
8. **S7** — the **amended** last-mile re-check (real item 15) re-fetching
   `headRefOid`, the PR body, `reviewDecision`, review requests and reviews,
   **every review-thread page**, required checks, resolution-commit ancestry,
   **a re-enumeration of active checkpoints owned by this unit**, and — if the
   unit published under S3.5 — a **re-proof that the ruleset still applies**. A
   nonzero re-enumeration returns the unit to S3 and voids the S6 approval;
   ruleset drift halts.
9. **S8** — merge only when all **seven** merge-bar conditions hold, with the
   locator and ancestry terms **explicitly conditional** so a zero-checkpoint
   unit can satisfy the bar; the merge call **pins the observed head SHA**; then
   items 16 (P-009, retained and **extended** with API-side merge-commit mode
   and a two-parent assertion) and 17 (P-017) apply.

The plan must state *why* a PR-body write is safe inside this order: it is
metadata and does not advance `headRefOid`. It must additionally state the
refresh rules — **any** HEAD change voids the approval and forces a fresh
push/review/metadata/gates/approval cycle; a thread or check change without a
HEAD change forces the affected gates plus a refreshed approval; **no stale
approval is ever reused**.

**Withdrawn with this rewrite.** The revision-8 claim that the approval,
re-fetch and merge items "already exist in Ship Step 5 and are unchanged" was
**false** — real item 15 re-runs only the P-018 gate and re-queries
`headRefOid`, never evaluating required checks and never re-paginating review
threads. Any criterion requiring item 15 to remain unmodified is withdrawn with
it, and revision 10 re-swept the whole document set for restatements of that
claim. *(Round-9 finding F-02.)*

**Folds into**: plan `RESOLUTION_PREFIX` segments S1–S8 (including S3.5) and
invariants 3, 5, 6, 7, 9; `HEAD_EVIDENCE_RULE`; **U2 acceptance criterion 3**
*(revision 10 correction — AC3 is the criterion that installs the canonical
block verbatim; revision 9 cited AC6, which is the three-phase criterion and
does not govern the segment order. Round-9 finding F-32.)*; U4 acceptance
criteria 2, 5, 7, 8, 9, 10; verification V4 and V20.

**Status**: applied in revision 9; fold references and segment list corrected in
revision 10.

### D6 (BLOCKING) — Recovery must not trust stored state

**Risk.** A session entering recovery reads a locator written by an earlier
session. Acting on its `final_head` without re-checking live state repeats the
error the 139-S checkpoint `resume_hint` warns about in capitals: *"ALWAYS
RE-FETCH THE LIVE PR HEAD AND LIVE THREAD LIST BEFORE TRUSTING ANY STORED SHA IN
A CHECKPOINT OR MEMORY FILE"* — a warning written after that session
independently discovered two further unresolved threads that post-dated its own
checkpoint.

**Required correction.** `LAST_MILE_RECOVERY` must treat every locator value as
a hint, never as truth. **Ordering (revised in revision 7 to compose with D10):**
the Step 1a locator-*status* gate runs **first**; only once it has admitted a
complete `RESOLUTION_PUBLISHED` locator does Step 1b classify **live** PR state
before trusting `final_head` or any other stored SHA. Stating "classify live PR
state first" without that qualifier would re-create the catastrophic
`RESOLUTION_PENDING` mis-classification D10 exists to prevent, because an empty
`final_head` would reach the HEAD-comparison rows. Within Step 1b, the
`Open, HEAD ≠ locator final_head` row must exist and must require re-establishing
readiness at the live HEAD rather than trusting the recorded one.

**Folds into**: plan `LAST_MILE_RECOVERY` Step 1b rows 1–2; **U11** acceptance
criteria 4–5; U5 acceptance criterion 1.

**Status**: applied. Carries forward the substance of prior H4.

### D7 (BLOCKING) — Discovery must be exhaustive or fail closed

**Risk.** A bounded `gh pr list --limit N` silently omits older PRs. An omitted
non-`RECONCILED` locator means startup concludes "clean" and selects new work
while an obligation is outstanding — the exact failure the mechanism exists to
prevent, now with a false assurance attached. A status-filtered scan fails the
same way: 139-S's shipment was **archived** while its obligation was open.

**Required correction.** Pagination to exhaustion; incomplete enumeration is an
**error that halts**, never evidence of absence; shipment-status filtering is
explicitly prohibited with the 139-S case as the stated reason.

**Folds into**: plan `CLOSURE_LOCATOR` read protocol; U3 acceptance criteria 1,
4, 5, 6; U6 acceptance criteria 2–3.

**Status**: applied. Raised by PR #396 thread `PRRT_kwDORJEduc6h3sTT`.

### D8 (BLOCKING) — Recovery must not become an auto-merge path

**Risk.** `LAST_MILE_RECOVERY` reconstructs readiness evidence and is entered
automatically at startup. Without an explicit bar, a session could reconstruct
evidence and proceed to merge without any operator approval — converting a
recovery protocol into an unsupervised merge route, which is a P-014 breach
dressed as resilience.

**Required correction.** A four-part bar, all parts required, halt otherwise:
complete locator; approval **proven at the live HEAD** and not read from a
memory file, checkpoint, or the locator itself; full current-HEAD gate set; and
an explicit statement that resumption authority never implies merge,
admin-fallback, or destructive approval. The plan must state that this path
**cannot supply** the approval signal P-014 requires — only consume one that
already exists and is verified live.

**Folds into**: plan `LAST_MILE_RECOVERY` Step 2; **U11** acceptance criterion 7;
U6 acceptance criterion 7.

**Status**: applied.

### D9 (BLOCKING) — The obligation must survive a failed closure

**Risk.** Marking the locator `RECONCILED` at merge discharges the obligation
before post-merge closure has actually succeeded. If closure then fails, or the
session crashes between merge and closure, the terminal locator is skipped by
every future discovery pass and the outstanding closure work is lost silently.

**Required correction.** `RECONCILED` is set **only after closure is verified**.
The plan must state the failure mode so the transition is not "simplified" back
to merge-time.

**Folds into**: plan `CLOSURE_LOCATOR` phase 3; U2 acceptance criteria 9 and 12;
**U12** acceptance criteria 3–6; U5 acceptance criterion 7.

**Status**: applied. Raised by PR #396 thread `PRRT_kwDORJEduc6h3sTe`.

### D10 (BLOCKING) — A pending locator must never be mistaken for a published one

*Added in revision 7, from the independent revision-6 review (Correctness P1).*

**Risk.** A locator published at phase 1 carries `status: RESOLUTION_PENDING`
with **empty** `resolution_commits` and `final_head` — by design, because the
commits do not exist yet. Revision 6's recovery table classified only on live PR
state, so a crash in that window produced two separate catastrophic
mis-classifications:

* **Open PR.** An empty `final_head` compares unequal to the live HEAD, matching
  the "HEAD advanced" row, whose action is *"re-establish readiness … update the
  locator … then the merge-authority bar."* Following it marks a branch with
  **no resolution commits at all** as `RESOLUTION_PUBLISHED` and walks it to the
  merge bar. The checkpoints are still active and would merge unresolved.
* **Merged PR.** The merged row asserts that *each* resolution commit is an
  ancestor of `origin/main`. Over an **empty list** that assertion is **vacuously
  true**. Closure proceeds, the locator is marked `RECONCILED`, and the active
  checkpoints are orphaned on `main` permanently — with the locator now terminal,
  so no future discovery will ever find them again.

The second case is strictly worse than the defect being fixed: the original 139-S
orphan was at least detectable, whereas this one erases its own evidence.

**Required correction.** A **locator-status gate must run before any live-PR
classification**. `RESOLUTION_PENDING` + open → do not touch the locator, do not
approach the merge bar; verify no resolution commits exist and re-enter
`RESOLUTION_PREFIX` at the resolve step, or halt. `RESOLUTION_PENDING` + merged →
unrecoverable orphan, halt immediately, never assert ancestry, never mark
`RECONCILED`. An ancestry assertion over an empty commit list must be unreachable.

**Folds into**: plan `LAST_MILE_RECOVERY` Step 1a; **U11** acceptance criterion
1; U5 acceptance criterion 4.

**Status**: applied.

### D11 (BLOCKING) — A rewritten branch must not silently drop the resolution commits

*Added in revision 7, from the independent revision-6 review (Correctness P1).*

**Risk.** Revision 6's `Open, HEAD ≠ final_head` row required re-establishing
readiness at the live HEAD but — unlike the `HEAD == final_head` row — omitted
the ancestry assertion entirely. A force-push, rebase, or branch reset after the
locator was published can drop the resolution commits from the new HEAD. Merging
that HEAD re-orphans exactly the commits the mechanism exists to protect, while
the locator still advertises them as published.

**Required correction.** The advanced-HEAD row must assert
`git merge-base --is-ancestor <sha> <the uniquely retained fetched ref>` for
**every** recorded resolution commit **before** re-establishing readiness, and
must halt to the operator on any failure. Re-establishing readiness first would
attach fresh evidence to a HEAD that has already lost the resolutions. *(Revision
10: the target is an explicit retained ref rather than `FETCH_HEAD`, which is
global and is silently rebound by the next candidate's fetch — round-9 finding
F-14.)*

**Folds into**: plan `LAST_MILE_RECOVERY` Step 1b advanced-HEAD row; **U11**
acceptance criterion 5; verification V22.

**Status**: applied.

### D12 (BLOCKING) — A universal policy must not have a procedural hole

*Added in revision 7, from the independent revision-6 review (Scope P1, Parity
P3).*

**Risk.** P-022 is agent-agnostic and names both `ship` and `stage`. Revision 6
rewired only Ship, leaving `_stage.agent.md` Session end item 2 with its
unconditional *"resolve any still-active checkpoints"* directive. An agent
following its own procedure would violate the policy that governs it, and
revision 6 recorded this only as a residual risk. A residual risk is a disclosure,
not a realization: the requirement RQ-1/RQ-2 claims to be universal would in fact
hold for one agent out of two.

**Required correction.** Either narrow P-022 to Ship, or close the gap. Closing
it is correct, because the defect is structural rather than Ship-specific — any
agent committing a resolution to a branch whose PR has merged produces the same
orphan. The Stage-side correction must be **narrow**: Stage holds no merge
authority (P-010) and therefore must **not** be given `RESOLUTION_PREFIX`. It
receives only the prohibition — never resolve once the carrying staging PR has
merged — plus the safe action in that case: leave the checkpoint active, surface
it, hand off to the operator.

**Folds into**: plan unit **U8** (all seven criteria); plan `## Constitution Check` P-010 row; residual
risk RR-2 closed.

**Status**: applied.

### D13 (BLOCKING) — A masked obligation is an undischarged obligation

*Added in revision 8, from the independent revision-7 review (Parity P1).*

**Risk.** Revision 7 wired Orchestrator locator discovery into the **global**
zero-candidate arm only — "no checkpoints at all". But the Orchestrator
explicitly permits planning overlap, so a **Stage-owned** checkpoint can be
legitimately active while Ship's last-mile obligation is outstanding. In that
state the Orchestrator sees "a checkpoint exists", skips discovery entirely, and
never routes Ship. Sequential Step 2 then skips the shipment because it is still
`active`. The result is a silent deadlock in which **no agent discharges the
obligation**, reached through the ordinary happy path rather than a crash. Direct
`ship next` would recover; `run pipeline` through the Orchestrator would not —
an agent-native parity defect.

**Required correction.** Orchestrator discovery must be scoped exactly as Ship's
is: keyed on the absence of a **`ship`-owned** active checkpoint, not on global
emptiness. A discovered non-`RECONCILED` locator routes Ship for
`LAST_MILE_RECOVERY` **before** Stage routing and before queue selection. When a
`stage`-owned checkpoint is also present, both are surfaced and no new queue work
is auto-selected. "Any checkpoint exists" must never be read as "no locator
obligation".

**Folds into**: plan unit **U6** acceptance criteria 1–3; plan `## Constitution
Check` P-001 row; V11.

**Status**: applied.

### D14 (BLOCKING) — A recovery step an agent cannot perform is not a recovery

*Added in revision 8, from the independent revision-7 review (Parity P2).*

**Risk.** `LAST_MILE_RECOVERY` Step 1a can require re-entry at the resolve step,
which **commits**. Recovery is entered at session start, when the working tree is
normally on `main` and may be a fresh checkout. Revision 7 named the branch in
the locator but never instructed the agent to get onto it. The two available
readings were both wrong: commit on `main`, which P-010 forbids and which would
put resolution commits on the default branch outside any PR; or stay on `main`
and be unable to proceed, leaving the obligation permanently undischarged while
appearing to have a defined recovery path.

**Required correction.** Before any re-entry that commits, the agent MUST read
`branch` and `pr` from the locator, `git fetch origin refs/pull/<pr>/head`, check
out a local branch at that tip, and confirm the checkout succeeded — halting to
the operator if any step fails, rather than proceeding on the wrong branch.
Read-only ancestry assertions require the fetch but not the checkout, and must
not be burdened with one.

**Folds into**: plan `LAST_MILE_RECOVERY` → **`Working-tree placement —
committing re-entry only`** (WP-0 … WP-7); **U11 acceptance criteria 9–12**
(defines the named procedure, the worktree-topology gate, the argv-array rule and
the cleanup rule); **U5 acceptance criterion 9** (invokes it **by that exact
name, without restating its steps**, before the only committing recovery branch);
**verification V12** with cases **V12a** (positive), **V12b** (dirty tree),
**V12c** (fetch failure), **V12d** (divergence / non-fast-forward), **V12e**
(branch, HEAD or detached-HEAD mismatch), **V12f** (read-only path performs fetch
but no switch) and **V12g** (topology gate).

**Status**: **applied in revision 9; fold references updated in revision 10** for
the U3→U11 split, the new WP-0 topology gate, and the change from *restate* to
*invoke by name* in U5 (round-9 findings F-09, F-21, F-23, F-28).

**Revision-9 discharge — all three prerequisites met.** Revision 8 required
three things before this could be marked applied; each is now done, **using the
existing units, with no new unit created**:

1. **U11 acceptance criterion 9** defines the named procedure in full — the
   topology gate, the clean-tree precondition, the trusted-source read, the
   fetch to a unique retained ref, the safe create/fast-forward-only switch, the
   current-branch, fetched-tip, not-main and not-detached assertions, the
   fail-closed halt, and the prohibition on `git reset`, force checkout,
   resolution or commit after any failure — and **U5 acceptance criterion 9**
   invokes it **by that exact name** as a precondition of any committing
   re-entry. *(Revision 10 changed U5 from restating the steps to invoking them
   by name: duplicating an eight-step procedure across two files is the drift
   vector round-9 findings F-08 and F-23 identify.)*
2. **V12** and its **seven** cases inspect for that sequence and its halt clause,
   and exercise the positive and fail-closed paths.
3. This fold reference is repointed at **U11 AC9–AC12**, U5 AC9 and V12.
   *(Revision 9 pointed it at U3 AC12; the U3→U11 split moved the procedure, and
   revision 10 repoints it accordingly.)*

Two defects revision 8 did not name are also closed: the placement paragraph had
**lost its heading**, so it was an unnamed fragment no acceptance criterion could
cite — it now carries the exact canonical name — and the read-only/committing
distinction is now an explicit criterion (V12f) rather than an aside, so
read-only ancestry assertions are not burdened with a switch.

**Retained revision-8 status (historical, not rewritten):**

> **NOT applied — OPEN P1 (Round 8, finding 3).** The previous "applied" claim,
> and its `U3 AC7 / U5 AC2` fold reference, were **incorrect** and are withdrawn.
> U5 AC2 names only `gh pr view`, `git fetch` and `git merge-base`: it requires
> **no** checkout of the PR branch and **no** verification that the checkout
> succeeded, and **no** V-check covers the placement. Because task-card
> acceptance criteria are declared exact, U5 could pass in full while recovery is
> still sitting on `main` — re-opening precisely the hazard D14 exists to close.

### D15 (BLOCKING) — The obligation record must survive deletion of the PR body

*Added in revision 9, from the P-013.6 escalation (finding D, RR-3 / RQ-7).*

> **STATUS CORRECTED IN REVISION 10.** The required correction stated below is
> correct and stands unchanged as the specification. Revision 9's `Status:
> applied` line was **wrong** and is withdrawn: the *mechanism* revision 9 folded
> it into did **not** satisfy it. Three round-9 reviewers found that (a) Channel
> B's candidate set was drawn from PRs whose bodies carry the locator marker, so
> it was **not independent of the mutable PR body** in the residual window it
> exists to cover (F-13); (b) its discovery and deletion commands were **not
> executable as written** — no commit-ish, an overwritten `FETCH_HEAD`, and no
> procedure binding history transitions to a record identity (F-14); and (c) the
> accepted-residual claim that force-push plus history-rewrite plus body-edit is
> "detectable, halting (OB-2)" was **unsubstantiated**, because RQ-8's no-SHA
> rule left no external reference point to distinguish "never existed" from
> "erased", and the attack needed only the PR author's own ordinary push and
> body-edit rights (F-19).
>
> **D15 is discharged in revision 10 by D16 plus the rebuilt Channel B**, under
> the operator's RR-3 decision. (a) is closed by deriving Channel B's candidates
> from the exhaustive trusted-PR enumeration with an explicit prohibition on any
> body-content filter (U13 AC2), tested against a **fully emptied** body (V17).
> (b) is closed by replacing every command with an explicit-revision-root form
> fetched to a **unique retained ref**, adding a shallow/partial-clone guard, and
> binding each transition to record identity and ancestry (U13 AC3, AC6), with
> V18 **executing** the workflow rather than inspecting strings. (c) is closed by
> **D16**: with `non_fast_forward` and `deletion` proven live before publication,
> the introducing commit stays reachable and the branch cannot vanish, so an
> ordinary later deletion commit is genuinely detectable (OB-1) instead of
> indistinguishable from absence. The residual narrows from *any collaborator's
> ordinary push rights* to *repository-admin privilege*, and that narrowed
> residual is stated honestly in plan RR-3 rather than claimed away.

**Risk.** `RESOLUTION_PREFIX` resolves **every** checkpoint before merge. That is
the fix — and it is also what makes the PR-body `CLOSURE_LOCATOR` the **sole**
record of an outstanding closure obligation. A PR body is mutable: a human or bot
can delete it, and nothing in revisions 6–8 would notice. Startup would then find
zero active checkpoints **and** zero locators, conclude "clean", and select new
queue work. This is **strictly worse than the defect being fixed** — the
pre-change behaviour left a still-active checkpoint behind, which remained
discoverable — and it directly contradicts RQ-7. The halt-on-incomplete rule does
not mitigate it: a *damaged* locator halts, but a *deleted* one is
indistinguishable from one that never existed.

**Required correction.** A second, **Git-tracked, history-immutable** obligation
record, introduced by the **resolution commit itself** and discoverable without
reading the PR body. It must:

* live on an **existing owned** repo-local state surface — the
  `operational-closure` pre-merge artifact under `docs/closure/` — never a new
  unowned tracker;
* carry a canonical schema, path and identity, and a **four**-transition
  lifecycle (`none` → `OPEN` → `CLOSED`, plus `none` → `none` as the legitimate
  zero-checkpoint terminal state);
* ride the **same commit** as the checkpoint resolutions and be pushed before the
  S5 review re-run;
* carry **no SHA**, so RQ-8 is preserved rather than traded — it is written
  inside the commit it would otherwise have to name;
* be discovered exhaustively over trusted PR/commit/tree history from a candidate
  set derived **independently of PR-body content**, with provenance taken
  **only** from API response fields and Git ancestry (PV-1…PV-3, PV-8, PV-B4,
  PV-B6, PV-6, PV-7) and **never** from PR-body text;
* detect **deletion via commit history** (`git log --diff-filter=D`, `git log
  -S'resolution_obligation'`, each with an **explicit revision root** against a
  **uniquely retained** fetched ref) rather than treating absence from the tree
  as absence of obligation;
* **fail closed** on deletion (OB-1), force-push/rebase/shallow-history gaps
  (OB-2), conflicting records (OB-3), multi-shipment records (OB-4), unparseable
  records (OB-5), Channel A/Channel B disagreement (OB-6, with the mutable
  channel never preferred), incomplete enumeration (OB-7), and **ruleset drift or
  bypass (OB-8)**;
* be discharged **only** by an `OPEN` → `CLOSED` transition in a **later** commit
  whose ancestry stays auditable and which is itself **merged** — deletion is
  never a discharge;
* introduce **no** executable persistence substrate, **no** locking or
  compare-and-swap, **no** cross-run cursor persistence, and **no** Defect-1
  construct, and confer **no** merge authority.

**Folds into**: plan `RESOLUTION_OBLIGATION_RECORD`; decision RQ-12; plan units
**U9** (schema and lifecycle), **U13** (body-independent discovery and
reconciliation) and **U10** (schema ownership and create-only initialization);
U4 AC12; U12 AC3–AC6; U5 AC6–AC8; U6 AC1, AC3, AC5; verification **V17**,
**V18**, **V19**, **V22**, **V23**; residual risk RR-3, closed **jointly with
D16**.

**Status**: **NOT discharged by revision 9** (see the status note above);
**discharged in revision 10** by the rebuilt Channel B (U13) together with D16.

### D16 (BLOCKING) — The durable record's detectability must rest on a proven protection, not an assumption

*Added in revision 10, implementing the operator's RR-3 decision.*

**Risk.** D15's Git-tracked record is only more durable than the PR body if the
commit that introduced it stays **reachable** and the branch carrying it stays
**present**. Neither holds by default. Measured read-only at revision 10: the
repository's single ruleset `PR-Required` (id `12812291`) is `active` but targets
`~DEFAULT_BRANCH` only; `GET /repos/{owner}/{repo}/rules/branches/main` returns
`["deletion","non_fast_forward","pull_request","copilot_code_review"]`, while the
same endpoint against this plan's own Ship source branch returns `[]`. Ship
source branches are therefore **unprotected today**. Without protection, a
force-push rewrites the introducing commit out of history and a branch deletion
removes the scan target entirely — and because RQ-8 forbids the record from
carrying any SHA, nothing distinguishes "obligation erased" from "obligation
never existed". This is exactly finding F-19, and it needs only the branch
author's ordinary rights.

**Required correction.** Two parts, and both are required — the second is what
makes the first *load-bearing at run time* rather than an assumption baked in at
plan time.

**Part 1 — the prerequisite (an external settings change, `ProposedAction`).** A
repository ruleset must cover **every permitted resolution-bearing Ship branch
pattern**, with:

* both the `deletion` and `non_fast_forward` rules;
* `enforcement: active`;
* **no bypass actors**, and `current_user_can_bypass: never`;
* branch patterns **derived from the installed Ship agent's own naming**, not
  guessed — `feat/**`, `chore/**` and `post-merge/**` as read from
  `.github/agents/_ship.agent.md`. `post-merge/**` is **not optional**: POST-B's
  `CLOSED` transition commits there, so an unprotected `post-merge/**` leaves the
  discharge itself erasable.

Because this mutates GitHub settings rather than repository files, it is carried
by a **distinct unit U0** classified `ProposedAction` / `ActionRisk: high` /
`approval_required: true`. **The planning pull request must not apply it**; U0
requires explicit operator or admin approval at implementation time, and the plan
must say so in the unit itself rather than only in a risk table.

**Part 2 — the run-time proof (`BRANCH_PROTECTION_PREREQUISITE`, segment
S3.5).** Before publishing any obligation, Ship must query the effective-rules
API for the **actual live `headRefName` read from the PR API** — never the
ambient checked-out branch, which is the same class of defect as F-11 — and prove
all four requirements hold. The branch name must be **URL-encoded** (a
`chore/...` ref contains a path separator). The canonical endpoint is
`GET /repos/{owner}/{repo}/rules/branches/{branch}`, with the
repository-supported equivalent permitted where that endpoint is unavailable.

**Every negative outcome halts *before* obligation publication**: the API is
unavailable or errors; the branch is not covered; the returned rules are
ambiguous or do not include both required rules; enforcement is not active; or
any bypass actor or current-user bypass capability exists. The protocol must not
proceed optimistically on an ambiguous answer, and must not treat an empty
response as "no restrictions, therefore fine" — an empty response is exactly the
uncovered case.

**Re-proof and drift.** The proof is not a one-time gate. The amended item 15
(S7) **re-proves** it before merge, and Channel B **re-proves it per candidate**
at B2 before scanning that candidate's history. Drift between proof and re-proof
— protection removed, enforcement disabled, or a bypass appearing — halts as
**OB-8**.

**What this buys, stated honestly.** With `non_fast_forward` proven live, the
introducing commit cannot be rewritten out of reach; with `deletion` proven live,
the branch cannot vanish. An ordinary later commit that deletes the record is
therefore still **detectable** by B5's deletion probe, which is the precise
property F-19 said was missing. It does **not** make the record unerasable by a
repository **admin**, who can edit or disable the ruleset. That residual is
narrower than the one it replaces — admin privilege rather than any
collaborator's ordinary push rights — is **detected on re-check** rather than
silently absorbed, and is recorded as such in plan RR-3.

**Folds into**: plan `BRANCH_PROTECTION_PREREQUISITE`; `RESOLUTION_PREFIX`
segment **S3.5** and invariant 9; requirement **RQ-13**; plan units **U0** (the
prerequisite itself), **U2** (AC11, installs the canonical text), **U4** (AC5,
invokes S3.5 before S4; AC7, re-proves at item 15) and **U13** (AC3 B2, per
candidate; AC8 OB-8); verification **V21**; residual risk **RR-3**, closed
jointly with D15.

**Status**: applied in revision 10. **Not yet independently reviewed.**

### Hardening coverage

| Hardening | Plan section | Task |
|---|---|---|
| D1 | `RESOLUTION_PREFIX` inv. 1 + residual-window prohibition | U4, U12 |
| D2 | `CLOSURE_LOCATOR` ph. 1; `LAST_MILE_RECOVERY` entry | U2, U11, U5, U6 |
| D3 | `CLOSURE_LOCATOR` surface | U2 |
| D4 | `RESOLUTION_PREFIX` inv. 2, 4 | U2, U4 |
| D5 | `RESOLUTION_PREFIX` segments S1–S8 (incl. S3.5); inv. 3, 5, 6, 7, 9; `HEAD_EVIDENCE_RULE` | U2 (AC3), U4 (AC2, AC5, AC7, AC8, AC9) — verified by V4, V20 |
| D6 | `LAST_MILE_RECOVERY` Step 1b (after Step 1a) | U11, U5 |
| D7 | `CLOSURE_LOCATOR` read protocol | U3, U6 |
| D8 | `LAST_MILE_RECOVERY` Step 2 | U11, U6 |
| D9 | `CLOSURE_LOCATOR` ph. 3 | U2, U12, U5 |
| D10 | `LAST_MILE_RECOVERY` Step 1a | U11, U5 |
| D11 | `LAST_MILE_RECOVERY` Step 1b advanced-HEAD row | U11 |
| D12 | Unit U8 | U8 — verified by V10 |
| D13 | Unit U6 scoping; P-001 row | U6 |
| D14 | `LAST_MILE_RECOVERY` → `Working-tree placement — committing re-entry only` (WP-0…WP-7) | **U11 (AC9–AC12), U5 (AC9, by-name invocation only)** — verified by V12/V12a–V12g |
| D15 | `RESOLUTION_OBLIGATION_RECORD`; decision RQ-12 | **U9 (schema/lifecycle), U13 (discovery), U10 (ownership)**; U4 (AC12), U12 (AC3–AC6), U5 (AC6–AC8), U6 (AC1, AC3, AC5) — verified by V17, V18, V19, V22, V23 |
| D16 | `BRANCH_PROTECTION_PREREQUISITE`; S3.5; inv. 9; RQ-13 | **U0 (all criteria)**, U2 (AC11), U4 (AC5, AC7), U13 (AC3 B2, AC8) — verified by V21 |

**Coverage is complete as of revision 10.** Every blocking hardening above folds
into at least one task acceptance criterion **and** at least one verification
check. No hardening remains narrative-only, and every unit named in the table
exists in the fourteen-unit structure the U3/U4/U9 splits produced.

D14 was the single documented exception at revision 8 and is discharged through
existing units — **U11** now defines the named procedure (it moved there in the
U3 split), U5 **invokes it by name without restating it**, and V12 verifies both
halves. **D15** was **not** discharged at revision 9, contrary to that revision's
own status line; revision 10 discharges it jointly with the new **D16**.

**This coverage claim is a statement about fold completeness, not a verdict.**
It asserts that each hardening has a named home in a task criterion and a
verification check. Whether those criteria are *adequate* is what the independent
review decides. The round-9 review answered **no** for revision 9. Revision 10's
review has **not yet returned**, so nothing in this table may be read as
validated, and `harvest_authorized` remains **false**.
