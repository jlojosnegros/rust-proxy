mod config;
mod pooldead;
mod pool;
mod proxy;
use std::{sync::Arc, time::Duration};

use deadpool::managed::Pool;
use tokio::{
    io::{AsyncWriteExt, copy_bidirectional},
    net::{TcpListener, TcpStream},
    sync::RwLock,
    time::timeout,
};
use tracing::{debug, error, info, instrument, warn};

use crate::{
    config::{Config, load_config},
    pooldead::TcpConnectionManager,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

type ConnectionPool = Pool<TcpConnectionManager>;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config = Arc::new(RwLock::new(load_config()));
    let manager = TcpConnectionManager::new(config.read().await.upstream.to_string());
    let pool = Pool::builder(manager)
        .max_size(10)
        .build()
        .expect("Failed to create the pool");
    debug!(max_size=pool.status().max_size, size=pool.status().size, used=pool.status().size-pool.status().available, available=pool.status().available, "Pool status");
    debug!("Connection Pool created");

    let listener = TcpListener::bind(config.read().await.listen.to_string()).await?;
    debug!(
        address = config.read().await.listen.to_string(),
        "Proxy listening "
    );

    loop {
        let (client, remote_addr) = listener.accept().await?;
        debug!("Client connected: {}", remote_addr);

        let config_cloned = config.clone();
        let pool_cloned = pool.clone();
        tokio::spawn(async move {
            match handle_connection(client, config_cloned, pool_cloned).await {
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

#[instrument(skip(client, _config, pool), fields(client_addr = %client.peer_addr().unwrap()))]
async fn handle_connection(
    mut client: TcpStream,
    _config: Arc<RwLock<Config>>,
    pool: ConnectionPool,
) -> std::io::Result<(u64, u64)> {
    // connect to upstream ( server ) at 127.0.0.1:9090
    debug!("Connecting to upstream");
    // Get connection from the pool
    //
    let upstream_result = timeout(CONNECT_TIMEOUT, pool.get()).await;

    debug!(used=pool.status().size-pool.status().available, available=pool.status().available, "Pool status");
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
            return Err(std::io::Error::new(std::io::ErrorKind::NotConnected, e));
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
    let result = timeout(IDLE_TIMEOUT, copy_bidirectional(&mut client, &mut *upstream)).await;

    match result {
        Ok(Ok((up, down))) => {
            info!(bytes_up = up, bytes_down = down, "Transfer complete");

            // NOTE: deadpool returns the connection when dropped
            // return connection to the pool (only if there is no error)
            // pool.release(upstream).await;

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
