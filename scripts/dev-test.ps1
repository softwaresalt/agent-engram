#!/usr/bin/env pwsh
# Windows entry point for the change-scoped, concurrency-bounded dev-test gate.
#
# On Windows, cargo does not resolve `.cmd`/`.bat` external subcommands, so
# `cargo dev-test` (which relies on a `cargo-devtest` external subcommand) works
# on Linux/macOS but not Windows. Windows contributors run this wrapper instead:
#
#   pwsh scripts/dev-test.ps1                       # current diff
#   pwsh scripts/dev-test.ps1 --changed src/db/x.rs # explicit diff
#
# `cargo ci` and `cargo full-test` remain PATH-free, cross-platform backstops.
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $here 'test-coverage-oracle.ps1') --mode run @args
exit $LASTEXITCODE
