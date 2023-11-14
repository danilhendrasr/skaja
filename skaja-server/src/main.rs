mod server;

use std::io;

use server::Server;

fn main() -> Result<(), io::Error> {
    Server::new("127.0.0.1:3000".parse().unwrap())?
        .listen()
        .expect("Failed to start server");
    Ok(())
}
