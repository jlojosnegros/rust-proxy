use std::time::Duration;

use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
    time::timeout,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> std::io::Result<()> {
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

async fn handle_connection(mut client: TcpStream) -> std::io::Result<(u64, u64)> {
    // connect to upstream ( server ) at 127.0.0.1:9090
    let upstream_result = timeout(CONNECT_TIMEOUT, TcpStream::connect("127.0.0.1:9090")).await;

    let mut upstream = match upstream_result {
        Ok(Ok(tcp_stream)) => tcp_stream,
        Ok(Err(e)) => {
            eprintln!("Failed to connecto to upstream: {}", e);
            return Err(e);
        }
        Err(_) => {
            eprintln!("Upstream connection timeout");
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
        Ok(Ok((up, down))) => Ok((up, down)),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            eprintln!("Idle timeout - no data for {:?}", IDLE_TIMEOUT);
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "idle timeout",
            ))
        }
    }
}
