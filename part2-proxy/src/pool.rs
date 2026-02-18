use std::sync::Arc;

use tokio::{net::TcpStream, sync::Mutex};
use tracing::{info, warn};

pub(crate) struct ConnectionPool {
    upstream_addr: String,
    connections: Mutex<Vec<TcpStream>>,
    max_size: usize,
}

impl ConnectionPool {
    pub(crate) fn new(upstream_addr: &str, max_size: usize) -> Self {
        Self {
            upstream_addr: upstream_addr.to_owned(),
            connections: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    pub(crate) async fn reserve(&self) -> std::io::Result<TcpStream> {

        // try to get from the pool
        {
            let mut conns = self.connections.lock().await;

            if let Some(conn) = conns.pop() {
                info!("Reusing pooled connection ({} remaining)", conns.len());
                return Ok(conn);
            }
        }

        // Create new connection
        info!("Creating new connection to {}", self.upstream_addr);
        TcpStream::connect(&self.upstream_addr).await
    }

    pub(crate) async fn release(&self, conn: TcpStream) {
        let mut conns = self.connections.lock().await;
        if conns.len() < self.max_size {
            info!("Returning connection to pool ({} total)", conns.len());
            conns.push(conn);
        } else {
            warn!("Pool full, dropping connection");
        }
    }

    pub(crate) async fn size(&self) -> usize {
        self.connections.lock().await.len()
    }
}

struct PooledConnection {
    conn: Option<TcpStream>,
    pool: Arc<ConnectionPool>,
}
