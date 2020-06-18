use crate::app_state::{AppState, GeneratePingMessage, GeneratedPing, UpdateFilesToSync};
use actix::prelude::*;
use log::error;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct PingResponse {
    status: String,
    files_to_recover: Vec<String>,
}

pub async fn ping_monitor(app_state: Arc<Addr<AppState>>) {
    if let Ok(generated_ping) = app_state.send(GeneratePingMessage {}).await {
        match send_ping(generated_ping).await {
            Ok(ping_response) => {
                let _ = app_state
                    .send(UpdateFilesToSync {
                        hashes: ping_response.files_to_recover,
                    })
                    .await;
            }
            Err(err) => {
                handle_request_error(err);
            }
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
        .await?
        .json::<PingResponse>()
        .await?;

    Ok(response)
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
