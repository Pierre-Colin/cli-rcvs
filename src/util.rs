use std::{error::Error, io::prelude::*, net::TcpStream};

pub fn read_packet(stream: &mut TcpStream, capacity: usize) -> Result<String, Box<dyn Error>> {
    let mut buffer = vec![0u8; capacity];
    let n = stream.read(&mut buffer)?;
    let mut r = std::str::from_utf8(&buffer)?.to_string();
    r.truncate(n);
    Ok(r)
}
