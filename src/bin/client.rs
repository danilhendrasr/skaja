use std::env;
use std::io::Write;
use std::os::unix::net::UnixStream;

use skaja::{ClientPayload, Command};

fn main() -> Result<(), String> {
    let command = Command::try_from(env::args())?;

    let mut stream = UnixStream::connect("mysocket").map_err(|_| "Failed connecting to socket.")?;

    // Construct full payload from command and its arguments.
    let client_payload = ClientPayload::from(command);

    stream
        .write(&client_payload.payload())
        .map_err(|_| "Failed writing to socket.")?;

    Ok(())
}
