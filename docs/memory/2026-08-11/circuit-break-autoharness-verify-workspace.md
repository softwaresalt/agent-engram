---
type: validation-record
timestamp: 2026-08-11T12:06:00-07:00
agent: stage
skill: direct
operation: autoharness verify-workspace
attempts_observed: 3
universal_breaker_tripped: false
---

# Validation record: autoharness verify-workspace

## Invocation Chain

### Invocation 1 — invalid usage

`autoharness verify-workspace --help` exited 2 because this subcommand does not accept `--help`.

### Invocation 2 — different invalid usage

`autoharness verify-workspace` exited 2 because `--workspace <path>` is required.

### Invocation 3 — completed validation with blockers

`autoharness verify-workspace --workspace .` completed report generation and exited 1: 25 strict-schema blockers, 0 ordinary blockers, 8 warnings, 3 migration proposals, 3 uninstalled artifacts, and 0 unresolved placeholders. The strict blockers are pre-existing schema mismatches in `.autoharness/harness-manifest.yaml` and `.autoharness/workspace-profile.yaml`; this Stage batch modified neither file.

## Breaker Analysis

The universal same-error breaker did **not** trip. Invocations 1 and 2 were distinct usage errors, while invocation 3 completed validation and exposed a different pre-existing strict-schema baseline. This record therefore must not claim three consecutive substantially same failures or a breaker stop.

The broad command is not being rerun because the operator expressly prohibited rerunning the already diagnosed validation. Targeted Stage validation is used instead; the 25 strict-schema blockers remain separately reported baseline blockers.

## Context

- Governing remediation: `docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md`
- Targeted validation: artifact frontmatter/schema/references, backlog sync/doctor, DAG/topology wording, one-worktree/admission/cleanup contracts, generated-template durability, retry semantics, and `git diff --check`
- Backlog doctor wording: no new actionable findings; historical 43 `archived_from_self_ref` advisories remain a known baseline
- Markdownlint: unavailable is advisory
