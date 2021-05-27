use dns::build::Serialize;
use dns::parse::parse_dns_packet;
use dns::utils::respond;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::net::SocketAddr;
use std::{env, io};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::LengthDelimitedCodec;

async fn tcp_handle_connection(socket: TcpStream, addr: SocketAddr) -> Result<(), io::Error> {
    let mut framed = LengthDelimitedCodec::builder()
        .length_field_length(2)
        .new_framed(socket);

    while let Some(frame) = framed.next().await {
        let packet = match parse_dns_packet(&frame?) {
            Ok((_, packet)) => packet,
            Err(e) => {
                eprintln!("Malformed DNS query from {:?}: {:?}", addr, e);
                continue;
            }
        };

        let mut out_buf = Vec::new();
        let resp = respond(packet, &addr);
        resp.serialize_to(&mut out_buf)?;
        match framed.send(out_buf.into()).await {
            Ok(()) => {
                println!("Sent response to {}", addr);
            }
            Err(e) => {
                eprintln!("Failed to send response to {:?}: {:?}", addr, e);
            }
        }
    }

    Ok(())
}

async fn tcp_server(socket: TcpListener) -> Result<(), io::Error> {
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
