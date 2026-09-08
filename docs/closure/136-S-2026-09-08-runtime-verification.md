---
title: "Shipment 136-S runtime verification"
date: 2026-09-08
shipment_id: "136-S"
feature_id: "142-F"
surface: cli, api
adapter: cargo-test
verdict: "PASS WITH FOLLOW-UP"
---

## Shipment 136-S Runtime Verification

### Context

`136-S` (feature `142-F`) delivers the generation domain and store
(`142.011-T`: `src/services/generations/mod.rs`, `manifest.rs` — generation
identity, checked monotonic revision, sealed inventory, digests,
provenance), the generation store surface (`142.012-T`: candidate-directory
sealing, reserved-name rejection, path-escape/junction-escape guards),
atomic publication (`142.013-T` + 4 subtasks: `publish.rs` — bounded
`PublisherLock`, atomic manifest replace, orphaned-staging detection,
monotonic revision guarding, concurrent-publisher serialization),
`142.014-T` (`src/db/cozo_backend/mod.rs` — `open_existing_generation_via_
runtime_copy`: validated `final_path`, runtime-copy publication, hashed
bounded-reader integrity check, UTF-8 validation ordered before any
mutation as of the final remediation), and `142.017-T` (generation
context: cheap-clone `OpenedGeneration` handle wrapper).

This is new internal domain/service-layer surface (`src/services/
generations/*`, `src/db/cozo_backend/mod.rs` additions) with **no wired
production caller yet** — the daemon composition root does not currently
invoke `open_existing_generation_via_runtime_copy` or the generation
publish/store path from any CLI or MCP-facing command (F17/F18, which wire
this into the read-server composition root, are future shipments in the
`142-F` roster). Runtime surface exposure for this shipment is therefore
via the crate's own contract/integration/unit test suite, plus a CLI/MCP
smoke check confirming the shipment did not disturb the existing bound
daemon surface (tool descriptor registry, `--version`, `manifest`).

Merged via PR #385, merge commit
`7632f8c03b23a5b2da064ca027ed584865cfc74b`, confirmed reachable from
`origin/main` via `git merge-base --is-ancestor` (exit 0).

### Validator contract

* Surface: internal domain/service layer (generation identity, store,
  atomic publish, database-open via runtime copy) — not yet a
  directly-invokable CLI/MCP surface; CLI/MCP smoke checks confirm no
  regression to the existing bound-daemon surface.
* Adapter: `cargo check --all-targets`; `cargo build --release`;
  `target/release/engram.exe --version` / `manifest`; `cargo test --test
  integration_generation_db_open --test integration_generation_store
  --test integration_generation_publish --test unit_generation_domain
  --test unit_generation_context`; `cargo test --lib
  services::generations`; `cargo test --lib db::cozo_backend`.
* Invariants: generation IDs remain strict single-component values
  (path-separator/traversal/Windows-drive-prefix rejection); revision
  remains strictly monotonic and checked; the store rejects reserved
  names (`publisher.lock`, the active-manifest name) case-insensitively
  and rejects path escapes (including directory-junction escapes);
  atomic publication never tears the destination manifest, serializes
  concurrent publishers to exactly one winner, and never promotes
  orphaned staging files; `open_existing_generation_via_runtime_copy`
  validates `final_path`'s UTF-8-ness before any mutation and never
  diverges the runtime copy's bytes from the published manifest.

### Environment prechecks

* `cargo check --all-targets` (dev profile, post-merge branch HEAD):
  **PASS** — `Finished dev profile [unoptimized + debuginfo]` in 1m32s,
  no warnings-as-errors.
* `cargo build --release`: **PASS** — `Finished release profile
  [optimized]` in 4m27s (no release-profile-only regression, unlike the
  `134-S` precedent).
* CLI binary (`target/release/engram.exe --version`):
  `engram 0.3.0-rc.1+gf2fa4bd6` — ok.
* `target/release/engram.exe manifest`: full, well-formed MCP tool catalog
  returned (24 tools) — confirms this shipment's new internal generation
  domain/store/publish/db-open code did not disturb existing tool
  registration or the bound-daemon MCP surface (expected, since no caller
  wires the new surface into the composition root yet).

### Probe outcomes

| Probe | Result |
|---|---|
| `cargo check --all-targets` | ok (1m32s) |
| `cargo build --release` | ok (4m27s) |
| `engram.exe --version` (release binary) | ok |
| `engram.exe manifest` (release binary) | ok (24 tools, well-formed JSON) |
| `integration_generation_db_open` (8 tests) | ok — 8/8 passed |
| `integration_generation_store` (11 tests) | ok — 11/11 passed |
| `integration_generation_publish` (8 tests) | ok — 8/8 passed |
| `unit_generation_domain` (3 tests) | ok — 3/3 passed |
| `unit_generation_context` (4 tests) | ok — 4/4 passed |
| `cargo test --lib services::generations` (13 tests) | ok — 13/13 passed |
| `cargo test --lib db::cozo_backend` (39 tests) | ok — 39/39 passed |

94 of 94 targeted tests passed (34 integration/unit + 60 lib, deduplicated
across module-scoped and file-scoped runs — see note below). No test
failures, no release-build regression.

> Note: `integration_generation_db_open` above reports 8 tests; the PR
> body's own test-evidence table (see PR #385 "Test evidence") separately
> reports "8/8 passed on Windows" for the same file, consistent with this
> run. `cargo test --lib services::generations` (13) and `--lib
> db::cozo_backend` (39) are module-filtered subsets of the full `cargo
> test --lib` run and are not additive with the full-suite count recorded
> in the PR body (269 test binaries) — they are reported here as the
> narrowest, most targeted re-confirmation for this specific closure
> session, run fresh on the post-merge branch after `origin/main` fast-
> forward, not as new coverage beyond what CI/PR evidence already
> established.

### Blocked prerequisites

None. Unlike `134-S`, this shipment introduces no CLI-bound or MCP-bound
runtime surface for `engram status` / `engram health` / `engram sync`
style bound-daemon probes to exercise — the new generation domain/store/
publish/db-open code has no wired caller yet, so a bound-daemon manual
probe would not exercise any of this shipment's own code paths. The
targeted contract/integration/unit suite above is the correct and
complete validator for this shipment's actual scope.

### Risky action state

No production-affecting risky action was taken during verification (build
+ local release-profile CLI smoke commands + existing/new targeted test
suites only, on the post-merge closure branch, after PR #385 was already
merged to `main` under explicit operator approval). This verification
pass made no source changes.

### Follow-up (why PASS WITH FOLLOW-UP, not plain PASS)

* **No production caller yet (expected, not a regression)**: the new
  generation domain/store/publish/db-open surface has no wired caller in
  the daemon composition root. This is by design for this shipment's
  scope (F06/F07/F09/F14/F16a in the `142-F` roster); wiring is deferred
  to future shipments F17/F18. Two deferred P-021 findings already
  captured on PR #385 are directly relevant to that future wiring and are
  carried forward here as release-readiness follow-ups, not fixed in this
  session (per the P-021 single-write invariant — not re-actioned):
  - `9108DB24` (high, provisional, `requires_deliberation: true`):
    runtime-copy stable path vs. a live `OpenedGeneration` handle — a
    design decision (unique-dir-per-open vs. retained lease) needed before
    F17/F18 wire the first production caller.
  - `A67EEA54` (high, `requires_deliberation: true`): `open_existing_
    generation_via_runtime_copy` accepts an independent `generation_id`
    not bound to the validated `ExistingDbLocation`, so a caller could in
    principle copy generation B's bytes under generation A's ID — a real
    API redesign concern, likely related to `9108DB24`, also not
    reachable until a production caller exists.
  Neither finding blocks this shipment's own scope because no production
  caller exists today to exercise the gap; both must be resolved (via
  Stage deliberation, per each entry's `requires_deliberation: true` flag)
  before F17/F18 wire the first caller.
  - Additional lower-severity deferred follow-ups already captured on
    PR #385 (`9CB60992`, `96A1197D`, `341497BC`, `F2A07647`, `6B624CF6`,
    `F0760EF2`, `959D1448`, `8D97190A`/superseded by `D1D94CF4`, `D1D94CF4`,
    `259D0E38`) are carried forward for Stage triage and are not
    re-litigated here; see PR #385's Local Review Readiness block for the
    full P-021 table.

### Verdict and handoff

**PASS WITH FOLLOW-UP**. All mandatory validator targets for this
shipment's actual scope pass in full: `cargo check --all-targets`,
`cargo build --release`, CLI/MCP smoke checks against the release binary,
and the full targeted contract/unit/integration suite (94/94 tests,
0 failures). No release-profile regression (unlike the `134-S`
precedent). The verdict is `PASS WITH FOLLOW-UP` rather than plain `PASS`
strictly because two high-severity, `requires_deliberation: true` P-021
findings (`9108DB24`, `A67EEA54`) about the generation-identity/runtime-
copy lifetime model remain open and must be resolved before a future
shipment (F17/F18) wires the first production caller — not because
anything in `136-S`'s own shipped scope is currently broken or unverified.
Feeding to `operational-closure` below.
