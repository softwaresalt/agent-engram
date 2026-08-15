---
title: "RustSec 2026-0041 production remediation memory"
type: memory
date: 2026-08-14
status: complete
---

# RustSec 2026-0041 production remediation memory

## Outcome

Applied the operator-approved compatibility-fork remediation for the
`engram -> cozo 0.7.6 -> swapvec 0.3.0 -> lz4_flex 0.10.0` advisory chain.
The immutable fork revision is `72b99cef424a739470cefc08f9a37b934a0afcd4`.
The production change was committed and pushed as `ed78d5780e22f86601d0139a07128acfe194c3d4`.

## Files changed

* `Cargo.toml` — pin the reviewed `softwaresalt/swapvec` Git revision
* `Cargo.lock` — resolve `swapvec 0.3.0`, `lz4_flex 0.11.6`, and `twox-hash 2.1.3`
* `docs/decisions/2026-08-10-rustsec-2026-0041-remediation-spike-findings.md` — record production adoption and verification
* `.backlogit/queue/119-F.md` and `.backlogit/queue/115-S.md` — record completion while retaining cleanup as separately gated

## Verification

* `cargo check --locked --all-targets` passed
* `cargo dev-test` passed: 599 tests, 0 failures
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` passed
* `cargo fmt --all -- --check` passed
* `cargo audit` passed with 0 vulnerabilities; existing allowed warnings remain
* Cargo tree confirms `lz4_flex 0.10.0` is absent and the chain resolves to `0.11.6`

## Decisions and constraints

The direct same-source crates.io patch was rejected by Cargo and cannot
satisfy `swapvec 0.3.0`'s `lz4_flex ^0.10.0` dependency requirement across the
`0.x` minor-version boundary. The fork changes only that dependency
declaration and keeps `swapvec 0.3.0` source/API unchanged. The all-features OpenTelemetry Clippy failure remains unrelated and
was not changed.

Sandbox artifacts under `tmp/rustsec-2026-0041/` remain retained. Deletion is a
separate destructive action and still requires exact operator approval.

## Next steps

* Review or merge the pushed branch through the normal PR workflow
* Keep sandbox cleanup separate until exact targets are approved
* Track the unrelated all-features OpenTelemetry incompatibility separately
