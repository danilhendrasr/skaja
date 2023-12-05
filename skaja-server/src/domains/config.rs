use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub address: Option<String>,
}
