use std::env;

use skaja::{Client, Command};

fn main() -> Result<(), String> {
    let mut client = Client::new("localhost:3000")?;
    let command = Command::try_from(env::args())?;

    match client.send(command) {
        Ok(_) => println!("Succeeded sending command."),
        Err(msg) => eprintln!("Failed sending command: {}", msg),
    }

    Ok(())
}
