use dns::parse::parse_dns_tcp_packet;
use dns::utils::respond;
use std::net::SocketAddr;
use std::{env, io};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn tcp_handle_connection(mut socket: TcpStream, addr: SocketAddr) -> Result<(), io::Error> {
    let mut buf = vec![0u8; 1024];

    // no loop: we don't handle pipelining correctly
    // so only handle one query per connection
    let nread = socket.read(&mut buf).await?;
    let packet = match parse_dns_tcp_packet(&buf[0..nread]) {
        Ok((_, packet)) => packet,
        Err(e) => {
            eprintln!("Malformed DNS query from {:?}: {:?}", addr, e);
            return Ok(());
        }
    };

    let mut out_buf = Vec::new();
    let resp = respond(packet, &addr);
    resp.serialize_tcp_to(&mut out_buf)?;
    match socket.write_all(&out_buf).await {
        Ok(()) => {
            println!("Sent response to {}", addr);
        }
        Err(e) => {
            eprintln!("Failed to send response to {:?}: {:?}", addr, e);
        }
    }

    Ok(())
}

async fn tcp_server(mut socket: TcpListener) -> Result<(), io::Error> {
    loop {
        let (conn_socket, addr) = socket.accept().await?;
        tokio::spawn(tcp_handle_connection(conn_socket, addr));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or_else(|| "[::]:35353".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    let tcp_socket = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    tcp_server(tcp_socket).await?;
    Ok(())
}
