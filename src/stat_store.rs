use serde::Deserialize;
#[derive(Deserialize)]
pub struct Stats {
    pub region: String,
    pub uptime: Vec<u32>,
    pub capacity: usize,
}

#[derive(Deserialize)]
pub struct StatStore {
    pub stats: Stats,
}

// pub trait StatStoreFunc {
//     fn from_file(path: &str);
// }

impl Stats {
    pub fn from_file(path: &str) -> Stats {
        let data = std::fs::read_to_string(path).expect("Unable to read file");
        let stats: Stats = serde_json::from_str(&data).expect("JSON was not well-formatted");
        return stats;
    }
}
