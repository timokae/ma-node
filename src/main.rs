extern crate actix;
extern crate actix_multipart;
extern crate actix_rt;
extern crate actix_web;
// extern crate futures;
extern crate rusqlite;

mod app_state;
mod availability;
mod ping;
mod server;

use actix::prelude::*;
use app_state::AppState;
use std::sync::Arc;

#[actix_rt::main]
async fn main() {
    let app_state = Arc::new(
        AppState {
            count: 0,
            stop_threads: false,
        }
        .start(),
    );
    let server_fut = server::start_server();
    // let ping_fut_1 = ping::start(app_state.clone());
    // let ping_fut_2 = ping::start(app_state.clone());

    println!("Services started");

    let _ = tokio::try_join!(server_fut /*, ping_fut_1 ping_fut_2*/);
    actix::System::current().stop();
}
