use std::io;

pub trait ReadToResponse {
    /// Reads target into a [`RawResponse`] struct without taking ownership of the target.
    fn read_to_response(&mut self) -> Result<RawResponse, io::Error>;
}

#[derive(Debug)]
pub enum StatusCodes {
    Ok,
    ClientErr,
    ServerErr,
}

impl std::fmt::Display for StatusCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            StatusCodes::Ok => "OK",
            StatusCodes::ClientErr => "Client error",
            StatusCodes::ServerErr => "Server error",
        };

        write!(f, "{}", msg)
    }
}

impl From<StatusCodes> for u32 {
    fn from(value: StatusCodes) -> Self {
        match value {
            StatusCodes::Ok => 0,
            StatusCodes::ClientErr => 1,
            StatusCodes::ServerErr => 2,
        }
    }
}

impl From<u32> for StatusCodes {
    fn from(value: u32) -> Self {
        match value {
            0 => StatusCodes::Ok,
            1 => StatusCodes::ClientErr,
            2 => StatusCodes::ServerErr,
            _ => panic!("Invalid status code."),
        }
    }
}

/// A response is a payload that is sent from the server to the client.
/// The following is the structure of the payload:
///
/// | 1st chunk   | 2nd         | 3rd   |
/// |-------------|-------------|-------|
/// | resp code   | len of msg  | msg   |
///
/// The first chunk is the number representation of the status code (see [`StatusCodes`]).
/// The second chunk is the length of the response message.
/// The third chunk is the response message.
///
/// The first chunk is called the header.
/// The second chunk is called the message header.
#[derive(Debug)]
pub struct RawResponse(pub Vec<u8>);

impl RawResponse {
    pub fn new(status_code: StatusCodes, msg: String) -> Self {
        let mut payload: Vec<u8> = Vec::new();

        // The length of the status code string, truncated to 32bit.
        let status_code_int: u32 = status_code.into();

        // Status code + the number of arguments for the command
        let msg_len = msg.len() as u32;

        payload.append(&mut status_code_int.to_ne_bytes().to_vec());
        payload.append(&mut msg_len.to_ne_bytes().to_vec());
        payload.append(&mut msg.into_bytes());

        RawResponse(payload)
    }

    pub fn payload(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug)]
pub struct Response {
    status_code: StatusCodes,
    message: String,
}

impl From<RawResponse> for Response {
    fn from(value: RawResponse) -> Self {
        let raw_bytes = value.payload();

        let status_code = u32::from_ne_bytes(raw_bytes[0..4].try_into().unwrap());
        let msg_len = u32::from_ne_bytes(raw_bytes[4..8].try_into().unwrap());
        let msg = String::from_utf8_lossy(&raw_bytes[8..(8 + msg_len as usize)]);

        Response {
            status_code: StatusCodes::from(status_code),
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = format!("[{}]: {}", self.status_code, self.message);
        write!(f, "{}", msg)
    }
}

impl From<RawResponse> for Vec<u8> {
    fn from(value: RawResponse) -> Self {
        value.0
    }
}
