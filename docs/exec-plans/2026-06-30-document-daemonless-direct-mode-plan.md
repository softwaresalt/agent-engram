---
title: "Surface & Document the Daemonless --direct Indexing Mode — Plan"
type: plan
date: 2026-06-30
slug: document-daemonless-direct-mode
status: reviewed
source_stash: 20477A6A
related_deliberation: 010-D
related_decisions:
  - docs/decisions/2026-05-08-cli-direct-daemonless-mode-deliberation.md
related_closure:
  - docs/closure/2026-05-08-030-S-cli-direct-mode-closure.md
shipped_feature: 045-F
shipped_shipment: 030-S
---

## Decision Summary

The daemonless `--direct` indexing mode (`engram index --direct`,
`engram sync --full --direct`, and `ENGRAM_DIRECT=1`) shipped in feature
**045-F** / shipment **030-S** as the escape hatch for when daemon startup or
IPC times out. The capability is complete and tested, but it is **under-surfaced
to users**. This plan is **docs-first and width-isolated**: it improves
discoverability and framing without re-deriving the feature.

Deliberation **010-D** already concluded the *implementation* is a duplicate of
045-F/030-S and explicitly flagged the residual "discoverability" gap (Option C,
item c2). This plan harvests exactly that docs residual — it is net-new
documentation work, not a re-implementation.

**Why no separate deliberation for this plan:** The decision space is already
closed. 010-D chose the direction ("document the existing escape hatch; do not
re-build it") and named the concrete surfaces (README, docs/, start scripts,
error output). This plan only sequences the docs work, so a fresh deliberation
would add ceremony without new decisions. A lightweight impl-plan is sufficient.

## Problem Frame

The daemonless indexing path exists in the shipped binary:

* `src/bin/engram.rs` — `Sync { full, direct }` (lines ~85–94) and
  `Index { direct }` (lines ~96–104). Both use
  `#[arg(long, env = "ENGRAM_DIRECT", value_parser = BoolishValueParser::new())]`
  with doc comments, so `engram sync --help` / `engram index --help` render the
  `--direct` flag. The **top-level `engram --help` summary does not signpost
  it**, and the doc comments are the only discoverability surface at the CLI.
* `src/errors/mod.rs` — `DaemonError::NotReady { timeout_ms }` renders
  `"Daemon failed to reach Ready state within {timeout_ms}ms"` (line ~161). This
  is the daemon-startup-timeout message a user hits before reaching for the
  escape hatch, and it does **not** mention `--direct`.

Current documentation coverage (verified 2026-06-30):

| Surface | State | Gap |
|---|---|---|
| `README.md` | **Zero** mentions of `--direct` / `ENGRAM_DIRECT` (QuickStart shows only `engram sync`, line ~62) | Primary discoverability gap |
| `docs/configuration.md` | Terse `ENGRAM_DIRECT` env-var table row (line ~47) + `ENGRAM_DATA_DIR` "direct mode" note (line ~44) + line ~58–59 mention | No cohesive, escape-hatch-framed section; `--direct` flag not explained as a unit |
| `docs/troubleshooting.md` | Line ~70 lists `engram sync --direct --format text` as a debugging step under "Indexing problems" | Debugging-framed, not escape-hatch-framed; omits `engram index --direct` and `ENGRAM_DIRECT=1`; not routed from the daemon-timeout symptom |
| `start.ps1` | **Uses** `engram sync --direct` for pre-warm (lines ~101–102) with a daemon fallback | No comment explaining *why* / no cross-reference to docs |
| `start.sh` | Does **not** invoke engram at all | No commented pointer to the direct pre-warm option |

Reuse the escape-hatch framing already captured in
`docs/decisions/2026-05-08-cli-direct-daemonless-mode-deliberation.md` (30s IPC
timeout, `start.ps1`/`start.sh` pre-warm friction, one-shot CLI usage) and
`docs/closure/2026-05-08-030-S-cli-direct-mode-closure.md`. Do not re-derive it.

## Requirements Trace

| Stash deliverable | Implementation action | Unit |
|---|---|---|
| (1) README quickstart documenting `engram index --direct` / `sync --full --direct` / `ENGRAM_DIRECT=1`, framed as the daemon-timeout / daemonless escape hatch | Add a "Daemonless indexing (`--direct`)" callout to `README.md` QuickStart, cross-linking the docs reference and troubleshooting pages | **Unit 1 (docs)** |
| (2) Improve top-level discoverability: docs/ user guide coverage + cross-reference from start.ps1/start.sh | Consolidate a focused direct-mode section in `docs/configuration.md`; upgrade the daemon-timeout entry in `docs/troubleshooting.md`; add comment cross-references in `start.ps1` and `start.sh` | **Unit 2 (docs)** + **Unit 3 (scripts)** |
| (2) …and/or "mention direct mode in the Index/Sync command help summary" | Enrich the top-level `Index`/`Sync` clap doc-comment summary in `src/bin/engram.rs` | **Unit 4 (code, deferred)** |
| (3) When daemon startup/bind times out, have the CLI/shim error output point the user at `--direct` | Augment `DaemonError::NotReady` message in `src/errors/mod.rs` to suggest `engram index --direct` / `ENGRAM_DIRECT=1`, with a test | **Unit 4 (code, deferred)** |

## Implementation Units

### Unit 1 — README daemonless escape-hatch section (docs)

* **Changes:** Add a short "Daemonless indexing (`--direct`)" subsection near the
  QuickStart in `README.md`. Cover `engram index --direct`,
  `engram sync --full --direct`, and `ENGRAM_DIRECT=1`, framed explicitly as the
  daemon-startup / IPC-timeout escape hatch. Cross-link to
  `docs/configuration.md` (reference) and `docs/troubleshooting.md` (symptom).
* **Files:** `README.md` (1).
* **Tests / verification:** Manual — markdown structure valid, no unresolved
  placeholders, all cross-linked files exist (`docs/configuration.md`,
  `docs/troubleshooting.md`).
* **Execution posture:** docs-first.
* **Depends on:** Unit 2 (anchors it links to must exist for cross-reference
  integrity).

### Unit 2 — User-guide direct-mode coverage (docs)

* **Changes:**
  * `docs/configuration.md`: add a focused "Daemonless / direct indexing"
    subsection that explains the `--direct` flag on `sync`/`index`, the
    `ENGRAM_DIRECT` env var, and the escape-hatch rationale (reusing the
    decision-doc framing). This becomes the canonical anchor other surfaces
    link to.
  * `docs/troubleshooting.md`: upgrade the "Indexing problems" daemon-timeout
    guidance to explicitly route the user to `--direct` / `ENGRAM_DIRECT=1` as
    the escape hatch, covering both `engram index --direct` and
    `engram sync --full --direct`.
* **Files:** `docs/configuration.md`, `docs/troubleshooting.md` (2).
* **Tests / verification:** Manual — headings/anchors resolve, terminology
  matches the shipped flags, no unresolved placeholders.
* **Execution posture:** docs-first.
* **Depends on:** none (root of the docs chain).

### Unit 3 — Start-script cross-references (scripts/docs)

* **Changes:**
  * `start.ps1`: add a comment above the existing `engram sync --direct`
    pre-warm block explaining *why* direct mode is used and pointing to the
    direct-mode docs. **No runtime behavior change.**
  * `start.sh`: add a commented note documenting the direct pre-warm option and
    linking the docs (start.sh does not currently invoke engram, so this is a
    documentation comment only, not a behavior change).
* **Files:** `start.ps1`, `start.sh` (2).
* **Tests / verification:** Manual — scripts still parse; comments only; links
  resolve. `pwsh -NoProfile -Command "$null = [ScriptBlock]::Create((Get-Content -Raw ./start.ps1))"` parse check optional.
* **Execution posture:** docs-first (comment-only edits).
* **Depends on:** Unit 2 (links target the canonical docs anchor).

### Unit 4 — Point daemon-timeout error at `--direct` + top-level help (code, DEFERRED)

* **Changes:**
  * `src/errors/mod.rs`: augment the `DaemonError::NotReady` `#[error(...)]`
    string (line ~161) to suggest the escape hatch, e.g. append
    `"; if startup keeps timing out, run 'engram index --direct' (or set ENGRAM_DIRECT=1) to index without the daemon."`
  * `src/bin/engram.rs` (optional, same domain): extend the top-level
    `Index`/`Sync` doc-comment summary so `engram --help` signposts direct mode.
* **Files:** `src/errors/mod.rs`, optionally `src/bin/engram.rs`, plus one test
  file (≤3 files).
* **Tests / verification:** **Test-first** — add/adjust a unit or contract test
  asserting the augmented `NotReady` message. Full quality gates apply
  (`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings
  -D clippy::pedantic`, `cargo test --all-targets`, `cargo audit`). CLI help
  snapshot/contract tests may need updating if the help summary changes.
* **Execution posture:** test-first.
* **Depends on:** none functionally; conceptually pairs with the docs so the
  message and docs use consistent wording.
* **Caveat:** Do **not** alter the `ENGRAM_DIRECT` `BoolishValueParser` (see
  `docs/compound/clap-bool-env-var-boolish-value-parser-2026-05-08.md`) — the
  bool env parser is required and out of scope here.
* **Width-isolation decision:** This unit touches compiled Rust (a different
  skill domain than docs) and is bound by Test-First + cargo gates. It is
  therefore **kept out of the docs shipment** and harvested as a separate
  `queued` code task labeled `deferred`. Ship can pick it up in its own code
  shipment. This satisfies the stash guidance: "if (3) requires touching Rust
  CLI/shim code, make it a SEPARATE code task … defer it as a separate
  queued/blocked task rather than bloating the docs shipment."

## Dependency Graph

```text
Unit 2 (docs canonical anchors)
  ├──> Unit 1 (README links to anchors)
  └──> Unit 3 (start scripts link to anchors)

Unit 4 (code, deferred) — independent; excluded from the docs shipment
```

No cycles. Unit 2 is the single upstream for the docs chain; Units 1 and 3 are
parallel downstreams once Unit 2 lands.

## Constitution Check

Mapped against the autoharness/agent-engram constitution (`AGENTS.md`,
`.github/instructions/constitution.instructions.md`):

| Principle | Applies? | Compliance |
|---|---|---|
| I. Safety-First Rust (`#![forbid(unsafe_code)]`, `Result<T, EngramError>`, no `unwrap`/`expect`, clippy pedantic) | Unit 4 only | Unit 4 is a `#[error]` string edit + optional doc-comment; introduces no unsafe, no new fallible paths, no `unwrap`/`expect`. Units 1–3 touch no Rust. ✅ |
| II. Test-First Development (NON-NEGOTIABLE) | Unit 4 only | Unit 4 posture is test-first: a unit/contract test asserting the augmented `NotReady` message precedes the change. Units 1–3 are docs; verification is markdown-structure + cross-reference checks (no compiled code). ✅ |
| III. Workspace Isolation / path traversal | No | No filesystem-path logic changes. ✅ |
| IV. CLI Workspace Containment | Yes | All edits are inside the repo tree (`README.md`, `docs/`, `start.ps1`, `start.sh`, `src/`). No out-of-tree writes. ✅ |
| V. Destructive Command Approval | No | No destructive commands; docs/comment/string edits only. ✅ |
| VI. Safety Modes for Risky Work | No | No elevated blast radius (see Plan Hardening Signals). ✅ |
| Task Granularity — 2-Hour Rule (≤3 files, ≤5 functions, ≤4 tests) | Yes | Unit 1: 1 file; Unit 2: 2 files; Unit 3: 2 files; Unit 4: ≤3 files, 1 message change, 1 test scenario. ✅ |
| Task Granularity — Width Isolation (single skill domain/task) | Yes | Docs units (1,2) and script-comment unit (3) are separate from the compiled-Rust unit (4); Unit 4 is excluded from the docs shipment. ✅ |
| Task Granularity — Atomic Milestone | Yes | Each unit yields a verifiable state change (a rendered doc section, script comment, or asserted error message). ✅ |
| Quality Gates (fmt/clippy/test/audit) | Unit 4 only | Unit 4 runs all four gates under Ship; docs units are exempt (no compiled artifacts). ✅ |
| Conventional commits (`docs:` / `feat:`/`fix:`) | Yes | Units 1–3 → `docs:`; Unit 4 → `fix:`/`feat:` when Ship builds it. ✅ |

No constitutional violations. The only compiled-code touch (Unit 4) is deferred,
test-first, and gated; it is deliberately isolated from the docs shipment.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Skip a fresh deliberation | 010-D already closed the decision and named the surfaces; only sequencing remains |
| Parent is a `feature` labeled `chore,docs` | backlogit has no `chore` artifact type; `feature` is the level-1 container, `chore` is the stash kind captured as a label |
| Unit 2 is the canonical anchor, harvested first | Guarantees cross-reference integrity for README + start-script links |
| `configuration.md` is the docs home (not a new page) | It already owns CLI flags + env vars; consolidating there avoids doc sprawl and a redundant page |
| Deliverable (3) split into a separate deferred code task | Width isolation: compiled Rust + Test-First gates must not ride in a docs shipment |
| Reuse decision/closure framing verbatim where possible | Avoids re-deriving the escape-hatch rationale; keeps docs consistent with shipped intent |

## Risks and Caveats

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Docs drift from shipped flag semantics | Low | Medium | Ground every example in `src/bin/engram.rs`; note `index --direct` == `sync --full --direct` |
| Overstating "docs have zero coverage" | — | Low | Plan records the existing terse mentions; work is framed as consolidation + README net-new, not first-ever coverage |
| Cross-reference breakage | Low | Low | Unit 2 lands first; Units 1/3 link to established anchors; verify all referenced files exist |
| Unit 4 CLI help snapshot tests break | Low | Low | Deferred + test-first; Ship updates snapshots under cargo gates |
| Scope creep into read-only `--direct` or auto-detect | Low | Medium | Explicitly out of scope (per 010-D and the decision doc's rejected options) |

## Plan Hardening Signals (REQUIRED)

* **Public API, schema, or contract change:** Absent for Units 1–3 (docs/comment
  only). Unit 4 changes a user-visible *error string* and optionally CLI help
  text — not a structured contract, but snapshot/contract tests may assert on
  it. Low blast radius, deferred, test-first.
* **Security, auth, permission, or compliance-sensitive behavior:** Absent.
* **Migration, backfill, destructive/irreversible action:** Absent.
* **External integration, operator checkpoint, external dependency:** Absent.
* **High runtime, rollout, or rollback risk:** Absent — docs and scripts are
  comment-only; Unit 4 is a message-string change behind full cargo gates and is
  trivially revertible.

**Requires plan hardening: no.** Docs-first, width-isolated, no elevated blast
radius. The single code touch (Unit 4) is a deferred, test-gated string change,
not a schema/CLI-distribution/multi-template change.

## Runtime Verification and Closure

* **Units 1–3 (docs/scripts):** No runtime surface change. Verification =
  markdown/structure validity, no unresolved `{{...}}` placeholders, and
  cross-reference integrity (all linked files/anchors exist). Closure = the
  documented commands match `src/bin/engram.rs` exactly.
* **Unit 4 (code, deferred):** Changes the CLI/daemon error-output runtime
  surface. Verification = a test asserting the augmented `NotReady` message plus
  a manual check that a forced daemon-startup timeout prints the `--direct`
  hint. Closure = full quality gates green; note in the code shipment that the
  message wording tracks the docs.

## Plan Review

Reviewed 2026-06-30 by the Stage agent via the `plan-review` gate. Personas:
Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor, Learnings
Researcher (single-model, always-on set; no cross-model escalation warranted for
a docs-first plan with one deferred string change).

### Findings

| ID | Severity | Persona | Finding | Disposition |
|---|---|---|---|---|
| F1 | **P1** | Constitution | Task requirement #1 mandates an explicit "Constitution Check" section; the plan expressed it only implicitly via Plan Hardening + Runtime Verification. | **Resolved** — added a dedicated `## Constitution Check` section before harvest. |
| F2 | P2 | Rust | `NotReady` is not the only timeout surface; `DaemonError`/`IpcTimeout` (`src/errors/mod.rs` ~149) can also fire. Unit 4 should either scope explicitly to `NotReady` or also cover the IPC timeout. | **Accepted** — Unit 4 is scoped to `NotReady` (the daemon-*startup* timeout the stash targets); IPC-timeout wording noted as an optional follow-up in the Unit 4 code task, not a blocker. |
| F3 | P2 | Rust | Augmenting a `thiserror` `#[error]` format string risks brace-interpolation errors and may break CLI help/error snapshot tests. | **Accepted** — hint text contains no `{`/`}`; snapshot-test risk already recorded in Risks table; Unit 4 is test-first under cargo gates. |
| F4 | P3 | Scope Boundary | Unit 3 edits scripts (`start.ps1`/`start.sh`) alongside markdown docs — arguably a sub-domain boundary. | **Advisory** — comment-only, no runtime/behavior change, no compiled artifacts; kept as its own task so it can be reviewed/shipped independently. |
| F5 | P3 | Learnings | Closure doc's compound note #1 ("use `value_parser!(bool)` not `BoolishValueParser`") conflicts with the shipped code (which uses `BoolishValueParser`). Do not propagate that inconsistency into new docs. | **Advisory** — new docs describe user-facing flags/env only, not the internal parser; Unit 4 caveat already forbids changing the parser. |

### Gate Decision

**PASS.** The single blocking finding (F1, P1) was resolved before harvest by
adding the `## Constitution Check` section. Remaining findings are P2/P3
(accepted with scoping notes or advisory) and do not block decomposition. No P0
findings. Plan is cleared for harvest.
