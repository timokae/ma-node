extern crate async_trait;
extern crate ctrlc;
extern crate fern;
extern crate futures;
extern crate log;
extern crate rusqlite;
extern crate serde;

mod app_state;
mod availability_actor;
mod config_store;
mod file_store;
mod ping_service;
mod recover_service;
mod server;
mod service;

use app_state::AppState;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use ping_service::PingService;
use rand::Rng;
use recover_service::RecoverService;
use serde::{Deserialize, Serialize};
// use service::Service;
use std::env;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    setup_logger();

    let keep_running: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    setup_close_handler(keep_running.clone(), shutdown_tx);

    let args: Vec<String> = env::args().collect();
    info!("{:?}", args);
    if args.len() < 2 {
        panic!("Not enough arguments")
    }
    let port: &u16 = &args[1].parse::<u16>().unwrap_or(8080);

    let manager_addr = String::from("http://localhost:3000");
    let monitor_addr = get_monitor_addr(&manager_addr).await;

    let mut rng = rand::thread_rng();
    // let weight = rng.gen_range(0.0, 1.0);
    let fingerprint = format!("node-{}", rng.gen::<u32>());

    info!("Assigned to monitor on address {}", monitor_addr);

    let app_state = Arc::new(AppState::new(
        &manager_addr,
        &monitor_addr,
        *port,
        &fingerprint,
    ));

    let ping_service = PingService::new(app_state.clone(), keep_running.clone(), 5);
    let recover_service = RecoverService::new(app_state.clone(), keep_running.clone(), 7);

    let server_fut = server::start_server(app_state.clone(), shutdown_rx);
    let ping_fut = ping_service.start();
    let recover_fut = recover_service.start();
    info!("Services started");
    // shutdown_tx.send(()).expect("Shutdown server");
    let _ = tokio::try_join!(server_fut, ping_fut, recover_fut);
    Ok(())
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

fn setup_logger() {
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::White)
        .debug(Color::White)
        .trace(Color::BrightBlack);
    let colors_level = colors_line.clone().info(Color::Green);

    fern::Dispatch::new()
        .chain(std::io::stdout())
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}]{} {}",
                colors_level.color(record.level()),
                chrono::Utc::now().format("[%Y-%m-%d %H:%M:%S]"),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .apply()
        .unwrap();
}

fn setup_close_handler(keep_running: Arc<AtomicBool>, sender: oneshot::Sender<()>) {
    ctrlc::set_handler(move || {
        info!("Send signal to terminate.");
        keep_running.swap(false, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");
}
