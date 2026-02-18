use tokio::{
    io::copy_bidirectional,
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
    // connect to upstream ( server ) at 127.0.0.1:9090
    let mut upstream = TcpStream::connect("127.0.0.1:9090").await?;
    println!("connected to upstream");

    // proxy data bidirectionally
    let (client2upstream, upstream2client) = copy_bidirectional(&mut client, &mut upstream).await?;

    println!(
        "Connection closed.\n\t Client -> Upstream: {} bytes.\n\t Upstream -> Client: {} bytes.",
        client2upstream, upstream2client
    );

    Ok(())
}
