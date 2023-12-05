use mio::{
    event::Event,
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};
use skaja_lib::{Command, RawResponse, ReadToRequest, StatusCodes, SERVER_TOKEN};
use std::{
    collections::HashMap,
    default::Default,
    io::{self, Write},
    net::SocketAddr,
};
use tracing::{debug, error, info};

mod domains;
pub use domains::*;

pub struct Connection {
    connection: TcpStream,
    ip: SocketAddr,
    payload: Option<Command>,
}

pub struct Server {
    address: Option<SocketAddr>,
    listener: Option<TcpListener>,
    poller: Option<Poll>,
    data_store: HashMap<String, String>,
    connections_store: HashMap<Token, Connection>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    // Creates a fresh unbinded instance of the server.
    pub fn new() -> Self {
        Self {
            address: None,
            listener: None,
            poller: None,
            data_store: HashMap::new(),
            connections_store: HashMap::new(),
        }
    }

    // Creates a new instance of the server and binds it to the address.
    pub fn bootstrap(address: SocketAddr) -> Self {
        let mut server = Self::new();
        server.set_address(address);
        server.bind()
    }

    // Binds the server to the address already tied to the instance.
    pub fn bind(self) -> Self {
        if let Some(address) = self.address {
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

            poller
                .registry()
                .register(&mut listener_binding, SERVER_TOKEN, Interest::READABLE)
                .unwrap_or_else(|_| {
                    error!("Failed registering server to poller.");
                    std::process::exit(-1);
                });

            Self {
                address: Some(address),
                listener: Some(listener_binding),
                poller: Some(poller),
                data_store: self.data_store,
                connections_store: self.connections_store,
            }
        } else {
            error!("Server address is not set.");
            std::process::exit(-1);
        }
    }

    pub fn address(&self) -> Option<SocketAddr> {
        self.address
    }

    pub fn set_address(&mut self, address: SocketAddr) {
        self.address = Some(address);
    }

    // Listen for incoming connections.
    pub fn listen(mut self) -> Result<(), io::Error> {
        if self.address.is_none() {
            error!("Server hasn't been initialized.");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Server address is not set.",
            ));
        }

        info!("Server listening on: {}", self.address().unwrap());
        let mut events_store = Events::with_capacity(1024);
        // Unique token for each connection
        let unique_token = Token(SERVER_TOKEN.0 + 1);

        loop {
            debug!("Polling for events...");
            if let Err(e) = self.poller.as_mut().unwrap().poll(&mut events_store, None) {
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
                        debug!("Handling server event: {:?}", event);
                        let (mut connection, address) =
                            match self.listener.as_ref().unwrap().accept() {
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
                        self.poller.as_ref().unwrap().registry().register(
                            &mut connection,
                            connection_token,
                            Interest::READABLE,
                        )?;

                        // For some reason we need to reregister the listener
                        // because if not, when there are multiple clients connected
                        // there will be cases where one of the clients just randomly
                        // hangs and doesn't receive any response from the server.
                        // I don't know why this happens, but reregistering the
                        // listener fixes it.
                        self.poller.as_ref().unwrap().registry().reregister(
                            self.listener.as_mut().unwrap(),
                            SERVER_TOKEN,
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

                            // Connection is done only if we get the following errors.
                            // I don't know if this is the best way to handle this. But it works.
                            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => true,
                            Err(e) if e.kind() == io::ErrorKind::ConnectionReset => true,
                            Err(e) if e.kind() == io::ErrorKind::ConnectionAborted => true,
                            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => true,

                            Err(e) => return Err(e),
                        };

                        if done {
                            if let Some(mut conn) = self.connections_store.remove(&token) {
                                info!("Connection closed: {}", conn.ip);
                                self.poller
                                    .as_ref()
                                    .unwrap()
                                    .registry()
                                    .deregister(&mut conn.connection)?;
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
                    Some(value) => {
                        response = RawResponse::new(StatusCodes::Ok, Some(value.to_string()))
                    }
                    None => response = RawResponse::new(StatusCodes::ErrNotFound, None),
                },
                Command::Set(key, value) => {
                    self.data_store.insert(key.to_string(), value.to_string());
                    response = RawResponse::new(StatusCodes::Ok, None);
                }
                Command::Delete(key) => {
                    if self.data_store.remove(key).is_none() {
                        response = RawResponse::new(StatusCodes::ErrNotFound, None);
                    } else {
                        response = RawResponse::new(StatusCodes::Ok, None);
                    }
                }
            }

            let payload: Vec<u8> = response.into();
            connection.write_all(&payload).map_err(|e| {
                error!("Failed writing response: {}", e);
                e
            })?;
            connection.flush()?;

            self.poller.as_ref().unwrap().registry().reregister(
                connection,
                event.token(),
                Interest::READABLE,
            )?;

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

            self.poller.as_mut().unwrap().registry().reregister(
                connection,
                event.token(),
                Interest::WRITABLE,
            )?;

            self.connections_store
                .entry(event.token())
                .and_modify(|val| {
                    val.payload = Some(command);
                });
        }

        Ok(())
    }
}
