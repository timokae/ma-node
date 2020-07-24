use log::info;
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

pub struct FileStore {
    files_to_sync: Vec<RecoverEntry>,
    files: HashMap<String, String>,
    capacity: u32,
    rejected_hashes: Vec<String>,
    new_hashes: Vec<String>,
}

pub trait FileStoreFunc {
    fn new(capacity: u32, path: &str) -> FileStore;
    fn get_file(&self, hash: &str) -> Option<&String>;
    fn insert_file(&mut self, hash: &str, content: &str);
    fn remove_file(&mut self, hash: &str);
    fn insert_files_to_recover(&mut self, entries: Vec<RecoverEntry>);
    fn next_file_to_recover(&mut self) -> Option<RecoverEntry>;
    fn hashes(&self) -> Vec<String>;
    fn capacity_left(&self) -> u32;
    fn reject_hash(&mut self, hash: &str);
    fn rejected_hashes(&self) -> Vec<String>;
    fn clear_rejected_hashes(&mut self);
    fn save_files(&self, name: &str);
    fn restore_files(path: &str) -> HashMap<String, String>;
    fn uploaded_hashes(&self) -> Vec<String>;
    fn add_hash_to_uploaded_hashes(&mut self, hash: &str);
    fn clear_uploaded_hashes(&mut self);
}

impl FileStoreFunc for FileStore {
    fn new(capacity: u32, path: &str) -> FileStore {
        let files = FileStore::restore_files(path);
        info!("FileStore initialized: {:?}", files);

        FileStore {
            files_to_sync: vec![],
            files,
            capacity,
            rejected_hashes: vec![],
            new_hashes: vec![],
        }
    }

    fn get_file(&self, hash: &str) -> Option<&String> {
        self.files.get(hash)
    }

    fn insert_file(&mut self, hash: &str, content: &str) {
        self.files.insert(String::from(hash), String::from(content));
        println!("{:?}", self.files);
    }

    fn remove_file(&mut self, hash: &str) {
        info!("Removing {}", hash);
        self.files.remove(hash);
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

    fn hashes(&self) -> Vec<String> {
        self.files
            .keys()
            .map(|key| String::from(key.clone()))
            .collect()
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

    fn save_files(&self, path: &str) {
        let serialized = serde_json::to_string(&self.files).unwrap();
        let mut file = File::create(path).unwrap();
        let _ = file.write_all(serialized.as_bytes());
    }

    fn restore_files(path: &str) -> HashMap<String, String> {
        // let mut file = File::open(path)?;
        info!("{}", path);
        let result = File::open(path)
            .and_then(|mut file| {
                let mut contents = String::new();
                let _ = file.read_to_string(&mut contents);
                info!("{:?}", contents);
                Ok(contents)
            })
            .and_then(|content| {
                let files: HashMap<String, String> = serde_json::from_str(&content).unwrap();
                info!("{:?}", files);
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
