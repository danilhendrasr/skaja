use std::os::unix::net::UnixListener;

use polling::{Event, Poller};
use skaja::Stream;

fn main() -> Result<(), ()> {
    let socket_path = "mysocket";
    let listener = UnixListener::bind(socket_path).expect("Could not create unix socket");

    listener
        .set_nonblocking(true)
        .expect("Failed setting listener as non-blocking.");

    let poller = Poller::new().expect("Failed creating new poll");
    let _ = poller.add(&listener, Event::readable(7));

    let mut events = Vec::new();
    loop {
        events.clear();
        poller
            .wait(&mut events, None)
            .expect("Dunno man, it failed.");

        for ev in &events {
            if ev.key == 7 {
                let (unix_stream, _) = listener.accept().expect("Failed accepting connection");
                handle_stream(Stream::new(unix_stream))?;
                let _ = poller.modify(&listener, Event::readable(7));
            }
        }
    }
}

fn handle_stream(mut stream: Stream) -> Result<(), ()> {
    let nstr = stream.read_x_bytes(4);
    let nstr = u32::from_ne_bytes(nstr.try_into().unwrap());

    for i in 0..nstr {
        let str_len = u32::from_ne_bytes(stream.read_x_bytes(4).try_into().unwrap());
        let str = stream.read_x_bytes(str_len as usize);
        println!("Str {i} length: {str_len}");
        println!("Str {i}: ",);
        println!("{}", String::from_utf8_lossy(&str));
    }

    Ok(())
}
