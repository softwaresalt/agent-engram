// This test exercises the legacy HTTP/SSE transport layer (axum router, SSE
// handler, health endpoint).  It is only compiled when the `legacy-sse` Cargo
// feature is enabled.  Default builds use the IPC transport exclusively.
#![cfg(feature = "legacy-sse")]

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::Request;
use serde_json::Value;
use tokio::{test, time};
use tower::ServiceExt;

use engram::server::{router::build_router, state::AppState};

#[test]
async fn sse_connection_lifecycle_accepts_and_times_out() {
    time::pause();

    let state = Arc::new(AppState::new(10));
    let app = build_router(state.clone());

    // Drive the SSE stream by advancing simulated time while reading the body.
    let drive_time = tokio::spawn(async {
        // Five keepalives at 15s intervals (take(5)); advance slightly extra to finish.
        for _ in 0..6 {
            time::advance(std::time::Duration::from_secs(15)).await;
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sse")
                .body(Body::empty())
                .expect("request builder"),
        )
        .await
        .expect("sse response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.starts_with("text/event-stream"));

    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("read body");
    drive_time.await.expect("time driver");

    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("keepalive"), "keepalive events present");

    // Stream ends after the configured keepalive window (~75s simulated)
    assert!(body_str.ends_with("\n\n"));
}

#[test]
async fn health_endpoint_reports_daemon_status() {
    let state = Arc::new(AppState::new(10));
    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request builder"),
        )
        .await
        .expect("health response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("read body");
    let payload: Value = serde_json::from_slice(&body).expect("valid json");

    assert_eq!(
        payload.get("active_workspaces").and_then(Value::as_u64),
        Some(0)
    );
    assert!(
        payload
            .get("uptime_seconds")
            .and_then(Value::as_u64)
            .is_some()
    );
}
