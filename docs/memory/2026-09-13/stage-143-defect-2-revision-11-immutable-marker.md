---
date: 2026-09-13
agent: stage
session: stage-143-defect-2-revision-11
phase: plan-revision
historical_shipment: 143-S
pr: 396
status: revision-pushed-awaiting-round-11-review
---

# Stage — Defect 2 plan revision 11 (protected immutable marker)

## Mandate

Third and final bounded remediation/review attempt on PR #396 under an
operator-authorized outcome cycle. Revisions 9 and 10 both FAILED independent
cross-model plan review (round 9: 1 P0 / 15 P1; round 10: 4 P0 / 17 P1,
unanimous across four reviewers). The Orchestrator authorized **replacing**
revision 10's branch-history / closure-record machinery with an
architecture-validated **protected immutable annotated-tag** design, pushing one
coherent revision 11, and running one fresh full plan review.

Gate rule for round 11: any P0/P1 ⇒ **FAIL, open the circuit**. P2-only ⇒
ADVISORY. None ⇒ PASS. Never harvest, claim, activate, implement, mutate GitHub
settings, reply to or resolve GitHub threads, or merge in this cycle.

Resolved escalation route: `claude-opus-5` / `anthropic` / `high`.

## Authorized RQ-7 clarification (recorded explicitly)

The immutable marker tracks **checkpoint-resolution delivery to the protected
default branch only**. Its status derives **solely** from marker-target ancestry
to `origin/main`.

Full P-001 release closure remains governed **independently** by existing
shipment state, the closure PR and its artifacts, knowledge graduation,
runtime/operational closure, and P-020 compaction. **A discharged marker never
authorizes the next shipment and never proves full release closure.**

This is **not** permission to weaken P-001, P-009, P-014, P-018, P-020 or P-022.
Recorded in the plan (§ *Authorized RQ-7 clarification*), the decision
(§ *Revision 5*), and hardening D22.

## U0 risk and approval boundary (recorded explicitly)

U0 configures the marker tag ruleset. It is a `ProposedAction` with
`ActionRisk: high` and `approval_required: true`. It is **never** agent-executed
and **never** dark-mode-approved — it is a P-019 manual operator/admin step.
Definition of done requires U0 **applied and verified**; the revision-10 option
to defer it while calling the feature complete is **removed** (round-10 F-21).
The current planning PR does **not** apply it.

## Architecture (replacement, not repair)

* **Primary key** = (immutable numeric repo ID, committed workspace ID,
  shipment ID). Deterministic ref
  `refs/tags/resolution-obligation/v1-<primary-key-digest>`.
* **Payload** = canonical UTF-8 JSON in the **annotated-tag message**, sorted
  keys, LF, no BOM, domain-separated SHA-256 digest excluding itself.
* **`C` is deliberately absent from the payload** — RQ-8 non-self-referentiality
  is satisfied by the tag object header pointing at `C`.
* **Status is derived, never stored**: `git merge-base --is-ancestor C
  origin/main` ⇒ `DISCHARGED`, else `PENDING`.
* **Last mile requires `C` ancestor of `H`**, not `C == H`, so review
  remediation can append commits above `C`.

Eight units, eleven dependency edges, three sub-epics, twelve backlog IDs at
harvest. Roots U0 and U1; acyclic. Every same-file pair carries a direct edge
(U1/U2 share the instructions file; U4/U5 share the Ship file), closing
round-10 F-22 without a harvest-time sequencing note.

## Empirically validated Git semantics (load-bearing plan claims)

Validated before writing the revision, in three throwaway local bare-repo
experiments. These are recorded in V7/V8/V9 and hardening D17/D18/D19.

1. `--force-with-lease=<ref>:` with an **empty** expected value asserts the ref
   **does not exist**. Absent tag ⇒ push succeeded; different tag present ⇒
   rejected with `stale info`.
2. `--atomic` makes either-ref rejection leave **both** refs unchanged, exit 1:
   stale branch lease produced `HEAD -> main (stale info)` **and**
   `<tag> (atomic push failed)` with the tag not created; a competing different
   tag produced `<tag> (stale info)` **and** `HEAD -> main (atomic push failed)`
   with the branch not advanced.
3. Annotated-tag ref OID ≠ commit: `ls-remote` shows `<T> refs/tags/<tag>` and
   `<C> refs/tags/<tag>^{}` as **distinct** OIDs.
4. **Gotcha**: when the remote tag OID is byte-identical to local `T`, Git
   classifies the refspec as already-up-to-date and **does not evaluate the
   lease**; the atomic push then proceeds for the branch alone and reports
   success. This is the idempotent lost-push-response path, safe **only**
   because the preconditions already proved byte-identical canonical presence.
   U1 AC11 and D19 forbid inferring identity from the push's silence.

## GitHub ruleset facts probed (read-only)

* Repo `id: 1150361017`, `node_id: R_kgDORJEduQ`, `default_branch: main`,
  `allow_merge_commit: true`, `allow_squash_merge: false`,
  `allow_rebase_merge: false`, `delete_branch_on_merge: false`.
* Only ruleset is `PR-Required` (id `12812291`), `target: branch`, active,
  rules `deletion`, `non_fast_forward`, `pull_request`, `copilot_code_review`;
  `bypass_actors: []`, `current_user_can_bypass: "never"`. **No tag ruleset
  exists** — U0 creates it.
* Effective rules for `main` confirm the protected-default-branch premise.
* Remote already carries unrelated `rescue-primary-stash-*` and `v0.x` tags,
  substantiating the exact-prefix filter in U2.

### The U0 tag-rule correction

`non_fast_forward` is a **branch** force-push rule; applied to a tag it would
only reject non-fast-forward movement, permitting a fast-forward tag retarget
along the same lineage. The load-bearing rule for tag immutability is
**restrict updates** plus **restrict deletions**. **Creation must NOT be
restricted** — Ship must publish new markers without bypass rights.

GitHub exposes **no** per-tag effective-rules endpoint (only
`GET /repos/{owner}/{repo}/rules/branches/{branch}`), so the proof is a
configuration read **by recorded ruleset ID**, asserted field by field, with an
operator UI check as a second witness. Recorded honestly as **RR-3b**; not
directly API-verified this session because verification would require a
mutating POST, which is forbidden.

## Satisfiability corrections made during validation

Round-10 F-17 was an unsatisfiable verification. Two rows in this revision had
the same latent defect and were rewritten to **classify rather than count**:

* **V25** — a raw grep for deleted terms necessarily hits deletion tables,
  prohibition statements (`Forbids FETCH_HEAD`, `non_fast_forward is NOT the
  load-bearing rule`), the V25 row itself, retained history, **and** the
  decision's option labels `B1`–`B5`, which are an older, unrelated namespace
  colliding with round-10's `B0`–`B7` Channel-B steps. Five permitted hit
  classes are now enumerated; the sweep was run and every hit classified.
* **V27** — "zero occurrences of all eleven" was falsified by the V27 row
  itself and by the decision's Defect-1 *Out of scope* enumeration. Rewritten
  with two permitted classes.

Also corrected: the decision said "**Four files**" while listing five; a
high-complexity de-risking declaration was added for U1/U2/U4; and `RR-2` is
now explicitly recorded as intentionally retired (it cited unit `U8`, which
revision 11 does not contain).

## Validation results

* `markdownlint-cli2` over all three artifacts: **0 issues**.
* Frontmatter parses as YAML in all three; plan/hardening at `revision: 11`,
  decision at `revision: 5`; `harvest_authorized: false`;
  `proposed_action_units: [U0]`.
* 8 units, 11 edges, **acyclic**, roots `U0`/`U1`.
* V1–V27 contiguous; zero dangling `U`/`V` cross-references in plan, hardening
  or decision.
* Every unit touches exactly one file (U0 touches none) and carries both `Size`
  and `Complexity`.
* `git status` shows **only** the three artifacts plus this memory file. No
  installed file, workflow file, task card, source file or repository setting
  was touched.

## Unresolved GitHub review threads

PR #396 carries **27** unresolved review threads (the mandate said 21 — the
count grew). All authored by `copilot-pull-request-reviewer`. They were
**enumerated read-only** as review input. **None** was replied to or resolved,
per the mandate.

## Historical evidence preserved

Plan lines 1148+ (`## Retained review history`) and hardening lines 490+
(`## Retained hardening history`) are preserved byte-for-byte from revision 10
under explicit **SUPERSEDED** banners. `143.*` identifiers remain abandoned
historical evidence and must not be reused at harvest.

## Next steps

1. Round-11 full plan review (cross-model, all required personas).
2. Append the review, update verdict frontmatter, commit and push.
3. Update the PR body `Reviewed HEAD` after the push.
4. Any P0/P1 ⇒ FAIL and open the circuit. Never harvest or merge in this cycle.
