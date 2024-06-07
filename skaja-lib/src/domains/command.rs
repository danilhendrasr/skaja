use super::Request;
use crate::Extract;
use std::{env::Args, io};

/// The commands that can be sent to the server.
#[derive(Debug, PartialEq)]
pub enum Command {
    Get(String),
    Set(String, String),
    Delete(String),
}

impl TryFrom<String> for Command {
    type Error = String;

    fn try_from(string_command: String) -> Result<Self, Self::Error> {
        let string_command = string_command.to_lowercase();
        let splitted_string = string_command.split(' ').collect::<Vec<&str>>();

        if splitted_string.is_empty() {
            return Err("No command provided".to_string());
        }

        let command = splitted_string[0];
        let command = match command {
            "get" => {
                let key = if splitted_string.len() < 2 {
                    return Err("\"get\" command needs 1 argument".to_string());
                } else {
                    splitted_string[1]
                };

                Command::Get(key.to_owned())
            }
            "set" => {
                let (key, value) = if splitted_string.len() < 3 {
                    return Err("\"set\" command needs 2 arguments".to_string());
                } else {
                    (splitted_string[1], splitted_string[2])
                };

                Command::Set(key.to_owned(), value.to_owned())
            }
            "del" => {
                let key = if splitted_string.len() < 2 {
                    return Err("\"del\" command needs 1 argument".to_string());
                } else {
                    splitted_string[1]
                };

                Command::Delete(key.to_owned())
            }
            _ => return Err("Invalid command".to_string()),
        };

        Ok(command)
    }
}

impl TryFrom<Args> for Command {
    type Error = String;

    fn try_from(value: Args) -> Result<Self, Self::Error> {
        let mut args = value.skip(1);
        let command = match args.next() {
            Some(command) => command,
            None => return Err("No command provided".to_string()),
        };

        let command = match command.as_str() {
            "get" => {
                let arg = match args.next() {
                    Some(value) => value,
                    None => return Err("'get' command needs 1 argument".to_string()),
                };

                Command::Get(arg)
            }
            "set" => {
                if args.len() < 2 {
                    return Err("'set' command needs 2 arguments".to_string());
                }

                Command::Set(args.next().unwrap(), args.next().unwrap())
            }
            "del" => {
                let arg = match args.next() {
                    Some(value) => value,
                    None => return Err("'del' command needs 1 argument".to_string()),
                };

                Command::Delete(arg)
            }
            _ => return Err("Invalid command".to_string()),
        };

        Ok(command)
    }
}

impl Extract<Request> for Command {
    type Error = io::Error;

    fn extract(&mut self) -> Result<Request, Self::Error>
    where
        Self: Sized,
    {
        let mut payload: Vec<u8> = Vec::new();

        let args_len = match self {
            Command::Get(_) | Command::Delete(_) => 1,
            Command::Set(_, _) => 2,
        };

        let command = match self {
            Command::Get(_) => "get",
            Command::Set(_, _) => "set",
            Command::Delete(_) => "del",
        };

        // Header = the command + the number of arguments for the command
        let header: u32 = 1 + args_len;

        payload.append(&mut header.to_le_bytes().into());
        payload.append(&mut (command.len() as u32).to_le_bytes().into());
        payload.append(&mut command.into());

        match self {
            Command::Get(arg) | Command::Delete(arg) => {
                let arg_len = arg.len() as u32;
                payload.append(&mut arg_len.to_le_bytes().into());
                payload.append(&mut arg.clone().into_bytes());
            }
            Command::Set(key, value) => {
                let key_len = key.len() as u32;
                payload.append(&mut key_len.to_le_bytes().into());
                payload.append(&mut key.clone().into_bytes());

                let value_len = value.len() as u32;
                payload.append(&mut value_len.to_le_bytes().into());
                payload.append(&mut value.clone().into_bytes());
            }
        }

        Ok(Request::new_with_payload(payload))
    }
}

#[cfg(test)]
mod command_from_string {
    use crate::Command;

    #[test]
    pub fn valid_string_should_parses_to_command() {
        let command = Command::try_from("get key".to_string()).unwrap();
        assert_eq!(command, Command::Get("key".to_owned()));

        let command = Command::try_from("set key value".to_string()).unwrap();
        assert_eq!(command, Command::Set("key".to_owned(), "value".to_owned()));

        let command = Command::try_from("del key".to_string()).unwrap();
        assert_eq!(command, Command::Delete("key".to_owned()));
    }

    #[test]
    pub fn valid_string_but_in_uppercase_should_parses_to_command_in_lowercase() {
        let command = Command::try_from("GET KEY".to_string()).unwrap();
        assert_eq!(command, Command::Get("key".to_owned()));

        let command = Command::try_from("SET key Value".to_string()).unwrap();
        assert_eq!(command, Command::Set("key".to_owned(), "value".to_owned()));

        let command = Command::try_from("DEL key".to_string()).unwrap();
        assert_eq!(command, Command::Delete("key".to_owned()));
    }

    #[test]
    #[should_panic]
    pub fn invalid_string_should_result_in_err() {
        Command::try_from("invalid command".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    pub fn get_command_without_key_should_result_in_err() {
        Command::try_from("get".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    pub fn set_command_without_key_should_result_in_err() {
        Command::try_from("set".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    pub fn set_command_without_value_should_result_in_err() {
        Command::try_from("set key".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    pub fn del_command_without_key_should_result_in_err() {
        Command::try_from("del".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    pub fn empty_string_should_result_in_err() {
        Command::try_from("".to_string()).unwrap();
    }
}

#[cfg(test)]
mod extract_request_from_command {
    use crate::{Command, Extract, Request};

    #[test]
    pub fn get_command_should_be_properly_converted_to_request() {
        let mut command = Command::Get("key".to_owned());
        let request = command.extract().unwrap();

        let mut expected_payload: Vec<u8> = Vec::new();
        expected_payload.append(&mut 2_u32.to_le_bytes().into());
        expected_payload.append(&mut 3_u32.to_le_bytes().into());
        expected_payload.append(&mut "get".into());
        expected_payload.append(&mut 3_u32.to_le_bytes().into());
        expected_payload.append(&mut "key".into());

        let expected_request = Request::new_with_payload(expected_payload);

        assert_eq!(request, expected_request);
    }

    #[test]
    pub fn set_command_should_be_properly_converted_to_request() {
        let mut command = Command::Set("key".to_owned(), "value".to_owned());
        let request = command.extract().unwrap();

        let mut expected_payload: Vec<u8> = Vec::new();
        expected_payload.append(&mut 3_u32.to_le_bytes().into());
        expected_payload.append(&mut 3_u32.to_le_bytes().into());
        expected_payload.append(&mut "set".into());
        expected_payload.append(&mut 3_u32.to_le_bytes().into());
        expected_payload.append(&mut "key".into());
        expected_payload.append(&mut 5_u32.to_le_bytes().into());
        expected_payload.append(&mut "value".into());

        let expected_request = Request::new_with_payload(expected_payload);

        assert_eq!(request, expected_request);
    }

    #[test]
    pub fn del_command_should_be_properly_converted_to_request() {
        let mut command = Command::Delete("key".to_owned());
        let request = command.extract().unwrap();

        let mut expected_payload: Vec<u8> = Vec::new();
        expected_payload.append(&mut 2_u32.to_le_bytes().into());
        expected_payload.append(&mut 3_u32.to_le_bytes().into());
        expected_payload.append(&mut "del".into());
        expected_payload.append(&mut 3_u32.to_le_bytes().into());
        expected_payload.append(&mut "key".into());

        let expected_request = Request::new_with_payload(expected_payload);

        assert_eq!(request, expected_request);
    }
}
