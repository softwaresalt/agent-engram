---
doc_type: memory
source: "Operator-requested hook CLI bootstrap and autoharness upstream handoff"
title: "Hook CLI registry bootstrap and upstream handoff"
date: "2026-09-25"
---

## Outcome

* Added poll and ack CLI mappings to
  `.autoharness\backlog-registry.yaml` without changing the active
  `142-S` manifest or P-012
* Wrote the upstream policy and template requirements to
  `docs\scratch\2026-09-25-backlog-cli-first-class-autoharness-handoff.md`
* Created local stash `7DE66A04` as an upstream-transfer reminder;
  existing `DDA2506F` and `037-D` remain the separate local scope record
* No file or backlog item was created in the sibling autoharness workspace;
  the operator must transfer the handoff and create its stash item there

## Verification and limits

* The registry parsed as YAML with both exact hook command templates
* `backlogit hooks poll --consumer-id orchestrator` exited successfully
  with empty `events` and `derived_signals`; no ack was sent
* The new handoff passed targeted `backlogit docs lint`
* The existing P-012 still treats the CLI as a fallback. Upstream
  autoharness templates must make CLI a first-class declared choice and
  re-render the installed harness before that longer-term behavior exists
* An active Stage checkpoint from the prior deliberation remains for
  owner-confirmed selection; this work did not restore or resolve it

## Decisions

* Bootstrap now via the two registry entries; keep the durable
  first-class-CLI policy and full registry inventory in the upstream
  template channel, per deliberation `037-D`
* Do not create a duplicate local implementation item for the upstream
  change or add it to active shipment `142-S`
