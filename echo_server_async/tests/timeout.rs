mod helper;
#[cfg(test)]
mod timeout {

    use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};

    use crate::helper::setup_echo_server;

    #[tokio::test]
    async fn test_timeout() -> std::io::Result<()> {
        let _echo_server = setup_echo_server().await?;
        let mut stream = TcpStream::connect("127.0.0.1:8081").await?;

        let msg = "Hello from connection ".to_string();
        stream.write_all(msg.as_bytes()).await?;

        let mut buffer = [0u8; 1024];
        let n = stream.read(&mut buffer).await?;

        assert_eq!(&buffer[..n], msg.as_bytes());

        //stop sending and wait for timeout
        let mut buf = [0u8; 1];
        let n = stream.read(&mut buf).await?;
        assert_eq!(n, 0);

        Ok(())
    }
}
