---
doc_type: exec-plan
date: 2026-09-13
revision: 11
scope: defect-2-only
status: review-failed
review_verdict: FAIL
review_verdict_revision: 11
review_attempts: 6
review_p0_count: 0
review_p1_count: 21
review_circuit: OPEN
escalation: P-013.6 fired at revision 8 (route gpt-5.6-sol/openai/xhigh). Rounds 6-10 all returned FAIL. Revision 11 is the THIRD and final bounded remediation revision under the operator's authorized outcome cycle for PR #396, carrying the architecture-validated protected immutable annotated-tag design that replaces revision 10's branch-history/closure-record machinery, plus ONE fresh full independent review. Round 11 returned 0 P0 and 21 P1 from a seven-persona, six-model panel. The circuit is now OPEN. No fourth remediation attempt is authorized by this cycle; disposition of PR #396 returns to the operator.
harvest_authorized: false
source_document: docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md
supersedes: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md
supersedes_normative_design_of: revision 10 of this document
stash_ids: [4EF24729]
policies: [P-001, P-003, P-005, P-006, P-008, P-009, P-010, P-011, P-012, P-014, P-015, P-016, P-017, P-018, P-019, P-020, P-022]
requires_plan_hardening: yes
hardening_document: docs/exec-plans/2026-09-13-checkpoint-resolution-durability-hardening.md
task_count: 8
dependency_edge_count: 11
sub_epic_count: 3
contains_proposed_action: true
proposed_action_units: [U0]
---

# Checkpoint Resolution Durability — Implementation Plan (revision 11, Defect 2 only)

**Source document**: `docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md`
**Requires plan hardening**: **yes** — this plan changes merge-adjacent ordering
governed by P-014/P-018, adds a startup route that could, if mis-specified,
become an unsupervised merge path, and carries one **`ProposedAction`** unit
(U0) that mutates GitHub repository settings.

**Machine-readable status.** The frontmatter is authoritative. `revision: 11`,
`scope: defect-2-only`, `harvest_authorized: false`, `review_verdict: pending`,
`review_verdict_revision: 11`, `status: under-review`,
`contains_proposed_action: true`.

## Revision 11 — what this revision is

Revision 11 is the **third and final** bounded remediation revision in the
operator-authorized outcome cycle for PR #396. Revisions 9 and 10 both returned
**FAIL** from fresh independent cross-model panels (round 9: 1 P0 / 15 P1;
round 10: 4 P0 / 17 P1, unanimous across four reviewers). Round 10's panel did
not recommend a third in-place repair of the same machinery. It recommended
**replacing** the durability mechanism with a protected immutable marker ref.
The operator accepted that recommendation and authorized this revision.

Revision 11 therefore **deletes rather than repairs**. The whole
`RESOLUTION_OBLIGATION_RECORD` apparatus — its `OPEN`/`CLOSED` lifecycle,
`POST-A`/`POST-B`, Channel A and Channel B, `B0`–`B7`, `PV-B*`, `OB-1`…`OB-8`,
the load-bearing PR-body `CLOSURE_LOCATOR` state machine, the
`operational-closure` schema change, the P-020 compaction exclusion, the broad
branch ruleset over `feat/**` `chore/**` `post-merge/**`, the post-merge branch
protection, and the history pickaxe / `FETCH_HEAD` / retained-PR-history
machinery — is **removed from the normative design**, together with the
oversized units and verifications that existed only to serve it.

What replaces it is one object: a **protected, immutable, annotated Git tag**,
published **atomically with the resolution commit**, whose status is derived
**solely from ancestry to the protected default branch**.

The prior review sections of this document are **retained unchanged as immutable
historical evidence** under `## Retained review history`. Every normative claim
they describe belongs to revision 10 or earlier and is **superseded**. No row,
finding, or definition below that heading may be cited as normative for revision
11.

### The four round-10 P0s, and how deletion answers each

| Round-10 P0 | Revision-11 disposition |
|---|---|
| **F-01** — U13 installed a different Channel B than the canonical block and reinstated the discharge-rejection rule | **Deleted.** There is no Channel B, no canonical/unit duplication, and no discharge transition to reject. The marker has exactly one canonical definition (U1 identity/publication, U2 discovery) and units reference it by name. |
| **F-02** — U13 AC6 halted on the protocol's own happy-path interval | **Deleted.** There is no `OPEN` record and no residual interval to misclassify. `C` being an ancestor of refreshed `origin/main` **is** the discharge condition, computed directly. |
| **F-03** — `FETCH_HEAD` versus retained-ref contract was unsatisfiable across four surfaces | **Deleted.** No PR-history fetch, no pickaxe, no `FETCH_HEAD`. Discovery fetches tag objects into unique `refs/autoharness/marker-scan/*` refs, and every ancestry test names an explicit commit-ish. One fetch form, one ancestry target, one surface. |
| **F-04** — U0's `deletion` + `non_fast_forward` over `feat/**`/`chore/**`/`post-merge/**` broke branch cleanup and rebase repository-wide | **Deleted.** U0 now targets **tags only**, at the exact path `refs/tags/resolution-obligation/**`. No working branch is covered. Rebase, amend, force-with-lease and branch cleanup are entirely unaffected. Blast radius is a namespace that exists only for this purpose. |

### Authorized RQ-7 clarification (recorded explicitly, per operator authorization)

The Orchestrator authorized one narrow clarification of RQ-7 on the basis of the
architecture review. It is recorded here verbatim because it changes what the
marker is allowed to mean:

* The immutable marker tracks **checkpoint-resolution delivery to the protected
  default branch only**. Its status derives **solely** from marker-target
  ancestry to `origin/main`.
* **Full P-001 release closure remains governed independently** by existing
  shipment state, the closure PR and its artifacts, knowledge graduation,
  runtime and operational closure, and P-020 compaction. **A discharged marker
  never authorizes the next shipment and never proves full release closure.**
* This clarification is **not** permission to weaken **P-001, P-009, P-014,
  P-018, P-020, or P-022**. Every one of those gates stands unchanged and is
  evaluated independently of marker state.

This is why revision 11 is smaller than revision 10 without being weaker. The
revision-10 design conflated two obligations into one record and then had to
build a lifecycle, two channels, and a compaction exclusion to keep them apart.
Separating them at the definition removes the need for all of it.

### U0's risk and approval boundary (recorded explicitly)

U0 is classified **`ProposedAction`**, **`ActionRisk: high`**,
**`approval_required: true`**. It is a **manual operator/admin step** under
**P-019**. It is **never agent-executed**, and it is **never** satisfied by a
dark-mode approval record: P-017's activation contract confers merge approval
within a recorded scope, not repository-settings-mutation authority, and U0 is
expressly excluded from any dark-mode pre-authorization. **This planning pull
request does not apply it.** Ship is **never** granted ruleset-write
credentials; Ship reads ruleset configuration and halts, it never repairs it.

## Primary objective

Make checkpoint resolution **ride the merge that carries the work**, and make the
fact that it did so **independently provable after the fact** from a
tamper-evident object that no PR body edit, branch deletion, or history rewrite
can erase — without introducing any executable persistence substrate, any
cross-run continuation semantics, or any workflow-breaking repository setting.

## Constraints

* **Documentation-only**, except U0, which changes one GitHub repository setting
  and touches no repository file.
* Every unit is a **single-file, single-domain** change achievable in under two
  hours. U0 touches no file and is bounded by a single ruleset creation.
* **No** Defect-1 construct is reintroduced. The continuation predicate, the
  activation record store, the drift checker, the hook shim, the fixture corpus
  and the parity gate remain out of scope.
* **No new executable substrate.** No script, no CI check, no workflow file, no
  external store, no lock, no compare-and-swap, no cursor.
* **No `143.*` identifier is a live target.** `143-F`, `143-S` and
  `143.001-T`…`143.014-T` are machine-state **abandoned** and appear in this
  document only as historical evidence. Replacement IDs stay unassigned until a
  later authorized harvest.
* **Stage holds no merge authority** (P-010) and executes no part of the Ship
  finalization order.

## Constitution Check

### Workflow policies

| Policy | Bearing on this plan |
|---|---|
| P-001 | Release closure stays governed by shipment state, the closure PR, knowledge graduation, runtime/operational closure and P-020. The marker is **not** a P-001 gate and never substitutes for one. U5's merge confirmation states this separation explicitly. |
| P-003 | Decision → plan → 1 release unit → 3 sub-epics → 8 tasks, each task naming its parent sub-epic and carrying ≥1 acceptance criterion. |
| P-005 | Every halt in this plan is recorded as a policy-violation event with its gate and action. |
| P-006 | `requires_plan_hardening: yes`; the hardening document is revision 11 and is reviewed with this plan. |
| P-009 | Merge-commit mode is preserved and **strengthened** (API-side check plus a two-parent assertion). Squash and rebase remain rejected. The repository already has `allow_squash_merge: false` and `allow_rebase_merge: false`. |
| P-010 | Stage's role boundary is unchanged. U6 adds a **guard**, not an authority: its only outcomes are "resolve in an explicit pre-merge staging finalization" or "halt". |
| P-011 / P-016 | No parallel worktree and no parallel branch is introduced. Discovery is read-only and creates only local refs under `refs/autoharness/marker-scan/`, never a worktree. |
| P-012 | Every tool this plan depends on is probed before the path that needs it; unavailability halts and is never read as an empty result. |
| P-014 | §1.9 ordering is preserved; the PR-body `Reviewed HEAD` record is **advisory metadata** written before the gate, and operator approval is pinned to a HEAD. |
| P-017 | The dark-mode fallback state machine is untouched. U0 is expressly **excluded** from dark-mode pre-authorization. |
| P-018 | The Copilot-review gate runs unchanged and is re-run at the last mile. Marker state never satisfies or bypasses it. |
| P-019 | U0 is the harness's recorded exception case: the harness does **not** edit rulesets. U0 is a `ProposedAction` executed by an operator or admin, and Ship is never given ruleset-write credentials. This plan cites P-019 explicitly rather than leaving the tension implicit (round-10 F-20). |
| P-020 | Compaction is untouched. Revision 10's compaction exclusion is **deleted** — the marker is not a compactable artifact, so there is nothing for compaction to move. |
| P-022 | Introduced by U3, scoped narrowly to checkpoint-resolution delivery and marker integrity (§U3). |

### Workspace constitution

This workspace's governing document set is `.github/policies/workflow-policies.md`
plus the installed instruction files. Revision 10 mapped an eleven-principle
constitution this workspace does not carry (round-10 F-19); that mapping is
**removed** rather than restated. The policy table above is the whole
constitution check.

## Canonical definitions

Revision 11 has **three** canonical definitions, each with exactly **one**
installed home. Every other surface references a definition **by name** and must
not restate its steps.

| Definition | Canonical installed home | Referencing surfaces |
|---|---|---|
| `HEAD_EVIDENCE_RULE` | `github-pr-automation.instructions.md` (U1) | `_ship.agent.md` (U4) |
| `RESOLUTION_MARKER` | `github-pr-automation.instructions.md` (U1) | P-022 in `workflow-policies.md` (U3); `_ship.agent.md` (U4, U5) |
| `RESOLUTION_MARKER_DISCOVERY` | `github-pr-automation.instructions.md` (U2, immediately after U1's subsection) | `_ship.agent.md` (U5); `_orchestrator.agent.md` (U7) |

**The finalization order itself is not a canonical definition in the
instructions file.** It lives **solely** in `_ship.agent.md` Step 5, where the
items actually are (U4). Revision 10 kept the order in a second file and needed
a standing drift check (V20) that had no implementation (round-10 F-16). One
home, no drift check needed.

### `HEAD_EVIDENCE_RULE`

Any artifact whose own commit advances HEAD must not restate a HEAD-pinned
verdict. HEAD-pinned evidence belongs in PR metadata (PR body `Reviewed HEAD`);
committed documents use point-in-time wording or defer to the PR body.

The PR-body `Reviewed HEAD` record is **advisory metadata for the §1.9 gate**.
It is **not** a durability mechanism and **no** recovery path reads it. Observed
on PR #395, threads `PRRT_kwDORJEduc6h2uOQ` and `PRRT_kwDORJEduc6h2viu`.

### `RESOLUTION_MARKER` *(RQ-2, RQ-3, RQ-7, RQ-8, RQ-12, RQ-13)*

#### Identity

Exactly **one** marker exists per **canonical primary key**:

```text
primary key = (immutable GitHub repository ID,
               committed workspace ID,
               shipment ID)
```

* **Immutable GitHub repository ID** — the numeric `id` from
  `GET /repos/{owner}/{repo}` (for this repository, `1150361017`). The numeric
  ID survives repository rename and transfer; `owner/name` does not. Owner and
  name are carried in the payload as **advisory** fields only and are never
  identity or validation inputs.
* **Committed workspace ID** — the workspace identifier recorded in the
  repository's own committed configuration, not an ambient environment value.
* **Shipment ID** — the shipment whose checkpoints the marker resolves.

The ref name is **deterministic**, derived from the primary key alone:

```text
refs/tags/resolution-obligation/v1-<primary-key-digest>

primary-key-digest =
  lowercase hex SHA-256 of the UTF-8 bytes of
    "autoharness/resolution-obligation/pk/v1\n"
    + <repo_id>   + "\n"
    + <workspace_id> + "\n"
    + <shipment_id>  + "\n"
```

The leading domain-separation string is part of the hashed input. Determinism is
required: discovery must be able to **compute** the expected ref for a primary
key without consulting any mutable surface.

#### Payload

The annotated tag's **message** is the payload: a single canonical UTF-8 JSON
object, **LF line endings, no BOM, object keys sorted lexicographically, no
insignificant whitespace**. Fields:

| Field | Content |
|---|---|
| `schema_version` | integer, `1` |
| `repo_id` | immutable numeric repository ID (identity input) |
| `workspace_id` | committed workspace ID (identity input) |
| `repo_owner`, `repo_name` | **advisory only**; never an identity or validation input |
| `shipment_id` | shipment ID (identity input) |
| `checkpoints` | array, **sorted by checkpoint ID, duplicates removed**, each element `{ "id", "path", "content_sha256" }` where `content_sha256` is the SHA-256 of the **resolved** checkpoint file's bytes **as they exist at target commit `C`** |
| `pr_number` | the original carrying PR number |
| `head_repo_id` | the immutable numeric ID of the PR's **head** repository (fork-safe) |
| `head_branch` | the PR's `headRefName` at publication |
| `base_repo_id`, `base_branch` | the default-base identity at publication |
| `digest_algorithm` | string, `sha256` |
| `digest` | lowercase hex SHA-256 over the **domain-separated canonical serialization of every field above except `digest` itself** |

The digest input is
`"autoharness/resolution-obligation/payload/v1\n" + <canonical JSON of the object with the `digest` key omitted>`.
**`digest` is excluded from its own input** — a self-including digest is
unsatisfiable.

**`C` is deliberately absent from the payload** *(RQ-8)*. No commit is ever
required to record its own SHA. The annotated tag **object header** supplies the
binding: `T`'s `object` field is `C`, written by Git at tag-creation time. The
reference is therefore **non-self-referential** — the payload does not name `C`,
and the object that names `C` is not the commit.

**Tag object identity.** The published tag **ref** resolves to the **annotated
tag object OID `T`**, not to `C`. `C` is obtained by **peeling exactly one
level** (`refs/tags/<tag>^{}`). Any protocol step that treats the tag ref's OID
as a commit is wrong and must halt. This was verified empirically against a real
remote: after an atomic publication, `git ls-remote` reported
`<T> refs/tags/<tag>` and `<C> refs/tags/<tag>^{}` as two distinct lines with
two distinct OIDs.

#### Idempotence and conflict

* **Identical canonical marker already present** for this primary key — the
  remote tag peels to the same `C` **and** its payload is byte-identical
  canonical JSON with a valid digest: this is **idempotent recovery**. It is the
  normal outcome of a **lost push response**. No second marker is created and
  publication is treated as already complete.
* **Competing marker** for the same primary key — the remote tag exists but
  peels to a different commit, or carries a different payload: this is a
  **conflict**. **Halt.** Never delete, never update, never re-point. The
  operator resolves it.

#### Preconditions, all proven before any checkpoint is resolved

1. **Prove atomic push capability.** The publication requires server-side atomic
   multi-ref update. Prove the remote accepts `--atomic` before relying on it. A
   remote that does not support atomic ref updates **halts**; there is no
   sequential fallback, because a sequential fallback is exactly the torn state
   the atomicity exists to prevent.
2. **Capture the exact remote branch OID `R`** for the PR head branch, read from
   the remote (`git ls-remote origin refs/heads/<head_branch>`), not from a
   possibly-stale remote-tracking ref.
3. **Capture canonical tag absence** — compute the expected ref from the primary
   key and prove the remote carries no such tag, or that it carries an identical
   canonical marker (the idempotent case above).
4. **Prove the ruleset.** Read the tag ruleset by its **recorded ruleset ID**
   and prove it is `enforcement: active`, `target: tag`, includes exactly
   `refs/tags/resolution-obligation/**`, has **no** matching exclusions, carries
   rule types prohibiting **tag updates** and **tag deletion**, has
   `bypass_actors: []`, and reports `current_user_can_bypass: "never"`. Any
   failure, ambiguity, or API unavailability **halts before publication**. See
   U0 for the exact configuration and for the honest note on tag-rule
   verification.

#### Finalization freeze and checkpoint enumeration

1. **Finish every ordinary branch mutation first.** CI-fix and shadow-review
   loops, runtime verification, operational-closure artifact generation,
   follow-up stash writes and the ordinary branch push all complete before the
   freeze.
2. **Enter the finalization freeze.** After the freeze, the only permitted
   branch mutation is the single resolution commit `C`, and later review
   remediation (see *After publication* below).
3. **Enumerate exhaustively, with no `status` or `agent` API prefilter.** Call
   `backlogit_list_checkpoints` with `consumer_id` **only**. A `status` or
   `agent` filter applied at the API call silently excludes parse-failure and
   schema-invalid records, which are commonly returned as quarantined summaries
   with empty `agent`/`status` — the exact records that must be seen.
4. **Inspect malformed and quarantined records FIRST**, over the full
   enumeration, before any partition. Any validation error, quarantine flag, or
   missing/malformed required field **halts**.
5. **Then select** validated, active, `agent == "ship"` checkpoints whose
   recorded context binds them to the **current shipment** and the **carrying
   PR**.
6. **Zero is a proven result, never a default.** A failed, malformed,
   quarantined, ambiguous or partially-enumerated result is **not zero** — it
   halts. A registry exposing no checkpoint operations at all is **not zero** —
   it halts (P-012).
7. **Prepare all resolutions** without committing them.
8. **Re-enumerate immediately before the commit.** If the selected set differs
   in any way from the set prepared in step 5, **preparation restarts from step
   3**. A changed set before publication never publishes.

#### Publication

Create **one** resolution commit `C` containing **every** prepared checkpoint
resolution, and **one** annotated tag object `T` whose message is the canonical
payload and whose target is `C`.

**Require `R` to be an ancestor of `C`.** If it is not, the remote branch moved
under the freeze; halt.

Publish branch and tag in a **single atomic push**, with fully-qualified
refspecs and explicit leases:

```bash
git push --atomic \
  --force-with-lease=refs/heads/<head_branch>:<R> \
  --force-with-lease=refs/tags/resolution-obligation/v1-<digest>: \
  origin \
  HEAD:refs/heads/<head_branch> \
  refs/tags/resolution-obligation/v1-<digest>:refs/tags/resolution-obligation/v1-<digest>
```

Two syntax points are load-bearing and were **empirically validated** against a
real remote before this revision was written:

* **`--force-with-lease=<ref>:` with an empty expected value asserts the ref
  does not exist.** This is the correct and supported form for a tag that must
  be absent. Validated: with the tag absent the push succeeded; with a
  **different** tag already present at that ref the push was rejected with
  `stale info`.
* **`--atomic` makes either-ref rejection leave both refs unchanged.**
  Validated in both directions: a stale branch lease produced
  `! [rejected] HEAD -> main (stale info)` **and**
  `! [rejected] <tag> (atomic push failed)` with the tag **not** created; a
  competing tag produced `! [rejected] <tag> (stale info)` **and**
  `! [rejected] HEAD -> main (atomic push failed)` with the branch **not**
  advanced. Exit code `1` in both cases, remote unchanged in both cases.

**One validated semantic must be recorded rather than assumed.** When the remote
already carries a tag at that ref whose OID is **identical** to the local `T`,
Git classifies the tag refspec as already up to date and does not attempt an
update, so the tag lease is not evaluated and the push proceeds for the branch
refspec alone. This is **exactly** the idempotent lost-push-response case and is
safe — but only because the preconditions above have already proven the remote
marker is byte-identical canonical. The protocol **must** perform that proof
before treating an existing tag as idempotent; it must never infer identity from
the push's silence.

**Unsupported `--atomic`, a failed lease, a rejection, or any ambiguity in the
push result must leave both refs unchanged and halt.** No partial retry, no
per-ref fallback, no re-push of one ref alone.

#### Post-publication verification

After a reported success, verify **from the remote**, not from local state:

1. `git ls-remote origin refs/heads/<head_branch>` equals `C`.
2. `git ls-remote origin refs/tags/<tag>` equals **`T`** — the annotated tag
   object OID, **not** `C`.
3. `git ls-remote origin refs/tags/<tag>^{}` equals `C` — the peeled target.
4. The object type at `T` is **`tag`** (`git cat-file -t`). A **lightweight**
   tag (type `commit`) is a protocol violation and halts: a lightweight tag
   carries no message and therefore no payload.
5. The ruleset still applies, re-read by recorded ruleset ID. Drift halts.

#### After publication

* **No second marker is ever created** for the same primary key, under any
  circumstance.
* **The resolved checkpoint set is frozen.** A checkpoint that appears late, or
  any ambiguity in a later enumeration, **halts**. It does **not** produce a
  second tag and does **not** amend the first.
* **Review-remediation commits may append above `C`.** The branch may legitimately
  advance after publication — a reviewer finding requires a fix. Such commits
  **must not** alter the resolved checkpoint set.
* **The last-mile requirement is `C` is an ancestor of the final head `H`, not
  `C == H`.** Requiring equality would make any post-publication review
  remediation unmergeable, which is why revision 10's equality form was wrong.

### `RESOLUTION_MARKER_DISCOVERY` *(RQ-7, RQ-9, RQ-10, RQ-11)*

**Discovery is independent of the PR body and of source-branch survival.** It
reads tags. An emptied PR body hides nothing; a deleted source branch hides
nothing; the marker is an independent ref in a protected namespace.

#### Fixed-point scan

1. `git ls-remote --tags --refs origin` — list remote tags. `--refs` suppresses
   `^{}` peel lines so the listing is unambiguous.
2. **Filter locally** by the exact prefix `refs/tags/resolution-obligation/`.
   The filter is applied **client-side** to the full listing; it is never
   expressed as a server-side glob, so a server-side pattern quirk cannot
   silently truncate the candidate set. Unrelated tags in the repository (for
   example `refs/tags/v0.3.0-rc.1`) are excluded by the prefix and must not be
   fetched.
3. Fetch each surviving candidate into a **unique** local ref under
   `refs/autoharness/marker-scan/<primary-key-digest>`. Never fetch into
   `FETCH_HEAD`, and never reuse a scan ref across candidates.
4. **Verify the listed OID equals the fetched OID** for every candidate. A
   mismatch means the ref moved mid-scan; halt.
5. **Repeat the scan** and require the set of `(name, OID)` pairs to be
   **stable**. Retries are bounded; an unstable set after the bound **halts**.
   This fixed point is what makes an otherwise non-atomic listing trustworthy.

#### Per-marker validation

For each candidate, all of the following must hold, or the candidate **halts**
the session:

1. The object type at the tag ref is **`tag`**. A lightweight tag halts.
2. The ref **peels to exactly one commit**. Zero or more than one level of
   peeling halts.
3. The message parses as the canonical payload schema; `schema_version` is
   known. An unknown schema version halts — it is never skipped, because an
   unreadable marker is indistinguishable from an unsatisfied obligation.
4. The recomputed digest equals the recorded `digest`, over the
   domain-separated canonical serialization with `digest` omitted. A bad digest
   halts.
5. `repo_id` and `workspace_id` match this repository and this workspace. A
   marker for a different identity halts rather than being ignored — a wrong
   identity in this namespace is evidence of a real problem.
6. Every checkpoint named in `checkpoints` is **resolved at `C`** and its file
   bytes at `C` hash to the recorded `content_sha256`. A changed or missing
   checkpoint halts.
7. **Duplicate and conflict rules are deterministic.** Two candidates with the
   same primary key are a conflict and halt. Two identical refs cannot occur, as
   the ref name is a function of the primary key.
8. A malformed payload, an incomplete scan, or any ambiguity halts.

#### Ancestry-derived status — the only status rule

```text
refresh origin/main   (fetch the protected default branch)

if  C  is an ancestor of  origin/main        →  DISCHARGED
else                                         →  PENDING
```

`git merge-base --is-ancestor <C> origin/main` is the whole computation. There
is no stored status, no lifecycle field, no transition, and nothing to
disagree with.

**`DISCHARGED` means exactly one thing: the checkpoint resolutions reached the
protected default branch.** It does **not** mean the release is closed, and it
does **not** authorize the next shipment. P-001 closure is evaluated
independently (RQ-7 clarification, §"Authorized RQ-7 clarification").

#### Pending-marker provenance

Only when a marker is `PENDING` does discovery need to find its carrying PR, and
only to route recovery — never to gain authority.

1. Query the commit→pulls association for `C` with **full pagination** and
   **bounded retries**.
2. Select the **unique** result that is provenance-valid: its number equals the
   payload's `pr_number`; its head repository's immutable numeric ID equals
   `head_repo_id`; its base repository ID and base branch equal `base_repo_id`
   and `base_branch`; and its `headRefName` equals the payload's `head_branch`.
   **Multiple raw results are acceptable** — a commit can legitimately be
   associated with several PRs — **provided exactly one is provenance-valid.**
3. Fetch the live head of that PR into a **unique** ref and require `C` to be an
   **ancestor** of it.
4. **Halt** on: a renamed or deleted head branch; a closed-unmerged PR; a
   missing PR; endpoint failure after the retry bound; zero provenance-valid
   results; or more than one provenance-valid result.

**Reaching this path confers no merge authority** *(RQ-10)*. It restores
readiness *evidence* and routes to a recovery owner. It never merges, never
approves, and never resolves a checkpoint by itself.

#### Ref hygiene

**Remote markers are never deleted and never updated** — by this protocol or by
any agent. Local `refs/autoharness/marker-scan/*` refs are scratch and may be
cleaned after the scan.

## Implementation units

Eight units. Each is a single-domain change achievable in under two hours. U0
touches no repository file; U1–U7 each touch exactly one file.

| Unit | Sub-epic | File | Size | Complexity |
|---|---|---|---|---|
| U0 | E1 | *(none — GitHub repository setting)* | S | medium |
| U1 | E1 | `.github/instructions/github-pr-automation.instructions.md` | M | high |
| U2 | E1 | `.github/instructions/github-pr-automation.instructions.md` | M | high |
| U3 | E1 | `.github/policies/workflow-policies.md` | S | low |
| U4 | E2 | `.github/agents/_ship.agent.md` | M | high |
| U5 | E2 | `.github/agents/_ship.agent.md` | M | medium |
| U6 | E3 | `.github/agents/_stage.agent.md` | S | medium |
| U7 | E3 | `.github/agents/_orchestrator.agent.md` | S | low |

**Sub-epics**: **E1 — Marker contract and policy** (U0, U1, U2, U3);
**E2 — Ship execution rewiring** (U4, U5); **E3 — Stage and Orchestrator
guards** (U6, U7).

**High-complexity de-risking (required, recorded).** Three units carry
`complexity: high` — U1, U2 and U4. The granularity gate requires that a
high-complexity unit be split *or* carry a named de-risking treatment. None is
split, because each is already the minimum coherent unit for its file and a
further split would leave a dangling forward reference inside one document.
Each therefore carries an explicit treatment:

* **U1** — the load-bearing Git semantics it encodes (absent-tag lease, atomic
  either-ref rejection, annotated-tag OID versus peeled commit, and the
  already-up-to-date lease-skip path) were **empirically validated against a
  real Git remote before this revision was written**, and the observed outputs
  are recorded in V7, V8 and V9. The unit installs a proven contract, not a
  hypothesis.
* **U2** — its uncertainty is concentrated in failure classification, which is
  discharged by V11, V12, V13, V14 and V15 enumerating **every** failure variant
  with its required outcome, so the unit is written against a closed case list
  rather than an open one.
* **U4** — its risk is order drift, which is discharged by the **verbatim Step 5
  extraction** reproduced in the unit itself: the implementer edits against a
  transcribed baseline rather than a described one, and V24 inspects the result
  top-to-bottom against that same baseline.

Each remains under two hours: the largest, U4, is a re-ordering of an existing
enumerated list in one file plus one inserted block.

### U0 — Configure the marker tag ruleset *(ops prerequisite; `ProposedAction`)*

**Depends on**: nothing (root).
**File**: none. This unit changes one **GitHub repository setting**.
**Classification**: **`ProposedAction`**, **`ActionRisk: high`**,
**`approval_required: true`**, **P-019 manual operator/admin step**.

**Acceptance criteria**

1. **Never agent-executed.** The unit is executed by an operator or repository
   administrator. It is **never** performed by Ship, Stage, or the Orchestrator,
   and it is **never** satisfied by a dark-mode approval record — P-017 confers
   merge approval within a recorded scope, not settings-mutation authority. The
   card states this prohibition in those terms and cites **P-019**.
2. **Exact configuration.** A repository ruleset with:
   * `target: "tag"` — **tags, not branches**. No branch ref is covered by this
     ruleset and no working branch is affected. Rebase, `--amend`,
     `--force-with-lease`, and branch cleanup on `feat/**`, `chore/**` and
     `post-merge/**` remain **fully permitted** (round-10 F-04).
   * `enforcement: "active"`.
   * `conditions.ref_name.include: ["refs/tags/resolution-obligation/**"]` —
     the **exact** include pattern, no broader.
   * `conditions.ref_name.exclude: []` — **no matching exclusions**. An
     exclusion that matched the marker namespace would silently void the rule.
   * **Rule types that prohibit tag UPDATES and tag DELETION.** For a tag
     ruleset these are the *restrict updates* and *restrict deletions* rule
     types (`update` and `deletion` in the REST payload).
   * `bypass_actors: []`.
   * The created ruleset reports `current_user_can_bypass: "never"`.
3. **`non_fast_forward` is NOT the load-bearing rule and must not be used as
   one.** The card states the reason explicitly: `non_fast_forward` is a
   *branch* force-push rule and, applied to a tag, would only reject
   non-fast-forward movement — a tag retarget that happens to be a
   fast-forward along the same lineage would still be permitted, which is
   precisely the mutation the marker's immutability must forbid. **`update`** is
   the rule that restricts *all* updates to a matching ref, so it is the
   load-bearing one. Reusing the branch rule here would be a silent
   under-protection.
4. **Creation is NOT restricted.** The `creation` rule type is **absent**. Ship
   must be able to create new markers through the ordinary push path with **no
   bypass**. A ruleset that restricted creation would force Ship to hold bypass
   rights, which AC5 forbids.
5. **Ship is never granted ruleset-write credentials.** Ship *reads* ruleset
   configuration to prove the precondition and *halts* when it fails. Ship never
   creates, edits, repairs, or deletes a ruleset.
6. **Verification, immediately before and immediately after publication.** The
   recorded **ruleset ID** and its full configuration are read back via
   `GET /repos/{owner}/{repo}/rulesets/{id}` and asserted against AC2 field by
   field. The card records the ID at apply time so every later proof reads the
   same object by ID rather than re-discovering it by name.
7. **Honest verification note.** GitHub exposes an effective-rules endpoint for
   **branches** (`GET /repos/{owner}/{repo}/rules/branches/{branch}`); this plan
   makes **no** claim that an equivalent per-tag effective-rules endpoint
   exists. Marker-ruleset proof is therefore a **configuration read by recorded
   ruleset ID**, asserted field by field, and the card says so plainly rather
   than implying a stronger, effective-rules-style guarantee. The operator
   verifies the applied ruleset in the GitHub UI at apply time as a second
   witness.
8. **Rollback is defined and non-self-trapping** (round-10 F-29): the rollback
   is deletion of this ruleset by ID. Because no protocol step *requires* the
   ruleset to be absent, and because the precondition proof **halts** rather
   than auto-repairs, rolling back disables further marker publication and does
   **not** trap any agent in an unrecoverable state. Existing markers remain
   valid objects; they simply lose forward tamper-protection, which the card
   records as the accepted consequence.
9. **Residual, stated honestly.** A repository **administrator** can edit or
   delete this ruleset. The guarantee is exactly as strong as the ruleset. This
   is a narrower exposure than the any-collaborator exposure it replaces, and
   it is detected at the next precondition proof rather than silently absorbed.
10. **Definition of done requires U0 applied AND verified.** U0 **cannot be
    deferred while the feature is called complete**. **The current planning pull
    request does not apply it.**

**Posture**: ops prerequisite. **Size**: S. **Complexity**: medium.

### U1 — Record the immutable marker contract *(root)*

**Depends on**: nothing (root).
**File**: `.github/instructions/github-pr-automation.instructions.md` — one new
subsection.

**Acceptance criteria**

1. Installs `HEAD_EVIDENCE_RULE` verbatim from this plan, **including** the
   statement that the PR-body `Reviewed HEAD` record is advisory metadata for
   the §1.9 gate and is **not** a durability mechanism and is read by **no**
   recovery path.
2. Installs the `RESOLUTION_MARKER` **identity** rules verbatim: the three-part
   primary key, the immutable numeric repository ID (with the rename/transfer
   rationale), the committed workspace ID, and the deterministic ref name with
   its domain-separated digest construction.
3. Installs the **payload** schema verbatim, including: canonical UTF-8 JSON
   with sorted keys and LF endings; every field in the table; sorted, deduplicated
   checkpoint entries carrying each resolved checkpoint's **content hash at
   `C`**; the original PR number, **immutable head repository ID**, head branch
   and default-base identity; `digest_algorithm`; and the rule that **`digest`
   is excluded from its own input**.
4. States that **`C` is not in the payload** and that the annotated tag object
   header supplies the non-self-referential binding to `C` (RQ-8).
5. States the **tag-object identity rule**: the tag ref's OID is the annotated
   tag object **`T`**, not `C`; `C` is obtained by peeling exactly one level;
   treating the ref OID as a commit is a protocol error that halts.
6. Installs the **idempotence and conflict** rules: an identical canonical
   marker is idempotent recovery; a competing marker for the same primary key is
   a conflict that **halts** and is never deleted, updated, or re-pointed.
7. Installs the **four preconditions** — prove atomic push capability; capture
   the exact remote branch OID `R` read from the remote; capture canonical tag
   absence (or prove byte-identical canonical presence); prove the ruleset by
   **recorded ruleset ID** against U0 AC2 field by field — and states that each
   is proven **before any checkpoint is resolved**.
8. Installs the **finalization freeze and enumeration** rules: ordinary
   mutations finish first; freeze; enumerate with `consumer_id` **only** and
   **no `status`/`agent` API prefilter**, with the quarantine rationale;
   **inspect malformed/quarantined records first, over the full enumeration,
   before any partition**; then select validated active Ship checkpoints for the
   current shipment and carrying PR; **zero is a proven result, never a
   default**, and a failed/malformed/quarantined/ambiguous enumeration or an
   operation-less registry **halts** (P-012); prepare all resolutions;
   **re-enumerate immediately before the commit** and **restart preparation** if
   the set changed.
9. Installs the **publication** rules: one commit `C` carrying every prepared
   resolution; one annotated tag object `T`; **`R` must be an ancestor of `C`**;
   and the **exact** atomic push command with fully-qualified refspecs and both
   explicit leases, in the form given in this plan.
10. States the two **validated syntax semantics**: `--force-with-lease=<ref>:`
    with an empty expected value asserts the ref **does not exist**, and
    `--atomic` makes either-ref rejection leave **both** refs unchanged. The
    text records that both were validated empirically against a real remote in
    both rejection directions.
11. States the **identical-tag semantic** explicitly: when the remote tag OID
    already equals `T`, Git treats the tag refspec as up to date and does not
    evaluate its lease, so the push proceeds for the branch alone; this is the
    idempotent lost-push-response case and is safe **only because** the
    precondition proof already established byte-identical canonical presence.
    The text forbids inferring identity from the push's silence.
12. States that **unsupported `--atomic`, a failed lease, a rejection, or any
    ambiguity must leave both refs unchanged and halt** — no partial retry, no
    per-ref fallback, no single-ref re-push.
13. Installs the **five post-publication verifications** read **from the
    remote**: branch `== C`; tag ref `== T`; peeled `^{}` `== C`; object type
    `tag` (a lightweight tag halts); ruleset still applies.
14. Installs the **after-publication** rules: no second marker ever; the
    resolved set is frozen and a late or ambiguous checkpoint **halts** without
    producing a second tag or amending the first; review-remediation commits may
    append above `C` but must not alter the resolved set; and the last-mile
    requirement is **`C` is an ancestor of `H`, not `C == H`**, with the reason
    stated.
15. The subsection cites **P-019** where the ruleset precondition is described,
    recording that Ship proves and halts and never writes a ruleset.
16. markdownlint passes.

**Posture**: documentation-first. **Size**: M. **Complexity**: high.
*(Single file, one new subsection, no cross-file coordination. It is the largest
single-file unit in the plan and is deliberately the only place the marker
contract is written.)*

### U2 — Record marker discovery, provenance, and ancestry-derived status

**Depends on**: **U1** (same file — **strictly sequential**; U2's subsection is
placed immediately after U1's and references its definitions by name).
**File**: `.github/instructions/github-pr-automation.instructions.md` — one new
subsection, after U1's.

**Acceptance criteria**

1. Installs `RESOLUTION_MARKER_DISCOVERY` and states up front that discovery is
   **independent of the PR body and of source-branch survival**: an emptied PR
   body and a deleted source branch hide nothing.
2. Installs the **fixed-point scan** verbatim: `git ls-remote --tags --refs
   origin`; **client-side** exact-prefix filter on
   `refs/tags/resolution-obligation/` with the stated reason that a server-side
   glob could silently truncate; fetch each candidate into a **unique**
   `refs/autoharness/marker-scan/<primary-key-digest>` ref; verify listed OID
   equals fetched OID; repeat the scan and require a **stable `(name, OID)`
   set** within a bounded retry count or **halt**.
3. **Forbids `FETCH_HEAD`** as a fetch destination or assertion target anywhere
   in the protocol, and requires every ancestry, `cat-file`, `show` and `log`
   command to name an **explicit commit-ish** (round-10 F-03).
4. Installs the **eight per-marker validation rules**: object type `tag`;
   peel **exactly one** level; canonical schema parse with a known
   `schema_version`; digest recomputation; `repo_id`/`workspace_id` identity
   match; each named checkpoint **resolved at `C`** with matching
   `content_sha256`; deterministic duplicate/conflict rules; and a final
   catch-all. It states that **lightweight, unknown-schema, malformed,
   wrong-identity, bad-digest, changed-checkpoint and incomplete-scan candidates
   all HALT** — none is silently skipped, because an unreadable marker is
   indistinguishable from an unsatisfied obligation.
5. Installs the **ancestry-derived status rule as the only status rule**: after
   refreshing the protected default branch, `C` an ancestor of `origin/main`
   means **`DISCHARGED`**; otherwise **`PENDING`**. It states that there is no
   stored status, no lifecycle field, and no transition.
6. States the **RQ-7 clarification** at the point of use: `DISCHARGED` means
   **checkpoint-resolution delivery to the protected default branch only**. It
   does **not** prove P-001 release closure and does **not** authorize the next
   shipment; shipment state, the closure PR and artifacts, knowledge graduation,
   runtime/operational closure and P-020 are evaluated independently.
7. Installs the **pending-marker provenance** protocol: bounded-retry,
   **fully paginated** commit→pulls query; selection of the **unique**
   provenance-valid result matching `pr_number`, **immutable `head_repo_id`**,
   `base_repo_id`/`base_branch`, and the payload `head_branch`; **multiple raw
   results are acceptable provided exactly one is provenance-valid**; the live
   head fetched into a **unique** ref; and `C` required to be an **ancestor** of
   that live head.
8. States the **halt set** for provenance: renamed or deleted head branch;
   closed-unmerged PR; missing PR; endpoint failure after the retry bound; zero
   provenance-valid results; more than one provenance-valid result.
9. States **RQ-10** explicitly: reaching the recovery path **confers no merge
   authority**; it restores readiness evidence and routes to a recovery owner
   only.
10. Installs the **ref hygiene** rule: **remote markers are never deleted and
    never updated** by any agent; local `refs/autoharness/marker-scan/*` refs
    are scratch and may be cleaned.
11. markdownlint passes.

**Posture**: documentation-first. **Size**: M. **Complexity**: high.

### U3 — State the resolution-durability policy (P-022)

**Depends on**: **U1** (P-022 references `RESOLUTION_MARKER` by name; installing
the reference before the definition leaves it dangling).
**File**: `.github/policies/workflow-policies.md` — one new policy section.

**Acceptance criteria**

1. Adds **P-022**, scoped **narrowly** to **checkpoint-resolution delivery and
   marker integrity**. The statement covers exactly: no Git-tracked checkpoint
   resolution after its carrying PR merges; checkpoint resolution must ride the
   same merge as the work; and the marker's integrity rules (one marker per
   primary key, never deleted, never updated, never duplicated).
2. **References `RESOLUTION_MARKER` by name** with a file reference to
   `github-pr-automation.instructions.md`, and **does not restate** its
   identity, payload, precondition, publication or verification steps. A reader
   must follow the reference.
3. States that a **`PENDING` marker blocks and routes recovery**.
4. States that a **`DISCHARGED` marker is an append-only audit record** that
   **does not prove P-001 closure** and **does not authorize the next
   shipment**.
5. **Preserves all existing closure gates explicitly.** The section names
   P-001, P-009, P-014, P-018 and P-020 and states that none of them is
   weakened, satisfied, or bypassed by any marker state.
6. Uses ordinary prose verbs freely; the only prohibition is restating the
   canonical sequence.
7. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: low.

### U4 — Re-order Ship Step 5 and publish the marker

**Depends on**: **U0**, **U1**, **U3**.
**File**: `.github/agents/_ship.agent.md` — **Step 5 (PR Lifecycle)** only.

**The real Step 5 item list, extracted verbatim rather than described.** This
extraction was independently re-derived from the live file by two round-9
personas and **re-confirmed accurate by the round-10 panel**. It is preserved
**unchanged** in revision 11 and the task card carries it verbatim.

| Real item | Content | Mutates branch? |
|---|---|---|
| 1, 1a | full quality gates; `pipeline-topology` lifecycle gate | no |
| 2 | session memory summary to `docs/memory/` | yes (commit) |
| 3 | full local build | no |
| 4, 5, 5a | confirm readiness covers HEAD; prepare §1.9 body block; topology gate | no |
| 6 | `pr-lifecycle` — create/update the PR | no |
| **7** *(first)* | `fix-ci` loop | **yes — may commit and push** |
| 7a | optional shadow-review loop | **yes — may commit and push** |
| 7b | **P-014 §1.9 readiness gate** | no |
| 7c | **P-018 copilot-review gate** | no |
| **7** *(second — the item number is duplicated in the live file)* | `runtime-verification` | yes |
| 8 | `operational-closure` (writes `docs/closure/`) | yes |
| 9 | follow-up stash writes | yes |
| 10 | push the branch | yes |
| 11, 12, 13 | broadcast; present PR; branch retention | no |
| 14 | P-014 operator approval gate | no |
| 15 | last-mile re-check — **P-018 re-run + `headRefOid` re-query only** | no |
| 16 | P-009 merge-commit guardrail | no |
| 17 | P-017 dark-mode fallback state machine | no |

Two facts follow and are the substance of this unit. **Items 7b and 7c currently
run before items 7(second)/8/9/10**, so readiness is gated *before* four
mutating items and the push. And **item 15 re-fetches only the P-018 verdict and
`headRefOid`** — it never evaluates required checks and never re-paginates
review threads — so the RQ-6 bar has no executable enforcement path in the live
file.

**The total order this unit installs**, preserving the common zero/nonzero tail:

```text
finish ordinary mutations            items 7(first), 7a, 7(second), 8, 9, 10
  → finalization freeze + enumerate   (RESOLUTION_MARKER, by name)
  → create C and T                    CONDITIONAL on a proven nonzero count
  → atomic publish                    CONDITIONAL
  → verify                            CONDITIONAL
  → actual LOCAL REVIEW at current H  RUNS FOR EVERY UNIT
  → advisory PR-body Reviewed HEAD == H
  → §1.9  +  EXPLICIT required-check evaluation  +  P-018
  → operator approval bound to H
  → last-mile re-fetch (full set, below)
  → expected-head merge commit
  → fetch main; require two-parent merge and C ancestor
```

**Acceptance criteria**

1. Step 5 invokes **`RESOLUTION_MARKER`** *by name*, referencing
   `github-pr-automation.instructions.md` as its canonical definition, and does
   **not** restate its identity, payload, precondition, publication or
   verification steps.
2. **The reordered common tail is installed exactly as above**, with the
   old-to-new item mapping recorded against the verbatim extract so the reorder
   is auditable: items 7(first) and 7a run to completion first; items 7(second),
   8, 9 and 10 complete the ordinary mutations; **items 7b and 7c MOVE** to
   after the push, joining the re-run local review, the advisory PR-body
   `Reviewed HEAD` write and an **explicit required-check evaluation**; item 14
   records the approved HEAD; item 15 is **amended**; items 16 and 17 follow the
   merge bar.
3. **Only the marker-publication segment is conditional on the checkpoint
   count.** A unit whose enumeration **proves zero** omits exactly three things
   — the creation of `C` and `T`, the atomic publication, and the
   post-publication verification — and **creates no tag**. It **executes the
   identical final review, gate, approval and last-mile tail**. The criterion
   **must not** claim any unit runs "the pre-existing path unchanged".
4. **Enumeration failure or ambiguity is NOT zero.** A failed, malformed,
   quarantined, ambiguous or partially-enumerated result **halts**, as does a
   registry exposing no checkpoint operations (P-012). The card states this in
   those words.
5. **No branch-mutating step remains after publication**, other than review
   remediation. Every mutating item in the verbatim extract — 2, 7(first), 7a,
   7(second), 8, 9, 10 — sits before the freeze.
6. **Item 15 is amended**, not retained unmodified. The amended item re-fetches
   and re-evaluates, unconditionally and fail-closed, **all** of: the live
   `headRefOid`; the PR body; `reviewDecision`; reviews and review requests;
   **every review-thread page to exhaustion**; required checks; the **P-018**
   verdict; the **current checkpoint count**; the **marker object** (ref OID
   `== T`, peel `== C`, type `tag`); the **ruleset**; and **`C` is an ancestor
   of `H`**.
7. **Refresh rules are stated precisely.** A **HEAD change** restarts the review
   and the approval — the prior approval is void and a fresh HEAD-bound approval
   is required. A **state-only change** (a thread, review, or check changing
   without a HEAD change) re-runs the **affected gates** and **refreshes the
   approval**; it does not require a new review pass over an unchanged diff.
8. **The merge call pins the expected head.** The observed `headRefOid` is
   passed to the merge API as the expected head SHA. The card states the honest
   bound: **expected-head protects only the HEAD race.** It does **not** make
   the surrounding metadata reads transactional, and the card **must not** claim
   transactional stability for thread, review or check state unless that
   stability is server-enforced (round-10 F-25 honesty requirement).
9. **P-009 is verified by API and asserted afterwards.** Merge-commit mode is
   checked through the API in addition to the existing rendered-UI
   confirmation, the merge is invoked explicitly in merge-commit mode, and the
   resulting commit is asserted to have **two parents**.
10. **After the merge**, `origin/main` is fetched and **both** are required: the
    merge commit has two parents, and **`C` is an ancestor of the refreshed
    `origin/main`**. On a zero-checkpoint unit the `C` term is **absent**, not
    empty, and its absence is not a failure.
11. The section cites **P-022**.
12. markdownlint passes.

**Posture**: documentation-first. **Size**: M. **Complexity**: high.
*(Single file, single section. The verbatim extract removes the re-derivation
work every prior round repeated, which is what keeps this inside two hours.)*

### U5 — Ship startup discovery, freeze, merge confirmation, and removal of session-end resolution

**Depends on**: **U2**, **U3**, **U4** (U4 is the same file — **strictly
sequential**).
**File**: `.github/agents/_ship.agent.md` — **startup recovery**, **Step 6 merge
confirmation**, and **Session end**.

**Acceptance criteria**

1. **Startup discovery** invokes `RESOLUTION_MARKER_DISCOVERY` **by name** and
   does not restate its steps. A **`PENDING`** marker for this repository and
   workspace **blocks new shipment work** and routes to marker recovery. A
   **`DISCHARGED`** marker is an audit record and does **not** bypass any
   ordinary P-001 closure check.
2. **The finalization freeze is stated in the agent's own procedure**: after the
   freeze, the only permitted branch mutations are the single resolution commit
   and later review remediation.
3. **Merge confirmation separates marker discharge from full P-001 closure.**
   The Step 6 Merge Confirmation Gate states that confirming the merge and
   observing `C` an ancestor of refreshed `origin/main` discharges the
   **marker** — checkpoint-resolution delivery — and that **P-001 release
   closure remains independently governed** by shipment state, the closure PR
   and artifacts, knowledge graduation, runtime/operational closure and P-020.
   A discharged marker **never** authorizes the next shipment.
4. **Generic, session-end and post-merge checkpoint resolution is REMOVED.**
   Session end no longer resolves checkpoints for the merged unit, and **no
   after-merge resolution commit is ever created**. The card states the removal
   explicitly and names the 139-S incident (`43e70430` resolved after PR #394
   merged at `56381226`, repaired only by the extra PR #395) as the reason.
5. **No post-merge closure commit carries a checkpoint resolution.** The
   post-merge closure branch protocol is otherwise untouched.
6. **P-020 compaction is untouched.** No compaction exclusion is added; the card
   states that revision 10's exclusion is deleted because the marker is not a
   compactable artifact.
7. markdownlint passes.

**Posture**: documentation-first. **Size**: M. **Complexity**: medium.

### U6 — Stage pre-merge staging-finalization carrier guard

**Depends on**: **U3**.
**File**: `.github/agents/_stage.agent.md` — the **Session end** resolution site
and the **`OWNER-SCOPED RESOLUTION`** block.

**Acceptance criteria**

1. **Checkpoint creation does not require a future PR identity.** The card
   states explicitly that Stage checkpoints legitimately **predate** the PR that
   will carry them, so no mandatory `context.pr` field is introduced at creation
   time. This closes round-10 **F-15**, which found revision 10's mandatory
   carrier binding unsatisfiable for exactly this reason.
2. **Stage resolves a Git-tracked checkpoint ONLY in an explicit pre-merge
   staging finalization**, with the **carrying PR supplied by the caller** and
   already existing.
3. **The carrying PR is queried exactly**, by number, and must satisfy **all**
   of: the repository binding matches; the head binding matches; and the PR
   state is **OPEN**.
4. **The carrier is NEVER derived from ambient state.** The card explicitly
   prohibits `gh pr list --head`, `git branch --show-current`,
   `git rev-parse --abbrev-ref`, and any head-name search as carrier inputs.
   Ambient branch data may be used for **logging only**.
5. **Missing, closed, merged, retargeted, or ambiguous** carrying PR ⇒ **leave
   the checkpoint active and HALT.** Never resolve, never guess, never fall back.
6. **Stage gains no merge authority** (P-010). The guard's only outcomes are
   "resolve within the explicit finalization" or "halt".
7. The section cites **P-022**.
8. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U7 — Orchestrator pre-queue pending-marker route

**Depends on**: **U2**, **U5**.
**File**: `.github/agents/_orchestrator.agent.md` — the pre-queue routing step.

**Acceptance criteria**

1. **Before queueing a shipment**, the Orchestrator runs the **fixed-point
   marker scan** by invoking `RESOLUTION_MARKER_DISCOVERY` **by name**, without
   restating its steps.
2. A **`PENDING`** marker **routes exclusively to Ship recovery** and **blocks
   new shipment work**. The Orchestrator does not resolve, publish, merge, or
   repair anything itself.
3. A **`DISCHARGED`** marker **does not bypass ordinary P-001 closure checks**.
   The card states that marker discharge and release closure are separate
   determinations.
4. Any discovery **halt** propagates to the operator; it is never downgraded to
   a warning and never treated as "no marker".
5. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: low.

## Requirement → unit traceability

Every requirement has exactly **one owning unit** — the unit that installs the
normative text — plus zero or more **enforcing units** that wire it into an
execution path.

| Requirement | Owning unit | Enforcing units |
|---|---|---|
| RQ-1 no post-merge resolution | **U3** (P-022) | U4, U5 (removal of session-end/post-merge resolution), U6 (Stage) |
| RQ-2 resolution rides the same merge | **U1** (`RESOLUTION_MARKER` publication) | U4. **Not U6** — Stage's resolution reaches `main` through the ordinary staging-PR merge, which Stage does not control. |
| RQ-3 push before evidence | **U1** (atomic publication precedes every verification) | U4 |
| RQ-4 re-run the actual review | **U4** (the review is a Ship procedure) | — |
| RQ-5 PR-body record precedes §1.9 | **U1** (`HEAD_EVIDENCE_RULE`) | U4 |
| RQ-6 approval after gate, live re-fetch before merge | **U4** (amended item 15, refresh rules, expected-head pin, two-parent assertion) | — |
| RQ-7 obligation discoverable from a fresh checkout with zero active checkpoints, **scoped to checkpoint-resolution delivery** | **U2** (`RESOLUTION_MARKER_DISCOVERY`, ancestry-derived status, RQ-7 clarification at point of use) | U5, U7 |
| RQ-8 non-self-referential | **U1** (`C` excluded from payload; tag object header supplies it) | — |
| RQ-9 exhaustive, trusted, fail-closed discovery | **U2** (fixed-point scan) | U5, U7 |
| RQ-10 no merge authority conferred | **U2** | U5, U6, U7 |
| RQ-11 every failure halts | **U2** | U1, U4, U5, U6, U7 |
| RQ-12 durable, history-immutable obligation record | **U1** (the annotated tag marker) | U0 (protection), U2 (discovery), U4 (publication) |
| RQ-13 the marker namespace is provably protected | **U0** (the ruleset) + **U1** (the precondition proof text) | U4 (proves before publication), U4 item 15 (re-proves at the last mile) |

## Dependencies

```text
U0 ─────────────────────────────→ U4 ──→ U5 ──┐
                                   ↑      ↑    │
U1 ─┬──→ U2 ─┬─────────────────────┼──────┘    ├──→ U7
    │        └────────────────────────────────→┘
    └──→ U3 ─┴──→ U4
             └──→ U6
```

**Eleven edges**, listed explicitly because the diagram is a reading aid and the
list is the contract:

`U1→U2`, `U1→U3`, `U0→U4`, `U1→U4`, `U3→U4`, `U2→U5`, `U3→U5`, `U4→U5`,
`U3→U6`, `U2→U7`, `U5→U7`.

The graph is **acyclic** with **two roots**: **U0** and **U1**.

* **U0 is a root** — an external GitHub settings change with no repository-file
  prerequisite. `U0→U4` is a real edge: U4's precondition proof describes a
  protection that must actually exist.
* **U1 is the other root** — every other document unit references a definition
  U1 creates. `U1→U3` exists because P-022 references `RESOLUTION_MARKER` by
  name; installing the reference first would leave it dangling.

**File-serialization check.** Two units share
`github-pr-automation.instructions.md` (U1, U2) and are strictly sequential by
`U1→U2`. Two units share `_ship.agent.md` (U4, U5) and are strictly sequential
by `U4→U5`. **Every same-file pair in this plan carries a direct edge**, so no
harvest-time sequencing note is required (round-10 F-22). U3 is the only unit
touching `workflow-policies.md`, U6 the only unit touching `_stage.agent.md`,
U7 the only unit touching `_orchestrator.agent.md`, and U0 touches **no
repository file at all**.

## Verification

Verification is stated as **scenarios with expected behaviour**, each traceable
to a unit. Every scenario below is required.

| # | Scenario | Expected result |
|---|---|---|
| V1 | `markdownlint` over every changed file | passes |
| V2 | **Canonical-copy check.** The `RESOLUTION_MARKER` and `RESOLUTION_MARKER_DISCOVERY` blocks appear verbatim exactly once each, in `github-pr-automation.instructions.md`. In each referencing file's changed region, assert that **no ordered enumeration of three or more canonical steps** appears. Ordinary prose verbs are permitted and expected. | exactly one verbatim copy each; no referencing file restates three or more canonical steps in order; each carries the bare definition name plus a file reference |
| V3 | **Backlog structure.** 1 release unit → 3 sub-epics → 8 tasks; shipment manifest membership equals those **12** IDs. IDs are allocated at harvest; `143.*` must not be reused | exact **membership** match; order is not asserted |
| V4 | Every task card's acceptance criteria are textually identical to its unit's criteria here | identical |
| V5 | **Zero checkpoints.** A unit whose enumeration proves a complete zero. Trace Step 5 | **no tag created**; `C`/`T` creation, atomic publication and post-publication verification all omitted; the **identical** review, §1.9, required-check evaluation, P-018, approval, amended-item-15 and merge tail executes; the merge bar's `C`-ancestry term is **absent**, not empty, and its absence is not a failure |
| V6 | **Malformed / incomplete enumeration.** Enumeration errors, returns a quarantined or schema-invalid record, is ambiguous, or the registry exposes no checkpoint operations | **halts**; is **never** classified as zero; no commit, no tag, no merge; recorded as a P-005 event |
| V7 | **Atomic success.** Nonzero checkpoints, all preconditions proven | one commit `C`, one annotated tag `T`; `R` ancestor of `C`; single atomic push succeeds; remote branch `== C`, tag ref `== T`, peel `== C`, object type `tag`, ruleset still applies |
| V8 | **Either-ref rejection.** (a) branch lease stale; (b) competing different tag present | **both** refs unchanged in both cases; non-zero exit; halt. *Validated empirically before this revision: (a) produced `HEAD -> main (stale info)` + `<tag> (atomic push failed)` with the tag not created; (b) produced `<tag> (stale info)` + `HEAD -> main (atomic push failed)` with the branch not advanced* |
| V9 | **Lost push response.** Push result is lost or ambiguous; re-run the protocol | the precondition scan finds the canonical marker present and **byte-identical**; recovery is **idempotent**; **no second tag**; the protocol does **not** infer identity from the push's silence but proves it from the payload and digest |
| V10 | **Ruleset drift.** Between the precondition proof and the last mile, the ruleset is altered, disabled, gains a bypass actor, gains a matching exclusion, or is deleted | the amended item-15 re-proof **halts** before merge in every variant |
| V11 | **Malformed / lightweight / wrong-identity / bad-digest markers.** Present each in the scan | each **halts**; none is silently skipped; a lightweight tag halts on object type; a wrong `repo_id`/`workspace_id` halts rather than being ignored |
| V12 | **Duplicate and conflict markers.** Two markers with the same primary key; a marker at the expected ref peeling to a different commit | both **halt** deterministically; neither is deleted, updated, or re-pointed |
| V13 | **Empty PR body and deleted source branch.** Empty the PR body entirely and delete the head branch; run discovery | the marker is **still found**, validated, and its status derived; discovery reads **no** PR body and **no** source branch to reach it |
| V14 | **Pending marker → one provenance-valid PR.** `C` not an ancestor of `origin/main`; the commit is associated with several PRs of which exactly one is provenance-valid | status **`PENDING`**; the unique provenance-valid PR is selected; `C` proven an **ancestor** of its live head; **multiple raw results do not halt** |
| V15 | **Endpoint ambiguity / unavailability.** Zero provenance-valid results; two provenance-valid results; endpoint failure past the retry bound; renamed or deleted head branch; closed-unmerged PR; missing PR | **halts** in every variant; merge status is never inferred; no blind second merge |
| V16 | **`H` above `C`.** Review remediation appends commits above `C` | the last mile requires **`C` ancestor of `H`**, and **passes**; `C == H` is **not** required; the HEAD change **voids the approval** and requires a fresh review and a fresh HEAD-bound approval |
| V17 | **Force-push / history loss.** The head branch is force-pushed so `C` is no longer reachable from `H` | the `C`-ancestor-of-`H` re-check **halts**; the **marker survives** (it is a protected tag in a namespace no branch operation touches) and still peels to `C` |
| V18 | **Late checkpoint.** A checkpoint appears after publication | **halts**; **no second tag**; the first tag is **not** amended; the resolved set stays frozen |
| V19 | **Expected-head merge + two-parent / P-009.** Merge with the observed head pinned | the merge API refuses server-side if the branch advanced; the result is a merge commit with **two parents**; the API-side merge-mode check and the rendered-UI confirmation both pass |
| V20 | **Squash / rebase.** Attempt each | both **rejected** (P-009); neither produces a discharge; the repository's `allow_squash_merge: false` / `allow_rebase_merge: false` settings are confirmed as a second witness. A squashed or rebased landing would not make `C` an ancestor of `origin/main`, so it would correctly read **`PENDING`**, never falsely discharged |
| V21 | **Refreshed main containing `C`.** After the merge, fetch `origin/main` | `C` is an ancestor ⇒ **`DISCHARGED`**; and discharge is asserted to cover **checkpoint-resolution delivery only** — the trace confirms the P-001 closure set is still evaluated independently |
| V22 | **No generic / post-merge resolution.** Read Ship's Step 6 and Session end after U5 | **zero** checkpoint-resolution steps for the merged unit; **no** after-merge resolution commit anywhere |
| V23 | **Stage carrier binding.** (a) caller supplies an OPEN, repo- and head-bound carrying PR; (b) missing; (c) closed; (d) merged; (e) retargeted; (f) ambiguous; (g) ambient branch is the only available signal | (a) resolves within the explicit finalization; (b)–(g) **leave the checkpoint active and halt**; in (g) no ambient-derived carrier is ever used. Grep the changed region for `gh pr list --head`, `git branch --show-current`, `rev-parse --abbrev-ref` ⇒ **zero** occurrences as carrier inputs |
| V24 | **Exact Step 5 order.** Ordered top-to-bottom inspection of `_ship.agent.md` Step 5 against U4's verbatim extract | items 7(first)/7a first; 7(second)/8/9/10 before the freeze; **7b and 7c now after the push**, with the re-run review, the advisory `Reviewed HEAD` write and the explicit required-check evaluation; item 14 HEAD-bound; item 15 **amended** to the full re-fetch set including marker object, ruleset and `C`-ancestry; item 16 gains the API check and two-parent assertion; **no branch-mutating item after the freeze other than review remediation** |
| V25 | **Round-10 term deletion.** Grep the **normative** sections of the plan, hardening and decision, and **all changed files**, for: `RESOLUTION_OBLIGATION_RECORD`, `POST-A`, `POST-B`, `Channel A`, `Channel B`, `B0`…`B7` **in their Channel-B step sense**, `PV-B`, `OB-1`…`OB-8`, `CLOSURE_LOCATOR`, `LAST_MILE_RECOVERY`, `BRANCH_PROTECTION_PREREQUISITE`, `RESOLUTION_PREFIX`, `RESOLUTION_POSTCONDITION`, `resolution_obligation`, `FETCH_HEAD`, `refs/autoharness/scan/`, `WP-0`…`WP-7`; and for `non_fast_forward` **used as a marker-ruleset rule**. Then **classify** each hit rather than counting it — this is an inspection, not a raw count, precisely so it stays satisfiable against a correct implementation (the round-10 F-17 defect class) | **Zero hits classified as LOAD-BEARING**, where load-bearing means the term names a construct the protocol depends on. Five hit classes are **expected and permitted**: (a) anywhere inside `## Retained review history` or `## Retained hardening history`; (b) inside this revision's explicit deletion tables, superseded-disposition tables, and the *Out of scope* enumeration, which exist to record the deletion; (c) an explicit **prohibition** statement — `Forbids FETCH_HEAD`, `never fetch into FETCH_HEAD`, `non_fast_forward is NOT the load-bearing rule` — whose whole purpose is to forbid the construct; (d) this V25 row itself, and any coverage-table cell that cites V25 by name; (e) the decision document's **option labels `B1`–`B5`**, a distinct and older namespace naming deliberation options, not Channel-B steps — these are disambiguated by context (they appear under the options section and are followed by an em-dash option title) and are never the deleted construct. Any hit outside those five classes **fails** the check. *Sweep run at revision 11 across all three documents: every hit classified into (a)–(e); zero LOAD-BEARING.* |
| V26 | **P-012 availability.** For each of `backlogit_list_checkpoints`, `backlogit_resolve_checkpoint`, `gh api` (repo, rulesets, commit→pulls), `git ls-remote`, `git fetch`, `git merge-base` and review-thread enumeration, simulate unavailability at its probe point | each probes **before** the path that needs it; only declared official CLI fallbacks are used; anything else **halts**; an operation-less registry **halts** and is never an implicit zero |
| V27 | **Defect-1 construct check.** Grep the changed files for `DARK_CONTINUATION_PREDICATE`, `ACTIVATION_RECORD_STORE`, `CURSOR_TYPING_RULES`, `CONTINUATION_HANDOFF_EVIDENCE`, `OWNER_SIDE_REVALIDATION`, `PREDICATE_PRECEDENCE`, `MIS_EVALUATION_DIRECTIONALITY`, `SCOPE_MATCH_RULES`, `check-continuation-predicate-drift`, `canonical-phrases.json`, `parity-gate`. **Classify, do not count** — for the same reason as V25 | **Zero hits classified as LOAD-BEARING**, i.e. zero hits that install, define, reference-as-required, or depend on any Defect-1 construct. Two hit classes are **expected and permitted**: (a) this V27 row itself, which must name the terms in order to forbid them; (b) the decision document's *Out of scope* enumeration, which names them in order to record that Defect-1 work is **excluded** and unauthorized. Any hit outside those two classes **fails** the check. *Sweep run at revision 11: all hits classified into (a)–(b); zero LOAD-BEARING; zero occurrences anywhere in the plan's or hardening's unit, definition, or acceptance-criteria text.* |

## Residual risks

**Numbering note.** `RR-2` is intentionally absent. It was closed in revision 7
and its disposition cited unit `U8`, which revision 11 does not contain.
Carrying a closed risk forward with a dangling unit reference would be a
cross-reference defect, so it is retired rather than renumbered; the historical
text survives verbatim under `## Retained review history`. Reference numbers are
never reused.

| Ref | Risk | Disposition |
|---|---|---|
| RR-1 | These are prose protocols executed by an LLM. Correct wording does not prove correct execution. | **Accepted and recorded.** Revision 11 narrows the exposure materially: the load-bearing checks are now **concrete commands with binary outcomes** — `git merge-base --is-ancestor`, `git cat-file -t`, `git ls-remote`, a digest recomputation — rather than multi-step protocols whose correctness depends on faithful sequencing. A miss is loud. |
| RR-3 | The durability object can be tampered with. | **Closed at revision 11 by U0 + U1**, replacing revision 9's `docs/closure/` record and revision 10's branch ruleset, both of which independent review invalidated. The marker is an **annotated tag in a namespace protected against update and deletion with no bypass**, published **atomically** with the commit it attests. It is not reachable by any branch operation: a force-push, a branch deletion, or a PR-body edit leaves it untouched. |
| RR-3a | **A repository administrator can edit or delete the U0 ruleset.** | **Accepted and recorded honestly** (U0 AC9). The guarantee is exactly as strong as the ruleset. This is narrower than the any-collaborator exposure it replaces, and it is **detected** at the next precondition proof and at the amended item-15 re-proof rather than silently absorbed. No claim of admin-proof durability is made. |
| RR-3b | **GitHub does not expose a per-tag effective-rules endpoint**, so the ruleset proof is a configuration read by recorded ID rather than an effective-rules evaluation. | **Accepted and stated plainly** (U0 AC7). The plan makes no claim to a stronger guarantee. Mitigation: the ruleset is read **by recorded ID** (not re-discovered by name, which could match a different object), asserted **field by field** against U0 AC2, and verified by the operator in the UI at apply time as a second witness. |
| RR-4 | The fixed-point tag scan costs two `ls-remote` calls plus one fetch per marker candidate. | **Accepted.** The candidate set is bounded by the exact prefix `refs/tags/resolution-obligation/` — in steady state at most one live marker — which is a far smaller surface than revision 10's full-PR enumeration. |
| RR-5 | A crash between `C` creation and the atomic push leaves a local commit and tag with **nothing published**. | **Handled, not merely accepted.** Nothing was published, so nothing is durable and nothing is discoverable: the remote is in its pre-freeze state, the preconditions re-prove cleanly on re-entry, and the protocol restarts preparation. This is the designed outcome of publishing **atomically and last**. |
| RR-6 | Ship-side coupling: the marker publication sits inside Step 5's finalization tail, so a future change to that tail can relocate it. | **Accepted and recorded.** Revision 11 reduces this coupling to **one** surface: the order lives only in `_ship.agent.md`, so a change is a visible edit to the one file that owns it, rather than a silent divergence between two files (round-10 F-16, the reason the standing drift check is deleted rather than reimplemented). |
| RR-7 | A shipment that cannot reach normal termination leaves a **`PENDING`** marker. | **Deliberately fail-closed.** A `PENDING` marker surfacing as a startup halt is the **intended** signal, not a defect. No auto-recovery is attempted, because auto-recovery would require the cross-run continuation semantics this plan is expressly forbidden to add. |

## Out of scope

* All Defect-1 work — see
  `docs/decisions/2026-09-13-dark-mode-continuation-auto-routing-deliberation.md`
  (status `open`). Harvest of Defect 1 is **not** authorized.
* The drift-checker script pair, fixture corpus, parity runner, hook shim, and
  the `.gitignore` entry they required.
* The task↔plan acceptance parity gate.
* **Everything deleted from revision 10's normative design**, enumerated in
  V25: the obligation-record lifecycle, `POST-A`/`POST-B`, Channel A/B,
  `B0`–`B7`, `PV-B*`, `OB-1`…`OB-8`, the load-bearing PR-body locator state
  machine, the `operational-closure` schema change, the P-020 compaction
  exclusion, the broad branch ruleset, post-merge branch protection, and the
  history pickaxe / `FETCH_HEAD` / retained-PR-history machinery.
* Any CI-check persistence substrate, workflow file, external store, lock,
  compare-and-swap, or cross-run cursor.
* backlogit tool changes; `src/`; `crates/`.
* Shipments 140-S, 141-S, 142-S; feature 142-F.
* Upstream autoharness template propagation.

## Plan Review — round 11

**Plan revision reviewed**: 11
**Commit reviewed**: `21dffd63e86add818b0c5237a7009c13109f838c`
**Reviewed HEAD at review time**: `21dffd63e86add818b0c5237a7009c13109f838c`
**Date**: 2026-09-13
**Gate**: `plan-review` (full, multi-persona, cross-model)

### Verdict — **FAIL**

| Severity | Count (raw) | Count (deduplicated) |
|---|---|---|
| **P0** | **0** | **0** |
| **P1** | **21** | **18** |
| P2 | 23 | ~19 |
| P3 | 7 | 7 |

**Zero P0 findings.** The four round-10 P0s are confirmed genuinely closed by
deletion, unanimously, by every reviewer that examined them. The replacement
mechanism is sound in its core: no reviewer challenged the annotated-tag
architecture itself.

**Twenty-one P1 findings block harvest.** Under the operator's bounded outcome
cycle, any P0 or P1 is a FAIL. **The review circuit is opened.** This was
attempt 3 of 3.

### Reviewer panel

Seven personas across six distinct models, for genuine cross-model diversity.

| Persona | Model | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Constitution Reviewer | `claude-opus-4.8` | 0 | 1 | 1 | 2 |
| Architecture Strategist | `gpt-6-astra` | 0 | 6 | 5 | 0 |
| Scope Boundary Auditor | `gpt-5.6-sol` | 0 | 6 | 3 | 0 |
| Agent-Native Parity Reviewer | `grok-4.6` | 0 | 4 | 3 | 0 |
| Security Lens Reviewer | `claude-opus-4.7` | 0 | 2 | 2 | 2 |
| Correctness Reviewer | `claude-sonnet-5` | 0 | 2 | 6 | 1 |
| Learnings Researcher | `claude-haiku-4.5` | 0 | 0 | 3 | 2 |

### What round 11 demonstrably fixed

Recorded because it is real progress and should not be re-litigated if this plan
is ever resumed.

* **All four round-10 P0s are closed by deletion, not repair.** No Channel B, no
  `OPEN`/`CLOSED` lifecycle, no `FETCH_HEAD` destination, no broad branch
  ruleset. U0 targets **tags only** at an exact path with creation unrestricted,
  so the repository-wide branch-cleanup and rebase blast radius (F-04) is gone.
* **The verbatim Step 5 extraction is faithful.** Two reviewers independently
  diffed it against the live `_ship.agent.md` and confirmed the duplicated item
  `7`, the 7a/7b/7c placement, and item 15's actual content. This was the defect
  class that sank rounds 9 and 10.
* **Git semantics are correct and empirically grounded.** The empty-value lease
  as an absence assertion, atomic either-ref rejection, the annotated tag OID
  `T` versus peeled commit `C`, and the already-up-to-date lease-skip gotcha
  were all validated and are handled correctly. No reviewer challenged them.
* **RQ-7 / P-001 separation holds everywhere.** Five reviewers checked it
  independently; none found a path where a discharged marker substitutes for a
  closure gate or authorizes the next shipment.
* **U0 is never agent-executed**, never dark-mode-approved, and Ship is never
  granted ruleset-write credentials. Confirmed consistent across all three
  documents.
* **Squash/rebase cannot falsely discharge** — confirmed correct.
* **RR-3a and RR-3b are honest.** No reviewer found an overclaim of a guarantee
  stronger than the server enforces. This was a round-10 defect class.
* **Prior-art check is clean**: zero contradictions with the compound library.

### P1 findings (deduplicated) — all block harvest

Findings confirmed by two or more independent reviewers are marked **[×2]**.

#### Cluster A — Verification criteria that a correct implementation would fail

This is the round-10 **F-17 defect class recurring**. The revision explicitly
set out to fix it, rewrote V25 and V27 for it, and still shipped four more
instances. This is the single most damning result of the round.

| ID | Finding |
|---|---|
| **F11-01** **[×2]** | *(A-01, S-02)* **Zero-checkpoint path is unsatisfiable at the last mile.** U4 AC6 requires item 15 to re-evaluate "unconditionally … all of" the marker object (`ref OID == T`, `peel == C`, `type tag`) and `C`-ancestry. On a proven-zero path neither `C` nor `T` exists. AC10 and V5 exempt the `C`-ancestry term but do **not** exempt the marker-object checks. A correct zero-path implementation fails its own gate. |
| **F11-02** | *(S-03)* **V24 is unsatisfiable.** It requires "no branch-mutating item after the freeze other than review remediation", but the required nonzero path creates commit `C` and pushes the branch *after* the freeze. U4 AC5 has the correct boundary (no mutation after **publication**); V24 states the wrong one. |
| **F11-03** | *(S-04)* **V25 is still unsatisfiable** despite being rewritten for exactly this reason. Its five permitted hit classes admit deletion **tables** but not the revision-11 **narrative prose**, and that prose contains `RESOLUTION_OBLIGATION_RECORD`, `POST-A`, `POST-B`, `Channel A` and `Channel B` outside every permitted class. The row's own claim that all hits were classified into (a)–(e) is therefore false as written. |
| **F11-04** | *(S-05)* **V6 is unsatisfiable.** It requires "no commit" after a malformed enumeration, but the mandated Step 5 order completes ordinary mutating commits (session memory, runtime verification, operational closure) *before* enumeration. The correct bar is "no resolution commit `C`, no tag, no post-enumeration branch mutation, no merge". |

#### Cluster B — Load-bearing inputs with no defined source

| ID | Finding |
|---|---|
| **F11-05** **[×2]** | *(A-06, N-02)* **`workspace_id` has no defined owner or retrieval contract.** It is one of three primary-key components and the ref name is a digest over it, so determinism is load-bearing. The plan says "the workspace identifier recorded in the repository's own committed configuration" but names **no file and no field**. `.autoharness/config.yaml` has no such field; `.autoharness/workspace-profile.yaml` carries only a machine-local `workspace_path`. Two competent agents hash different strings, produce different refs, and idempotent recovery degrades into a conflict halt. |
| **F11-06** | *(N-03)* **The "recorded ruleset ID" has no durable, agent-readable home.** U0 touches no repository file, name-based rediscovery is forbidden, and marker publication is a permanent Ship duty — so every future session is told to "read ruleset ID X" with no committed X. The task card is not a store. |
| **F11-07** | *(N-04)* **The checkpoint-selection predicate is not deterministic.** "Checkpoints whose recorded context binds them to the current shipment and the **carrying PR**" has no field, no equality rule, and no halt-versus-exclude rule — and U6 AC1 forbids storing the PR at creation. Three plausible agent behaviours diverge, one of which silently reproduces the original 139-S defect by classifying leftover actives as a proven zero. |
| **F11-08** **[×2]** | *(A-05; SEC-03 at P2)* **Canonical JSON is under-specified.** "UTF-8, no BOM, sorted keys, LF, no insignificant whitespace" does not fix Unicode normalization, numeric encoding, string-escape policy, duplicate-key handling, or the sort's code-unit basis. Because idempotence requires **byte-identical** payloads and conflicts can never be repaired, any serializer drift becomes a permanent conflict halt. Recommend RFC 8785 (JCS) plus fixed test vectors. |

#### Cluster C — Contracts that contradict the live harness

| ID | Finding |
|---|---|
| **F11-09** **[×2]** | *(N-01; R-08 at P2)* **Round-10 F-15 is not closed.** The ambient-derivation hole *is* closed in text (AC4's prohibitions hold, and Stage gains no merge authority — P-010 is preserved). But U6 requires a **caller-supplied carrying PR**, and **no such caller exists in Stage's control flow**: Stage's inputs are stash/deliberation/plan/preview, Stage is forbidden from opening PRs, and the Orchestrator step that creates the staging PR runs *after* Stage completes. A compliant agent therefore halts on every ordinary Stage session end; a non-compliant one invents a carrier and reopens F-15. |
| **F11-10** | *(A-03)* **The claimed total order is not enforced — a delegated skill can merge first.** U4 describes Step 5 item 6 as "`pr-lifecycle` — create/update the PR" with no branch mutation. The real `.github/skills/pr-lifecycle/SKILL.md` pushes in Step 1 and runs its own approval, last-mile and **merge execution** in Steps 5b–5d, including admin fallback. *Verified directly during this review.* Nothing in U4 restricts that invocation to returning before merge, so the nested workflow can merge before Ship publishes the marker. |
| **F11-11** | *(A-04)* **The freeze is not reconciled with the checkpoint producers.** Live Ship requires checkpoints when review produces findings, when CI remediation resolves or blocks, and at session end. U5 removes resolution sites but not these **creation** obligations, so the post-publication "late checkpoint halts" rule collides with instructions that mandate creating exactly such checkpoints. |
| **F11-12** | *(C-01)* **The Constitution Check is premised on a false statement of workspace fact.** The plan asserts the workspace "does not carry" an eleven-principle constitution and deletes the mapping, citing round-10 F-19. *Verified directly during this review*: `.github/instructions/constitution.instructions.md` **exists**, is active (`applyTo: '**'`), and carries principles **I–XI** plus a NON-NEGOTIABLE **Task Granularity** section. Principle **XI (Merge Commit History Preservation, NON-NEGOTIABLE)** governs exactly the P-009 work this plan touches, and the constitution's Governance clause makes the Constitution Check a hard MUST. Revision 11 over-corrected F-19 into the opposite error. |

#### Cluster D — Recovery, lifecycle and trust-model gaps

| ID | Finding |
|---|---|
| **F11-13** **[×2]** | *(A-02, R-01)* **Unhandled crash window during resolution *preparation*.** "Prepare all resolutions without committing them" is silent on whether preparation mutates checkpoint state. If it does, a crash after preparation but before `git commit` leaves every checkpoint locally resolved with nothing committed or pushed; on restart the exhaustive enumeration finds zero actives, takes the zero path, creates no marker, and **silently reproduces the exact 139-S defect this plan exists to prevent**. RR-5 covers only the post-`C` window. |
| **F11-14** | *(S-01)* **The ambiguous-push contract is self-contradictory.** U1 AC12 requires that "any ambiguity must leave both refs unchanged and halt", but V9 handles a lost response where the marker *is* already present — i.e. the push succeeded and both refs changed. A client cannot force an already-successful immutable publication back to unchanged. Rejection and indeterminate transport must be separated. |
| **F11-15** | *(R-02)* **A legitimately abandoned shipment permanently blocks all future work.** Its carrying PR closes unmerged (an ordinary event), so its marker can never become DISCHARGED; ref hygiene forbids deleting or updating it; and U5 AC1 makes a PENDING marker block new shipment work. No unit defines an operator recovery path. |
| **F11-16** | *(SEC-01)* **Namespace-poisoning denial of service.** Creation is deliberately unrestricted, deletion is restricted with empty bypass actors. Any push-capable identity can create a wrong-identity tag under the prefix; discovery's rule 5 **halts** on it rather than skipping it, and nothing in the protocol can remove it. Every startup and pre-queue halts until an administrator disables U0, deletes the tag, and reinstates it. Undisclosed in the residual risks. |
| **F11-17** | *(SEC-02)* **Deterministic-ref pre-emption.** All three primary-key inputs are public or committed, so an adversary can compute the next shipment's exact ref and pre-plant a conflicting payload. The conflict rule correctly halts Ship — but the ref cannot then be removed by the protocol *or* by a non-admin operator, and the plan's "the operator resolves it" has no defined procedure. |
| **F11-18** | *(S-06)* **RQ-13 violates the table's own exactly-one-owner rule**, listing "U0 … + U1" in the Owning-unit column. |

### P2 and P3 findings

Recorded for completeness; they do not change the verdict. Principal items:
Principle IX tension undocumented (C-02); discovery cost is linear in **all
retained markers**, not "at most one live marker" as RR-4 claims (A-08, S-09,
**[×2]**); fixed-point scan overstates its synchronization guarantee (A-07);
no explicit same-repository binding precondition for fork-headed PRs (A-09);
single-file authoring used as a proxy for bounded effort (A-10); hardening D15′
reintroduces a PR-body provenance dependency the architecture removed (A-11);
unspecified retry bounds (S-07); V27 passes against renamed equivalents (S-08);
U7's insertion locus is not a real Orchestrator step (N-05); merge pin does not
name `--match-head-commit` (N-06); "prove atomic push capability" has no command
(N-07); Ship's ruleset read needs `Administration: Read`, one toggle from write
(SEC-04); unbounded candidate fetches before the halt classification (SEC-05);
advisory `repo_owner`/`repo_name` not forbidden as routing inputs (SEC-06);
commit-to-pulls eventual consistency conflated with permanent absence (R-03);
**a revert of the merge leaves the marker permanently DISCHARGED** though the
effect was undone (R-04); "current checkpoint count" comparison semantics
undefined at the last mile (R-05); the enumeration/commit race is narrowed, not
closed, and is not carried as a residual (R-06); a workspace-ID rename
permanently orphans every prior marker as an unfixable wrong-identity halt
(R-07); five compound-library prior-art references the plan should cite
(L-01…L-05).

### Gate decision and circuit state

**FAIL.** Harvest is **not** authorized. `harvest_authorized` stays `false`.

Per the operator's bounded outcome cycle, this was **attempt 3 of 3** and both
prior attempts also failed. **The review circuit is now OPEN.** No fourth
remediation attempt is authorized by this cycle. Disposition of PR #396 returns
to the operator.

### Assessment for the operator

The mechanism is not the problem. Round 11 replaced a design that failed on
**four architectural P0s** with one that drew **zero P0s** from a seven-persona,
six-model panel, and the reviewers independently confirmed its hardest technical
claims — Git lease and atomic semantics, tag-object identity, ancestry-derived
status, squash/rebase safety, and the RQ-7 boundary. That is a genuine and
substantial improvement.

What failed is **specification completeness at the seams**. The 21 P1s are
overwhelmingly of three kinds: inputs the protocol depends on but never sources
(`workspace_id`, the ruleset ID, the checkpoint-selection predicate, the Stage
caller); verification rows that a correct implementation would fail; and
contracts that contradict the live harness the plan must edit (`pr-lifecycle`'s
merge authority, the checkpoint-creation obligations, the constitution's actual
existence).

Two findings deserve particular operator attention because they are recurrences
of the precise defect classes this revision was commissioned to eliminate:
**F11-01/02/03/04** are four new instances of round-10's F-17 unsatisfiable-
verification class, in a revision that explicitly rewrote two rows to fix it;
and **F11-13** describes a crash window that would silently reproduce the
original 139-S defect the entire plan exists to prevent.

A recurring meta-pattern across three rounds is that the plan's self-checks pass
where an independent panel does not. Any future attempt should invert the order:
establish the missing input contracts and diff every claim against the live
harness files **first**, and write the verification rows **last**, against a
concrete implementation rather than against an intended one.

---

**Everything below this line is immutable historical evidence.** It records what
revisions 1–10 were reviewed against. **The revision-10 normative design it
describes is SUPERSEDED by revision 11.** No definition, unit, requirement,
verification, or disposition below may be cited as normative for revision 11.

## Retained review history

This section is **evidence, not authority**. It records what previous revisions
were reviewed against, and it is **appended to, never rewritten**. Revision 8's
own Round 8 review genuinely returned **FAIL** with three open P1s and left the
circuit **OPEN** at attempt counter 3; that record stands unaltered below. The
round-9 review returned **FAIL** with one P0 and fifteen P1s and is appended
verbatim at the end of this document. Revision 10 remediated those findings under
explicit operator authorization, but **no row below, and no earlier revision's
verdict, may be cited as a harvest gate for revision 10**. Revision 10's own gate
was a fresh independent four-persona cross-model full-plan review, appended at the
end of this document as **round 10**. It returned **FAIL** with four P0s and
seventeen P1s. The circuit is **OPEN at attempt counter 5**, and revision 10 is
**not harvestable**.

| Round | Reviewers | Verdict | Scope reviewed |
|---|---|---|---|
| 1 | Scope Boundary Auditor (`gpt-5.6-sol`), Constitution Reviewer (`claude-opus-4.8`) | FAIL (6×P1, 4×P2) / ADVISORY | revision 1 (both defects) |
| 2 | Scope Boundary Auditor | FAIL (5 blocking, 1 new P2) | revision 2 (both defects) |
| 3 | Scope Boundary Auditor | FAIL (5 blocking, mechanical cross-reference contradictions) | revision 3 (both defects) |
| 3-confirm | Scope Boundary Auditor | PASS — **superseded and withdrawn** | Same-reviewer, scoped to its own five findings, obtained without the mandatory escalation. Never a valid harvest gate. |
| 4 — P-013.6 escalation | Independent escalation reviewer, route `gpt-5.6-sol` / `openai` / `xhigh`; same-route guard NOT triggered | **ESCALATION_BLOCKS** | 8 blocking corrections. **Seven were incorporated into revision 4; blocker 8 — the requirement for a fresh independent full-plan review — remained outstanding at that point.** |
| 5 | Independent seven-persona panel: Constitution, Rust/feasibility, Scope Boundary Auditor, Learnings, Architecture, Agent-Native Parity, Security | **FAIL** (13 blocking P1 plus P2/advisory) | revision 4 (both defects). Blocker 8 discharged **as process** — the review was performed — but its verdict was FAIL, so the gate it guarded stayed closed. |
| 6 | Independent four-persona panel: Scope Boundary Auditor (`gpt-5.6-sol`), Constitution (`claude-opus-4.8`), Correctness (`gemini-3.8-flash`), Agent-Native Parity (`grok-4.6`) | **FAIL** (3 FAIL, 1 ADVISORY) | revision 6 (Defect 2 only). All findings were specification defects; none falsified the design. Remediated into revision 7 — see the disposition table below. |
| 7 | Independent four-persona panel, same personas and models as round 6 | **FAIL** (Scope, Correctness, Parity FAIL; Constitution ADVISORY) | revision 7. Panel confirmed **every** revision-6 finding genuinely closed and the design still sound; new findings were deeper specification defects exposed by the earlier fixes. Remediated into revision 8 — see below. |
| 8 | Independent Scope Boundary Auditor (`gpt-5.6-sol`, xhigh) | **FAIL** (3 P1, 5 P2) | revision 8. **Third consecutive FAIL. Review circuit OPEN — attempt counter 3.** P-013.6 escalation fired; Stage halted without harvesting. |
| 8-escalation | P-013.6 escalation reviewer, route `gpt-5.6-sol` / `openai` / `xhigh`, against HEAD `bbb52b65` | **ESCALATION_BLOCKS** | revision 8. Reasoning-only. Produced findings A–E, remediated into revision 9 under explicit operator authorization for ONE bounded revision plus ONE fresh full review. |
| 9 | Independent five-persona panel, cross-model | **FAIL** (1 P0, 15 P1, 11 P2, 6 P3) | revision 9. Returned **RR-3** as the blocking design question for an operator decision. Findings appended verbatim below and remediated into revision 10; the operator's RR-3 decision — an approval-gated GitHub branch-ruleset prerequisite — is implemented as **U0** + the rebuilt Channel B in **U13**. |
| 10 | Independent four-persona panel, cross-model: Correctness (`gpt-5.6-sol`), Constitution/Policy (`claude-opus-4.8`), Scope Boundary and Maintainability (`grok-4.6`), Security and Ops Risk (`gemini-3.8-flash`) | **FAIL** (4 P0, 17 P1, 9 P2, 5 P3) — **unanimous** | revision 10, at HEAD `23731cba`. Confirmed the round-9 **P0 (F-11) genuinely closed** and the Ship Step 5 structural extraction accurate. Found four new P0s: the canonical/U13 Channel B divergence with the discharge-rejection bug reinstated; U13 AC6 halting on the protocol's own happy-path interval; an unsatisfiable `FETCH_HEAD` vs retained-ref contract; and repository-wide workflow breakage from U0's `deletion` + `non_fast_forward` rules. Panel recommended returning a narrower design — immutable marker ref instead of branch ruleset — to the operator. Findings appended verbatim below. **Circuit OPEN at attempt counter 5.** |

### Round 8 — outstanding findings (SUPERSEDED by revision 9; recorded as they stood)

**Status of this subsection.** The three P1s below were genuinely open when
revision 8 was reviewed, and the text is preserved as it stood rather than
rewritten. Revision 9 remediates each; the remediation column names where.
Marking them remediated here is **not** a verdict — only the pending revision-9
review can confirm the remediations are adequate.

Revision 8 closed every round-7 P1 the auditor could verify: the P-003 sub-epic
tier is restored, U4 AC1 references without restating, the RQ-2 trace explicitly
excludes U8, U4 AC4 exposes the Step-5 reorder, hardening D6 defers to D10, U2's
locus is fixed, `pr_role` is gone, and U6 is owner-scoped. No Defect-1 construct
is reintroduced. All eight units remain single-file documentation units.
*(Historical: revision 9 raised the count to ten; every unit is still a
single-file documentation unit.)*

These three P1s were open at revision 8 and were carried to escalation. The
revision-9 remediation for each is recorded in the fourth column:

| # | Finding | Required fix | Revision-9 remediation |
|---|---|---|---|
| 1 | **RQ-6 has no executable enforcement path.** The canonical prefix asserts that a live re-fetch of HEAD, threads and CI already exists in Ship Step 5 and is unchanged. It does not: real item 15 re-runs the P-018 gate and re-queries `headRefOid` only — it never re-fetches required CI, and it does not refresh all review threads when P-018 is disabled. U4 AC9 then *requires* item 15 to stay unmodified, so RQ-6 is credited to a unit that is forbidden from implementing it. | Expand U4 to amend the last-mile item with an explicit fail-closed post-approval query of live HEAD, full thread state and required checks; drop the "item 15 unmodified" criterion. | **Applied.** `RESOLUTION_PREFIX` restructured into S1…S8 with the exact safe order; U4 carries a verbatim Step-5 extract; the "item 15 unmodified" criterion is **withdrawn** and U4 AC9 now **amends** item 15 to re-fetch headRefOid, PR body, reviewDecision, review requests/reviews, every review-thread page, required checks and resolution-commit ancestry; U4 AC10 adds the six-part merge bar and the no-stale-approval refresh rules; item 16 (P-009) stays unmodified. Verified by V4. |
| 2 | **The zero-checkpoint bypass claim is false.** U4 AC3 promises a zero-checkpoint unit runs "the pre-existing path unchanged", while AC2/AC4 require all mutating items to move before the prefix and the readiness gate to move after it. Real Ship Step 5 has readiness items 7b/7c *before* runtime verification, closure-artifact generation, follow-up writes and the push (items 7–10), so the reorder changes the common path for every unit, including zero-checkpoint ones. | State that zero-checkpoint units skip locator publication and resolution but use the newly ordered common readiness path. Do not claim their step order is unchanged. | **Applied.** The checkpoint count now selects segment **S4 only**; the "pre-existing path unchanged" claim is removed everywhere and expressly prohibited by U4 AC3. Enumeration failure/malformed/quarantine/ambiguity is **not zero** and halts; a late-appearing checkpoint forces re-evaluation; no empty locator; Stage startup recovery preserved as separate. Verified by V13–V16. |
| 3 | **D14 is not actually folded into a task criterion.** D14 requires reading `branch`/`pr`, fetching the PR head, checking out a local branch, confirming the checkout and halting on failure. Its cited U5 AC2 lists only `gh pr view`, `git fetch` and `git merge-base` — no checkout, no verification — and no V-check covers it. Because task-card criteria are declared exact, U5 could pass while recovery is still sitting on `main`. This re-opens the very hazard D14 was written to close. | Add a U5 acceptance criterion requiring the by-name working-tree placement, checkout and verification before any committing re-entry, plus a matching verification check; then repoint D14's fold reference. | **Applied using existing units — no new unit.** The placement paragraph (whose heading revision 8 had lost, leaving it uncitable) is named `Working-tree placement — committing re-entry only` and defined in **U3 AC12** (WP-1…WP-7); **U5 AC9** executes it by that exact name before the only committing recovery branch; the U3→U5 dependency is unchanged. Verified by **V12** and cases **V12a–V12f**. D14's fold reference is repointed. |

Open P2s at revision 8, both now closed: V2 was unsatisfiable as written (it
forbade verbs U1/U8 are required to use) — **replaced in revision 9** by a
sequence-restatement check that permits ordinary prose verbs; and V8's reference
command lacked `--paginate` so its comparison was invalid — **fixed in revision
9** by paginating the reference command.

**Closed on 2026-09-13** (PR #396 Copilot review remediation pass, commit
recorded in the PR): the decision document's in-scope list now carries
`_stage.agent.md`, states the unit count explicitly instead of "7-task plan", and
no longer names the retired `RESOLUTION_ORDER`; hardening D14's stale
`Folds into` reference is withdrawn along with its incorrect "applied" status.
*(The unit count that sentence recorded was **eight (U1–U8)** at revision 8; it
is **ten (U1–U10)** at revision 9 following the RR-3 closure.)*

### Additional findings from the PR #396 Copilot review (remediated in the revision-8 pass)

These were raised on the published PR rather than by the four-persona panel.
They are **specification hardenings and honesty corrections**. The dispositions
below are recorded **as they stood at revision 8** and are not rewritten; at that
point none of them closed any of the three P1s and none changed the circuit
state. Where revision 9 has since advanced a disposition, that is noted inline.

| Thread | Finding | Disposition |
|---|---|---|
| `PRRT_kwDORJEduc6h6juE` | The "exhaustive and trusted" read protocol validated no provenance, so a fork PR could forge a locator and halt startup or steer recovery. | **Fixed.** New *Provenance validation* block: PV-1…PV-7, with untrusted candidates **discarded silently** (so an outsider cannot deny startup) and trusted-but-inconsistent ones **halting**. All downstream rules operate over the TRUSTED set only. |
| `PRRT_kwDORJEduc6h6juv` | RR-3's "no worse than today" disposition contradicts RQ-7. | **Fixed by honest reclassification** *(at revision 8)*. RR-3 was reclassified **OPEN**, not accepted: resolving every checkpoint pre-merge makes the deletable PR-body marker the *sole* obligation record, which is strictly worse than the pre-change still-active checkpoint. The durable publication record was **not yet designed** at that point. **Superseded at revision 9:** RQ-12, units U9/U10 and hardening D15 design and install it; RR-3 is now **CLOSED**, subject to the pending revision-9 review. |
| `PRRT_kwDORJEduc6h6jv7` | Body status said `review_verdict: none` while frontmatter said FAIL. | **Fixed.** The status paragraph now states the round-8 FAIL, the three open P1s, attempt counter 3 and the open circuit. |
| `PRRT_kwDORJEduc6h6jv-` | Retained-history note still said "revision 7". | **Fixed.** Now names revision 8 and its FAIL verdict. |
| `PRRT_kwDORJEduc6h6ldd` | "Handled normally" undefined for two locators naming the same shipment. | **Fixed.** Byte-identical duplicates collapse to one record; **any** field difference halts. `updated_at` explicitly barred as a tie-breaker (it is body text). |
| `PRRT_kwDORJEduc6h6ldr` | "Resolution commits exist" had no executable definition on the `RESOLUTION_PENDING` path. | **Fixed.** New *Resolution-state classification*: per-checkpoint state at the fetched PR head, reduced to `NONE` / `PARTIAL` / `ALL` / `INDETERMINATE`, with strict precedence. Only `NONE` resumes; the other three halt. |

## Escalation record (P-013.6)

**Trigger**: plan-review attempt counter reached **3** with three consecutive FAIL
verdicts (revisions 6, 7, 8).

**Resolved escalation route**: `gpt-5.6-sol` / `openai` / `xhigh`, read fresh from
`.autoharness/config.yaml` `model_routing.stage.escalation` at session start. The
legacy flat `model_routing.escalation` key is empty, so there is no both-present
ambiguity.

**Same-route guard**: NOT triggered. Stage's own role route is
`claude-opus-5` / `anthropic` / `high`, which differs from the escalation route in
all three fields. `ESCALATION_DEGRADED` therefore does **not** apply and the
escalation is live rather than a no-op.

**Disposition**: the failing operation was **not** re-executed at revision 8.
Stage halted: no harvest, no shipment assembly, no successor ID allocation. The
escalation is a reasoning escalation only and confers no authority to promote
this plan.

**Escalation execution and operator re-authorization (revision 9).** The
escalation ran under route `gpt-5.6-sol` / `openai` / `xhigh` against HEAD
`bbb52b65df1e63f9e8ebbd28b4ccd0fc61718cdd` and returned `ESCALATION_BLOCKS` with
findings A–E. The operator then explicitly authorized **one bounded planning
revision plus one fresh full independent plan-review gate** — and nothing
further. Revision 9 is that revision. It remains within the reasoning-escalation
boundary: **no** harvest, **no** shipment claim or assembly, **no** activation,
**no** implementation, **no** merge, and **no** revival of the abandoned `143.*`
artifacts. `harvest_authorized` stays `false` and moves only if the fresh review
of revision 9 returns PASS **and** the Orchestrator separately routes the next
step.

**Answer to the escalation question.** The escalation asked whether U4 should be
decomposed against a **verbatim extract of the real Step 5 item list** rather
than against a prose description of it. **Yes — and revision 9 does exactly
that.** U4 now carries the extract inline, including the two facts every prior
prose round missed: the item number `7` is **duplicated** in the live file, and
readiness items 7b/7c sit **before** the mutating items 7(second)/8/9/10 and the
push. Those two facts are the direct cause of open findings 1 and 2, which is
strong evidence the escalation's diagnosis was correct.

**Assessment carried to escalation**: the architecture has not been falsified.
Three independent panels have each confirmed the design sound and each closed
finding has stayed closed. The failure mode is that this plan specifies *edits to
agent prompt files* against a target (`_ship.agent.md` Step 5) whose real item
ordering is more entangled than a documentation-domain unit can restate safely —
every round has surfaced a further mismatch between what the plan asserts Step 5
contains and what it actually contains. The escalation question is therefore
whether U4 should be decomposed against a **verbatim extract of the real Step 5
item list** rather than against a prose description of it.

### Round 7 — disposition of the independent revision-7 review

| Finding | Source | Revision-8 disposition |
|---|---|---|
| Circular precondition `work complete AND PR merge-ready` still in the **canonical block** (line 135) and U4, despite the disposition table claiming it fixed; U2 installs that block verbatim, so the circularity would ship | Correctness P1 | **Fixed.** Canonical block rewritten with an explicit `entry:` clause — at least one active checkpoint plus all branch-mutating work complete except the resolution-dependent gates. The phrase "PR merge-ready" is gone from the definition and from U4. |
| `RESOLUTION_ORDER` spans both sides of the merge, so U4's "invoke by name, do not restate" is unsatisfiable — an agent either re-merges or stops at an undefined boundary | Parity P2, Scope P1 | **Fixed.** Split into `RESOLUTION_PREFIX` (ends at readiness, before approval/merge) and `RESOLUTION_POSTCONDITION` (a Step 6 metadata write). The existing approval/re-fetch/merge items are explicitly not moved or duplicated. |
| U4 AC6–AC9 restated ordering the canonical owner owns, so the surfaces could drift while V2 still passed | Scope P1 | **Fixed.** Those ACs removed; U4 now carries only invocation, placement and non-restatement criteria. V2 gained part (b): grep for the sequence's step verbs in the referencing files and require **zero**. |
| U4 hid a materially larger Step-5 reorder than its S/<2h estimate admitted — AC2 and AC3 together require moving several existing mutating items | Scope P1 | **Fixed.** New U4 AC4 requires all branch-mutating items to sit before the invocation and the old-to-new order to be recorded in the task card. V4 gained part (b): assert the set of items between invocation and merge contains no mutation. |
| Orchestrator discovery wired only into the **global** zero-candidate arm — a legitimately active Stage checkpoint masks the obligation, Ship is never routed, and Step 2 then skips the shipment for being `active` | Parity P1 | **Fixed.** U6 rescoped to fire whenever there is no **ship-owned** active checkpoint, matching Ship's own scoping (U5 AC6), with explicit precedence over Stage routing and queue selection. |
| Recovery re-entry is not executable from a fresh checkout — it must commit, but startup is on `main`, where committing is P-010-forbidden | Parity P2 | **Fixed.** New **Working-tree placement** rule: read `branch`/`pr` from the locator, fetch `refs/pull/<pr>/head`, check out, halt on failure. Read-only ancestry assertions need a fetch but no checkout. |
| U8 patched only Session end, gave no executable predicate, and left the undischargeable best-effort checkpoint in place | Parity P2, Scope P1 | **Fixed.** U8 now covers **both** Stage resolve sites, supplies `gh pr list --state merged --head <branch>` as the test (halt on lookup failure), and qualifies the checkpoint-creation directive. New V10. |
| U8 credited with enforcing RQ-2 although it only prohibits; Stage cannot guarantee its resolution reaches `main` | Scope P1 | **Fixed.** Trace now credits U8 with RQ-1 only, and states why it is not credited with RQ-2. |
| P-003 item 4 requires every task to reference a parent **sub-epic**; the flat decomposition was justified by precedent, not policy text, and P-003's violation action is Halt | Scope P1, Constitution P2 | **Fixed.** Two sub-epics restored — `143-E1` (ordering contract: U1, U2, U4, U8) and `143-E2` (discovery and recovery: U3, U5, U6, U7) — mirroring the plan's own prevention/recovery split. V5 updated. |
| `pr_role: implementation \| closure` is a ghost specification — no unit ever publishes a closure-PR locator, and recovery never branches on it | Correctness P2, Scope P2 | **Fixed.** Field removed everywhere. The locator is stated to be implementation-PR-only, with the reason: the PR body survives merge and branch deletion. |
| No bypass specified for a unit owning zero checkpoints — the prefix would publish an empty locator | Correctness P2 | **Fixed.** Entry condition requires ≥1 active checkpoint; zero-checkpoint units skip the sequence and run the pre-existing path unchanged. |
| U1 lacked the Amendment Log row and version bump every prior policy addition carries; `Gate Point` value never specified | Constitution P2, P3 | **Fixed.** New U1 AC3 (concrete gate point) and AC8 (amendment row `1.25.0` plus header version reconciliation). |
| U2's locus was self-contradictory — "sibling of `### 1.9`" but "placed after `#### 1.9.2`", which would orphan 1.9.3 onward | Scope P2, Correctness P3 | **Fixed.** Locus is now a level-3 section after the **end** of `### 1.9`, with the orphaning hazard stated as the reason. |
| Constitution Check omitted P-017 (recovery auto-enters and walks to a merge bar; dark mode could supply approval for a prior unit's obligation), P-012 and P-008; P-010 row not updated for U8 | Constitution P2, P3 ×3 | **Fixed.** All three rows added, P-010 extended, and U6 AC7 states that a dark approval for the current scope does not satisfy the merge bar for a prior unit's recovered obligation. |
| U1 and U2 listed as co-roots although U1 references a definition only U2 creates | Scope P2 | **Fixed.** Edge **U2→U1** added; U2 is the single root; edge count 10 → 11. |
| Step 1a's "resolution never happened" prose ignored the crash-after-push-before-phase-2 window | Correctness P3 | **Fixed.** The row now branches: no commits → re-enter; commits present → halt, because the locator cannot be trusted to enumerate them. |
| V2/V3/V4/V7/V8 not uniformly falsifiable | Scope P2 | **Fixed.** V2 split into two parts, V3 given concrete tokens, V4 given an explicit mutation check, V7 closed to eleven literal names, V8 rewritten to force a page boundary with `per_page=2`. V10 and V11 added. |
| Hardening D6 ("classify live PR state first") contradicted D10 (locator-status gate first) | Scope P1 | **Fixed** in the hardening document: D6 now applies only after Step 1a admits a complete `RESOLUTION_PUBLISHED` locator. Stale `Folds into` AC references refreshed throughout. |

### Round 6 — disposition of the independent revision-6 review

| Finding | Source | Revision-7 disposition |
|---|---|---|
| `RESOLUTION_ORDER` ownership contradiction — U1 said the instructions file owned it, but U2/U3 never installed it there while U4 required it verbatim in Ship | Scope P1 | **Fixed.** New `Canonical ownership` table names one installed home per definition. U2 AC3 installs `RESOLUTION_ORDER` verbatim; U1, U4 and U8 reference it by name only. V2 rewritten to assert exactly one verbatim copy. |
| `gh pr list --state all` is not an exhaustive-discovery command — no `--paginate`, bounded `--limit 30`, and returns no body | Scope P1, Correctness P3, Parity P2 | **Fixed.** Read protocol now gives the exact `gh api --paginate … /pulls?state=all&per_page=100 --jq …` command, with an explicit note on *why* `gh pr list` is forbidden. New V8 executes it. |
| U4 targeted the wrong loci — resolution is only at Session end item 2; Step 6.0 has no resolve step; the executable merge path is Step 5 | Scope P1, Correctness P2, Parity P1 | **Fixed.** U4 retargeted at **Step 5** and Session end item 2, with an AC requiring the invocation to precede the readiness gate, approval, last-mile re-check and merge. Circular "PR merge-ready" precondition replaced. |
| Session end still directs creating a best-effort checkpoint on yield — hardening D1 recursion, one level down | Correctness P1, Parity P2 | **Fixed.** New `Residual-window checkpoint prohibition` in `RESOLUTION_ORDER`; U4 AC5 retires the directive explicitly; V4 inspects for it. |
| `RESOLUTION_PENDING` locator unhandled — an open PR would be marked `RESOLUTION_PUBLISHED` with no resolution commits; a merged one would vacuously pass ancestry and be marked `RECONCILED`, orphaning the checkpoints | Correctness P1 | **Fixed.** New `Step 1a` locator-status gate runs before any live-PR classification, with explicit open (re-enter `RESOLUTION_ORDER`) and merged (unrecoverable orphan, halt) rows. |
| `Open, HEAD ≠ final_head` row omitted the ancestry assertion — a force-push could drop the resolution commits and the row would merge anyway | Correctness P1 | **Fixed.** Row now asserts ancestry against the fetched PR head **first** and halts on failure. |
| RQ-1/RQ-2 universal but only Ship procedurally rewired; RR-2 does not realize a requirement | Scope P1, Parity P3 | **Fixed.** New unit **U8** adds the narrow Stage qualifier. RR-2 closed rather than carried. |
| Decision DoD required "exactly one implementation unit" per RQ while the trace was many-to-many | Scope P1 | **Fixed.** Trace table now names one **owning** unit plus **enforcing** units per RQ; the decision's DoD wording is corrected to match. |
| U6 routed to an undefined "owning agent"; Ship's own zero-candidate path had no discovery | Scope P2, Parity P2 | **Fixed.** U6 AC6 routes explicitly to **Ship**; U5→U6 edge added; U5 AC6 adds discovery to Ship's `ZERO-CANDIDATE NORMAL STARTUP`. |
| U5's "open PR not an error" could be read as weakening the NON-NEGOTIABLE Merge Confirmation Gate | Parity P2 | **Fixed.** New `Boundary with the Merge Confirmation Gate` paragraph; U3 AC10 and U5 AC3 both state the boundary. |
| No remediation loop specified for a gate failure after resolution | Correctness P2 | **Fixed.** New `Gate-failure remediation loop` in `RESOLUTION_ORDER`; U4 AC9. |
| Constitution Check omitted P-001, P-020, P-015; P-014 row overclaimed "strengthened"; P-003 sub-epic tier unaddressed | Constitution P2/P3 ×4 | **Fixed.** All added; the P-014 row now names the hazard this plan *introduces* and the specific mitigation; P-003 states the flat feature-direct shape explicitly. |
| `RECONCILED` could be set before the P-020 compaction record completes | Constitution P2 | **Fixed.** Phase 3, U2 AC8 and U5 AC7 all require the full P-001 closure set including the P-020 record. U4 AC10 and V9 protect the P-020 invocation itself. |
| U7 was listed as realizing a requirement but implements none | Scope P2 | **Fixed.** U7 reclassified as a closure deliverable and deliberately excluded from the trace table. |
| Verification was weak — V2 token-presence only, V4 asked grep to infer ordering, V7 list incomplete, "byte-for-byte" unfalsifiable | Scope P2 | **Fixed.** V2 is a canonical-copy check; V4 is an ordered-step inspection; V7 expanded to eleven names; U6 AC7 replaced with a concrete fall-through criterion; V8 and V9 added. |
| Multiple non-`RECONCILED` locators across different shipments undefined | Correctness P3 | **Fixed.** Read protocol states this is a P-001 violation that halts; U3 AC5, U6 AC4. |
| Constraint "nothing in this plan references Defect 1" factually false | Scope P3 | **Fixed.** Narrowed to "no implementation unit introduces or depends on a Defect-1 construct"; V7 scans changed files, not this plan. |

**Why revision 6 is a reduction rather than a revision-5 remediation.** The
revision-5 findings R3, R4, R5, R11 and R12 were each attempts to specify
executable persistence in prose. They were remediated by *adding more prose*. The
halted revision-6 attempt recognized that this could not converge and classified
Defect-1 safety as requiring a new executable component or upstream support — a
conclusion the operator has approved. Revision 6 therefore removes Defect 1
rather than attempting an eighth specification of it. The revision-5 findings
that apply to Defect 2 — R7 (self-referential locator), R8 (live-PR-state-first),
R9 (status-independent discovery), R10 (re-run the review), R13 (merge-authority
bar) — are all carried forward and are realized by U2–U6.

## Plan Review — round 9

**Plan reviewed**: `docs/exec-plans/2026-09-13-checkpoint-resolution-durability-plan.md` revision 9
**Reviewed at HEAD**: `9b15fd4347c49c8a5f6d2277a8eba5dcd4f07242`
**Attempt**: 4 (consecutive FAILs: revisions 6, 7, 8, 9)
**Gate decision**: **FAIL**

### Gate rationale

This was a fresh, full, independent review — not a confirmation of the
revision-8 round. Seven personas read the plan, the source decision (revision 3),
the hardening (revision 9) and the live `.github/agents/_ship.agent.md` from
scratch, and were told explicitly not to manufacture findings where the plan was
sound.

The gate fails on three independent grounds, any one of which is sufficient:

1. **One P0.** The Stage-side safeguard in U8 AC2 keys off the ambient
   working-tree branch instead of the selected checkpoint's carrying PR, which
   permits the exact post-merge resolution P-022 exists to forbid.
2. **Fifteen P1 findings after dedupe.** Four are cross-confirmed by three or
   more independent personas.
3. **RR-3 is re-opened, not closed.** The durable-publication mechanism that
   revision 9 introduced to close RR-3 was found not to hold. Per the governing
   directive, an unresolved RR-3 is itself a FAIL condition.

**Plan hardening**: required (`requires_plan_hardening: yes`) and present
(`docs/exec-plans/2026-09-13-checkpoint-resolution-durability-hardening.md`
revision 9, D1–D15). The hardening document is not the cause of this FAIL; its
D15 fold is invalidated as a consequence of RR-3 re-opening, not the reverse.

### Panel

| Persona | Model | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Constitution Reviewer | `claude-opus-4.8` | 0 | 2 | 3 | 2 |
| Scope Boundary Auditor | `gpt-5.6-sol` (xhigh) | 0 | 5 | 4 | 0 |
| Correctness Reviewer | `gemini-3.8-flash` | 0 | 3 | 2 | 2 |
| Architecture Strategist | `grok-4.6` | 0 | 3 | 4 | 1 |
| Agent-Native Parity Reviewer | `gpt-5.6-terra` | 1 | 6 | 2 | 0 |
| Security Lens Reviewer | `claude-sonnet-5` | 0 | 3 | 1 | 1 |
| Learnings Researcher | `claude-haiku-4.5` | 0 | 1 | 3 | 2 |
| **Merged, deduplicated** | — | **1** | **15** | **11** | **6** |

Cross-model diversity was satisfied: six distinct models across four vendors.
Where personas disagreed on severity, the more conservative severity was taken.

### What the panel confirmed as genuinely sound

Recorded so the FAIL is not read as a wholesale rejection.

* **The verbatim Step 5 extract in U4 is accurate.** Two personas independently
  re-derived it from the live `.github/agents/_ship.agent.md`: the item number
  `7` really is duplicated, readiness items 7b/7c really do precede the mutating
  items and the push, and item 15 really does re-run only P-018 and `headRefOid`.
  This closes the escalation's core diagnosis and is the one structural advance
  revision 9 genuinely delivers.
* **Fork-PR forgery is blocked at the root.** PV-1 discards fork-originated
  candidates for both channels before any body content is trusted, and the
  silent-discard versus halt asymmetry is drawn correctly against outsider DoS.
* **P-009 and P-017 hold.** Item 16 is untouched; U6 AC7's dark-mode carve-out
  closes a real hole.
* **P-010 role separation holds.** U8 gives Stage only a prohibition and a halt,
  never `RESOLUTION_PREFIX` and never merge authority.
* **No Defect-1 leakage.** The obligation record is a passive, halt-only
  frontmatter field with no lock, CAS, cursor, or auto-routing. Three personas
  checked this specifically.
* **Abandoned-ID discipline holds.** No `143.*` ID is revived, re-parented, or
  reused; replacement IDs remain unassigned.
* **`docs/closure/` was the right surface to have chosen.** The architecture
  review agreed it is the only per-shipment Git-tracked artifact Ship already
  writes on the PR branch before resolution, and that the alternatives were
  correctly rejected. The mechanism fails on execution detail, not on venue.

### P0 findings

#### F-11 (P0) — U8 AC2 keys the Stage safeguard off the ambient branch, not the checkpoint's carrying PR

*Agent-Native Parity Reviewer. File: plan, U8 AC2.*

U8 AC2 has Stage `determine the current branch, then
gh pr list --state merged --head <branch>`. Stage is permitted to run on `main`,
and the Checkpoint Payload Contract does not require a carrying-PR number or
branch. A resumed Stage checkpoint created on a staging branch can therefore be
handled while Stage sits on `main`: the query inspects `main`, returns no
matching merged PR, and the agent proceeds to resolve a checkpoint whose
carrying PR has already merged. That is precisely the post-merge resolution
P-022 is written to forbid, reachable through the safeguard meant to prevent it.

**Required fix.** Take the carrying PR number and branch from the selected,
ownership-validated checkpoint context and make those fields mandatory whenever
a Git-tracked checkpoint may be resolved. Query that exact PR
(`gh pr view <pr> --json state,mergedAt,headRefName,baseRefName`), verify head
and base against the stored identity, require state `OPEN`, and halt on absent,
mismatched, closed, or merged. Never use the ambient working-tree branch as the
authority.

### P1 findings

Ordered by cross-persona confirmation count, then by severity of consequence.

#### F-01 (P1) — `RESOLUTION_POSTCONDITION` is self-contradictory: it both closes the record in a commit and "commits nothing"

*Confirmed independently by Constitution, Scope Boundary, Architecture and
Agent-Native Parity — four of seven personas.*

The canonical block requires Ship to "set `CLOSURE_LOCATOR` to `RECONCILED` and
close the `RESOLUTION_OBLIGATION_RECORD` in the same closure commit". The
sentence immediately following it still says the postcondition "is a PR-body
metadata write only and commits nothing", and RR-5 still calls it "a Step 6
metadata write". U2 installs this block **verbatim**, so the contradiction is
installed into the instructions file. Closing the record is a change to a
Git-tracked `docs/closure/` file and necessarily requires a commit.

The consequence is not cosmetic. If "commits nothing" governs, `OPEN` records
are never discharged and Channel B fail-closes every future startup forever. If
the commit governs, the canonical definition the plan installs is false. The
plan additionally never establishes which branch may legally carry that commit:
Ship must not commit to `main` (P-010), and the plan removed `pr_role: closure`,
so no closure-PR path is established — which risks reintroducing the very
"extra PR for a one-line status flip" pattern this work exists to eliminate.

**Required fix.** Split the postcondition into two explicitly named mutations —
the PR-body `RECONCILED` write (metadata, no commit) and the `OPEN` → `CLOSED`
transition (a commit on a named, P-010-compliant closure branch that must be
merged before the obligation is discharged). Delete the "commits nothing"
sentence and the RR-5 phrasing that depends on it. Name the branch.

#### F-02 (P1) — the Constitution Check still asserts the withdrawn "item 15 retained unmodified"

*Confirmed by Constitution (P1), Scope Boundary (P2) and Architecture (P2).*

The Constitution Check P-018 row reads: "Step 5's existing unconditional
last-mile re-check (item 15) is retained unmodified." That is the exact claim
escalation finding A identified as open finding 1 and that revision 9 withdraws
everywhere else — S7 says "AMENDED", U4 AC9 says "Item 15 is amended, not
retained unmodified", and hardening D5 agrees. The Constitution Check is a
second normative statement of Step 5's shape, so an implementer or later editor
reading it is told not to make the change U4 requires.

This finding matters beyond its own content: it is a **residual instance of the
very escalation finding revision 9 was authorized to remediate**, surviving in a
section the revision did not sweep. It is direct evidence that the remediation
was applied section by section rather than globally.

**Required fix.** Rewrite the P-018 row: item 15 is amended per S7, P-018 is
evaluated at S5 and re-evaluated at S7, item 16 remains unmodified. Then grep
the whole document for every other surviving assertion about Step 5's shape.

#### F-03 (P1) — V4(g) forbids the mutation that S4 requires, so every nonzero-checkpoint implementation fails its own verification

*Scope Boundary Auditor.*

V4(g) asserts that no branch-mutating item appears after the S3 enumeration
proof. Canonical S4 deliberately resolves checkpoints, writes the obligation
record, commits and pushes — all after S3. The expected-empty mutation set
therefore makes V4 unsatisfiable for exactly the case the plan exists to handle.

**Required fix.** Restate V4(g) as: no mutation after S3 **except** the
conditional S4 resolution commit and its push; and no branch mutation at all
after S4.

#### F-04 (P1) — S8's five-way HEAD equality is unsatisfiable for zero-checkpoint units

*Correctness Reviewer.*

S8 condition 1 requires `live headRefOid == local HEAD == locator final_head ==
PR-body Reviewed HEAD == approved_head`, "all five agree", under a rule that any
doubt halts. But the plan establishes that a zero-checkpoint unit omits S4
entirely and that "no empty locator is ever published", so `locator final_head`
does not exist. Every zero-checkpoint PR therefore halts unconditionally at the
merge bar and becomes unmergeable.

This is a **new zero-checkpoint truthfulness defect introduced by revision 9's
own finding-B remediation** — the remediation correctly made S5–S8 universal but
did not make the locator-dependent clauses conditional at the same time.

**Required fix.** Make the locator clause conditional: the four always-present
heads must agree, and `locator final_head` joins them when a locator was
published. Likewise condition 2 becomes "every recorded resolution commit, if
any, is an ancestor".

#### F-05 (P1) — `RESOLUTION_POSTCONDITION` demands an impossible transition on zero-checkpoint units

*Correctness Reviewer.*

On a zero-checkpoint unit no locator exists and `resolution_obligation` was
initialized `none` at S2, so `none → OPEN` never happened. Step 6 nonetheless
unconditionally orders Ship to set the locator `RECONCILED` and move the record
`OPEN → CLOSED`, while U10 AC3 states those are the only permitted transitions
and "no other transition is permitted". The agent is instructed to perform a
transition the schema forbids on a record that is not in the required state.

**Required fix.** Make `RESOLUTION_POSTCONDITION` an explicit no-op when no
locator was published and `resolution_obligation` is `none`, leaving `none`
intact in the closure commit. State it in the canonical definition, U4 AC7 and
U5 AC7.

#### F-06 (P1) — S7 never re-enumerates checkpoints, so the late-checkpoint race the plan claims to close stays open

*Correctness Reviewer.*

The plan asserts that "a checkpoint that appears after S3 completed forces
re-evaluation from S3 before merge", and V16 tests exactly that. But S7's
exhaustive re-fetch list — `headRefOid`, PR body, `reviewDecision`, review
requests and reviews, every review-thread page, required checks and
resolution-commit ancestry — does not include active checkpoints, and S8's merge
bar never asserts a zero active-checkpoint count. An agent executing S7 and S8 as
written will never observe a late-appearing checkpoint. The claim and V16 are
unbacked.

**Required fix.** Add "re-enumerate active checkpoints owned by this unit and
assert count == 0; any active checkpoint forces re-evaluation from S3" to S7's
list in both the canonical definition and U4 AC9.

#### F-07 (P1) — legitimate P-020 compaction relocates the obligation record and OB-1 reads it as the deletion attack

*Architecture Strategist (P1); Constitution Reviewer raised the same coupling at P2.*

Channel B reads `docs/closure/`, and OB-1 halts when a record that once appeared
in history is absent from the tree without a recorded `OPEN → CLOSED`
transition. But `compact-context` — whose invocation U4 AC8 leaves **unmodified**
— compacts `docs/closure/` and moves originals to `docs/archive/closure/` for
completed-feature records older than `threshold_days` (default 14). A pre-merge
artifact written at S2 can easily exceed 14 days by merge, and `CLOSED` is
defined to be written *after* the full closure set, which includes P-020. So
routine compaction archives a still-`OPEN` record, and every subsequent startup
fail-closes permanently on what is in fact correct behaviour.

**Required fix.** Add a path-stability invariant: an artifact that has ever
carried `resolution_obligation` must not be renamed, compacted or archived while
its status is `none` or `OPEN`. Either exclude `OPEN` records from
compact-context candidates (and say so in U10 and the P-020 surface) or extend
Channel B to follow `docs/archive/closure/` and treat an archive move as not
OB-1. `CLOSED` must be written before any compaction that can touch the file.

#### F-08 (P1) — S1–S8 hard-codes live item numbers with no standing drift check

*Architecture Strategist.*

The canonical prefix is expressed in terms of "real Step 5 items 7 (first)/7a,
second item 7, 8, 9, 10, 7b/7c moved, item 15 amended, item 16 unchanged",
including the live file's duplicated `7`. U4 then requires `_ship.agent.md` to
invoke the prefix by name and not restate it. The only consistency check, V4,
runs once at implementation time; the wording-drift checker was withdrawn as
Defect-1. This is the *same coupling* that caused revisions 6, 7 and 8 to
describe a Step 5 that did not exist. When Step 5 gains an item or is
renumbered, the instructions and the agent diverge silently, and
readiness-before-mutation can be restored without anything noticing.

**Required fix.** Define S1–S8 by **role** (fix-ci loop, mutating tail,
enumeration proof, conditional resolution, readiness gates, pinned approval,
last-mile re-check, merge bar) with no item numbers in the instructions file, and
keep the item-to-segment binding solely in `_ship.agent.md`. Add a standing
invariant asserting the executable order still matches those roles. V4 is not a
substitute for it.

#### F-09 (P1) — U3, U4 and U9 each exceed the NON-NEGOTIABLE 2-hour granularity rule

*Scope Boundary Auditor raised all three at P1; Constitution Reviewer raised U4
at P3.*

* **U3** combines exhaustive API discovery, seven provenance rules, duplicate
  handling, locator-status classification, a four-state checkpoint classifier,
  seven live-PR outcomes, a merge-authority bar and WP-1…WP-7. V8 plus
  V12a–V12f place at least seven behavioural scenarios on it against a
  fewer-than-four limit.
* **U4** declares its locus as Step 5 and session end, but AC7–AC8 also modify
  Step 6; it carries 13 acceptance criteria, moves two gates, amends item 15,
  adds a merge bar and refresh rules, and is exercised by V4, V9 and V13–V16.
  Raising it to M/high was honest but does not make it compliant.
* **U9** bundles schema and identity, lifecycle, rejected alternatives,
  two-channel enumeration, provenance, deletion archaeology and OB-1…OB-7, while
  labelled S/medium.

**Required fix.** Split each along the seams the auditor named: discovery and
provenance apart from recovery and working-tree placement (U3); the Step 5
reorder apart from Step 6 and session-end reconciliation (U4); record schema and
lifecycle apart from Channel B discovery and reconciliation (U9). Re-derive
`task_count`, the dependency graph and the verification mapping afterwards.

#### F-10 (P1) — S3/S4 is not a mechanically executable `backlogit` contract

*Agent-Native Parity Reviewer.*

S3 says "enumerate every checkpoint owned by THIS unit" and S4 says "resolve
every checkpoint owned by this unit", supplying neither the
`backlogit_list_checkpoints` invocation nor a deterministic ownership predicate.
The installed Ship and Stage recovery protocols are far stricter: call with
`consumer_id` only, apply no `status`/`agent` API filter, inspect quarantine and
validation anomalies *before* partitioning, and resolve only a validated
owner-selected record after a confirmed successful handling. The registry also
exposes no `agent` list parameter. As written, an agent could filter at the API
call and hide quarantined records, sweep in another unit's records, or
bulk-resolve without a per-record handling proof.

**Required fix.** Add a normative S3 algorithm and a matching U4 acceptance
criterion specifying the exact call, the no-filter rule, anomaly-before-partition
ordering, a precise current-unit identity predicate (for example validated
`context.shipment_id == current shipment_id` together with `agent == ship`) and a
per-checkpoint successful-handling proof before any
`backlogit_resolve_checkpoint`. Prohibit bulk and cross-unit resolution
explicitly.

#### F-13 (P1) — Channel B is not independent of the mutable PR body (RR-3 blocker 1)

*Agent-Native Parity Reviewer.*

Channel A selects "entries whose body contains `autoharness:closure-locator`".
Channel B then scans "every PR admitted by PV-1…PV-3 of the same exhaustive
paginated enumeration". If the body locator was deleted before merge, that PR is
no longer a marker-bearing candidate and no instruction unambiguously admits it
for Channel B. The claimed body-deletion recovery therefore fails in exactly the
open-PR residual window it was designed for — which is the whole of RR-3.

**Required fix.** Derive two independent candidate sets from the one paginated
response: apply PV-1…PV-3 to every enumerated PR using API fields alone; Channel
A may then inspect only trusted bodies carrying the marker, but Channel B must
fetch and inspect every trusted PR head **regardless of body contents**. Add an
executable test for an `OPEN` record on an unmerged PR whose body has been
deleted.

#### F-14 (P1) — Channel B's discovery and deletion commands are not executable (RR-3 blocker 2)

*Agent-Native Parity Reviewer.*

`git log --follow --diff-filter=D --format=%H -- <path>` carries no commit-ish,
so it examines only the currently checked-out history rather than each fetched PR
head; `FETCH_HEAD` is overwritten by every subsequent PR fetch; and a fully
deleted artifact has no current-tree path for the preceding `ls-tree` to supply.
`git log -S'resolution_obligation' -- docs/closure/` yields commits but the plan
defines no procedure mapping introduction, deletion and `OPEN → CLOSED`
transitions to a particular record identity. OB-1 and OB-2 therefore cannot be
evaluated exhaustively from a fresh checkout.

**Required fix.** Fetch each trusted PR to a unique retained local ref rather
than relying on `FETCH_HEAD`; scan that ref and `origin/main` explicitly;
enumerate historical artifact paths and field transitions from those explicit
refs; parse parent and child blobs to bind each record identity to its `OPEN` and
`CLOSED` transitions; require an ancestry check from the introducing commit to
the retained ref or `origin/main`; halt on fetch, history or parse failure.
Rewrite V18 to execute the whole workflow, including deletion from an open PR
head and deletion after merge.

#### F-15 (P1) — PV-B4/PV-B5 structurally reject the correct post-merge `CLOSED` transition

*Agent-Native Parity Reviewer.*

Channel B requires "the record's embedded `pr` equals the PR it was read from".
The record is opened in the implementation PR and closed in a *different*
post-merge closure PR, where its embedded `pr` and `branch` correctly still name
the implementation PR. A correct `CLOSED` transition therefore fails PV-B4/PV-B5,
and no alternate provenance relation for the closure PR is defined.

**Required fix.** Define provenance in terms of the immutable implementation-PR
identity, and define the authorized closure-transition carrier separately:
validate the introduction against the implementation PR's head and history, then
validate `CLOSED` as a descendant on a closure PR whose relationship to that
implementation PR is explicitly recorded and checked by Git or API ancestry. Do
not apply one "read from this PR" equality rule to both trees.

#### F-16 (P1) — P-012 degraded mode is asserted but never wired for the new critical-path tools

*Agent-Native Parity Reviewer.*

The Constitution Check says locator discovery "is probed per P-012", but U4, U5
and U6 add dependence on `backlogit_list_checkpoints`, `backlogit_get_checkpoint`,
`backlogit_resolve_checkpoint`, `backlogit_create_checkpoint`, `gh api`,
review-thread enumeration and Git history access without a single acceptance
criterion updating Ship's tool-availability gate, and without specifying
CLI-fallback versus halt behaviour when the registry lacks checkpoint operations.
The dangerous case is a registry with **no** checkpoint operations being treated
as an implicit zero-checkpoint result — a silent fail-open into the merge path.

**Required fix.** Add an explicit pre-S3 and pre-startup availability contract:
probe every required operation before entering the affected path, use only
declared official CLI fallbacks, and otherwise halt before S3, before either
discovery channel, before recovery and before merge. Define the
no-checkpoint-operations registry case explicitly as a halt, never as zero.

#### F-17 (P1) — a trusted decoy PR can indefinitely deny recovery for a real shipment

*Security Lens Reviewer (confidence 0.75).*

Both channels admit any same-repository, default-branch-targeting PR authored by
an `OWNER`/`MEMBER`/`COLLABORATOR` as a carrier for an arbitrary real shipment
ID. PV-6/PV-7 only check that the referenced shipment and feature exist and that
checkpoint filenames look plausible; PV-4/PV-5/PV-B4/PV-B5 only check internal
self-consistency. Nothing checks that the PR *is* the shipment's designated
implementation PR. A trusted-but-careless or compromised collaborator can
therefore open an unrelated decoy PR carrying a fabricated record or locator
naming a real in-flight shipment; any field difference then trips OB-3 or the
duplicate-locator rule and halts that shipment's every future startup until a
human disambiguates. No fork and no admin access is needed, and the risk is
disclosed nowhere in the residual-risks table.

**Required fix.** Add PV-8/OB-8 binding a discovered locator or record to the
shipment's backlog-recorded implementation PR number, and **discard** — not halt
on — candidates whose PR number does not match that binding. At minimum, record
it as an explicit accepted residual risk.

#### F-18 (P1) — S8 is check-then-act; the merge call does not pin the observed HEAD

*Security Lens Reviewer (confidence 0.65).*

S8 requires five values to agree at the moment of evaluation but never requires
the merge invocation itself to be atomic against that observation. Between the S8
read and the merge API call, a push by the author, a bot, or any app with write
access can advance `headRefOid`. GitHub's merge endpoint accepts an expected-head
`sha` parameter for exactly this purpose and the plan does not use it, so the
six-part bar is a TOCTOU pattern rather than a guarantee.

**Required fix.** Require S8 (and item 16 / P-009) to pass the S8-observed HEAD
as the expected head SHA to the merge call so GitHub refuses server-side on
mismatch.

#### F-19 (P1) — the RR-3 "detectable, halting" claim is unsubstantiated (RR-3 blocker 3)

*Security Lens Reviewer (confidence 0.68).*

Revision 9's accepted-residual sentence claims the force-push + history-rewrite +
body-edit combination is now "a detectable, halting condition (OB-2) rather than
a silent one". It is not. The record deliberately carries no SHA anywhere (RQ-8),
and Channel B's probes scan only currently reachable history. Force-push the
branch to before the resolution commit and edit the body, and both probes return
empty — indistinguishable from the obligation never having existed. No checksum,
prior-observed SHA or reflog check exists anywhere in the design to tell the two
apart. This needs only the PR author's own ordinary push and body-edit rights on
their own open branch: no admin, no fork.

**Required fix.** Correct the disposition to state honestly that this combination
is *not* detectable once both channels are erased by the same actor, and then
either require branch protection forbidding force-push once S4 has run (for
example through a required status check set at resolution time) or introduce a
corroboration signal outside that actor's erasure surface — a CI check run or
deployment status posted at S4 is not deletable by an account that can
force-push. **This is an operator decision, not a Stage decision.**

#### F-21 (P1) — the plan ignores established prior art on post-merge worktree isolation

*Learnings Researcher, confidence medium. Prior art:
`docs/compound/workflow-issues/post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`.*

The established pattern uses two distinct worktree paths — an implementation
worktree and a clean post-merge worktree spawned at the merge SHA — with an
explicit rule not to mutate the primary working tree. WP-1…WP-7 describes
fetching `refs/pull/<n>/head` and switching, but never states whether Ship's
pre-merge resolution commit is made in the implementation worktree or a separate
space. The compound library already solved this and the plan does not reuse it.

**Required fix.** State explicitly which worktree carries the resolution commit
and reconcile WP-1…WP-7 with the recorded pattern, or record why it does not
apply here.

### P2 findings

| Ref | Finding | Raised by |
|---|---|---|
| F-20 | U2 AC3 requires the canonical text "including its four ordering invariants" while the canonical definition declares **eight**. U2 installs the block verbatim and V6 checks exactness, so the criterion has no satisfiable reading and an implementer could drop invariants 5–8 — which are the RQ-6 reorder, pinned approval, explicit check evaluation and the RQ-12 record. Fix: say eight, and have V6/V2 assert all eight headings. | Scope, Correctness, Architecture, Agent-Native (4×) |
| F-22 | U1's heading still carries `*(root)*` and has no `Depends on: U2`, contradicting the dependency section's `U2→U1` edge and single-root-U2 statement. An extractor would run U1 first and create the dangling `RESOLUTION_PREFIX` reference the edge exists to prevent. | Correctness |
| F-23 | U5 AC9 restates all seven WP steps although U3 owns the canonical procedure and U5 is meant to invoke it by name. The duplicate can drift undetected because V2 only guards S1–S8 restatements. | Scope |
| F-24 | V19 is not a runnable grep: the file legitimately contains `resolution_commits` and `final_head` in the adjacent locator definition, and a plain grep cannot honour the "within this subsection" boundary. Extract the fenced schema or parse its YAML keys instead. | Scope |
| F-25 | U10 is a behaviour change, not a schema-only change: initializing `none` is new skill behaviour, AC2/AC3 restate a lifecycle U9 owns, and a re-run of `operational-closure` in `pre-merge` may clobber `OPEN`. The graph also lets U4 and U10 run concurrently after U9 — the exact unowned-squatter window U10 exists to close. Add edge `U10→U4`; require create-only initialization; forbid clobbering `OPEN`/`CLOSED`. | Architecture |
| F-26 | The `LAST_MILE_RECOVERY` entry rule requires both channels, but U3 — which owns that rule — has no acceptance criterion mentioning Channel B, the record, or OB-6. U3 can pass with a single mutable channel still installed. | Architecture |
| F-27 | The Constitution Check maps only workflow policies P-001…P-020 and never the workspace constitution's own Principles I–XI, though the Governance clause requires it. Principles VII (destructive-command approval) and VIII (explicit safety modes) are unmapped despite WP performing fetch, branch create/switch, fast-forward and commit. | Constitution |
| F-28 | WP-1 asserts only a clean tree and omits the P-011/P-016 worktree-topology gate before a committing re-entry that creates or switches branches and commits. P-011 is absent from both the Constitution Check and the frontmatter policy list. | Constitution |
| F-29 | OB-7 defines incomplete enumeration only as a non-zero exit or a truncated page. A shallow or grafted clone makes `git log` exit 0 while silently omitting commits past the horizon, so a deletion beyond it scans clean. Require `git rev-parse --is-shallow-repository` and fail closed or unshallow first. | Security |
| F-30 | Step 5 item 16 requires confirming the GitHub merge button in the rendered UI, which U4 AC9 retains unmodified. There is no agent-runnable equivalent, so the new S8 path depends on a human-only check. Verify merge-commit capability and mergeability by API, invoke merge explicitly in merge-commit mode, and assert two parents afterwards. | Agent-Native |
| F-31 | Three coupling points flagged from prior art: Step 5 reordering versus Step 6's assumptions about backlogit record state; WP-1…WP-7 versus the linked-worktree bounded-startup pattern; and the standing `backlogit shipment ship` non-termination risk whose P-015 safe-close fallback the plan does not mention. | Learnings |

### P3 findings

| Ref | Finding | Raised by |
|---|---|---|
| F-32 | Hardening D5's fold cites `U2 acceptance criterion 6`, which concerns locator phase-1 timing (D2). The criterion installing S1–S8 is U2 AC3. | Correctness |
| F-33 | `*(corrected in revision 9; see B below)*` has no referent — the document labels that item Round 8 finding **2**, not B. | Correctness |
| F-34 | `none` is used as a lifecycle state but the schema union is only `OPEN \| CLOSED`. Make it `none \| OPEN \| CLOSED` and state that an omitted field after S2 is OB-5, not "no obligation". | Architecture |
| F-35 | WP-4 passes a locator-sourced branch name to git without requiring argv-array execution or a `--` separator. Git's ref-naming rules limit exploitability, but the plan should foreclose shell interpolation explicitly. | Security |
| F-36 | Describing revision 9 as "bounded" while it adds two units, a requirement, a hardening and an in-scope file is generous. It is sanctioned by escalation finding D and decision revision 3, but the plan should say so explicitly so the framing is auditable. | Constitution |
| F-37 | Prior-art advisories: resolution-commit side effects on `.backlogit/stash.jsonl` and `docs/memory/**` must be committed or carried forward to avoid a dirty-worktree classification next session; and WP re-entry must leave no fetched branch or worktree state on disk before the next claim. | Learnings |

### Runtime verification and operational closure

Explicitly assessed, as the skill requires.

* **Runtime verification** is *specified* (V1–V19 including V12a–V12f) but two
  checks are **unsatisfiable as written** — V4(g) contradicts S4 (F-03) and V19
  is not a runnable grep (F-24) — and one check, **V16, tests behaviour the
  specification does not contain** (F-06). V18 does not execute the workflow it
  claims to verify (F-14). The verification suite therefore cannot currently
  discharge the plan.
* **Operational closure** is the principal structural gap. The plan makes
  `docs/closure/` load-bearing for a merge-gating obligation without reconciling
  it against the two mechanisms that already own that directory's lifecycle:
  `operational-closure` re-invocation (F-25) and P-020 `compact-context` archival
  (F-07). Closure readiness is **not** demonstrated.

### Required disposition

Per the governing directive and the gate table:

1. **FAIL.** Do **not** harvest. Do **not** assemble or claim a shipment. Do
   **not** activate, implement or merge. `harvest_authorized` stays `false`.
2. **RR-3 is re-opened** and recorded as unresolved in `## Residual risks`. Its
   closure requires an operator decision between a branch-protection remedy and
   an out-of-band corroboration signal (F-19), because both options reach past
   the reduced Defect-2 boundary this plan is confined to.
3. **The circuit is open again** at `review_attempts: 4`. This is the fourth
   consecutive FAIL. Stage does **not** self-authorize a fifth attempt; the
   remediation of these findings requires fresh operator authorization, and the
   RR-3 decision must be made before another revision is attempted, because
   F-13, F-14 and F-15 are all consequences of the mechanism RR-3 selects.

### Honest assessment for the operator

Revision 9 is a real advance on revision 8 in one specific, important way: the
verbatim Step 5 extract ends the four-round pattern of planning against a Step 5
that did not exist, and both personas who checked it confirmed it is accurate.
Escalation findings A, C and E are substantially remediated.

It fails on the other two. Finding B's remediation was correct in principle —
making S5–S8 universal — but introduced two *new* zero-checkpoint defects (F-04,
F-05) because the locator-dependent clauses were not made conditional at the same
time. And finding D's remediation, the RR-3 closure, does not hold: three
independent reviewers found the second channel is neither independent of the
first (F-13) nor executable (F-14) nor honest about its residual (F-19).

The directive anticipated this outcome: *"If no existing owned state surface can
safely support this without expanding into Defect 1 or new executable
persistence, do not fabricate a solution."* The venue chosen — `docs/closure/` —
was endorsed by the architecture review as correct. The mechanism built on it was
not sufficient. The narrowest honest position is the one now recorded: RR-3 stays
open, RQ-7 stays unweakened, and the operator decides.

## Plan Review — round 10

- **Attempt**: 5
- **Reviewed revision**: 10 (plan), 10 (hardening), 4 (decision)
- **Reviewed HEAD**: `23731cba156b8aee3334d65b7ed58ff91837aa55`
- **Panel**: four independent reviewers, cross-model (GPT-5.6 Sol — correctness;
  Claude Opus 4.8 — constitution and policy; Grok 4.6 — scope boundary and
  maintainability; Gemini 3.8 Flash — security and ops risk). Each read the plan,
  hardening, decision, and the live agent/instruction files independently, with no
  shared context and no access to each other's reports.
- **Mandated probes**: Channel B with an emptied PR body; ruleset drift and bypass;
  zero-checkpoint versus nonzero paths; P-020 ordering; the verbatim Ship Step 5
  extraction; D14 discharge; P-010 ambient-branch independence. All seven were
  executed. Results are recorded per probe below.
- **Verdict**: **FAIL** — unanimous across all four reviewers.
- **Counts**: **P0 = 4**, **P1 = 17**, P2 = 9, P3 = 5.
- **Gate outcome**: **BLOCKED**. Not harvestable. Not mergeable.

### What revision 10 did close

Recording this first, because it is real and should not be relitigated in round 11.

- **The round-9 P0 (F-11) is closed.** All four reviewers independently confirmed it.
  U8 AC2 now derives merge-authority from the selected checkpoint's own validated
  `context.pr` / `context.branch`, explicitly prohibits `gh pr list --head`,
  `git branch --show-current`, and `git rev-parse --abbrev-ref` as safety-predicate
  inputs, and permits ambient branch data only for logging. No ambient-branch read
  survives anywhere as a predicate input. This probe passes cleanly.
- **The Ship Step 5 structural extraction is confirmed accurate.** An independent
  re-derivation from `.github/agents/_ship.agent.md` confirmed the duplicated item
  number `7` and confirmed that items 7b/7c currently precede the second item 7 and
  items 8/9/10. The structural claim is true.
- **Fail-closed behaviour on the API surface is correct.** Empty effective-rules
  response, 404, 403, rate limit, network error, schema error, and unencoded `/` all
  halt. An empty `[]` is correctly read as *uncovered*, not as *unrestricted*.
- **The F-09 splits are genuine seams, not relabelling.** U3→U3+U11, U4→U4+U12,
  U9→U9+U13 each divide along a real boundary (discovery/recovery, Step 5/Step 6,
  schema/discovery). Two reviewers confirmed this explicitly, while still finding the
  residual units oversized — see F-09 below.
- **Abandoned-ID hygiene is clean.** No unit, verification, or traceability row uses
  `143-F`, `143-S`, or `143.001-T`…`143.014-T` as a live implementation target. The
  plan remains harvestable into fresh IDs.
- **Frontmatter counts reconcile.** `task_count: 14`, `sub_epic_count: 3`
  (E1=5, E2=7, E3=2), `dependency_edge_count: 22` against the direct-edge list,
  acyclic with roots U0 and U2.

### P0 findings

#### F-01 (P0) — U13 installs a different Channel B protocol than the canonical block, and reinstates the discharge-rejection bug the canonical block exists to prevent

Found independently by three of four reviewers.

The canonical `RESOLUTION_OBLIGATION_RECORD` Channel B defines B0–B7 as: B1 routes
the *scan target* by PR state; B2 fetches the retained ref; B3 reads the tree
including `docs/archive/closure/`; B4 walks history; B5 binds transitions; B6 proves
ancestry. U13 AC3 installs a materially different sequence: B1 is an unconditional
`origin/main` tree scan, B2 is the ruleset proof, B3 is the fetch, B4 is a tree read
that **omits `docs/archive/closure/`**, B5 is history, B6 is binding. The plan header
asserts the protocol is "not restated differently anywhere." It is.

Worse, the canonical block splits provenance into PV-B4 (introduction) and PV-B6
(discharge) precisely because a `CLOSED` transition is committed on
`post-merge/{feature_slug}` while the record's embedded `pr` and `branch` still name
the *implementation* PR — the plan's own rationale states that a single "embedded `pr`
equals the PR it was read from" rule "structurally rejected every correct discharge."
U13 AC5 reinstalls exactly that rejected rule: PV-B4 requires `pr` to equal the PR the
record was read from, and PV-B6 requires `branch` to equal that PR's live
`headRefName`. Under U13 as written, POST-B can never pass provenance validation.

This is round-9 finding F-15 re-opened inside the unit that was created to fix it.
Because task cards are declared exact (V6), an implementer installs U13's text, not
the canonical text.

**Remediation**: U13 AC3 and AC5 must reference the canonical block by name and
reproduce it verbatim — including the `docs/archive/closure/` follow and the
introduction/discharge provenance split — or the canonical block must be deleted and
U13 made the single source. One of the two must go.

#### F-02 (P0) — U13 AC6 halts on the exact state the residual-window machinery was built to handle

U13 AC6 states that "an `OPEN` record whose PR has since merged is the 139-S shape and
halts." That is wrong. The 139-S shape is a resolution commit that is **not** an
ancestor of `main`. An `OPEN` record whose introducing commit *did* merge is the
normal RQ-7 interval: the merge succeeded and POST-B has not yet been written. The
canonical Channel B B1 routes merged PRs to `origin/main` for exactly this reason, and
the canonical `LAST_MILE_RECOVERY` merged + `RESOLUTION_PUBLISHED` row says to assert
ancestry against `origin/main` and then *continue the P-001 closure set*.

U13 turns the protocol's own happy-path interval into a permanent operator halt.

**Remediation**: Align U13 AC6 with canonical B1 and the LMR merged row — merged plus
`OPEN` plus introducing commit ancestor of `origin/main` continues to POST-B; halt only
when ancestry fails.

#### F-03 (P0) — The fetch and ancestry contract is self-contradictory; V22 and U11 cannot both pass

Found independently by two reviewers.

The canonical `LAST_MILE_RECOVERY` step 1b uses
`git merge-base --is-ancestor <sha> FETCH_HEAD`; the resolution-state classifier uses
`git cat-file -e FETCH_HEAD:<file>`; WP-3 and U11 AC9 use
`git fetch origin refs/pull/<pr>/head` with **no destination refspec**. Meanwhile U5
AC2 forbids bare `FETCH_HEAD`, hardening D11 and the D14 discharge both claim the
target is a unique retained ref, and V22 requires "zero occurrences of bare
`FETCH_HEAD`" and "zero fetches without a `:refs/autoharness/scan/pr-<n>` destination."

U11 is required to install the canonical block *and* to pass V22. Those outcomes are
mutually exclusive. Three different fetch/ancestry protocols are specified across the
canonical block, the units, the hardening document, and the verification suite.

This also means **hardening D14's discharge claim is factually false**: it asserts U11
fetches to a unique retained ref; U11 AC9 does not.

**Remediation**: One fetch form and one ancestry target, everywhere. Retained refs are
the correct choice (they are what makes the multi-candidate scan safe). Purge
`FETCH_HEAD` from the canonical `LAST_MILE_RECOVERY` block, WP-3, the classifier, and
U11 AC9; keep V22 as the enforcement grep.

#### F-04 (P0) — U0's ruleset, as specified, breaks branch cleanup and rebase repository-wide, and the plan does not acknowledge it

The ops prerequisite specifies `deletion` and `non_fast_forward` over
`refs/heads/feat/**`, `refs/heads/chore/**`, and `refs/heads/post-merge/**`, with
`bypass_actors: []` and `current_user_can_bypass: never`.

- The `deletion` rule in a GitHub ruleset forbids **any** deletion of a matching ref,
  merged or not. With no bypass actors, neither a human, nor Ship, nor GitHub's
  `delete_branch_on_merge` automation can delete a merged `feat/*`, `chore/*`, or
  `post-merge/*` branch. `gh pr merge --delete-branch` fails. Every branch ever created
  accumulates permanently. This directly contradicts `_ship.agent.md`'s own branch
  management rule, which deletes feature and closure branches after their PRs merge.
- The `non_fast_forward` rule makes every in-progress working branch strictly
  append-only from first push. Rebasing onto `main` to resolve conflicts, squashing WIP
  commits, and `git commit --amend` followed by `--force-with-lease` are all rejected by
  the forge. The repository's own `git-merge.instructions.md` prescribes rebase flows
  that this rule would break.

The plan treats force-push and deletion exclusively as adversarial erasure vectors. It
never enumerates the legitimate workflows they also serve, never mitigates them, and
its rollback statement ("the ruleset is re-configurable") is not a rollback procedure —
rolling the ruleset back immediately trips OB-8 and halts every subsequent agent
session.

This is a finding against the *operator-selected mechanism*, not merely against its
write-up. It is reported plainly per the directive's instruction to gate honestly.

**Remediation, in order of preference**:

1. **Narrow the ref scope.** The property actually needed is that the obligation
   record's introducing commit stays reachable. That does not require immutability of
   working branches. Pushing an immutable marker ref — `refs/autoharness/obligation/{shipment_id}`
   or an annotated tag — at S4 and protecting *that namespace* delivers the same
   reachability guarantee with near-zero blast radius. Working branches stay fully
   rebaseable and deletable.
2. If branch-level protection is retained, scope it to `post-merge/**` only (the branch
   that carries the `CLOSED` transition), and permit bypass for repository
   administrators and GitHub's branch-cleanup automation.
3. Whatever is chosen, U0 must carry the exact API payload, the exact rollback payload,
   and an explicit enumeration of the workflows it breaks, so the approving operator
   approves the real consequence rather than an abstract requirement.

### P1 findings

#### F-05 (P1) — Channel B candidate selection is circular: PV-8 cannot be evaluated from API fields alone

The candidate diagram applies PV-1, PV-2, PV-3 **and PV-8** "using API FIELDS ALONE"
before any body or tree is read. But PV-8 admits a record "naming shipment X" only if
the PR is shipment X's backlog-recorded implementation PR. The shipment name comes from
*inside the record*, which has not been fetched yet. The GitHub pull-request API returns
no shipment field. The prefilter as specified cannot run.

Secondarily, no live backlog field is named that would carry the "backlog-recorded
implementation PR" binding, and no unit populates one.

**Remediation**: Prefilter with PV-1…PV-3 only; fetch and parse; then apply PV-8 against
a specifically named, populated backlog field. Add the unit that populates it, or drop
PV-8.

#### F-06 (P1) — With PV-8 removed from the prefilter, the trusted candidate set becomes every PR in the repository, and OB-2/OB-8 then halt on ordinary contributor history

This is the operational consequence of F-05, and it is severe enough to stand alone.
Once PV-8 cannot prefilter, the trusted set is every same-repo open and closed-unmerged
PR. Then:

- Any open PR on a non-Ship branch (`docs/*`, `dependabot/*`, a hotfix) returns `[]`
  from the effective-rules probe, and B2 halts under OB-8.
- Any closed-unmerged PR whose head branch was deleted — GitHub's default behaviour —
  hits B1's "if the branch is gone, that is OB-2, not clean" and halts.

A single abandoned contributor PR permanently blocks every future agent session in the
repository. The mechanism fails closed so aggressively that it becomes a
denial-of-service on the harness itself.

**Remediation**: Bound the candidate set to PRs positively bound to active shipments
before OB-2 and OB-8 are armed. Coverage and branch-existence obligations must apply
only to PRs the protocol actually owns.

#### F-07 (P1) — An `OPEN` record on a PR that is closed without merging has no discharge path and deadlocks startup forever

POST-B can only run on the `post-merge/{feature_slug}` branch of a **merged** PR. If a
unit executes S4, publishes `OPEN`, and the PR is then abandoned — re-scoped, superseded,
or failed — there is no defined transition to `CLOSED`, no `ABANDONED` state, and no
authorized cancellation procedure. U13 AC6 classifies it as an unrecoverable orphan and
halts. Every subsequent session re-discovers it and halts again.

The four-row lifecycle (`none→none`, `none→OPEN`, `OPEN→CLOSED`) has no terminal state
for abandoned work.

**Remediation**: Add an explicit, authorized abandonment transition with its own
provenance requirements, or forbid publishing `OPEN` until merge is imminent.

#### F-08 (P1) — `RESOLUTION_POSTCONDITION` POST-B is ordered both before and after P-020 compaction

Found independently by two reviewers.

The canonical entry condition requires "the FULL required post-merge closure set" first.
POST-B's closure PR must be merged to discharge the obligation. Yet POST-B must precede
compaction. U12 AC5 says POST-B runs "only after the full P-001 post-merge closure set
is complete and verified, including the P-020 compaction record"; U12 AC6 says POST-B's
`CLOSED` write "completes before any compaction or archival that can move the closure
artifact."

Both cannot hold. Literal execution either writes POST-B after its carrier merged —
recreating the exact orphaning defect this plan exists to fix — or violates its own
entry condition. POST-A can also mark the locator `RECONCILED` before POST-B has merged.

Round-9 F-01 and F-07 are therefore only partially closed: the "commits nothing"
contradiction is gone, but the ordering contradiction it was entangled with is not.

**Remediation**: Fix one total order — POST-B commit, then closure-branch review, push,
merge, verify that merge, then P-020 compaction, then POST-A locator write, then final
P-001 completion. State it once, in the canonical block, and have U12 reference it.

#### F-09 (P1) — The P-020 compaction exclusion is declared on a surface that does not own candidate selection

P-020 makes `compact-context` the owner of compaction candidate selection. The plan's
changed-file set includes `operational-closure/SKILL.md` and the instructions file, but
**not** `.github/skills/compact-context/SKILL.md`. The path-stability invariant — that
`OPEN` and `none` artifacts are excluded from compaction candidates — is therefore
written where it is not enforced. An unmodified `compact-context` can archive an aged
`OPEN` closure artifact and move the record out from under OB-1.

**Remediation**: Bring `compact-context/SKILL.md` into scope as its own unit, or replace
the exclusion with an ordering guarantee that does not depend on candidate selection
behaviour.

#### F-10 (P1) — The history probes cannot detect the transitions they exist to detect

Found independently by two reviewers.

B4 uses `git log -S'resolution_obligation'`. Pickaxe `-S` detects a change in the number
of occurrences of the string. The key `resolution_obligation` is present in `none`,
`OPEN`, and `CLOSED` states alike, so its occurrence count does not change across
`none→OPEN` or `OPEN→CLOSED`. Neither transition is found.

U9 AC7 is worse: it searches `-S'"resolution_obligation": "OPEN"'` — JSON syntax —
against the YAML schema U9 AC1 installs (`resolution_obligation:` / `  status: OPEN`).
It can never match.

The canonical recovery procedure ("Recovering resolution commits without a SHA field")
does the right thing — walk `git log --reverse <scan_ref> -- <path>` and parse
parent/child YAML — but U9 and B4 install something else.

**Remediation**: One executable procedure: enumerate commits touching the artifact path
on the retained scan ref and parse the YAML `status` on each side. Delete both pickaxe
forms.

#### F-11 (P1) — Channel B cannot distinguish merged from closed-unmerged PRs with the fields it collects

B1 routes on `state == MERGED` versus `state == CLOSED, unmerged`. But U3's exact REST
projection collects `state` and not `merged_at`, and the GitHub pulls-list API reports a
merged PR as `state: "closed"` with a non-null `merged_at`. There is no `MERGED` value
to match. The router cannot decide between scanning `origin/main`, scanning a live head,
and reporting an unrecoverable orphan.

Additionally, `origin/main` is never explicitly refreshed before the merged-history scan,
so that scan can read a stale local ref.

**Remediation**: Add `merged_at` to the projection; define merged as `merged_at != null`;
add an explicit `git fetch origin main` before the `origin/main` scan.

#### F-12 (P1) — S3.5 permits bypass actors that U0 AC3 and RQ-13 forbid

Found independently by two reviewers.

RQ-13 and hardening D16 require "no bypass actors" and halt on "any bypass capability."
U0 AC3 requires `bypass_actors` to be empty. V21 and U13 use the strong wording. But the
canonical S3.5 predicate accepts `.bypass_actors is empty OR contains no actor the acting
identity holds, at any bypass_mode`.

An administrator who later adds a CI service account or an admin team to `bypass_actors`
leaves S3.5 passing while a third party retains the ability to force-push or delete the
branch — defeating the only property the ruleset was bought for.

**Remediation**: S3.5 must require `bypass_actors` length zero and
`current_user_can_bypass == "never"`. Align every restatement.

#### F-13 (P1) — No ruleset proof is performed on the `post-merge/*` branch that carries the `CLOSED` transition

U0 AC1 includes `refs/heads/post-merge/**` on the justification that this branch carries
the `OPEN → CLOSED` discharge. But S3.5 lives inside `RESOLUTION_PREFIX`, and the plan
states "no step of `RESOLUTION_PREFIX` may be performed on a merged branch; the prefix
ends before merge." U12 creates `post-merge/{feature_slug}` and commits POST-B without
ever invoking S3.5 or any equivalent probe on it.

The discharge commit is written to an unverified branch. Equally, merged records route to
`origin/main` with no runtime re-proof that `main` is still protected, so main-ruleset
drift is outside OB-8's implemented route.

**Remediation**: Require an effective-rules proof on `post-merge/{feature_slug}` before
POST-B is committed, and on `origin/main` before any merged-history scan is trusted.

#### F-14 (P1) — The S7 late-checkpoint loop is illegal under the record lifecycle

S7 sends a nonzero re-enumeration back through S3 → S3.5 → S4. After an earlier nonzero
S4, however, the record is already `OPEN`, and the lifecycle admits only `none→none`,
`none→OPEN`, and `OPEN→CLOSED`. There is no `OPEN→OPEN` amendment, and the locator would
have to regress from `RESOLUTION_PUBLISHED` back to phase 1, which the three-phase
locator forbids.

The advertised race recovery is therefore executable only when the original count was
zero — the case that needed it least.

**Remediation**: Either halt on late checkpoints discovered after a nonzero S4, or define
an explicit, auditable `OPEN` amendment transition and a locator re-publication rule.

#### F-15 (P1) — Stage cannot satisfy U8's mandatory carrier binding, because Stage checkpoints predate the PR that carries them

U8 makes `context.pr` and `context.branch` mandatory at every Stage checkpoint creation
and prohibits checkpoint creation without an open staging PR. But live Stage creates
checkpoints throughout its session (Session-continuity, mid-session checkpoints), Stage's
own role boundary forbids it from creating or pushing PRs, and the Orchestrator creates
the staging PR only after Stage completes. A mid-session Stage checkpoint cannot know its
future PR number. U8 nevertheless asserts that no other Stage behaviour changes.

Separately, even when an open PR does exist, the one-time `state == OPEN` read is a TOCTOU:
the PR can merge between the read and the resolve.

Note that the **round-9 P0 itself is closed** — the predicate no longer reads the ambient
branch. This finding is about whether the replacement binding can be produced at all.

**Remediation**: Name who produces and durably backfills the carrier binding and when;
reconcile with Stage's mid-session checkpoint obligations; add a final live-state plus
ancestry guard immediately before resolve.

#### F-16 (P1) — The "standing executable" drift check V20 has no implementation and cannot run inside markdownlint

Found independently by two reviewers.

Round-9 F-08 required an executable drift check across the duplicated protocol surfaces.
V20 claims semantic role extraction and comparison "run as part of V1's lint pass." V1 is
markdownlint. The plan's own constraints forbid adding scripts. No unit adds a checker or
a lint rule. V20 is therefore another one-time manual instruction wearing the label of a
standing check.

V20 also conflicts with U4 AC1, which requires Ship's Step 5 not to restate "any of its
steps, orderings or rationale" — if Step 5 does not state an order, V20 has nothing to
extract from it; if it does, AC1 fails.

**Remediation**: Either bring a real checker into scope and wire it to an existing gate, or
delete V20 and record the drift exposure honestly as a residual risk. Resolve the AC1/V20
conflict either way.

#### F-17 (P1) — Several verifications are unsatisfiable against a correct implementation

- **V8** requires every enumerated PR to have a non-empty body; **V17** deliberately
  empties one, and the live repository currently contains an empty-body PR. V8 as written
  fails on correct behaviour.
- **V14** asserts "the locator, the resolutions and the `OPEN` obligation record ride one
  commit." The locator is PR-body metadata, published before and after that commit; it
  does not ride a commit at all.
- **V23** asserts no mutation after the S5 review except POST-A/POST-B, while live Step 6
  performs backlog archival, closure generation, documentation updates, compound refresh,
  compaction, and index sync.
- **V19** parses "the schema block and its lifecycle text" as YAML; the lifecycle is a
  markdown table and the schema uses `none | OPEN | CLOSED` as a prose union. Neither
  parses.

**Remediation**: V8 → assert body-field presence, not non-emptiness. V14 → separate
PR-body metadata from commit contents. V23 → scope to the implementation branch and the
review cycle it belongs to. V19 → parse the schema mapping only, or delete.

#### F-18 (P1) — U4 AC6 forbids post-S4 mutation that the canonical gate-failure loop requires

U4 AC6 states "no branch-mutating step remains after S4" and that every mutating item
sits in S1 or S2. The canonical gate-failure loop pushes a remediation commit *on top of*
the resolution commits and re-enters at S5. Step 5 item 2 (the session-memory commit) is
mutating and is not mapped into S1 or S2 in U4's extract.

**Remediation**: Scope AC6 to the ordinary path, name the remediation-loop exception
explicitly, and map item 2.

#### F-19 (P1) — The Constitution Check maps a constitution this workspace does not have

Found independently by two reviewers.

The plan's "Principles I–XI" table lists *Specification before implementation, Single
source of truth, Traceability, Incremental delivery, Test/verification first,
Reversibility, Fail closed, Least privilege, Auditability*. The actual
`.github/instructions/constitution.instructions.md` principles are *Safety-First Rust,
Test-First Development, Workspace Isolation and Security Boundaries, CLI Workspace
Containment, Structured Observability, Single Responsibility, Git-Friendly Persistence,
Agent Context Efficiency, Merge Commit History Preservation*. Only VII and VIII coincide.

Round-9 F-27 asked for the workspace constitution's own principles to be mapped.
Revision 10 answered it by inventing a plausible-sounding different rubric. F-27 is not
discharged, and genuinely relevant principles go unchecked — notably **IV (CLI Workspace
Containment)**, which U0 violates most directly by reaching outside the workspace to
mutate forge settings.

**Remediation**: Map the real Principles I–XI, marking genuine N/A explicitly, and add a
real check for IV against U0.

#### F-20 (P1) — P-019 states that the harness never edits rulesets; U0 is not reconciled with it, and P-019 is cited nowhere

`workflow-policies.md` P-019 states verbatim that ruleset configuration "remains an
operator configuration action — the harness never edits rulesets." U0's ProposedAction box
says "the **implementing agent** MUST obtain explicit operator/admin approval immediately
before **applying this change**," implying an agent applies it. P-019 appears in none of
the three artifacts' `policies:` frontmatter and in no Constitution Check row.

This compounds with a least-privilege problem: creating a ruleset requires
`Administration: write`. If Ship applies U0, Ship must hold an admin token — a
significant and unnecessary privilege escalation for a documentation harness. If the
operator applies it, U0 is a human prerequisite, not one of fourteen agent tasks, and it
must carry the exact payload the operator will execute.

**Remediation**: Add P-019 to the Constitution Check. Reclassify U0 explicitly as a manual
operator prerequisite performed outside the agent pipeline, with its exact create payload
and exact rollback payload inline. Forbid any agent from holding ruleset-write credentials.

#### F-21 (P1) — The Definition of Done can be satisfied while the load-bearing prerequisite is absent

Found independently by two reviewers.

Decision §7 permits the ruleset to have "been explicitly deferred with the consequence
recorded." But S3.5 halts before obligation publication on an uncovered branch, and the
plan's own measured evidence shows Ship source branches return `[]` today. Deferring U0
and shipping U1–U13 yields a release that is "done" while every nonzero-checkpoint unit
halt-loops at S3.5.

This also collides with the traceability contract's "exactly one owner per requirement":
RQ-13 is assigned to "U0 + U2."

**Remediation**: DoD must require U0 applied and V21 green, or require the S3.5/RQ-13
durability claims to be withdrawn. Remove the "deferred but done" branch. Assign RQ-13 one
owner and classify the other as enforcement.

### P2 findings

#### F-22 (P2) — Same-file units are not fully serialized, and the count is wrong

The file-serialization paragraph says "Four units share
`github-pr-automation.instructions.md`" and then lists five (U2, U3, U11, U9, U13),
calling them strictly sequential. The edge list has `U3→U11` and `U3→U9→U13` but neither
`U11→U9` nor `U11→U13`. The claim that U11/U13 is the only pair without a direct edge is
also false — U11/U9 lack one too. Dependency-driven execution could edit the canonical
file concurrently.

#### F-23 (P2) — U0's branch globs are stated two incompatible ways

The `BRANCH_PROTECTION_PREREQUISITE` table uses `feat/*`, `chore/*`, `post-merge/*`; U0
AC1 uses `refs/heads/feat/**`, `refs/heads/chore/**`, `refs/heads/post-merge/**`. In
GitHub ruleset `ref_name.include`, `*` and `**` are not equivalent, and the `refs/heads/`
prefix changes matching. One include list, derived from `_ship.agent.md` with line
citations, must be used by U0, S3.5, and V21 alike.

#### F-24 (P2) — Channel B has no adoption boundary for pre-existing closure artifacts

B3 halts on any closure artifact whose schema is invalid, while OB-5 exempts only
artifacts "created after S2" — a test that is never made deterministic. The working tree
currently contains 177 closure and archive artifacts, none of which carry
`resolution_obligation`. A literal implementation either halts on all of them or exempts
them by an unspecified rule.

#### F-25 (P2) — The P-009 guardrail is described as unmodified and as amended

The P-018 row says "the merge-commit guardrail item is retained unmodified" and S8's tail
says the P-009 item "applies unchanged," while U4 AC10 and V4(f) both add an API-side
merge-commit-mode check and a two-parent assertion to that same item. This is the same
defect class as round-9 F-02 (the "item 15 unmodified" contradiction), which revision 10
swept for — and missed in its item-16 form.

#### F-26 (P2) — The drift surface is five layers deep and V20 covers one of them

One protocol rule now lives in the canonical block, in unit acceptance criteria, in
verification expected-results, in hardening D1–D16, and in the decision's RQ table.
Changing a single rule — "do not use `FETCH_HEAD`" — already requires six edits, and the
canonical/unit disagreements catalogued in F-01, F-03, F-10 and F-12 are direct products
of this. V20 compares only S1–S8 role ordering and would have caught none of them.

#### F-27 (P2) — U4 AC3's zero-checkpoint omission list is narrower than the canonical rule

The canonical rule omits segments S3.5 and S4 in their entirety for a zero-checkpoint
unit. U4 AC3 enumerates five specific omissions and misses the `OPEN` record write and the
remote-head proof.

#### F-28 (P2) — The Step 5 extract is a normalized mapping presented as verbatim

The structure is confirmed accurate, but the extract states that item 15 re-fetches "only"
the P-018 verdict and `headRefOid`; the live item 15 also conditionally re-runs §1.9 when
HEAD has changed. The table also annotates item 2 as "yes (commit)" where live item 2 says
only to write the memory file. Both are inferences, so "verbatim" overclaims.

#### F-29 (P2) — Rollback of U0 is undefined and self-trapping

U0's reversibility note observes that the ruleset is re-configurable, but there is no
rollback payload and no procedure for unwinding in-flight units. If U0 must be rolled back
because of F-04, in-flight sessions halt at S7 and every future startup halts at B2 under
OB-8, with no defined recovery.

#### F-30 (P2) — The repository-supported ruleset fallback path is unpaginated

The effective-rules probe is correct, but the `GET /rulesets` fallback is issued without
`--paginate`. Truncation produces a false halt. This is the same defect class as round-9's
V8 pagination finding, in a new location.

### P3 findings

- **F-31 (P3)** — The Constitution Check row is titled "P-011 Worktree topology"; P-011 is
  actually named "Branch-Before-Mutation." The substance attached is right; the label is wrong.
- **F-32 (P3)** — U12's header says "Depends on: U4, U9, U10" but the 22-edge list contains
  only `U4→U12`; U9 and U10 reach U12 transitively. Either add the direct edges and update the
  count, or annotate the transitivity.
- **F-33 (P3)** — U0 declares `action_class`, `ActionRisk`, `approval_required`, approver,
  surface, and reversibility, but never the `ActionResult` state the Action Contract expects
  (`planned` at plan time).
- **F-34 (P3)** — Revision 10 repeatedly describes round 9 as "15 P1"; the retained round-9
  block contains 18 P1 headings. It also calls the seven-persona panel "five-persona" in one
  place.
- **F-35 (P3)** — Hardening D1–D4 cite stale U2/U4 criterion numbers after the revision-10
  renumbering (for example, phase-1 timing is U2 AC7, not AC5; non-self-reference is AC10, not
  AC3/AC8).

### Results of the seven mandated probes

| Probe | Result | Basis |
|---|---|---|
| Channel B with an emptied PR body | **FAIL** | The candidate set is genuinely body-independent — that part works. But PV-8 cannot prefilter (F-05), the canonical and U13 algorithms differ (F-01), the transition probes cannot match (F-10), merged/closed-unmerged cannot be distinguished (F-11), and `origin/main` is never refreshed. The procedure is not executable end to end. |
| Ruleset drift and bypass | **FAIL** | `[]` is correctly read as uncovered and the API surface fails closed. But S3.5 permits non-acting bypass actors that RQ-13/U0 AC3 forbid (F-12), no proof runs on `post-merge/*` or `origin/main` (F-13), and an admin can disable, rewrite, and restore protection between observations. |
| Zero-checkpoint vs nonzero | **PARTIAL / FAIL** | The zero path is now genuinely satisfiable: four always-present HEAD values, vacuous-safe ancestry, a complete-zero S7 result, and explicit POST-A/POST-B no-ops on `none`. Round-9 F-04/F-05 are closed. The **nonzero** path is not satisfiable: POST-B is ordered both before and after compaction (F-08), and a late checkpoint after a nonzero S4 cannot legally re-enter S4 (F-14). |
| P-020 ordering | **FAIL** | F-08 (contradictory order) and F-09 (exclusion declared on a surface that does not own candidate selection). V23 also excludes Step 6 mutations that genuinely occur (F-17). |
| Exact Ship Step 5 extraction | **PARTIAL** | Structure independently re-derived and confirmed: the duplicated item `7` is real, and 7b/7c do precede the second 7 and 8/9/10. The "item 15 re-fetches only…" claim and the item-2 commit annotation are inferences, so the "verbatim" label overclaims (F-28). |
| D14 discharge | **FAIL** | U11 AC9–AC12, U5 AC9 and V12/V12a–g correctly invoke the named procedure rather than restating it — that structural requirement is met. But WP-3 and U11 AC9 still fetch without a destination refspec and assert against bare `FETCH_HEAD`, contradicting V22 and falsifying D14's own discharge claim (F-03). |
| P-010 ambient-branch independence | **PASS** | Unanimous. U8 AC2 keys off the checkpoint's validated `context.pr`/`context.branch`, prohibits `gh pr list --head`, `git branch --show-current`, and `rev-parse --abbrev-ref` as predicate inputs, and confines ambient branch data to logging. The round-9 P0 is genuinely closed. The separate question of whether the binding can be produced is F-15, not a regression of F-11. |

### Reviewer convergence

Findings reached independently by more than one reviewer, with no shared context, and
therefore carrying the highest confidence:

| Finding | Reviewers | Models |
|---|---|---|
| F-01 Channel B canonical/U13 divergence | 3 of 4 | GPT-5.6 Sol, Grok 4.6, Gemini 3.8 Flash |
| F-03 `FETCH_HEAD` contract unsatisfiable | 2 of 4 | GPT-5.6 Sol, Grok 4.6 |
| F-08 POST-B ordering contradiction | 2 of 4 | GPT-5.6 Sol, Claude Opus 4.8 |
| F-10 pickaxe probes cannot match | 2 of 4 | GPT-5.6 Sol, Grok 4.6 |
| F-12 S3.5 bypass weaker than RQ-13 | 2 of 4 | GPT-5.6 Sol, Gemini 3.8 Flash |
| F-16 V20 is not a standing check | 2 of 4 | GPT-5.6 Sol, Grok 4.6 |
| F-19 fabricated constitution mapping | 2 of 4 | Claude Opus 4.8, Grok 4.6 |
| F-21 DoD permits a deferred prerequisite | 2 of 4 | GPT-5.6 Sol, Grok 4.6 |
| Residual oversizing of U2/U4/U11 | 3 of 4 | Claude Opus 4.8, Grok 4.6, GPT-5.6 Sol |
| P-010 probe passes | 4 of 4 | all |

### The structural judgement, stated plainly

Two reviewers reached the same conclusion by different routes, and it is the most
important thing in this review.

The defect this plan exists to close — Defect-2 — is that Ship resolves checkpoints
*after* the carrying PR merges, orphaning the resolution commit on a dead branch. The
load-bearing fix is small and is already fully specified in this plan: move resolve before
merge, push it, re-run the review at that HEAD, assert ancestry at merge, and forbid
post-merge resolution in both Ship and Stage.

Everything built on top of that — the obligation record, Channel B's B0–B7, PV-1…PV-8,
OB-1…OB-8, `LAST_MILE_RECOVERY`'s WP-0…WP-7, the seven-part S8 bar applied to every
merge including zero-checkpoint chores, and now U0's repository-wide ruleset — exists to
*detect* a residual window rather than to *prevent* the defect. And by the plan's own V17,
the detection path terminates in a halt: it does not recover, it fail-closes. The
machinery buys detection of PR-body deletion and force-push at the cost of four P0s, a
repository-wide workflow regression, and a startup path that halts on ordinary contributor
history.

Ten revisions of escalating residual-window machinery around a one-step ordering bug is
itself the finding. Round 11 should not attempt to repair thirty findings in place. The
recommendation from this panel is to take the smaller design back to the operator:

1. Keep the ordering fix, the locator, the merge-authority bar, and the ancestry assertion.
   These close Defect-2 and are the parts every reviewer found sound.
2. Replace the branch-ruleset prerequisite with an immutable marker ref or annotated tag
   pushed at S4 and protected in its own namespace. This delivers the same reachability
   property with near-zero blast radius, leaves working branches rebaseable and deletable,
   and avoids the P-019 conflict and the admin-token requirement entirely.
3. Demote Channel B, the obligation record, and the OB/PV invariant sets to an explicitly
   optional hardening layer scoped to PRs bound to active shipments — or drop them from this
   unit and record the residual honestly.

That is a recommendation to the operator, not a decision taken by this review. The gate
outcome stands on the findings regardless of which design is chosen.

### Gate

**BLOCKED.** 4 P0 and 17 P1 findings. The plan is not harvestable, and this PR is not
mergeable in its current state. No harvest, no shipment claim, no activation, and no merge
was performed. The eighteen unresolved Copilot review threads on PR #396 remain
unresolved by deliberate instruction; no comment cycle was started in this round.

<!-- plan-review-attempt: 5 -->
