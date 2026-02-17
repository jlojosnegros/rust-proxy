use std::sync::Arc;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use tracing::{debug, info, warn};
use tracing_subscriber::fmt::writer;

use crate::ServerState;

pub async fn handle_echo_connection(tcp_stream: TcpStream, state: Arc<ServerState>, addr: String) {
    state.metrics.connection_started();
    info!(
        "Client {} connected. (active: {})",
        addr,
        state.metrics.active_connections()
    );

    let result = echo_loop(tcp_stream, state.clone()).await;

    if let Err(e) = result {
        debug!("Connection {}, error: {}", addr, e);
    }

    state.metrics.connection_ended();
    info!(
        "Client {} disconnected. (active: {})",
        addr,
        state.metrics.active_connections()
    );
}

async fn echo_loop(tcp_stream: TcpStream, state: Arc<ServerState>) -> std::io::Result<()> {
    let (reader, mut writer) = tcp_stream.into_split();

    let mut bufreader = BufReader::new(reader);
    let mut line = String::new();

    let shutdown = state.shutdown.clone();

    loop {
        line.clear();

        tokio::select! {
            _ = shutdown.cancelled() => {
                // Graceful shutdown -> send goodbye message
                warn!("Shutdown signal received, closing connection");
                let _ = writer.write_all(b"Server shutting down, goodbye!\n").await;
                break;
            }
            result = bufreader.read_line(&mut line) => {

                let bytes_read = result?;

                if bytes_read == 0 {
                    // client disconnected
                    break;
                }

                state.metrics.record_bytes_received(bytes_read);

                // Check message size limit
                let max_size = state.config.read().await.max_message_size;
                if line.len() > max_size {
                    let error_message = format!("Message too large (max {} bytes)\n", max_size);
                    writer.write_all(error_message.as_bytes()).await?;
                    state.metrics.record_bytes_sent(error_message.len());
                    continue;
                }


                // Hendle special commands
                let trimmed = line.trim();

                if trimmed == "STATS" {
                    let snapshot = state.metrics.snapshot();
                    let response = serde_json::to_string_pretty(&snapshot).unwrap() + "\n";
                    writer.write_all(response.as_bytes()).await?;
                    state.metrics.record_bytes_sent(response.len());
                } else if trimmed == "CONFIG" {
                    let config = state.config.read().await.clone();
                    let response = serde_json::to_string_pretty(&config).unwrap() + "\n";
                    writer.write_all(response.as_bytes()).await?;
                    state.metrics.record_bytes_sent(response.len());
                } else if trimmed == "SET_MAX " {
                    if let Ok(new_size) = trimmed[8..].parse::<usize>() {
                        state.config.write().await.max_message_size = new_size;
                        let response = format!("Updated max_message_size to {}\n", new_size);
                        writer.write_all(response.as_bytes()).await?;
                        state.metrics.record_bytes_sent(response.len());
                    } else {
                        let msg = "Invalid number\n";
                        writer.write_all(msg.as_bytes()).await?;
                        state.metrics.record_bytes_sent(msg.len());
                    }
                } else {
                    // Echo back
                    writer.write_all(line.as_bytes()).await?;
                    state.metrics.record_bytes_sent(line.len());
                }
            }
        }
    }

    Ok(())
}

pub async fn handle_health_connection(tcp_stream: TcpStream, state: Arc<ServerState>) {
    let result = health_request(tcp_stream, state).await;
    if let Err(e) = result {
        debug!("Health check error: {}", e);
    }
}

async fn health_request(mut tcp_stream: TcpStream, state: Arc<ServerState>) -> std::io::Result<()> {
    let mut buffer = [0u8; 1024];
    let n = tcp_stream.read(&mut buffer).await?;

    let request = String::from_utf8_lossy(&buffer[..n]);

    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    let response = match path {
        "/health" => {
            let body = r#"{"status":"healthy"}"#;
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
        }
        "/metrics" => {
            let snapshot = state.metrics.snapshot();
            let body = serde_json::to_string(&snapshot).unwrap();
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
        }
        _ => {
            let body = r#"{"error":"not found"}"#;
            format!(
                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
        }
    };


    tcp_stream.write_all(response.as_bytes()).await?;

    Ok(())
}
