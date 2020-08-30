use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

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

#[derive(Deserialize, Serialize, Debug)]
pub struct FileEntry {
    pub hash: String,
    pub file_name: String,
    pub content_type: String,
    path: String,
}

impl FileEntry {
    pub fn content(&self) -> Option<Vec<u8>> {
        match std::fs::read(&self.path) {
            Ok(content) => Some(content),
            Err(_err) => None,
        }
    }
}

pub struct FileStore {
    path: String,
    files_to_sync: Vec<RecoverEntry>,
    files_to_distribute: Vec<String>,
    files: HashMap<String, FileEntry>,
    capacity: u32,
    rejected_hashes: Vec<String>,
    new_hashes: Vec<String>,
}

pub trait FileStoreFunc {
    fn new(capacity: u32, path: &str) -> FileStore;
    fn get_file(&self, hash: &str) -> Option<&FileEntry>;
    fn save_file(&mut self, hash: &str, content: &[u8], content_type: &str, file_name: &str);
    fn remove_file(&mut self, hash: &str);
    fn insert_files_to_recover(&mut self, entries: Vec<RecoverEntry>);
    fn next_file_to_recover(&mut self) -> Option<RecoverEntry>;
    fn insert_file_to_distribute(&mut self, hash: &str);
    fn next_file_to_distribute(&mut self) -> Option<String>;
    fn hashes(&self) -> Vec<String>;
    fn capacity_left(&self) -> u32;
    fn reject_hash(&mut self, hash: &str);
    fn rejected_hashes(&self) -> Vec<String>;
    fn clear_rejected_hashes(&mut self);
    fn serialize_state(&self);
    fn deserialize_state(path: &str) -> HashMap<String, FileEntry>;
    fn uploaded_hashes(&self) -> Vec<String>;
    fn add_hash_to_uploaded_hashes(&mut self, hash: &str);
    fn clear_uploaded_hashes(&mut self);
}

impl FileStoreFunc for FileStore {
    fn new(capacity: u32, path: &str) -> FileStore {
        let file_state_path = format!("{}/file_state.json", path);
        let files = FileStore::deserialize_state(&file_state_path);
        let tmp = files
            .values()
            .map(|fe| format!("{}: {}", &fe.hash, &fe.file_name))
            .collect::<Vec<String>>();
        info!("FileStore initialized: {:?}", tmp);

        if let Err(err) = std::fs::create_dir_all(path) {
            if !std::path::Path::new(path).exists() {
                panic!("{}", err);
            }
        }

        FileStore {
            path: String::from(path),
            files_to_sync: vec![],
            files_to_distribute: vec![],
            files,
            capacity,
            rejected_hashes: vec![],
            new_hashes: vec![],
        }
    }

    fn get_file(&self, hash: &str) -> Option<&FileEntry> {
        self.files.get(hash)
    }

    fn remove_file(&mut self, hash: &str) {
        if let Some(file_entry) = self.files.get(hash) {
            match std::fs::remove_file(&file_entry.path) {
                Ok(_) => {
                    self.files.remove(hash);
                    info!("Removed {}", hash);
                }
                Err(err) => error!("{}", err),
            }
        }
    }

    fn save_file(&mut self, hash: &str, content: &[u8], content_type: &str, file_name: &str) {
        // Create dir if not exist
        let file_dir = std::path::Path::new(&self.path).join("files");
        if !file_dir.exists() {
            let _ = std::fs::create_dir_all(file_dir);
        }
        // Create physical file
        let file_path = format!("{}/files/{}", self.path, hash);
        let mut file = std::fs::File::create(&file_path).unwrap();
        let _res = file.write_all(content);

        // Create file entry
        let file_entry = FileEntry {
            hash: String::from(hash),
            file_name: String::from(file_name),
            content_type: String::from(content_type),
            path: String::from(&file_path),
        };
        self.files.insert(String::from(hash), file_entry);
    }

    fn insert_files_to_recover(&mut self, entries: Vec<RecoverEntry>) {
        for entry in entries {
            if !self.files.contains_key(&entry.hash) {
                self.files_to_sync.push(entry);
            }
        }
    }

    fn next_file_to_recover(&mut self) -> Option<RecoverEntry> {
        self.files_to_sync
            .iter()
            .position(|entry| entry.waited_enough())
            .and_then(|index| Some(self.files_to_sync.remove(index)))
    }

    fn insert_file_to_distribute(&mut self, hash: &str) {
        self.files_to_distribute.push(String::from(hash));
    }

    fn next_file_to_distribute(&mut self) -> Option<String> {
        if self.files_to_distribute.is_empty() {
            return None;
        }

        Some(self.files_to_distribute.remove(0))
    }

    fn hashes(&self) -> Vec<String> {
        self.files
            .keys()
            .clone()
            .map(|k| String::from(k))
            .collect::<Vec<String>>()
    }

    fn capacity_left(&self) -> u32 {
        let stored_files = self.files.len() as u32;
        let to_be_stored_files = self.files_to_sync.len() as u32;
        self.capacity - stored_files - to_be_stored_files
    }

    fn reject_hash(&mut self, hash: &str) {
        self.rejected_hashes.push(String::from(hash));
    }

    fn rejected_hashes(&self) -> Vec<String> {
        self.rejected_hashes.clone()
    }

    fn clear_rejected_hashes(&mut self) {
        self.rejected_hashes.clear();
    }

    fn serialize_state(&self) {
        let path = format!("{}/file_state.json", self.path);
        let serialized = serde_json::to_string(&self.files).unwrap();
        let mut file = File::create(path).unwrap();
        let _ = file.write_all(serialized.as_bytes());
    }

    fn deserialize_state(path: &str) -> HashMap<String, FileEntry> {
        let result = File::open(path)
            .and_then(|mut file| {
                let mut contents = String::new();
                let _ = file.read_to_string(&mut contents);
                Ok(contents)
            })
            .and_then(|content| {
                let files: HashMap<String, FileEntry> = serde_json::from_str(&content).unwrap();
                Ok(files)
            });

        match result {
            Ok(files) => files,
            Err(_err) => HashMap::new(),
        }
    }

    fn uploaded_hashes(&self) -> Vec<String> {
        self.new_hashes.clone()
    }

    fn add_hash_to_uploaded_hashes(&mut self, hash: &str) {
        self.new_hashes.push(String::from(hash));
    }

    fn clear_uploaded_hashes(&mut self) {
        self.new_hashes.clear();
    }
}
