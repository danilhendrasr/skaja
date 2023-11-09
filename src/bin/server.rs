use skaja::Server;

fn main() -> Result<(), String> {
    Server::new("127.0.0.1:3000")?.listen();
    Ok(())
}
