
use std::time::Duration;
use tokio::process::{Child, Command};

pub struct EchoServer {
    process: Child,
}
pub async fn setup_echo_server() -> std::io::Result<EchoServer> {
    let server = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "/home/jojosneg/source/mine/rust/proxy/echo_server_async/Cargo.toml",
        ])
        .kill_on_drop(true) // Mata el servidor cuando el test termina
        .spawn()?;
    // let mut server = Command::new("cargo")
    //     .args(["run", "--manifest-path", "/home/jojosneg/source/mine/rust/proxy/echo_server_async/Cargo.toml"])
    //     .kill_on_drop(true)
    //     .spawn()?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(EchoServer { process: server })
}

impl Drop for EchoServer {
    fn drop(&mut self) {
        self.process.kill();
    }
}
