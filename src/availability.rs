use actix::prelude::*;
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;

// --- Actor ---
pub struct AvailabilityStat {
    last_timestamp: i64,
}

impl AvailabilityStat {
    pub fn new() -> AvailabilityStat {
        AvailabilityStat {
            last_timestamp: Utc::now().timestamp(),
        }
    }
}

impl Actor for AvailabilityStat {
    type Context = Context<Self>;
}

// --- Messages ---
pub struct Trigger();

impl Message for Trigger {
    type Result = usize;
}

impl Handler<Trigger> for AvailabilityStat {
    type Result = usize;

    fn handle(&mut self, _: Trigger, _: &mut Context<Self>) -> Self::Result {
        self.last_timestamp = Utc::now().timestamp();
        write_timestamp_to_file(self.last_timestamp).expect("Could not write to file");
        0
    }
}

fn write_timestamp_to_file(timestamp: i64) -> Result<(), std::io::Error> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open("availability.txt");

    match file {
        Ok(mut file) => {
            let content = timestamp.to_string() + "\n";
            file.write_all(content.as_bytes())
        }
        Err(err) => Err(err),
    }
}
