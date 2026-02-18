use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    join,
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

async fn handle_connection(client: TcpStream) -> std::io::Result<()> {
    // Target : connect to upstream ( server ) at 127.0.0.1:9090
    // and forward downstream (client ) data to it.

    let upstream = TcpStream::connect("127.0.0.1:9090").await?;
    println!("connected to upstream");

    let (mut client_read, mut client_write) = client.into_split();
    let (mut upstream_read, mut upstream_write) = upstream.into_split();

    let client2upstream = tokio::spawn(async move {
        let mut buffer = [0u8; 4096];
        let mut total_bytes = 0usize;
        loop {
            let ret = client_read.read(&mut buffer).await;
            let bytes_read = match ret {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            total_bytes += bytes_read;

            if upstream_write
                .write_all(&buffer[..bytes_read])
                .await
                .is_err()
            {
                break;
            }
        }

        total_bytes
    });

    let upstream2client = tokio::spawn(async move {
        let mut buffer = [0u8; 4096];
        let mut total_bytes = 0usize;
        loop {
            let ret = upstream_read.read(&mut buffer).await;
            let bytes_read = match ret {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            total_bytes += bytes_read;

            if client_write.write_all(&buffer[..bytes_read]).await.is_err() {
                break;
            }
        }

        total_bytes
    });

    let (total_client2upstream, total_upstream2client) = join!(client2upstream, upstream2client);

    println!(
        "Connection closed.\n\t Client -> Upstream: {} bytes.\n\t Upstream -> Client: {} bytes.",
        total_client2upstream.unwrap_or(0),
        total_upstream2client.unwrap_or(0)
    );

    Ok(())
}
