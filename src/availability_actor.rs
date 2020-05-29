use actix::prelude::*;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result, NO_PARAMS};

struct AvailabilityRow {
    started_at: i64,
    last_timestamp: i64,
}

// --- Actor ---
pub struct AvailabilityActor {
    started_at: DateTime<Utc>,
    last_timestamp: DateTime<Utc>,
    uptime: (i64, i64, i64),
    conn: Connection,
    interval: i64,
}

impl AvailabilityActor {
    pub fn new() -> AvailabilityActor {
        let conn = Connection::open("client.db").unwrap();

        let mut instance = AvailabilityActor {
            started_at: Utc::now(),
            last_timestamp: Utc::now(),
            uptime: (0, 0, 0),
            conn,
            interval: 10,
        };
        let _create_table_res = instance.create_table();
        // eprintln!("{:?}", create_table_res);
        let _new_entry_result = instance.new_entry();
        // eprintln!("{:?}", new_entry_result);
        let _log_res = instance.log();
        // eprintln!("{:?}", log_res);
        instance
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
        let rows = self
            .conn
            .prepare("SELECT started_at, last_timestamp FROM availability WHERE id < ?1;")
            .unwrap()
            .query_map(&[last_row_id], |row| {
                Ok(AvailabilityRow {
                    started_at: row.get(0).unwrap(),
                    last_timestamp: row.get(1).unwrap(),
                })
            })
            .unwrap()
            .flatten()
            .collect::<Vec<AvailabilityRow>>();

        AvailabilityActor::find_avg_min_max(rows)
    }

    fn find_avg_min_max(rows: Vec<AvailabilityRow>) -> (i64, i64, i64) {
        let mut max_uptime = 0;
        let mut min_uptime = std::i64::MAX;
        let mut total_uptime = 0;
        let mut count = 0;

        for row in rows {
            let duration = row.last_timestamp - row.started_at;

            if duration > max_uptime {
                max_uptime = duration;
            } else if duration < min_uptime {
                min_uptime = duration;
            }

            total_uptime += duration;
            count += 1;
        }

        let average_uptime = total_uptime / count;

        (average_uptime, min_uptime, max_uptime)
    }
}

impl Actor for AvailabilityActor {
    type Context = Context<Self>;
}

// --- Messages ---
pub struct Trigger();

impl Message for Trigger {
    type Result = usize;
}

impl Handler<Trigger> for AvailabilityActor {
    type Result = usize;

    fn handle(&mut self, _: Trigger, _: &mut Context<Self>) -> Self::Result {
        let time_sinced_last_trigger = elapsed_time(self.last_timestamp, Utc::now());
        if time_sinced_last_trigger < self.interval {
            return 1;
        }
        self.last_timestamp = Utc::now();
        let _res = self.log();
        // eprintln!("{:?}", res);

        self.uptime = self.uptime_stats();
        0
    }
}

fn elapsed_time(dt1: DateTime<Utc>, dt2: DateTime<Utc>) -> i64 {
    return (dt1.timestamp() - dt2.timestamp()).abs();
}

// fn timestamp_to_utc(timestamp: i64) -> DateTime<Utc> {
//     chrono::DateTime::<Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(timestamp, 0), Utc)
// }

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
