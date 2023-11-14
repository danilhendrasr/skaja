mod constants;
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

pub use constants::*;
pub use types::*;

pub struct Client {
    connection: TcpStream,
}

impl Client {
    pub fn connect(address: SocketAddr) -> Result<Self, String> {
        let stream = TcpStream::connect(address).map_err(|_| "Failed connecting to server.")?;
        Ok(Self { connection: stream })
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

pub struct Server {
    listener: TcpListener,
    poller: Poll,
    data_store: HashMap<String, String>,
    connections_store: HashMap<Token, (TcpStream, Option<Command>)>,
}

impl Server {
    pub fn new(address: SocketAddr) -> Result<Self, io::Error> {
        let mut listener_binding = TcpListener::bind(address)?;
        let poller = Poll::new()?;

        poller
            .registry()
            .register(&mut listener_binding, SERVER_TOKEN, Interest::READABLE)?;

        Ok(Self {
            listener: listener_binding,
            data_store: HashMap::new(),
            poller,
            connections_store: HashMap::new(),
        })
    }

    pub fn listen(&mut self) -> Result<(), io::Error> {
        let mut events_store = Events::with_capacity(1024);
        // Unique token for each connection
        let unique_token = Token(SERVER_TOKEN.0 + 1);

        loop {
            if let Err(e) = self.poller.poll(&mut events_store, None) {
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }

                return Err(e);
            }

            for event in events_store.iter() {
                match event.token() {
                    Token(0) => {
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

                        let connection_token = Token(unique_token.0 + self.connections_store.len());
                        self.poller.registry().register(
                            &mut connection,
                            connection_token,
                            Interest::READABLE,
                        )?;

                        self.connections_store
                            .insert(connection_token, (connection, None));
                    }
                    token => {
                        let done = match self.handle_connection_event(event) {
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => return Err(e),
                            Ok(result) => result,
                        };

                        if done {
                            if let Some((mut conn, _)) = self.connections_store.remove(&token) {
                                self.poller.registry().deregister(&mut conn)?;
                                println!("Closed connection: {}", conn.peer_addr().unwrap());
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_connection_event(&mut self, event: &Event) -> Result<bool, io::Error> {
        let (connection, payload) = self
            .connections_store
            .get_mut(&event.token())
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get connection."))?;

        if event.is_writable() {
            let response: RawResponse;
            match payload.as_ref().unwrap() {
                Command::Get(key) => match self.data_store.get(key) {
                    Some(value) => response = RawResponse::new(StatusCodes::Ok, value.to_string()),
                    None => {
                        println!("Key \"{key}\" not found.");
                        response = RawResponse::new(
                            StatusCodes::ClientErr,
                            format!("Key \"{key}\" not found."),
                        )
                    }
                },
                Command::Set(key, value) => {
                    self.data_store.insert(key.to_string(), value.to_string());
                    response = RawResponse::new(
                        StatusCodes::Ok,
                        format!("Key \"{key}\" set to \"{value}\"."),
                    );
                }
                Command::Delete(key) => {
                    if self.data_store.remove(key).is_none() {
                        response = RawResponse::new(
                            StatusCodes::ClientErr,
                            format!("Key \"{key}\" not found."),
                        );
                    } else {
                        response =
                            RawResponse::new(StatusCodes::Ok, format!("Key \"{key}\" deleted."));
                    }
                }
            }

            let payload: Vec<u8> = response.into();
            connection.write_all(&payload)?;

            return Ok(true);
        }

        if event.is_readable() {
            let request = connection.as_request()?;

            let command: Command = match request.try_into() {
                Ok(command) => command,
                Err(err_msg) => {
                    println!("Invalid payload: {}", err_msg);
                    return Ok(true);
                }
            };

            self.poller
                .registry()
                .reregister(connection, event.token(), Interest::WRITABLE)?;

            self.connections_store
                .entry(event.token())
                .and_modify(|val| {
                    val.1 = Some(command);
                });
        }

        Ok(false)
    }
}
