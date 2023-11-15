use std::{env::Args, io};

use super::{ReadToRequest, Request};

/// The commands that can be sent to the server.
#[derive(Debug, PartialEq)]
pub enum Command {
    Get(String),
    Set(String, String),
    Delete(String),
}

impl Command {
    /// Returns the command as a string.
    pub fn as_str(&self) -> &str {
        match self {
            Command::Get(_) => "get",
            Command::Set(_, _) => "set",
            Command::Delete(_) => "del",
        }
    }

    /// Returns the arguments for the command.
    pub fn arguments(&self) -> Vec<&str> {
        match self {
            Command::Set(key, val) => vec![key, val],
            Command::Get(key) => vec![key],
            Command::Delete(key) => vec![key],
        }
    }
}

impl TryFrom<String> for Command {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let splitted_string = value.split(' ').collect::<Vec<&str>>();

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

impl ReadToRequest for Command {
    fn read_to_request(&mut self) -> Result<Request, io::Error> {
        let mut payload: Vec<u8> = Vec::new();

        let command_str = self.as_str();

        // The length of the command string, truncated to 32bit.
        let command_len = command_str.len() as u32;

        // Command + the number of arguments for the command
        let command_args = self.arguments();
        let header = 1 + command_args.len() as u32;

        payload.append(&mut header.to_ne_bytes().into());
        payload.append(&mut command_len.to_ne_bytes().into());
        payload.append(&mut command_str.into());

        for arg in command_args {
            let arg_len = arg.len() as u32;
            payload.append(&mut arg_len.to_ne_bytes().into());
            payload.append(&mut arg.into());
        }

        Ok(Request::new_with_payload(payload))
    }
}
