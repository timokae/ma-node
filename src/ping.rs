use crate::app_state::{AppState, FilesChanged};
use crate::availability_actor::{AvailabilityActor, Trigger};
use actix::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub async fn start(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    let availability_stat = AvailabilityActor::new().start();
    let _ = tokio::spawn(async move {
        loop {
            // let _ = app_state.send(Ping(1)).await;
            availability_stat.do_send(Trigger());
            std::thread::sleep(Duration::from_secs(1));
        }
    })
    .await
    .unwrap();

    // println!("Ping Res: {:?}", res);

    Ok(())
}
