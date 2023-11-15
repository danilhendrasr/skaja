use core::panic;
use std::{
    io::{self, Write},
    net::SocketAddr,
    time::Duration,
};

use mio::{net::TcpStream, Events, Interest, Poll};
use skaja_lib::{AsRequest, AsResponse, Command, Response, CLIENT_TOKEN};

pub struct Client {
    connection: TcpStream,
}

impl Client {
    /// Connect to the given address. Panics if the connection fails.
    pub fn connect(address: SocketAddr) -> Self {
        let stream = match TcpStream::connect(address) {
            Ok(stream) => stream,
            Err(msg) => panic!("Failed connecting to server: {}", msg),
        };

        Self { connection: stream }
    }

    pub fn send(&mut self, mut command: Command) -> Result<(), io::Error> {
        let request = command.as_request()?;

        let mut poller = Poll::new()?;
        let mut events = Events::with_capacity(1);

        poller
            .registry()
            .register(&mut self.connection, CLIENT_TOKEN, Interest::WRITABLE)?;

        loop {
            poller.poll(&mut events, Some(Duration::new(30, 0)))?;

            for event in events.iter() {
                if event.is_writable() {
                    self.connection.write_all(request.payload())?;
                    poller.registry().reregister(
                        &mut self.connection,
                        CLIENT_TOKEN,
                        Interest::READABLE,
                    )?;
                }

                if event.is_readable() {
                    let response: Response = self.connection.as_response()?.into();
                    println!("Response:\n{}", response);
                    return Ok(());
                }
            }
        }
    }
}
