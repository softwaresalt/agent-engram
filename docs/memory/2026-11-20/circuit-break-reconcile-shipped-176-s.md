---
type: circuit-breaker
timestamp: "2026-11-20T20:58:10Z"
agent: "Ship"
skill: "shipment-reconcile"
breaker_type: universal
operation: "backlogit shipment reconcile-shipped 176-S --dry-run"
attempts: 3
identity: "reconcile-shipped-176-S-closure-feature-annotation"
---

# Circuit Breaker - reconcile-shipped 176-S

## Failure Chain

### Attempt 1

* Exit: 10
* Operation evidence: closure evidence validation, `cwd=C:\Source\GitHub\engram`
* Normalized message: closure section lacks a feature-annotated merge commit
* Diagnostic artifact: none

### Attempt 2

* Exit: 10
* Operation evidence: closure evidence validation after adding the feature annotation
* Normalized message: closure section lacks a feature-annotated merge commit
* Diagnostic artifact: none

### Attempt 3

* Exit: 10
* Operation evidence: closure evidence validation after canonical one-line formatting
* Normalized message: closure section lacks a feature-annotated merge commit
* Diagnostic artifact: none

## Context

* Files involved:
  * `docs/closure/176-S-142-F-post-merge-closure.md`
  * `docs/closure/evidence/176-S-142-F-close-evidence.json`
  * `.backlogit/archive/142.089-T.md`
  * `.backlogit/archive/176-S.md`
* Shipment and task cursor: `176-S`, `142.089-T`
* Provisional-to-concrete identity: the same native command, target shipment, phase, and stable parser message were observed three times
* Logging controls: no raw capture; bounded error summaries only
* Resolution: circuit breaker triggered; await operator guidance
* Suggested next step: inspect the installed backlogit closure-evidence parser's exact accepted delivery-merge syntax, then resume without rerunning through an agent retry loop

## Resolution

* Operator reset: `Authorized` at `2026-11-20T21:21:56Z`
* Canonical parser fix: the `Delivery:` line uses the unbackticked 40-hex
  feature merge SHA and ends with `(merge commit, P-009).`
* Artifact normalization: `docs/closure/176-S-142-F-post-merge-closure.md`
  is normalized to LF line endings before `reconcile-shipped`
* Resume state: the tripped circuit is reset and the governed 176-S
  `reconcile-shipped` dry-run may resume after all closure gates pass
