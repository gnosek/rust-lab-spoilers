use dns::build::Serialize;
use dns::parse::{parse_dns_packet, parse_dns_tcp_packet};
use dns::utils::respond;
use futures::{lazy, try_ready};
use std::net::SocketAddr;
use std::{env, io};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::prelude::*;

struct UdpServer {
    socket: UdpSocket,
    buf: Vec<u8>,
    to_send: Option<(Vec<u8>, SocketAddr)>,
}

impl Future for UdpServer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        loop {
            if let Some((ref packet, ref peer)) = self.to_send {
                let nwritten = try_ready!(self.socket.poll_send_to(&packet, &peer));
                if nwritten != packet.len() {
                    eprintln!(
                        "Failed to send whole response, sent {}/{} bytes",
                        nwritten,
                        packet.len()
                    );
                } else {
                    println!("Sent response to {}", peer);
                }
                self.to_send = None
            }

            let (nread, addr) = try_ready!(self.socket.poll_recv_from(&mut self.buf));
            match parse_dns_packet(&self.buf[0..nread]) {
                Ok((_, packet)) => {
                    let mut out_buf = Vec::new();
                    let resp = respond(packet, &addr);
                    resp.serialize_to(&mut out_buf)?;
                    self.to_send = Some((out_buf, addr));
                }
                Err(e) => {
                    eprintln!("Malformed DNS query from {:?}: {:?}", addr, e);
                }
            }
        }
    }
}

struct TcpServer {
    socket: TcpStream,
    buf: Vec<u8>,
    to_send: Option<(usize, Vec<u8>)>,
}

impl Future for TcpServer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        loop {
            if let Some((ref mut offset, ref mut buf)) = self.to_send {
                let nwritten = try_ready!(self.socket.poll_write(&buf[*offset..]));
                *offset += nwritten;
                let remaining = buf.len() - *offset;
                if remaining == 0 {
                    self.to_send = None;
                } else {
                    *offset += nwritten;
                }
            }

            if self.to_send.is_some() {
                continue;
            }

            let nread = try_ready!(self.socket.poll_read(&mut self.buf));
            if nread == 0 {
                return Ok(Async::Ready(()));
            }

            let addr = self.socket.peer_addr()?;
            match parse_dns_tcp_packet(&self.buf[0..nread]) {
                Ok((_, packet)) => {
                    let mut out_buf = Vec::new();
                    let resp = respond(packet, &addr);
                    resp.serialize_tcp_to(&mut out_buf)?;
                    self.to_send = Some((0, out_buf));
                }
                Err(e) => {
                    eprintln!("Malformed DNS query from {:?}: {:?}", addr, e);
                    return Ok(Async::Ready(()));
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or("[::]:35353".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    let udp_socket = UdpSocket::bind(&addr)?;
    let tcp_socket = TcpListener::bind(&addr)?;
    println!("Listening on: {}", addr);

    let udp_server = UdpServer {
        socket: udp_socket,
        buf: vec![0; 1024],
        to_send: None,
    }
    .map_err(|e| println!("Failed to handle UDP packet: {:?}", e));

    let tcp_server = tcp_socket
        .incoming()
        .map_err(|e| println!("Failed to accept connection: {:?}", e))
        .for_each(move |socket| {
            let server = TcpServer {
                socket,
                buf: vec![0; 1024],
                to_send: None,
            };

            tokio::spawn(server.map_err(|e| println!("Failed to handle connection: {:?}", e)));
            Ok(())
        });

    tokio::run(lazy(|| {
        tokio::spawn(udp_server);
        tokio::spawn(tcp_server);
        Ok(())
    }));

    Ok(())
}
