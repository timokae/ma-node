use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;
use warp::Filter;

use crate::app_state::AppState;
use crate::config_store::ConfigStoreFunc;
use crate::file_store::FileStoreFunc;
use crate::http_requests::lookup_hash_on_monitor;
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

    let download_hash = warp::get()
        .and(warp::path("download"))
        .and(warp::path::param::<String>())
        .and(state_filter.clone())
        .and_then(download);

    let upload_file = warp::post()
        .and(warp::path("upload"))
        .and(warp::body::json())
        .and(state_filter.clone())
        .and_then(upload);

    let lookup_hash = warp::get()
        .and(warp::path("download"))
        .and(warp::path::param::<String>())
        .and(state_filter.clone())
        .and_then(lookup);

    let routes = download_hash.or(upload_file).or(lookup_hash);
    let (addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], port), async {
            receiver.await.ok();
        });

    info!("Startet server on {}", addr);
    tokio::task::spawn(server).await.unwrap();

    Ok(())
}

// fn with_state(
//     app_state: Arc<AppState>,
// ) -> impl Filter<Extract = (Arc<AppState>,), Error = std::convert::Infallible> + Clone {
//     warp::any().map(move || app_state.clone())
// }

#[derive(Deserialize, Serialize, Clone)]
pub struct DownloadResponse {
    pub hash: String,
    pub content: String,
}

async fn download(hash: String, state: Arc<AppState>) -> Result<impl warp::Reply, warp::Rejection> {
    match state.file_store.read().unwrap().get_file(&hash) {
        Some(content) => {
            let response = DownloadResponse {
                hash,
                content: String::from(content),
            };

            let reply = warp::reply::json(&response);
            return Ok(warp::reply::with_status(reply, warp::http::StatusCode::OK));
        }
        None => {
            error!("Could not find file with hash {}", hash);
        }
    }

    let empty_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let reply = warp::reply::json(&empty_map);
    return Ok(warp::reply::with_status(
        reply,
        warp::http::StatusCode::NOT_FOUND,
    ));
}

#[derive(Deserialize)]
struct UploadRequest {
    content: String,
}
#[derive(Serialize)]
struct UploadResponse {
    hash: String,
    content: String,
}
async fn upload(
    upload_request: UploadRequest,
    state: Arc<AppState>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if state.file_store.read().unwrap().capacity_left() <= 0 {
        let empty_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let reply = warp::reply::json(&empty_map);
        return Ok(warp::reply::with_status(
            reply,
            warp::http::StatusCode::CONFLICT,
        ));
    }

    let content = upload_request.content.clone();
    let hash = state.config_store.write().unwrap().hash_content(&content);
    state
        .file_store
        .write()
        .unwrap()
        .insert_file(&hash, &content);

    let response = UploadResponse { hash, content };
    let reply = warp::reply::json(&response);
    Ok(warp::reply::with_status(reply, warp::http::StatusCode::OK))
}

#[derive(Serialize)]
struct LookupResponse {
    hash: String,
    content: String,
}
async fn lookup(hash: String, state: Arc<AppState>) -> Result<impl warp::Reply, warp::Rejection> {
    // Lookup in local filestore
    if let Some(content) = state.file_store.read().unwrap().get_file(&hash) {
        let reply = warp::reply::json(&DownloadResponse {
            hash,
            content: String::from(content),
        });
        return Ok(warp::reply::with_status(reply, warp::http::StatusCode::OK));
    }

    // Forward lookup to monitor
    let monitor_addr = state.config_store.read().unwrap().monitor();
    if let Ok(response) = lookup_hash_on_monitor(&hash, &monitor_addr).await {
        let reply = warp::reply::json(&LookupResponse {
            hash,
            content: response.node_addr,
        });
        return Ok(warp::reply::with_status(reply, warp::http::StatusCode::OK));
    }

    // Hash not found
    let empty_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let reply = warp::reply::json(&empty_map);
    return Ok(warp::reply::with_status(
        reply,
        warp::http::StatusCode::NOT_FOUND,
    ));
}
