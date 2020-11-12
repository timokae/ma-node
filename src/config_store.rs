use crypto::{digest::Digest, sha1::Sha1};
use serde::{Deserialize, Serialize};

/*
 * This store saves all critical configs, like all kinds of monitors or the port.
 */

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
        manager_addr: &str,         // Address of the manager
        own_monitor: Monitor,       // Assigned monitor
        monitors: Vec<Monitor>,     // All monitors in network
        port: u16,                  // Port the backend server should use
        fingerprint: &str,          // Identifier for this node
        ipv6: Option<String>,       // Optional, must be set when IPv6 is used
    ) -> ConfigStore;
    fn monitor(&self) -> Monitor;           // Return the assigned monitor
    fn monitors(&self) -> Vec<Monitor>;     // Returns all monitors
    fn manager(&self) -> String;            // Returns manager address
    fn port(&self) -> u16;                  // Returns port
    fn fingerprint(&self) -> String;        // Returns own fingerprint
    fn hash_content(&mut self, content: &[u8]) -> String; // Returns the hash of a file content
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
