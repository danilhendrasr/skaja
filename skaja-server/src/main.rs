use clap::Parser;
pub use skaja_server::Server;
use std::io;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(clap::Parser)]
pub struct Args {
    /// The address to bind to, defaults to 127.0.0.1:3000.
    #[arg(short, long)]
    address: Option<String>,
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();
    let address = args.address.unwrap_or("127.0.0.1:8000".to_string());

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed setting tracing default subscriber.");

    Server::new(address.parse().unwrap()).listen().unwrap();
    Ok(())
}
