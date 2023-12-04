use std::io;

pub trait ReadToResponse {
    /// Reads target into a [`RawResponse`] struct without taking ownership of the target.
    fn read_to_response(&mut self) -> Result<RawResponse, io::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusCodes {
    Ok,
    ClientErr,
    ServerErr,
    ErrNotFound,
}

impl std::fmt::Display for StatusCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            StatusCodes::Ok => "OK",
            StatusCodes::ClientErr => "Client error",
            StatusCodes::ServerErr => "Server error",
            StatusCodes::ErrNotFound => "Key not found",
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
            StatusCodes::ErrNotFound => 3,
        }
    }
}

impl From<u32> for StatusCodes {
    fn from(value: u32) -> Self {
        match value {
            0 => StatusCodes::Ok,
            1 => StatusCodes::ClientErr,
            2 => StatusCodes::ServerErr,
            3 => StatusCodes::ErrNotFound,
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

impl From<RawResponse> for Vec<u8> {
    fn from(value: RawResponse) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub struct Response {
    status_code: StatusCodes,
    message: Option<String>,
}

impl Response {
    pub fn new(status_code: StatusCodes, msg: Option<String>) -> Self {
        Response {
            status_code,
            message: msg,
        }
    }

    pub fn status_code(&self) -> StatusCodes {
        self.status_code
    }

    pub fn message(&self) -> Option<&str> {
        if let Some(ref msg) = self.message {
            return Some(msg.as_str());
        }

        None
    }
}

impl From<RawResponse> for Response {
    fn from(value: RawResponse) -> Self {
        let raw_bytes = value.payload();

        let status_code = u32::from_ne_bytes(raw_bytes[0..4].try_into().unwrap());
        let msg_len = u32::from_ne_bytes(raw_bytes[4..8].try_into().unwrap());
        if msg_len == 0 {
            return Response {
                status_code: StatusCodes::from(status_code),
                message: None,
            };
        }

        let msg = String::from_utf8_lossy(&raw_bytes[8..(8 + msg_len as usize)]);
        Response {
            status_code: StatusCodes::from(status_code),
            message: Some(msg.into()),
        }
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self.message {
            Some(ref msg) => msg,
            None => "",
        };
        let msg = format!("[{}]: {}", self.status_code, msg);
        write!(f, "{}", msg)
    }
}

#[cfg(test)]
mod raw_response {
    use super::{RawResponse, StatusCodes};
    use crate::Response;

    #[test]
    pub fn new_ok_should_result_in_correct_payload() {
        let raw_response = RawResponse::new(StatusCodes::Ok, "OK".into());
        let payload = raw_response.payload();

        let header = &payload[0..4];
        let msg_header = &payload[4..8];
        let msg = &payload[8..];

        assert_eq!(payload.len(), 10);
        assert_eq!(header, [0, 0, 0, 0]);
        assert_eq!(msg_header, [2, 0, 0, 0]);
        assert_eq!(msg, [79, 75]);
    }

    #[test]
    pub fn new_client_err_should_result_in_correct_payload() {
        let raw_response =
            RawResponse::new(StatusCodes::ClientErr, r#"Key "testing" not found."#.into());
        let payload = raw_response.payload();

        let header = &payload[0..4];
        let msg_header = &payload[4..8];
        let msg = &payload[8..];

        assert_eq!(payload.len(), 32);
        assert_eq!(header, [1, 0, 0, 0]);
        assert_eq!(msg_header, [24, 0, 0, 0]);
        assert_eq!(
            msg,
            [
                75, 101, 121, 32, 34, 116, 101, 115, 116, 105, 110, 103, 34, 32, 110, 111, 116, 32,
                102, 111, 117, 110, 100, 46
            ]
        );
    }

    #[test]
    pub fn new_server_err_should_result_in_correct_payload() {
        let raw_response = RawResponse::new(StatusCodes::ServerErr, r#"Server error"#.into());
        let payload = raw_response.payload();

        let header = &payload[0..4];
        let msg_header = &payload[4..8];
        let msg = &payload[8..];

        assert_eq!(payload.len(), 20);
        assert_eq!(header, [2, 0, 0, 0]);
        assert_eq!(msg_header, [12, 0, 0, 0]);
        assert_eq!(
            msg,
            [83, 101, 114, 118, 101, 114, 32, 101, 114, 114, 111, 114]
        );
    }

    #[test]
    pub fn ok_should_be_parsed_correctly_to_response() {
        let raw_response = RawResponse::new(StatusCodes::Ok, "OK".into());
        let response: Response = raw_response.into();

        assert_eq!(response.status_code(), StatusCodes::Ok);
        assert_eq!(response.message(), Some("OK"));
    }

    #[test]
    pub fn client_err_should_be_parsed_correctly_to_response() {
        let raw_response = RawResponse::new(StatusCodes::ClientErr, "There's an error".into());
        let response: Response = raw_response.into();

        assert_eq!(response.status_code(), StatusCodes::ClientErr);
        assert_eq!(response.message(), Some("There's an error"));
    }

    #[test]
    pub fn server_err_should_be_parsed_correctly_to_response() {
        let raw_response = RawResponse::new(StatusCodes::ServerErr, "Server error".into());
        let response: Response = raw_response.into();

        assert_eq!(response.status_code(), StatusCodes::ServerErr);
        assert_eq!(response.message(), Some("Server error"));
    }
}
