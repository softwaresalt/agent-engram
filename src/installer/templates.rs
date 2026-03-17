//! Configuration file templates for the engram plugin installer.
//!
//! Provides template strings and generation functions for:
//! - `.vscode/mcp.json` — VS Code MCP client configuration
//! - `.gitignore` entries — exclude runtime artifacts from version control
//! - `.github/copilot-instructions.md` — GitHub Copilot agent instructions
//! - `.claude/instructions.md` — Claude Code agent instructions
//! - `.cursor/mcp.json` — Cursor MCP configuration

use std::path::Path;

/// Generate the contents of `.vscode/mcp.json` for the given engram executable.
///
/// The generated configuration registers the engram binary as an MCP stdio
/// server. The shim discovers the workspace from its current working directory
/// at startup, so no workspace argument is required in the configuration.
///
/// Path separators are normalised to forward slashes for cross-platform JSON
/// compatibility.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// let json = engram::installer::templates::mcp_json(Path::new("/usr/local/bin/engram"));
/// assert!(json.contains("mcpServers"));
/// assert!(json.contains("stdio"));
/// ```
pub fn mcp_json(engram_exe: &Path) -> String {
    let exe_str = engram_exe.to_string_lossy();
    // Normalise backslashes so the JSON is valid on Windows too.
    let exe_normalized = exe_str.replace('\\', "/");
    format!(
        r#"{{
  "mcpServers": {{
    "engram": {{
      "type": "stdio",
      "command": "{exe_normalized}",
      "args": []
    }}
  }}
}}"#
    )
}

/// Return the `.gitignore` entries that should be appended for engram.
///
/// Only excludes runtime artifacts (Unix socket, lock files) from version
/// control. State files (`tasks.md`, `graph.surql`, `.version`, `.lastflush`)
/// are Git-friendly and intentionally committed per the constitution.
///
/// # Examples
///
/// ```
/// let entries = engram::installer::templates::gitignore_entries();
/// assert!(entries.contains(".engram/run/"));
/// assert!(!entries.contains(".engram/\n"));
/// ```
pub fn gitignore_entries() -> &'static str {
    "\n# engram plugin (runtime artifacts — state files are intentionally tracked)\n.engram/run/\n"
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

/// Generate the Cursor MCP configuration JSON for `.cursor/mcp.json`.
///
/// The generated JSON registers the Engram HTTP SSE endpoint. Unlike the
/// Markdown hook files, this uses a JSON merge strategy — the `engram` key is
/// upserted into `mcpServers` without removing other entries.
///
/// # Examples
///
/// ```
/// let json = engram::installer::templates::cursor_mcp_json(7437);
/// assert!(json.contains("http://127.0.0.1:7437/mcp"));
/// assert!(json.contains("mcpServers"));
/// ```
pub fn cursor_mcp_json(port: u16) -> String {
    format!(
        r#"{{
  "mcpServers": {{
    "engram": {{
      "url": "http://127.0.0.1:{port}/mcp"
    }}
  }}
}}"#
    )
}
