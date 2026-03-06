//! Configuration file templates for the engram plugin installer.
//!
//! Provides template strings and generation functions for:
//! - `.vscode/mcp.json` — VS Code MCP client configuration
//! - `.gitignore` entries — exclude runtime artifacts from version control

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
/// Covers runtime artifacts and embedded database files that must not be
/// tracked by version control.
///
/// # Examples
///
/// ```
/// let entries = engram::installer::templates::gitignore_entries();
/// assert!(entries.contains(".engram/"));
/// ```
pub fn gitignore_entries() -> &'static str {
    "\n# engram plugin (workspace-local state)\n.engram/\n"
}
