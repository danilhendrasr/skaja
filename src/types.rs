use std::{
    env::Args,
    io::{Error, Read},
    os::unix::net::UnixStream,
};

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

/// Buffer to be used for processing [`ClientPayload`].
pub struct Buff {
    buffer: Vec<u8>,
    last_index: usize,
}

impl Buff {
    pub fn new(mut stream: UnixStream) -> Result<Self, Error> {
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;

        Ok(Buff {
            buffer: buf,
            // Start from 4 because the first 4 bytes are the header
            last_index: 4,
        })
    }

    /// Get the header of the payload.
    pub fn header(&mut self) -> u32 {
        let mut header = [0u8; 4];
        header.copy_from_slice(&self.buffer[0..4]);
        println!("buffer: {:?}", self.buffer);
        u32::from_ne_bytes(header)
    }

    /// Get the length of the next message in the payload.
    fn next_msg_len(&mut self) -> u32 {
        let mut msg_len = [0u8; 4];
        msg_len.copy_from_slice(&self.buffer[self.last_index..self.last_index + 4]);
        self.last_index += 4;
        u32::from_ne_bytes(msg_len)
    }

    /// Get the next message in the payload.
    pub fn next_msg(&mut self) -> String {
        let msg_len = self.next_msg_len();

        let mut msg = vec![0u8; msg_len as usize];
        msg.copy_from_slice(&self.buffer[self.last_index..self.last_index + msg_len as usize]);
        self.last_index += msg_len as usize;
        let msg = String::from_utf8_lossy(&msg);
        println!("msg: {}", msg);
        return msg.to_string();
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
/// | 1st | 2nd | 3rd | 4th | 5th | ... | n-th | n+1-th |
/// |---------|---------|---------|---------|---------|---------|---------|---------|
/// | nstr | len | str1 | len | str2 | ... | len | strn |
pub struct ClientPayload(Vec<u8>);

impl ClientPayload {
    /// Get the array of bytes that is the payload.
    pub fn payload(&self) -> &[u8] {
        &self.0
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

        ClientPayload(payload)
    }
}
