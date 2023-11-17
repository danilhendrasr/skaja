mod constants;
mod domains;

pub use domains::*;

use std::io::{self, Error, ErrorKind, Read};

use mio::net::TcpStream;

pub use constants::*;

impl ReadToRequest for TcpStream {
    fn read_to_request(&mut self) -> Result<Request, io::Error> {
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
                Err(err) => return Err(err),
            }
        }

        if !done_reading_command {
            return Err(Error::new(
                ErrorKind::WouldBlock,
                "Connection not ready to be read from.",
            ));
        }

        Ok(Request::new_with_payload(received_data))
    }
}

impl ReadToResponse for TcpStream {
    fn read_to_response(&mut self) -> Result<RawResponse, io::Error> {
        let mut done_reading = false;
        let mut received_data = Vec::new();

        let mut next_chunk_len = 4;
        let mut cur_chunk = 0;
        let chunks_len = 3;

        loop {
            if cur_chunk >= chunks_len {
                done_reading = true;
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
                    if chunk_is_msg_header {
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
                Err(err) => return Err(err),
            }
        }

        if !done_reading {
            return Err(Error::new(
                ErrorKind::WouldBlock,
                "Connection not ready to be read from.",
            ));
        }

        Ok(RawResponse(received_data))
    }
}
