use std::env;

pub use skaja_client::Client;
use skaja_lib::Command;

fn main() -> Result<(), String> {
    let mut client = Client::connect("127.0.0.1:3000".parse().unwrap());
    let command = Command::try_from(env::args())?;

    match client.send(command) {
        Ok(_) => {}
        Err(msg) => eprintln!("Failed sending command: {}", msg),
    }

    Ok(())
}
