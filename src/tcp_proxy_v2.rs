use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr, Ipv4Addr};
use std::thread;
use tracing::{error, info, warn};

const SIGNATURE: [u8; 12] = [0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A, ];

fn build_proxy_header(src_ip: Ipv4Addr, src_port: u16, dst_ip: Ipv4Addr, dst_port: u16, ) -> Vec<u8> {
    let mut header: Vec<u8> = Vec::new();
    for byte in SIGNATURE { header.push(byte); }
    header.push(0x21);
    header.push(0x11);

    let length_bytes = 12u16.to_be_bytes();
    header.push(length_bytes[0]);
    header.push(length_bytes[1]);

    let src_octets = src_ip.octets();
    header.push(src_octets[0]);
    header.push(src_octets[1]);
    header.push(src_octets[2]);
    header.push(src_octets[3]);

    let dst_octets = dst_ip.octets();
    header.push(dst_octets[0]);
    header.push(dst_octets[1]);
    header.push(dst_octets[2]);
    header.push(dst_octets[3]);

    let src_port_bytes = src_port.to_be_bytes();
    header.push(src_port_bytes[0]);
    header.push(src_port_bytes[1]);

    let dst_port_bytes = dst_port.to_be_bytes();
    header.push(dst_port_bytes[0]);
    header.push(dst_port_bytes[1]);

    return header;
}

fn pipe(mut from: TcpStream, mut to: TcpStream, direction: &'static str) {
    let mut buffer = [0u8; 1024];
    loop {
        let bytes_read = match from.read(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(error) => {
                warn!(%direction, %error, "tcpv2 read failed");
                break;
            }
        };
        if bytes_read == 0 {
            info!(%direction, "tcpv2 stream closed");
            break;
        }
        let data = &buffer[..bytes_read];
        if let Err(error) = to.write_all(data) {
            warn!(%direction, %error, bytes_read, "tcpv2 write failed");
            break;
        }
    }
}

fn handle_client(client_stream: TcpStream, target: &str) {
    let client_addr = match client_stream.peer_addr() {
        Ok(client_addr) => client_addr,
        Err(error) => {
            error!(target, %error, "failed to read tcpv2 client address");
            return;
        }
    };
    info!(%client_addr, target, "accepted tcpv2 connection");

    let mut server_stream = match TcpStream::connect(target) {
        Ok(server_stream) => server_stream,
        Err(error) => {
            error!(%client_addr, target, %error, "failed to connect to tcpv2 target");
            return;
        }
    };
    let client_ip: Ipv4Addr;
    let client_port: u16;
    match client_addr {
        SocketAddr::V4(addr) => {
            client_ip = *addr.ip();
            client_port = addr.port();
        }
        _ => {
            warn!(%client_addr, "skipping non-ipv4 client for tcpv2");
            return;
        }
    }
    let proxy_ip = Ipv4Addr::new(0, 0, 0, 0);
    let proxy_port: u16 = 25566;
    let header = build_proxy_header(client_ip, client_port, proxy_ip, proxy_port);
    if let Err(error) = server_stream.write_all(&header) {
        error!(%client_addr, target, %error, "failed to write proxy protocol header");
        return;
    }
    let client_read = client_stream.try_clone().unwrap();
    let client_write = client_stream.try_clone().unwrap();
    let server_read = server_stream.try_clone().unwrap();
    let server_write = server_stream.try_clone().unwrap();
    let thread1 = thread::spawn(move || { pipe(client_read, server_write, "client->target"); });
    let thread2 = thread::spawn(move || { pipe(server_read, client_write, "target->client"); });
    thread1.join().unwrap_or(());
    thread2.join().unwrap_or(());
    info!(%client_addr, target, "tcpv2 session finished");
}

pub fn run(bind: &str, target: &str) {
    info!(bind, target, "starting tcpv2 proxy listener");
    let listener = TcpListener::bind(bind).unwrap();
    for incoming in listener.incoming() {
        let client_stream = match incoming {
            Ok(client_stream) => client_stream,
            Err(error) => {
                error!(bind, target, %error, "failed to accept tcpv2 connection");
                continue;
            }
        };
        let target = target.to_string();
        thread::spawn(move || { handle_client(client_stream, &target); });
    }
}
