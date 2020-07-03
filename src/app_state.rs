use serde::Serialize;
use std::sync::RwLock;

use crate::file_store::{FileStore, FileStoreFunc};
use crate::config_store::{ConfigStore, ConfigStoreFunc};

#[derive(Serialize)]
pub struct Ping {
    pub fingerprint: String,
    pub port: u16,
    pub weight: f32,
    pub files: Vec<String>,
}

pub struct AppState {
    pub file_store: RwLock<FileStore>,
    pub config_store: RwLock<ConfigStore>,
}

impl AppState {
    pub fn new(manager_addr: &str, monitor_addr: &str, port: u16, fingerprint: &str) -> AppState {
        let file_store = RwLock::new(FileStore::new());
        let config_store = RwLock::new(ConfigStore::new(
            manager_addr.clone(),
            monitor_addr.clone(),
            port.clone(),
            fingerprint.clone(),
        ));

        AppState {
            file_store,
            config_store,
        }
    }

    pub fn generate_ping(&self) -> Ping {
        let config = self.config_store.read().unwrap();
        let ping = Ping {
            fingerprint: config.fingerprint(),
            port: config.port(),
            weight: 0.5,
            files: self.file_store.read().unwrap().hashes(),
        };

        return ping;
    }
}