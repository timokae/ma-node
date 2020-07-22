use serde::Serialize;
use std::sync::{atomic::AtomicBool, Arc, RwLock};

use crate::config_store::{ConfigStore, ConfigStoreFunc};
use crate::file_store::{FileStore, FileStoreFunc};
use crate::stat_store::{StatStore, StatStoreFunc, Stats};
#[derive(Serialize)]
pub struct Ping {
    pub fingerprint: String,
    pub port: u16,
    pub weight: f32,
    pub files: Vec<String>,
    pub rejected_hashes: Vec<String>,
    pub capacity_left: u32,
}

pub struct AppState {
    pub file_store: RwLock<FileStore>,
    pub config_store: RwLock<ConfigStore>,
    pub stat_store: RwLock<StatStore>,
    pub stop_services: Arc<AtomicBool>,
    pub force_ping: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(
        manager_addr: &str,
        monitor_addr: &str,
        port: u16,
        fingerprint: &str,
        stats: Stats,
        stop_services: Arc<AtomicBool>,
        force_ping: Arc<AtomicBool>,
    ) -> AppState {
        let file_store = RwLock::new(FileStore::new(stats.capacity.value));
        let config_store = RwLock::new(ConfigStore::new(
            manager_addr.clone(),
            monitor_addr.clone(),
            port.clone(),
            fingerprint.clone(),
        ));
        let stat_store = RwLock::new(StatStore { stats });

        AppState {
            file_store,
            config_store,
            stat_store,
            stop_services,
            force_ping,
        }
    }

    pub fn generate_ping(&self) -> Ping {
        let config = self.config_store.read().unwrap();

        let capacity_left = self.file_store.read().unwrap().capacity_left();

        let ping = Ping {
            fingerprint: config.fingerprint(),
            port: config.port(),
            weight: self.calculate_weight(),
            files: self.file_store.read().unwrap().hashes(),
            capacity_left,
            rejected_hashes: self.file_store.read().unwrap().rejected_hashes(),
        };

        return ping;
    }

    fn calculate_weight(&self) -> f32 {
        let usage = self.file_store.read().unwrap().capacity_left();
        self.stat_store.read().unwrap().total_rating(usage)
    }
}
