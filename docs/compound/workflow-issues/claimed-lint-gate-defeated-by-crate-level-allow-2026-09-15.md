---
title: "A plan's claimed lint gate is unearned until checked against crate-level allow attributes"
date: 2026-09-15
category: workflow-issues
confidence: high
evidence:
  - docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md (finding A4, two-persona)
  - src/lib.rs:12-13 (`#![allow(clippy::missing_errors_doc)]`, `#![allow(clippy::missing_panics_doc)]`)
  - .cargo/config.toml (`lint = clippy --all-targets --all-features -- -D warnings -D clippy::pedantic`)
---

## Problem

A plan asserted that rustdoc `# Errors` / `# Panics` sections were enforced by a
per-unit `cargo lint` gate, citing the workspace alias
`clippy --all-targets --all-features -- -D warnings -D clippy::pedantic`. The
alias is real and `missing_errors_doc` / `missing_panics_doc` *are* pedantic
lints, so the claim reads as airtight.

It was false. `src/lib.rs` carries crate-level
`#![allow(clippy::missing_errors_doc)]` and `#![allow(clippy::missing_panics_doc)]`.
Crate-level `allow` beats the command-line `-D clippy::pedantic` **group** flag,
so the gate could never fire. The plan had been written to close a prior review's
finding about exactly those lints, and the closure mechanism did not exist.

The same plan used that gate as the named mechanism for a hardening item, which
meant a hardening section — the part written to be conservative — was itself the
place an unearned guarantee entered.

## Solution

Before citing a lint as a gate, verify the lint can actually fire at the target
site:

* Grep the crate root and the target module for `allow(` of that lint, and for
  `allow(clippy::pedantic)` / `allow(clippy::all)` group suppressions.
* Remember the precedence: a narrower `#![allow(specific_lint)]` in source wins
  over a broader `-D group` on the command line. Enabling a group does **not**
  re-enable members the source has allowed.
* If the lint is suppressed and you need it, the fix is a **scoped re-denial**
  at the new module — `#![deny(clippy::missing_panics_doc, clippy::missing_errors_doc)]`
  on the new `mod.rs` — not a crate-wide change, and not a louder claim.
* If you will not add the denial, stop citing the linter. Downgrade the item to
  a reviewed acceptance criterion and say so.

## When This Applies

Any plan, hardening section, or review response that names a linter, formatter,
type checker, or CI job as the mechanism that closes a finding. The failure mode
generalizes past Rust: `# type: ignore` / `disable=` comments, ESLint
`/* eslint-disable */`, `.editorconfig` overrides, and per-file suppressions all
defeat an aggregate command-line flag the same way.

It applies with particular force when the cited gate is the **closure mechanism
for a previous review finding**, because nobody re-checks a closure.

## Detection

Reviewers should treat "the linter enforces this" as a claim requiring evidence,
not a premise. The cheap check is one grep for `allow(` naming the lint. Where a
plan says a gate closes a finding, confirm the gate is armed at the exact path
the finding lives on — an armed gate elsewhere in the repo proves nothing.

The broader tell: a claim whose truth depends on configuration the claim does not
cite. Ask what file makes it true, then open that file.
