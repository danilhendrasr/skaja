pub use skaja_client::Client;
use skaja_lib::Command;

fn main() -> Result<(), String> {
    let mut client = Client::connect("127.0.0.1:8000".parse().unwrap());
    let mut client2 = Client::connect("127.0.0.1:8000".parse().unwrap());
    let mut client3 = Client::connect("127.0.0.1:8000".parse().unwrap());

    match client.send(Command::Set("hello".to_string(), "world".to_string())) {
        Ok(response) => println!("Client 1 sent set command: {}", response),
        Err(msg) => eprintln!("Failed sending command: {}", msg),
    }

    match client2.send(Command::Get("hello".to_string())) {
        Ok(response) => println!("Client 2 received: {}", response),
        Err(msg) => eprintln!("Failed sending command: {}", msg),
    }

    match client3.send(Command::Get("hello".to_string())) {
        Ok(response) => println!("Client 3 received: {}", response),
        Err(msg) => eprintln!("Failed sending command: {}", msg),
    }

    Ok(())
}
