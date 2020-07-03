use crate::app_state::{AppState, Ping, RecoverEntry};
use chrono::{TimeZone, Utc};
use log::{error, info};
use serde::Deserialize;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::time::Duration;

#[derive(Deserialize)]
pub struct PingResponse {
    status: String,
    files_to_recover: Vec<String>,
}

pub async fn start_ping_loop(
    app_state: Arc<AppState>,
    keep_running: Arc<AtomicBool>,
) -> std::io::Result<()> {
    let _ = tokio::spawn(async move {
        info!("Starting ping service.");
        loop {
            let _ = ping_monitor(app_state.clone()).await;

            if keep_running.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(15));
            } else {
                info!("Shutting down ping service");
                break;
            }
        }
    })
    .await
    .unwrap();

    info!("Ping service terminated.");

    Ok(())
}

pub async fn ping_monitor(app_state: Arc<AppState>) {
    // if let Ok(generated_ping) = app_state.send(GeneratePingMessage {}).await {
    //     match send_ping(generated_ping).await {
    //         Ok(ping_response) => handle_request_success(&app_state, ping_response).await,
    //         Err(err) => handle_request_error(err),
    //     }
    // } else {
    //     error!("Failed to generate ping!");
    // }

    let ping = app_state.generate_ping();
    let monitor_addr = app_state.config_store.read().unwrap().monitor();
    match send_ping(&ping, &monitor_addr).await {
        Ok(ping_response) => handle_request_success(app_state, ping_response).await,
        Err(err) => handle_request_error(err),
    }
}

pub async fn send_ping(ping: &Ping, monitor_addr: &str) -> Result<PingResponse, reqwest::Error> {
    let url = format!("{}/ping", monitor_addr);

    let response = reqwest::Client::new().post(&url).json(ping).send().await?;

    match response.error_for_status() {
        Ok(res) => {
            let ping_res = res.json::<PingResponse>().await?;
            return Ok(ping_res);
        }
        Err(err) => Err(err),
    }
}

async fn handle_request_success(app_state: Arc<AppState>, ping_response: PingResponse) {
    let dt = Utc.ymd(1970, 1, 1).and_hms(0, 1, 1);
    let entries = ping_response
        .files_to_recover
        .into_iter()
        .map(|f| RecoverEntry {
            hash: f,
            last_checked: dt.clone(),
        })
        .collect::<Vec<RecoverEntry>>();

    // let _ = app_state.send(UpdateFilesToSync { entries: entries }).await;
    app_state
        .file_store
        .write()
        .unwrap()
        .insert_files_to_recover(entries);

    info!("Ping successfull.")
}

fn handle_request_error(error: reqwest::Error) {
    if error.is_redirect() {
        if let Some(final_stop) = error.url() {
            error!("redirect loop at {}", final_stop);
        }
    } else if error.is_builder() {
        error!("Builder error");
    } else if error.is_status() {
        if let Some(status) = error.status() {
            error!("Status error {}", status);
        }
    } else if error.is_timeout() {
        error!("Request timeout");
    } else {
        error!("Unknown error: {}", error.to_string());
    }
}
