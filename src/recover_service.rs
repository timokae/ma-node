use async_trait::async_trait;
use log::{error, info};
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::time::Duration;

use crate::app_state::AppState;
use crate::config_store::ConfigStoreFunc;
use crate::file_store::{FileStoreFunc, RecoverEntry};
// use crate::service::Service;

pub struct RecoverService {
    pub app_state: Arc<AppState>,
    pub keep_running: Arc<AtomicBool>,
    pub timeout: u64,
}

impl RecoverService {
    pub async fn start(self) -> std::io::Result<()> {
        let _ = tokio::spawn(async move {
            info!("Starting recover service");
            loop {
                let monitor_addr = self
                    .app_state
                    .clone()
                    .config_store
                    .read()
                    .unwrap()
                    .monitor();

                let recover_opt = self
                    .app_state
                    .file_store
                    .write()
                    .unwrap()
                    .next_file_to_recover();

                if let Some(entry) = recover_opt {
                    match RecoverService::lookup_hash(&monitor_addr, &entry.hash).await {
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

    async fn lookup_hash(
        monitor_addr: &str,
        hash: &str,
    ) -> Result<HashMap<String, String>, reqwest::Error> {
        info!("Lookup hash {} on monitor", hash);
        let url = format!("{}/lookup/{}?forward=true", monitor_addr, hash);

        let response = reqwest::Client::new().get(&url).send().await?;

        match response.error_for_status() {
            Ok(res) => {
                let result = res.json::<HashMap<String, String>>().await?;
                return Ok(result);
            }
            Err(err) => Err(err),
        }

        // Ok(response)
    }

    async fn handle_lookup_success(
        app_state: Arc<AppState>,
        hash: &str,
        result: HashMap<String, String>,
    ) {
        info!("{:?}", result);
        let node_addr = result.get("node_addr").unwrap();

        match RecoverService::download_from_node(node_addr, hash).await {
            Ok(result) => {
                let content = result.get("content").unwrap();
                // let x = &app_state.clone().file_store;
                // &app_state.clone().insert_new_file(content);
                // app_state.insert_new_file(content);
                let hash = app_state
                    .config_store
                    .write()
                    .unwrap()
                    .hash_content(content);
                info!("Recovered file {} with hash {}", content, hash)
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

    async fn download_from_node(
        node_addr: &str,
        hash: &str,
    ) -> Result<HashMap<String, String>, reqwest::Error> {
        let url = format!("{}/download/{}", node_addr, hash);
        let response = reqwest::Client::new().get(&url).send().await?;

        match response.error_for_status() {
            Ok(res) => {
                let result = res.json::<HashMap<String, String>>().await?;
                return Ok(result);
            }
            Err(err) => Err(err),
        }
    }
}
