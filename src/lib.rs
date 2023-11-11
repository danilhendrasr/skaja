mod types;

use std::{
    collections::HashMap,
    io::{self, Write},
    net::SocketAddr,
    time::Duration,
};

use mio::{
    event::Event,
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};
pub use types::*;

pub struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn connect(address: SocketAddr) -> Result<Self, String> {
        let stream = TcpStream::connect(address).map_err(|_| "Failed connecting to server.")?;
        Ok(Self { stream })
    }

    pub fn send(&mut self, command: Command) -> Result<(), io::Error> {
        const CLIENT_TOKEN: Token = Token(0);

        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(1);

        poll.registry()
            .register(&mut self.stream, CLIENT_TOKEN, Interest::WRITABLE)?;

        loop {
            poll.poll(&mut events, Some(Duration::new(30, 0)))?;

            for event in events.iter() {
                if event.token() != CLIENT_TOKEN || !event.is_writable() {
                    continue;
                }

                let request: Request = command.into();
                self.stream.write(request.payload())?;
                return Ok(());
            }
        }
    }
}

pub struct Server {
    listener: TcpListener,
    store: HashMap<String, String>,
}

impl Server {
    pub fn new(address: SocketAddr) -> Result<Self, io::Error> {
        let stream = TcpListener::bind(address)?;
        Ok(Self {
            listener: stream,
            store: HashMap::new(),
        })
    }

    pub fn listen(&mut self) -> Result<(), io::Error> {
        const SERVER_TOKEN: Token = Token(0);

        let mut poller = Poll::new()?;
        let mut connections: HashMap<Token, TcpStream> = HashMap::new();

        poller
            .registry()
            .register(&mut self.listener, SERVER_TOKEN, Interest::READABLE)?;

        // Unique token for each connection
        let unique_token = Token(SERVER_TOKEN.0 + 1);

        let mut events = Events::with_capacity(1024);

        loop {
            if let Err(e) = poller.poll(&mut events, None) {
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }

                return Err(e);
            }

            for event in events.iter() {
                match event.token() {
                    SERVER_TOKEN => {
                        let (mut connection, address) = match self.listener.accept() {
                            Ok((connection, addr)) => (connection, addr),
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                // If we get a `WouldBlock` error we know our
                                // listener has no more incoming connections queued,
                                // so we can return to polling and wait for some
                                // more.
                                break;
                            }
                            Err(e) => return Err(e),
                        };

                        println!("Accepted connection from: {}", address);

                        let connection_token = Token(unique_token.0 + connections.len());
                        poller.registry().register(
                            &mut connection,
                            connection_token,
                            Interest::READABLE.add(Interest::WRITABLE),
                        )?;

                        connections.insert(connection_token, connection);
                    }
                    token => {
                        if let Some(connection) = connections.get_mut(&token) {
                            match self.handle_connection(connection, event) {
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                                Err(e) => return Err(e),
                                Ok(_) => {}
                            }
                        };
                    }
                }
            }
        }
    }

    fn handle_connection(
        &mut self,
        connection: &mut TcpStream,
        event: &Event,
    ) -> Result<bool, io::Error> {
        if event.is_readable() {
            let request = connection.to_request()?;

            let command: Command = match request.try_into() {
                Ok(command) => command,
                Err(err_msg) => {
                    println!("Invalid payload: {}", err_msg);
                    return Ok(true);
                }
            };

            match command {
                Command::Get(key) => match self.store.get(&key) {
                    Some(value) => println!("Value: {}", value),
                    None => println!("Key {key} not found."),
                },
                Command::Set(key, value) => {
                    self.store.insert(key, value);
                }
                Command::Delete(key) => {
                    if let None = self.store.remove(&key) {
                        println!("Key {key} not found.");
                    }
                }
            }
        }

        Ok(true)
    }
}
