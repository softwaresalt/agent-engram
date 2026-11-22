---
title: "177-S strict-safety evidence"
date: 2026-11-22
doc_type: closure
source: "177-S / PR #493"
shipment_id: "177-S"
feature_id: "142-F"
pull_request: 493
---

## Shipment and manifest archive actions

> [!IMPORTANT]
> This supplement preserves the destructive-action approval and captured
> byte representation for the 177-S post-merge closure. The canonical
> closure record is not rewritten to replace either historical digest.

## Closure attestation representation

The governed `reconcile-shipped` event attests closure-content hash
`f053ae15d77cee02115db29d4d453d8b9a65a38527c535dc2b1345dca5ac2124`.
The pre-attestation bytes are reproduced from committed closure candidate
`4623564c1fe7455bfa3d6963664674b1209b44e1` by:

1. Reading the committed LF blob.
2. Removing the two later-added frontmatter lines `doc_type: closure` and
   `source: "177-S / 142.058-T"`.
3. Converting LF to CRLF to reproduce the Windows working-tree capture.

That procedure produces the exact attested hash above. The committed LF
candidate itself has SHA-256
`31c74a8af8f7d172d764438a9c3e0453888ba8f2ea60c86b4f7121691957ec10`.
Later explanatory edits are carried in the supplement instead of rewriting
the historical transaction digest.

## Protected observation representation

The classifier evidence records 121 observation-set hashes for the mixed
Windows working-tree capture. At final verification, 117 observations match
CRLF bytes and four match LF bytes. The LF records are `142-F`,
`142.059-T`, `142.085-T`, and `142.089-T`.

Replay from committed LF blobs therefore converts 117 records to CRLF and
leaves the four named records as LF. This reproduces all 121 recorded
SHA-256 values without hand-editing the immutable pre-close JSON.

## Shipment artifact relocation

| Field | Value |
|---|---|
| `ProposedAction` | Relocate `.backlogit/queue/177-S.md` to `.backlogit/archive/177-S.md`; do not cascade into `142-F` or any successor. |
| `ActionRisk` | `destructive` because the source queue record is deleted by relocation. The archived record preserves the shipment body and reconciliation envelope. |
| `ApprovalSource` | The operator's 177-S Phase 4 directive: “Archive ONLY `142.058-T` and `177-S` individually (`SAFE_CLOSE`).” It separately declared `destructive_cascade_close_pre_authorized: false`. |
| `ActionResult` | `applied`; `.backlogit/queue/177-S.md` was removed and `.backlogit/archive/177-S.md` carries `status: archived` and `archived_status: shipped`. |
| `Containment` | No covering feature, successor shipment, task, or child subtask was relocated by this closure action. Independently captured hashes for the five named protected records were unchanged. |
| `Rollback` | After fresh explicit operator approval, restore only the pre-close `177-S` queue record, remove the duplicate archived record at the exact path, and verify one authoritative shipment record remains. The rollback does not authorize reversal of `142.058-T` or any child subtask. |

## Manifest task archival

| Field | Value |
|---|---|
| `ProposedAction` | Archive `142.058-T` explicitly in its existing archive-resident record with `archived_status: done`. |
| `ActionRisk` | `moderate` lifecycle mutation; no source path is deleted by this step. |
| `ApprovalSource` | The same bounded Phase 4 directive explicitly authorized individual archival of `142.058-T` and `177-S`. |
| `ActionResult` | `applied`; the task carries `status: archived`, `archived_status: done`, and merge traceability. |
| `Rollback` | Any reversal requires a new lifecycle disposition; it does not authorize restoring stale parent or child state. |

## Approval separation

The individual-archive directive is a destructive-action authorization
bounded to `142.058-T` and `177-S`. It does not grant, replace, or alter
feature or closure PR merge approval. PR #493 remains subject to its own
current-HEAD readiness, Copilot, and P-014 gates.

## Code rollback boundary

A code rollback for PR #492 must limit itself to the contract-test code
delta. It must preserve the completed backlog, merge-traceability, and
closure evidence already delivered by the feature and closure PRs.
