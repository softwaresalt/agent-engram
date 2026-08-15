---
title: "Completed release-unit memory compaction: 080-S through 096-S"
type: compacted-memory
date: 2026-08-14
status: complete
sources:
  - docs/archive/memory/2026-07-14-ship-080S-dax-intelligence-merge-closure.md
  - docs/archive/memory/2026-07-28-ship-092S-same-file-shadowing-closure.md
  - docs/archive/memory/2026-07-28-ship-093S-cargo-audit-closure.md
  - docs/archive/memory/2026-07-28-ship-094S-versioned-revalidation-closure.md
  - docs/archive/memory/2026-07-29-ship-095S-daemon-drain-hardening-closure.md
  - docs/archive/memory/2026-07-29-ship-096S-forced-index-certify-completeness-closure.md
---

# Completed release-unit memory compaction

## 080-S / 085-F — DAX intelligence

Merge commit `f10caff` delivered the DAX intelligence work under the merge-only
Git policy. Power BI/DAX extraction, linting, and cross-domain impact tooling
passed CI and Copilot gates. Deferred follow-ups remain for DAX invalidation
fingerprints, symlink traversal, `--` comments, impact descriptions, and
`index --force` documentation.

## 092-S / 100-F — Same-file shadowing

PR `#291` (`8a6c6e3`) made duplicate same-file function names fail closed and
added language-agnostic direct-edge guards plus tests. RED testing corrected
the original Python-focused diagnosis to the Rust `cfg`-gated duplicate case.
Follow-ups include versioned backfill, Python last-wins behavior, cargo-audit,
and daemon index routing/hang investigation.

## 093-S / 102-F — Cargo audit remediation

PR `#295` (`308c04b`) reduced nine transitive advisories through lockfile
updates and Cozo feature reduction. The remaining `lz4_flex` advisory was
accepted pending a Cozo major-version path; rmcp 2.x and Cozo 0.8+ were out of
scope. Local tests require clearing inherited `ENGRAM_DATA_DIR`. Stash
`99AFF44B` tracks the Cozo 0.8+ upgrade.

## 094-S / 101-F — Versioned code-graph revalidation

PR `#293` (`bb96896`) added a versioned generation marker, opt-in revalidation,
stale-edge retraction, and deletion/eviction teardown. Initial design gaps
around orphan rows, deletion reconciliation, zero-byte handling, and pending
sync ordering were fixed during review. Later 095-S/096-S addressed related
daemon-drain and eviction work; provenance reconciliation remains separate.

## 095-S / 104-F — Daemon pending-sync drain hardening

PR `#297` (`315e538`) packed pending-sync flags into `AtomicU8`, added atomic
full-mask publishing, and bounded drain-to-completion across all drain paths.
Deterministic concurrency tests and the full quality gates passed. Deferred
items are producer/consumer handoff race `0B5AAAD2`, generation-scoped queue
`B7F52777`, and provenance reconciliation `A85DC0E3`.

## 096-S / 103-F — Forced-index certification

PR `#299` (`300d020`) made forced-index certification evict the file set, sweep
orphan edges, and advance the generation marker only after the clean pass.
Targeted suites, recall tests, quality gates, runtime checks, and Copilot gates
passed. Stash `7A317008` tracks eviction ordering versus cross-file singleton
recall recovery.

## Preserved, not compacted

Memory for 091-F, the Spark lineage spike, and 097-F/099-S was retained because
the source reports active or queued follow-up work. Those files remain at their
original paths for continuity.
