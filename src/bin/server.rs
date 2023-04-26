use std::net::{TcpListener, TcpStream};

use polling::{Event, Poller};
use skaja::{ClientPayload, Command};

fn main() -> Result<(), ()> {
    let socket_addr = "127.0.0.1:3000";
    let socket =
        TcpListener::bind(socket_addr).expect(&format!("Could not bind server to {}", socket_addr));

    socket
        .set_nonblocking(true)
        .expect("Failed setting listener as non-blocking.");

    let poller = Poller::new().expect("Failed creating new poll");
    let _ = poller.add(&socket, Event::readable(7));

    let mut events = Vec::new();
    loop {
        events.clear();
        poller
            .wait(&mut events, None)
            .expect("Dunno man, it failed.");

        for ev in &events {
            if ev.key == 7 {
                let (stream, _) = socket.accept().expect("Failed accepting connection");
                _ = poller.modify(&socket, Event::readable(7));

                match handle_connection(stream) {
                    Err(msg) => eprintln!("Failed handling connection: {}", msg),
                    _ => println!("Succeeded handling connection."),
                }
            }
        }
    }
}

fn handle_connection(stream: TcpStream) -> Result<(), String> {
    // Create a buffer from the stream
    let buffer = match ClientPayload::try_from(stream) {
        Ok(buffer) => buffer,
        Err(_) => {
            return Err("failed creating buffer, skipping connection.".to_string());
        }
    };

    let command =
        Command::try_from(buffer).map_err(|err_msg| format!("Invalid payload: {}", err_msg))?;
    println!("{:?}", command);

    Ok(())
}
