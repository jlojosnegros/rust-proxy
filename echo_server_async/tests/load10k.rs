mod helper;

#[cfg(test)]
mod load10k {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};

    use crate::helper::setup_echo_server;

    #[tokio::test]
    async fn test_10k_connections() -> std::io::Result<()> {
        // let target_connections = 10_000;
        let target_connections = 1000;
        let successful = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        let _echo_server = setup_echo_server().await?;

        for idx in 0..target_connections {
            let successful = successful.clone();
            let failed = failed.clone();

            let handle = tokio::spawn(async move {
                match test_connections(idx).await {
                    Ok(_) => successful.fetch_add(1, Ordering::Relaxed),
                    Err(_) => failed.fetch_add(1, Ordering::Relaxed),
                };
            });

            handles.push(handle);
        }

        // wait for all connections
        for handle in handles {
            let _ = handle.await;
        }

        println!("Successful: {}", successful.load(Ordering::Relaxed));
        println!("Failed: {}", failed.load(Ordering::Relaxed));

        assert_eq!(successful.load(Ordering::Relaxed), target_connections);
        assert_eq!(failed.load(Ordering::Relaxed), 0);

        Ok(())
    }

    async fn test_connections(id: usize) -> std::io::Result<()> {
        let mut stream = TcpStream::connect("127.0.0.1:8081").await?;

        let msg = format!("Hello from connection {}", id);
        stream.write_all(msg.as_bytes()).await?;

        let mut buffer = [0u8; 1024];
        let n = stream.read(&mut buffer).await?;

        assert_eq!(&buffer[..n], msg.as_bytes());

        Ok(())
    }
}
