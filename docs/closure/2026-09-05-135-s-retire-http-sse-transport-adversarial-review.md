---
title: 135-S Retire HTTP and SSE Transport Surfaces — Adversarial Review
description: Multi-model consensus review of range dca08a4c..HEAD (commits 3d9b9976, c5f85ddd, e7a53729, 28a55ca3, 13317b81) on branch feat/135-s-retire-http-and-sse-transport-surfaces.
---

## Method note (tooling constraint, disclosed for transparency)

This session had no `bash`/`git`/`grep` execution available to the orchestrator
or any dispatched sub-agent — only the `view` file-reading tool. `git diff
dca08a4c..HEAD` could not be executed. The review was therefore conducted by:

1. Directly viewing the current (HEAD) state of every file the operator's task
   description and stash record (`9A7C9F8F`) identified as owned by 135-S, and
   independently confirming each of the four tasks' stated deliverables against
   that ground truth (deletions present, feature/deps removed, ADRs marked
   superseded, doc sections updated).
2. Dispatching **3 independent reviewer passes** (Tier 1 `gpt-5.4-mini`, Tier 2
   `gpt-5.5`, Tier 3 `claude-opus-5`), each given the same task/commit context
   and the same review brief, each independently using `view` to inspect the
   repository and return structured JSON findings.
3. Aggregating reviewer output with the orchestrator's own direct findings
   (config/CLI-flag dead-surface tracing, `hyper` dependency-chain check,
   CI workflow check) per the standard confidence-weighting protocol.

No `cargo check`/`cargo test` could be run in this session. Per the operator's
instructions, the confirmed pre-existing `otlp-export`/`opentelemetry_sdk`
API-drift break (stash `9A7C9F8F`) is **out of scope** and is not re-flagged
below.

---

## Ground truth confirmed directly (not disputed by any reviewer)

- `src/server/mcp.rs`, `router.rs`, `sse.rs` are deleted; `src/server/mod.rs`
  now declares only `observability` and `state`, with an accurate doc comment
  describing the three-surface model and citing ADR-0016 (superseded).
- `tests/integration/connection_test.rs` is deleted and has no `[[test]]`
  entry in `Cargo.toml`.
- `tests/contract/lifecycle_test.rs` and `tests/integration/benchmark_test.rs`
  contain **no** remaining `#[cfg(feature = "legacy-sse")]` code — the orphaned
  test removal claimed by 142.023-T is confirmed complete.
- Root `Cargo.toml` has no `legacy-sse` feature and no `axum`/`tower`/
  `tower-http`/`tokio-stream` dependency; `sysinfo` is retained.
- `tests/contract/supported_transport_surface_test.rs` is a real, substantive
  4-test file (not a placeholder) covering the three live surfaces plus a
  manifest-content regression guard.
- `src/installer/templates.rs` and the doc comments in `src/installer/mod.rs`
  no longer render an `http://127.0.0.1:{port}/mcp` URL; `docs/architecture.md`'s
  **"Compatibility note"** (end of file) is accurate and correctly describes
  the three surfaces.
- `docs/adrs/0016-legacy-sse-feature-gate.md` and
  `docs/adrs/0003-sliding-window-rate-limiter.md` both carry accurate,
  well-written **"Supersession"** sections without deleting original content.
- `tests/integration/installer_test.rs` `s064_fresh_install_creates_hook_files`
  and `s068_custom_port_in_hook_urls` were confirmed rewritten with real,
  non-tautological assertions (`!contains("http://")`, both default- and
  custom-port cases checked, `stdio MCP` presence asserted).
- `.github/workflows/ci.yml` has no residual `legacy-sse` feature reference in
  any CI command — clean.
- `hyper` remains in `Cargo.lock` legitimately via `fastembed → hf-hub →
  reqwest` and `opentelemetry-otlp → tonic`, **not** via axum/tower-http — this
  is unrelated to the retired transport and not a residual-reference bug.

**The four tasks' core, explicitly-scoped deliverables are correctly and
completely implemented.** All findings below are about surface area **outside**
the four tasks' literal file list that the shipment's own stated goal
("retire HTTP and SSE transport surfaces") implies should also have been swept,
but wasn't.

---

## Consensus findings (confidence: HIGH — flagged by all 3 reviewers)

### 1. `src/config/mod.rs` — orphaned `Config::port` / `ENGRAM_PORT` surface
**Severity: MAJOR | Confidence: HIGH (3/3) | P1**

`Config::port` (`#[arg(long, env = "ENGRAM_PORT", default_value_t = 7437)]`,
doc comment `"Port for the HTTP/SSE server"`) is still a fully live, clap-parsed
CLI flag and environment variable. Independently traced (orchestrator +
Reviewer-C): it is read nowhere in `src/bin/engram.rs`, `src/daemon/*`,
`src/shim/*`, or `src/server/state.rs`. It is orphaned configuration for a
transport that no longer exists in any build.

- **Fix:** Remove the `port` field, its env binding, and any validation
  branch that depends on it (Reviewer-C also flagged a `port == 0` validation
  rule and a `defaults_are_sensible` test asserting `cfg.port == 7437` that
  should go with it), or explicitly mark it inert/no-op with a rationale
  comment matching the style already used for `installer::DEFAULT_PORT`.

### 2. `src/bin/engram.rs` — stale `install --port` CLI help text
**Severity: MAJOR | Confidence: HIGH (3/3) | P1**

The `install --port` flag's clap doc comment (surfaced verbatim by
`engram install --help`) still reads to the effect of "MCP HTTP endpoint port
to embed in hook file URLs." Hook templates no longer render any port/URL
(142.025-T removed that), so the binary's own `--help` output now advertises
behavior the same shipment deliberately deleted.

- **Fix:** Update the help text to state the flag is retained for
  backward-compatible parsing only and is never rendered into hook content,
  mirroring the rationale already documented on `InstallOptions::port` /
  `DEFAULT_PORT`, or hide it with `#[arg(hide = true)]` pending removal.

### 3. `README.md` — stale "Transport note" section
**Severity: MAJOR (escalated from mixed MINOR/MAJOR/MAJOR by most-conservative rule) | Confidence: HIGH (3/3) | P1**

The file's trailing `## Transport note` section states: *"HTTP/SSE exists only
as an optional compatibility path behind the `legacy-sse` feature and should
not be treated as the default setup."* This directly contradicts the
shipment: the feature and all its code were **fully deleted**, not merely
feature-gated. This is the single most user-visible surface in the repo and
it was missed by the repo-wide sweep that 142.025-T's stated scope implies.

- **Fix:** Replace with a statement naming the three supported surfaces
  (direct daemon IPC, `engram` CLI over IPC, stdio MCP via `engram shim`) and
  stating HTTP/SSE was removed in 135-S (cf. `docs/architecture.md`'s
  "Compatibility note", which already has correct wording to copy from).

### 4. `docs/configuration.md` — `ENGRAM_PORT` documented as a working compatibility transport
**Severity: MAJOR | Confidence: HIGH (3/3) | P1**

Three separate passages are stale: the overview ("Treat HTTP/SSE settings as
compatibility settings rather than the main path"), the env-var reference
table (`ENGRAM_PORT | Compatibility transport | Port used by the optional
legacy HTTP/SSE path`), and an `[!IMPORTANT]` callout ("`ENGRAM_PORT` still
exists because the compatibility transport is feature-gated"). The feature
gate was removed entirely; none of this is true anymore.

- **Fix:** Remove the `ENGRAM_PORT` row and callout, and correct the overview
  sentence. If `Config::port` is retained per finding #1 pending a follow-up
  removal, document it explicitly as inert rather than as a working setting.

### 5. `docs/troubleshooting.md` — instructs operators to check a deleted build feature
**Severity: MAJOR | Confidence: HIGH (3/3) | P1**

The symptom table row *"Old HTTP/SSE instructions fail → Verify whether you
actually built with `legacy-sse` → Prefer the stdio shim path unless
compatibility requires otherwise"* tells an operator to check for a Cargo
feature that will now hard-error (`error: Package does not have feature
legacy-sse`) if attempted, and implies a working fallback that no longer
exists.

- **Fix:** Rewrite the row: HTTP/SSE was removed in 135-S; there is no
  build-time way to re-enable it; reconfigure the client as a stdio MCP
  server (`engram shim`).

### 6. `docs/workflows.md` — `legacy-sse` listed as a live non-default feature
**Severity: MAJOR (escalated; two reviewers said MINOR, one said MAJOR) | Confidence: HIGH (3/3) | P2**

*"`cargo dev-test` and `cargo full-test` run under default features, so they
skip targets gated on non-default features (`git-graph`, `legacy-sse`,
`otlp-export`)."* `legacy-sse` no longer exists as a feature at all (and its
one gated test target, `connection_test.rs`, is deleted), so this both names
a nonexistent feature and implies untested coverage that doesn't exist.

- **Fix:** Drop `legacy-sse` from the parenthetical, leaving `(`git-graph`,
  `otlp-export`)`.

---

## Majority findings (confidence: MEDIUM — flagged by 2 of 3 reviewers)

### 7. `tests/integration/installer_test.rs` — stale scenario-index comment contradicts the rewritten test
**Severity: MINOR | Confidence: MEDIUM (2/3 — Reviewer-B, Reviewer-C) | P2**

The `// ── US5 Agent Hook Tests (T043) ──` block (~line 571) still lists
`"S068: custom port substituted into hook file URLs"`. 142.025-T inverted the
actual `s068_custom_port_in_hook_urls` test to assert the **opposite** — that
no port/URL is ever rendered. The scenario index and the test's own (updated,
correct) docstring a few lines below now describe contradictory contracts in
the same file the commit touched. The function name itself
(`s068_custom_port_in_hook_urls`) is also now a misnomer.

- **Fix:** Update the bullet to "custom port accepted but never rendered into
  hook content (no HTTP endpoint advertised)"; consider renaming the test
  function to `s068_custom_port_not_rendered_in_hooks` in a follow-up (out of
  scope to force a rename in this pass, but worth tracking).

### 8. `src/server/state.rs` — SSE-era `RateLimiter` left as dead code after its only caller was deleted
**Severity: MINOR (per both reviewers; see note) | Confidence: MEDIUM (2/3 — Reviewer-B, Reviewer-C) | P2**

ADR-0003's supersession text is accurate that the sliding-window rate limiter
"no longer has a live call site" — but the implementation (`RateLimiter`
struct, its constructor, `check_and_record`, and the `AppState.rate_limiter`
field constructed by `AppState::with_mode`) remains in production code with
no remaining reader. This is the exact "dead code suppressed / left in place"
condition that motivated ADR-0016 in the first place (project constitution
Principle VI, "No dead code") — now reintroduced by the shipment that retired
the surface this code protected.

- **Note on severity:** both reviewers rated this MINOR; the orchestrator's
  own judgment is that it deserves a closer look given it's a direct
  constitution-principle violation the ADR chain itself calls out, but per
  protocol (no severity conflict between the two reviewers who flagged it)
  it is reported at MINOR/MEDIUM confidence rather than escalated.
- **Fix:** Delete `RateLimiter` and the `AppState.rate_limiter` field in a
  fast follow-up task scoped explicitly to that cleanup (this file was
  outside 135-S's owned-file list, so doing it inside this shipment would
  itself have been scope creep — correctly, no reviewer suggested fixing it
  in-place here).

---

## Unique findings (confidence: LOW — flagged by exactly 1 of 3 reviewers)

### 9. `src/lib.rs` — possibly-stale `hyper=info` default tracing directive
**Severity: MINOR | Confidence: LOW (1/3 — Reviewer-C) | P3, likely non-issue**

`EnvFilter::new("engram=debug,hyper=info")` was flagged as a leftover from the
axum/tower-http transport. **Orchestrator rebuttal:** independently confirmed
via `Cargo.lock` that `hyper` remains a legitimate transitive dependency via
`fastembed → hf-hub → reqwest` and (when `otlp-export` is enabled)
`opentelemetry-otlp → tonic` — chains entirely unrelated to the retired
HTTP/SSE transport. This directive most likely predates 135-S and was never
tied to axum. Recommend **not acting** on this without further investigation
into whether `hyper` spans were ever observed from the fastembed download
path; not a 135-S regression.

### 10. `tests/contract/supported_transport_surface_test.rs` — manifest guard is a fragile substring match
**Severity: MINOR | Confidence: LOW (1/3 — Reviewer-C) | P3**

`legacy_http_sse_transport_is_fully_retired` asserts on raw `Cargo.toml` text
(`!manifest.contains("axum")`, `!manifest.contains("tower =")`, etc.). This is
whitespace/form-sensitive (a differently-formatted re-addition could slip
through) and proves nothing about whether `src/server/{mcp,router,sse}.rs`
themselves still exist — the modules could theoretically be restored and this
test would still pass.

- **Suggested fix (non-blocking):** parse the manifest with the already
  vendored `toml` crate and assert against the parsed `[features]`/
  `[dependencies]` key sets; optionally assert the three module files are
  absent from disk.

### 11. `tests/contract/supported_transport_surface_test.rs` — `cli_runner_routes_over_ipc_not_http` is a weak source-text assertion
**Severity: MINOR | Confidence: LOW (1/3 — Reviewer-C) | P3**

This test only inspects `cli/runner.rs`'s raw text for the substrings
`ipc_endpoint`/`IpcRequest` (positive) and `axum`/`http://` (negative) rather
than exercising behavior. It is the weakest of the four tests relative to
what its name claims, though it is still a legitimate, non-tautological
regression guard (unlike the placeholder it replaced) and not a blocking
concern.

- **Suggested fix (non-blocking):** assert behaviorally — resolve the
  runner's transport endpoint for a temp workspace and assert it equals
  `ipc_endpoint(workspace)`, matching the approach already used in
  `direct_ipc_endpoint_is_not_http`.

---

## Explicit answers to the review's targeted questions

**Correctness beyond `cargo check`:** Yes — findings #1 and #2 (orphaned
`Config::port`/`ENGRAM_PORT` and stale CLI help text) are real dangling
references to the deleted transport that a compiler cannot catch (they are
valid, reachable Rust; the field is simply never read). Finding #8 is
confirmed genuine dead code (unused struct + field) that also compiles clean.

**Scope discipline (P-021):** No violations found. Every file touched by the
five commits, per direct inspection, falls within the operator's declared
owned-files list (stash `9A7C9F8F`). All findings in this report are things
the commits **did not touch** but arguably should have under the shipment's
stated goal — this is a completeness gap, not scope creep. Notably, no
reviewer nor the orchestrator found any commit touching an out-of-scope file.

**Test quality:** `supported_transport_surface_test.rs` and the two rewritten
`installer_test.rs` cases are real, meaningful, non-tautological tests — a
genuine upgrade from a placeholder. Two minor test-robustness critiques (LOW
confidence, findings #10–11) are worth a future pass but are not blocking.

**Documentation accuracy:** `docs/architecture.md`'s Compatibility note and
both ADRs are accurate. However, the repo-wide sweep implied by "retire HTTP
and SSE transport surfaces" was **not completed** — README.md,
`docs/configuration.md`, `docs/troubleshooting.md`, and `docs/workflows.md`
all still contain factually incorrect claims that HTTP/SSE exists as an
optional, feature-gated compatibility path (findings #3–6, all HIGH
confidence).

**ADR-0003 security reasoning:** **Sound.** Confirmed independently
(orchestrator: no `TcpListener`/`SocketAddr` bind anywhere in `src/`;
`ipc_endpoint()` resolves only to a Unix domain socket or Windows named pipe)
and by Reviewer-C (explicit `src/daemon/ipc_server.rs` check): none of the
three live surfaces accepts an unauthenticated inbound network connection in
the sense FR-025 addressed, so retiring the sliding-window connection-rate
limiter introduces no new exposure. The only caveat is finding #8 — the
limiter's code was left in place as dead code rather than removed.

**Residual references:** Confirmed present in README.md, `docs/configuration.md`,
`docs/troubleshooting.md`, `docs/workflows.md` (findings #3–6), plus the two
orphaned CLI/config surfaces (findings #1–2). Confirmed **absent** from
`.github/workflows/ci.yml`, `AGENTS.md`, `CHANGELOG.md`, `docs/quickstart.md`,
`docs/mcp-tool-reference.md`, and `docs/cli-mcp-parity.md`.

---

## Remediation plan (ordered by confidence × severity)

| # | Finding | Confidence | Severity | Priority score | Action class |
|---|---|---|---|---|---|
| 1 | `src/config/mod.rs` orphaned `Config::port`/`ENGRAM_PORT` | HIGH (3) | MAJOR (3) | 9 | `gated_auto` — confirm before removing a public CLI flag |
| 2 | `src/bin/engram.rs` stale `--port` help text | HIGH (3) | MAJOR (3) | 9 | `safe_auto` — doc-comment-only edit |
| 3 | `README.md` stale Transport note | HIGH (3) | MAJOR (3) | 9 | `safe_auto` — doc-only edit |
| 4 | `docs/configuration.md` stale `ENGRAM_PORT`/HTTP-SSE claims | HIGH (3) | MAJOR (3) | 9 | `safe_auto` — doc-only edit |
| 5 | `docs/troubleshooting.md` stale `legacy-sse` symptom row | HIGH (3) | MAJOR (3) | 9 | `safe_auto` — doc-only edit |
| 6 | `docs/workflows.md` stale `legacy-sse` feature mention | HIGH (3) | MAJOR (3) | 9 | `safe_auto` — doc-only edit |
| 7 | `installer_test.rs` stale S068 scenario-index comment | MEDIUM (2) | MINOR (2) | 4 | `safe_auto` — comment-only edit |
| 8 | `src/server/state.rs` dead `RateLimiter` | MEDIUM (2) | MINOR (2) | 4 | `manual` — needs its own scoped task, not a doc edit |
| 9 | `src/lib.rs` `hyper=info` directive | LOW (1) | MINOR (2) | 2 | `advisory` — likely non-issue, do not act without further investigation |
| 10 | Fragile manifest substring test guard | LOW (1) | MINOR (2) | 2 | `advisory` |
| 11 | Weak `cli_runner` source-text assertion | LOW (1) | MINOR (2) | 2 | `advisory` |

## Backlog work items (P0/P1 findings)

```yaml
type: bug
title: "orphaned-http-port-config: Config::port / ENGRAM_PORT survives HTTP/SSE deletion"
description: "src/config/mod.rs's Config.port field (CLI --port / env ENGRAM_PORT, doc comment \"Port for the HTTP/SSE server\") is read nowhere in production code after 135-S deleted the HTTP/SSE transport it configured. Confirmed orphaned by tracing src/bin/engram.rs, src/daemon/*, src/shim/*, src/server/state.rs."
file: "src/config/mod.rs"
line: 34
severity: "MAJOR"
confidence: "HIGH"
fix: "Remove the port field, ENGRAM_PORT binding, and any port==0 validation branch, or mark it explicitly inert with a rationale comment matching installer::DEFAULT_PORT's style."
linked_review: "docs/closure/2026-09-05-135-s-retire-http-sse-transport-adversarial-review.md"
```

```yaml
type: bug
title: "stale-cli-help: engram install --port help text still describes an HTTP endpoint"
description: "src/bin/engram.rs's install --port clap doc comment (surfaced by `engram install --help`) still describes embedding an MCP HTTP endpoint port in hook file URLs. Hook templates no longer render any port/URL since 142.025-T."
file: "src/bin/engram.rs"
line: 44
severity: "MAJOR"
confidence: "HIGH"
fix: "Update the help text to state the flag is retained for backward-compatible parsing only and is never rendered into hook content."
linked_review: "docs/closure/2026-09-05-135-s-retire-http-sse-transport-adversarial-review.md"
```

```yaml
type: bug
title: "docs-stale-transport-claim: README.md Transport note still describes HTTP/SSE as an optional feature-gated path"
description: "README.md's trailing '## Transport note' section states HTTP/SSE exists as an optional compatibility path behind legacy-sse. The feature and all its code were fully deleted by 135-S, not merely feature-gated."
file: "README.md"
line: 133
severity: "MAJOR"
confidence: "HIGH"
fix: "Replace with a statement naming the three supported surfaces (direct IPC, CLI over IPC, stdio MCP via engram shim) and note HTTP/SSE was removed in 135-S."
linked_review: "docs/closure/2026-09-05-135-s-retire-http-sse-transport-adversarial-review.md"
```

```yaml
type: bug
title: "docs-stale-transport-claim: docs/configuration.md documents ENGRAM_PORT as a working legacy HTTP/SSE compatibility setting"
description: "docs/configuration.md's overview, env-var reference table, and an [!IMPORTANT] callout all describe ENGRAM_PORT and an HTTP/SSE compatibility transport as still existing/feature-gated. Both were fully removed by 135-S."
file: "docs/configuration.md"
line: 31
severity: "MAJOR"
confidence: "HIGH"
fix: "Remove the ENGRAM_PORT row/callout and correct the overview sentence; if Config::port is retained pending removal, document it as inert."
linked_review: "docs/closure/2026-09-05-135-s-retire-http-sse-transport-adversarial-review.md"
```

```yaml
type: bug
title: "docs-stale-transport-claim: docs/troubleshooting.md instructs operators to check for the deleted legacy-sse feature"
description: "docs/troubleshooting.md's symptom table tells operators to verify whether they built with legacy-sse when old HTTP/SSE instructions fail. That feature no longer exists in Cargo.toml and will hard-error if attempted."
file: "docs/troubleshooting.md"
line: 38
severity: "MAJOR"
confidence: "HIGH"
fix: "Rewrite the row to state HTTP/SSE was removed in 135-S with no build-time re-enable path, and to reconfigure the client as stdio MCP."
linked_review: "docs/closure/2026-09-05-135-s-retire-http-sse-transport-adversarial-review.md"
```

```yaml
type: bug
title: "stale-feature-doc: docs/workflows.md lists legacy-sse as a live non-default feature"
description: "docs/workflows.md's test-gate coverage note lists legacy-sse alongside git-graph and otlp-export as a non-default feature that cargo dev-test/full-test skip. legacy-sse no longer exists as a Cargo feature after 135-S."
file: "docs/workflows.md"
line: 96
severity: "MAJOR"
confidence: "HIGH"
fix: "Remove legacy-sse from the parenthetical feature list, leaving (git-graph, otlp-export)."
linked_review: "docs/closure/2026-09-05-135-s-retire-http-sse-transport-adversarial-review.md"
```

---

## Readiness verdict: **READY_WITH_FOLLOWUPS**

The four 135-S tasks' literal, stated scope (delete the HTTP/SSE modules and
their tests, remove the `legacy-sse` feature and its now-unused dependencies,
retire HTTP endpoint claims from the installer and `docs/architecture.md`,
mark the two ADRs superseded) is **completely and correctly implemented** with
no scope-discipline violations, no dangling module/import references, no new
security regression, and no compile-breaking dead code. The rate-limiter
retirement reasoning in ADR-0003 is sound for all three live transport
surfaces.

However, three consensus (HIGH-confidence, 3/3 reviewer) findings — an
orphaned `Config::port`/`ENGRAM_PORT` CLI/env surface, stale `install --port`
help text, and four separate doc files (README.md, `docs/configuration.md`,
`docs/troubleshooting.md`, `docs/workflows.md`) still describing HTTP/SSE as
an existing, optional, feature-gated compatibility path — mean the shipment's
own stated goal, "retire HTTP and SSE transport surfaces," is **not yet fully
realized from an operator/user-facing perspective**. An operator reading
README.md or `docs/troubleshooting.md` today would be actively misled into
attempting a `--features legacy-sse` build that hard-errors, or setting an
`ENGRAM_PORT` env var that does nothing.

None of these findings block merge on correctness or security grounds (no
compile failure, no vulnerability introduced), so this is not a `BLOCKED`
verdict. But given the doc-accuracy findings are unanimous across all three
independent reviewers and directly contradict the shipment's stated purpose,
recommend a fast, narrowly-scoped follow-up task (covering findings #1–6, all
`safe_auto`/`gated_auto` doc-and-comment edits plus one CLI-flag decision)
before considering 135-S fully closed out.
