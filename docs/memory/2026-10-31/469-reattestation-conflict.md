---
title: "PR 469 attested-input recovery verified"
date: 2026-10-31
agent: ship
shipment_id: "167-S"
feature_id: "142-F"
status: recovered-validation-pending
---

## Current state

PR #469 remains open on
`post-merge/167-s-142-f-slot-19-1-142-080-t-pa5-t1`.
HEAD remains `5f14d8332e2e2c748562b94b30c98f2c84c7e0fe`.
No corrective commit, push, thread resolution, or merge occurred.
No new P-021 stash entry was created. Existing follow-ups are `D74F9442`
and `A5BEA885`.
The operator-approved Path A restoration is now verified. Review readiness
and merge remain unverified; this is not a completed review or merge.

## Historical Path B failure

The operator authorized the two Copilot corrections and Path B re-attestation.
The initial Path A attempt failed verification: the raw `9220588f` closure blob hashes to
`e7e2927983b031d686d11be66857e6a90735cf94d85404e1eeca4146176311f5`;
its CRLF representation also differs from the original attestation.

Delegated contract corrections and companion updates remained uncommitted.
The Path B canonical closure input hashed to
`c3e692e2ff5eae65f158f309fd0d647784aecc930e8901edf340b31ccf5bc5da`.
The implementation leaf reported closure-evidence PASS and diff-check PASS
before the provenance reset. No current-HEAD local readiness is certified.

The approved preparatory reset changed only the shipment archive:
`archived_status: active`, with the prior reconciliation fields removed.
The subsequent official command was:

```text
backlogit shipment reconcile-shipped 167-S --reason "reconcile safe-closed partial-feature shipment 167-S under 142-F to shipped" --actor ship --idempotency-key idem-167-s-2 --merge-sha c2e82cb263b45226707a50c88b6f659020deb0b0 --closure-evidence docs/closure/167-S-142-F-post-merge-closure.md --confirm "reconcile-shipped 167-S"
```

It exited 10:

```text
shipment 167-S reconciliation request conflicts with a different already-recorded reconciliation for idempotency key "idem-167-s-2"
```

Read-only verification at that point confirmed `status: archived`,
`archived_status: active`. The reset was an uncommitted partial correction,
not completed closure. No journal edit, alternate-key retry, digest
substitution, or rollback ran during that failed attempt.

## Safety and verification

ProposedAction was the operator-approved Path B provenance reset followed by
the governed transaction. ActionRisk was destructive; rollback source is
the archive at HEAD above. ActionResult was blocked after the reset because
the governed transaction rejected the request. Recovery required operator
disposition while preserving the historical event journal.

The parent feature remains byte-identical with SHA-256
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770`.
The earlier startup found zero active checkpoints and zero quarantine
anomalies. No recovery checkpoint was selected or resolved during that
earlier attempt.

## Current recovery

The operator explicitly selected and confirmed restoration of
`checkpoint-20261031-070206.json`. Recovery read the bound engram state,
retained a bounded prune summary preserving the active cursor, gate
verdicts, and reconciliation conflict, then resumed. The checkpoint was
resolved through the official CLI at `2026-10-31T07:15:59Z`; subsequent
verification found zero active checkpoints.

The operator explicitly approved Path A restoration. The parent restored
`.backlogit/archive/167-S.md` from HEAD, rolling back the uncommitted archive
reset. This bounded remediation did not mutate the archive, event journal,
or backlog.

The canonical closure input was reconstructed from the UTF-8 `9220588f`
blob using the approved field and paragraph replacements, normalized to
CRLF, and encoded as UTF-8. SHA-256 was asserted before writing and verified
after writing:

```text
e887309db2081d8cf2a39ffb06f6d4b8d9ec7ec0396b54fa902f512b42ec0ee1
```

The closure file was not edited again after exact restoration.
Post-reconcile outcomes remain in the companion records
`.backlogit/reconcile/167-S-post-20261031T061714Z.md` and
`docs/memory/compacted/2026-10-31-167-s-compacted.md`, not in the attested
input.

Both required read-only checks passed:

* `backlogit shipment reconcile-shipped 167-S --dry-run` with the original
  `idem-167-s-1` key, approved reason, merge SHA, actor, and closure evidence
  exited 0: `outcome: no_op`; the reconciliation was already durably
  recorded and no action was taken.
* `autoharness gate closure-evidence --path docs/closure/167-S-142-F-post-merge-closure.md --shipment 167-S --json`
  exited 0 with `passed: true` and no warnings.

ActionRisk for the explicitly approved restoration was destructive;
ActionResult is applied and verified. Only the canonical closure input and
this recovery memory were edited by this bounded remediation. Existing
stash IDs `D74F9442` and `A5BEA885` remain the follow-ups; no new captures
were created. No commit, push, branch change, thread resolution, or merge
was performed.

## Remaining verification

Fresh terminal diff certification, local report-only review, and
current-HEAD P-014/P-018 gates remain required before merging. Neither
successful local checks nor the historical reconciliation no-op certify
review completion, closure PR merge, or a completed observation window.
