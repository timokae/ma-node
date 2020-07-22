use crate::app_state::Ping;
use crate::server::DownloadResponse;
use crate::stat_store::Stats;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub region: String,
    pub uptime: Vec<u32>,
}
impl RegisterRequest {
    pub fn from_stats(stats: &Stats) -> RegisterRequest {
        RegisterRequest {
            region: stats.region.clone(),
            uptime: stats.uptime.value.clone(),
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct RegisterResponse {
    pub monitor: String,
}
pub async fn register_on_manager(
    manager_addr: &str,
    register_request: RegisterRequest,
) -> Result<RegisterResponse, reqwest::Error> {
    let url = format!("{}/register/node", manager_addr);
    let response = reqwest::Client::new()
        .post(&url)
        .json(&register_request)
        .send()
        .await?;

    match response.error_for_status() {
        Ok(res) => Ok(res.json::<RegisterResponse>().await?),
        Err(err) => Err(err),
    }
    // match response {
    //     Ok(r) => {
    //         if let reqwest::StatusCode::OK = r.status() {
    //             let rr = r.json::<RegisterResponse>().await.unwrap();
    //             return rr.monitor;
    //         } else {
    //             panic!("Problems with server response");
    //         }
    //     }
    //     Err(_err) => panic!("Failed to register!"),
    // }
}

#[derive(Deserialize)]
pub struct LookupMonitorResponse {
    pub hash: String,
    pub node_addr: String,
}
pub async fn lookup_hash_on_monitor(
    hash: &str,
    monitor_addr: &str,
) -> Result<LookupMonitorResponse, reqwest::Error> {
    let url = format!("{}/lookup/{}?forward=true", monitor_addr, hash);
    let response = reqwest::Client::new().get(&url).send().await?;

    match response.error_for_status() {
        Ok(res) => Ok(res.json::<LookupMonitorResponse>().await?),
        Err(err) => Err(err),
    }
}

#[derive(Deserialize)]
pub struct PingResponse {
    pub status: String,
    pub files_to_recover: Vec<String>,
}
pub async fn ping_monitor(ping: &Ping, monitor_addr: &str) -> Result<PingResponse, reqwest::Error> {
    let url = format!("{}/ping", monitor_addr);
    let response = reqwest::Client::new().post(&url).json(ping).send().await?;

    match response.error_for_status() {
        Ok(res) => {
            let ping_res = res.json::<PingResponse>().await?;
            return Ok(ping_res);
        }
        Err(err) => Err(err),
    }
}

pub async fn download_from_node(
    node_addr: &str,
    hash: &str,
) -> Result<DownloadResponse, reqwest::Error> {
    let url = format!("{}/download/{}", node_addr, hash);
    let response = reqwest::Client::new().get(&url).send().await?;

    match response.error_for_status() {
        Ok(res) => {
            let result = res.json::<DownloadResponse>().await?;
            return Ok(result);
        }
        Err(err) => Err(err),
    }
}

pub async fn notify_monitor_about_shutdown(
    fingerprint: &str,
    monitor_addr: &str,
) -> Result<(), reqwest::Error> {
    let url = format!("{}/shutdown/{}", monitor_addr, fingerprint);
    let response = reqwest::Client::new().get(&url).send().await?;

    match response.error_for_status() {
        Ok(_res) => {
            return Ok(());
        }
        Err(err) => Err(err),
    }
}
