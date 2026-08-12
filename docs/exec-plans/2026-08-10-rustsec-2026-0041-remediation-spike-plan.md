---
title: "RUSTSEC-2026-0041 dependency remediation feasibility spike"
type: implementation-plan
date: 2026-08-10
source: docs/decisions/2026-07-29-cozo-0_8-major-bump-feasibility-deliberation.md
status: reviewed
execution_kind: spike
source_stash_ids: [27F691AE]
source_deliberation: 017-D
references:
  - docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md
  - .backlogit/archive/119.001-R-rustsec-2026-0041-spike-plan-security-review.md
  - .backlogit/queue/115-S.md
  - .backlogit/queue/116-S.md
  - docs/compound/workflow-issues/dark-mode-single-worktree-disk-admission-gates-2026-08-02.md
  - docs/memory/2026-08-11/pr-337-adversarial-remediation-memory.md
---

# RUSTSEC-2026-0041 dependency remediation feasibility spike

## Problem Frame

The locked chain is `engram -> cozo 0.7.6 -> swapvec 0.3.0 -> lz4_flex 0.10.0`. `RUSTSEC-2026-0041` affects lz4_flex block decompression and is patched at 0.11.6/0.12.1. No straightforward compatible upgrade currently exists. This shipment authorizes only a bounded compatibility and supply-chain investigation; it does not authorize a production override, fork, vendoring decision, Cozo replacement, migration, second worktree, sibling repository, global cache write, live-data access, or implicit cleanup.

## Requirements Trace

- `27F691AE` and `017-D`: U1 establishes immutable identity and admission evidence; U2 proves or rejects one approved patch; U3 verifies synthetic runtime/data compatibility and records `proceed`, `pivot`, `defer`, or `abandon`.
- Dark-factory admission: exactly the current core worktree, a separately disposed `.autoharness/config.yaml`, a clean tracked baseline, process/handle quiescence, adequate disk, an empty isolated workspace-local target, inventoried prior artifacts, synthetic data only, and named cleanup ownership are mandatory before mutation or execution.
- Untrusted execution: candidate discovery and static inspection are read-only. Build scripts, proc macros, tests, and binaries from the exact candidate require explicit operator approval after immutable identity, hashes, license, source delta, executable inventory, and containment evidence are presented. Generic, inferred, shipment, or dark-factory approval never satisfies this gate.
- Closure: byte restoration, process exit, path and external-directory fingerprint verification precede a separately approved exact cleanup. `116-S` also requires a successful cleanup result and verification; refusal, unavailable approval, or failed cleanup keeps it blocked.

## Claim and Admission Contract

`115-S` remains queued and unclaimable while the unrelated tracked `.autoharness/config.yaml` modification lacks separate operator disposition. Stage publication may use a path-scoped commit that excludes it, but Ship must not claim `115-S` until the operator separately chooses how to dispose of that change and `git status --porcelain --untracked-files=no` is empty. This plan neither reads policy intent from nor depends on any value in that config file.

After claim and before the first Cargo/cargo-audit command, manifest/lock mutation, candidate execution, or prototype execution, record all of these gates in the findings artifact:

1. `git worktree list --porcelain` reports exactly one registered worktree whose canonical path is the current core root. Branch, HEAD, and clean tracked status match the admitted baseline. No repository copy or second worktree exists.
2. The OS process table and handle inspection show no running `cargo`, `rustc`, `rust-analyzer`, Engram daemon, or other process holding the core `target/`, run target, `Cargo.toml`, `Cargo.lock`, candidate files, or relevant lock files. Require orderly owner-approved shutdown; do not kill processes implicitly.
3. Inventory the core `target/`, every prior path under `tmp/rustsec-2026-0041/`, manifest/lock baselines, candidate source, caches, data, and logs. Select a workspace-local `CARGO_TARGET_DIR` that is absent or empty. Existing prior-run content blocks execution until separately inventoried and approved for targeted cleanup; never reuse it silently and never run `cargo clean` against the core target.
4. Measure baseline footprint `B` as allocated bytes for the current core target plus the locked-package source/cache material needed for a fresh local build. Require free bytes on the workspace volume of at least `max(20 GiB, ceil(1.5 * B) + 2 GiB)` before execution, record the values, and recheck before U2 and U3. Insufficient or unmeasurable capacity blocks.
5. Prove `tmp/rustsec-2026-0041/data/` is absent or empty and cannot resolve to an operator, production, or existing Engram database. All runtime scenarios create uniquely marked synthetic data there; no live data is read, copied, migrated, repaired, or deleted.
6. Name the Ship executor as owner of baseline capture, process shutdown verification, byte restoration, and cleanup request preparation. Name the operator as approver of the exact destructive target list. Name the approved cleanup executor only in the approval record.

## Workspace-Local Execution Contract

| Purpose | Required path/control |
|---|---|
| Run root | `tmp/rustsec-2026-0041/` |
| Cargo package/Git cache/config | `tmp/rustsec-2026-0041/cargo-home/` via workspace-local `CARGO_HOME` |
| Isolated build output | `tmp/rustsec-2026-0041/target/` via explicit `CARGO_TARGET_DIR`; absent or empty at admission |
| cargo-audit advisory DB | `tmp/rustsec-2026-0041/cargo-audit/advisory-db/` via supported `cargo audit --db`; block if the installed tool cannot honor it |
| cargo-audit data/cache | `tmp/rustsec-2026-0041/cargo-audit/data/` and `.../cache/` via supported tool-specific flags/paths; otherwise block |
| Candidate/static inspection | `tmp/rustsec-2026-0041/candidate-source/` |
| Prototype artifacts | `tmp/rustsec-2026-0041/prototype/` |
| Logs | `tmp/rustsec-2026-0041/logs/` |
| Temporary files | `tmp/rustsec-2026-0041/temp/`; `TMP`/`TEMP` on Windows and `TMPDIR` on Unix |
| Baselines/hash ledger | `tmp/rustsec-2026-0041/baselines/` |
| Synthetic Cozo data | `tmp/rustsec-2026-0041/data/` |

On Windows, effective containment is the workspace-local `CARGO_HOME`, explicit `CARGO_TARGET_DIR`, supported cargo-audit `--db`/data-path controls, tool-specific flags, Windows temporary-directory controls, and unchanged external-directory fingerprints. `XDG_DATA_HOME` and `XDG_CACHE_HOME` are Unix-specific supplemental controls only; do not claim they redirect Windows tools. Before and after each unit, fingerprint default external Cargo and cargo-audit database/data/cache directories (record absent as absent), then prove those fingerprints did not change. Missing support or proof blocks.

## Implementation Units

### U1 — Establish admission, immutable evidence, and approval stop

**Domain/files:** one findings artifact, `docs/decisions/2026-08-10-rustsec-2026-0041-remediation-spike-findings.md`. **Cap:** 110 minutes, no candidate execution.

Complete the claim/admission contract and inventory every named run path, including `tmp/rustsec-2026-0041/data/`. Establish the Windows/Unix controls accurately, capture `Cargo.toml` and `Cargo.lock` bytes plus SHA-256, fingerprint protected external paths, and record baseline graph/audit/release/call-path evidence.

Candidate discovery is read-only. Acquire at most one immutable candidate for static inspection under `candidate-source/`; do not invoke its build system or execute code. Record exact source URL/repository, immutable revision/version, archive/crate checksum and content hash, license, owner/maintenance status, reviewed source delta, and complete build-script/proc-macro/test/binary plus new-transitive-code inventory. Present that evidence and containment plan to the operator. Record explicit approval bound to the exact identity, or record `blocked`. No auto-check, generic request, inferred consent, or unavailable approval permits U2.

### U2 — Prototype one explicitly approved patch

**Domain/files:** temporary dependency/prototype edits only in the current core worktree and run root. **Cap:** 110 minutes, one approved candidate, no Engram Rust source change.

Re-prove all admission gates, process/handle quiescence, disk threshold, empty isolated target, external fingerprints, and the exact approval before execution. Capture byte baselines and hashes for every allowlisted manifest, lock, candidate, or harness file before mutation. Any candidate build script, proc macro, test, or binary execution without the exact approval is a high-risk manual-gate failure and stops the shipment.

Run only the bounded compile/graph/audit prototype. Prove no affected or duplicate lz4 version, native audit semantics, and focused compile success. Stop on a second third-party package, mutable source, Cozo fork, unsafe/backport work, scope widening, process collision, disk pressure, containment uncertainty, or restoration risk.

Before U2 completes, wait for all child processes to exit, re-run handle checks, restore every temporary edit from captured bytes, verify hashes and admitted tracked status, and recheck external fingerprints and canonical artifact paths.

### U3 — Verify synthetic runtime/data compatibility and close

**Domain/files:** focused synthetic verification, one prototype-only reopen harness, and the findings artifact. **Cap:** 110 minutes excluding operator approval wait.

Re-prove U2's gates. Create/populate only uniquely marked synthetic Cozo data under `tmp/rustsec-2026-0041/data/`; close every baseline handle before candidate reopen. Verify exact graph counts/query results, the separate dehydration/hydration regression, focused Cozo targets, locked graph/audit, Windows result, and Linux/macOS compile disposition. No unavailable platform proof may be represented as success.

Record `proceed`, `pivot`, `defer`, or `abandon`, then complete post-spike closure: all spawned processes exited; no protected handle remains; temporary files are restored byte-identically; tracked status equals admission; every path remains under the run root; external fingerprints are unchanged; no live data was touched.

Prepare an exact cleanup inventory limited to approved workspace-local prototype/cache/data/log artifacts under `tmp/rustsec-2026-0041/`. Cleanup is a separate `ActionRisk: destructive` action. Present exact canonical paths, hashes/sizes, owner, exclusions, and verification steps after spike evidence exists. Blanket dark-factory or shipment approval is not cleanup approval. If explicitly approved, remove only listed targets and verify their absence plus unchanged protected paths/fingerprints. Record `approved-and-verified`, `not-approved`, or `failed`. The latter two are valid closure facts but do not unlock `116-S`.

## Dependency Graph

U1 blocks U2; U2 blocks U3. Shipment `115-S` remains batch `dark-factory-2026-08-10`, order 1, predecessors `[]`. Shipment `116-S` remains the same batch, order 2, predecessors `[115-S]`, with hard edge `116-S -> 115-S`.

Technical independence does not permit early claim. `116-S` remains queued/unclaimable until backlogit records `115-S` shipped, its shipment/items archived, merge commit evidence present, and the findings/closure evidence records an exact cleanup approval, successful targeted cleanup, and passing absence/path/fingerprint verification. If cleanup is not approved, unavailable, partial, or failed, `116-S` stays blocked even if `115-S` shipped.

## Decisions and Rationale

- Keep one core worktree and one candidate; do not manufacture isolation through a second repository.
- Use a fresh workspace-local target rather than cleaning/reusing the core target.
- Treat static discovery as read-only and candidate execution as `ActionRisk: high`, manual approval required.
- Treat targeted run-artifact cleanup as a later `ActionRisk: destructive`, exact approval required.
- Preserve Windows accuracy: XDG variables supplement Unix only and do not prove Windows containment.

## Plan Hardening

Hardening is required for security, untrusted execution, process/disk admission, destructive cleanup, filesystem containment, and durable-store risk.

- **ProposedAction:** statically inspect one immutable candidate. **ActionRisk:** moderate. **approval_required:** no execution approval for read-only inspection; admission evidence still required. **ActionResult:** planned.
- **ProposedAction:** build or execute the exact candidate. **ActionRisk:** high. **approval_required:** explicit operator approval after identity/hash/license/inventory/containment evidence; no substitute. **ActionResult:** blocked until approval.
- **ProposedAction:** temporarily mutate allowlisted manifests/lock/harness files. **ActionRisk:** high. **approval_required:** exact baseline and restoration contract. **ActionResult:** planned after admission.
- **ProposedAction:** remove exact run artifacts. **ActionRisk:** destructive. **approval_required:** separate post-spike operator approval for canonical targets; blanket approval invalid. **ActionResult:** blocked pending post-spike inventory.
- **Protected invariants:** one worktree, clean tracked baseline, quiescent processes/handles, disk threshold, isolated target, no live data, no external writes, byte restoration, exact cleanup, and `116-S` cleanup-result gate.

## Runtime Verification and Closure

Healthy closure requires all admission evidence, exact approval, bounded prototype results, child-process exit, no open handles, byte-identical restoration, admitted status, canonical path proof, unchanged external fingerprints, synthetic-only data, and a recorded cleanup disposition. Cleanup success additionally requires exact approval, targeted deletion only, path absence, and protected-state re-verification. A recommendation may be recorded when cleanup is declined or fails, but `116-S` remains blocked.

## Historical Plan Review

The 2026-08-10 review and first 2026-08-11 focused re-review are preserved in `119.001-R`. They predate the second adversarial findings and do not by themselves authorize execution.

## Focused Plan Re-review — 2026-08-11 (second remediation)

**Stage gate: PASS; final adversarial rerun still required.** Constitution, scope, architecture, security/supply-chain, strict-safety, Windows containment, rollback/cleanup, and operational-sequencing lenses found P0 0, P1 0, P2 0, P3 0 in this final Stage contract. The gate confirms the dirty-config claim stop, process/disk/target/prior-artifact/live-data admission, explicit exact-candidate approval, byte/process/path closure, exact cleanup approval/result, and extended `116-S` block. It authorizes only the bounded investigation after every gate; it does not approve candidate execution, cleanup, or production remediation.
