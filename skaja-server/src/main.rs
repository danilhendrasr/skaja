use std::io;

pub use skaja_server::Server;

fn main() -> Result<(), io::Error> {
    Server::new("127.0.0.1:3000".parse().unwrap())
        .listen()
        .expect("Failed to start server");
    Ok(())
}
