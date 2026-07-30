---
title: "Python last-wins recall recovery for same-file duplicate defs — DECISION: keep fail-closed"
type: deliberation-decision
date: 2026-07-29
deliberation_id: "016-D"
stash_id: "B94772CB"
layers_on: ["100-F", "092-F"]
governed_by: ["013-D"]
decision: "KEEP FAIL-CLOSED — do not build last-wins recovery now"
status: decided (park / archive)
tags:
  - code-graph
  - resolution
  - recall
  - precision
  - decision
---

## Decision

**Keep the shipped language-agnostic fail-closed behavior. Do not implement
Python last-wins recall recovery at this time.** Park 016-D; revisit only if
measured recall loss on a real Python corpus justifies the added
precision/complexity risk.

## Context

`100-F` / `092-F` (merged, `092-S`) shipped **Option A — fail-closed**: when a
file has `> 1` top-level def of the same name, the direct-edge minting sites do
**not** mint a first-match edge; they drop it (`find_unique_function_id`
additive helper). This guarantees **zero wrong-target edges** (013-D no-false-edge)
across both Rust and Python. Recovering recall for the rare Python
last-def-shadows case (bind the bare call to the effective/last def) is a
documented **v1 non-goal**. 016-D asks whether to build that recovery now.

## Options weighed

- **Option A — keep fail-closed (CHOSEN).** Zero false edges; recall loss is
  confined to the rare same-file duplicate-name legitimate call, already
  documented as a v1 non-goal.
- **Option B — blanket last-wins.** **Rejected as unsound.** "Last in source
  order" ≠ "effective at the call site" under non-linear redefinition
  (`if/try/with`, decorator-driven rebinding, `@overload` stubs preceding the
  implementation). Worse, the resolver is a **shared** Rust+Python consumer:
  Rust same-name defs in different inline `mod` blocks per file are **distinct
  targets disambiguated by module path, not source order** — blanket last-wins
  would mint a **new** wrong Rust edge, trading one false-edge class for another.
- **Option C — Python-gated conditional last-wins** (last-wins only on provable
  linear module-level redefinition; fail closed on non-linear; Rust unchanged).
  Sound in principle but **complex and precision-risky** for a rare pattern; its
  correctness proof and adversarial-corpus test burden are disproportionate to
  the recall recovered.

## Rationale

1. **013-D (no-false-edge) is the governing invariant; recall is a documented v1
   non-goal**, not a rollback trigger. Fail-closed already satisfies the
   constitution.
2. **Blast radius vs benefit:** the effective code paths touch the shared
   resolver used at 7 call sites across index/sync/resolve for both Rust and
   Python. A precision regression here is high-cost; the recovered recall is
   low-frequency (same-file duplicate top-level defs are rare).
3. **`@overload`/decorator reality:** typed Python routinely has multiple same-name
   `@overload` stubs before the real implementation — precisely the non-linear
   case where naive last-wins is correct-by-luck at best and wrong at worst. A
   sound Option C must special-case this, adding complexity.
4. **No blocker removed by waiting:** `function_meta` already carries `line_start`
   (source order is available), so a future last-wins implementation is not
   blocked by missing data — only by the precision/complexity tradeoff. Deferring
   costs nothing structurally.

## Revisit trigger

Reopen 016-D only when there is **measured recall loss on a real Python corpus**
attributable to this fail-closed drop (especially `@overload`-heavy typed code).
At that point, pursue **Option C** (Python-gated, linear/`@overload`-aware
last-wins, fail-closed on non-linear redefinition, Rust unchanged), reusing
100-F's ambiguity-aware resolver with an additive Python branch, with a mandatory
adversarial same-file duplicate-name regression corpus proving zero false edges.

## Disposition

- 016-D chosen-direction updated to this firm KEEP-FAIL-CLOSED verdict.
- Stash `B94772CB` archived (provenance recorded via comment before archive, per
  `docs/decisions/2026-07-29-stage-harvest-provenance-convention.md`).
- 016-D archived/parked. No shipment.
