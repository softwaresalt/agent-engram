---
title: "Content-addressed generation reuse for Engram daemon cold start"
description: "Adopt shared immutable content-addressed generations with a private per-opener branch overlay, reusing the already-shipped 136-S generation subsystem, instead of per-branch mutable Cozo databases."
topic: "engram-daemon-cold-start-storage-model"
depth: "deep"
decision_status: "decided"
promoted_to: "both"
linked_artifacts:
  - "docs/decisions/2026-09-10-engram-cold-start-readiness-spike.md"
  - "docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md"
  - "144-F"
  - "144-S"
  - "002-SP"
tags:
  - "daemon"
  - "cozo"
  - "cold-start"
  - "generations"
  - "content-addressing"
  - "release-blocker"
---

# Decision — Content-addressed generation reuse for Engram daemon cold start

* **Date:** 2026-09-10
* **Status:** Accepted (operator-directed conceptual direction, confirmed by evidence)
* **Implementation planning status:** **DEFERRED — and the derived plan has since
  TERMINALLY FAILED review** (see the terminal note below)
* **Drives:** `002-SP` (critical), cold-start/readiness release blocker
* **Source spike:** `docs/decisions/2026-09-10-engram-cold-start-readiness-spike.md`
* **Author:** Stage agent (planning only)

## Terminal plan-review note (added 2026-09-10, Revision 3 terminal FAIL)

**The architecture direction recorded in this decision is UNCHANGED and remains
ACCEPTED. Only the implementation plan derived from it failed.**

Nothing in this note alters, retracts, or reopens the accepted content-addressed
generation direction. `decision_status` remains `decided`.

The third and final independent `plan-review` gate on
`docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md` returned a
**terminal FAIL** (see that plan's `## Plan Review — Revision 3`). Plan hardening
was required and present, but the plan's **executable contracts remained unsafe
or unimplementable** after three cycles. The review-cycle circuit breaker is
**open**.

Consequences for this decision's downstream work:

* The measurement shipment `144-S` did **not** reach a claimable state. Every one
  of its manifest members, including `144-F`, is now **blocked**. The measured
  report this decision was waiting on has therefore **not** been produced.
* `144-S` and `143-S` remain `queued` only because the shipment lifecycle has no
  blocked state. Neither is claimable; intake must fail because their manifest
  members are blocked.
* The 28 deferred Phase 2 items remain blocked and retained under `144-F`,
  exactly as described in the section below. Their disposition is unchanged.
* The precondition stated below — that a future Stage plan may be written only
  after `144-S` reports and only if it answers Q1–Q7 — **still stands**, and now
  additionally requires an **explicit new operator-authorized Stage cycle**
  before any planning resumes.

## Implementation planning deferred (added 2026-09-10, Revision 3 scope reset)

**This decision remains ACCEPTED as architecture direction. It is not an
implementation plan, and no implementation plan may be written from it until the
measurement shipment reports.**

The Revision 2 `plan-review` gate on
`docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md` FAILed
because the generation-reuse work was **under-measured and over-designed**: the
plan specified eight implementation units in depth against a cost premise — that
HNSW index construction inside `schema_bootstrap` dominates cold-start cost —
that has never been directly observed, while the design questions determining
whether that specification is correct remained open.

Consequently:

* `144-S` was narrowed to a **measurement and observability shipment only**. It
  ships no cold-start fix; its product is a measured report that returns to
  Stage.
* The 28 implementation items derived from this decision (`144.003-T` …
  `144.008-T` and their subtasks) are **blocked and deferred**, removed from
  every shipment manifest and from queue ordering, and stripped of all
  dependency edges. They are **retained** under `144-F` as traceable follow-on
  candidates — not archived, not deleted, and **not claimable**.
* A future Stage plan may be written only after `144-S` reports, and only if it
  answers the mandatory design inputs **Q1–Q7** recorded in the plan's
  *Deferred Phase 2* section: catalog vs active pointer; clean-candidate
  producer and lifecycle; serving copy vs snapshot source; attach integrity
  preconditions; runtime-copy lease; publication atomicity and quiescence
  authority; and the shared reader/GC pin closing the GC TOCTOU window.

The recommendation below — content-addressed generations reusing the shipped
`136-S` subsystem — is still Stage's recommended direction. Nothing in this
document authorises implementation.

## Context

A newly created feature branch whose source tree is byte-identical to its
parent pays a full Cozo cold start. Measured: daemon PID `30528` ran ~2.5
hours, burned 72 CPU-minutes at ~198 % of one core, held 1.84 GB, grew its
branch DB 7.4 MB → 99.9 MB, and never reached readiness. Three readiness checks
failed and the circuit breaker opened.

Storage is keyed by branch name alone
(`{data_dir}/cozo/{branch_safe}/engram.db`), so 45 branch databases now consume
2.29 GB of largely redundant content, and `main`'s already-computed index
cannot pre-warm a branch created from it.

**Rejected up front (operator decision, and already recorded in `002-SP`):**
raising `ENGRAM_READY_TIMEOUT_MS`. At ~2.5 hours and climbing, no timeout value
helps; the work must leave the readiness path.

## Options considered

### Option 1 — Status quo: per-branch mutable database (baseline)

Keep `{data_dir}/cozo/{branch}/engram.db`.

* **Cold start:** full bootstrap + index build per branch. Unbounded.
* **Reuse:** none. Identical trees share nothing.
* **Storage:** O(branches × full index) — measured 2.29 GB / 45 branches.
* **Isolation:** good (physically separate files).
* **Verdict:** **Rejected.** This is the defect.

### Option 2 — Safe parent DB snapshot copy (copy parent's DB on branch create)

On branch creation, copy the parent branch's database directory.

* **Cold start:** fast when the copy is valid — no index rebuild.
* **Reuse:** by *branch lineage*, not by content. Copies from a parent whose
  tree has since diverged are silently wrong, and there is no digest to detect
  it.
* **Storage:** still O(branches × full index); a copy is a full duplicate.
* **Correctness:** no identity binding. Cannot answer "is this index valid for
  this tree?" Requires a separate invalidation mechanism that does not exist.
* **Crash safety:** a partially copied DB is indistinguishable from a complete
  one without a digest.
* **Verdict:** **Rejected.** Fixes latency, not correctness; keeps the storage
  blow-up; introduces a silent-staleness class that is worse than slowness.

### Option 3 — One mutable multi-branch database

Single database holding all branches, rows tagged by branch.

* **Cold start:** fast — one warm DB, opened once.
* **Storage:** best — shared structure, no duplication.
* **Isolation:** **poor.** Every query must filter by branch; a missing filter
  is a silent cross-branch leak. HNSW vector indexes are global structures and
  cannot be cheaply partitioned per branch, so ANN results would mix branches
  unless post-filtered — degrading both correctness and recall.
* **Concurrency:** reintroduces the SQLITE_BUSY contention class that
  `connect_db`'s lock ladder, `open_db_with_retry`, and `catch_busy_panic`
  exist to contain (`U015-FLK1`). All branches would now contend on one file.
* **Blast radius:** one corrupt DB takes out every branch.
* **Verdict:** **Rejected.** Isolation and ANN-correctness costs are
  disqualifying, and it regresses a hard-won concurrency fix.

### Option 4 — Shared immutable content-addressed generations + branch overlay ✅

Index content is stored once per **content identity**, immutably, and branches
*reference* a generation rather than owning a database. A branch whose tree
matches an existing generation attaches to it instantly (copy-on-open into a
private runtime copy); a branch that diverges builds a new generation off the
readiness path.

* **Cold start:** O(1) attach for a matching tree — the pre-synced-parent case
  becomes a manifest lookup plus a runtime copy, not an index build.
* **Reuse:** by **content**, so it is correct by construction: identical trees
  provably share an index; divergent trees provably do not.
* **Storage:** O(distinct content states), not O(branches). Collapses the
  observed 45-way duplication.
* **Isolation:** preserved — each opener gets a private runtime copy; immutable
  generations are never written in place, so there is no cross-branch write
  path.
* **Crash safety:** atomic publication (`active.json` + `.publisher.lock`) makes
  a torn generation unpublishable and therefore invisible.
* **Cost:** requires a well-specified identity key and a GC story.
* **Verdict:** **Accepted.** Matches the operator's preferred direction and is
  the only option that fixes latency, storage, *and* staleness correctness
  together.

### Decisive implementation factor

Option 4 is **already substantially built** but unwired. Shipment `136-S`
landed `src/services/generations/` with `GenerationStore`, `GenerationId`,
`GenerationRevision`, `GenerationManifest` (carrying `WorkspaceIdentity`,
`BranchIdentity` with `source_revision`, `SealedInventory` of per-file SHA-256
digests, `GenerationProvenance`, `schema_version`), atomic publication via
`active.json` + `.publisher.lock`, and
`open_existing_generation_via_runtime_copy` / `publish_runtime_copy` in
`src/db/cozo_backend/mod.rs`.

The gap is narrow and specific: **`connect_db` still resolves storage by branch
name and never consults `GenerationStore`.** Option 4 is therefore the
*cheapest* remaining option as well as the most correct.

## Decision

Adopt **Option 4**. Reuse the existing generation subsystem; do not build a new
storage model.

## Recommended identity key

A generation is addressed by the SHA-256 of a canonical, versioned tuple. All
five components are **required** — omitting any one produces a false-positive
reuse:

```
generation_id = sha256(canonical_encode(
    content_key_kind,       // discriminator: "git-tree-v1" | "inventory-v1" | "multi-root-v1"
                            // included IN the hash so a fast-path key can never
                            // collide with a fallback key computed over the same content
    workspace_identity,     // WorkspaceIdentity.id — .engram/.workspace-id
                            // prevents cross-workspace reuse of same-tree content
    content_identity,       // per content_key_kind, below
    schema_index_version,   // schema_version + HNSW parameter set
                            // (dim=384, distance=Cosine, m=50) + Cozo storage version.
                            // An index built under different HNSW params is NOT reusable.
    config_hash,            // canonicalized code-graph config affecting index content:
                            // include/exclude globs, language set, embedding model id
                            // + dimension, chunking parameters
))
```

Rationale per component:

* **Content key kind** — makes the fast path and the fallback path occupy
  disjoint key spaces. Without it, a git-tree-derived key and an
  inventory-derived key for the same content could be conflated, and a bug in
  either path would silently contaminate the other.
* **Workspace identity** — the same tree indexed in a different workspace may
  resolve paths differently; `.workspace-id` already exists and is stable.
* **Git tree OID, not commit OID** — this is what makes "new branch, same
  content as parent" reuse work at all. Two branches pointing at different
  commits with the same tree must hit the same generation. Commit OID would
  defeat the entire optimisation.
* **Schema/index version** — an index built with different HNSW parameters or a
  different relation schema is structurally incompatible. Must invalidate.
* **Config hash** — changing include/exclude globs or the embedding model
  changes index *content* for an unchanged tree. Must invalidate.

### Canonical encoding (normative)

`canonical_encode` is a **length-prefixed, domain-separated TLV concatenation**,
not ad-hoc string joining. Ad-hoc joining is rejected because it is not
injective: `"ab" + "c"` and `"a" + "bc"` produce identical bytes, which is
exactly a false-positive-reuse primitive.

```
canonical_encode(fields) :=
      b"engram.generation_id.v1"                  // domain separation tag
   || u32le(field_count)
   || for each field, in the fixed order listed above:
          u32le(byte_len(name_utf8))  || name_utf8
       || u32le(byte_len(value_utf8)) || value_utf8
```

* Field order is **fixed by the specification above**, never by map iteration
  order.
* All values are UTF-8 byte strings; binary OIDs are lowercase hex.
* Integer lengths are **explicitly little-endian `u32`**, so the encoding is
  byte-identical on every platform.
* The leading domain tag carries the format version. Any future change to the
  tuple shape MUST bump it to `...v2`, which invalidates all prior keys by
  construction rather than relying on a separate migration.

This yields the required property: the encoding is injective, so two distinct
field tuples can never produce the same byte string, and therefore never the
same `generation_id`.

### Defining "clean" and "dirty" (normative)

Reuse correctness depends entirely on this definition, so it is specified as a
testable oracle rather than left to intuition.

A workspace root is **clean** if and only if **all** of the following hold:

1. A git work tree resolves for the root.
2. `HEAD` resolves to a commit that has a tree OID (**not** an unborn/empty
   `HEAD`).
3. There are no staged differences (index vs `HEAD`).
4. There are no unstaged differences (work tree vs index).
5. There are **no untracked files that are not ignored** by the effective
   gitignore chain. This clause is load-bearing and is the one most easily
   missed: an untracked, non-ignored `.rs` file **is indexed content**, so a
   tree carrying one has different index content from the committed tree while
   sharing its tree OID. Treating such a tree as clean would publish a
   generation whose content does not match its own identity — the exact
   silent-wrong-reuse failure this design exists to prevent.
6. No in-progress merge, rebase, cherry-pick, bisect, or unresolved conflict
   state.
7. No submodule within the indexed set reports dirty.

Operational oracle: `git status --porcelain=v2 --untracked-files=all
--ignore-submodules=none` emits **zero lines** (ignored files are, correctly,
not reported and do not affect cleanliness). Anything else is **dirty**.

### Content identity by kind

| Kind | When | `content_identity` |
|---|---|---|
| `git-tree-v1` | Clean, single-root git workspace | **Git tree OID** of the indexed subtree — the O(1) fast path, no file walk |
| `inventory-v1` | Non-git root, unborn/empty `HEAD`, or any root where a tree OID cannot be resolved | Merkle digest over the actual indexed inventory: the sorted list of (workspace-relative normalized path, file mode, SHA-256 of file content), using the same construction as the existing `SealedInventory` |
| `multi-root-v1` | Multi-root workspace | Digest over the ordered, length-prefixed list of (canonical root label, per-root content key), sorted by canonical root label; each per-root key computed by whichever kind above applies to that root |

Three rules govern the fallback, and all three exist to prevent
false-positive reuse:

1. **The git tree OID fast path is preserved** for the clean single-repo case —
   the common case and the one the whole optimisation targets. The fallback
   never displaces it.
2. **Inability to compute a content key is a hard non-match, never a wildcard.**
   If no key can be derived, the daemon builds privately and reuses nothing.
   "Unknown identity" must never resolve to "any generation".
3. **False negatives are acceptable; false positives are not.** Because
   `content_key_kind` is inside the hash, the same content addressed via
   `git-tree-v1` and via `inventory-v1` yields two different generations. That
   is deliberate: the cost is one redundant build (merely slow), whereas
   unifying them would mean a bug in either path could serve the wrong index
   (silently wrong).

Note that `inventory-v1` is O(files) rather than O(1). That cost is accepted
because it is confined to workspaces that cannot use the fast path, and because
a correct slow key is strictly better than a fast ambiguous one.

### Dirty-working-tree behaviour

A dirty tree has **no stable content identity**, so it must never publish a
shared generation.

1. Determine cleanliness by the oracle defined above.
2. If the working tree is clean → normal shared-generation path.
3. If dirty → **attach read-only to the nearest matching clean generation** (if
   one exists) and layer dirty-file changes into the **private runtime copy
   only**. The runtime copy is already private per opener, so this requires no
   new isolation primitive.
4. A dirty-tree runtime copy is **never sealed and never published**. It is
   marked non-publishable in its manifest, and **that marking is immutable for
   the lifetime of the copy**. It is garbage-collected on close.
5. If no clean generation matches, build a **private, permanently
   non-publishable** generation for the dirty opener.

#### A dirty-built generation is never promoted by the tree merely becoming clean

This is an explicit safety rule, and it replaces the intuitive-but-unsafe idea
that a privately built generation can be "published later, once the tree is
clean".

That shortcut is unsound for two independent reasons:

* **The content may never have existed in any committed tree.** A dirty build
  indexes the working tree *as it was during the build* — possibly mid-edit,
  and possibly a state that was subsequently discarded. The tree later becoming
  clean says nothing about whether the built index corresponds to the clean
  content.
* **The build races the edits.** A working tree can be modified *while* the
  index is being constructed, so a dirty-built generation can contain a mixture
  of content states that was never simultaneously on disk, let alone committed.

Therefore, to publish a generation for a now-clean identity, the system MUST do
**one** of the following:

* **(a) Rebuild** the generation from the clean tree. This is the default and
  strongly preferred path.
* **(b) Complete content-inventory verification against the exact clean
  identity.** Recompute the full `SealedInventory` — every indexed path, mode,
  and content SHA-256 — and prove it matches the clean identity's inventory
  **in its entirety**. A partial, sampled, timestamp-based, or count-based
  check is explicitly insufficient. Only on a total match may the candidate be
  sealed and published under that identity.

Observing a clean `git status` is **not** by itself a sufficient condition for
publication under either path.

This preserves the invariant: **published generations are always reproducible
from a committed tree.**

### Atomic publication

Reuse the shipped mechanism rather than inventing one:

1. Build into a temporary candidate directory (`GenerationStore::seal_candidate`).
2. Compute the `SealedInventory` of per-file SHA-256 digests.
3. `fsync` files, then the containing directory.
4. Acquire `.publisher.lock`; write the manifest; atomically rename
   `active.json` (rename is atomic on both NTFS and POSIX); `fsync` the parent.
5. Readers resolve through `active.json` only. A crash before the rename leaves
   the candidate orphaned and **invisible** — never partially visible.
6. Orphaned candidates are reclaimed by a GC sweep that is safe to run
   concurrently with readers because published generations are immutable. The
   sweep MUST reclaim **every** class of orphan, including private runtime
   copies abandoned by a process that crashed or was killed by the
   maximum-startup watchdog (`143.003-T`) — those never run a close path, so
   they cannot rely on GC-on-close. It MUST NOT reclaim any copy that is still
   in active use by a live opener.

## Scope split (mandated separation)

The cold-start fix and the liveness fix are **independent defects** and must not
share a shipment:

* **Workstream A — liveness safety.** Health probes must not reset the idle
  TTL; add a maximum-startup watchdog so a never-ready daemon self-terminates.
  Independent of root cause; caps blast radius even if cold start is still slow.
  Ships first because it is small, low-risk, and immediately stops the observed
  2.5-hour CPU burn.
* **Workstream B — cold-start correctness.** Durable per-phase startup
  observability first (to convert the HNSW hypothesis into a measurement), then
  move index construction off the readiness path and wire generation reuse into
  `connect_db`.

Workstream B's first task is **observability, not a fix**: the spike could not
attribute time *within* `schema_bootstrap` because `run_scripts` emits no
sub-phase timings and the `connect_db` timing log is emitted only after the
blocking closure returns. Implementing a fix before measuring would be guessing.

## Consequences

**Positive:** pre-synced same-tree branches become near-instant; storage
collapses from O(branches) to O(distinct content); staleness becomes
structurally impossible; the existing 136-S investment is finally realised.

**Negative / risks:**

* Identity-key bugs cause either silent wrong reuse (severe) or total reuse
  failure (merely slow). Mitigated by making all five components mandatory
  (including the `content_key_kind` discriminator), by an injective canonical
  encoding, and by testing false-positive reuse explicitly.
* A GC story is required or storage grows unboundedly in a new dimension.
* Sequencing risk against active shipment `138-S`, which touches generation
  activation and the startup gate. Workstream B must sequence behind it.

## Sequencing constraint

Shipment `138-S` is **active** on
`feat/138-s-generation-activation-request-context-startup-gate-and-request-entry`
with an active Ship checkpoint. Nothing in this decision may modify `138-S`,
its manifest, its checkpoint, or any `142.*` item. Workstream B overlaps 138-S's
surfaces (generation activation, startup gate) and must therefore be **queued,
not claimed**, and sequenced behind 138-S's completion. The design-time
observation that Workstream A does not overlap 138-S is retained here as a
historical record only and is **superseded by the terminal Revision 3 review**:
`143-S` is **not currently claimable** because all of its manifest members are
blocked, and only a fresh operator-authorized Stage cycle producing a new PASS
may change that.
