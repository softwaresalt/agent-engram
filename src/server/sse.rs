#![allow(dead_code)]

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use tokio_stream::{StreamExt, wrappers::IntervalStream};
use uuid::Uuid;

use crate::errors::{SystemError, TMemError};
use crate::server::state::SharedState;

/// Guard that unregisters a connection on drop (US5/T095).
///
/// When the SSE stream ends (client disconnect or timeout), the guard
/// is dropped, spawning a cleanup task that removes the connection
/// from the registry and decrements the active count.
struct ConnectionGuard {
    state: SharedState,
    connection_id: String,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let state = self.state.clone();
        let id = std::mem::take(&mut self.connection_id);
        tokio::spawn(async move {
            state.unregister_connection(&id).await;
        });
    }
}

/// SSE handler with rate limiting (FR-025) and connection cleanup (T095).
///
/// Assigns a unique connection ID (UUID v4) per FR-003, enforces the
/// connection rate limit per FR-025, and sends keepalive comments every
/// 15 seconds per FR-004 with a timeout after 5 intervals.
pub async fn sse_handler(State(state): State<SharedState>) -> Response {
    // FR-025: Rate limit check before accepting connection
    if !state.check_rate_limit().await {
        let err = TMemError::System(SystemError::RateLimited);
        return (StatusCode::TOO_MANY_REQUESTS, Json(err.to_response())).into_response();
    }

    // FR-003: Assign unique connection ID
    let connection_id = Uuid::new_v4().to_string();
    state.register_connection(connection_id.clone()).await;

    // T095: Guard cleans up on stream end / client disconnect
    let guard = ConnectionGuard {
        state: state.clone(),
        connection_id,
    };

    let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(15)))
        .take(5)
        .map(move |_| {
            // Hold guard alive for the stream's lifetime
            let _guard = &guard;
            Ok::<_, Infallible>(Event::default().comment("keepalive"))
        });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}
