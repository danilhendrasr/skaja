use std::os::unix::net::UnixListener;

use polling::{Event, Poller};
use skaja::Buff;

fn main() -> Result<(), ()> {
    let socket_path = "mysocket";
    let socket = UnixListener::bind(socket_path).expect("Could not create unix socket");

    socket
        .set_nonblocking(true)
        .expect("Failed setting listener as non-blocking.");

    let poller = Poller::new().expect("Failed creating new poll");
    let _ = poller.add(&socket, Event::readable(7));

    let mut events = Vec::new();
    loop {
        events.clear();
        poller
            .wait(&mut events, None)
            .expect("Dunno man, it failed.");

        for ev in &events {
            if ev.key == 7 {
                let (unix_stream, _) = socket.accept().expect("Failed accepting connection");
                _ = poller.modify(&socket, Event::readable(7));

                let buffer = match Buff::new(unix_stream) {
                    Ok(buffer) => buffer,
                    Err(_) => {
                        println!("Failed creating buffer, skipping connection.");
                        break;
                    }
                };

                handle_connection(buffer)?;
            }
        }
    }
}

fn handle_connection(mut buffer: Buff) -> Result<(), ()> {
    for _ in 0..buffer.header() {
        let str = buffer.next_msg();
        println!("{}", str);
    }

    Ok(())
}
