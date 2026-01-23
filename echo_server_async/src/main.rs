use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::RwLock,
    time::timeout,
};

#[derive(Debug, Clone)]
struct Config {
    max_message_size: usize,
    server_name: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_message_size: 1024,
            server_name: String::from("RustProxy"),
        }
    }
}
#[derive(Debug)]
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
    let config = Arc::new(RwLock::new(Config::default()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8081").await?;
    println!(
        "Server {} listening on 127.0.0.1:8081",
        config.read().await.server_name
    );

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
        let clone_config = config.clone();

        // spawn a task for each connection
        tokio::spawn(async move {
            if let Err(e) = handle_client(tcp_stream, cloned_state.clone(), clone_config).await {
                eprint!("Error handling {}: {}", sock_addr, e);
            }
            let remaining = cloned_state.connection_ended();
            println!("Client disconnected (active: {})", remaining);
        });
    }
}

async fn handle_client(
    stream: tokio::net::TcpStream,
    _state: Arc<ServerState>,
    config: Arc<RwLock<Config>>,
) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let idle_timeout = Duration::from_secs(30);
    const SET_MAX: &str = "SET_MAX ";

    loop {
        line.clear();
        // Read asynchronously --> does NOT block the thread
        // use a timeout
        let ret = timeout(idle_timeout, reader.read_line(&mut line)).await;
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
        let trimmed = line.trim();

        if trimmed == "CONFIG" {
            let cfg = config.read().await;

            let response = format!("Config: {:?}\n", cfg);
            writer.write_all(response.as_bytes()).await?;
        } else if trimmed.starts_with(SET_MAX) {
            if let Ok(new_size) = trimmed[SET_MAX.len()..].parse::<usize>() {
                let mut cfg = config.write().await;
                cfg.max_message_size = new_size;

                let response = format!("Updated max_message_size to {}\n", new_size);
                writer.write_all(response.as_bytes()).await?;
            } else {
                writer.write_all(b"Invalid number\n").await?;
            }
        } else {
            // Just need to echo the message
            // But first lets check against the max_message_size
            let max_size = config.read().await.max_message_size;
            if line.len() > max_size {
                writer
                    .write_all(format!("Message too long (max: {})", max_size).as_bytes())
                    .await?;
            } else {
                writer.write_all(line.as_bytes()).await?;
            }
        }

        // write async
        // stream.write_all(&buffer[..bytes_read]).await?;
    }
    Ok(())
}
