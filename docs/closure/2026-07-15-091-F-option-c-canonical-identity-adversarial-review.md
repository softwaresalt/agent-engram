---
title: "Plan-harden + Adversarial Review — 091-F Option C canonical identity"
doc_type: closure
source: "091-F Option C plan + spike 091.001-T; pre-harvest adversarial gate"
description: >-
  Plan-hardening and adversarial review of every material conclusion in the 091.001-T spike and the
  Option C exec plan, before harvest/shipment assembly. Multi-lens adversarial method (parser
  correctness, security, migration, concurrency, performance, scope). HIGH P1 findings remediated
  in-plan; others dispositioned. A formal multi-model adversarial panel is encoded as a required
  Unit-B release gate before edges flip on.
topic: "Adversarial gate for Option C before decomposition ships"
depth: "review"
decision_status: "PROCEED-WITH-REMEDIATIONS — all HIGH P1 folded into plan; Unit-B multi-model panel required at gate"
author: stage
date: 2026-07-15
verdict: PROCEED-WITH-REMEDIATIONS
gate_blocking: true
linked_artifacts:
  - "091-F"
  - "091.001-T"
  - "091.002-T"
  - "docs/decisions/2026-07-15-091-001-canonical-identity-spike.md"
  - "docs/exec-plans/2026-07-15-091-F-option-c-canonical-identity-plan.md"
---

# Plan-harden + Adversarial Review — 091-F Option C

- **Mode:** report-only; no source/test modified. Pre-harvest gate over the spike + plan.
- **Verdict:** **PROCEED-WITH-REMEDIATIONS.** GO stands; six HIGH P1 findings are folded into the plan
  before harvest (D1–D6). No P0 that blocks decomposition, because the additive design + fail-closed
  discipline neutralise the worst classes.

## Method & honesty note

This is a **single-agent (Stage aggregator) multi-lens adversarial review**, each finding re-derived
against the actual origin/main source cited in the spike. It is **not** a live multi-model panel. The
subject is a *plan*, not a diff, so the formal **multi-model adversarial panel** (Anthropic/OpenAI/Google
tiers, as run for the 088 diff in `docs/closure/2026-07-15-088-rec1-call-resolution-adversarial-review.md`)
is **encoded as a required Ship-time release gate for Unit B** before any `calls_resolved_canonical` edge
flips on (proof obligation, see D9 and the plan §7). Lenses applied: parser correctness, security,
migration, concurrency, performance, scope boundaries.

## Findings

| ID | Lens | Sev | Conf | Finding (re-derived vs source) | Remediation / disposition |
|---|---|---|---|---|---|
| **D1** | parser correctness | **P1** | HIGH | Module-path derivation (A1) that assumes the default file→module mapping is **wrong** for `#[path="…"]` `mod` attributes and `#[cfg]`-gated modules. A wrong module path yields a wrong canonical identity → a mis-resolved edge, violating the invariant. tree-sitter sees the attribute but the mapping is non-default. | **REMEDIATE in A1:** honour `#[path]`; treat `#[cfg]`-gated / non-derivable module mappings as **fail-closed** (no `canonical_path` ⇒ never a canonical match). Add `#[path]` + cfg fixtures. |
| **D2** | scope boundary | **P1** | HIGH | The plan/spike implied "method-aware" resolution broadly. Arbitrary-receiver method calls `x.foo()` require **type inference Engram does not have**; canonicalising them is unsound. | **SCOPE CORRECTION (spike+plan+B1/B2):** resolve method calls **only** when the receiver type is statically known — i.e. `self.m()` / `Self::m()` inside an impl. All other `is_method` (`x.foo()`) calls stay **dropped (fail-closed)**. Recall claim narrowed accordingly. |
| **D3** | correctness | **P1** | HIGH | Distinguishing a **workspace** crate root (resolvable) from an **external** dep (`mem`, `tokio`) needs the set of workspace crate names; A3 cannot classify roots without it. Getting this wrong reopens **F1**. | **REMEDIATE in A1/A3:** enumerate workspace crate names (crate roots / Cargo metadata); A3 classifies a leading segment as workspace vs external; **external ⇒ fail-closed**. Fixture: external qualifier never resolves. |
| **D4** | correctness | **P1** | HIGH | Non-Rust / legacy / unresolved defs carry an **empty** `canonical_path` after the additive migration. If B2 ever treats empty as a match key, an empty-qualifier call could collide → false edge. | **REMEDIATE in A6/B2:** empty `canonical_path` is **never** a candidate target; B2 filters empties before the singleton test. Fixture: empty-vs-empty asserts 0 edges. |
| **D5** | migration / concurrency | **P1** | HIGH | The A8 forced re-index on a fingerprint bump could block daemon **open**, double-run under concurrent index, or trip SQLITE_BUSY on large DBs. | **REMEDIATE in A8:** run the re-index in the **normal index path** (not a blocking open hook), behind a **single-flight** guard, using `run_script_retrying` (086.001-T) for BUSY; surface duration; rollback = revert version. |
| **D6** | parser correctness | **P1** | HIGH | `use other::Widget;` alongside a local `struct Widget`/`mod Widget` — Rust name-resolution precedence (explicit item/`use` > glob) must be honoured or the resolver mis-binds the alias. | **REMEDIATE in A3:** implement precedence (local item & explicit `use` over glob-`*`); **fail-closed on genuine ambiguity**. Shadowing fixture. |
| D7 | security | P2 | MED | A workspace module shadowing an external crate root (`mod std { pub fn swap }`) could bind `std::swap()` to the local item. | **DISPOSITION → fold into A3:** when a workspace module name shadows a known external crate root, **fail-closed** unless a leading `::`/`crate::` disambiguates. Adversarial fixture in B3. |
| D8 | performance | P2 | MED | Pathological `pub use` graphs could make the re-export closure (A4) super-linear. | **DISPOSITION → A4:** memoise + explicit **depth/size cap**; exceed ⇒ fail-closed. Cap constant + test. |
| D9 | eval integrity | P2 | HIGH | 088's lesson (F3/F4): a green suite gave false confidence at the precision boundary. | **DISPOSITION → B3/B4:** fixtures must be proven **RED before B2**; the gate must include a **seeded mis-resolution-to-a-real-target** that the old `dangling==0` metric misses. **Plus:** run the formal multi-model panel at the Unit-B gate. |
| D10 | migration | P2 | MED | Rolling back A8 (format/version) while Unit-B canonical edges exist orphans those edges. | **DISPOSITION → plan §8:** rollback **order** = Unit-B retract `calls_resolved_canonical` **before** Unit-A A8 down-migration. Documented. |
| D11 | correctness | P2 | MED | `#[cfg]`-duplicated defs can share a `canonical_path`. | **DISPOSITION:** already handled by fail-closed (≥2 matches ⇒ drop); add an explicit collision fixture in B3. |
| D12 | scope | P3 | LOW | Generic normalisation collapses `Vec<u8>`/`Vec<u16>` to one target — **correct** (same method), but `Foo` vs `Foo<T>` distinct defs must ≥2-drop. | **DISPOSITION:** acceptable; covered by fail-closed ≥2 rule + a normalisation fixture. |

## Plan-harden — elevated blast radius (confirmed)

Option C touches the **DB schema** (`function_meta`, `staged_call` additive columns; `schema_meta`
fingerprint), the **parser** (module tree, use-graph, Self), the **indexer/post-pass**, and the **eval
gate** — multi-family, so hardening is warranted and was performed:

- **Additive-only identity surface** (new `canonical_path`, never overwrite `name`) keeps search /
  `references_edge` / bare-name resolution / JSONL display **unchanged** — the single biggest blast-radius
  reducer. Retained.
- **Two-unit split** isolates the precision-neutral infrastructure (Unit A) from the gated flip-on
  (Unit B). Retained.
- **Fail-closed is the default**, not an exception; every uncertainty path is enumerated (spike §6, plus
  D1/D3/D6/D7). Retained + extended.
- **Hard 084-S dependency** for the durable-staging substrate (avoids double JSONL format churn).
  Retained; reinforced by D-note that B1 extends the **089-F-landed** format additively.

## Disposition summary

- **HIGH P1 (D1–D6): remediated in-plan** — spike §6 fail-closed table extended, and tasks A1/A3/A6/A8
  and B1/B2 scopes updated (this session, before harvest).
- **P2/P3 (D7–D12): dispositioned** into task scopes/fixtures and rollback order; none blocks harvest.
- **No P0.**
- **Required Unit-B gate:** formal multi-model adversarial panel over the Unit-B *diff* + the
  identity-based precision/recall gate green on the adversarial fixtures — **before** edges flip on.

**Gate outcome: PROCEED to harvest + shipment assembly with D1–D6 folded in.**
