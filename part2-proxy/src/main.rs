use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> std::io::Result<()>{

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


    Ok(())
}
