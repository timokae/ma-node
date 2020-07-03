// use parking_lot::RwLock;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

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

    // pub fn insert_new_file(&mut self, content: &str) -> String {
    //     let hash = self.hash_content(content);

    //     self.file_store.write().unwrap().insert_file(&hash, content);
    //     // self.file_store.insert_file(&hash, content);
    //     return hash;
    // }

    pub fn generate_ping(&self) -> Ping {
        let config = self.config_store.read().unwrap();
        let ping = Ping {
            fingerprint: config.fingerprint.clone(),
            port: config.port,
            weight: 0.5,
            files: self.file_store.read().unwrap().hashes(),
        };

        return ping;
    }
}

#[derive(Debug)]
pub struct RecoverEntry {
    pub hash: String,
    pub last_checked: chrono::DateTime<chrono::Utc>,
}

impl RecoverEntry {
    pub fn waited_enough(&self) -> bool {
        let t1 = self.last_checked.timestamp();
        let t2 = chrono::Utc::now().timestamp();
        let dif = t2 - t1;
        dif > 5 * 60
    }
}

pub struct FileStore {
    files_to_sync: Vec<RecoverEntry>,
    files: HashMap<String, String>,
}

impl FileStore {
    pub fn new() -> FileStore {
        FileStore {
            files_to_sync: vec![],
            files: HashMap::new(),
        }
    }

    pub fn get_file(&self, hash: &str) -> Option<&String> {
        self.files.get(hash)
    }

    pub fn insert_file(&mut self, hash: &str, content: &str) {
        self.files.insert(String::from(hash), String::from(content));
        println!("{:?}", self.files);
    }

    pub fn insert_files_to_recover(&mut self, entries: Vec<RecoverEntry>) {
        for entry in entries {
            if !self.files.contains_key(&entry.hash) {
                self.files_to_sync.push(entry);
            }
        }
    }

    pub fn next_file_to_recover(&mut self) -> Option<RecoverEntry> {
        self.files_to_sync
            .iter()
            .position(|entry| entry.waited_enough())
            .and_then(|index| Some(self.files_to_sync.remove(index)))
    }

    pub fn hashes(&self) -> Vec<String> {
        self.files
            .keys()
            .map(|key| String::from(key.clone()))
            .collect()
    }
}

pub struct ConfigStore {
    manager_addr: String,
    monitor_addr: String,
    port: u16,
    fingerprint: String,
    hasher: DefaultHasher,
}

impl ConfigStore {
    pub fn new(
        manager_addr: &str,
        monitor_addr: &str,
        port: u16,
        fingerprint: &str,
    ) -> ConfigStore {
        ConfigStore {
            manager_addr: String::from(manager_addr),
            monitor_addr: String::from(monitor_addr),
            port,
            fingerprint: String::from(fingerprint),
            hasher: DefaultHasher::new(),
        }
    }
    pub fn monitor(&self) -> String {
        self.monitor_addr.clone()
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn hash_content(&mut self, content: &str) -> String {
        content.hash(&mut self.hasher);
        let hash = self.hasher.finish().to_string();
        return hash.clone();
    }
}

// pub struct App {
//     inner: RwLock<StorageInner>,
// }

// impl Storage {
//     pub fn new() -> Arc<Storage> {
//         Arc::new(Storage {
//             inner: RwLock::new(StorageInner {
//                 data_map: HashMap::new(),
//                 foreign_map: HashMap::new(),
//             }),
//         })
//     }

//     // Calculates a hash based on the data-string and saves in in the hash along the given data-stirng
//     // Then in returns the hash.
//     pub fn insert(&self, data: String) -> u64 {
//         let hash = self.calculate_hash(&data);
//         self.inner
//             .write()
//             .unwrap()
//             .data_map
//             .insert(hash, data.clone());

//         let msg = format!("Inserted {} with hash {}", data.clone(), hash);
//         logger::log("Storage", &msg);
//         hash
//     }

//     // Returns the data saved under the given hash
//     // If the has could not be found, it returns None
//     pub fn get(&self, hash: u64) -> Option<String> {
//         match self.inner.read().unwrap().data_map.get(&hash) {
//             Some(value) => Some(value.clone()),
//             _ => None,
//         }
//     }

//     // Returns a ip address for the given hash
//     pub fn get_foreign(&self, hash: u64) -> Option<String> {
//         match self.inner.read().unwrap().foreign_map.get(&hash) {
//             Some(value) => Some(value.clone()),
//             _ => None,
//         }
//     }

//     // Replaces all foreign hashes with the given vector of hashes
//     pub fn insert_foreign(&self, new_hashes: Vec<ForeignHash>) {
//         {
//             let foreign_map = &mut self.inner.write().unwrap().foreign_map;
//             foreign_map.clear();
//             for f_hash in new_hashes {
//                 foreign_map.insert(f_hash.hash.parse::<u64>().unwrap(), f_hash.addr);
//             }
//         }
//         println!("{:?}", self.inner.read().unwrap().foreign_map);
//     }

//     // Returns all local hashes
//     pub fn hashes(&self) -> Vec<String> {
//         self.inner
//             .read()
//             .unwrap()
//             .data_map
//             .keys()
//             .map(|key| key.to_string().clone())
//             .collect()
//     }

//     fn calculate_hash<T: Hash>(&self, t: &T) -> u64 {
//         let mut hasher = DefaultHasher::new();
//         t.hash(&mut hasher);
//         hasher.finish()
//     }
// }
