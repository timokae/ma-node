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
    pub ipv6: Option<String>,
}
impl RegisterRequest {
    pub fn from_stats(stats: &Stats, ipv6: Option<String>) -> RegisterRequest {
        RegisterRequest {
            region: stats.region.clone(),
            uptime: stats.uptime.value.clone(),
            ipv6,
        }
    }
}

#[derive(Deserialize)]
pub struct RegisterResponse {
    pub own_monitor: Monitor,   // Monitor which is assigned to the node
    pub monitors: Vec<Monitor>, // All monitors that are currently known
    pub addr: String,           // IP on which other nodes will try to connect to the node
}

/* Send RegistratioRequest to manager, for it to assign node to a monitor
 *
 * manager_addr: Ulr of the manager
 * register_request: RegisterRequest to send to the manager
 */
pub async fn register_on_manager(
    manager_addr: &str,
    register_request: RegisterRequest,
) -> Result<RegisterResponse, reqwest::Error> {
    let url = format!("{}/api/register/node", manager_addr);
    let response = reqwest::Client::new()
        .post(&url)
        .json(&register_request)
        .send()
        .await?;

    match response.error_for_status() {
        Ok(res) => Ok(res.json::<RegisterResponse>().await?),
        Err(err) => Err(err),
    }
}

#[derive(Deserialize)]
pub struct LookupMonitorResponse {
    pub hash: String,       // Hash of the searched file
    pub node_addr: String,  // Node which stores the file
}

/* Send a LookupRequest to a monitor
 * When the hash is found, it returns a LookupMonitorResponse
 * 
 * hash: Hash of the file to search
 * monitor_addr: Url of the monitor to lookup the hash
 */
pub async fn lookup_hash_on_monitor(
    hash: &str,
    monitor_addr: &str,
) -> Result<LookupMonitorResponse, reqwest::Error> {
    let url = format!("{}/lookup/{}?forward=true", monitor_addr, hash);
    let response = reqwest::Client::new().get(&url).send().await;

    match response {
        Ok(response) => match response.error_for_status() {
            Ok(res) => Ok(res.json::<LookupMonitorResponse>().await?),
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    }
}

#[derive(Deserialize)]
pub struct PingResponse {
    pub status: String,                 // Not used at the moment
    pub files_to_recover: Vec<String>,  // Array of the files the node should downlaod from other nodes
    pub files_to_delete: Vec<String>,   // Array of the files the node should delete
}

/* Send the given ping to the monitor
 * 
 * ping: Reference of the ping to send
 * monitor_addr: Url of the monitor which should receive the ping
 */
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

/* Download the file for the given hash from another monitor
 * Reads the file information and content from response and returns it as a DownloadResponse
 * 
 * node_addr: Url of the node which helds the file
 * hash: Hash of the file to download
 */
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

// Read the file name from a header of a http response
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

/* Send a http request to the monitor to inform it about this node to shutdown
 * 
 * fingerprint: fingerprint of the node
 * monitor_addr: Url of the monitor to notify
 */ 
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
    pub replications: i32,      // Number of replications to create
    pub to_own_monitor: bool,   // Is request send to the nodes own monitor?
    pub fingerprint: String,    // Fingerprint of the node
}

/* Send a DistributionRequest to the given monitor
 * 
 * hash: The hash of the file to distribute
 * monitor_addr: Url of the monitor which should receive the DistributionRequest
 */
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
