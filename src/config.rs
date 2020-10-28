use serde::Deserialize;

#[derive(Deserialize)]
pub struct ConfigFromFile {
    pub fingerprint: String,
    pub port: u16,
    pub manager_addr: String,
}

pub fn parse_config(path: &str) -> ConfigFromFile {
    let complete_path = format!("{}/config.json", path);
    let data = std::fs::read_to_string(&complete_path).expect("Unable to read file");
    let config: ConfigFromFile = serde_json::from_str(&data).expect("JSON was not well-formatted");
    return config;
}
