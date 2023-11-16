use std::path::PathBuf;

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

#[test]
pub fn get_non_existent_key_should_return_err() {
    let skaja_server_exe = skaja_server_exe();
    let mut server = std::process::Command::new(skaja_server_exe)
        .spawn()
        .unwrap();
    let mut client = skaja_client::Client::connect("127.0.0.1:3000".parse().unwrap());
    match client.send(skaja_lib::Command::Get("hello".to_string())) {
        Ok(response) => {
            let status_code = response.status_code();
            assert_eq!(status_code, skaja_lib::StatusCodes::ClientErr);
            assert_eq!(response.message(), r#"Key "hello" not found."#.to_string());
        }
        Err(_) => {}
    }
    server.kill().unwrap();
}
