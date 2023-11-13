use std::{
    env::Args,
    io::{self, Error, ErrorKind, Read},
};

use mio::net::TcpStream;

pub trait AsRequest {
    /// Converts the stream into a [`Request`] struct without taking ownership of the stream,
    /// which is different from [`Into<Request>`] trait which takes ownership of the param.
    fn as_request(&mut self) -> Result<Request, io::Error>;
}

impl AsRequest for TcpStream {
    fn as_request(&mut self) -> Result<Request, io::Error> {
        let mut done_reading_command = false;
        let mut received_data = Vec::new();

        let mut next_chunk_len = 4;
        let mut cur_chunk = 0;
        let mut chunks_len = 0;

        loop {
            if cur_chunk > chunks_len {
                done_reading_command = true;
                break;
            }

            let chunk_is_header = cur_chunk == 0;
            let chunk_is_msg_header = cur_chunk % 2 != 0;
            if !chunk_is_header && chunk_is_msg_header {
                // If msg_count is odd, we read 4 bytes which is the length of the next message.
                next_chunk_len = 4;
            }

            let mut buf = vec![0; next_chunk_len];
            match self.read_exact(&mut buf) {
                Ok(_) => {
                    if chunk_is_header {
                        // The first chunk is the header, which is the number of chunks in the payload.
                        // We set it to its value * 2 because each message in the payload has 2 chunks:
                        // 1. The length of the message.
                        // 2. The message itself.
                        chunks_len = u32::from_ne_bytes(buf.clone().try_into().unwrap()) * 2;
                    } else if chunk_is_msg_header {
                        next_chunk_len =
                            u32::from_ne_bytes(buf.clone().try_into().unwrap()) as usize;
                    }

                    received_data.append(&mut buf);
                    cur_chunk += 1;
                }

                // Would block "errors" are the OS's way of saying that the
                // connection is not actually ready to perform this I/O operation.
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
                // Other errors we'll consider fatal.
                Err(err) => {
                    eprintln!("Fatal error: {:?}", err);
                    return Err(err);
                }
            }
        }

        if !done_reading_command {
            return Err(Error::new(
                ErrorKind::WouldBlock,
                "Connection not ready to be read from.",
            ));
        }

        Ok(Request {
            payload: received_data,
            // Points to the first byte of the payload.
            // The first 4 bytes are the header, so we skip them.
            pointer_pos: 4,
        })
    }
}

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
                    None => return Err("'get' command needs 1 argument".to_string()),
                };

                Command::Get(arg)
            }
            "set" => {
                if args.len() < 2 {
                    return Err("'set' command needs 2 arguments".to_string());
                }

                Command::Set(args.next().unwrap(), args.next().unwrap())
            }
            "del" => {
                let arg = match args.next() {
                    Some(value) => value,
                    None => return Err("'del' command needs 1 argument".to_string()),
                };

                Command::Delete(arg)
            }
            _ => return Err("Invalid command".to_string()),
        };

        Ok(command)
    }
}

impl Into<Request> for Command {
    fn into(self) -> Request {
        let mut payload: Vec<u8> = Vec::new();

        let command_str = self.as_str();

        // The length of the command string, truncated to 32bit.
        let command_len = command_str.len() as u32;

        // Command + the number of arguments for the command
        let command_args = self.arguments();
        let header = 1 + command_args.len() as u32;

        payload.append(&mut header.to_ne_bytes().into());
        payload.append(&mut command_len.to_ne_bytes().into());
        payload.append(&mut command_str.into());

        for arg in command_args {
            let arg_len = arg.len() as u32;
            payload.append(&mut arg_len.to_ne_bytes().into());
            payload.append(&mut arg.into());
        }

        Request {
            payload,
            pointer_pos: 4,
        }
    }
}

pub enum StatusCodes {
    Ok,
    Err,
    NX,
}

pub struct Request {
    /// Contains an array of bytes, which is the payload that is sent to the server.
    /// The array of bytes' structure is described in the following table, the header
    /// represents the position of bytes in the array.
    ///
    /// | 1st     | 2nd     | 3rd     | 4th     | 5th     | ...     | n-th    | n+1-th  |
    /// |---------|---------|---------|---------|---------|---------|---------|---------|
    /// | nstr    | len     | str1    | len     | str2    | ...     | len     | strn    |
    payload: Vec<u8>,

    /// The position of the pointer in the payload.
    /// This is used to parse the payload into a [`Command`] struct.
    pointer_pos: usize,
}

impl Request {
    /// Get the actual bytes payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Get the header of the payload.
    pub fn header(&mut self) -> u32 {
        let mut header = [0u8; 4];
        header.copy_from_slice(&self.payload[0..4]);
        u32::from_ne_bytes(header)
    }

    /// Get the length of the next message in the payload.
    /// Returns None if there is no next message.
    fn next_msg_len(&mut self) -> Option<u32> {
        // The message metadata is a 32bit integer.
        const BYTES_TO_READ: usize = 4;
        let buffer_len = self.payload().len();

        if self.pointer_pos + 4 > buffer_len {
            return None;
        }

        let mut msg_len = [0u8; BYTES_TO_READ];
        msg_len.copy_from_slice(&self.payload[self.pointer_pos..self.pointer_pos + BYTES_TO_READ]);
        self.pointer_pos += BYTES_TO_READ;

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
        if self.pointer_pos + next_msg_len > buffer_len {
            return None;
        }

        let mut msg = vec![0u8; next_msg_len];
        msg.copy_from_slice(&self.payload[self.pointer_pos..self.pointer_pos + next_msg_len]);
        self.pointer_pos += next_msg_len as usize;
        let msg = String::from_utf8_lossy(&msg);
        return Some(msg.to_string());
    }
}

impl TryInto<Command> for Request {
    type Error = String;

    fn try_into(mut self) -> Result<Command, Self::Error> {
        // Implements [`TryFrom`] trait instead of [`From`] because the payload might be invalid.
        // Even though it's unlikely that the client binary will send invalid payload
        // given it already has enough validations to ensure the payload is valid.
        // You can't be too safe by adding server-side payload validation.

        let next_msg = match self.next_msg() {
            Some(next_msg) => next_msg,
            None => return Err("Payload doesn't contain any command.".to_string()),
        };

        match next_msg.as_str() {
            "get" => {
                let arg = match self.next_msg() {
                    Some(arg) => arg,
                    None => return Err("Missing argument for command \"get\".".to_string()),
                };

                return Ok(Command::Get(arg));
            }
            "set" => {
                let key = match self.next_msg() {
                    Some(key) => key,
                    None => return Err("Missing key argument for command \"set\".".to_string()),
                };

                let value = match self.next_msg() {
                    Some(val) => val,
                    None => return Err("Missing value argument for command \"set\".".to_string()),
                };

                return Ok(Command::Set(key, value));
            }
            "del" => {
                let arg = match self.next_msg() {
                    Some(arg) => arg,
                    None => return Err("Missing argument for command \"del\".".to_string()),
                };

                return Ok(Command::Delete(arg));
            }
            _ => return Err("Invalid command.".to_string()),
        }
    }
}
