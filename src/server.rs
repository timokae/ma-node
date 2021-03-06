use crate::app_state::AppState;
use crate::config_store::ConfigStoreFunc;
use crate::file_store::FileStoreFunc;
use crate::http_requests::lookup_hash_on_monitor;

use bytes::buf::Buf;
use futures::stream::StreamExt;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::oneshot;
use warp::Filter;

/*
 * Server Backend
 * Offers HTTP entdpoints to download, lookup and upload a file.
 * The ping endpoint can be used to test if the node is visible to others outside the own network.
 * 
 * receiver: channel which shuts the server down.
 */

#[derive(Serialize)]
struct JsonResponse {
    status: String,
    message: String,
}

#[allow(dead_code)]
pub async fn start_server(
    app_state: Arc<AppState>,
    receiver: oneshot::Receiver<()>,
) -> std::io::Result<()> {
    let port = app_state.config_store.read().unwrap().port();
    let state_filter = warp::any().map(move || app_state.clone());

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec![
            "User-Agent",
            "Sec-Fetch-Mode",
            "Referer",
            "Origin",
            "Access-Control-Request-Method",
            "Access-Control-Request-Headers",
            "content-type",
            "x-csrf-token",
        ])
        .allow_methods(vec!["POST", "GET", "DELETE"]);

    let download_hash = warp::get()
        .and(warp::path("download"))
        .and(warp::path::param::<String>())
        .and(state_filter.clone())
        .and_then(download);

    let lookup_hash = warp::get()
        .and(warp::path("lookup"))
        .and(warp::path::param::<String>())
        .and(state_filter.clone())
        .and_then(lookup);

    let upload_multipart = warp::post()
        .and(warp::path("upload"))
        .and(warp::filters::multipart::form().max_length(1024 * 1024 * 10))
        .and(state_filter.clone())
        .and_then(upload_multipart_fun);

    let ping = warp::get().and(warp::path("ping")).and_then(ping_fun);

    let routes = download_hash
        .or(lookup_hash)
        .or(upload_multipart)
        .or(ping)
        .with(cors);

    let addr = std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);
    let (addr, server) = warp::serve(routes).bind_with_graceful_shutdown((addr, port), async {
        receiver.await.ok();
    });

    info!("Startet server on {}", addr);
    tokio::task::spawn(server).await.unwrap();

    Ok(())
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DownloadResponse {
    pub hash: String,           // hash of the filke
    pub content: Vec<u8>,       // file content
    pub content_type: String,   // file type
    pub file_name: String,      // file name
}

async fn download(hash: String, state: Arc<AppState>) -> Result<impl warp::Reply, warp::Rejection> {
    // Check if file with hash is stored on this node
    match state.file_store.read().unwrap().get_file(&hash) {
        Some(file_entry) => {
            // Send file as response
            let response = warp::http::Response::builder()
                .header("Content-Type", &file_entry.content_type)
                .header(
                    "Content-Disposition",
                    format!(":attachment; filename={}", &file_entry.file_name),
                )
                .body(file_entry.content().unwrap())
                .unwrap();

            return Ok(warp::reply::with_status(
                response,
                warp::http::StatusCode::OK,
            ));
        }
        None => {
            error!("Could not find file with hash {}", hash);
            let response = warp::http::Response::builder()
                .status(warp::http::StatusCode::NOT_FOUND)
                .body(vec![])
                .unwrap();

            return Ok(warp::reply::with_status(
                response,
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
    }
}

#[derive(Serialize)]
struct LookupResponse {
    hash: String,
    content: String,
}
async fn lookup(hash: String, state: Arc<AppState>) -> Result<impl warp::Reply, warp::Rejection> {
    // Lookup in local filestore
    if let Some(file_entry) = state.file_store.read().unwrap().get_file(&hash) {
        let reply = warp::reply::json(&DownloadResponse {
            hash,
            content: file_entry.content().unwrap(),
            content_type: String::from(&file_entry.content_type),
            file_name: String::from(&file_entry.file_name),
        });
        return Ok(warp::reply::with_status(reply, warp::http::StatusCode::OK));
    }

    // Forward lookup to monitor
    let monitor = state.config_store.read().unwrap().monitor();
    if let Ok(response) = lookup_hash_on_monitor(&hash, &monitor.addr).await {
        let reply = warp::reply::json(&LookupResponse {
            hash,
            content: response.node_addr,
        });
        return Ok(warp::reply::with_status(reply, warp::http::StatusCode::OK));
    }

    // Hash not found
    return Ok(warp::reply::with_status(
        empty_reply(),
        warp::http::StatusCode::NOT_FOUND,
    ));
}

async fn upload_multipart_fun(
    mut data: warp::filters::multipart::FormData,
    state: Arc<AppState>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut query = String::from("status=error");

    while let Some(Ok(mut part)) = data.next().await {
        // Process uploaded data
        if part.name() == "upload[data]" {
            let mut buf = part.data().await.unwrap().unwrap();
            let binary_vec = buf.to_bytes().to_vec();
            let content_type = part
                .content_type()
                .or(Some("application/octet-stream"))
                .unwrap();

            // Check if hash already exists on other nodes
            // let hash = state
            //     .config_store
            //     .write()
            //     .unwrap()
            //     .hash_content(binary_vec.as_slice());
            // if let Some(_) = state.file_store.read().unwrap().get_file(&hash) {
            //     error!("Uploaded file with hahs {} already exists!", &hash);
            //     return Ok(warp::reply::with_status(
            //         empty_reply(),
            //         warp::http::StatusCode::CONFLICT,
            //     ));
            // }

            // let monitor = state.config_store.read().unwrap().monitor();
            // if let Ok(_) = lookup_hash_on_monitor(&hash, &monitor.addr).await {
            //     error!("Uploaded file with hahs {} already exists!", &hash);
            //     return Ok(warp::reply::with_status(
            //         warp::reply(),
            //         warp::http::StatusCode::CONFLICT,
            //     ));
            // }

            let filename = part.filename().or(Some("unknown")).unwrap();
            let hash = state.add_new_file(binary_vec.as_slice(), content_type, filename, true);
            query = format!("status=success&hash={}", hash);
        }
    }

    let manager_addr = state.config_store.read().unwrap().manager();
    let addr = format!("{}?{}", manager_addr, query);
    let uri = warp::http::Uri::from_str(&addr).unwrap();

    info!("Sending reply");
    return Ok(warp::redirect(uri));
}

async fn ping_fun() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(warp::reply::with_status(
        String::from("pong"),
        warp::http::StatusCode::OK,
    ))
}

fn empty_reply() -> warp::reply::Json {
    let empty_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    warp::reply::json(&empty_map)
}
