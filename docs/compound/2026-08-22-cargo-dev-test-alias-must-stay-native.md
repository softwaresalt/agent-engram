---
title: Do not repurpose the canonical cargo dev-test alias into a script-backed subcommand
date: 2026-08-22
type: compound-learning
tags: [cargo, testing, dev-test, ci, tooling, aliases]
shipment: 122-S
feature: 126-F
pr: 355
---

## Problem

Shipment 122-S set out to make the canonical `cargo dev-test` gate prove
comprehensive test coverage (it previously ran only 6 of 214 declared `[[test]]`
targets). The initial implementation redefined the `.cargo/config.toml`
`dev-test` alias to a change-scoped, concurrency-bounded runner by pointing it at
a `cargo-devtest` external subcommand backed by shell/PowerShell scripts.

## What broke

Copilot review flagged three load-bearing regressions:

1. **cargo aliases cannot shell out.** A cargo alias's first token must be a
   cargo subcommand (built-in or an external `cargo-<name>` executable on PATH).
   It cannot invoke `pwsh`/`bash` directly.
2. **External subcommands are not zero-setup and are platform-fragile.** cargo
   resolves `cargo-<name>` by exact executable name plus the host suffix
   (`.exe` on Windows). An extensionless shell shim works on Linux/macOS only if
   `scripts/` is on PATH; a `.cmd`/`.bat` shim is invisible to cargo on Windows,
   so `cargo dev-test` failed outright there.
3. **The alias also dropped `--lib`.** Modelling only declared `[[test]]`
   targets and running `cargo test --test <name>` skipped the colocated
   library unit tests the old gate ran via `--lib`.

`cargo dev-test` is referenced pervasively as a zero-setup contract (constitution
"all tests MUST pass via cargo dev-test", `.autoharness/workspace-profile.yaml`,
and the build-feature / fix-ci / harness-architect skills). Breaking that
contract is a repository-wide regression, not a local inconvenience.

## Resolution

Keep `cargo dev-test` a native, zero-setup, cross-platform cargo alias:

```toml
dev-test = "test --all-targets"
```

This runs every target (including `--lib`) under default features, matching what
CI runs. Deliver the measurable coverage guarantee as a **separate** checked-in
tool rather than by repurposing the alias:

* `scripts/test-coverage-oracle.{sh,ps1} --mode report` — required / selected /
  omitted with pass condition `omitted == 0` (the measurable audit).
* `--mode completeness` — fails if any declared target or `src/` module is
  unmapped, so the surface-to-target manifest cannot drift.
* `--mode run` — an OPTIONAL change-scoped, concurrency-bounded, feature-aware
  fast runner (also runs `--lib`).

## Rules

* Do not convert a pervasively-referenced, zero-setup cargo alias into a
  script-backed external subcommand. Keep the alias native; ship extra tooling
  as separate commands.
* When a runner invokes targets explicitly (`cargo test --test X`), pass each
  target's `required-features`; otherwise cargo errors on feature-gated targets
  (git-graph, legacy-sse) rather than skipping them.
* Shell scripts destined for Linux CI must be committed with LF
  (`.gitattributes: *.sh text eol=lf`); the shell oracle requires Bash 4+
  (associative arrays), so document that macOS needs a modern bash.
* A coverage oracle that reads a diff must fail closed when the diff is
  indeterminate (unresolvable base ref OR missing merge base in a shallow
  checkout); treating an unknown diff as empty reports a false PASS.
