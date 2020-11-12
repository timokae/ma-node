use chrono::{TimeZone, Utc};
use log::{error, info};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

use crate::app_state::AppState;
use crate::config_store::ConfigStoreFunc;
use crate::file_store::{FileStoreFunc, RecoverEntry};
use crate::http_requests::{ping_monitor, PingResponse};
use crate::stat_store::StatStoreFunc;

/*
 * PingService
 * Send ping requests to the assigned monitor.
 * A ping is send every time the 'force_ping' flag is set or a a specific amount of time has passed.
 * 
 * timeout: The amount of time the service should wait before sending the next ping
 */

pub struct PingService {
    pub app_state: Arc<AppState>,
    pub timeout: u64,
}

impl PingService {
    pub async fn start(self) -> std::io::Result<()> {
        tokio::spawn(async move {
            info!("Ping service started");
            let mut last_ping = std::time::Instant::now();
            let stop_services = self.app_state.stop_services.clone();
            let force_ping = self.app_state.force_ping.clone();

            loop {
                // Check if flag is set or enough time has passend sicne last ping
                if force_ping.load(Ordering::Relaxed)
                    || last_ping.elapsed().as_secs() > self.timeout
                {
                    // Update the counter for the time the node is online
                    self.app_state
                        .stat_store
                        .write()
                        .unwrap()
                        .increase_uptime_counter(last_ping.elapsed().as_secs());

                    // Send ping and process response
                    let _ = PingService::ping_monitor(self.app_state.clone()).await;

                    // save current state to disk, in case the node get killed unexpectedly
                    self.app_state.serialize_state();

                    // Reset flag
                    force_ping.swap(false, Ordering::Relaxed);

                    // Update time of last ping
                    last_ping = std::time::Instant::now();
                } else {
                    // If flag is set, exit thread
                    if stop_services.load(Ordering::Relaxed) {
                        info!("Shutting down ping service");
                        break;
                    }

                    // Otherwise send thread to sleep
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        })
        .await
        .unwrap();

        info!("Ping service terminated.");
        Ok(())
    }


    pub fn new(app_state: Arc<AppState>, timeout: u64) -> PingService {
        PingService { app_state, timeout }
    }

    // Generates new ping and send it to the monitor
    async fn ping_monitor(app_state: Arc<AppState>) {
        let ping = app_state.generate_ping();
        let monitor = app_state.config_store.read().unwrap().monitor();
        match ping_monitor(&ping, &monitor.addr).await {
            Ok(ping_response) => {
                PingService::handle_request_success(app_state.clone(), ping_response);
            }
            Err(err) => {
                PingService::handle_request_error(err);
            }
        }
    }

    fn handle_request_success(app_state: Arc<AppState>, ping_response: PingResponse) {
        let dt = Utc.ymd(1970, 1, 1).and_hms(0, 1, 1);
        // Create FileEntry for every hash in response which needs to be recovered
        let entries = ping_response
            .files_to_recover
            .into_iter()
            .map(|f| RecoverEntry {
                hash: f,
                last_checked: dt.clone(),
            })
            .collect::<Vec<RecoverEntry>>();

        if entries.len() > 0 {
            info!("Files to sync {:?}", entries);
        }

        // Insert hashes into AppState
        app_state
            .file_store
            .write()
            .unwrap()
            .insert_files_to_recover(entries);

        // Remove of all hashes which needs to be deleted
        ping_response
            .files_to_delete
            .iter()
            .for_each(|hash| app_state.file_store.write().unwrap().remove_file(hash));
    }

    // Output error message
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
