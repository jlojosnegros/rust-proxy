use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn main() -> std::io::Result<()> {
    // TcpListener::bind combines "socket() + bind() + listen()" calls
    let listener = TcpListener::bind("127.0.0.1:8081")?;
    println!("Server listening on 127.0.0.1:8081");

    // loop forever accepting new connections
    // incoming() returns an iterator of Result<TcpStream>
    for accept_result in listener.incoming() {
        match accept_result {
            Ok(tcp_stream) => {
                // new incomming connection -> need to spawn a new thread
                // OJO: "move ||" captura el contexto (tcp_stream aqui) y lo MUEVE
                // es decir el thread toma el ownership del stream
                thread::spawn(move || {
                    if let Err(e) = handle_client(tcp_stream) {
                        eprintln!("Error handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    // Get the Address of the client connected to the "socket" (aka TcpStream)
    let addr = stream.peer_addr()?;

    let thread_id = thread::current().id();
    println!("[{:?}] Client connected from {}", thread_id, addr);

    let mut buffer = [0u8; 1024];

    loop {
        let bytes_read = stream.read(&mut buffer)?;

        if bytes_read == 0 {
            println!("[{:?}] Client {} disconnected", thread_id, addr);
            break;
        }

        println!(
            "[{:?}] Received {} bytes from {}",
            thread_id, bytes_read, addr
        );
        stream.write_all(&buffer[..bytes_read])?;
    }
    Ok(())
}
