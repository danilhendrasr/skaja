use core::panic;
use std::{
    io::{self, Write},
    net::SocketAddr,
    time::Duration,
};

use mio::{net::TcpStream, Events, Interest, Poll};
use skaja_lib::{Command, ReadToRequest, ReadToResponse, Response, CLIENT_TOKEN};

pub struct Client {
    connection: TcpStream,
    poller: Poll,
}

impl Client {
    /// Connect to the given address. Panics if the connection fails.
    pub fn connect(address: SocketAddr) -> Self {
        let mut connection = match TcpStream::connect(address) {
            Ok(stream) => stream,
            Err(msg) => panic!("Failed connecting to server: {}", msg),
        };

        let poller = Poll::new().unwrap();

        poller
            .registry()
            .register(&mut connection, CLIENT_TOKEN, Interest::WRITABLE)
            .unwrap();

        Self { connection, poller }
    }

    pub fn send(&mut self, mut command: Command) -> Result<Response, io::Error> {
        let request = command.read_to_request()?;

        let mut events = Events::with_capacity(1);

        loop {
            self.poller.poll(&mut events, Some(Duration::new(30, 0)))?;

            for event in events.iter() {
                if event.is_writable() {
                    self.connection.write_all(request.payload())?;
                    self.poller.registry().reregister(
                        &mut self.connection,
                        CLIENT_TOKEN,
                        Interest::READABLE,
                    )?;
                }

                if event.is_readable() {
                    let response: Response = self.connection.read_to_response()?.into();
                    // self.poller.registry().deregister(&mut self.connection)?;
                    self.poller.registry().reregister(
                        &mut self.connection,
                        CLIENT_TOKEN,
                        Interest::WRITABLE,
                    )?;
                    return Ok(response);
                }
            }
        }
    }
}
