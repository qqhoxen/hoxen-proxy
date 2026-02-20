use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

fn write_varint(value: u16) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    let mut val = value as u16;
    loop {
        let mut byte = (val & 0x7F) as u8;
        val = val >> 7;
        if val != 0 { byte = byte | 0x80; }
        result.push(byte);
        if val == 0 { break; }
    }
    result
}

fn read_varint(stream: &mut TcpStream) -> Option<u16> {
    let mut result: u16 = 0;
    let mut num_read: u32 = 0;
    loop {
        let mut byte_buf = [0u8; 1];

        let read_result = stream.read(&mut byte_buf);
        if read_result.is_err() { return None; }

        let bytes = read_result.unwrap();
        if bytes == 0 { return None; }
        let byte = byte_buf[0];

        let value = (byte & 0x7F) as u16;
        result = result | (value << (7 * num_read));

        num_read = num_read + 1;
        if byte & 0x80 == 0 { break; }
        if num_read > 5 { return None; }
    }
    Some(result)
}

fn build_status_response(fake: String) -> Vec<u8> {
    let mut packet: Vec<u8> = Vec::new();

    // let fake = r#"{"description":"A Minecraft Server","players":{"max":20,"online":0},"version":{"name":"Hoxen v1","protocol":774}}"#;
    let data = fake.as_bytes();

    let packet_id = write_varint(0x00);
    let string_length = write_varint(data.len() as u16);
    let data_length = packet_id.len() + string_length.len() + data.len();
    let packet_length = write_varint(data_length as u16);

    for byte in &packet_length { packet.push(*byte); }
    for byte in &packet_id { packet.push(*byte); }
    for byte in &string_length { packet.push(*byte); }
    for byte in data { packet.push(*byte); }

    packet
}

fn build_pong_response(payload: i64) -> Vec<u8> {
    let mut packet: Vec<u8> = Vec::new();

    let packet_id = write_varint(0x01);
    let payload_bytes = payload.to_be_bytes();
    let data_length = packet_id.len() + 8;
    let packet_length = write_varint(data_length as u16);

    for byte in &packet_length { packet.push(*byte); }
    for byte in &packet_id { packet.push(*byte); }
    for byte in &payload_bytes { packet.push(*byte); }

    packet
}

fn handle_client(mut client_stream: TcpStream, custom_fake: Option<String>) {
    let client_addr = client_stream.peer_addr().unwrap();

    let packet_length = read_varint(&mut client_stream);
    if packet_length.is_none() { return; }

    let mut handshake_data = vec![0u8; packet_length.unwrap() as usize];
    if client_stream.read_exact(&mut handshake_data).is_err() { return; }

    let packet_length = read_varint(&mut client_stream);
    if packet_length.is_none() { return; }

    let mut request_data = vec![0u8; packet_length.unwrap() as usize];
    if client_stream.read_exact(&mut request_data).is_err() { return; }

    let fake = custom_fake.unwrap_or_else(|| r#"{"description":"A Minecraft Server","players":{"max":20,"online":0},"version":{"name":"Hoxen v1","protocol":774}}"#.to_string());
    let response = build_status_response(fake);
    if client_stream.write_all(&response).is_err() { return; }

    let ping_length = read_varint(&mut client_stream);
    if ping_length.is_none() { return; }

    let mut ping_data = vec![0u8; ping_length.unwrap() as usize];
    if client_stream.read_exact(&mut ping_data).is_err() { return; }

    let mut original_timestamp_bytes = [0u8; 8];
    original_timestamp_bytes[0] = ping_data[1];
    original_timestamp_bytes[1] = ping_data[2];
    original_timestamp_bytes[2] = ping_data[3];
    original_timestamp_bytes[3] = ping_data[4];
    original_timestamp_bytes[4] = ping_data[5];
    original_timestamp_bytes[5] = ping_data[6];
    original_timestamp_bytes[6] = ping_data[7];
    original_timestamp_bytes[7] = ping_data[8];
    let original_timestamp = i64::from_be_bytes(original_timestamp_bytes);

    let pong = build_pong_response(original_timestamp);
    if client_stream.write_all(&pong).is_err() { return; }
}

pub fn run(bind: &str, data: Option<String>) {
    let listener = TcpListener::bind(bind).unwrap();
    for incoming in listener.incoming() {
        let client_stream = incoming.unwrap();
        let data_clone = data.clone();
        thread::spawn(move || { handle_client(client_stream, data_clone); });
    }
}
