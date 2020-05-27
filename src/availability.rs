use actix::prelude::*;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result, NO_PARAMS};
use std::fs::OpenOptions;
use std::io::Write;

struct AvailabilityRow {
    started_at: i64,
    last_timestamp: i64,
}

// --- Actor ---
pub struct AvailabilityStat {
    started_at: DateTime<Utc>,
    last_timestamp: DateTime<Utc>,
    conn: Connection,
}

impl AvailabilityStat {
    pub fn new() -> AvailabilityStat {
        let conn = Connection::open("client.db").unwrap();

        let mut instance = AvailabilityStat {
            started_at: Utc::now(),
            last_timestamp: Utc::now(),
            conn,
        };

        let create_table_res = instance.create_table();
        eprintln!("{:?}", create_table_res);
        let new_entry_result = instance.new_entry();
        eprintln!("{:?}", new_entry_result);
        instance
    }

    pub fn since_started(&mut self) -> i64 {
        return self.last_timestamp.timestamp() - self.started_at.timestamp();
    }

    fn create_table(&mut self) -> Result<usize> {
        self.conn.execute(
            "create table if not exists availability(
                id integer primary key,
                started_at integer not null,
                last_timestamp integer not null
            )",
            NO_PARAMS,
        )
    }

    fn log(&mut self) -> Result<usize> {
        let last_row_id = self.conn.last_insert_rowid().to_string();
        self.conn.execute(
            "
            UPDATE availability
            SET started_at = ?1,
                last_timestamp = ?2
            WHERE
                id = ?3;
            ",
            &[
                self.started_at.timestamp().to_string(),
                self.last_timestamp.timestamp().to_string(),
                last_row_id,
            ],
        )
    }

    fn new_entry(&mut self) -> Result<usize> {
        self.conn.execute(
            "
            INSERT INTO availability (started_at, last_timestamp)
            VALUES (?1, ?2);
            ",
            &[
                self.started_at.timestamp().to_string(),
                self.last_timestamp.timestamp().to_string(),
            ],
        )
    }

    fn uptime_stats(&mut self) -> (i64, i64, i64) {
        let last_row_id = self.conn.last_insert_rowid().to_string();
        let mut rows = self
            .conn
            .prepare("SELECT started_at, last_timestamp FROM availability WHERE id < ?1;")
            .unwrap();
        let logs = rows
            .query_map(&[last_row_id], |row| {
                Ok(AvailabilityRow {
                    started_at: row.get(0).unwrap(),
                    last_timestamp: row.get(1).unwrap(),
                })
            })
            .unwrap()
            .flatten()
            .collect::<Vec<AvailabilityRow>>();

        let mut max_uptime = 0;
        let mut min_uptime = std::i64::MAX;
        let mut average_uptime = 0;
        let mut count = 0;

        for log in logs {
            let duration = log.last_timestamp - log.started_at;

            if duration > max_uptime {
                max_uptime = duration;
            } else if duration < min_uptime {
                min_uptime = duration;
            }

            average_uptime += duration;
            count += 1;
        }

        average_uptime = average_uptime / count;

        (average_uptime, min_uptime, max_uptime)
    }

    // fn log_to_file(&mut self, new_line: bool) -> Result<(), std::io::Error> {
    //     let file = OpenOptions::new()
    //         .write(true)
    //         .create(true)
    //         .append(true)
    //         .open("availability.txt");
    //     match file {
    //         Ok(mut file) => {
    //             let content = timestamp.to_string() + "\n";
    //             file.write_all(content.as_bytes())
    //         }
    //         Err(err) => Err(err),
    //     }
    // }
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
        self.last_timestamp = Utc::now();
        println!(
            "{} {}",
            self.started_at.timestamp(),
            self.last_timestamp.timestamp()
        );
        let res = self.log();
        match res {
            Ok(_) => {}
            Err(err) => eprintln!("{}", err),
        }

        let (avg, min, max) = self.uptime_stats();
        println!("{} / {} / {}", avg, min, max);
        // println!("{}", self.since_started());
        // write_timestamp_to_file(self.last_timestamp).expect("Could not write to file");
        0
    }
}

fn timestamp_to_utc(timestamp: i64) -> DateTime<Utc> {
    chrono::DateTime::<Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(timestamp, 0), Utc)
}
