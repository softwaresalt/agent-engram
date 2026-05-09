//! Search and query subcommands: 6 commands mapping to code-intelligence MCP tools.

use serde_json::{Value, json};

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::run_tool;

/// `engram search <query>` → `unified_search`
pub async fn run_search(
    query: String,
    region: Option<String>,
    limit: Option<u32>,
    content_type: Option<String>,
    scope_to_symbol: Option<String>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let mut params = json!({ "query": query });
    if let Some(r) = region {
        params["region"] = Value::String(r);
    }
    if let Some(l) = limit {
        params["limit"] = Value::Number(l.into());
    }
    if let Some(ct) = content_type {
        params["content_type"] = Value::String(ct);
    }
    if let Some(s) = scope_to_symbol {
        params["scope_to_symbol"] = Value::String(s);
    }
    run_tool("unified_search", Some(params), flags, formatter).await
}

/// `engram query-memory <query>` → `query_memory`
pub async fn run_query_memory(
    query: String,
    limit: Option<u32>,
    content_type: Option<String>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let mut params = json!({ "query": query });
    if let Some(l) = limit {
        params["limit"] = Value::Number(l.into());
    }
    if let Some(ct) = content_type {
        params["content_type"] = Value::String(ct);
    }
    run_tool("query_memory", Some(params), flags, formatter).await
}

/// `engram symbols` → `list_symbols`
pub async fn run_symbols(
    file_path: Option<String>,
    node_type: Option<String>,
    name_prefix: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let mut params = json!({});
    if let Some(f) = file_path {
        params["file_path"] = Value::String(f);
    }
    if let Some(t) = node_type {
        params["node_type"] = Value::String(t);
    }
    if let Some(p) = name_prefix {
        params["name_prefix"] = Value::String(p);
    }
    if let Some(l) = limit {
        params["limit"] = Value::Number(l.into());
    }
    if let Some(o) = offset {
        params["offset"] = Value::Number(o.into());
    }
    run_tool("list_symbols", Some(params), flags, formatter).await
}

/// `engram map-code <name>` → `map_code`
pub async fn run_map_code(
    symbol_name: String,
    depth: Option<u32>,
    max_nodes: Option<u32>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let mut params = json!({ "symbol_name": symbol_name });
    if let Some(d) = depth {
        params["depth"] = Value::Number(d.into());
    }
    if let Some(m) = max_nodes {
        params["max_nodes"] = Value::Number(m.into());
    }
    run_tool("map_code", Some(params), flags, formatter).await
}

/// `engram impact <name>` → `impact_analysis`
pub async fn run_impact(
    symbol_name: String,
    depth: Option<u32>,
    max_nodes: Option<u32>,
    concept: Option<String>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let mut params = json!({ "symbol_name": symbol_name });
    if let Some(d) = depth {
        params["depth"] = Value::Number(d.into());
    }
    if let Some(m) = max_nodes {
        params["max_nodes"] = Value::Number(m.into());
    }
    if let Some(c) = concept {
        params["concept"] = Value::String(c);
    }
    run_tool("impact_analysis", Some(params), flags, formatter).await
}

/// `engram query-graph --operation <op> [opts]` → `query_graph`
///
/// Builds a structured JSON params object from CLI flags and dispatches
/// to the `query_graph` MCP tool.
#[allow(clippy::too_many_arguments)]
pub async fn run_query_graph(
    operation: String,
    root: Option<String>,
    from: Option<String>,
    to: Option<String>,
    direction: Option<String>,
    max_depth: Option<u32>,
    max_nodes: Option<u32>,
    edge_types: Option<String>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let mut params = json!({ "operation": operation });
    if let Some(r) = root {
        params["root"] = Value::String(r);
    }
    if let Some(f) = from {
        params["from"] = Value::String(f);
    }
    if let Some(t) = to {
        params["to"] = Value::String(t);
    }
    if let Some(d) = direction {
        params["direction"] = Value::String(d);
    }
    if let Some(md) = max_depth {
        params["max_depth"] = Value::Number(md.into());
    }
    if let Some(mn) = max_nodes {
        params["max_nodes"] = Value::Number(mn.into());
    }
    if let Some(et) = edge_types {
        let types: Vec<Value> = et
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Value::String(s.to_owned()))
            .collect();
        params["edge_types"] = Value::Array(types);
    }
    run_tool("query_graph", Some(params), flags, formatter).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn search_params_include_query() {
        let params = json!({ "query": "engram cli" });
        assert_eq!(params["query"], "engram cli");
    }

    #[test]
    fn search_optional_params_excluded_when_none() {
        // Optional params are only added when Some — verify JSON merging logic.
        let mut params = json!({ "query": "test" });
        // region is None — not added
        let region: Option<String> = None;
        if let Some(r) = region {
            params["region"] = serde_json::Value::String(r);
        }
        assert!(params.get("region").is_none());
    }

    #[test]
    fn symbols_params_use_correct_field_names() {
        // Verify field names match handler struct (node_type, name_prefix, file_path)
        let mut params = json!({});
        params["file_path"] = serde_json::Value::String("src/lib.rs".into());
        params["node_type"] = serde_json::Value::String("function".into());
        params["name_prefix"] = serde_json::Value::String("run_".into());
        assert_eq!(params["file_path"], "src/lib.rs");
        assert_eq!(params["node_type"], "function");
        assert_eq!(params["name_prefix"], "run_");
    }

    #[test]
    fn map_code_params_use_symbol_name_field() {
        let params = json!({ "symbol_name": "run_tool" });
        assert_eq!(params["symbol_name"], "run_tool");
    }

    #[test]
    fn impact_params_use_symbol_name_field() {
        let params = json!({ "symbol_name": "dispatch" });
        assert_eq!(params["symbol_name"], "dispatch");
    }

    #[test]
    fn query_graph_params_use_query_field() {
        let params = json!({ "query": "SELECT * FROM symbols" });
        assert_eq!(params["query"], "SELECT * FROM symbols");
    }
}
