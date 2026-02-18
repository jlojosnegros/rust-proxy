use std::sync::atomic::AtomicUsize;

use deadpool::managed::Manager;
use tokio::net::TcpStream;
use tracing::info;

pub(crate) struct TcpConnectionManager {
    upstream_addr: String,
    created: AtomicUsize,
}

impl TcpConnectionManager {
    pub(crate) fn new(upstream_addr: String) -> Self {
        Self {
            upstream_addr,
            created: AtomicUsize::new(0),
        }
    }
}

impl Manager for TcpConnectionManager {
    type Type = TcpStream;

    type Error = std::io::Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let count = self.created.load(std::sync::atomic::Ordering::Relaxed) + 1;
        info!(
            number = count,
            address = self.upstream_addr,
            "Creating connection"
        );
        TcpStream::connect(&self.upstream_addr).await
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        _metrics: &deadpool::managed::Metrics,
    ) -> deadpool::managed::RecycleResult<Self::Error> {
        // Check if connection is still valid
        // A simple check : try to peek (non-blocking read)
        let mut buf = [0u8; 1];

        match conn.try_read(&mut buf) {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Would block => connection is healthy
                Ok(())
            }
            Ok(0) => {
                // connection closed
                Err(deadpool::managed::RecycleError::Message(
                    "Connection closed".into(),
                ))
            }
            Ok(_) => {
                // got data unexpectedly
                Err(deadpool::managed::RecycleError::Message(
                    "Unexpected data from upstream".into(),
                ))
            }

            Err(e) => {
                //any other error
                Err(deadpool::managed::RecycleError::Message(
                    format!("Connection Error: {}", e).into(),
                ))
            }
        }
    }
}
