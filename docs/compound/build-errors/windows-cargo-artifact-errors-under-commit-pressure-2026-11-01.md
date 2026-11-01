---
title: "Windows Cargo artifact errors under memory commit pressure"
description: "Compiler artifact errors can mask paging-file exhaustion during all-target linking."
source: "docs/compound/build-errors/windows-cargo-artifact-errors-under-commit-pressure-2026-11-01.md"
doc_type: learning
problem_type: build
category: build-errors
component: cargo-validation
root_cause: "Windows memory commit exhaustion during parallel compilation"
resolution_type: workaround
severity: medium
message: "The paging file is too small for this operation to complete (os error 1455)"
file_path: ".cargo/config.toml"
citations:
  - "docs/memory/2026-11-01/173-s-build-validation.md"
tags: [windows, cargo, memory, linking]
---

## Problem

Shipment 173-S passed targeted tests and compilation checks, but
`cargo build --all-targets` failed with missing rlibs, invalid metadata
and a proc-macro import library reported as the wrong crate type.
An unchanged retry also reported failed memory allocations, exit
0xc000012d, and failure to mmap an existing rlib with Windows error 1455.

## Root cause

The explicit allocation and mmap diagnostics identify memory commit
pressure during parallel compilation. Artifact existence on disk does
not establish that every compiler process can map it successfully.
The earlier missing-artifact symptoms alone were insufficient evidence
to justify changing source, dependencies or the toolchain.

## Resolution

Set process-local `CARGO_BUILD_JOBS=2` and keep
`CARGO_TARGET_DIR=target/validation`. The unchanged all-target build
then passed in 25 minutes 58 seconds. No artifact was deleted and no
system or tracked configuration was modified.

## Prevention

Check memory-allocation and OS diagnostics before treating missing
metadata as a source defect. Bound compiler jobs when evidence supports
resource exhaustion. Compiler concurrency is separate from test
parallelism: this does not authorize serializing the mandated full suite,
weakening its gates or extending a slot budget.
