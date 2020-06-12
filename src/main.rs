extern crate actix;
extern crate actix_multipart;
extern crate actix_rt;
extern crate actix_web;
extern crate futures;
extern crate rusqlite;
extern crate serde;

mod app_state;
mod availability_actor;
mod loops;
mod ping;
mod server;

use actix::prelude::*;
use app_state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[actix_rt::main]
async fn main() {
    let manager_addr = String::from("http://localhost:3000");
    let monitor_addr = get_monitor_addr(&manager_addr).await;
    println!("Assigned to monitor on address {}", monitor_addr);
    let app_state = Arc::new(
        AppState::new(
            manager_addr,
            monitor_addr,
            8080,
            String::from("./files"),
            20000000,
            120000,
            String::from("Germany"),
        )
        .start(),
    );
    // let server_fut = server::start_server(app_state.clone());
    let ping_fut = loops::start_ping(app_state.clone());
    let sync_fut = loops::start_syncing(app_state.clone());

    println!("Services started");

    // let _ = tokio::try_join!(server_fut);
    let _ = tokio::try_join!(ping_fut, sync_fut);
    actix::System::current().stop();
}

#[derive(Serialize, Deserialize)]
struct RegisterBody {
    value: i32,
}

#[derive(Serialize, Deserialize)]
struct RegisterResponse {
    monitor: String,
}
async fn get_monitor_addr(manager_addr: &str) -> String {
    let url = format!("{}/register", manager_addr);
    let rb = RegisterBody { value: 1337 };
    let response = reqwest::Client::new().post(&url).json(&rb).send().await;

    match response {
        Ok(r) => {
            if let reqwest::StatusCode::OK = r.status() {
                let rr = r.json::<RegisterResponse>().await.unwrap();
                return rr.monitor;
            } else {
                panic!("Problems with server response");
            }
        }
        Err(_err) => panic!("Failed to register!"),
    }
}
