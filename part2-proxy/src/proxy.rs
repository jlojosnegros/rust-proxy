use std::time::{Duration, Instant};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::watch,
};

struct CopyMetrics {
    bytes_transferred: u64,
    last_activity: Instant,
}

async fn copy_with_metrics<R, W>(
    mut reader: R,
    mut writer: W,
    activity_tx: watch::Sender<Instant>,
) -> std::io::Result<u64>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut total = 0u64;
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }

        writer.write_all(&buffer[..bytes_read]).await?;
        total += bytes_read as u64;

        // notify activity
        let _ = activity_tx.send(Instant::now());
    }

    Ok(total)
}

async fn proxy_with_idle_timeout(
    client: TcpStream,
    upstream: TcpStream,
    idle_timeout: Duration,
) -> std::io::Result<(u64, u64)> {

    let (client_read, client_write) = client.into_split();
    let (upstream_read, upstream_write) = upstream.into_split();

    let (activity_tx, mut activity_rx) = watch::channel(Instant::now());
    let activity_tx2 = activity_tx.clone();

    let client2upstream = tokio::spawn(async move {
        copy_with_metrics(client_read, upstream_write, activity_tx).await
    });

    let upstream2client = tokio::spawn(async move {
        copy_with_metrics(upstream_read, client_write, activity_tx2).await
    });

    // idle timeout monitor
    let timeout_monitor = async {
        loop {
            let last = *activity_rx.borrow();
            if last.elapsed() > idle_timeout {
                return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "idle timeout"));
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };


    tokio::select! {
        result = async { tokio::try_join!(client2upstream, upstream2client)} => {
            let (client2upstream_result, upstream2client_result) = result?;
            Ok((client2upstream_result?, upstream2client_result?))
        }
        err = timeout_monitor => {
            Err(err?)
        }
    }
}
