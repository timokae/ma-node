use crate::app_state;

use actix::prelude::*;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;

use crate::app_state::{AppState, RecoverEntry};

pub async fn start_recover_loop(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    let monitor_addr = app_state.send(app_state::MonitorAddr {}).await.unwrap();
    let _ = tokio::spawn(async move {
        loop {
            if let Ok(entry_opt) = &app_state.send(app_state::NextFileToRecover {}).await {
                match entry_opt {
                    Some(entry) => match lookup_hash(&monitor_addr, &entry.hash).await {
                        Ok(result) => handle_lookup_success(&app_state, &entry.hash, result).await,
                        Err(err) => handle_lookup_fail(&app_state, &entry.hash, err).await,
                    },
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
    app_state: &Arc<Addr<AppState>>,
    hash: &str,
    result: HashMap<String, String>,
) {
    info!("{:?}", result);
    let result = app_state
        .send(app_state::RecoveredFile {
            hash: String::from(hash),
        })
        .await;

    match result {
        Ok(_) => info!("Recovered file {}", hash),
        Err(err) => error!("{}", err),
    }
}

async fn handle_lookup_fail(app_state: &Arc<Addr<AppState>>, hash: &str, error: reqwest::Error) {
    let entries = vec![RecoverEntry {
        hash: String::from(hash),
        last_checked: chrono::Utc::now(),
    }];

    match app_state
        .send(app_state::UpdateFilesToSync { entries })
        .await
    {
        Ok(_) => info!("Failed to recover file {}: {}", hash, error),
        Err(err) => error!("{}", err),
    }
}
