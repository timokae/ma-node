use log::{error, info};
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::time::Duration;

use crate::app_state::AppState;
use crate::config_store::ConfigStoreFunc;
use crate::file_store::{FileStoreFunc, RecoverEntry};
use crate::http_requests::{download_from_node, lookup_hash_on_monitor, LookupMonitorResponse};
// use crate::service::Service;

pub struct RecoverService {
    pub app_state: Arc<AppState>,
    pub keep_running: Arc<AtomicBool>,
    pub timeout: u64,
}

impl RecoverService {
    pub async fn start(self) -> std::io::Result<()> {
        tokio::spawn(async move {
            let monitor_addr = self
                .app_state
                .clone()
                .config_store
                .read()
                .unwrap()
                .monitor();
            info!("Starting recover service");
            loop {
                let recover_opt = self
                    .app_state
                    .file_store
                    .write()
                    .unwrap()
                    .next_file_to_recover();

                if let Some(entry) = recover_opt {
                    info!("Trying to recover {}", entry.hash);
                    match lookup_hash_on_monitor(&entry.hash, &monitor_addr).await {
                        Ok(result) => {
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

                if self.keep_running.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_secs(self.timeout));
                } else {
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

    // async fn perform(self) {
    //     let monitor_addr = self.app_state.clone().config_store.read().unwrap().monitor();
    //     let recover_opt = self.app_state.file_store.write().unwrap().next_file_to_recover();
    //     if let Some(entry) = recover_opt {
    //         match RecoverService::lookup_hash(&monitor_addr, &entry.hash).await {
    //             Ok(result) => {
    //                 RecoverService::handle_lookup_success(self.app_state.clone(), &entry.hash, result).await
    //             }
    //             Err(err) => RecoverService::handle_lookup_fail(self.app_state.clone(), &entry.hash, err).await,
    //         }
    //     } else {
    //         std::thread::sleep(std::time::Duration::from_secs(self.timeout));
    //     }
    // }

    pub fn new(
        app_state: Arc<AppState>,
        keep_running: Arc<AtomicBool>,
        timeout: u64,
    ) -> RecoverService {
        RecoverService {
            app_state,
            keep_running,
            timeout,
        }
    }

    async fn handle_lookup_success(
        app_state: Arc<AppState>,
        hash: &str,
        lookup_response: LookupMonitorResponse,
    ) {
        let node_addr = lookup_response.node_addr;

        match download_from_node(&node_addr, hash).await {
            Ok(result) => {
                let hash = app_state
                    .config_store
                    .write()
                    .unwrap()
                    .hash_content(&result.content);

                app_state
                    .file_store
                    .write()
                    .unwrap()
                    .insert_file(&hash, &result.content);

                info!("Recovered file {} with hash {}", &result.content, hash)
            }
            Err(err) => error!("{:?}", err),
        }
    }

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

    // async fn download_from_node(
    //     node_addr: &str,
    //     hash: &str,
    // ) -> Result<HashMap<String, String>, reqwest::Error> {
    //     let url = format!("{}/download/{}", node_addr, hash);
    //     let response = reqwest::Client::new().get(&url).send().await?;

    //     match response.error_for_status() {
    //         Ok(res) => {
    //             let result = res.json::<HashMap<String, String>>().await?;
    //             return Ok(result);
    //         }
    //         Err(err) => Err(err),
    //     }
    // }
}
