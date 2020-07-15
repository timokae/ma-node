use crate::stat_store::Stats;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ConfigFromFile {
    pub port: u16,
    pub manager_addr: String,
    pub stats: Stats,
}

pub fn parse_config(path: &str) -> ConfigFromFile {
    let data = std::fs::read_to_string(path).expect("Unable to read file");
    let config: ConfigFromFile = serde_json::from_str(&data).expect("JSON was not well-formatted");
    return config;
}
