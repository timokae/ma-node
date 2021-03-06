use log::{error, info};
use rand::{seq::IteratorRandom, thread_rng};
use std::collections::HashMap;
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

use crate::app_state::AppState;
use crate::config_store::{ConfigStoreFunc, Monitor};
use crate::file_store::FileStoreFunc;
use crate::http_requests::{distribute_to_monitor, DistributionRequest};

/*
 * DistributionService
 * Looks for hashes to distribute in the AppState. If hash is stored for distribution,
 * the service chooses a (sub-)set of monitors, sends a DistributionRequest to them
 * and removes the hash from the queue in the AppState
 * 
 * timeout: The amount of time the service should wait if no hash is found in the queue.
 */

pub struct DistributionService {
    pub app_state: Arc<AppState>,
    pub timeout: u64,
}

impl DistributionService {
    pub async fn start(self) -> std::io::Result<()> {
        tokio::spawn(async move {
            let own_monitor = self.app_state.config_store.read().unwrap().monitor();
            let own_fingerprint = self.app_state.config_store.read().unwrap().fingerprint();

            // Filter own monitor from monitors
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

            info!("Distribution Service started");

            let stop_services = self.app_state.stop_services.clone();

            loop {
                // Get next entry in queue
                let hash_opt = self
                    .app_state
                    .file_store
                    .write()
                    .unwrap()
                    .next_file_to_distribute();

                // If a hash is found, distribute it, otherwise send t hread to sleep
                if let Some(hash) = hash_opt {
                    DistributionService::simple_distribution(
                        &own_fingerprint,
                        &own_monitor,
                        &foreign_monitors,
                        &hash,
                    )
                    .await;
                } else {
                    std::thread::sleep(Duration::from_secs(self.timeout));
                }

                // If the flag is set, break out of the loop and exit thread
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

    // Distribution of a hash to all nodes on all monitors
    #[allow(dead_code)]
    async fn non_prioritized_distribution(
        own_fingerprint: &str,
        own_monitor: &Monitor,
        foreign_monitors: &Vec<Monitor>,
        hash: &str,
    ) {
        let replications = -1;

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

    // Distribution of a hash to n nodes on all monitors
    #[allow(dead_code)]
    async fn simple_distribution(
        own_fingerprint: &str,
        own_monitor: &Monitor,
        foreign_monitors: &Vec<Monitor>,
        hash: &str,
    ) {
        let replications = 3;

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

    // Distribution of hash to n nodes of m monitors of all partitions
    #[allow(dead_code)]
    async fn region_based_distribution(
        own_fingerprint: &str,
        own_monitor: &Monitor,
        foreign_monitors: &Vec<Monitor>,
        hash: &str,
    ) {
        let replications_per_monitor = 2;
        let monitor_per_partition = 1;

        // Create group for every bound/partition found and put each monitor in its assigned partition
        let monitor_map = DistributionService::group_monitors(own_monitor, foreign_monitors);

        // Iterate over each monitor group and select m monitors
        for (bound, monitor_vec) in monitor_map.iter() {
            let mut choosen_monitors = monitor_vec
                .iter()
                .choose_multiple(&mut thread_rng(), monitor_per_partition);

            // Make sure that hashes get distributed to the own monitor
            if bound == own_monitor.bound.first().unwrap()
                && !choosen_monitors.contains(&own_monitor)
            {
                choosen_monitors.pop();
                choosen_monitors.push(own_monitor);
            }

            // Iterate over monitors inside each group
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
                    replications: replications_per_monitor,
                };

                if let Err(err) =
                    distribute_to_monitor(&hash, &monitor.addr, &distribution_request).await
                {
                    error!("{}", err);
                }
            }
        }
    }

    // Distribution of a hash to n nodes to m monitors of own partition 
    // and a foreign monitor
    #[allow(dead_code)]
    async fn locale_distribution(
        own_fingerprint: &str,
        own_monitor: &Monitor,
        foreign_monitors: &Vec<Monitor>,
        hash: &str,
    ) {
        let mut distant_relations: HashMap<String, String> = HashMap::new();
        distant_relations.insert("europe".to_string(), "south_america".to_string());
        distant_relations.insert("south_america".to_string(), "europe".to_string());
        distant_relations.insert("north_america".to_string(), "oceania".to_string());
        distant_relations.insert("asia".to_string(), "north_america".to_string());
        distant_relations.insert("oceania".to_string(), "europe".to_string());

        let replications_per_monitor = 2;
        let number_of_locale_monitors = 2;
        let number_of_distant_monitors = 1;
        let monitor_map = DistributionService::group_monitors(own_monitor, foreign_monitors);

        // Choose locale monitors
        let mut locale_monitors = monitor_map
            .get(own_monitor.bound.first().unwrap())
            .unwrap()
            .iter()
            .choose_multiple(&mut thread_rng(), number_of_locale_monitors);

        // Make sure own monitor is in locale monitors
        if !locale_monitors.contains(&own_monitor) {
            locale_monitors.pop();
            locale_monitors.push(own_monitor);
        }

        // Choose distant monitors
        let distant_region = distant_relations
            .get(own_monitor.bound.first().unwrap())
            .unwrap();
        let distant_monitors = monitor_map
            .get(distant_region)
            .unwrap()
            .iter()
            .choose_multiple(&mut thread_rng(), number_of_distant_monitors);

        // Create one lsit of all selecte monitors
        let mut choosen_monitors: Vec<&Monitor> = Vec::new();
        choosen_monitors.extend(locale_monitors.iter());
        choosen_monitors.extend(distant_monitors.iter());

        // Iterate over all monitors and send a DistributionRequest
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
                replications: replications_per_monitor,
            };

            if let Err(err) =
                distribute_to_monitor(&hash, &monitor.addr, &distribution_request).await
            {
                error!("{}", err);
            }
        }
    }

    // Creates a Hashmap, with a key for every partition found 
    // and store all monitors in their assigned partition
    fn group_monitors(
        own_monitor: &Monitor,
        monitors: &Vec<Monitor>,
    ) -> HashMap<String, Vec<Monitor>> {
        let mut monitor_map: HashMap<String, Vec<Monitor>> = HashMap::new();
        monitor_map
            .entry(String::from(own_monitor.bound.get(0).unwrap()))
            .or_insert(vec![])
            .push(own_monitor.clone());

        // Group monitors by bounds
        for monitor in monitors {
            let key = String::from(monitor.bound.get(0).unwrap());
            monitor_map
                .entry(key)
                .or_insert(vec![])
                .push(monitor.clone());
        }

        return monitor_map;
    }
}
