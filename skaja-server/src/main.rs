use std::io;

use clap::Parser;
pub use skaja_server::Server;

#[derive(clap::Parser)]
pub struct Args {
    /// The address to bind to, defaults to 127.0.0.1:3000.
    #[arg(short, long)]
    address: Option<String>,
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();
    let address = args.address.unwrap_or("127.0.0.1:3000".to_string());
    Server::new(address.parse().unwrap()).listen().unwrap();
    Ok(())
}
