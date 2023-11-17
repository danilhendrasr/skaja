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

fn launch_server_process() -> (std::process::Child, String) {
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

#[test]
pub fn setting_a_key_should_result_in_ok() {
    let (mut server, server_address) = launch_server_process();

    let mut client = skaja_client::Client::connect(server_address.parse().unwrap());
    let response = client
        .send(skaja_lib::Command::Set(
            "hello".to_string(),
            "world".to_string(),
        ))
        .unwrap();

    let status_code = response.status_code();
    println!("status_code: {:?}", status_code);
    assert_eq!(status_code, skaja_lib::StatusCodes::Ok);
    assert_eq!(
        response.message(),
        r#"Key "hello" set to "world"."#.to_string()
    );

    server.kill().unwrap();
}

#[test]
pub fn getting_existing_key_should_result_in_ok() {
    let (mut server, server_address) = launch_server_process();

    let mut client = skaja_client::Client::connect(server_address.parse().unwrap());
    client
        .send(skaja_lib::Command::Set(
            "hello".to_string(),
            "world".to_string(),
        ))
        .unwrap();

    let response = client
        .send(skaja_lib::Command::Get("hello".to_string()))
        .unwrap();
    let status_code = response.status_code();
    println!("status_code: {:?}", status_code);
    assert_eq!(status_code, skaja_lib::StatusCodes::Ok);
    assert_eq!(response.message(), "world".to_string());

    server.kill().unwrap();
}

#[test]
pub fn getting_non_existing_key_should_result_in_client_error() {
    let (mut server, server_address) = launch_server_process();

    let mut client = skaja_client::Client::connect(server_address.parse().unwrap());
    let response = client
        .send(skaja_lib::Command::Get("hello".to_string()))
        .unwrap();
    let status_code = response.status_code();
    assert_eq!(status_code, skaja_lib::StatusCodes::ClientErr);
    assert_eq!(response.message(), r#"Key "hello" not found."#.to_string());

    server.kill().unwrap();
}

#[test]
pub fn deleting_existing_key_should_result_in_ok() {
    let (mut server, server_address) = launch_server_process();

    let mut client = skaja_client::Client::connect(server_address.parse().unwrap());
    client
        .send(skaja_lib::Command::Set(
            "hello".to_string(),
            "world".to_string(),
        ))
        .unwrap();
    let response = client
        .send(skaja_lib::Command::Delete("hello".to_string()))
        .unwrap();
    let status_code = response.status_code();
    assert_eq!(status_code, skaja_lib::StatusCodes::Ok);
    assert_eq!(response.message(), r#"Key "hello" deleted."#.to_string());

    server.kill().unwrap();
}

#[test]
pub fn deleting_non_existent_key_should_result_in_client_error() {
    let (mut server, server_address) = launch_server_process();

    let mut client = skaja_client::Client::connect(server_address.parse().unwrap());
    let response = client
        .send(skaja_lib::Command::Delete("hello".to_string()))
        .unwrap();
    let status_code = response.status_code();
    assert_eq!(status_code, skaja_lib::StatusCodes::ClientErr);
    assert_eq!(response.message(), r#"Key "hello" not found."#.to_string());

    server.kill().unwrap();
}
