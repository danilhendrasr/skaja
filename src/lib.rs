mod constants;
mod types;
pub mod utils;

use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

use polling::{Event, Poller};
pub use types::*;

pub struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn new(address: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(address).map_err(|_| "Failed connecting to server.")?;
        Ok(Self { stream })
    }

    pub fn send(&mut self, command: Command) -> Result<(), String> {
        let client_payload = ClientPayload::from(command);

        self.stream
            .write(&client_payload.payload())
            .map_err(|_| "Failed writing to stream.")?;

        Ok(())
    }
}

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn new(address: &str) -> Result<Self, String> {
        let stream = TcpListener::bind(address).map_err(|_| "Failed connecting to server.")?;
        Ok(Self { listener: stream })
    }

    pub fn listen(&self) {
        self.listener
            .set_nonblocking(true)
            .expect("Failed setting listener as non-blocking.");

        let poller = Poller::new().expect("Failed creating new poll");
        let _ = poller.add(&self.listener, Event::readable(7));

        let mut events = Vec::new();
        loop {
            events.clear();
            poller
                .wait(&mut events, None)
                .expect("Dunno man, it failed.");

            for ev in &events {
                if ev.key == 7 {
                    let (stream, _) = self.listener.accept().expect("Failed accepting connection");
                    _ = poller.modify(&self.listener, Event::readable(7));

                    match self.handle_connection(stream) {
                        Err(msg) => eprintln!("Failed handling connection: {}", msg),
                        _ => println!("Succeeded handling connection."),
                    }
                }
            }
        }
    }

    fn handle_connection(&self, stream: TcpStream) -> Result<(), String> {
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
}
