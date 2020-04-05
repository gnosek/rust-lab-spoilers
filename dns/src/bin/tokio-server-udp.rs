use dns::build::Serialize;
use dns::parse::parse_dns_packet;
use dns::utils::respond;
use std::net::SocketAddr;
use std::{env, io};
use tokio::net::UdpSocket;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or_else(|| "[::]:35353".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    let udp_socket = UdpSocket::bind(&addr).await?;
    println!("Listening on: {}", addr);

    udp_server(udp_socket).await?;
    Ok(())
}
