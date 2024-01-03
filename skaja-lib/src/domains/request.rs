use super::command::Command;

#[derive(Debug, PartialEq)]
pub struct Request {
    /// Contains an array of bytes, which is the payload that is sent to the server.
    /// The array of bytes' structure is described in the following table:
    ///
    /// | 1st chunk   | 2nd          | 3rd   | 4th         | 5th   | ...     | n-th         | n+1-th |
    /// |-------------|--------------|-------|-------------|-------|---------|--------------|--------|
    /// | num of msg  | len of msg1  | msg1  | len of msg2 | msg2  | ...     | len of msgN  | msgN   |
    ///
    /// The first chunk is the number of messages in the payload.
    /// The second chunk is the length of the first message.
    /// The third chunk is the first message.
    /// The fourth chunk is the length of the second message.
    /// The fifth chunk is the second message.
    /// And so on...
    ///
    /// The first chunk is called the header.
    /// The even chunks (or odd if it's 0-based) are called the message headers.
    payload: Vec<u8>,

    /// The position of the pointer in the payload.
    /// This is used to parse the payload into a [`Command`] struct.
    ///
    /// Usually the initial value is 4, because the first 4 bytes are the header,
    /// which is the number of messages in the payload, so we skip them and point
    /// straight to the data.
    pointer_pos: usize,
}

impl Request {
    /// Create a new empty [`Request`] struct.
    pub fn new() -> Self {
        Self {
            payload: Vec::new(),
            pointer_pos: 0,
        }
    }

    /// Create a new [`Request`] struct with a payload.
    ///
    /// This function doesn't do any validation on the payload. So make sure the payload is valid.
    pub fn new_with_payload(payload: Vec<u8>) -> Self {
        Self {
            payload,
            // Points to the first byte of the payload.
            // The first 4 bytes are the header, so we skip them.
            pointer_pos: 4,
        }
    }

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
        self.pointer_pos += next_msg_len;
        let msg = String::from_utf8_lossy(&msg);
        Some(msg.to_string())
    }
}

impl std::default::Default for Request {
    fn default() -> Self {
        Self::new()
    }
}

impl TryInto<Command> for Request {
    type Error = String;

    /// Implements [`TryFrom`] trait instead of [`From`] because the payload might be invalid.
    /// Even though it's unlikely that the client binary will send invalid payload
    /// given it already has enough validations to ensure the payload is valid.
    /// You can't be too safe by adding server-side payload validation.
    fn try_into(mut self) -> Result<Command, Self::Error> {
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

                Ok(Command::Get(arg))
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

                Ok(Command::Set(key, value))
            }
            "del" => {
                let arg = match self.next_msg() {
                    Some(arg) => arg,
                    None => return Err("Missing argument for command \"del\".".to_string()),
                };

                Ok(Command::Delete(arg))
            }
            _ => Err("Invalid command.".to_string()),
        }
    }
}

#[cfg(test)]
mod request_to_command {
    use super::Request;
    use crate::domains::command::Command;

    #[test]
    pub fn valid_get_payload_should_deserialized_correctly() {
        // Payload for "get testing" command
        let payload = vec![
            2, 0, 0, 0, 3, 0, 0, 0, 103, 101, 116, 7, 0, 0, 0, 116, 101, 115, 116, 105, 110, 103,
        ];
        let request = Request::new_with_payload(payload);
        let command: Command = request.try_into().unwrap();
        assert_eq!(command, Command::Get("testing".to_string()));
    }

    #[test]
    pub fn valid_set_payload_should_deserialized_correctly() {
        // Payload for "set key value" command
        let payload = vec![
            3, 0, 0, 0, 3, 0, 0, 0, 115, 101, 116, 3, 0, 0, 0, 107, 101, 121, 5, 0, 0, 0, 118, 97,
            108, 117, 101,
        ];
        let request = Request::new_with_payload(payload);
        let command: Command = request.try_into().unwrap();
        assert_eq!(
            command,
            Command::Set("key".to_string(), "value".to_string())
        );
    }

    #[test]
    pub fn valid_del_payload_should_deserialized_correctly() {
        // Payload for "del testing" command
        let payload = vec![
            2, 0, 0, 0, 3, 0, 0, 0, 100, 101, 108, 7, 0, 0, 0, 116, 101, 115, 116, 105, 110, 103,
        ];
        let request = Request::new_with_payload(payload);
        let command: Command = request.try_into().unwrap();
        assert_eq!(command, Command::Delete("testing".to_string()));
    }

    #[test]
    #[should_panic]
    pub fn invalid_payload_should_result_in_error() {
        let request = Request::new_with_payload(vec![0, 0, 0, 0]);
        let _: Command = request.try_into().unwrap();
    }
}
