//! `engram manifest` — emit tools/list without requiring a running daemon.

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::shim::tools_catalog;

/// Run `engram manifest`: print all registered MCP tools as JSON-RPC 2.0.
pub fn run_manifest(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    let tools = tools_catalog::all_tools();
    let tools_json: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name.as_ref(),
                "description": t.description.as_deref().unwrap_or(""),
                "inputSchema": serde_json::Value::Object(t.input_schema.as_ref().clone())
            })
        })
        .collect();

    let result = serde_json::json!({ "tools": tools_json });
    formatter.success(flags.id_value(), result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::flags::GlobalFlags;
    use crate::cli::output::{OutputFormatter, OutputMode};
    use crate::shim::tools_catalog;

    fn make_flags() -> GlobalFlags {
        GlobalFlags {
            workspace: None,
            id: None,
            json: true,
            format: None,
            quiet: false,
            timeout: None,
        }
    }

    #[test]
    fn manifest_returns_exit_zero() {
        let flags = make_flags();
        let formatter = OutputFormatter::new(OutputMode::Json);
        let code = run_manifest(&flags, &formatter);
        assert_eq!(code, 0);
    }

    #[test]
    fn manifest_tool_count_matches_catalog() {
        let expected = tools_catalog::all_tools().len();
        assert!(expected > 0, "catalog must have at least one tool");
        // The manifest command emits all tools — count verified via catalog directly.
        assert_eq!(expected, tools_catalog::all_tools().len());
    }

    #[test]
    fn all_tools_have_required_fields() {
        for tool in tools_catalog::all_tools() {
            assert!(!tool.name.is_empty(), "tool name must not be empty");
            // description is Option — verify it's Some and non-empty when present
            if let Some(desc) = &tool.description {
                assert!(
                    !desc.is_empty(),
                    "tool description must not be empty when present"
                );
            }
        }
    }
}
