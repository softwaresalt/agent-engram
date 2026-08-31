---
title: "An opt-in capability pack can leave unguarded directives and a hard foundation dependency behind when disabled"
description: "Removing the opt-in agent-intercom pack from the engram harness surfaced two upstream autoharness v1.5.0 defects. (1) Autoharness templates are plain {{VAR}} substitution with no conditionals, so pack content renders unconditionally; most of it is self-guarding prose and therefore inert, but review/SKILL.md.tmpl and doc-review/SKILL.md.tmpl emit an UNGUARDED 'Call ping at session start' directive under a NON-NEGOTIABLE heading, plus a gated_auto routing row naming a tool surface that a pack-less install never installs. (2) FOUNDATION_ASSERTIONS.copilot_remote_operator_guidance unconditionally requires a literal '### agent-intercom' heading in copilot-instructions.md, so a workspace that legitimately disables the opt-in pack cannot delete the section without failing a foundation check."
problem_type: "optional_feature_content_not_gated_on_feature_enablement"
category: "harness-correctness"
component: "autoharness templates/skills/{review,doc-review}/SKILL.md.tmpl and src/autoharness/verify_workspace.py FOUNDATION_ASSERTIONS"
root_cause: "the template engine has no conditional construct, so pack-scoped content is authored inline and relies on hand-written 'When the <pack> is installed' prose guards; where an author omitted that guard the directive became unconditional, and a foundation-level verification assertion was written against a heading that only an opt-in pack justifies"
resolution_type: "guard_locally_and_report_upstream"
date: "2026-08-31"
shipment: "n/a — harness maintenance on chore/autoharness-merge-install-20260831"
---

# An opt-in capability pack can leave unguarded directives and a hard foundation dependency behind when disabled

## Problem

`agent-intercom` is an **opt-in** autoharness capability pack. The v1.5.0 registry marks
it `default_in_preset: []` with `mcp_requirements: []` — it is in no preset and ships no
MCP server. Disabling it should therefore be a clean, low-risk operation.

It is not. Disabling it leaves two classes of residue that the harness's own verification
cannot reconcile.

## Root cause

Autoharness templates use plain `{{UPPER_SNAKE}}` substitution with **no conditional
construct**. Pack-scoped content is authored inline in shared templates and depends
entirely on hand-written prose guards of the form:

> When the `agent-intercom` capability pack is installed, …

That convention works — a guarded clause is inert when the pack is absent, and the
rendered bytes are identical whether or not the pack is enabled. The defects are the
places where the convention was **not** followed, and one place where a *verification
assertion* was written against pack content at foundation level.

### Gap A — unguarded directives in two skill templates

Every other intercom clause across ~25 templates is guarded. These are not:

| Upstream file (autoharness @ v1.5.0, `main` = `2661c1c8`) | Line | Content |
|---|---|---|
| `templates/skills/review/SKILL.md.tmpl` | 13 | `Call \`ping\` at session start. If agent-intercom is reachable, broadcast at every step. …` |
| `templates/skills/doc-review/SKILL.md.tmpl` | 38 | same, wrapped |
| `templates/skills/review/SKILL.md.tmpl` | 80 | `` | `gated_auto` | agent-intercom approval | Fix exists but changes behavior/contracts | `` |
| `templates/skills/doc-review/SKILL.md.tmpl` | 80 | `` | `gated_auto` | agent-intercom approval | Fix changes meaning or structure | `` |

In `review/SKILL.md.tmpl` the unguarded sentence sits directly under a
`## Agent-Intercom Communication (NON-NEGOTIABLE)` heading, so a pack-less install ships
a NON-NEGOTIABLE instruction to call a tool that was never installed. The `gated_auto`
rows name `agent-intercom approval` as the **default owner** of an entire approval class,
so with the pack absent that class routes to nothing.

This is internally inconsistent with autoharness's own model: `PACK_ASSERTIONS`
(`src/autoharness/verify_workspace.py`) gates `review_intercom_workflow` on the pack
being enabled, proving the section is understood to be pack-scoped — but the template
emits it unconditionally and without a guard.

### Gap B — a foundation assertion hard-coupled to an opt-in pack

`src/autoharness/verify_workspace.py:454`, inside **`FOUNDATION_ASSERTIONS`** (which is
evaluated unconditionally, unlike `PACK_ASSERTIONS`):

```python
{
    "key": "copilot_remote_operator_guidance",
    "path": ".github/copilot-instructions.md",
    "must_contain": [
        "## Remote Operator Integration",
        "### agent-intercom",
        "### agent-engram",
        "sync_workspace",
    ],
},
```

The literal `### agent-intercom` heading is required of **every** workspace. A workspace
that legitimately disables this opt-in pack cannot remove the section without failing a
foundation check. `### agent-engram` has the same shape; engram happens to enable that
pack, so the bug is latent there rather than active.

The corresponding template, `templates/foundation/copilot-instructions.md.tmpl:197`,
emits the heading unconditionally to satisfy the assertion — so template and checker are
consistent with each other but both wrong with respect to pack enablement.

## Resolution

**In this workspace (engram)** — deliberately minimal, because template-identical content
is *correct* content:

* Left every **guarded** clause untouched (~110 mentions). Before the change, the intercom
  reference count of each file matched the pristine template exactly
  (`_ship.agent.md` 30/30, `AGENTS.md` 8/8, `copilot-instructions.md` 9/9). A fresh install
  without the pack produces byte-identical files, so stripping them would manufacture
  permanent drift that `verify-workspace` re-flags on every tune, for zero behavioral gain.
* Fixed only the genuinely unguarded items — added the standard
  `When the agent-intercom capability pack is installed, …` guard to both skills and
  re-pointed both `gated_auto` owners to `Operator approval`.
* **Kept** the `### agent-intercom` heading required by Gap B, replacing its dangling
  `ping-loop.prompt.md` sentence with an explicit "this pack is **not enabled** in this
  workspace" note that redirects approval to local operator confirmation.

Result: 0 blockers, 0 warnings, **68/68 targeted checks pass**. The total fell 71 → 68
purely because three pack-scoped checks (`agent_intercom_instruction`,
`review_intercom_workflow`, `dark_factory_intercom_contract`) became *not-applicable*.

**Upstream (autoharness)** — the real fix, not applied here because Constitution IV forbids
writing outside the workspace tree:

* Gap A: add the standard prose guard to `review/SKILL.md.tmpl:13` and
  `doc-review/SKILL.md.tmpl:38`; change the `gated_auto` owner at both `:80` to a
  pack-neutral value such as `Operator approval`, mentioning intercom only as the routing
  path *when installed*.
* Gap B: move `### agent-intercom` (and `### agent-engram`) out of `FOUNDATION_ASSERTIONS`
  into the respective `PACK_ASSERTIONS` entries, leaving only
  `## Remote Operator Integration` as the unconditional foundation requirement.
* Structural: the absence of conditionals in the template engine is the underlying cause.
  Until templates can gate on pack enablement, the installer must post-render prune
  disabled-pack sections — which is already the *de facto* expectation, since disabled-pack
  sections (`graphtor-docs`, `browser-verification`) otherwise survive into installed output.

## Lessons

* **An opt-in feature is only genuinely optional if disabling it is verifiable.** Here the
  verifier itself hard-required the feature's content, so "disabled" was not a state the
  harness could represent.
* **Prose guards are a convention, not a mechanism.** When a template engine has no
  conditionals, every guarded clause is one author's discipline away from becoming an
  unconditional directive. Grep for the guard phrase and diff it against the mention count
  to find the misses — the two defects here were exactly the mentions whose lines lacked
  the guard.
* **Check tier encodes intent.** A rule living in a foundation/unconditional assertion list
  versus a pack-scoped one *is* the specification of whether something is optional.
  Contradictions between the two tiers (`review_intercom_workflow` pack-gated while its
  template is unconditional) reliably mark a defect.
* **Template-identical output is a feature.** Resist "cleaning up" inert generated content;
  divergence from the template is drift that costs you on every future tune. Fix only what
  is behaviorally wrong locally, and send the rest upstream.
