use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use tracing::{error, info, warn};

fn pipe(mut from: TcpStream, mut to: TcpStream, direction: &'static str) {
    let mut buffer = [0u8; 1024];
    loop {
        let bytes_read = match from.read(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(error) => {
                warn!(%direction, %error, "tcp read failed");
                break;
            }
        };
        if bytes_read == 0 {
            info!(%direction, "tcp stream closed");
            break;
        }
        let data = &buffer[..bytes_read];
        if let Err(error) = to.write_all(data) {
            warn!(%direction, %error, bytes_read, "tcp write failed");
            break;
        }
    }
}

fn handle_client(client_stream: TcpStream, target: &str) {
    let client_addr = client_stream.peer_addr().ok();
    info!(client_addr = ?client_addr, target, "accepted tcp connection");

    let server_stream = match TcpStream::connect(target) {
        Ok(server_stream) => server_stream,
        Err(error) => {
            error!(client_addr = ?client_addr, target, %error, "failed to connect to tcp target");
            return;
        }
    };
    let client_read = client_stream.try_clone().unwrap();
    let client_write = client_stream.try_clone().unwrap();
    let server_read = server_stream.try_clone().unwrap();
    let server_write = server_stream.try_clone().unwrap();
    let thread1 = thread::spawn(move || { pipe(client_read, server_write, "client->target"); });
    let thread2 = thread::spawn(move || { pipe(server_read, client_write, "target->client"); });
    thread1.join().unwrap_or(());
    thread2.join().unwrap_or(());
    info!(client_addr = ?client_addr, target, "tcp session finished");
}

pub fn run(bind: &str, target: &str) {
    info!(bind, target, "starting tcp proxy listener");
    let listener = TcpListener::bind(bind).unwrap();
    for incoming in listener.incoming() {
        let client_stream = match incoming {
            Ok(client_stream) => client_stream,
            Err(error) => {
                error!(bind, target, %error, "failed to accept tcp connection");
                continue;
            }
        };
        let target = target.to_string();
        thread::spawn(move || { handle_client(client_stream, &target); });
    }
}
