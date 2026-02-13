use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    signal,
    sync::RwLock,
    time::timeout,
};
use tokio_util::sync::CancellationToken;

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
    shutdown: CancellationToken,
}
impl ServerState {
    fn new() -> Self {
        Self {
            active_connections: AtomicUsize::new(0),
            total_connections: AtomicUsize::new(0),
            shutdown: CancellationToken::new(),
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

async fn wait_for_shutdown() {
    // lets create a future that completes only when ctrl+c is pressed
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    };

    // ------------------------------
    // Handle for TERMINATE (SIGTERM)
    // ------------------------------

    // In Unix/Linux create a future that completes when receive SIGTERM
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::SignalKind;

        signal::unix::signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    // for not unix systems just create a never ending future for now
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // now wait for any of these futures to complete
    tokio::select! {
        _ = ctrl_c => println!("Received Ctrl+C"),
        _ = terminate => println!("Received SIGTERM"),
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
        tokio::select! {
            // Note: tokio::select pools branches in no specific order by default.
            // If one branch returns "Ready" all the other branches are cancelled.
            // This may imply some data loss if the branch was, for example, reading from network
            // some operations are safe and some others are not you need to check case by case.
            // "biased" instruct tokio select to pool branches in the order they are defined.
            // this helps if you have some branches that have more priority than others, but
            // may cause starvation if one of them is defined before and is returning Ready often
            biased;

            // Branch 1: Shutdown signal Received
            _ = wait_for_shutdown() => {
                println!("Shutdown signal received");
                break;
            }
            // Branch 2: Accept new connection
            result = listener.accept() => {
                match result {
                    Ok((tcp_stream, sock_addr)) => {

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
                    Err(e) => eprintln!("Accept error: {}", e),
                }
            }
        }
    }
    println!("Server Shutdown");

    // NOTE:  as we have exited the loop we are not accepting any more connections
    //
    // Signal all handlers to stop
    shared_state.shutdown.cancel();

    let drain_timeout = Duration::from_secs(5);
    let drain_result = timeout(drain_timeout, async {
        while shared_state.active_connections.load(Ordering::Relaxed) > 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    match drain_result {
        Ok(_) => println!("All connections drained successfully"),
        Err(_) => {
            let remaining = shared_state.active_connections.load(Ordering::Relaxed);
            println!(" Drain timeout, {} connections remaining", remaining);
        }
    }

    Ok(())
}

async fn handle_client(
    stream: tokio::net::TcpStream,
    state: Arc<ServerState>,
    config: Arc<RwLock<Config>>,
) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let idle_timeout = Duration::from_secs(30);
    const SET_MAX: &str = "SET_MAX ";

    loop {
        line.clear();
        tokio::select! {
            // jlom: Here biased is not really needed because we are saving the bytes readded in
            // a BufReader outside of the loop so *the state is saved between loop iterations and 
            // so there is no data loss*
            biased;

            // Branch one: Shutdown requested via CancellationToken
            _ = state.shutdown.cancelled() => {
                    println!(" handler received shutdown signal");
                    let _ = writer.write_all(b"Server shutting down ... \n").await;
                    break;
            }
            // Branch two: Normal read operation
            ret = timeout(idle_timeout, reader.read_line(&mut line)) => {
                // Read asynchronously --> does NOT block the thread
                // use a timeout
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
            }
        }
    }
    Ok(())
}
