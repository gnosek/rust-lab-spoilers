use dns::build::Serialize;
use dns::parse::parse_dns_packet;
use dns::utils::respond;
use std::net::UdpSocket;

fn main() -> Result<(), std::io::Error> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or(String::from("[::]:35353"));

    let socket = UdpSocket::bind(&addr)?;

    let mut in_buf = vec![0u8; 1024];
    loop {
        let (msgsize, src) = socket.recv_from(&mut in_buf)?;
        let msg = &in_buf[0..msgsize];

        match parse_dns_packet(msg) {
            Ok((_, packet)) => {
                let mut out_buf = Vec::new();
                let resp = respond(packet, &src);
                resp.serialize_to(&mut out_buf)?;
                socket.send_to(&out_buf, src)?;
            }
            Err(e) => {
                eprintln!("Malformed DNS query from {:?}: {:?}", src, e);
            }
        }
    }
}
