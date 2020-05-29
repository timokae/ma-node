use actix::prelude::*;

struct Ping {
    port: u16,
    weight: f32,
    files: Vec<String>,
}
// PING
pub struct SendPing();

impl Message for SendPing {
    type Result = usize;
}

impl Handler<SendPing> for AppState {
    type Result = usize;

    fn handle(&mut self, _msg: SendPing, _: &mut Context<Self>) -> Self::Result {
        let ping = Ping {
            port: self.port,
            weight: self.calculate_weight(),
            files: self.files.clone(),
        };
        self.send_ping_to_monitor(ping);
        0
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

// MYACTOR
pub struct AppState {
    file_dir: String,
    file_dir_size: u64,
    disk_space: u64,
    bandwidth: u32,
    location: String,
    files: Vec<String>,
    port: u16,
}

impl AppState {
    pub fn new(
        port: u16,
        file_dir: String,
        disk_space: u64,
        bandwidth: u32,
        location: String,
    ) -> AppState {
        let mut instance = AppState {
            port,
            file_dir,
            file_dir_size: 0,
            disk_space,
            bandwidth: bandwidth / 8,
            location,
            files: Vec::new(),
        };

        instance.files_changed();

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
            Files: {:?}
        ",
            self.disk_usage() * 100.0,
            self.location,
            self.bandwidth * 8,
            self.files,
        );
    }

    fn calculate_weight(&mut self) -> f32 {
        return 0.78;
    }

    fn send_ping_to_monitor(&mut self, ping: Ping) {
        // IMPLEMENT
    }
}

impl Actor for AppState {
    type Context = Context<Self>;
}
