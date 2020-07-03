use std::collections::HashMap;

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

pub trait FileStoreFunc {
  fn new() -> FileStore;
  fn get_file(&self, hash: &str) -> Option<&String>;
  fn insert_file(&mut self, hash: &str, content: &str);
  fn insert_files_to_recover(&mut self, entries: Vec<RecoverEntry>);
  fn next_file_to_recover(&mut self) -> Option<RecoverEntry>;
  fn hashes(&self) -> Vec<String>;
}

impl FileStoreFunc for FileStore {
  fn new() -> FileStore {
      FileStore {
          files_to_sync: vec![],
          files: HashMap::new(),
      }
  }

  fn get_file(&self, hash: &str) -> Option<&String> {
      self.files.get(hash)
  }

  fn insert_file(&mut self, hash: &str, content: &str) {
      self.files.insert(String::from(hash), String::from(content));
      println!("{:?}", self.files);
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
}