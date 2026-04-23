---
date: 2026-04-21
shipment: 005-S
feature: 027-F
branch: feature/027-F-language-pack-compiled
verdict: PASS
surface: internal-library (parsing service)
---

# Runtime Verification — 005-S Compiled Language Parsers

## Surface

`src/services/parsing/` — tree-sitter parsers for Swift, C, C++ added to the
code-graph indexing service. This is an internal library surface invoked when
the daemon indexes source files.

## Environment Prechecks

| Check | Result |
|---|---|
| Branch | `feature/027-F-language-pack-compiled` @ `39429ce` |
| Build target | `cargo build --no-default-features --features surreal-backend` |
| Binary | `target/debug/engram.exe` present ✅ |
| Daemon (IPC) | Not started — internal library verification via unit tests |
| Grammar crates | tree-sitter-swift 0.7.1, tree-sitter-c 0.23.4, tree-sitter-cpp 0.23.4 linked ✅ |

Full daemon IPC runtime verification (file-watcher triggering graph hydration for
.swift/.c/.cpp files) requires a running daemon instance and is out of scope for
this pre-merge check. Unit test coverage is the primary verification path for parser
correctness.

## Verification Mode

**Mode**: `auto` → resolved to `manual` (internal library; daemon IPC requires a running
instance).

## Targets and Scenarios

| Scenario | Command | Expected |
|---|---|---|
| Swift grammar ABI | `a1_spike_swift_grammar_loads` | Grammar loads without ABI error |
| Swift symbol extraction | `test_swift_parsing` | ≥1 Function, ≥1 Class, ≥1 Interface, ≥1 Imports edge |
| C symbol extraction | `test_c_parsing` | ≥1 Function, ≥1 Class (struct), ≥1 Imports edge |
| C++ symbol extraction | `test_cpp_parsing` | ≥1 Function, ≥1 Class, ≥1 Imports edge |
| Kotlin no-op stub | `b1_kotlin_stub_returns_ok` | Ok(empty result), no symbols |
| Binary links | `cargo build --no-default-features --features surreal-backend` | Finished with no errors |

## Evidence

```
test b1_kotlin_stub_returns_ok ... ok
test a1_spike_swift_grammar_loads ... ok
test test_c_parsing ... ok
test test_swift_parsing ... ok
test test_cpp_parsing ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out
```

Binary: `target/debug/engram.exe` present — all grammar crates (including
`tree-sitter-swift 0.7.1` at ABI 15) link correctly with the `tree-sitter 0.25`
runtime.

## Invariants Verified

- ✅ Swift grammar loads under tree-sitter 0.25 runtime (ABI 15 accepted)
- ✅ C struct extraction handles both `declaration`-wrapped and bare `struct_specifier` forms
- ✅ C++ class/struct extraction handles both `declaration`-wrapped and bare `class_specifier` forms
- ✅ Kotlin stub returns `Ok(empty)` — files silently skipped, no indexing errors
- ✅ No linker errors or ABI panics

## Known Gaps (Not Blocking)

- **Daemon IPC path not verified**: The end-to-end flow (daemon watches `.swift`/`.c`/`.cpp`
  file → triggers code-graph hydration → persists symbols to SurrealDB) requires a running
  daemon with a bound workspace. This is captured as a follow-up monitoring item.
- **Inline C++ member functions**: Not extracted at this level (module doc updated to
  clarify). Tracked as follow-up scope.

## Verdict

**PASS WITH FOLLOW-UP**

Parser logic verified via unit tests and binary link check. Daemon IPC file-indexing
path and inline C++ member extraction are known gaps, tracked as follow-up items.

## Handoff to Operational Closure

- **Verdict**: PASS WITH FOLLOW-UP
- **Surfaces verified**: internal parsing library (unit tests + binary link)
- **Evidence**: 5/5 parser smoke tests pass; binary builds cleanly
- **Follow-up 1**: Daemon IPC end-to-end verification for .swift/.c/.cpp files
- **Follow-up 2**: Activate Kotlin parser when tree-sitter-kotlin publishes a 0.25-compatible release
- **Follow-up 3**: C++ inline member function extraction (out-of-scope for this PR)
