use std::io::Write;
use std::{env, net::TcpStream};

use skaja::{ClientPayload, Command};

fn main() -> Result<(), String> {
    let command = Command::try_from(env::args())?;

    let mut stream =
        TcpStream::connect("localhost:3000").map_err(|_| "Failed connecting to socket.")?;

    // Construct full payload from command and its arguments.
    let client_payload = ClientPayload::from(command);

    stream
        .write(&client_payload.payload())
        .map_err(|_| "Failed writing to socket.")?;

    Ok(())
}
