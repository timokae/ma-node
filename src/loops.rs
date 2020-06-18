use crate::app_state;
use crate::ping;

use actix::prelude::*;
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// use crate::availability_actor::AvailabilityActor;

use crate::app_state::AppState;

pub async fn start_ping(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    // let availability_stat = AvailabilityActor::new().start();
    let _ = tokio::spawn(async move {
        loop {
            // let _ = app_state.send(Ping(1)).await;
            // let _res = availability_stat.send(Trigger()).await;
            let _ = ping::ping_monitor(app_state.clone()).await;

            std::thread::sleep(Duration::from_secs(15));
        }
    })
    .await
    .unwrap();

    Ok(())
}

pub async fn start_syncing(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    let monitor_addr = app_state.send(app_state::MonitorAddr {}).await.unwrap();
    let _ = tokio::spawn(async move {
        loop {
            if let Ok(next_hash) = app_state.send(app_state::NextHash {}).await {
                match next_hash {
                    Some(hash) => {
                        info!("Syncing {}", &hash);
                        let result = lookup_hash(&monitor_addr, &hash).await;
                        info!("{:?}", result);

                        let _ = app_state
                            .send(app_state::RecoveredFile { hash: hash })
                            .await;
                    }
                    None => std::thread::sleep(std::time::Duration::from_secs(2)),
                }
            }
        }
    })
    .await
    .unwrap();

    Ok(())
}

async fn lookup_hash(
    monitor_addr: &str,
    hash: &str,
) -> Result<HashMap<String, String>, reqwest::Error> {
    let url = format!("{}/lookup/{}?forward=true", monitor_addr, hash);

    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await?
        .json::<HashMap<String, String>>()
        .await?;

    Ok(response)
}
