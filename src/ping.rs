use crate::app_state::{AppState, GeneratePingMessage, UpdateFilesToSync};
use actix::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
#[derive(Deserialize)]
struct PingResponse {
    status: String,
    files_to_recover: Vec<String>,
}

pub async fn send_ping_to_monitor(app_state: Arc<Addr<AppState>>) {
    let ping = app_state.send(GeneratePingMessage {}).await;
    match ping {
        Ok(ping) => {
            let response = reqwest::Client::new()
                .post("http://localhost:3000/ping")
                .json(&ping)
                .send()
                .await;

            match response {
                Ok(r) => {
                    let result = handle_ping_response(app_state, r).await;
                    if let Err(err) = result {
                        eprintln!("{:?}", err);
                    }
                }
                Err(err) => handle_request_error(err),
            }
        }
        _ => println!("Failed to generate ping!"),
    }
}

fn handle_request_error(error: reqwest::Error) {
    if error.is_redirect() {
        if let Some(final_stop) = error.url() {
            eprintln!("redirect loop at {}", final_stop);
        }
    } else if error.is_builder() {
        eprintln!("Builder error");
    } else if error.is_status() {
        if let Some(status) = error.status() {
            eprintln!("Status error {}", status);
        }
    } else if error.is_timeout() {
        eprintln!("Request timeout");
    } else {
        eprintln!("Unknown error: {}", error.to_string());
    }
}

async fn handle_ping_response(
    app_state: Arc<Addr<AppState>>,
    response: reqwest::Response,
) -> Result<(), Box<dyn std::error::Error>> {
    match response.status() {
        reqwest::StatusCode::OK => {
            println!("[{}] Ping send successfully", chrono::Utc::now());
            let ping_response = response.json::<PingResponse>().await;
            let ping_response = ping_response?;

            let _ = app_state
                .send(UpdateFilesToSync {
                    hashes: ping_response.files_to_recover,
                })
                .await;
        }
        reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
            eprintln!("[{}] Ping: Internal Server error", chrono::Utc::now());
        }
        _ => {
            if let Ok(text) = response.text().await {
                eprintln!("[{}] {}", chrono::Utc::now(), text);
            }
        }
    }

    Ok(())
}
