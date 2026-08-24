---
title: "Cloud-placeholder Git admission verification"
type: spike
date: 2026-08-24
time_box: "2h"
conclusion: defer
confidence: high
status: blocked
source_stash_id: "49000348"
promoted_to:
  - blocked-environment-gate
tags: [windows, cloud-files, reparse, workspace-admission]
---

# Cloud-placeholder Git admission verification

## Goal

Does the uniform Windows reparse-point gate reject a legitimate repository whose `.git` metadata contains a dehydrated OneDrive or Dropbox cloud placeholder?

## Success Criteria

Observe the current merged binary against a disposable, real cloud-backed repository; record provider, reparse tag, dehydrated path, hydrated control result, admission result, and exact error. Local synthetic fixtures do not satisfy this spike.

## Scope Constraints

No production change and no claim that local policy tests validate cloud providers. Do not use the operator repository or mutate a live working copy. Use a disposable cloud-backed repository with provider sync status visible.

## Investigation Approach

1. Re-read 121-S rollback trigger 1 and the 48-hour observation record.
2. Check whether this machine exposes a disposable cloud-backed repository with dehydrated `.git` content.
3. If absent, define the exact manual environment gate and stop.
4. After an environment is supplied, run hydrated and dehydrated controls and capture provider-native reparse evidence.

## Findings

### What Was Discovered

Shipment 121-S deliberately rejects every `FILE_ATTRIBUTE_REPARSE_POINT` in the validated Git chain. Its closure records this cloud-placeholder case as LOW-confidence and unverified. Shipment 124-S demonstrates ordinary Windows client/daemon operation only; it supplies no cloud-backed `.git` evidence and cannot close this observation.

No authorized disposable OneDrive/Dropbox repository with confirmed dehydrated `.git` content is available in this session. A normal local checkout, a junction, or a mocked attributes predicate would test different behavior and must not be presented as validation.

### Environment Gate

Required evidence set:
- Windows 11 or supported Windows host with OneDrive Files On-Demand or Dropbox Smart Sync.
- Disposable native Git repository physically inside the provider root.
- At least one validated `.git` path (`HEAD`, `objects`, `refs`, `worktrees`, or linked-worktree metadata) confirmed dehydrated and carrying a provider cloud reparse tag.
- Provider/tag evidence from `fsutil reparsepoint query` or equivalent read-only inspection.
- Hydrated control admitted by the same binary.
- Dehydrated run recording admission success or exact `NotGitRoot` failure.

### Remaining Unknowns

Whether providers permit `.git` metadata to dehydrate in practice, which tags appear, and whether Git itself hydrates the path before Engram observes it.

## Recommendation

**Conclusion: defer. Confidence: high that environment evidence is mandatory.** Keep the stash active and create no implementation shipment. Resume only when the manual environment gate is satisfied. If a legitimate hydrated-control/dehydrated-test pair shows rejection, return to Stage for a new security-and-availability design decision; do not weaken the uniform gate inside this spike.

## References

- `docs/closure/2026-08-21-568b257c-runtime-verification.md` lines 168 and 266-267
- Shipment `121-S`, merge `119230fe`
- `docs/closure/2026-08-23-124-s-runtime-verification.md` (ordinary control only)
- Shipment `124-S`, merge `8f9904a0`
