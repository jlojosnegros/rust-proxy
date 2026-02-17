use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Proxy listening on 127.0.0.1:8080");

    loop {
        let (client, remote_addr) = listener.accept().await?;
        println!("Client connected: {}", remote_addr);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(client).await {
                eprintln!("Error: {}", e);
            }
        });
    }
}

async fn handle_connection(mut client: TcpStream) -> std::io::Result<()> {
    // Target : connect to upstream ( server ) at 127.0.0.1:9090
    // and forward downstream (client ) data to it.

    let mut upstream = TcpStream::connect("127.0.0.1:9090").await?;
    println!("connected to upstream");

    let mut buffer = [0u8; 1024];

    loop {
        let bytes_read = client.read(&mut buffer).await?;
        if bytes_read == 0 {
            println!("Client disconnected");
            break;
        }

        println!("Forwarding {} bytes to upstream", bytes_read);
        upstream.write_all(&buffer[..bytes_read]).await?;
    }

    Ok(())
}
