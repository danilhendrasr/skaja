use core::panic;
use mio::{net::TcpStream, Events, Interest, Poll};
use skaja_lib::{Command, OutOf, RawResponse, Request, Response, CLIENT_TOKEN};
use std::{
    io::{self, Write},
    net::SocketAddr,
    time::Duration,
};

pub struct Client {
    connection: TcpStream,
    poller: Poll,
}

impl Client {
    /// Connect to the given address. Panics if the connection fails.
    pub fn connect(address: SocketAddr) -> Self {
        let mut connection = match TcpStream::connect(address) {
            Ok(stream) => {
                println!("Connected to server at {}", address);
                stream
            }
            Err(msg) => panic!("Failed connecting to server: {}", msg),
        };

        let poller = Poll::new().unwrap();

        poller
            .registry()
            .register(&mut connection, CLIENT_TOKEN, Interest::WRITABLE)
            .unwrap();

        Self { connection, poller }
    }

    pub fn shutdown(&mut self) -> Result<(), io::Error> {
        self.connection.shutdown(std::net::Shutdown::Both)
    }

    pub fn send(&mut self, mut command: Command) -> Result<Response, io::Error> {
        let request = Request::outof(&mut command)?;

        let mut events = Events::with_capacity(1);

        loop {
            self.poller.poll(&mut events, Some(Duration::new(30, 0)))?;

            for event in events.iter() {
                if event.is_writable() {
                    self.connection.write_all(request.payload())?;
                    self.connection.flush()?;

                    self.poller.registry().reregister(
                        &mut self.connection,
                        CLIENT_TOKEN,
                        Interest::READABLE,
                    )?;
                }

                if event.is_readable() {
                    let response: Response = RawResponse::outof(&mut self.connection)?.into();
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

impl Drop for Client {
    fn drop(&mut self) {
        match self.shutdown() {
            Ok(_) => println!("Successfully shuts down client."),
            Err(msg) => eprintln!("Failed shutting down client: {}", msg),
        }
    }
}
