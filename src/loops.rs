use crate::app_state;
use crate::ping;

use actix::prelude::*;
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
            let _ = ping::send_ping_to_monitor(app_state.clone()).await;

            std::thread::sleep(Duration::from_secs(15));
        }
    })
    .await
    .unwrap();

    Ok(())
}

pub async fn start_syncing(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    let _ = tokio::spawn(async move {
        loop {
            if let Ok(next_hash) = app_state.send(app_state::NextHash {}).await {
                match next_hash {
                    Some(h) => println!("Syncing {}", h),
                    None => std::thread::sleep(std::time::Duration::from_secs(2)),
                }
            }
        }
    })
    .await
    .unwrap();

    Ok(())
}
