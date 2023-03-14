use std::{
    env::Args,
    io::{Error, Read},
    os::unix::net::UnixStream,
};

#[derive(Debug)]
pub enum Command {
    Get(String),
    Set(String, String),
    Delete(String),
}

impl Command {
    pub fn as_str(&self) -> &str {
        match self {
            Command::Get(_) => "get",
            Command::Set(_, _) => "set",
            Command::Delete(_) => "del",
        }
    }

    pub fn arguments(&self) -> Vec<&str> {
        match self {
            Command::Set(key, val) => vec![key, val],
            Command::Get(key) => vec![key],
            Command::Delete(key) => vec![key],
        }
    }
}

impl TryFrom<Args> for Command {
    type Error = String;

    fn try_from(value: Args) -> Result<Self, Self::Error> {
        let mut args = value.skip(1);
        let command = match args.next() {
            Some(command) => command,
            None => return Err("No command provided".to_string()),
        };

        let command = match command.as_str() {
            "get" => {
                let arg = match args.next() {
                    Some(value) => value,
                    None => return Err("Command get needs 1 argument".to_string()),
                };

                Command::Get(arg)
            }
            "set" => {
                if args.len() < 2 {
                    return Err("Command set needs 2 arguments".to_string());
                }

                Command::Set(args.next().unwrap(), args.next().unwrap())
            }
            "del" => {
                let arg = match args.next() {
                    Some(value) => value,
                    None => return Err("Command get needs 1 argument".to_string()),
                };

                Command::Delete(arg)
            }
            _ => return Err("Invalid command".to_string()),
        };

        Ok(command)
    }
}

impl TryFrom<ClientPayload> for Command {
    type Error = String;

    fn try_from(mut client_payload: ClientPayload) -> Result<Self, Self::Error> {
        // Implements [`TryFrom`] trait instead of [`From`] because the payload might be invalid.
        // Even though it's unlikely that the client binary will send invalid payload
        // given it already has enough validations to ensure the payload is valid.
        // You can't be too safe by adding server-side payload validation.

        let next_msg = match client_payload.next_msg() {
            Some(next_msg) => next_msg,
            None => return Err("Payload doesn't contain any command.".to_string()),
        };

        match next_msg.as_str() {
            "get" => {
                let arg = match client_payload.next_msg() {
                    Some(arg) => arg,
                    None => return Err("Missing argument for command \"get\".".to_string()),
                };

                return Ok(Self::Get(arg));
            }
            "set" => {
                let key = match client_payload.next_msg() {
                    Some(key) => key,
                    None => return Err("Missing key argument for command \"set\".".to_string()),
                };

                let value = match client_payload.next_msg() {
                    Some(val) => val,
                    None => return Err("Missing value argument for command \"set\".".to_string()),
                };

                return Ok(Self::Set(key, value));
            }
            "del" => {
                let arg = match client_payload.next_msg() {
                    Some(arg) => arg,
                    None => return Err("Missing argument for command \"del\".".to_string()),
                };

                return Ok(Self::Delete(arg));
            }
            _ => return Err("Invalid command.".to_string()),
        }
    }
}

pub enum StatusCodes {
    Ok,
    Err,
    NX,
}

/// Contains an array of bytes, which is the payload that is sent to the server.
/// The array of bytes' structure is described in the following table, the header
/// represents the position of bytes in the array.
///
/// | 1st     | 2nd     | 3rd     | 4th     | 5th     | ...     | n-th    | n+1-th  |
/// |---------|---------|---------|---------|---------|---------|---------|---------|
/// | nstr    | len     | str1    | len     | str2    | ...     | len     | strn    |
pub struct ClientPayload {
    buffer: Vec<u8>,
    last_index: usize,
}

impl ClientPayload {
    /// Get the array of bytes that is the payload.
    pub fn payload(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the header of the payload.
    pub fn header(&mut self) -> u32 {
        let mut header = [0u8; 4];
        header.copy_from_slice(&self.buffer[0..4]);
        u32::from_ne_bytes(header)
    }

    /// Get the length of the next message in the payload.
    /// Returns None if there is no next message.
    fn next_msg_len(&mut self) -> Option<u32> {
        // The message metadata is a 32bit integer.
        const BYTES_TO_READ: usize = 4;
        let buffer_len = self.payload().len();

        if self.last_index + 4 > buffer_len {
            return None;
        }

        let mut msg_len = [0u8; BYTES_TO_READ];
        msg_len.copy_from_slice(&self.buffer[self.last_index..self.last_index + BYTES_TO_READ]);
        self.last_index += BYTES_TO_READ;

        Some(u32::from_ne_bytes(msg_len))
    }

    /// Get the next message in the payload.
    /// Returns None if there is none.
    pub fn next_msg(&mut self) -> Option<String> {
        let next_msg_len: usize = match self.next_msg_len() {
            Some(value) => value as usize,
            None => return None,
        };

        let buffer_len = self.payload().len();
        if self.last_index + next_msg_len > buffer_len {
            return None;
        }

        let mut msg = vec![0u8; next_msg_len];
        msg.copy_from_slice(&self.buffer[self.last_index..self.last_index + next_msg_len]);
        self.last_index += next_msg_len as usize;
        let msg = String::from_utf8_lossy(&msg);
        return Some(msg.to_string());
    }
}

impl TryFrom<UnixStream> for ClientPayload {
    type Error = Error;

    fn try_from(mut stream: UnixStream) -> Result<Self, Self::Error> {
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;

        Ok(ClientPayload {
            buffer: buf,
            // Start from 4th byte because the first 4 bytes are the header
            last_index: 4,
        })
    }
}

impl From<Command> for ClientPayload {
    fn from(command: Command) -> Self {
        let command_str = command.as_str();
        let command_len_in_4b = command_str.len() as u32;
        let command_args = command.arguments();

        // Command + the number of arguments for the command
        let nstr: u32 = 1 + command_args.len() as u32;

        let mut payload = nstr.to_ne_bytes().to_vec();
        payload.append(&mut command_len_in_4b.to_ne_bytes().to_vec());
        payload.append(&mut command_str.as_bytes().to_vec());

        for value in command_args {
            let value_len_in_4b = value.len() as u32;
            payload.append(&mut value_len_in_4b.to_ne_bytes().to_vec());
            payload.append(&mut value.as_bytes().to_vec());
        }

        ClientPayload {
            buffer: payload,
            last_index: 4,
        }
    }
}
