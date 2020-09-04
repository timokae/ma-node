use crate::app_state::Ping;
use crate::config_store::Monitor;
use crate::server::DownloadResponse;
use crate::stat_store::Stats;
use log::error;
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
#[derive(Deserialize)]
pub struct RegisterResponse {
    pub own_monitor: Monitor,
    pub monitors: Vec<Monitor>,
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
    pub files_to_delete: Vec<String>,
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
            // let result = res.json::<DownloadResponse>().await?;
            // return Ok(result);
            let headers = res.headers().clone();

            let octet_stream =
                &reqwest::header::HeaderValue::from_str("application/octet-stream").unwrap();
            let content_type = headers
                .get("Content-Type")
                .or(Some(octet_stream))
                .unwrap()
                .to_str()
                .unwrap();

            let header_value = headers
                .get("content-disposition")
                .unwrap()
                .to_str()
                .unwrap();

            let content = res.bytes().await.unwrap().to_vec();
            Ok(DownloadResponse {
                hash: String::from(hash),
                content,
                content_type: String::from(content_type),
                file_name: get_file_name(header_value),
            })
        }
        Err(err) => Err(err),
    }
}

fn get_file_name(header_value: &str) -> String {
    if !header_value.contains("filename") {
        error!("Header does not contain filename!");
        return String::from("Does not contain filename");
    }

    match header_value.split(";").find(|s| s.contains("filename")) {
        Some(result) => {
            let parts: Vec<&str> = result.split("=").collect();
            let mut name = String::from(parts[1]);
            name.retain(|c| c != '\'');
            let result = name.trim();
            return String::from(result);
        }
        None => {
            error!("Could not extract filename from header");
            return String::from("unknown");
        }
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

#[derive(Serialize)]
pub struct DistributionRequest {
    pub replications: u32,
    pub own_monitor: bool,
    pub fingerprint: String,
}

pub async fn distribute_to_monitor(
    hash: &str,
    monitor_addr: &str,
    distribution_request: &DistributionRequest,
) -> Result<(), reqwest::Error> {
    let url = format!("{}/distribute/{}?forward=false", monitor_addr, hash);
    let response = reqwest::Client::new()
        .post(&url)
        .json(distribution_request)
        .send()
        .await?;

    match response.error_for_status() {
        Ok(_res) => Ok(()),
        Err(err) => Err(err),
    }
}
