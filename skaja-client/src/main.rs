pub use skaja_client::Client;
use skaja_lib::Command;
use std::env;

fn main() -> Result<(), String> {
    let mut client = Client::connect("127.0.0.1:3002".parse().unwrap());
    let command = Command::try_from(env::args())?;

    match client.send(command) {
        Ok(response) => println!("{}", response),
        Err(msg) => eprintln!("Failed sending command: {}", msg),
    }

    match client.send(Command::Set("hello".to_string(), "world".to_string())) {
        Ok(response) => println!("{}", response),
        Err(msg) => eprintln!("Failed sending command: {}", msg),
    }

    match client.send(Command::Get("hello".to_string())) {
        Ok(response) => println!("{}", response),
        Err(msg) => eprintln!("Failed sending command: {}", msg),
    }

    Ok(())
}
