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
mod http_requests;
mod ping_service;
mod recover_service;
mod server;
mod service;
mod stat_store;

use app_state::AppState;
use fern::colors::{Color, ColoredLevelConfig};
use http_requests::register_on_manager;
use log::info;
use ping_service::PingService;
use rand::Rng;
use recover_service::RecoverService;
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
    let port: &u16 = &args[1].parse::<u16>().unwrap_or(8080);
    let stats_path: &String = &args[2].parse::<String>().unwrap();
    let manager_addr = String::from("http://localhost:3000");
    let stats = stat_store::Stats::from_file(stats_path);

    let mut rng = rand::thread_rng();
    let fingerprint = format!("node-{}", rng.gen::<u32>());
    let monitor_addr = run_registration(&manager_addr, &stats).await;

    info!("Assigned to monitor on address {}", monitor_addr);

    let app_state = Arc::new(AppState::new(
        &manager_addr,
        &monitor_addr,
        *port,
        &fingerprint,
        stats.capacity,
        stats,
    ));

    info!(
        "Region: {}",
        app_state.stat_store.read().unwrap().stats.region
    );

    let ping_service = PingService::new(app_state.clone(), keep_running.clone(), 10);
    let recover_service = RecoverService::new(app_state.clone(), keep_running.clone(), 10);

    let server_fut = server::start_server(app_state.clone(), shutdown_rx);
    let ping_fut = ping_service.start();
    let recover_fut = recover_service.start();
    info!("Services started");
    // shutdown_tx.send(()).expect("Shutdown server");
    let _ = tokio::try_join!(server_fut, ping_fut, recover_fut);
    Ok(())
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
    let sender_opt = std::sync::Mutex::new(Some(sender));
    ctrlc::set_handler(move || {
        info!("Send signal to terminate.");
        keep_running.swap(false, Ordering::Relaxed);
        if let Some(tx) = sender_opt.lock().unwrap().take() {
            tx.send(()).unwrap();
        }
    })
    .expect("Error setting Ctrl-C handler");
}

async fn run_registration(manager_addr: &str, stats: &stat_store::Stats) -> String {
    let register_request = http_requests::RegisterRequest::from_stats(&stats);
    let response = register_on_manager(&manager_addr, register_request).await;

    match response {
        Ok(result) => result.monitor,
        Err(err) => panic!("{}", err),
    }
}
