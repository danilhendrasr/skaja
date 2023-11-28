use std::{
    io::{BufRead, BufReader},
    net::TcpListener,
    panic,
    path::PathBuf,
    process::Stdio,
};

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

pub fn launch_server_process() -> (std::process::Child, String) {
    let target_port = get_available_port().unwrap();
    let target_address = format!("127.0.0.1:{}", target_port);
    let mut process_handle = std::process::Command::new(skaja_server_exe())
        .arg("--address")
        .arg(&target_address)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to launch server process");

    let server_stdout = process_handle.stdout.as_mut().unwrap();
    let stdout_buf = BufReader::new(server_stdout);
    let lines = stdout_buf.lines();
    let mut server_listening = false;

    // Wait for the server to start listening
    for line in lines.flatten() {
        if line.contains("Server listening on") {
            server_listening = true;
            break;
        }
    }

    if !server_listening {
        let server_stderr = process_handle.stderr.as_mut().unwrap();
        let stderr_buf = BufReader::new(server_stderr);
        let lines = stderr_buf.lines();

        // Wait for the server to start listening
        for line in lines.flatten() {
            if line.contains("panic") {
                println!("Process panicked, relaunching server in another port");
                return launch_server_process();
            }
        }
    }

    println!(
        "Launched server process with pid {} on: {}",
        process_handle.id(),
        target_address
    );

    (process_handle, target_address)
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
    let (mut server_handle, server_address) = launch_server_process();

    let test_result = panic::catch_unwind(|| test(server_address));

    server_handle.kill().unwrap();

    if let Err(e) = test_result {
        panic::resume_unwind(e);
    }
}
