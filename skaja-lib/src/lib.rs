mod constants;
mod domains;

pub use constants::*;
pub use domains::*;
use mio::net::TcpStream;
use std::io::{self, ErrorKind, Read};

/// Kind of like the [`TryFrom`] trait, but doesn't take ownership of the source.
///
/// Not recommended to be implemented directly. It's just here to provide more syntactical
/// option for converting one type to another without taking ownership.
/// Use [`Extract`] instead and this trait will be implemented automatically.
pub trait OutOf<T> {
    type Error;

    /// Converts the source into the target type without taking ownership of the source.
    fn outof(source: &mut T) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// Kind of like the [`TryInto`] trait, but doesn't take ownership of self.
pub trait Extract<T> {
    type Error;

    /// Converts self into the target type without taking ownership of self.
    fn extract(&mut self) -> Result<T, Self::Error>;
}

/// Blanket implementation of [`OutOf`] for any type that implements [`Extract`].
impl<T, R> OutOf<T> for R
where
    T: Extract<R>,
{
    type Error = <T as Extract<R>>::Error;

    fn outof(source: &mut T) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        source.extract()
    }
}

impl Extract<Request> for TcpStream {
    type Error = io::Error;

    fn extract(&mut self) -> Result<Request, Self::Error> {
        // The container for the data we'll receive.
        let mut received_data = Vec::new();

        // The current chunk we're reading.
        let mut current_chunk = 0;

        // How many chunks we expect to receive.
        let mut chunks_len = 0;

        // In bytes.
        let mut next_chunk_size = 4;

        loop {
            if current_chunk > chunks_len {
                break;
            }

            let chunk_is_header = current_chunk == 0;
            let chunk_is_msg_header = current_chunk % 2 != 0;
            if !chunk_is_header && chunk_is_msg_header {
                // If msg_count is odd, we read 4 bytes which is the length of the next message.
                next_chunk_size = 4;
            }

            let mut buf = vec![0; next_chunk_size];
            match self.read_exact(&mut buf) {
                Ok(_) => {
                    if chunk_is_header {
                        // The first chunk is the header, which is the number of chunks in the payload.
                        // We set it to its value * 2 because each message in the payload has 2 chunks:
                        // 1. The length of the message.
                        // 2. The message itself.
                        chunks_len = u32::from_ne_bytes(buf.clone().try_into().unwrap()) * 2;
                    } else if chunk_is_msg_header {
                        next_chunk_size =
                            u32::from_ne_bytes(buf.clone().try_into().unwrap()) as usize;
                    }

                    received_data.append(&mut buf);
                    current_chunk += 1;
                }

                // Would block "errors" are the OS's way of saying that the
                // connection is not actually ready to perform this I/O operation.
                Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
                // Other errors we'll consider fatal.
                Err(err) => return Err(err),
            }
        }

        Ok(Request::new_with_payload(received_data))
    }
}

impl Extract<RawResponse> for TcpStream {
    type Error = io::Error;

    fn extract(&mut self) -> Result<RawResponse, Self::Error> {
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

                        if next_chunk_len == 0 {
                            done_reading = true;
                            break;
                        }
                    }

                    received_data.append(&mut buf);
                    cur_chunk += 1;
                }

                // Would block "errors" are the OS's way of saying that the
                // connection is not actually ready to perform this I/O operation.
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => {
                    break;
                }
                Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
                // Other errors we'll consider fatal.
                Err(err) => {
                    return Err(err);
                }
            }
        }

        if !done_reading {
            return Err(io::Error::new(
                ErrorKind::WouldBlock,
                "Connection not ready to be read from.",
            ));
        }

        Ok(RawResponse(received_data))
    }
}
