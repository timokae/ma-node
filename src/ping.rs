use crate::app_state::{AppState, Ping};
use crate::availability::{AvailabilityStat, Trigger};
use actix::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub async fn start(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    let availability_stat = AvailabilityStat::new().start();
    let res = tokio::spawn(async move {
        loop {
            let _ = app_state.send(Ping(1)).await;
            let _ = availability_stat.send(Trigger()).await;
            std::thread::sleep(Duration::from_secs(5));
        }
    })
    .await
    .unwrap();

    // println!("Ping Res: {:?}", res);

    Ok(())
}
