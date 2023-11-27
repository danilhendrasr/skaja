pub mod test_utils {
    use std::{net::TcpListener, panic, path::PathBuf};

    fn skaja_server_exe() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../target");

        if cfg!(debug_assertions) {
            path.push("debug");
        } else {
            path.push("release");
        }

        if cfg!(target_os = "windows") {
            path.push("skaja_server.exe");
        } else {
            path.push("skaja_server");
        }

        path
    }

    fn get_available_port() -> Option<u16> {
        (8000..9000).find(|port| port_is_available(*port))
    }

    fn port_is_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    pub fn launch_server_process(target_address: &str) -> std::process::Child {
        let process_handle = std::process::Command::new(skaja_server_exe())
            .arg("--address")
            .arg(target_address)
            .spawn()
            .unwrap();

        process_handle
    }

    pub fn new_client(server_address: &str) -> skaja_client::Client {
        skaja_client::Client::connect(server_address.parse().unwrap())
    }

    /// Run the provided test function with a server process. It launches a server
    /// process on a random port, runs the provided test function,
    /// and then kills the server.
    pub fn with_server<T>(test: T)
    where
        T: FnOnce(String) + panic::UnwindSafe,
    {
        let target_address = format!("127.0.0.1:{}", get_available_port().unwrap());
        let mut server_handle = launch_server_process(&target_address);

        let _ = panic::catch_unwind(|| {
            test(target_address);
        });

        server_handle.kill().unwrap();
    }
}
