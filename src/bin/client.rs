use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::exit;

const MAX_MESSAGE_SIZE: usize = 4096;

fn main() {
    let mut stream = UnixStream::connect("mysocket").expect("Failed connecting to socket.");

    let msg = "Hello?";
    let msg_len = msg.as_bytes().len() as u32;
    let mut header = msg_len.to_ne_bytes().to_vec();
    header.append(&mut msg.as_bytes().to_owned());

    stream.write(&header).expect("Failed writing to stream.");

    let mut resp_header = vec![0u8; 4];
    stream
        .read_exact(&mut resp_header)
        .expect("Failed reading from stream.");

    let msg_len = u32::from_ne_bytes(resp_header.try_into().unwrap());
    if msg_len as usize > MAX_MESSAGE_SIZE {
        println!("Message too long! Maximum is 4096 bytes.");
        exit(1);
    }

    let mut body = vec![0u8; msg_len as usize];
    stream
        .read_exact(&mut body)
        .expect("Failed reading from stream.");

    println!("Received: {}", String::from_utf8_lossy(&body));
}
