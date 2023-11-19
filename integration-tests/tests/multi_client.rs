use integration_tests::test_utils::launch_server_process;

#[test]
pub fn client1_set_command_result_should_be_visible_to_client2() {
    let (mut server, server_address) = launch_server_process();

    let mut client1 = skaja_client::Client::connect(server_address.parse().unwrap());
    let mut client2 = skaja_client::Client::connect(server_address.parse().unwrap());

    client1
        .send(skaja_lib::Command::Set(
            "hello".to_string(),
            "world".to_string(),
        ))
        .unwrap();

    let response = client2
        .send(skaja_lib::Command::Get("hello".to_string()))
        .unwrap();
    let status_code = response.status_code();
    println!("status_code: {:?}", status_code);
    assert_eq!(status_code, skaja_lib::StatusCodes::Ok);
    assert_eq!(response.message(), "world".to_string());

    server.kill().unwrap();
}
