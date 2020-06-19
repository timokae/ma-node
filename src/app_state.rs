use actix::prelude::*;
use log::info;
use rand::Rng;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct GeneratedPing {
    pub ping: Ping,
    pub monitor_addr: String,
}

#[derive(Serialize)]
pub struct Ping {
    pub fingerprint: String,
    pub port: u16,
    pub weight: f32,
    pub files: Vec<String>,
}
// PING
pub struct GeneratePingMessage();

impl Message for GeneratePingMessage {
    type Result = GeneratedPing;
}

impl Handler<GeneratePingMessage> for AppState {
    type Result = MessageResult<GeneratePingMessage>;

    fn handle(&mut self, _msg: GeneratePingMessage, _: &mut Context<Self>) -> Self::Result {
        let ping = Ping {
            fingerprint: self.fingerprint.clone(),
            port: self.port,
            weight: self.calculate_weight(),
            files: self.files.clone(),
        };

        let generated_ping = GeneratedPing {
            ping,
            monitor_addr: self.monitor_addr.clone(),
        };

        MessageResult(generated_ping)
    }
}

pub struct FilesChanged();
impl Message for FilesChanged {
    type Result = u64;
}
impl Handler<FilesChanged> for AppState {
    type Result = u64;

    fn handle(&mut self, _msg: FilesChanged, _ctx: &mut Self::Context) -> Self::Result {
        self.files_changed();
        self.file_dir_size
    }
}

pub struct UpdateFilesToSync {
    pub entries: Vec<RecoverEntry>,
}
impl Message for UpdateFilesToSync {
    type Result = bool;
}
impl Handler<UpdateFilesToSync> for AppState {
    type Result = bool;

    fn handle(&mut self, msg: UpdateFilesToSync, _ctx: &mut Self::Context) -> Self::Result {
        for entry in msg.entries {
            if !self.files.contains(&entry.hash) {
                self.files_to_sync.push(entry);
            }
        }
        // self.files_to_sync.extend(msg.hashes.iter().cloned());
        self.print_state();
        true
    }
}

pub struct RecoveredFile {
    pub hash: String,
}
impl Message for RecoveredFile {
    type Result = bool;
}
impl Handler<RecoveredFile> for AppState {
    type Result = bool;
    fn handle(&mut self, msg: RecoveredFile, _ctx: &mut Self::Context) -> Self::Result {
        self.files.push(msg.hash);
        true
    }
}

pub struct NextFileToRecover {}
impl Message for NextFileToRecover {
    type Result = Option<RecoverEntry>;
}
impl Handler<NextFileToRecover> for AppState {
    type Result = Option<RecoverEntry>;

    fn handle(&mut self, _msg: NextFileToRecover, _ctx: &mut Self::Context) -> Self::Result {
        self.files_to_sync
            .iter()
            .position(|entry| entry.waited_enough())
            .and_then(|index| Some(self.files_to_sync.remove(index)))
    }
}

pub struct MonitorAddr {}
impl Message for MonitorAddr {
    type Result = String;
}
impl Handler<MonitorAddr> for AppState {
    type Result = String;

    fn handle(&mut self, _msg: MonitorAddr, _ctx: &mut Self::Context) -> Self::Result {
        return self.monitor_addr.clone();
    }
}

#[derive(Debug)]
pub struct RecoverEntry {
    pub hash: String,
    pub last_checked: chrono::DateTime<chrono::Utc>,
}

impl RecoverEntry {
    fn waited_enough(&self) -> bool {
        let t1 = self.last_checked.timestamp();
        let t2 = chrono::Utc::now().timestamp();
        let dif = t2 - t1;
        dif > 5 * 60
    }
}

#[allow(dead_code)]
// MYACTOR
pub struct AppState {
    manager_addr: String,
    monitor_addr: String,
    fingerprint: String,
    file_dir: String,
    file_dir_size: u64,
    disk_space: u64,
    bandwidth: u32,
    location: String,
    files: Vec<String>,
    port: u16,
    weight: f32,
    files_to_sync: Vec<RecoverEntry>,
}

#[allow(dead_code)]
impl AppState {
    pub fn new(
        manager_addr: String,
        monitor_addr: String,
        port: u16,
        file_dir: String,
        disk_space: u64,
        bandwidth: u32,
        location: String,
    ) -> AppState {
        let mut rng = rand::thread_rng();
        let weight = rng.gen_range(0.0, 1.0);
        let fingerprint = format!("node-{}", rng.gen::<u32>());

        // let num_files = (weight * 10.0) as i8;

        let mut instance = AppState {
            manager_addr,
            monitor_addr,
            fingerprint: fingerprint.clone(),
            port,
            file_dir,
            file_dir_size: 0,
            disk_space,
            bandwidth: bandwidth / 8,
            location,
            files: generate_random_file_names(2, fingerprint.clone()),
            weight,
            files_to_sync: vec![],
        };

        instance.print_state();
        // instance.files_changed();

        instance
    }

    fn files_changed(&mut self) {
        let path = std::path::Path::new(self.file_dir.as_str());
        let file_iter = std::fs::read_dir(path).unwrap();

        let mut total_file_size = 0;
        self.files.clear();
        for file in file_iter {
            let file = file.unwrap();
            let file_size = std::fs::metadata(file.path()).unwrap().len();
            total_file_size += file_size;

            self.files.push(file.file_name().into_string().unwrap());
        }

        self.file_dir_size = total_file_size;
        self.print_state();
    }

    fn disk_usage(&mut self) -> f32 {
        return (self.file_dir_size as f64 / self.disk_space as f64) as f32;
    }

    fn print_state(&mut self) {
        info!(
            "\n\
            \tFingerprint: {}\n\
            \tFiles: {:#?}\n\
            \tTo Sync: {:#?}\
        ",
            self.fingerprint, self.files, self.files_to_sync
        );
    }

    fn calculate_weight(&mut self) -> f32 {
        return self.weight;
    }
}

impl Actor for AppState {
    type Context = Context<Self>;
}

fn generate_random_file_names(n: i8, seed: String) -> Vec<String> {
    let mut files: Vec<String> = vec![];
    let mut hasher = DefaultHasher::new();
    for i in 0..n {
        let tmp = format!("{}{}", seed, i);
        tmp.hash(&mut hasher);
        let file_hash = hasher.finish().to_string();
        files.push(file_hash);
    }

    return files;
}
