use serde::Serialize;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock};

use crate::config::ConfigFromFile;
use crate::config_store::{ConfigStore, ConfigStoreFunc, Monitor};
use crate::file_store::{FileStore, FileStoreFunc};
use crate::stat_store::{StatStore, StatStoreFunc};
#[derive(Serialize)]
pub struct Ping {
    pub fingerprint: String,
    pub port: u16,
    pub weight: f32,
    pub files: Vec<String>,
    pub rejected_hashes: Vec<String>,
    pub capacity_left: u32,
    pub uploaded_hashes: Vec<String>,
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
        config: ConfigFromFile,
        monitor_addr: &str,
        monitors: Vec<Monitor>,
        stop_services: Arc<AtomicBool>,
        force_ping: Arc<AtomicBool>,
    ) -> AppState {
        let path = format!("./files/{}", &config.fingerprint);

        let file_store = RwLock::new(FileStore::new(config.stats.capacity.value, &path));
        let config_store = RwLock::new(ConfigStore::new(
            &config.manager_addr,
            monitor_addr.clone(),
            monitors,
            config.port,
            &config.fingerprint,
        ));
        let stat_store = RwLock::new(StatStore {
            stats: config.stats,
        });

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
            files: self.file_store.read().unwrap().hashes().unwrap(),
            capacity_left,
            rejected_hashes: self.file_store.read().unwrap().rejected_hashes(),
            uploaded_hashes: self.file_store.read().unwrap().uploaded_hashes(),
        };

        self.file_store.write().unwrap().clear_uploaded_hashes();

        return ping;
    }

    pub fn add_new_file(&self, content: &[u8], distribute: bool) -> String {
        let hash = self.config_store.write().unwrap().hash_content(content);
        let fingerprint = self.config_store.read().unwrap().fingerprint();

        self.file_store
            .write()
            .unwrap()
            .save_file(&fingerprint, &hash, &content);

        self.file_store
            .write()
            .unwrap()
            .add_hash_to_uploaded_hashes(&hash);

        if distribute {
            self.file_store
                .write()
                .unwrap()
                .insert_file_to_distribute(&hash);
        }

        self.force_ping.swap(true, Ordering::Relaxed);

        return hash;
    }

    pub fn write_to_disk(&self) {
        let fingerprint = self.config_store.read().unwrap().fingerprint();
        let path = format!("files/{}.json", fingerprint);
        let _ = self.file_store.read().unwrap().save_files(&path);
    }

    fn calculate_weight(&self) -> f32 {
        let usage = self.file_store.read().unwrap().capacity_left();
        self.stat_store.read().unwrap().total_rating(usage)
    }
}
