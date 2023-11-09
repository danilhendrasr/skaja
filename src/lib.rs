mod constants;
mod types;
pub mod utils;

use std::{io::Write, net::TcpStream};

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
