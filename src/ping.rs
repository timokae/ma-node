use crate::app_state::{AppState, Ping};
use actix::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub async fn start(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    let res = tokio::spawn(async move {
        loop {
            let _ = app_state.send(Ping(1)).await;
            std::thread::sleep(Duration::from_secs(1));
        }
    })
    .await
    .unwrap();

    println!("Ping Res: {:?}", res);

    Ok(())
}
