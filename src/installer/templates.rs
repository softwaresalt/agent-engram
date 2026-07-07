//! Configuration file templates for the engram plugin installer.
//!
//! Provides template strings and generation functions for:
//! - `.mcp.json` — workspace-root MCP client configuration (engram entry)
//! - `.gitignore` entries — exclude runtime artifacts from version control
//! - `.github/copilot-instructions.md` — GitHub Copilot agent instructions
//! - `.claude/instructions.md` — Claude Code agent instructions

/// Workspace-root `.mcp.json` content registering engram as a stdio MCP server.
///
/// Applied with add-if-absent semantics: when a workspace has no `.mcp.json`
/// this is written verbatim; when one exists without an `engram` entry, the
/// `mcpServers.engram` object is merged in, preserving all other servers. The
/// shim resolves the workspace from `ENGRAM_WORKSPACE` (expanded by the MCP
/// client to the workspace folder).
///
/// # Examples
///
/// ```
/// let s = engram::installer::templates::ROOT_MCP_JSON;
/// assert!(s.contains("\"engram\""));
/// assert!(s.contains("stdio"));
/// assert!(s.contains("shim"));
/// ```
pub const ROOT_MCP_JSON: &str = r#"{
  "mcpServers": {
    "engram": {
      "type": "stdio",
      "command": "engram",
      "args": ["shim"],
      "env": {
        "ENGRAM_WORKSPACE": "${workspaceFolder}"
      }
    }
  }
}"#;

/// Return the `.gitignore` entries that should be appended for engram.
///
/// Excludes runtime artifacts (Unix socket, lock files) and the embedded
/// database directory from version control. State files (`tasks.md`,
/// `graph.surql`, `.version`, `.lastflush`) are Git-friendly and
/// intentionally committed per the constitution.
///
/// # Examples
///
/// ```
/// let entries = engram::installer::templates::gitignore_entries();
/// assert!(entries.contains(".engram/run/"));
/// assert!(entries.contains(".engram/db/"));
/// assert!(!entries.contains(".engram/\n"));
/// ```
pub fn gitignore_entries() -> &'static str {
    "\n# engram plugin (runtime artifacts — state files are intentionally tracked)\n.engram/run/\n.engram/db/\n"
}

/// Generate the GitHub Copilot instructions markdown for `.github/copilot-instructions.md`.
///
/// The content includes the MCP endpoint URL (using `port`), the list of
/// available Engram tools, and recommended workflow patterns.
///
/// # Examples
///
/// ```
/// let md = engram::installer::templates::copilot_instructions(7437);
/// assert!(md.contains("http://127.0.0.1:7437/mcp"));
/// assert!(md.contains("query_memory"));
/// ```
pub fn copilot_instructions(port: u16) -> String {
    format!(
        r#"## Engram Agent Memory — GitHub Copilot Integration

Engram is running as an MCP server at `http://127.0.0.1:{port}/mcp`.

### Available Tools

| Tool | Purpose |
|------|---------|
| `set_workspace` | Register this workspace at session start |
| `query_memory` | Retrieve stored context, tasks, and code knowledge |
| `create_task` | Create a new task in the workspace task list |
| `update_task` | Update task status or details |
| `map_code` | Index code files for semantic navigation |
| `unified_search` | Search across all content types |
| `query_changes` | Query git commit history by file, symbol, or date |

### Recommended Workflow

1. **Session start**: Call `set_workspace` with the current workspace path.
2. **Before coding**: Call `query_memory` to load relevant context.
3. **Task tracking**: Use `create_task` and `update_task` to record progress.
4. **Code navigation**: Use `map_code` and `unified_search` for codebase exploration.
5. **Change history**: Use `query_changes` to understand recent modifications."#
    )
}

/// Generate the Claude Code instructions markdown for `.claude/instructions.md`.
///
/// The content includes the MCP endpoint URL (using `port`), the list of
/// available Engram tools, and recommended workflow patterns for Claude.
///
/// # Examples
///
/// ```
/// let md = engram::installer::templates::claude_instructions(7437);
/// assert!(md.contains("http://127.0.0.1:7437/mcp"));
/// assert!(md.contains("set_workspace"));
/// ```
pub fn claude_instructions(port: u16) -> String {
    format!(
        r#"## Engram Agent Memory — Claude Code Integration

Engram is running as an MCP server at `http://127.0.0.1:{port}/mcp`.

### Available Tools

| Tool | Purpose |
|------|---------|
| `set_workspace` | Register this workspace at session start |
| `query_memory` | Retrieve stored context, tasks, and code knowledge |
| `create_task` | Create a new task in the workspace task list |
| `update_task` | Update task status or details |
| `map_code` | Index code files for semantic navigation |
| `unified_search` | Search across all content types |
| `query_changes` | Query git commit history by file, symbol, or date |

### Recommended Workflow

1. **Session start**: Always call `set_workspace` first to bind this workspace.
2. **Context loading**: Call `query_memory` to retrieve relevant prior context.
3. **Task management**: Track all work items with `create_task` and `update_task`.
4. **Code exploration**: Use `map_code` before navigating unfamiliar modules.
5. **Change awareness**: Use `query_changes` to understand what changed recently."#
    )
}
