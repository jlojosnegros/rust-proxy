use std::{io::{Read, Write}, net::TcpListener};

fn main() -> std::io::Result<()> {

    // TcpListener::bind combines "socket() + bind() + listen()" calls
    let listener = TcpListener::bind("127.0.0.1:8081")?;
    println!("Server listening on 127.0.0.1:8081");

    // This accepts a single connection
    // Will block until a client connects
    // return the connected socket (tcp_stream) and the client address (socker_addr)
    let (mut tcp_stream, socker_addr) = listener.accept()?;
    println!("Client connected from {}", socker_addr);

    // Buffer for reading data
    // 1024 bytes initialized at 0
    let mut buffer = [0u8; 1024];


    loop {
        // read data from client
        // read() blocks until data is available or connection closes
        // Return number of bytes read (0 -> means closed connection)

        let bytes_read = tcp_stream.read(&mut buffer)?;
        if bytes_read == 0 {
            //connection closed by client -> exit read/write loop
            println!("Client disconnected");
            break;
        }
        println!("Received {} bytes", bytes_read);

        // Echo data back to client
        // tcp_stream is a two way connection
        tcp_stream.write_all(&buffer[..bytes_read])?;

    }

    Ok(())
}
