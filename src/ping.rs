use crate::app_state::{AppState, Ping, SendPing};
// use crate::availability_actor::AvailabilityActor;
use actix::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub async fn start(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    // let availability_stat = AvailabilityActor::new().start();
    let _ = tokio::spawn(async move {
        loop {
            // let _ = app_state.send(Ping(1)).await;
            // let _res = availability_stat.send(Trigger()).await;
            let ping = app_state.send(SendPing()).await;
            match ping {
                Ok(ping) => {
                    let _ = send_ping_to_monitor(ping).await;
                }
                _ => println!("error"),
            }
            std::thread::sleep(Duration::from_secs(15));
        }
    })
    .await
    .unwrap();

    Ok(())
}

async fn send_ping_to_monitor(ping: Ping) {
    let res = reqwest::Client::new()
        .post("http://localhost:3000/ping")
        .json(&ping)
        .send()
        .await;

    match res {
        Ok(resp) => println!(
            "[{}] {} Ping sent successfully",
            chrono::Utc::now(),
            resp.status()
        ),
        Err(_) => eprintln!("[{}] Failed to send ping", chrono::Utc::now()),
    }

    // println!("{:?}", res);
}
