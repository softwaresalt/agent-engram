@echo off
rem cargo external subcommand backing the `cargo dev-test` alias (Windows).
rem Delegates to the change-scoped, concurrency-bounded coverage-oracle runner.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0test-coverage-oracle.ps1" --mode run %*
