use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

struct ServerState {
    active_connections: AtomicUsize,
    total_connections: AtomicUsize,
}
impl ServerState {
    fn new() -> Self {
        Self {
            active_connections: AtomicUsize::new(0),
            total_connections: AtomicUsize::new(0),
        }
    }

    fn connection_started(&self) -> (usize, usize) {
        let total = self.total_connections.fetch_add(1, Ordering::Relaxed) + 1;
        let active = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        (active, total)
    }
    fn connection_ended(&self) -> usize {
        self.active_connections.fetch_sub(1, Ordering::Relaxed) - 1
    }
}
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create shared state
    let shared_state = Arc::new(ServerState::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8081").await?;
    println!("Server listening on 127.0.0.1:8081");

    loop {
        // Accept a single connection
        let (tcp_stream, sock_addr) = listener.accept().await?;
        let (active, total) = shared_state.connection_started();
        println!(
            "Client connected from {} (active: {}, total: {})",
            sock_addr, active, total
        );

        // Clone the state to share it with the async tasks
        let cloned_state = shared_state.clone();

        // spawn a task for each connection
        tokio::spawn(async move {
            if let Err(e) = handle_client(tcp_stream, cloned_state.clone()).await {
                eprint!("Error handling {}: {}", sock_addr, e);
            }
            let remaining = cloned_state.connection_ended();
            println!("Client disconnected (active: {})", remaining);
        });
    }
}

async fn handle_client(
    mut stream: tokio::net::TcpStream,
    _state: Arc<ServerState>,
) -> std::io::Result<()> {
    let mut buffer = [0u8; 1024];
    let idle_timeout = Duration::from_secs(30);

    loop {
        // Read asynchronously --> does NOT block the thread
        // use a timeout
        let ret = timeout(idle_timeout, stream.read(&mut buffer)).await;
        let bytes_read = match ret {
            Ok(result) => result?,
            Err(_) => {
                println!("timeout");
                break;
            }
        };

        if bytes_read == 0 {
            break;
        }

        println!("Received {} bytes", bytes_read);
        // write async
        stream.write_all(&buffer[..bytes_read]).await?;
    }
    Ok(())
}
