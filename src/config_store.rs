use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct ConfigStore {
  manager_addr: String,
  monitor_addr: String,
  port: u16,
  fingerprint: String,
  hasher: DefaultHasher,
}

pub trait ConfigStoreFunc {
    fn new(
        manager_addr: &str,
        monitor_addr: &str,
        port: u16,
        fingerprint: &str,
    ) -> ConfigStore;
    fn monitor(&self) -> String;
    fn manager(&self) -> String;
    fn port(&self) -> u16;
    fn fingerprint(&self) -> String;
    fn hash_content(&mut self, content: &str) -> String;
}

impl ConfigStoreFunc for ConfigStore {
    fn new(
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
    
    fn monitor(&self) -> String {
        self.monitor_addr.clone()
    }

    fn manager(&self) -> String {
        self.manager_addr.clone()
    }

    fn port(&self) -> u16 {
        self.port
    }

    fn fingerprint(&self) -> String {
        self.fingerprint.clone()
    }

    fn hash_content(&mut self, content: &str) -> String {
        content.hash(&mut self.hasher);
        let hash = self.hasher.finish().to_string();
        return hash.clone();
  }
}