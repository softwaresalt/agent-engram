# Agent Instructions

This file is read automatically by `copilot` CLI and other agent tools that
support `AGENTS.md`. It defines the authoritative rules for working in this
repository. All agents operating here must follow these instructions regardless
of flags such as `--allow-all`, `--yolo`, or `--autopilot`.

Last updated: 2026-03-19 | Constitution version: 2.2.0

---

## Core Principles

### I. Safety-First Rust (NON-NEGOTIABLE)

All production code MUST be written in Rust (stable toolchain, edition 2024,
`rust-version = "1.85"`). `unsafe` code is forbidden at the workspace level
(`#![forbid(unsafe_code)]`). Clippy pedantic lints MUST pass with zero warnings.
`unwrap()` and `expect()` are denied; all fallible operations MUST use the
`Result`/`EngramError` pattern defined in `src/errors/mod.rs`.

### II. MCP Protocol Fidelity

The server MUST implement the Model Context Protocol via the `mcp-sdk` 0.0.3
crate (JSON-RPC 2.0). All MCP tools MUST be unconditionally visible to every
connected agent regardless of configuration. Tools called in inapplicable
contexts (e.g., workspace-scoped tools before `set_workspace`) MUST return a
descriptive error rather than being hidden. Transport is SSE (GET `/sse`) with
JSON-RPC dispatch (POST `/mcp`).

### III. Test-First Development (NON-NEGOTIABLE)

Every feature MUST have tests written before implementation code. The test
directory structure (`tests/contract/`, `tests/integration/`, `tests/unit/`)
MUST be maintained. All tests MUST pass via `cargo test` before any code is
merged. Steps: write test → confirm it fails (red) → implement → confirm it
passes (green). Never write production code before the corresponding test
exists and has been observed to fail.

### IV. Workspace Isolation and Security Boundaries

All file-system operations MUST resolve within the configured workspace root.
Path traversal attempts MUST be rejected. Each workspace MUST map to a unique
SurrealDB database via deterministic SHA-256 hash of the canonical workspace
path. Database queries MUST execute solely within the active workspace's
database context. The daemon MUST bind exclusively to `127.0.0.1`; no external
network exposure is permitted.

### V. Structured Observability

All significant operations MUST emit structured tracing spans to stderr via
`tracing-subscriber`. Span coverage MUST include: MCP tool call execution,
workspace lifecycle events (bind, hydrate, flush), database operations, SSE
connection management, and embedding/search operations.

### VI. Single-Binary Simplicity

The project MUST produce a single binary (`engram`). New dependencies MUST be
justified by a concrete requirement — do not add libraries speculatively.
Prefer the standard library over external crates when adequate. SurrealDB
embedded (surrealkv) is the sole persistence layer; do not introduce additional
databases or caches. Optional capabilities (e.g., embeddings via `fastembed`)
MUST use Cargo feature flags.

### VII. CLI Workspace Containment (NON-NEGOTIABLE)

When an agent operates in CLI mode, it MUST NOT create, modify, or delete any
file or directory outside the current working directory tree. This applies to
all file operations. Paths that resolve above or outside the cwd — whether via
absolute paths, `..` traversal, symlinks, or environment variable expansion —
MUST be refused. The only exception is reading files explicitly provided by
the user as context.

### VIII. Destructive Terminal Command Approval (NON-NEGOTIABLE)

All destructive terminal commands MUST go through agent-intercom operator
approval before execution, regardless of `--allow-all`, `--yolo`, or any
other permissive mode. A terminal command is destructive if it:

- Deletes files or directories (`rm`, `Remove-Item`, `del`, `rmdir`)
- Overwrites files without backup (`mv` to existing target, `Move-Item -Force`)
- Modifies system configuration (`reg`, `Set-ExecutionPolicy`, `chmod`, `chown`)
- Alters version control history (`git reset --hard`, `git push --force`, `git clean -fd`)
- Drops or truncates database content (`DROP TABLE`, `TRUNCATE`, `DELETE FROM` without `WHERE`)
- Installs or removes system-level packages (`npm install -g`, `cargo install`, `apt remove`)
- Executes arbitrary code from untrusted sources (`curl | sh`, `iex (irm ...)`)

Required workflow: `auto_check` → `check_clearance` → execute only after
`status: "approved"`. Permissive flags do NOT bypass this gate.

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work atomically
bd close <id>         # Complete work
bd dolt push          # Push beads data to remote
```

---

## Technical Constraints

| Concern | Constraint |
|---|---|
| Language | Rust stable, edition 2024, `rust-version = "1.85"` |
| Async runtime | Tokio 1 (full features) |
| MCP SDK | `mcp-sdk` 0.0.3 — JSON-RPC 2.0 over SSE |
| HTTP transport | Axum 0.7 — SSE at `/sse`, JSON-RPC at `/mcp` |
| Persistence | SurrealDB 2 embedded (surrealkv), per-workspace namespace via SHA-256 |
| Serialization | serde 1, serde_json 1, `#[serde(rename_all = "snake_case")]` on enums |
| CLI | clap 4 (derive + env), env prefix `ENGRAM_` |
| Tracing | tracing 0.1, tracing-subscriber 0.3 |
| Embeddings | `fastembed` 3 (optional, behind `embeddings` feature flag) |
| Diff/Merge | `similar` 2 for structured diff merge during dehydration |
| Testing | proptest 1, tokio-test 0.4; TDD required |
| Formatting | `cargo fmt --all -- --check` |
| Linting | `cargo clippy` pedantic deny, `unwrap_used` deny, `expect_used` deny |
| Build check | `cargo test && cargo clippy` MUST pass before merge |

---

## Quality Gates

Run in order. Do not skip any gate.

```powershell
# Gate 1 — Compilation
cargo check
# Gate 2 — Lint (zero warnings required)
cargo clippy -- -D warnings
# Gate 3 — Formatting
cargo fmt --all -- --check
# If violations: cargo fmt --all
# Gate 4 — Tests (all must pass)
cargo test
# If output truncated:
cargo test 2>&1 | Out-File logs\test-results.txt
```

---

## Code Style and Conventions

### Error Handling

- All fallible operations return `Result<T, EngramError>`
- `EngramError` wraps domain-specific sub-errors via `#[from]`; each variant maps to a u16 error code
- Error codes: 1xxx (Workspace), 2xxx (Hydration), 3xxx (Task), 4xxx (Query), 5xxx (System)
- Map external errors via `From` impls or `.map_err()` — never `unwrap()` or `expect()`

### Naming

- Module files: `src/{module}/mod.rs` pattern for directories
- Struct IDs: prefixed strings (`task:uuid`, `context:uuid`, `spec:uuid`)
- Status values: `snake_case` (`todo`, `in_progress`, `done`, `blocked`)
- Default visibility: `pub(crate)` unless the item needs to be public API

### Documentation

- All public items require `///` doc comments
- Module-level `//!` doc comments on every `mod.rs` or standalone module file

### Database (SurrealDB)

- All DB access goes through `Queries` struct methods — no raw `db.query()` in tool handlers
- IDs use `Thing` type with table prefixes (`task:uuid`, `context:uuid`)
- Use `*Row` structs for DB read/write; convert `Thing` to `String` for public models
- Namespace: `engram`, database: SHA-256 hash of canonical workspace path

### MCP Tools

- Follow the pattern: Validate Workspace → Parse Params → Connect DB → Execute Logic → Return `Result<Value, EngramError>`
- Tools are stateless functions in `tools/`
- All tools always registered and visible; inapplicable calls return descriptive errors

### Testing

- TDD required: write tests first, verify they fail, then implement
- Three test tiers in `tests/` directory (not inline):
  - `unit/` — isolated logic tests
  - `contract/` — MCP tool response contract verification
  - `integration/` — end-to-end flows with real SSE/DB
- Test DB: always use in-memory SurrealDB
- Use `serial_test` crate for tests requiring sequential execution
---

## Development Workflow

1. **Harness before code**: Every feature MUST have a compiling but
   failing BDD test harness before implementation begins. The
   Harness Architect generates test files and structural stubs.
2. **Beads-driven planning**: All task tracking MUST use Beads
   (`bd ready`, `bd create`, `bd update`, `bd close`). Static
   markdown task lists are not permitted.
3. **Branch per feature**: Each feature MUST be developed on a
   dedicated branch.
4. **Contract-first design**: MCP tool schemas defined before implementation.
   Changes to contracts require updating corresponding contract tests.
5. **Commit discipline**: Each commit MUST be coherent and buildable.
   Commit messages follow conventional commits format
   (`feat:`, `fix:`, `docs:`, `test:`).
6. **No dead code**: Placeholder modules MUST be replaced or removed before
   a feature is considered complete.

---

## Remote Approval Workflow for Destructive File Operations

File creation and modification proceed directly — no approval needed.
The approval workflow applies to **destructive operations only** (deletion,
directory removal, permanent content removal).

### Required Call Sequence

```text
1. auto_check      → Is this auto-approved by workspace policy?
2. check_clearance → Submit proposal; blocks until operator responds via Slack
3. check_diff      → Execute only after status: "approved"
```

### Rules

1. File creation and modification: write directly, then broadcast the change.
2. After every non-destructive file write, call `broadcast` at `info` level with
   `[FILE] {created|modified}: {file_path}` and include the diff or full content.
3. Destructive operations: always route through `auto_check` → `check_clearance` → `check_diff`.
4. One destructive operation per approval — never batch deletions.
5. Set `risk_level: "high"` or `"critical"` for config files, security modules,
   or DB schema files.
6. Do not retry rejected proposals with the same content.
7. Always branch on `approved`, `rejected`, and `timeout` — never assume approval.

---

## Terminal Command Execution Policy

**Do NOT chain terminal commands.** Run each command as a separate, standalone
invocation and inspect output before proceeding.

### Rules

1. **One command per call.** Never combine with `;`, `&&`, `||`, or `|` except
   for permitted output-redirection exceptions below.
2. **No `cmd /c` wrappers** unless strictly necessary; even then, single command only.
3. **No exit-code echo suffixes.** Don't append `; echo "EXIT: $LASTEXITCODE"`.
4. **Check results between commands.** Inspect output and exit code before continuing.
5. **Always use `pwsh`, never `powershell`.** Use the PowerShell 7+ executable.
6. **Use relative paths for output redirection.** Never absolute paths — they break
   auto-approve regex matching.
7. **Temporary output files go in `logs/`.** Never write to `target/` or the root.

### Permitted Exceptions (output redirection only)

```powershell
cargo test 2>&1 | Out-File logs\test-results.txt
cargo test > logs\test-results.txt 2>&1
some-command | Out-String
```

### Correct

```powershell
cargo check
cargo clippy -- -D warnings
cargo test 2>&1 | Out-File logs\test-results.txt
```

### Incorrect

```powershell
cargo check; cargo clippy; cargo test        # chained — forbidden
cargo fmt && cargo clippy && cargo test      # AND-chained — forbidden
cargo test 2>&1 | Out-File target\out.txt   # wrong output dir — forbidden
```

---

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on
confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i`
(interactive) mode on some systems, causing the agent to hang indefinitely
waiting for y/n input.

**Use these forms instead:**

```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**

- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

<!-- BEGIN BEADS INTEGRATION profile:full hash:d4f96305 -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Dolt-powered version control with native sync
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update <id> --claim --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task atomically**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Auto-Sync

bd automatically syncs via Dolt:

- Each write auto-commits to Dolt history
- Use `bd dolt push`/`bd dolt pull` for remote sync
- No manual export/import needed!

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->
