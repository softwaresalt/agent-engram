# Quickstart: T-Mem Development

**Purpose**: Get developers up and running with T-Mem development
**Prerequisites**: Rust 1.82+, Git

## Environment Setup

### 1. Install Rust Toolchain

```bash
# Install rustup (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ensure 2024 edition support
rustup update stable
rustup default stable

# Verify version (1.82+ required)
rustc --version
```

### 2. Install Development Tools

```bash
# Code formatting
rustup component add rustfmt

# Linting
rustup component add clippy

# Security audit
cargo install cargo-audit

# Coverage (optional)
cargo install cargo-tarpaulin
```

### 3. Clone and Build

```bash
# Clone repository
git clone https://github.com/softwaresalt/t-mem.git
cd t-mem

# Build all targets
cargo build

# Run tests
cargo test

# Run lints
cargo clippy -- -D warnings

# Check formatting
cargo fmt --check
```

---

## Project Structure

```
t-mem/
├── Cargo.toml           # Workspace manifest
├── src/
│   ├── lib.rs           # Library root
│   ├── bin/t-mem.rs     # Daemon binary entry
│   ├── server/          # HTTP/SSE layer
│   ├── db/              # SurrealDB layer
│   ├── models/          # Domain entities
│   ├── services/        # Business logic
│   ├── tools/           # MCP tool implementations
│   ├── errors/          # Error types
│   └── config/          # Configuration
├── tests/               # Integration tests
└── specs/               # Feature specifications
```

---

## Running the Daemon

### Development Mode

```bash
# Start with debug logging
RUST_LOG=t_mem=debug cargo run

# Start on specific port
PORT=7437 cargo run
```

### Testing with curl

```bash
# Connect to SSE endpoint
curl -N http://127.0.0.1:7437/sse

# Send MCP tool call (in another terminal)
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_daemon_status",
      "arguments": {}
    },
    "id": 1
  }'
```

---

## Development Workflow

### 1. Create Feature Branch

```bash
git checkout -b 001-core-mcp-daemon
```

### 2. Write Tests First (TDD)

```rust
// tests/contract/lifecycle_test.rs
#[tokio::test]
async fn test_set_workspace_valid_path() {
    let daemon = TestDaemon::spawn().await;
    let response = daemon.call("set_workspace", json!({
        "path": "/tmp/test-repo"
    })).await;
    
    assert!(response.is_ok());
    assert!(response["hydrated"].as_bool().unwrap());
}

#[tokio::test]
async fn test_set_workspace_invalid_path() {
    let daemon = TestDaemon::spawn().await;
    let response = daemon.call("set_workspace", json!({
        "path": "/nonexistent"
    })).await;
    
    assert!(response.is_err());
    assert_eq!(response.error_code(), 1001);
}
```

### 3. Run Tests (Expect Failure)

```bash
cargo test test_set_workspace
# Should fail - not yet implemented
```

### 4. Implement Feature

```rust
// src/tools/lifecycle.rs
pub async fn set_workspace(
    state: &AppState,
    path: String,
) -> Result<WorkspaceResult, TMemError> {
    let canonical = std::fs::canonicalize(&path)
        .map_err(|_| WorkspaceError::NotFound { path: path.clone() })?;
    
    if !canonical.join(".git").is_dir() {
        return Err(WorkspaceError::NotGitRoot { 
            path: canonical.display().to_string() 
        }.into());
    }
    
    // ... hydration logic
}
```

### 5. Run Tests (Expect Pass)

```bash
cargo test test_set_workspace
# Should pass now
```

### 6. Lint and Format

```bash
cargo fmt
cargo clippy -- -D warnings
```

### 7. Commit

```bash
git add -A
git commit -m "feat(lifecycle): implement set_workspace tool"
```

---

## Testing Guide

### Unit Tests

Co-located with source files:

```rust
// src/services/hydration.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_task_markdown() {
        let md = "## task:abc123\n---\nid: task:abc123\n---\n";
        let task = parse_task_block(md).unwrap();
        assert_eq!(task.id, "task:abc123");
    }
}
```

### Integration Tests

Full daemon tests in `tests/`:

```rust
// tests/integration/round_trip_test.rs
#[tokio::test]
async fn test_hydration_dehydration_round_trip() {
    let repo = TempGitRepo::new();
    repo.write_tmem_tasks(sample_tasks());
    
    let daemon = TestDaemon::spawn().await;
    daemon.set_workspace(repo.path()).await.unwrap();
    
    // Modify state
    daemon.update_task("task:1", "in_progress", "Starting").await.unwrap();
    
    // Flush
    daemon.flush_state().await.unwrap();
    
    // Verify file content
    let content = repo.read_tmem_tasks();
    assert!(content.contains("status: in_progress"));
}
```

### Property Tests

Serialization round-trips:

```rust
// tests/unit/proptest_models.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn task_roundtrip(task in arb_task()) {
        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(task, parsed);
    }
}
```

---

## Common Tasks

### Add a New MCP Tool

1. Define schema in `contracts/mcp-tools.json`
2. Add error codes in `contracts/error-codes.md`
3. Write contract tests in `tests/contract/`
4. Implement in `src/tools/`
5. Register in MCP router

### Add a New Entity

1. Define in `data-model.md`
2. Add Rust struct in `src/models/`
3. Add SurrealDB schema in `src/db/schema.rs`
4. Add serialization tests

### Debug Database Issues

```bash
# Connect to embedded SurrealDB
# (requires surreal CLI)
surreal sql --conn file://~/.local/share/t-mem/db/{workspace_hash}

# Query tasks
SELECT * FROM task;

# Check relationships
SELECT * FROM depends_on;
```

---

## Troubleshooting

### Build Failures

```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update
```

### Test Failures

```bash
# Run with verbose output
cargo test -- --nocapture

# Run single test
cargo test test_name -- --exact
```

### Performance Issues

```bash
# Build with release optimizations
cargo build --release

# Profile with flamegraph
cargo install flamegraph
cargo flamegraph --bin t-mem
```

---

## Resources

- [Feature Spec](spec.md) — User stories and requirements
- [Implementation Plan](plan.md) — Technical approach
- [Research](research.md) — Technology decisions
- [Data Model](data-model.md) — Entity definitions
- [MCP Tools](contracts/mcp-tools.json) — API contracts
- [Error Codes](contracts/error-codes.md) — Error taxonomy
- [Constitution](../../.specify/memory/constitution.md) — Development principles
