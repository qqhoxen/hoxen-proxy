use std::net::UdpSocket;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::time::Instant;

pub fn run(bind: &str, target: &str) {
    let proxy_socket = UdpSocket::bind(bind).unwrap();
    let mut clients: HashMap<SocketAddr, (UdpSocket, Instant)> = HashMap::new();
    let mut reverse_map: HashMap<SocketAddr, SocketAddr> = HashMap::new();
    let mut buffer = [0u8; 4096];
    let timeout = std::time::Duration::from_millis(1);
    proxy_socket.set_read_timeout(Some(timeout)).unwrap();
    loop {
        let recv_result = proxy_socket.recv_from(&mut buffer);
        if recv_result.is_ok() {
            let (bytes_read, client_addr) = recv_result.unwrap();
            let data = &buffer[..bytes_read];
            let has_client = clients.contains_key(&client_addr);
            if !has_client {
                let client_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
                client_socket.set_read_timeout(Some(timeout)).unwrap();
                client_socket.connect(target).unwrap();
                let local_addr = client_socket.local_addr().unwrap();
                reverse_map.insert(local_addr, client_addr);
                clients.insert(client_addr, (client_socket, Instant::now()));
            }
            let entry = clients.get_mut(&client_addr).unwrap();
            let client_socket = &entry.0;
            entry.1 = Instant::now();
            client_socket.send(data).unwrap_or(0);
        }

        let client_addrs: Vec<SocketAddr> = clients.keys().cloned().collect();
        for client_addr in client_addrs {
            let entry = clients.get(&client_addr);
            if entry.is_none() { continue; }
            let client_socket = &entry.unwrap().0;
            let recv_result = client_socket.recv(&mut buffer);
            if recv_result.is_ok() {
                let bytes_read = recv_result.unwrap();
                let data = &buffer[..bytes_read];
                proxy_socket.send_to(data, client_addr).unwrap_or(0);
            }
        }

        let timeout_seconds = 30;
        let now = Instant::now();
        let mut to_remove: Vec<SocketAddr> = Vec::new();
        for (addr, (_socket, last_time)) in clients.iter() {
            let elapsed = now.duration_since(*last_time);
            if elapsed.as_secs() > timeout_seconds {
                to_remove.push(*addr);
            }
        }

        for addr in to_remove {
            let entry = clients.get(&addr);
            if entry.is_some() {
                let local_addr = entry.unwrap().0.local_addr().unwrap();
                reverse_map.remove(&local_addr);
            }
            clients.remove(&addr);
        }
    }
}