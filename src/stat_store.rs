use chrono::{Datelike, Local, TimeZone};
use log::error;
use serde::Deserialize;
#[derive(Deserialize)]
pub struct Stat<T> {
    pub value: T,
    pub weight: f32,
}

#[derive(Deserialize)]
pub struct Stats {
    pub region: String,
    pub uptime: Stat<Vec<u32>>,
    pub capacity: Stat<u32>,
    pub connection: Stat<u32>,
}

#[derive(Deserialize)]
pub struct StatStore {
    pub stats: Stats,
}

pub trait StatStoreFunc {
    fn total_rating(&self, capacity_left: u32) -> f32;
    fn connection_rating(&self) -> f32;
    fn capacity_rating(&self, capacity_left: u32) -> f32;
    fn uptime_rating(&self) -> f32;
    fn uptime_left_rating(&self) -> f32;
}

impl StatStoreFunc for StatStore {
    fn total_rating(&self, capacity_left: u32) -> f32 {
        self.connection_rating()
            + self.capacity_rating(capacity_left)
            + self.uptime_rating()
            + self.uptime_left_rating()
    }

    fn connection_rating(&self) -> f32 {
        let speed = self.stats.connection.value;
        let speed_rating: f32;

        if speed < 6000 {
            speed_rating = 0.1;
        } else if speed >= 6000 && speed < 16000 {
            speed_rating = 0.3;
        } else if speed >= 16000 && speed < 50000 {
            speed_rating = 0.4;
        } else if speed >= 50000 && speed < 200000 {
            speed_rating = 0.6;
        } else if speed >= 200000 && speed < 1000000 {
            speed_rating = 0.8;
        } else {
            speed_rating = 1.0;
        }

        // error!(
        //     "Connection Rating: {}",
        //     speed_rating * self.stats.connection.weight,
        // );

        return speed_rating * self.stats.connection.weight;
    }

    fn uptime_rating(&self) -> f32 {
        let up = self.stats.uptime.value[0] as f32;
        let down = self.stats.uptime.value[1] as f32;
        let uptime_in_hours = down - up;

        // error!(
        //     "Uptime Rating: {}",
        //     (uptime_in_hours / 24.0) * self.stats.uptime.weight,
        // );

        (uptime_in_hours / 24.0) * self.stats.uptime.weight
    }

    fn uptime_left_rating(&self) -> f32 {
        let now = Local::now();
        let total_uptime_in_minutes =
            ((self.stats.uptime.value[1] - self.stats.uptime.value[0]) * 60) as f32;
        let down =
            Local
                .ymd(now.year(), now.month(), now.day())
                .and_hms(self.stats.uptime.value[1], 0, 0);
        let minutes_left = down.signed_duration_since(now).num_minutes() as f32;

        // error!(
        //     "UptimeLeft Rating: {}",
        //     (minutes_left / total_uptime_in_minutes) * self.stats.uptime.weight
        // );

        (minutes_left / total_uptime_in_minutes) * self.stats.uptime.weight
    }

    fn capacity_rating(&self, capacity_left: u32) -> f32 {
        // error!(
        //     "Capacity Rating: {}",
        //     (capacity_left as f32 / self.stats.capacity.value as f32) * self.stats.capacity.weight
        // );
        (capacity_left as f32 / self.stats.capacity.value as f32) * self.stats.capacity.weight
    }
}

impl StatStore {}
