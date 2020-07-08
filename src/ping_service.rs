use chrono::{TimeZone, Utc};
use log::{error, info};
use serde::Deserialize;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::time::Duration;
// use async_trait::async_trait;

use crate::app_state::{AppState, Ping};
use crate::config_store::ConfigStoreFunc;
use crate::file_store::{FileStoreFunc, RecoverEntry};
// use crate::service::Service;

#[derive(Deserialize)]
pub struct PingResponse {
    status: String,
    files_to_recover: Vec<String>,
}

pub struct PingService {
    pub app_state: Arc<AppState>,
    pub keep_running: Arc<AtomicBool>,
    pub timeout: u64,
}

impl PingService {
    pub async fn start(self) -> std::io::Result<()> {
        actix::spawn(async move {
            info!("Starting ping service");

            loop {
                let _ = PingService::ping_monitor(self.app_state.clone()).await;

                if self.keep_running.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_secs(self.timeout));
                } else {
                    info!("Shutting down ping service");
                    break;
                }
            }
        });
        // actix::spawn(async move {
        //     info!("Starting ping service.");
        // loop {
        //     let _ = PingService::ping_monitor(self.app_state.clone()).await;

        //     if self.keep_running.load(Ordering::Relaxed) {
        //         std::thread::sleep(Duration::from_secs(self.timeout));
        //     } else {
        //         info!("Shutting down ping service");
        //         break;
        //     }
        // }
        // })
        // .await
        // .unwrap();

        info!("Ping service terminated.");
        Ok(())
    }

    // async fn perform(self) {
    //     let _ = PingService::ping_monitor(self.app_state.clone()).await;
    // }

    pub fn new(
        app_state: Arc<AppState>,
        keep_running: Arc<AtomicBool>,
        timeout: u64,
    ) -> PingService {
        PingService {
            app_state,
            keep_running,
            timeout,
        }
    }

    async fn ping_monitor(app_state: Arc<AppState>) {
        let ping = app_state.generate_ping();
        let monitor_addr = app_state.config_store.read().unwrap().monitor();

        match PingService::send_ping(&ping, &monitor_addr).await {
            Ok(ping_response) => {
                PingService::handle_request_success(app_state.clone(), ping_response);
            }
            Err(err) => {
                PingService::handle_request_error(err);
            }
        }
    }

    async fn send_ping(ping: &Ping, monitor_addr: &str) -> Result<PingResponse, reqwest::Error> {
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

    fn handle_request_success(app_state: Arc<AppState>, ping_response: PingResponse) {
        let dt = Utc.ymd(1970, 1, 1).and_hms(0, 1, 1);
        let entries = ping_response
            .files_to_recover
            .into_iter()
            .map(|f| RecoverEntry {
                hash: f,
                last_checked: dt.clone(),
            })
            .collect::<Vec<RecoverEntry>>();

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
}
