use log::{error, info};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

use crate::app_state::AppState;
use crate::config_store::{ConfigStoreFunc, Monitor};
use crate::file_store::FileStoreFunc;
use crate::http_requests::distribute_to_monitor;

pub struct DistributionService {
    pub app_state: Arc<AppState>,
    pub timeout: u64,
}

impl DistributionService {
    pub async fn start(self) -> std::io::Result<()> {
        tokio::spawn(async move {
            let own_monitor_addr = self.app_state.config_store.read().unwrap().monitor();
            let monitors: Vec<Monitor> = self
                .app_state
                .config_store
                .read()
                .unwrap()
                .monitors()
                .iter()
                .cloned()
                .filter(|m| m.addr != own_monitor_addr)
                .collect();

            info!("Starting distribution service");

            let stop_services = self.app_state.stop_services.clone();

            loop {
                let hash_opt = self
                    .app_state
                    .file_store
                    .write()
                    .unwrap()
                    .next_file_to_distribute();

                if let Some(hash) = hash_opt {
                    for monitor in &monitors {
                        info!("Distributing {} to monitor {}", hash, monitor.addr);

                        if let Err(err) = distribute_to_monitor(&hash, &monitor.addr).await {
                            error!("{}", err);
                        }
                    }
                } else {
                    std::thread::sleep(Duration::from_secs(self.timeout));
                }

                if stop_services.load(Ordering::Relaxed) {
                    info!("Shutting down distribution service");
                    break;
                }
            }
        })
        .await
        .unwrap();

        info!("Distribution service terminated");

        Ok(())
    }

    pub fn new(app_state: Arc<AppState>, timeout: u64) -> DistributionService {
        DistributionService { app_state, timeout }
    }
}
