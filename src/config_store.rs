use crypto::{digest::Digest, sha1::Sha1};
use serde::{Deserialize, Serialize};

pub struct ConfigStore {
    manager_addr: String,
    own_monitor: Monitor,
    monitors: Vec<Monitor>,
    port: u16,
    fingerprint: String,
    pub ipv6: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Monitor {
    pub addr: String,
    pub bound: Vec<String>,
}

impl PartialEq for Monitor {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

pub trait ConfigStoreFunc {
    fn new(
        manager_addr: &str,
        own_monitor: Monitor,
        monitors: Vec<Monitor>,
        port: u16,
        fingerprint: &str,
        ipv6: Option<String>,
    ) -> ConfigStore;
    fn monitor(&self) -> Monitor;
    fn monitors(&self) -> Vec<Monitor>;
    fn manager(&self) -> String;
    fn port(&self) -> u16;
    fn fingerprint(&self) -> String;
    fn hash_content(&mut self, content: &[u8]) -> String;
}

impl ConfigStoreFunc for ConfigStore {
    fn new(
        manager_addr: &str,
        own_monitor: Monitor,
        monitors: Vec<Monitor>,
        port: u16,
        fingerprint: &str,
        ipv6: Option<String>,
    ) -> ConfigStore {
        ConfigStore {
            manager_addr: String::from(manager_addr),
            own_monitor,
            monitors,
            port,
            fingerprint: String::from(fingerprint),
            ipv6,
        }
    }

    fn monitor(&self) -> Monitor {
        self.own_monitor.clone()
    }

    fn monitors(&self) -> Vec<Monitor> {
        self.monitors.clone()
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

    fn hash_content(&mut self, content: &[u8]) -> String {
        // TODO: Use Sha256 instead of Sha1
        let mut hasher = Sha1::new();
        hasher.input(content);
        hasher.result_str()
    }
}
