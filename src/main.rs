extern crate actix;
extern crate actix_multipart;
extern crate actix_rt;
extern crate actix_web;
extern crate futures;
extern crate rusqlite;
extern crate serde;

mod app_state;
mod availability_actor;
mod ping;
mod server;

use actix::prelude::*;
use app_state::AppState;
use std::sync::Arc;

#[actix_rt::main]
async fn main() {
    let app_state = Arc::new(
        AppState::new(
            8080,
            String::from("./files"),
            20000000,
            120000,
            String::from("Germany"),
        )
        .start(),
    );
    // let server_fut = server::start_server(app_state.clone());
    let ping_fut_1 = ping::start(app_state.clone());

    println!("Services started");

    // let _ = tokio::try_join!(server_fut);
    let _ = tokio::try_join!(ping_fut_1);
    actix::System::current().stop();
}
