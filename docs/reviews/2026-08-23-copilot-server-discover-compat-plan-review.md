---
title: Plan review — pre-initialize `server/discover` compatibility
date: 2026-08-23
type: plan-review
status: approved-with-changes
reviewer: stage (adversarial gate)
plan: docs/exec-plans/2026-08-23-copilot-server-discover-compat-plan.md
source: docs/decisions/2026-08-23-copilot-prerelease-server-discover-mcp-compatibility-spike.md
cycles: 1
---

## Verdict

**Approved with changes.** One review-fix cycle was applied. Findings F1, F2,
and F4 were folded back into the plan before harvest. F3 and F5 are accepted as
task-level acceptance notes rather than plan changes.

## Scope Check

| Gate | Result |
|---|---|
| Plan traces to an authoritative investigation | Pass — spike artifact linked, evidence not re-derived |
| Plan avoids the rejected fix (timeout increase) | Pass — explicitly out of scope |
| Cozo cold start excluded from this shipment | Pass — routed to an independent spike item |
| Every unit fits the 2-hour rule | Pass — U1–U5 each scoped ≤ 2h |
| Width isolation (no transport work mixed with DB work) | Pass |
| Test-first posture | Pass — U1 RED precedes U2 GREEN |
| No new dependency | Pass — `serde_json` already present |
| Rollback documented | Pass after F1 |

## Findings

### F1 — Major: kill-switch was unnamed

The rollback section promised an environment-variable kill-switch but never
named it, leaving the contract untestable and the runbook unactionable.

**Resolution (applied):** named `ENGRAM_MCP_PREINIT_COMPAT`, default enabled,
set to `0` to restore strict rmcp ordering. Named in U2 and U5.

### F2 — Major: over-broad pre-initialize interception

The original design answered `-32601` to *any* id-bearing pre-`initialize`
method. That is a silent behavior change well beyond the observed defect. rmcp
may legitimately handle other pre-handshake methods (for example `ping`), and
blanket interception would replace rmcp's own error semantics with the shim's
for methods that were never broken. It also risks masking genuine client
ordering bugs that the current strict path surfaces.

**Resolution (applied):** the filter allowlist is narrowed to `server/discover`
only. Every other frame — including unknown id-bearing methods — is forwarded to
rmcp unchanged, preserving existing semantics. This shrinks blast radius to
exactly the reproduced defect.

### F3 — Minor: RED test determinism on Windows

U1 asserts that the shim "does not exit". Absence-of-exit is a non-event and is
easy to assert flakily. The test needs a bounded, deterministic positive signal
rather than a sleep.

**Disposition:** accepted as a task acceptance note on the U1 task — the RED
test must assert the positive `-32601` response frame and the subsequent
`initialize` result within a bounded timeout, and must not rely on a bare sleep.

### F4 — Major: JSON-RPC id `0` round-trip is unverified

The reproduced evidence shows Copilot using request id **exactly `0`**. Zero is
the classic falsy-id serialization bug: implementations routinely coerce it to
absent/null and emit a response the client cannot correlate, or misclassify the
request as a notification and drop it. The plan asserted a `-32601` response but
never asserted that `id: 0` is echoed as the numeric literal `0`.

**Resolution (applied):** U1/U4 must assert the error response carries
`"id": 0` as a JSON number, and that an id-less pre-initialize notification
still produces no response frame. Added to the requirements trace.

### F5 — Advisory: brittle 20-tool literal

U3 hardcodes the expectation of 20 tools. The repository already has an
independent MCP catalog oracle (123-S, hardened by 129-F). Hardcoding a count
duplicates a weaker oracle and will churn whenever the catalog changes.

**Disposition:** accepted as a task acceptance note on the U3 task — assert
catalog integrity by delegating to the existing catalog oracle where practical,
and treat the count as a smoke assertion only.

## Residual Risk After Review

* Whether stable Copilot `1.0.80` also emits `server/discover` remains unproven
  locally. This does not block: the filter is a no-op for clients that never
  send the probe.
* `server/discover` semantics remain undocumented upstream. Answering `-32601`
  is the conservative choice and matches the behavior of the GitHub MCP server,
  which Copilot demonstrably tolerates in the same run.
* No public github/copilot-cli issue exists to track upstream resolution; U5
  should record the prerelease provenance so the compatibility layer can be
  retired deliberately rather than forgotten.
