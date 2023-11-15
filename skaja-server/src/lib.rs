use std::{
    collections::HashMap,
    io::{self, Write},
    net::SocketAddr,
};

use mio::{
    event::Event,
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};

use skaja_lib::{Command, RawResponse, ReadToRequest, StatusCodes, SERVER_TOKEN};

pub struct Server {
    listener: TcpListener,
    poller: Poll,
    data_store: HashMap<String, String>,
    connections_store: HashMap<Token, (TcpStream, Option<Command>)>,
}

impl Server {
    /// Create a new server instance bound to the given address.
    /// Panics if the binding fails.
    pub fn new(address: SocketAddr) -> Self {
        let mut listener_binding = TcpListener::bind(address).unwrap();
        let poller = Poll::new().unwrap();

        poller
            .registry()
            .register(&mut listener_binding, SERVER_TOKEN, Interest::READABLE)
            .unwrap();

        Self {
            listener: listener_binding,
            data_store: HashMap::new(),
            poller,
            connections_store: HashMap::new(),
        }
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
            let request = connection.read_to_request()?;

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
