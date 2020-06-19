use crate::app_state::{
    AppState, GeneratePingMessage, GeneratedPing, RecoverEntry, UpdateFilesToSync,
};
use actix::prelude::*;
use chrono::{TimeZone, Utc};
use log::error;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

#[derive(Deserialize)]
pub struct PingResponse {
    status: String,
    files_to_recover: Vec<String>,
}

pub async fn start_ping_loop(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    // let availability_stat = AvailabilityActor::new().start();
    let _ = tokio::spawn(async move {
        loop {
            // let _ = app_state.send(Ping(1)).await;
            // let _res = availability_stat.send(Trigger()).await;
            let _ = ping_monitor(app_state.clone()).await;

            std::thread::sleep(Duration::from_secs(15));
        }
    })
    .await
    .unwrap();

    Ok(())
}

pub async fn ping_monitor(app_state: Arc<Addr<AppState>>) {
    if let Ok(generated_ping) = app_state.send(GeneratePingMessage {}).await {
        match send_ping(generated_ping).await {
            Ok(ping_response) => handle_request_success(&app_state, ping_response).await,
            Err(err) => handle_request_error(err),
        }
    } else {
        error!("Failed to generate ping!");
    }
}

pub async fn send_ping(generated_ping: GeneratedPing) -> Result<PingResponse, reqwest::Error> {
    let url = format!("{}/ping", generated_ping.monitor_addr);

    let response = reqwest::Client::new()
        .post(&url)
        .json(&generated_ping.ping)
        .send()
        .await?;

    match response.error_for_status() {
        Ok(res) => {
            let ping_res = res.json::<PingResponse>().await?;
            return Ok(ping_res);
        }
        Err(err) => Err(err),
    }
}

async fn handle_request_success(app_state: &Arc<Addr<AppState>>, ping_response: PingResponse) {
    let dt = Utc.ymd(1970, 1, 1).and_hms(0, 1, 1);
    let entries = ping_response
        .files_to_recover
        .into_iter()
        .map(|f| RecoverEntry {
            hash: f,
            last_checked: dt.clone(),
        })
        .collect::<Vec<RecoverEntry>>();

    let _ = app_state.send(UpdateFilesToSync { entries: entries }).await;
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

// async fn handle_request_success(app_state: Arc<Addr<AppState>>, ping_response: PingResponse) {
//     // match response.status() {
//     //     reqwest::StatusCode::OK => {
//     //         println!("[{}] Ping send successfully", chrono::Utc::now());
//     //         let ping_response = response.json::<PingResponse>().await?;

//     //         let _ = app_state
//     //             .send(UpdateFilesToSync {
//     //                 hashes: ping_response.files_to_recover,
//     //             })
//     //             .await;
//     //     }
//     //     reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
//     //         eprintln!("[{}] Ping: Internal Server error", chrono::Utc::now());
//     //     }
//     //     _ => {
//     //         if let Ok(text) = response.text().await {
//     //             eprintln!("[{}] {}", chrono::Utc::now(), text);
//     //         }
//     //     }
//     // }
// }
