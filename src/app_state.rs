use actix::prelude::*;

// PING
pub struct Ping(pub usize);

impl Message for Ping {
    type Result = usize;
}

impl Handler<Ping> for AppState {
    type Result = usize;

    fn handle(&mut self, msg: Ping, _: &mut Context<Self>) -> Self::Result {
        self.count += msg.0;
        println!("Ping: {}", self.count);
        self.count
    }
}

// MYACTOR
pub struct AppState {
    pub count: usize,
    pub stop_threads: bool,
}

impl Actor for AppState {
    type Context = Context<Self>;
}
