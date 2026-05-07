---
session: post-merge/042-F-startup-preloading
date: 2026-05-07
phase: post-merge-closure
shipment: 027-S
merge_sha: ad867b3
pr: 88
status: SHIPPED
---

# 027-S Post-Merge Closure Memory

## Work Completed

### PR #88 Merged
- Admin bypass merge (ruleset required 1 approval; Copilot left COMMENTED not APPROVED)
- All 13 review threads resolved (8 from second Copilot review, 5 from first)
- Merge SHA: ad867b3

### Second Copilot Review — 8 Comments Addressed
| Comment | Action | Commit |
|---|---|---|
| start.ps1: @args missing | Already fixed in 8443af2 | Reply + resolve |
| start.ps1: --workspace . | Already fixed in 8443af2 | Reply + resolve |
| 042-F ID collision | Declined — known backlogit limitation | Reply + resolve |
| 042.001-T ID collision | Declined — same root cause | Reply + resolve |
| 027-S shipment ID refs | Declined — same root cause | Reply + resolve |
| stash.jsonl harvested_artifact_id | Already fixed in 8443af2 | Reply + resolve |
| deliberation.md typo | Already fixed in 8443af2 | Reply + resolve |
| memory file ID refs | Declined — same root cause | Reply + resolve |

### Backlog Archival
- Created: .backlogit/archive/027-S.md (merge sha ad867b3)
- Deleted: .backlogit/queue/027-S.md, 042-F.md, 042.001-T.md
- NOTE: 042-F and 042.001-T NOT archived (ID collision with old CLI parity in archive/)
- Follow-up stash entries added: C3A8E7F4 (FU-1 binary install) and F2D1B9C5 (FU-2 cold-start test)

### Closure Doc Updated
- docs/closure/2026-05-07-027-S-startup-preloading-closure.md → SHIPPED mode
- Updated frontmatter: mode=post-merge, merge_sha, status=SHIPPED
- Added Post-Merge Status section explaining ID collision and follow-ups

### Post-Merge PR
- Branch: post-merge/042-F-startup-preloading
- PR #89 created
- Commit: 5effeba

## Decisions Made

1. **Admin merge bypass**: GitHub ruleset requires 1 approving review. Copilot only leaves COMMENTED.
   Owner used --admin flag to merge. This is the expected pattern for this repo.

2. **No 042-F/042.001-T archive**: Cannot overwrite old CLI parity archive files.
   Queue items deleted instead. Shipment (027-S) archived normally.

3. **ID collision comment responses**: Declined renaming suggestion. Backlogit auto-increment
   does not skip archived IDs — known limitation, queue items at different path from archive.

## Files Modified in Post-Merge Branch

- .backlogit/archive/027-S.md (created)
- .backlogit/queue/027-S.md (deleted)
- .backlogit/queue/042-F.md (deleted)
- .backlogit/queue/042.001-T.md (deleted)
- .backlogit/stash.jsonl (added FU-1 C3A8E7F4 and FU-2 F2D1B9C5)
- docs/closure/2026-05-07-027-S-startup-preloading-closure.md (updated to SHIPPED)

## Open Items

- PR #89 (post-merge closure) awaiting merge approval
- FU-1 (C3A8E7F4): cargo install --path . to activate sync subcommand
- FU-2 (F2D1B9C5): Manual cold-start test after FU-1
