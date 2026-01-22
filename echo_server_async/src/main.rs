use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8081").await?;
    println!("Server listening on 127.0.0.1:8081");

    loop {
        // Accept a single connection
        let (tcp_stream, sock_addr) = listener.accept().await?;
        println!("Client connected from {}", sock_addr);

        // spawn a task for each connection
        tokio::spawn(async move {
            if let Err(e) = handle_client(tcp_stream).await {
                eprint!("Error handling {}: {}", sock_addr, e);
            }
        });
    }
}

async fn handle_client(mut stream: tokio::net::TcpStream) -> std::io::Result<()> {
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
            println!("Client disconnected");
            break;
        }

        println!("Received {} bytes", bytes_read);

        // write async
        stream.write_all(&buffer[..bytes_read]).await?;
    }
    Ok(())
}
