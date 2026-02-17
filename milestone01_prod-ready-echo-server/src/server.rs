use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::error;

use crate::ServerState;

pub async fn run_echo_server(state: Arc<ServerState>, addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        tokio::select! {
            biased;

            _ = state.shutdown.cancelled() => {
                break;
            }

            result = listener.accept() => {
                match result {
                    Ok((tcp_stream, remote_addr)) => {
                        let state = state.clone();
                        let remote_addr_str = remote_addr.to_string();
                        tokio::spawn(async move {
                            handle_echo_connection(tcp_stream, state, remote_addr_str).await;
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn run_health_server(state: Arc<ServerState>, addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        tokio::select! {
            biased;

            _ = state.shutdown.cancelled() => {
                break;
            }

            result = listener.accept() => {
                match result {
                    Ok((tcp_stream, remote_addr)) => {
                        let state = state.clone();
                        let remote_addr_str = remote_addr.to_string();
                        tokio::spawn(async move {
                            handle_health_connection(tcp_stream, state, remote_addr_str).await;
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
        }
    }
    Ok(())
}
