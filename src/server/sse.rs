#![allow(dead_code)]

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use tokio_stream::{StreamExt, wrappers::IntervalStream};

use crate::server::state::SharedState;

/// SSE handler that sends keepalive comments every 15 seconds and times out after 60 seconds.
pub async fn sse_handler(
    State(state): State<SharedState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    state.increment_connections();

    let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(15)))
        .take(5)
        .map(|_| Ok(Event::default().comment("keepalive")));

    let state_for_close = state.clone();

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .on_close(move || async move {
            state_for_close.decrement_connections();
        })
}
