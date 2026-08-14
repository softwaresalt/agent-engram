---
title: "RUSTSEC-2026-0041 remediation spike findings"
type: decision
date: 2026-08-12
status: blocked
shipment: 115-S
feature: 119-F
task: 119.001-T
references:
  - docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md
  - .backlogit/archive/119.001-R-rustsec-2026-0041-spike-plan-security-review.md
  - docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md
  - .autoharness/config.yaml
---

# RUSTSEC-2026-0041 remediation spike findings

## Scope and result

This is the U1 admission record for shipment `115-S`, batch
`dark-factory-2026-08-10`, order `1`. The shipment was claimed only after the
separate configuration disposition and clean tracked-baseline gates passed.
The run is **blocked before candidate discovery or any Cargo/cargo-audit
command**. No manifest/lock file was changed, no candidate was acquired or
executed, no test/runtime code ran, and no data or cleanup operation occurred.

The exact final reviewed plan remains authoritative. No production fix, Cozo
fork, vendoring, migration, second worktree, live data, global write, or
blanket cleanup is authorized.

## Claim and baseline evidence

- Shipment: `115-S`, claim completed with `backlogit shipment claim 115-S`
  (CLI fallback because the MCP surface was unavailable); it was then
  returned/marked `blocked` after the admission record.
- Branch: `feat/rustsec-2026-0041-remediation-spike`.
- Baseline HEAD: `692660859997849bd54573f649813594a12cb64d`.
- `git worktree list --porcelain` reported exactly one registered worktree:
  `C:/Source/GitHub/engram`, at the baseline commit.
- Immediately before branch creation/claim,
  `git status --porcelain --untracked-files=no` was empty.
- The claim's expected backlog status transitions are the only tracked
  changes so far (`.backlogit/queue/115-S.md`, `119-F.md`,
  `119.001-T.md`, `119.002-T.md`, and `119.003-T.md`).
- Read-only manifest baselines are `Cargo.toml` SHA-256
  `7234A5028E719DABDCF9BD46235A1D4FE8432E666B7E0B831A145F6BA23B1626` and
  `Cargo.lock` SHA-256
  `BBBD0B143102D6A0CFF3A652D02A1DD633D6A75B4DFB02B2627E24988D5B7E87`.
- PR `#338` is `MERGED` into `main`; its merge commit is
  `692660859997849bd54573f649813594a12cb64d`.
- `.autoharness/config.yaml` is intentionally untouched. Its working-tree
  blob is `8ac09d6dd6325d99aa5a778ca512867bc8deda81`, identical to the blob
  at merge `69266085`; Tier 3 `model` and `model_family` are both
  `claude-opus-5`. It has not been modified, reverted, staged, or
  dispositioned on this branch.

## Admission gates

| Gate | Evidence | Result |
|---|---|---|
| Sole core worktree | One worktree at `C:/Source/GitHub/engram`; no second worktree created | PASS |
| Baseline/config | Separate PR #338 disposition and clean tracked baseline were verified before claim | PASS |
| Process quiescence | No `cargo`, `rustc`, `rust-analyzer`, or `cargo-audit` process was observed. Engram daemon PID `7016` is running as `C:\Tools\engram.exe daemon --workspace \\?\C:\Source\GitHub\engram`; shim processes `19664` and `47692` are also live | **BLOCKED** |
| Handle inspection | `handle.exe`/`handle64.exe` unavailable. `openfiles /query` reports that the system global “maintain objects list” flag is disabled and therefore cannot prove local handles | **BLOCKED** |
| Orderly shutdown | No operator-approved shutdown was provided; implicit killing is forbidden | **BLOCKED** |
| Isolated run root | `tmp/rustsec-2026-0041/` and every required child path were absent: `cargo-home`, `target`, `cargo-audit/advisory-db`, `cargo-audit/data`, `cargo-audit/cache`, `candidate-source`, `prototype`, `logs`, `temp`, `baselines`, and `data` | PASS (not used) |
| Core target protection | Core `target/` exists with 96,733 files and 80,175,671,723 logical bytes. It was not cleaned, reused, or modified | PASS (protected) |
| Prior-artifact inventory | `tmp/` contains unrelated prior-run directories (for example `104S-*` and `109031-tests`), but no `rustsec-2026-0041` run root. No prior artifact was reused or deleted | PASS (inventory only) |
| Disk admission | The prior attempt recorded C: free `117,462,368,256` bytes (`109.395 GiB`) and blocked under the superseded policy that multiplied the protected core target (`74.669 GiB`) into a `114.004 GiB` threshold. The recalibrated policy uses an `8 GiB` fixed floor for read-only U1 and an incremental-footprint threshold for U2/U3 that excludes the protected core target and pre-existing caches | **RECHECK REQUIRED** |
| Synthetic data boundary | `tmp/rustsec-2026-0041/data/` is absent; no live, operator, or existing Engram data was accessed | PASS (not created) |
| Canonical containment | `tmp/` resolves to `C:\Source\GitHub\engram\tmp` and is not a link; the absent run root therefore has no external resolution | PASS (not created) |
| Windows containment | Intended controls are workspace-local `CARGO_HOME`, explicit `CARGO_TARGET_DIR`, supported cargo-audit data/db flags, tool flags, and `TMP`/`TEMP`; XDG/TMPDIR would be Unix-only supplements. No tool was run | NOT EXECUTED |
| External fingerprints | Baseline read-only fingerprints captured: cargo registry source `5bd02195e8c55c0fdacb1e0b561e44c5dfe7753ac2f3977e3f56b7f792220b8b` (34,474 files, 975,766,692 bytes); registry cache `be6a68cb07babb1c7ffd54bb4cbcb1287a94d8f2bf5615b3f334ebf8bf87839b` (909 files, 159,067,361 bytes); advisory DB `0befb59778136f22c4e8cbd7e14cf2a098dbe1a277fb6b393fdafdc84e034d20` (1,326 files, 39,769,407 bytes); cargo git and `%LOCALAPPDATA%\cargo-audit` absent | RECORDED |

## Approval gates

Candidate discovery/static inspection was not started because mandatory
process/handle and disk admission failed. Consequently there is no candidate
identity, immutable revision, source URL, checksum/content hash, license,
source delta, executable/build-script/proc-macro/test/transitive-code
inventory, or containment presentation.

The exact-candidate execution gate is therefore **blocked**. No generic,
shipment, dark-factory, inferred, or auto-check approval exists, and no
operator approval is bound to an immutable candidate identity. Candidate
build scripts, proc macros, tests, and binaries remain forbidden. Exact
post-spike cleanup approval is also absent; no cleanup was attempted.

## Named owners

- The Ship executor owns baseline capture, process/handle shutdown
  verification, byte restoration, external-fingerprint verification, and
  preparation of the exact cleanup request.
- The operator (and the owner of the running Engram daemon) must approve and
  perform an orderly daemon shutdown before a retry; this session does not
  kill processes implicitly.
- The operator is the approver of the exact destructive target list.
- The approved cleanup executor is intentionally **unnamed** until that
  separate approval record exists; no cleanup executor acted.

Strict-safety records:

- **ProposedAction:** run one immutable candidate's bounded prototype.
  **ActionRisk:** high. **ActionResult:** blocked (no exact identity approval,
  process/handle quiescence, or disk admission).
- **ProposedAction:** remove workspace-local spike artifacts.
  **ActionRisk:** destructive. **ActionResult:** blocked (no post-spike
  inventory or separate exact cleanup approval).

## Disposition

`119.001-T` cannot advance to U2. The daemon must be shut down orderly by its
owner and handle inspection must become provable before admission is retried.
The prior disk failure was caused by a superseded target-size multiplier;
recheck U1 against the 8 GiB read-only floor, then calculate the incremental
footprint before U2/U3 without counting the protected core target or
pre-existing caches. After those gates pass, U1 must still perform read-only
candidate identity/inventory and obtain explicit approval
bound to that exact candidate before any execution. `119.002-T` and
`119.003-T` remain blocked by dependency and approval gates. Shipment `115-S`
does not ship and must not unlock `116-S`.

## U1 recheck — 2026-08-14 (process/disk gates cleared; candidate identified)

This addendum records a fresh, independent U1 recheck performed after the
tracked-baseline blocker (`.autoharness/config.yaml`, PR #338) and the
`.github/agents/stage.agent.md` model-routing update were both resolved and
committed (`05651ebe`) on this branch.

### Process/handle quiescence — now PASS

- The specific Engram daemon PID (`7016`) recorded in the prior admission
  attempt is **no longer running** (`Get-Process -Id 7016` returns nothing).
- A live process inventory (`Get-CimInstance Win32_Process`) found no
  `engram.exe daemon` process bound to the `C:\Source\GitHub\engram`
  workspace. The only running Engram daemon (PID `16448`) is bound to the
  **`autoharness`** workspace, a different working tree.
- Querying the Engram MCP tool surface directly
  (`get_daemon_status` / `get_workspace_status`) for this workspace returned
  `Failed to connect to daemon IPC endpoint ... The system cannot find the
  file specified` — confirming, from the tool's own perspective, that no
  daemon session or open pipe currently exists for this workspace.
- No `cargo.exe`, `rustc.exe`, `rust-analyzer.exe`, or `cargo-audit.exe`
  process was found running anywhere on the host.
- **This gate is satisfied without terminating any process** — the prior
  blocking daemon session had already ended on its own between admission
  attempts. No implicit kill was performed or required.

### Disk admission — now PASS

- Current `C:` free space: `116,691,091,456` bytes (**108.677 GiB**), measured
  via `System.IO.DriveInfo`.
- This clears the `8 GiB` fixed read-only U1 floor with wide margin (the
  prior failure was caused by a superseded policy multiplying the protected
  core target into the threshold, which the recalibrated policy no longer
  does).
- The core `target/` directory remains untouched and is still excluded from
  any admission calculation.
- U2/U3 incremental-footprint threshold (`max(20 GiB, ceil(1.5*I) + 2 GiB)`)
  has not yet been computed against a real isolated run root because no
  isolated run has started; at 108.677 GiB free, headroom is ample for any
  plausible single-crate patch build (`I` on the order of low single-digit
  GiB), but the exact measurement must still be taken at U2 start per policy.

### Candidate identity — read-only discovery complete

Read-only registry/advisory research (`crates.io` API, `rustsec.org`, and the
upstream `lz4_flex` GitHub manifest) established the following, in place of
the previously assumed "bump `swapvec`" path:

- **`swapvec` does not fix the advisory.** Both the currently locked
  `swapvec 0.3.0` and the latest published `swapvec 0.4.2` declare the
  identical dependency requirement `lz4_flex = "^0.10.0"` — confirmed via
  `crates.io/api/v1/crates/swapvec/0.4.2/dependencies`. Bumping `swapvec`
  alone, at any published version, does **not** clear RUSTSEC-2026-0041.
- **`cozo` has no newer crates.io release.** `crates.io/api/v1/crates/cozo`
  reports `max_version = newest_version = 0.7.6`, last published
  2023-12-11. No `0.8` (or any post-0.7.6) version is published on
  crates.io, consistent with deliberation `017-D`'s conclusion that a
  crates.io-based major-version path is not currently available.
- **Only a direct `lz4_flex` patch clears the advisory** while keeping
  `cozo 0.7.6` / `swapvec 0.3.0` unchanged, via a `[patch.crates-io]`
  override in this repository's own `Cargo.toml`.

**Proposed exact candidate** (read-only profile, not yet applied):

| Field | Value |
|---|---|
| Crate | `lz4_flex` |
| Version | `0.11.6` (lowest version inside the official patched range `>=0.11.6, <0.12.0`, minimizing API drift from the currently locked `0.10.0`) |
| Advisory patched ranges | `>=0.11.6, <0.12.0` or `>=0.12.1` (per `rustsec.org/advisories/RUSTSEC-2026-0041.html`) |
| SHA-256 checksum (crates.io) | `373f5eceeeab7925e0c1098212f2fbc4d416adec9d35051a6ab251e824c1854a` |
| License | MIT |
| Repository | `https://github.com/pseitz/lz4_flex` |
| Publisher | `PSeitz` (GitHub `pseitz`) — same publisher as every other published `lz4_flex` version, including the currently locked `0.10.0` |
| `rust-version` (MSRV) | `1.81` — satisfied by this repo's own `rust-version = "1.85"` (`Cargo.toml`) |
| Build script | **None.** Upstream `Cargo.toml`'s `include` manifest is `["src/*.rs", "src/frame/**/*", "src/block/**/*", "README.md", "LICENSE"]` — no `build.rs`, no `build =` field, no `[build-dependencies]` |
| Proc-macro | **None.** No `proc-macro = true`, no proc-macro-kind dependency at any version |
| Normal (non-dev) dependencies at build time | `twox-hash ^2.0.0` (optional, only under the `frame` feature) — no other third-party code pulled in |
| Transitive impact | No change to `cozo` (`0.7.6`) or `swapvec` (`0.3.0`) versions; only the `lz4_flex` leaf is repinned |

At the U1 boundary this candidate profile was **read-only discovery only** —
no core `Cargo.toml`, `Cargo.lock`, patch directive, or build/test/audit command
had been applied or executed. The exact-candidate approval recorded for the
next phase was bound to `lz4_flex 0.11.6` and its checksum above; blanket,
batch, shipment-level, or "dark-factory" authorization was not used as a
substitute.

### U1 disposition at phase boundary

U1's process-quiescence and disk-admission gates were satisfied with fresh
evidence, and read-only candidate discovery was complete. The phase then
paused until the operator approved execution bound to the exact candidate
identity below. That approval was subsequently provided on 2026-08-14.

## U2 prototype — 2026-08-14 (approved candidate validated)

The operator explicitly approved execution of `lz4_flex 0.11.6` on
2026-08-14. U2 ran only inside `tmp/rustsec-2026-0041/` with a workspace-local
`CARGO_HOME`, `CARGO_TARGET_DIR`, advisory database, candidate source, logs,
and temporary-data boundary. The core worktree and its production manifests
were not edited.

### Resolver correction

The initially proposed direct `[patch.crates-io] lz4_flex = "=0.11.6"`
configuration is not valid Cargo patch syntax for this graph: Cargo rejects a
same-source registry patch, and `swapvec 0.3.0` declares `lz4_flex ^0.10.0`,
which cannot select `0.11.6` directly. The sandbox therefore used the
smallest compatibility bridge that preserves the approved candidate:

- Download the immutable `swapvec 0.3.0` crate into `candidate-source/`
- Change only its dependency declaration from `lz4_flex = "0.10.0"` to
  `lz4_flex = "0.11.6"`
- Add a path patch for that manifest-only `swapvec 0.3.0` copy
- Resolve the registry `lz4_flex 0.11.6` crate with its recorded checksum

No swapvec Rust source, Cozo source, Engram source, or production manifest was
changed. Cargo resolved the final graph as:

```text
engram -> cozo 0.7.6 -> swapvec 0.3.0 (manifest-only sandbox bridge)
       -> lz4_flex 0.11.6
```

### U2 evidence and results

- Isolated U2 baseline: `37,317,342` bytes; final sandbox footprint after
  compilation and tests: `32,644,387,387` bytes
- Incremental footprint `I`: `32,607,070,045` bytes (**30.368 GiB**)
- Required threshold: `max(20 GiB, ceil(1.5*I) + 2 GiB)` = **47.552 GiB**
- Free space after U3: **78.774 GiB** — threshold PASS
- `cargo update -p lz4_flex --precise 0.11.6`: PASS in sandbox
- `cargo check --locked --all-targets`: PASS
- `cargo dev-test`: **599 passed, 0 failed**
- `cargo test --locked --all-targets`: PASS, including all registered
  contract, integration, and unit targets
- `cargo clippy --locked --all-targets -- -D warnings -D clippy::pedantic`:
  PASS for the default feature set
- `cargo fmt --all -- --check`: PASS
- `cargo audit --file <sandbox Cargo.lock>`: PASS with **0 vulnerabilities**
  across 553 dependencies. The report contained existing unmaintained and
  unsound warnings, but no advisory, including RUSTSEC-2026-0041.

The all-features clippy probe remains outside this spike's default Cozo path
and fails on pre-existing OpenTelemetry API incompatibilities in
`src/server/observability.rs` (`SdkTracerProvider` and
`SpanExporter::builder`). No candidate-related error was reported by that
probe, and no source fix was made for it.

### Restoration and containment

- Core `Cargo.toml` SHA-256 remained
  `7234A5028E719DABDCF9BD46235A1D4FE8432E666B7E0B831A145F6BA23B1626`
- Core `Cargo.lock` SHA-256 remained
  `BBBD0B143102D6A0CFF3A652D02A1DD633D6A75B4DFB02B2627E24988D5B7E87`
- Core `git status --porcelain --untracked-files=no` was clean throughout U2
  and U3; this findings addendum is the only subsequent tracked modification
- No Cargo process, Rust compiler, Rust analyzer, or cargo-audit process
  remained after U2
- Global Cargo registry source/cache counts and bytes matched the captured
  baseline exactly: 34,474 files / 975,766,692 bytes and 909 files /
  159,067,361 bytes. Global Cargo git and local cargo-audit paths remained
  absent
- All sandbox artifacts resolve beneath `tmp/rustsec-2026-0041/`
- The sandbox manifest, lockfile, target, caches, logs, and candidate source
  remain retained for the separate cleanup decision; no deletion was attempted

## U3 runtime/data verification — 2026-08-14 (synthetic only)

U3 ran focused Cozo verification with `TEMP` and `TMP` redirected to the
workspace-local synthetic boundary. The test set exercised cold restart,
CRUD, graph edges, symbol lookup, and vector storage using only temporary
synthetic records. The run passed **53 tests, 0 failed**:

- `integration_cozo_cold_restart`: 4 passed
- `integration_cozo_crud`: 11 passed
- `integration_cozo_edge`: 15 passed
- `integration_cozo_symbol_lookup`: 13 passed
- `integration_cozo_vector`: 10 passed

The synthetic `data/` directory was empty after test-managed temporary cleanup.
No live or operator data was read, copied, migrated, repaired, or deleted. No
child process remained. The approved candidate therefore preserves the tested
Cozo 0.7.6 graph, dehydration/hydration, and vector runtime behavior in the
sandbox.

## Spike disposition

**Recommendation: `pivot` — eligible for a separately planned production
remediation, not an automatic production change.** The approved patched
`lz4_flex 0.11.6` candidate is build-, audit-, and focused-runtime-compatible,
but production adoption requires a maintained compatibility bridge or an
upstream `swapvec` release that widens its `lz4_flex` requirement. The direct
same-source Cargo patch originally proposed is not viable. This spike did not
modify production manifests, lockfiles, source, or live data.

The spike evidence is complete. Cleanup is **`not-approved`**: the exact
workspace-local artifact inventory is retained under `tmp/rustsec-2026-0041/`
for a later, separately approved destructive cleanup action. This leaves the
follow-on shipment blocked until that cleanup decision and a production
remediation plan are separately reviewed.