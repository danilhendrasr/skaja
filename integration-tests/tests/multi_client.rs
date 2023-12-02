use std::time::Duration;

use integration_tests::test_utils::{new_client, with_server};

#[test]
pub fn client1_set_command_result_should_be_visible_to_client2() {
    with_server(|server_address| {
        let mut client1 = new_client(&server_address);
        std::thread::sleep(Duration::from_millis(5));
        let mut client2 = new_client(&server_address);

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
        assert_eq!(status_code, skaja_lib::StatusCodes::Ok);
        assert_eq!(response.message(), "world".to_string());
    })
}
