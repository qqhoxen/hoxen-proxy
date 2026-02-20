use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn pipe(mut from: TcpStream, mut to: TcpStream) {
    let mut buffer = [0u8; 1024];
    loop {
        let bytes_read = from.read(&mut buffer).unwrap_or(0);
        if bytes_read == 0 { break; }
        let data = &buffer[..bytes_read];
        let write_result = to.write_all(data);
        if write_result.is_err() { break; }
    }
}

fn handle_client(client_stream: TcpStream, target: &str) {
    let server_stream = TcpStream::connect(target);
    if server_stream.is_err() { return; }
    let server_stream = server_stream.unwrap();
    let client_read = client_stream.try_clone().unwrap();
    let client_write = client_stream.try_clone().unwrap();
    let server_read = server_stream.try_clone().unwrap();
    let server_write = server_stream.try_clone().unwrap();
    let thread1 = thread::spawn(move || { pipe(client_read, server_write); });
    let thread2 = thread::spawn(move || { pipe(server_read, client_write); });
    thread1.join().unwrap_or(());
    thread2.join().unwrap_or(());
}

pub fn run(bind: &str, target: &str) {
    let listener = TcpListener::bind(bind).unwrap();
    for incoming in listener.incoming() {
        let client_stream = incoming.unwrap();
        let target = target.to_string();
        thread::spawn(move || { handle_client(client_stream, &target); });
    }
}