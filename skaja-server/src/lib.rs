use mio::{
    event::Event,
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};
use skaja_lib::{Command, RawResponse, ReadToRequest, StatusCodes, SERVER_TOKEN};
use std::{
    collections::HashMap,
    io::{self, Write},
    net::SocketAddr,
};
use tracing::{debug, error, info};

pub struct Connection {
    connection: TcpStream,
    ip: SocketAddr,
    payload: Option<Command>,
}

pub struct Server {
    address: SocketAddr,
    listener: TcpListener,
    poller: Poll,
    data_store: HashMap<String, String>,
    connections_store: HashMap<Token, Connection>,
}

impl Server {
    /// Create a new server instance bound to the given address.
    /// Panics if the binding fails.
    pub fn new(address: SocketAddr) -> Self {
        let mut listener_binding = match TcpListener::bind(address) {
            Ok(listener) => listener,
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                error!("Address already in use: {}", address);
                std::process::exit(-1);
            }
            Err(err) => {
                error!("Failed starting server: {}", err);
                std::process::exit(-1);
            }
        };

        debug!("Server bound to: {}", address);

        let poller = Poll::new().unwrap_or_else(|_| {
            error!("Failed creating poller.");
            std::process::exit(-1);
        });
        debug!("Successfully created poller.");

        poller
            .registry()
            .register(&mut listener_binding, SERVER_TOKEN, Interest::READABLE)
            .unwrap_or_else(|_| {
                error!("Failed registering server to poller.");
                std::process::exit(-1);
            });
        debug!("Successfully registered server to poller.");

        Self {
            address,
            listener: listener_binding,
            data_store: HashMap::new(),
            poller,
            connections_store: HashMap::new(),
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Start the server and listen for incoming connections.
    pub fn listen(&mut self) -> Result<(), io::Error> {
        info!("Server listening on: {}", self.address());
        let mut events_store = Events::with_capacity(1024);
        // Unique token for each connection
        let unique_token = Token(SERVER_TOKEN.0 + 1);

        loop {
            debug!("Polling for events...");
            if let Err(e) = self.poller.poll(&mut events_store, None) {
                if e.kind() == io::ErrorKind::Interrupted {
                    debug!("Polling interrupted.");
                    continue;
                }

                error!("An error occurred when polling events: {}", e);
                return Err(e);
            }

            for event in events_store.iter() {
                match event.token() {
                    SERVER_TOKEN => {
                        let (mut connection, address) = match self.listener.accept() {
                            Ok((connection, addr)) => (connection, addr),
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                // If we get a `WouldBlock` error we know our
                                // listener has no more incoming connections queued,
                                // so we can return to polling and wait for some
                                // more.
                                debug!("No more incoming connections, returning to polling.");
                                break;
                            }
                            Err(e) => {
                                error!("Failed accepting connection: {}", e);
                                return Err(e);
                            }
                        };

                        info!("Accepted connection from: {}", address);

                        let connection_token = Token(unique_token.0 + self.connections_store.len());
                        self.poller.registry().register(
                            &mut connection,
                            connection_token,
                            Interest::READABLE,
                        )?;

                        self.connections_store.insert(
                            connection_token,
                            Connection {
                                connection,
                                ip: address,
                                payload: None,
                            },
                        );
                    }
                    token => {
                        debug!("Handling connection event: {:?}", token);
                        let done = match self.handle_connection_event(event) {
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,

                            Ok(_) => false,
                            Err(e) if e.kind() == io::ErrorKind::Interrupted => false,
                            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => false,

                            // Connection is done only if we get the following errors.
                            // I don't know if this is the best way to handle this. But it works.
                            Err(e) if e.kind() == io::ErrorKind::ConnectionReset => true,
                            Err(e) if e.kind() == io::ErrorKind::ConnectionAborted => true,
                            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => true,

                            Err(e) => return Err(e),
                        };

                        if done {
                            if let Some(mut conn) = self.connections_store.remove(&token) {
                                info!("Connection closed: {}", conn.ip);
                                self.poller.registry().deregister(&mut conn.connection)?;
                                continue;
                            }

                            debug!("Unable to remove connection from store, returning to polling.");
                        }
                    }
                }
            }
        }
    }

    fn handle_connection_event(&mut self, event: &Event) -> Result<(), io::Error> {
        let Connection {
            connection,
            payload,
            ip: _,
        } = self
            .connections_store
            .get_mut(&event.token())
            .ok_or_else(|| {
                error!("Failed getting connection from store.");
                io::Error::new(io::ErrorKind::Other, "Failed to get connection.")
            })?;

        if event.is_writable() {
            debug!("Handling writable event.");
            let response: RawResponse;
            match payload.as_ref().unwrap() {
                Command::Get(key) => match self.data_store.get(key) {
                    Some(value) => response = RawResponse::new(StatusCodes::Ok, value.to_string()),
                    None => {
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
            connection.write_all(&payload).map_err(|e| {
                error!("Failed writing response: {}", e);
                e
            })?;
            connection.flush()?;
            debug!("Successfully wrote response.");

            self.poller
                .registry()
                .reregister(connection, event.token(), Interest::READABLE)?;
            debug!("Successfully reregistered connection.");

            return Ok(());
        }

        if event.is_readable() {
            debug!("Handling readable event.");
            let request = connection.read_to_request().map_err(|e| {
                error!("Failed parsing payload to Request: {:?}", e);
                e
            })?;

            let command: Command = match request.try_into() {
                Ok(command) => command,
                Err(err_msg) => {
                    error!(
                        "Payload is invalid, failed parsing Request to Command: {}",
                        err_msg
                    );

                    return Ok(());
                }
            };

            self.poller
                .registry()
                .reregister(connection, event.token(), Interest::WRITABLE)?;
            debug!("Successfully reregistered connection.");

            self.connections_store
                .entry(event.token())
                .and_modify(|val| {
                    val.payload = Some(command);
                });
        }

        Ok(())
    }
}
