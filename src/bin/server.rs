#![feature(read_buf)]
use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    process::exit,
};

const MAX_MESSAGE_SIZE: usize = 4096;

fn main() -> Result<(), ()> {
    let socket_path = "mysocket";
    let listener = UnixListener::bind(socket_path).expect("Could not create unix socket");

    loop {
        let (unix_stream, _) = listener.accept().expect("Failed accepting connection");
        handle_stream(unix_stream)?
    }
}

fn handle_stream(mut stream: UnixStream) -> Result<(), ()> {
    let mut header = vec![0u8; 4];

    stream
        .read_exact(&mut header)
        .expect("Failed reading from stream.");

    let message_len = u32::from_ne_bytes(header.try_into().unwrap());

    if message_len as usize > MAX_MESSAGE_SIZE {
        println!("Message too long! Maximum is 4096 bytes.");
        exit(1);
    }

    let mut body = vec![0u8; message_len as usize];
    stream
        .read_exact(&mut body)
        .expect("Failed reading from stream.");

    println!("Received: {}", String::from_utf8_lossy(&body));
    println!("Replying...");

    let msg = "Hola!";
    let msg_len = msg.as_bytes().len() as u32;
    let mut resp_header = msg_len.to_ne_bytes().to_vec();
    resp_header.append(&mut msg.as_bytes().to_owned());

    stream
        .write(&resp_header)
        .expect("Failed writing to stream.");

    println!("Replied with: {}\n", msg);

    Ok(())
}
