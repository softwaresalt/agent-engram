# Clap 4 bool env var requires BoolishValueParser for "1"/"0" support

## Problem

In clap 4, a `bool` field with `#[arg(long, env = "FOO")]` and no custom
`value_parser` only accepts `"true"` / `"false"` from environment variables.
Values like `"1"`, `"0"`, `"yes"`, `"no"`, `"on"`, `"off"` are rejected with
a parse error (exit 2).

This caused an integration test `env_var_activates_direct_mode` to fail because
`ENGRAM_DIRECT=1` was rejected by the default bool parser.

## Fix

Add `value_parser = clap::builder::BoolishValueParser::new()` to the argument
annotation:

```rust
#[arg(
    long,
    env = "ENGRAM_DIRECT",
    value_parser = clap::builder::BoolishValueParser::new()
)]
direct: bool,
```

`BoolishValueParser` accepts (case-insensitive):
- `"true"` / `"false"`
- `"1"` / `"0"`
- `"yes"` / `"no"`
- `"on"` / `"off"`

## When to Apply

Apply `BoolishValueParser` to any `bool` CLI flag that also reads from an
environment variable where users might set `VAR=1` or `VAR=0` (common shell
convention).

## Evidence

- `src/bin/engram.rs`: `Sync { direct }` and `Index { direct }` both use
  `BoolishValueParser`
- `tests/integration/cli_direct_test.rs`: `env_var_activates_direct_mode`
  test verifies `ENGRAM_DIRECT=1` works

## Date

2026-05-08 | Shipment 030-S (045-F CLI-direct mode)
