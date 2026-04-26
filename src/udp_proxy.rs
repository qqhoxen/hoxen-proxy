use std::net::UdpSocket;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{error, info, warn};

pub fn run(bind: &str, target: &str, timeout_seconds: Option<u64>) {
    info!(bind, target, ?timeout_seconds, "starting udp proxy listener");
    let proxy_socket = UdpSocket::bind(bind).unwrap();
    let mut clients: HashMap<SocketAddr, (UdpSocket, Instant)> = HashMap::new();
    let mut reverse_map: HashMap<SocketAddr, SocketAddr> = HashMap::new();
    let mut buffer = [0u8; 4096];
    let timeout = std::time::Duration::from_millis(1);
    proxy_socket.set_read_timeout(Some(timeout)).unwrap();
    loop {
        let recv_result = proxy_socket.recv_from(&mut buffer);
        if let Ok((bytes_read, client_addr)) = recv_result {
            let data = &buffer[..bytes_read];
            let has_client = clients.contains_key(&client_addr);
            if !has_client {
                let client_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
                client_socket.set_read_timeout(Some(timeout)).unwrap();
                if let Err(error) = client_socket.connect(target) {
                    error!(%client_addr, target, %error, "failed to connect udp client socket to target");
                    continue;
                }
                let local_addr = client_socket.local_addr().unwrap();
                reverse_map.insert(local_addr, client_addr);
                clients.insert(client_addr, (client_socket, Instant::now()));
                info!(%client_addr, %local_addr, target, "registered udp client");
            }
            let entry = clients.get_mut(&client_addr).unwrap();
            let client_socket = &entry.0;
            entry.1 = Instant::now();
            match client_socket.send(data) {
                Ok(sent) => {
                    if sent != bytes_read {
                        warn!(%client_addr, target, sent, bytes_read, "partial udp send to target");
                    }
                }
                Err(error) => {
                    warn!(%client_addr, target, %error, "failed to forward udp packet to target");
                }
            }
        } else if let Err(error) = recv_result {
            if error.kind() != std::io::ErrorKind::WouldBlock && error.kind() != std::io::ErrorKind::TimedOut {
                warn!(bind, %error, "udp listener recv failed");
            }
        }

        let client_addrs: Vec<SocketAddr> = clients.keys().cloned().collect();
        for client_addr in client_addrs {
            let entry = clients.get(&client_addr);
            if entry.is_none() { continue; }
            let client_socket = &entry.unwrap().0;
            let recv_result = client_socket.recv(&mut buffer);
            if let Ok(bytes_read) = recv_result {
                let data = &buffer[..bytes_read];
                if let Err(error) = proxy_socket.send_to(data, client_addr) {
                    warn!(%client_addr, %error, "failed to send udp packet back to client");
                }
            } else if let Err(error) = recv_result {
                if error.kind() != std::io::ErrorKind::WouldBlock && error.kind() != std::io::ErrorKind::TimedOut {
                    warn!(%client_addr, %error, "udp target recv failed");
                }
            }
        }

        let mut to_remove: Vec<SocketAddr> = Vec::new();
        if let Some(timeout_seconds) = timeout_seconds {
            let now = Instant::now();
            for (addr, (_socket, last_time)) in clients.iter() {
                let elapsed = now.duration_since(*last_time);
                if elapsed.as_secs() > timeout_seconds {
                    to_remove.push(*addr);
                }
            }
        }

        for addr in to_remove {
            let entry = clients.get(&addr);
            if entry.is_some() {
                let local_addr = entry.unwrap().0.local_addr().unwrap();
                reverse_map.remove(&local_addr);
                info!(client_addr = %addr, %local_addr, "removing inactive udp client");
            }
            clients.remove(&addr);
        }
    }
}
