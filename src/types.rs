use std::{io::Read, os::unix::net::UnixStream};

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

pub struct Stream(UnixStream);

impl Stream {
    pub fn new(stream: UnixStream) -> Self {
        Self(stream)
    }

    pub fn read_x_bytes(&mut self, x: usize) -> Vec<u8> {
        let mut buffer = vec![0u8; x];
        self.0
            .read_exact(&mut buffer)
            .expect("Failed reading from stream.");
        return buffer;
    }
}

pub enum StatusCodes {
    Ok,
    Err,
    NX,
}
