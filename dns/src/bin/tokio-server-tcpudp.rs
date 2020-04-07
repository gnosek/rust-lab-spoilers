use dns::build::Serialize;
use dns::parse::parse_dns_packet;
use dns::utils::respond;
use futures::future;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::net::SocketAddr;
use std::{env, io};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_util::codec::LengthDelimitedCodec;

async fn udp_server(mut socket: UdpSocket) -> Result<(), io::Error> {
    let mut buf = vec![0u8; 1024];
    loop {
        let (nread, addr) = socket.recv_from(&mut buf).await?;

        let packet = match parse_dns_packet(&buf[0..nread]) {
            Ok((_, packet)) => packet,
            Err(e) => {
                eprintln!("Malformed DNS query from {:?}: {:?}", addr, e);
                continue;
            }
        };

        let mut out_buf = Vec::new();
        let resp = respond(packet, &addr);
        resp.serialize_to(&mut out_buf)?;
        match socket.send_to(&out_buf, addr).await {
            Ok(nsent) if nsent == out_buf.len() => {
                println!("Sent response to {}", addr);
            }
            Ok(nsent) => {
                eprintln!(
                    "Failed to send whole response, sent {}/{} bytes",
                    nsent,
                    out_buf.len()
                );
            }
            Err(e) => {
                eprintln!("Failed to send response to {:?}: {:?}", addr, e);
            }
        }
    }
}

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

    let udp_socket = UdpSocket::bind(&addr).await?;
    let tcp_socket = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    let udp = udp_server(udp_socket);
    let tcp = tcp_server(tcp_socket);

    let server = future::join(udp, tcp);
    let (udp_res, tcp_res) = server.await;
    udp_res?;
    tcp_res?;

    Ok(())
}
