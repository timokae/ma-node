extern crate async_trait;
extern crate crypto;
extern crate ctrlc;
extern crate fern;
extern crate futures;
extern crate log;
extern crate rusqlite;
extern crate serde;

mod app_state;
mod config;
mod config_store;
mod distribution_service;
mod file_store;
mod http_requests;
mod ping_service;
mod recover_service;
mod server;
mod service;
mod stat_store;

use app_state::AppState;
use config_store::ConfigStoreFunc;
use distribution_service::DistributionService;
use fern::colors::{Color, ColoredLevelConfig};
use http_requests::{register_on_manager, RegisterResponse};
use log::info;
use ping_service::PingService;
use recover_service::RecoverService;
use stat_store::StatStoreFunc;
use std::env;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    setup_logger();

    let stop_services: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    setup_close_handler(stop_services.clone(), shutdown_tx);

    let force_ping = Arc::new(AtomicBool::new(true));

    // Read config from file
    let args: Vec<String> = env::args().collect();
    let state_path: &String = &args[1].parse::<String>().unwrap();
    let config_from_file = config::parse_config(state_path);
    let stats = stat_store::StatStore::deserialize_state(state_path);

    // Register on manager
    let register_response = run_registration(&config_from_file.manager_addr, &stats).await;
    let monitor_addr = String::from(&register_response.own_monitor.addr);
    info!("Assigned to monitor on address {}", monitor_addr);

    // Create appstate
    let app_state = Arc::new(AppState::new(
        config_from_file,
        stats,
        register_response.own_monitor,
        register_response.monitors,
        stop_services.clone(),
        force_ping.clone(),
        state_path
    ));
    info!(
        "Region: {}",
        app_state.stat_store.read().unwrap().stats.region
    );

    //  Create background services
    let ping_service = PingService::new(app_state.clone(), 30);
    let recover_service = RecoverService::new(app_state.clone(), 10);
    let distribution_service = DistributionService::new(app_state.clone(), 10);

    // Start background services
    let server_fut = server::start_server(app_state.clone(), shutdown_rx);
    let ping_fut = ping_service.start();
    let recover_fut = recover_service.start();
    let distribution_fut = distribution_service.start();

    info!("Services started");
    let _ = tokio::try_join!(server_fut, ping_fut, recover_fut, distribution_fut);

    info!("Sending shutdown signal");
    let fingerprint = app_state.config_store.read().unwrap().fingerprint();
    let _ = http_requests::notify_monitor_about_shutdown(&fingerprint, &monitor_addr).await;

    app_state.serialize_state();

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
        keep_running.swap(true, Ordering::Relaxed);
        if let Some(tx) = sender_opt.lock().unwrap().take() {
            tx.send(()).unwrap();
        }
    })
    .expect("Error setting Ctrl-C handler");
}

async fn run_registration(manager_addr: &str, stats: &stat_store::Stats) -> RegisterResponse {
    let register_request = http_requests::RegisterRequest::from_stats(&stats);
    let response = register_on_manager(&manager_addr, register_request).await;

    match response {
        Ok(result) => result,
        Err(err) => panic!("{}", err),
    }
}
