mod handler;
mod config;
mod metrics;
mod server;

use metrics::Metrics;
use std::sync::Arc;
use tokio::{signal, sync::RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::config::Config;

pub struct ServerState {
    pub metrics: Metrics,
    pub config: RwLock<Config>,
    pub shutdown: CancellationToken,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            metrics: Metrics::new(),
            config: RwLock::new(Config::default()),
            shutdown: CancellationToken::new(),
        }
    }
}

async fn drain_connections(metrics: &Metrics, drain_timeout: u64) {
    todo!()
}
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::SignalKind;

        signal::unix::signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C"),
        _ = terminate => info!("Receiver SIGTERM"),
    }
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting RustProxy Milestone 1");

    // Create server state
    let state = Arc::new(ServerState::new());

    // Start echo Service
    let echo_state = state.clone();
    let echo_handle = tokio::spawn(async move {
        if let Err(e) = server::run_echo_server(echo_state, "127.0.0.1:8080").await {
            error!("Echo server error: {}", e);
        }
    });

    info!("Echo server listeing on 127.0.0.1:8080");

    // Start echo Service
    let health_state = state.clone();
    let health_handle = tokio::spawn(async move {
        if let Err(e) = server::run_health_server(health_state, "127.0.0.1:8081").await {
            error!("Health server error: {}", e);
        }
    });

    info!("Health server listeing on 127.0.0.1:8081");
    info!("Press Ctrl+c to shutdown");

    shutdown_signal().await;

    info!("Shutdown signal received. Initiating graceful shutdown ... ");
    // Signal all handlers to stop
    state.shutdown.cancel();

    // Wait for connections to drain
    let drain_timeout = state.config.read().await.drain_timeout_secs;
    drain_connections(&state.metrics, drain_timeout).await;

    // stop server tasks
    echo_handle.abort();
    health_handle.abort();

    let final_metrics = state.metrics.snapshot();
    info!(
        "Final stats: {} total connections, {} bytes in, {} bytes out",
        final_metrics.total_connections, final_metrics.bytes_received, final_metrics.bytes_sent
    );

    info!("Server shut down");
}
