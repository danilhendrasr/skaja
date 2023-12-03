use std::io::{self, Write};

use clap::Parser;
pub use skaja_client::Client;
use skaja_lib::Command;

#[derive(clap::Parser)]
pub struct Args {
    /// The address to bind to, defaults to 127.0.0.1:3000.
    #[arg(long)]
    host: String,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let mut client = Client::connect(args.host.parse().unwrap());

    loop {
        print!("skaja > ");
        // Stdout is buffered, therefore the print above won't be printed until
        // the code below is done reading from stdin. Flushing stdout will
        // force it to be printed immediately.
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_string();

        let command = match Command::try_from(input.clone()) {
            Ok(command) => command,
            Err(_) => {
                println!("Unrecognized command: {}.", input);
                continue;
            }
        };

        match client.send(command) {
            Ok(response) => println!("{}", response),
            Err(error) => println!("Error: {}.", error),
        }
    }
}
