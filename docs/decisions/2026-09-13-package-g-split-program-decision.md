---
type: program-decision
date: 2026-09-13
program: finalization write-boundary
subject: Package G decomposition
supersedes_package: G
status: accepted
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
source: docs/decisions/2026-09-13-package-g-split-deliberation.md
---

# Program decision — Package G is permanently superseded by G0 → G1 → G2

## Decision

**The combined Package G ("agent-contract assertion harness") is PERMANENTLY
SUPERSEDED.** It is replaced by three independently planned, independently
reviewed, independently shipped release units:

| Unit | Title | Depends on | Ships |
|---|---|---|---|
| **G0** | Contained test-filesystem seam | — (in-degree 0) | `crates/agent-contract-fs` + exerciser + manifest surface |
| **G1** | Typed contract assertion engine | **G0** | `crates/agent-contract-assert` + exerciser + manifest surface |
| **G2** | Harness registry and CI/local integration | **G1** | seed registry, real-root binding, CI/oracle registration |

**The combined package is closed to reopening.** No fourth combined-G attempt is
authorized. Any future work in this area attaches to G0, G1, or G2, or to a new
unit that declares its dependency on one of them.

## Superseded artifacts

These remain in the repository **unmodified** as evidence. None is deleted,
edited, or rewritten.

| Artifact | Status |
|---|---|
| `docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation.md` | superseded (v1) |
| `docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v2.md` | superseded (v2) |
| `docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v3.md` | superseded (v3) |
| `docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan.md` | superseded, blocked |
| `docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan-v2.md` | superseded, blocked |
| `docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan-v3.md` | superseded, blocked |
| `docs/closure/2026-09-13-package-g-v2-plan-review-record.md` | **preserved as evidence — body not modified** |
| `docs/closure/2026-09-13-package-g-v3-plan-review-record.md` | **preserved as evidence — body not modified** |

The two review records are load-bearing evidence for this decision and for the
"inherited, not re-argued" set in the split deliberation. They are read-only.

## Rationale

Three attempts failed at three different layers (v1 broad; v2 filesystem filters
+ fault-injection seam; v3 root-symlink fail-open + non-injectable
canonicalization). The v3 review's own adjudication localizes its two terminal
findings to a single layer — the `FileAccess` seam — while recording that every
other layer passed, including a clean 0 P0/P1/P2 scope audit and a 7/7
affirmation of the no-speculative-limits decision.

A package whose sound 85% is destroyed three times by its unsound 15% is
mis-packaged. Decomposing along the dependency layer boundary gives each layer
its own gate, so a seam defect can no longer block a sound evaluator and a CI
wiring defect can no longer block a sound seam.

## Revised program DAG

```text
        G0  (test-filesystem seam)          [in-degree 0 — new program root]
         |
         v
        G1  (typed assertion engine)
         |
         v
        G2  (harness registry + CI integration)
         |
    +----+----+
    |         |
    v         v
   B         C          (were: "depends on Package G")
    |         |
    +----+----+
         |
         v
         A
         |
         v
         D  (DEFERRED)
         |
         v
         E  (DEFERRED)

   F — EXCLUDED (separate open deliberation)
```

### Dependency edges changed by this decision

| Edge before | Edge after | Reason |
|---|---|---|
| `G → B` | **`G2 → B`** | B needs executable contract assertions bound to the real harness roots and running in CI. That capability is complete only at G2. G0 or G1 alone cannot satisfy B. |
| `G → C` | **`G2 → C`** | Same. |
| — | **`G0 → G1`** | G1 consumes G0's resolved corpus; it must not re-implement discovery. |
| — | **`G1 → G2`** | G2 loads a G1 registry and invokes the G1 evaluator; it must not re-implement evaluation. |

**G0 replaces the combined G as the program's in-degree-0 root and is the
recommended first shipment of this program.**

## Gate policy for this decomposition

Carried from the operator's instruction and applied per package, independently:

1. **G0 is reviewed first**, with full required personas and cross-model
   diversity, explicitly probing: root lstat-before-canonicalize ordering,
   injectable canonicalization, Windows determinism without platform
   privileges, absence of silent filtering, absence of dead code, and the
   test-only topology claim.
2. **If G0 FAILs, G1 and G2 are NOT reviewed and NOT harvested** — their
   prerequisite authority is absent. Their draft plans are **preserved** as
   dependent drafts.
3. **If G0 PASSes, G1 is reviewed separately.** If G1 PASSes, G2 is reviewed
   separately. One package's verdict never masks another's.
4. **One mechanical correction + confirmation round per package, maximum.** Any
   architecture-level P0/P1 blocks that package immediately; the correction
   budget does not open for architecture-level findings.
5. **PASS requires zero P0 and zero P1.** A P2-only ADVISORY verdict is **not**
   auto-harvested.
6. **Harvest only PASS packages whose prerequisites are PASS or already
   available.** One top-level chore/feature and one shipment **per package**.
   G0, G1, and G2 shipments are **never combined**.

## Explicit non-actions recorded by this decision

* Packages A–F are not modified, not restaged, and not re-reviewed.
* PR #396 is untouched.
* `143.*` remains abandoned.
* No application, source, test, or configuration file is modified by this
  decision or by the planning artifacts it governs.
