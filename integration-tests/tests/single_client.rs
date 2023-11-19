use integration_tests::test_utils::launch_server_process;

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
