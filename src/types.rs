use std::{
    io::{Error, Read},
    os::unix::net::UnixStream,
};

pub enum Commands {
    Get(Option<String>),
    Set(Option<(String, String)>),
    Delete(Option<String>),
}

impl Commands {
    pub fn as_str(&self) -> &str {
        match self {
            Commands::Set(_) => "set",
            Commands::Get(_) => "get",
            Commands::Delete(_) => "del",
        }
    }

    pub fn arguments(&self) -> Option<Vec<&str>> {
        match self {
            Commands::Set(val) => {
                if let Some((key, value)) = val {
                    return Some(vec![key, value]);
                }

                return None;
            }
            Commands::Get(key) => {
                if let Some(key) = key {
                    return Some(vec![key]);
                }

                return None;
            }
            Commands::Delete(key) => {
                if let Some(key) = key {
                    return Some(vec![key]);
                }

                return None;
            }
        }
    }
}

impl TryFrom<String> for Commands {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "get" => Ok(Self::Get(None)),
            "set" => Ok(Self::Set(None)),
            "del" => Ok(Self::Delete(None)),
            _ => Err("Invalid command".to_string()),
        }
    }
}

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

    pub fn header(&mut self) -> u32 {
        let mut header = [0u8; 4];
        header.copy_from_slice(&self.buffer[0..4]);
        println!("buffer: {:?}", self.buffer);
        u32::from_ne_bytes(header)
    }

    fn next_msg_len(&mut self) -> u32 {
        let mut msg_len = [0u8; 4];
        msg_len.copy_from_slice(&self.buffer[self.last_index..self.last_index + 4]);
        self.last_index += 4;
        u32::from_ne_bytes(msg_len)
    }

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
