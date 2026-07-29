# Stage Cycle 4 — Held Investigative Items (015-D spike, 016-D deliberation, 014-D staleness)

- **Date:** 2026-07-29
- **Agent:** Stage
- **Base:** local main == origin/main == `00665738` (096-S absorbed)
- **Mode:** burn-through; stash empty; process 3 operator-named held items
- **Role posture:** planning/decomposition only. No build, no code, no branch, no PR. `start.ps1` untouched. Artifacts left UNCOMMITTED (Orchestrator commits).

## Disposition summary

| Item | Type | Disposition | Outcome |
|---|---|---|---|
| 015-D (stash 5765BAAB) | spike | **SPIKE COMPLETE → DEFER to runtime-verification spike** | Both symptoms reproduced; root cause narrowed, not pinned; no fix authored. Stash 5765BAAB stays active. |
| 016-D (stash B94772CB) | deliberation | **DECIDED: keep fail-closed → archived/parked** | Stash B94772CB archived; 016-D archived. |
| 014-D (stash FF7DE872) | deliberation | **MOOT / superseded → archived** | Fully shipped as 100-F/092-S; follow-up tracked as 016-D. |

**Shipments produced this cycle: NONE.** (Honest outcome — 015-D deferred pending runtime verification; 016-D/014-D closed without build.)

## 015-D spike — daemon `engram index` non-persist + IPC hang

Hands-on/live-daemon investigation on post-104-F main. Findings artifact (UNCOMMITTED):
`docs/decisions/2026-07-29-daemon-index-ipc-hang-spike-findings.md`.

**Method:** fresh 2-file git workspace (`mod_a.py: def beta`; `mod_b.py: from mod_a import beta; def alpha(): return beta()`), bounded-timeout `Start-Process … WaitForExit(ms)` probing.

**Evidence:**
- **Symptom 2 (IPC hang) REPRODUCED:** daemon-path `engram index --workspace <tmp> --timeout 200` hung the CLI **>270s** (killed), exceeding its own `--timeout`, while the daemon completed the scan server-side in seconds (`scan_status.running=false`). In-process `--direct` returned in **~1.0s** on the same corpus → hang is **daemon/IPC-path-specific**.
- **Symptom 1 (non-persist) CORROBORATED:** `workspace-status` reported `edges:2` (both `defines`); `map-code beta` showed no incoming `calls` edge; `edges.jsonl` had no `calls` row → cross-file `alpha→beta` singleton **absent from the persisted resolved graph** (not merely hidden behind the hang).

**Hypothesis resolution:** H2 (synchronous long-op response + daemon-spawn/model-load OUTSIDE the client timeout via `ensure_daemon_running` before the timed `send_request`) **confirmed as the hang mechanism**. H1/H4 (commit-boundary vs post-pass-not-invoked) **open, not isolated**. H3 partially supported. Original `direct.rs:162` attribution stays **refuted**.

**104-F relationship:** 104-F hardened the pending-sync drain (companion-leak/loop-drain) — a different layer. It does **NOT** fix either symptom here.

**Confounds/caveats (honesty):**
1. Per-workspace-daemon + auto-reindex-on-query → nondeterministic partial states (a `workspace-status` query itself triggered a fresh partial re-index: `code_files:1, edges:1, last_flush:null`).
2. Repro-corpus validity: even `--direct` reported `cross_file_edges_dropped:1` and no resolved `alpha→beta` edge; the minimal `from N import name; name()` shape may not reproduce the exact GREEN-suite singleton — a runtime follow-up must validate the corpus against a known-GREEN case first.
3. Daemon-path CLI killed at ~270s → finalize/post-pass completeness uncertain.

**Recommendation:** DEFER to a **runtime-verification spike** (Ship-owned/instrumented) with a single pre-warmed daemon on one workspace, corpus validated against the in-process GREEN path, and daemon-internal tracing of post-pass invocation/commit + IPC response framing. Candidate fix directions (do NOT build now): async/streaming index response; bound daemon-spawn/model-load under the client deadline; persist-boundary fix if H1/H4 confirmed. **No fix fabricated on an unproven root cause (013-D discipline).**

**Cleanup:** temp per-workspace daemons PIDs **35240** and **12648** killed; both temp workspaces removed. Original repo daemon **PID 31852** (model-loaded, 1.3GB) left bound and untouched.

## 016-D deliberation — Python last-wins recall recovery

**Decision: KEEP FAIL-CLOSED.** Decision doc (UNCOMMITTED): `docs/decisions/2026-07-29-python-last-wins-recall-recovery-decision.md`.

- Option B (blanket last-wins) **unsound**: non-linear redefinition (if/try/decorators/`@overload`) and shared Rust+Python resolver (inline-mod same-name distinct targets) → would mint new wrong Rust edges.
- Option C (Python-gated linear/`@overload`-aware last-wins) sound but complex/precision-risky for a rare pattern; burden disproportionate.
- 013-D (no-false-edge) governs; recall is a documented v1 non-goal. `function_meta.line_start` exists, so deferral costs nothing structurally.
- **Revisit trigger:** measured recall loss on a real Python corpus (esp. `@overload`-heavy typed code).
- Stash B94772CB archived; 016-D archived.

## 014-D staleness — MOOT

014-D's chosen direction (Option A fail-closed, additive `find_unique_function_id`) is exactly what shipped in **100-F (092-S, merged)**. Its only deferred follow-up (Python last-wins) is tracked as 016-D. No independent unresolved value → **archived** (FE8B3B2D precedent).

## Parked / untouched (per operator)

- **017-D** (cozo bump) — PARKED, not actioned this cycle.
- **025-S / 081-S** — blocked, not touched.
- **5765BAAB** — remains active in stash (015-D deferred, not resolved).

## Next Orchestrator action

1. Commit the 3 UNCOMMITTED artifacts to main (spike findings, last-wins decision — plus this memo).
2. For 015-D: schedule a **runtime-verification spike** (Ship-owned or dedicated instrumented investigation) per the repro procedure — do NOT open a fix shipment until root cause is pinned.
3. No queued shipment handed to Ship this cycle.

## Stop-condition / role check

No stop condition hit. No build/PR/code scope taken. Consecutive-failure counter clean. Artifacts UNCOMMITTED as instructed.
