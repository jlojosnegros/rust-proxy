use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8081").await?;
    println!("Server listening on 127.0.0.1:8081");

    // Accept a single connection
    let (mut tcp_stream, sock_addr) = listener.accept().await?;
    println!("Client connected from {}", sock_addr);

    let mut buffer = [0u8; 1024];

    loop {
        // Read asynchronously --> does NOT block the thread
        let bytes_read = tcp_stream.read(&mut buffer).await?;

        if bytes_read == 0 {
            println!("Client disconnected");
            break;
        }

        println!("Received {} bytes", bytes_read);

        // write async
        tcp_stream.write_all(&buffer[..bytes_read]).await?;
    }
    Ok(())
}
