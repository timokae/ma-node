use log::{error, info};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

use crate::app_state::AppState;
use crate::config_store::ConfigStoreFunc;
use crate::file_store::{FileStoreFunc, RecoverEntry};
use crate::http_requests::{download_from_node, lookup_hash_on_monitor, LookupMonitorResponse};

/*
 * RecoverService
 * Looks for hashes in the AppState which needs to be downloaded from other nodes. 
 * If such a hash is found, send a lookup request and download the node.
 * 
 * timeout: The amount of time the service should wait if no hash is found in the queue.
 */

pub struct RecoverService {
    pub app_state: Arc<AppState>,
    pub timeout: u64,
}

impl RecoverService {
    pub async fn start(self) -> std::io::Result<()> {
        tokio::spawn(async move {
            let monitor = self
                .app_state
                .clone()
                .config_store
                .read()
                .unwrap()
                .monitor();
            info!("Recover service started");

            let stop_services = self.app_state.stop_services.clone();

            loop {
                // Look for a hash in the AppState
                let recover_opt = self
                    .app_state
                    .file_store
                    .write()
                    .unwrap()
                    .next_file_to_recover();

                // If a hash is found, look it up and download it
                if let Some(entry) = recover_opt {
                    info!("Trying to recover {}", entry.hash);

                    // The if there is still space left on the disk
                    let has_no_capacity =
                        self.app_state.file_store.read().unwrap().capacity_left() <= 0;

                    // If no space exists anymore, reject the hash
                    if has_no_capacity {
                        self.app_state
                            .file_store
                            .write()
                            .unwrap()
                            .reject_hash(&entry.hash);

                        info!("Rejected hash {}", &entry.hash)
                    } else {
                        // Send a lookup request
                        match lookup_hash_on_monitor(&entry.hash, &monitor.addr).await {
                            Ok(result) => {
                                // Download the file from the node
                                RecoverService::handle_lookup_success(
                                    self.app_state.clone(),
                                    &entry.hash,
                                    result,
                                )
                                .await
                            }
                            Err(err) => {
                                RecoverService::handle_lookup_fail(
                                    self.app_state.clone(),
                                    &entry.hash,
                                    err,
                                )
                                .await
                            }
                        }
                    }
                } else {
                    std::thread::sleep(Duration::from_secs(self.timeout));
                }

                if stop_services.load(Ordering::Relaxed) {
                    info!("Shutting down recover service");
                    break;
                }
            }
        })
        .await
        .unwrap();

        info!("Recover service terminated");

        Ok(())
    }

    pub fn new(app_state: Arc<AppState>, timeout: u64) -> RecoverService {
        RecoverService { app_state, timeout }
    }

    async fn handle_lookup_success(
        app_state: Arc<AppState>,
        hash: &str,
        lookup_response: LookupMonitorResponse,
    ) {
        let node_addr = lookup_response.node_addr;

        // Insert the downloaded file into the AppState
        match download_from_node(&node_addr, hash).await {
            Ok(result) => {
                app_state.add_new_file(
                    &result.content,
                    &result.content_type,
                    &result.file_name,
                    false,
                );
                info!("Recovered file with hash {}", hash);
                // let hash = app_state.add_new_file(&result.content, false);
                // info!("Recovered file {} with hash {}", &result.content, hash)
            }
            Err(err) => error!("{:?}", err),
        }
    }

    // If the file could not be downloaded, reinsert the hash in the queue
    async fn handle_lookup_fail(app_state: Arc<AppState>, hash: &str, error: reqwest::Error) {
        let entries = vec![RecoverEntry {
            hash: String::from(hash),
            last_checked: chrono::Utc::now(),
        }];

        app_state
            .file_store
            .write()
            .unwrap()
            .insert_files_to_recover(entries);
        error!(
            "Failed to recover {}, trying again later! Reason: {}",
            hash, error
        );
    }
}
