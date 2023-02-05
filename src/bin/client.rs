use std::io::Write;
use std::os::unix::net::UnixStream;
use std::{env, process};

use skaja::Commands;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Needs a command.");
        process::exit(1);
    }

    let command = match args[1].as_str() {
        "get" => Commands::Get(Some(args[2].to_owned())),
        "set" => Commands::Set(Some((args[2].to_owned(), args[3].to_owned()))),
        "del" => Commands::Delete(Some(args[2].to_owned())),
        _ => {
            eprintln!("Command not recognized.");
            process::exit(1);
        }
    };

    let mut stream = UnixStream::connect("mysocket").expect("Failed connecting to socket.");

    let command_args = match command.arguments() {
        Some(args) => args,
        None => {
            eprintln!("Command {} needs argument(s).", command.as_str());
            process::exit(1);
        }
    };

    let nstr: u32 = command_args.len() as u32;
    let mut payload = nstr.to_ne_bytes().to_vec();

    let command_str = command.as_str();
    let command_len_in_4b = command_str.len() as u32;
    payload.append(&mut command_len_in_4b.to_ne_bytes().to_vec());
    payload.append(&mut command_str.as_bytes().to_vec());

    println!("command args: {:?}", command_args);
    for value in command_args {
        let value_len_in_4b = value.len();
        payload.append(&mut value_len_in_4b.to_ne_bytes().to_vec());
        payload.append(&mut value.as_bytes().to_vec());
    }

    stream.write(&payload).expect("Failed writing to stream.");
}
