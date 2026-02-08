use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

use t_mem::{
    config::Config,
    init_tracing,
    server::{router::build_router, state::AppState},
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();
    config.validate().map_err(anyhow::Error::msg)?;
    config.ensure_data_dir()?;
    init_tracing(config.log_format());

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let state = Arc::new(AppState::new());
    let app = build_router(state.clone());

    let listener = TcpListener::bind(addr).await?;
    println!("t-mem daemon listening on {addr}");
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }

    println!("Shutting down t-mem daemon");
}
