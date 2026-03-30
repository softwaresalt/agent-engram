use chrono::Utc;
use proptest::prelude::*;

use engram::models::class::Class;
use engram::models::code_edge::{CodeEdge, CodeEdgeType};
use engram::models::code_file::CodeFile;
use engram::models::config::{BatchConfig, CodeGraphConfig, WorkspaceConfig};
use engram::models::function::Function;
use engram::models::interface::Interface;
use engram::models::metrics::{
    MetricsConfig, MetricsSummary, SymbolCount, TimeRange, ToolMetrics, UsageEvent,
};

fn arb_workspace_config() -> impl Strategy<Value = WorkspaceConfig> {
    (1..500u32,).prop_map(|(batch,)| WorkspaceConfig {
        batch: BatchConfig { max_size: batch },
        code_graph: CodeGraphConfig::default(),
        metrics: MetricsConfig::default(),
        policy: engram::models::policy::PolicyConfig::default(),
        evaluation: engram::models::evaluation::EvaluationConfig::default(),
        query_timeout_ms: 5_000,
        query_row_limit: 1_000,
    })
}

fn arb_usage_event() -> impl Strategy<Value = UsageEvent> {
    (
        "[a-z_]{3,20}",
        "2026-03-27T12:[0-5][0-9]:00Z",
        1..10_000u64,
        0..100u32,
        0..100u32,
        "[a-z0-9_]{3,20}",
        prop::option::of("[a-z0-9-]{8,36}"),
    )
        .prop_map(
            |(
                tool_name,
                timestamp,
                response_bytes,
                symbols_returned,
                results_returned,
                branch,
                connection_id,
            )| UsageEvent {
                tool_name,
                timestamp,
                response_bytes,
                estimated_tokens: response_bytes / 4,
                symbols_returned,
                results_returned,
                branch,
                connection_id,
                agent_role: None,
                outcome: "success".to_string(),
            },
        )
}

fn arb_metrics_summary() -> impl Strategy<Value = MetricsSummary> {
    prop::collection::btree_map("[a-z_]{3,20}", (1..20u64, 1..5_000u64), 0..5).prop_map(
        |raw_metrics| {
            let mut by_tool = std::collections::BTreeMap::new();
            let mut total_tool_calls = 0_u64;
            let mut total_tokens = 0_u64;
            let mut top_symbols = Vec::new();

            for (name, (call_count, tool_tokens)) in raw_metrics {
                #[allow(clippy::cast_precision_loss)]
                let raw_avg = if call_count == 0 {
                    0.0
                } else {
                    tool_tokens as f64 / call_count as f64
                };
                let avg_tokens = (raw_avg * 100.0).round() / 100.0;
                total_tool_calls += call_count;
                total_tokens += tool_tokens;
                top_symbols.push(SymbolCount {
                    name: name.clone(),
                    count: u32::try_from(call_count).unwrap_or(u32::MAX),
                });
                by_tool.insert(
                    name,
                    ToolMetrics {
                        call_count,
                        total_tokens: tool_tokens,
                        avg_tokens,
                    },
                );
            }

            MetricsSummary {
                total_tool_calls,
                total_tokens,
                by_tool,
                top_symbols,
                time_range: TimeRange {
                    start: "2026-03-27T12:00:00Z".to_owned(),
                    end: "2026-03-27T12:59:00Z".to_owned(),
                },
                session_count: 0,
            }
        },
    )
}

fn arb_code_file() -> impl Strategy<Value = CodeFile> {
    (
        "code_file:[a-f0-9]{8}",
        "src/[a-z]{3,10}\\.rs",
        Just("rust".to_owned()),
        100..100_000u64,
        "[a-f0-9]{64}",
    )
        .prop_map(|(id, path, language, size_bytes, content_hash)| CodeFile {
            id,
            path,
            language,
            size_bytes,
            content_hash,
            last_indexed_at: Utc::now().to_rfc3339(),
        })
}

fn arb_embedding() -> impl Strategy<Value = Vec<f32>> {
    prop::collection::vec(-1.0f32..1.0f32, 384..=384)
}

fn arb_function() -> impl Strategy<Value = Function> {
    (
        (
            "function:[a-z0-9]{8}",
            "[a-z_]{3,20}",
            "src/[a-z]{3,10}\\.rs",
            1..500u32,
            1..500u32,
            "fn [a-z_]{3,20}\\(\\)",
            prop::option::of(".{5,60}"),
        ),
        (
            ".{10,200}",
            "[a-f0-9]{64}",
            1..500u32,
            prop::sample::select(vec!["explicit_code", "summary_pointer"]),
            arb_embedding(),
            ".{10,100}",
        ),
    )
        .prop_map(
            |(
                (id, name, file_path, line_start, line_end_offset, signature, docstring),
                (body, body_hash, token_count, embed_type, embedding, summary),
            )| {
                Function {
                    id,
                    name,
                    file_path,
                    line_start,
                    line_end: line_start + line_end_offset,
                    signature,
                    docstring,
                    body,
                    body_hash,
                    token_count,
                    embed_type: embed_type.to_owned(),
                    embedding,
                    summary,
                }
            },
        )
}

fn arb_class() -> impl Strategy<Value = Class> {
    (
        "class:[a-z0-9]{8}",
        "[A-Z][a-z]{2,15}",
        "src/[a-z]{3,10}\\.rs",
        1..500u32,
        1..500u32,
        prop::option::of(".{5,60}"),
        ".{10,200}",
        "[a-f0-9]{64}",
        1..500u32,
        prop::sample::select(vec!["explicit_code", "summary_pointer"]),
        arb_embedding(),
        ".{10,100}",
    )
        .prop_map(
            |(
                id,
                name,
                file_path,
                line_start,
                line_end_offset,
                docstring,
                body,
                body_hash,
                token_count,
                embed_type,
                embedding,
                summary,
            )| {
                Class {
                    id,
                    name,
                    file_path,
                    line_start,
                    line_end: line_start + line_end_offset,
                    docstring,
                    body,
                    body_hash,
                    token_count,
                    embed_type: embed_type.to_owned(),
                    embedding,
                    summary,
                }
            },
        )
}

fn arb_interface() -> impl Strategy<Value = Interface> {
    (
        "interface:[a-z0-9]{8}",
        "[A-Z][a-z]{2,15}",
        "src/[a-z]{3,10}\\.rs",
        1..500u32,
        1..500u32,
        prop::option::of(".{5,60}"),
        ".{10,200}",
        "[a-f0-9]{64}",
        1..500u32,
        prop::sample::select(vec!["explicit_code", "summary_pointer"]),
        arb_embedding(),
        ".{10,100}",
    )
        .prop_map(
            |(
                id,
                name,
                file_path,
                line_start,
                line_end_offset,
                docstring,
                body,
                body_hash,
                token_count,
                embed_type,
                embedding,
                summary,
            )| {
                Interface {
                    id,
                    name,
                    file_path,
                    line_start,
                    line_end: line_start + line_end_offset,
                    docstring,
                    body,
                    body_hash,
                    token_count,
                    embed_type: embed_type.to_owned(),
                    embedding,
                    summary,
                }
            },
        )
}

fn arb_code_edge_type() -> impl Strategy<Value = CodeEdgeType> {
    prop_oneof![
        Just(CodeEdgeType::Calls),
        Just(CodeEdgeType::Imports),
        Just(CodeEdgeType::InheritsFrom),
        Just(CodeEdgeType::Defines),
        Just(CodeEdgeType::Concerns),
    ]
}

fn arb_code_edge() -> impl Strategy<Value = CodeEdge> {
    (
        arb_code_edge_type(),
        "function:[a-z0-9]{8}",
        "function:[a-z0-9]{8}",
        prop::option::of("[a-z:]{5,30}"),
        prop::option::of("[a-z_]{3,15}"),
    )
        .prop_map(|(edge_type, from, to, import_path, linked_by)| CodeEdge {
            edge_type,
            from,
            to,
            import_path,
            linked_by,
            created_at: Utc::now().to_rfc3339(),
        })
}

proptest! {
    #[test]
    fn workspace_config_roundtrip(config in arb_workspace_config()) {
        let json = serde_json::to_string(&config).unwrap();
        let decoded: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(config, decoded);
    }

    #[test]
    fn code_file_roundtrip(cf in arb_code_file()) {
        let json = serde_json::to_string(&cf).unwrap();
        let decoded: CodeFile = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(cf, decoded);
    }

    #[test]
    fn function_roundtrip(f in arb_function()) {
        let json = serde_json::to_string(&f).unwrap();
        let decoded: Function = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(f, decoded);
    }

    #[test]
    fn class_roundtrip(c in arb_class()) {
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Class = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(c, decoded);
    }

    #[test]
    fn interface_roundtrip(i in arb_interface()) {
        let json = serde_json::to_string(&i).unwrap();
        let decoded: Interface = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(i, decoded);
    }

    #[test]
    fn code_edge_type_roundtrip(et in arb_code_edge_type()) {
        let json = serde_json::to_string(&et).unwrap();
        let decoded: CodeEdgeType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(et, decoded);
    }

    #[test]
    fn code_edge_roundtrip(edge in arb_code_edge()) {
        let json = serde_json::to_string(&edge).unwrap();
        let decoded: CodeEdge = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(edge, decoded);
    }

    #[test]
    fn usage_event_roundtrip(event in arb_usage_event()) {
        let json = serde_json::to_string(&event).unwrap();
        let decoded: UsageEvent = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(event, decoded);
    }

    #[test]
    fn metrics_summary_roundtrip(summary in arb_metrics_summary()) {
        let json = serde_json::to_string(&summary).unwrap();
        let decoded: MetricsSummary = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(summary, decoded);
    }
}

// ── Phase 2: IPC protocol round-trip tests (T012) ────────────────────────────

use engram::daemon::DaemonStatus;
use engram::daemon::protocol::{
    IpcError as ProtocolError, IpcRequest, IpcResponse as ProtocolResponse,
};

fn arb_json_id() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        (0i64..100_000).prop_map(serde_json::Value::from),
        "[a-zA-Z0-9_-]{1,30}".prop_map(serde_json::Value::from),
    ]
}

fn arb_protocol_error() -> impl Strategy<Value = ProtocolError> {
    ("[a-z_]{1,30}", ".{1,80}").prop_map(|(method, message)| ProtocolError {
        code: match method.len() % 4 {
            0 => -32_700,
            1 => -32_600,
            2 => -32_601,
            _ => -32_603,
        },
        message,
        data: None,
    })
}

fn arb_ipc_request() -> impl Strategy<Value = IpcRequest> {
    (
        prop_oneof![Just("2.0".to_owned()), Just("1.0".to_owned())],
        prop::option::of(arb_json_id()),
        "[a-z_]{1,40}",
        // Avoid Some(Value::Null): JSON null deserializes to None for Option<Value>,
        // so Some(Null) does not round-trip. Use None or a non-null value.
        prop_oneof![
            Just(None::<serde_json::Value>),
            "[a-z]{1,10}".prop_map(|s| Some(serde_json::Value::from(s))),
        ],
    )
        .prop_map(|(jsonrpc, id, method, params)| IpcRequest {
            jsonrpc,
            id,
            method,
            params,
        })
}

fn arb_ipc_response() -> impl Strategy<Value = ProtocolResponse> {
    prop_oneof![
        // Use a non-null result value: Some(Null) round-trips as None via serde.
        arb_json_id().prop_map(|id| ProtocolResponse::success(id, serde_json::json!({}))),
        (arb_json_id(), arb_protocol_error())
            .prop_map(|(id, err)| ProtocolResponse::error(id, err)),
    ]
}

fn arb_daemon_status() -> impl Strategy<Value = DaemonStatus> {
    prop_oneof![
        Just(DaemonStatus::Starting),
        Just(DaemonStatus::Ready),
        Just(DaemonStatus::ShuttingDown),
    ]
}

proptest! {
    #[test]
    fn ipc_request_roundtrip(req in arb_ipc_request()) {
        let json = serde_json::to_string(&req).unwrap();
        let decoded: IpcRequest = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(req, decoded);
    }

    #[test]
    fn ipc_response_roundtrip(resp in arb_ipc_response()) {
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ProtocolResponse = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(resp, decoded);
    }

    #[test]
    fn ipc_error_roundtrip(err in arb_protocol_error()) {
        let json = serde_json::to_string(&err).unwrap();
        let decoded: ProtocolError = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(err, decoded);
    }

    #[test]
    fn daemon_status_roundtrip(status in arb_daemon_status()) {
        let json = serde_json::to_string(&status).unwrap();
        let decoded: DaemonStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(status, decoded);
    }
}
