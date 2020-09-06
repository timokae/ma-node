use log::{error, info};
use rand::{seq::IteratorRandom, thread_rng};
use std::collections::HashMap;
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

use crate::app_state::AppState;
use crate::config_store::{ConfigStoreFunc, Monitor};
use crate::file_store::FileStoreFunc;
use crate::http_requests::{distribute_to_monitor, DistributionRequest};

pub struct DistributionService {
    pub app_state: Arc<AppState>,
    pub timeout: u64,
}

impl DistributionService {
    pub async fn start(self) -> std::io::Result<()> {
        tokio::spawn(async move {
            let own_monitor = self.app_state.config_store.read().unwrap().monitor();
            let own_fingerprint = self.app_state.config_store.read().unwrap().fingerprint();

            let foreign_monitors: Vec<Monitor> = self
                .app_state
                .config_store
                .read()
                .unwrap()
                .monitors()
                .iter()
                .cloned()
                .filter(|m| m.addr != own_monitor.addr)
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
                    DistributionService::region_based_distribution(
                        &own_fingerprint,
                        &own_monitor,
                        &foreign_monitors,
                        &hash,
                    )
                    .await;
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

    #[allow(dead_code)]
    async fn simple_distribution(
        own_fingerprint: &str,
        own_monitor: &Monitor,
        foreign_monitors: &Vec<Monitor>,
        hash: &str,
    ) {
        let replications = 2;

        // Distribute to own monitor
        let own_distribution_request = DistributionRequest {
            fingerprint: String::from(own_fingerprint),
            to_own_monitor: true,
            replications,
        };

        info!("Distributing {} to own monitor {}", hash, own_monitor.addr);
        if let Err(err) =
            distribute_to_monitor(&hash, &own_monitor.addr, &own_distribution_request).await
        {
            error!("{}", err);
        }

        // Distribute to foreign monitors
        for monitor in foreign_monitors {
            info!("Distributing {} to monitor {}", hash, monitor.addr);
            let distribution_request = DistributionRequest {
                fingerprint: String::from(own_fingerprint),
                to_own_monitor: false,
                replications,
            };

            if let Err(err) =
                distribute_to_monitor(&hash, &monitor.addr, &distribution_request).await
            {
                error!("{}", err);
            }
        }
    }

    #[allow(dead_code)]
    async fn region_based_distribution(
        own_fingerprint: &str,
        own_monitor: &Monitor,
        foreign_monitors: &Vec<Monitor>,
        hash: &str,
    ) {
        let replications_per_partition = 2;

        let mut monitor_map: HashMap<String, Vec<Monitor>> = HashMap::new();
        monitor_map
            .entry(String::from(own_monitor.bound.get(0).unwrap()))
            .or_insert(vec![])
            .push(own_monitor.clone());

        // Group monitors by bounds
        for monitor in foreign_monitors {
            let key = String::from(monitor.bound.get(0).unwrap());
            monitor_map
                .entry(key)
                .or_insert(vec![])
                .push(monitor.clone());
        }

        // Iterate over each monitor group
        for (bound, monitor_vec) in monitor_map.iter() {
            let mut choosen_monitors = monitor_vec.iter().choose_multiple(&mut thread_rng(), 1);

            // Make sure that hashes get distributed to the own monitor
            if bound == own_monitor.bound.first().unwrap()
                && !choosen_monitors.contains(&own_monitor)
            {
                choosen_monitors.pop();
                choosen_monitors.push(own_monitor);
            }

            // Iterate over monitors inside groups
            for monitor in choosen_monitors {
                info!(
                    "Distributing {} to [{}]{}",
                    hash,
                    monitor.bound.first().unwrap(),
                    monitor.addr
                );

                let distribution_request = DistributionRequest {
                    fingerprint: String::from(own_fingerprint),
                    to_own_monitor: monitor.addr == own_monitor.addr,
                    replications: replications_per_partition,
                };

                if let Err(err) =
                    distribute_to_monitor(&hash, &own_monitor.addr, &distribution_request).await
                {
                    error!("{}", err);
                }
            }
        }
    }
}
