pub mod test_utils {
    use std::{
        io::{BufRead, BufReader},
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

    pub fn launch_server_process() -> (std::process::Child, String) {
        let mut server_address: String = "127.0.0.1:3000".to_string();
        let mut process_handle = std::process::Command::new(skaja_server_exe())
            .arg("--address")
            .arg(&server_address)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let server_stdout = process_handle.stdout.as_mut().unwrap();
        let stdout_buf = BufReader::new(server_stdout);
        let mut lines = stdout_buf.lines();
        while let Some(line) = lines.next() {
            if let Ok(line) = line {
                if line.contains("Server listening on") {
                    let final_server_address = line.replace("Server listening on: ", "");
                    server_address = final_server_address;
                    break;
                }
            }
        }

        (process_handle, server_address)
    }
}
