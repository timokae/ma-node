use actix::prelude::*;
use rand::Rng;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
    type Result = Ping;
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
        MessageResult(ping)
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
    pub hashes: Vec<String>,
}

impl Message for UpdateFilesToSync {
    type Result = bool;
}

impl Handler<UpdateFilesToSync> for AppState {
    type Result = bool;

    fn handle(&mut self, msg: UpdateFilesToSync, _ctx: &mut Self::Context) -> Self::Result {
        self.files_to_sync.extend(msg.hashes.iter().cloned());
        println!("{:?}", self.files_to_sync);
        true
    }
}

pub struct NextHash {}
impl Message for NextHash {
    type Result = Option<String>;
}
impl Handler<NextHash> for AppState {
    type Result = Option<String>;

    fn handle(&mut self, _msg: NextHash, ctx: &mut Self::Context) -> Self::Result {
        if self.files_to_sync.is_empty() {
            None
        } else {
            Some(self.files_to_sync.remove(0))
        }
    }
}

// MYACTOR
pub struct AppState {
    fingerprint: String,
    file_dir: String,
    file_dir_size: u64,
    disk_space: u64,
    bandwidth: u32,
    location: String,
    files: Vec<String>,
    port: u16,
    weight: f32,
    files_to_sync: Vec<String>,
}

impl AppState {
    pub fn new(
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
        println!(
            "
            Disk Usage: {:.2}%,
            Location: {},
            Bandwidth: {}Mbit/s,
            Files: {:?},
            Weight: {},
            Files to sync: {:?},
        ",
            self.disk_usage() * 100.0,
            self.location,
            self.bandwidth * 8,
            self.files,
            self.weight,
            self.files_to_sync
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
