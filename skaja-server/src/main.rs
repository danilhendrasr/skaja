use clap::Parser;
use skaja_server::config::Config;
pub use skaja_server::Server;
use std::{fs, io, net::SocketAddr, path::PathBuf};
use tracing::{error, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(clap::Parser)]
pub struct Args {
    // The address to bind to, defaults to 127.0.0.1:3000.
    #[arg(short, long)]
    address: Option<String>,

    // The absolute path to the config file. Only supports TOML.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

fn main() -> Result<(), io::Error> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed setting tracing default subscriber.");

    let args = Args::parse();

    let mut server = Server::default();
    let mut address = "127.0.0.1:8080".to_owned();

    if let Some(config_path) = args.config {
        let config_file = fs::read_to_string(config_path).unwrap_or_else(|_| {
            error!(
                "Failed reading config file at {}, make sure the file exists.",
                address
            );
            std::process::exit(-1);
        });

        let config: Config = toml::from_str(&config_file).unwrap_or_else(|_| {
            error!("Failed parsing config file, check if it's valid.");
            std::process::exit(-1);
        });

        if let Some(addr) = config.address {
            address = addr;
        }
    }

    if let Some(addr) = args.address {
        address = addr;
    }

    let address: SocketAddr = address.parse().unwrap_or_else(|e| {
        let mut message = "Failed parsing target address".to_string();
        if address.contains("localhost") {
            message = format!(r#"{}, use "127.0.0.1" instead of "localhost"."#, message);
        } else {
            message = format!(r#"{}, {}."#, message, e);
        }

        error!(message);
        std::process::exit(-1);
    });

    server.set_address(address);
    server.bind().listen().expect("Server shuts down.");

    Ok(())
}
