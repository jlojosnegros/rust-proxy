mod proxy;
use std::time::Duration;

use tokio::{
    io::{AsyncWriteExt, copy_bidirectional},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tracing::{debug, error, info, instrument, warn};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Proxy listening on 127.0.0.1:8080");

    loop {
        let (client, remote_addr) = listener.accept().await?;
        println!("Client connected: {}", remote_addr);

        tokio::spawn(async move {
            match handle_connection(client).await {
                Ok((up, down)) => {
                    println!("[{}] Complete: {} up, {}  down", remote_addr, up, down);
                }
                Err(e) => {
                    eprintln!("[{}] Error: {}", remote_addr, e);
                }
            }
        });
    }
}

#[instrument(skip(client), fields(client_addr = %client.peer_addr().unwrap()))]
async fn handle_connection(mut client: TcpStream) -> std::io::Result<(u64, u64)> {
    // connect to upstream ( server ) at 127.0.0.1:9090
    debug!("Connecting to upstream");
    let upstream_result = timeout(CONNECT_TIMEOUT, TcpStream::connect("127.0.0.1:9090")).await;

    let mut upstream = match upstream_result {
        Ok(Ok(tcp_stream)) => {
            info!("Upstream connected");
            tcp_stream
        }
        Ok(Err(e)) => {
            let msg = format!("Failed to connecto to upstream: {}", e);
            eprintln!("{}", msg);
            let _ = client.write_all(msg.as_bytes()).await;
            error!(msg);
            return Err(e);
        }
        Err(_) => {
            eprintln!("Upstream connection timeout");
            warn!("Upstream connection timeout");
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "upstream connection timeout",
            ));
        }
    };

    println!("connected to upstream");

    // proxy data bidirectionally
    let result = timeout(IDLE_TIMEOUT, copy_bidirectional(&mut client, &mut upstream)).await;

    match result {
        Ok(Ok((up, down))) => {
            info!(bytes_up = up, bytes_down = down, "Transfer complete");
            Ok((up, down))
        }
        Ok(Err(e)) => {
            warn!(error = %e, "Transfer failed");
            Err(e)
        }
        Err(_) => {
            eprintln!("Idle timeout - no data for {:?}", IDLE_TIMEOUT);
            info!("Idle timeout - no data for {:?}", IDLE_TIMEOUT);
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "idle timeout",
            ))
        }
    }
}
