// use async_trait::async_trait;
// use log::info;
// use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};

// #[async_trait]
// pub trait Service {
//     async fn start(self) -> std::io::Result<()>;
//     // async fn perform(self);
// }

// impl Service {
//     async fn start(&self + Send, timeout: u64, keep_running: &Arc<AtomicBool>) -> std::io::Result<()> {
//         let run = keep_running.clone();
//         let _ = tokio::spawn(async move {
//             loop {
//                 self.perform();

//                 if keep_running.load(Ordering::Relaxed) {
//                     info!("Sutting down recover service");
//                     break;
//                 }
//             }
//         })
//         .await
//         .unwrap();
    
//         Ok(())
//     }
// }

// pub struct ServiceWorker {
//     labour: dyn Service
// }

// impl ServiceWorker {
//     pub async fn start(&mut self) -> std::io::Result<()> {
//         let _ = tokio::spawn(async move {
//             self.start();
//         }).await.unwrap();

//         Ok(())
//     }
// }
