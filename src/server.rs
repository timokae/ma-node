use crate::app_state::AppState;
use crate::config_store::ConfigStoreFunc;
use crate::file_store::FileStoreFunc;
use crate::http_requests::lookup_hash_on_monitor;

use bytes::buf::Buf;
use futures::stream::StreamExt;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;
use warp::Filter;
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

    // let download_hash = warp::get()
    //     .and(warp::path("download"))
    //     .and(warp::fs::dir(file_dir));

    // let upload_file = warp::post()
    //     .and(warp::path("upload"))
    //     .and(warp::body::json())
    //     .and(warp::query::<UploadRequestQuery>())
    //     .and(state_filter.clone())
    //     .and_then(upload);

    let lookup_hash = warp::get()
        .and(warp::path("lookup"))
        .and(warp::path::param::<String>())
        .and(state_filter.clone())
        .and_then(lookup);

    let upload_multipart = warp::post()
        .and(warp::path("upload"))
        .and(warp::filters::multipart::form())
        .and(state_filter.clone())
        .and_then(upload_multipart_fun);

    let routes = download_hash.or(lookup_hash).or(upload_multipart);
    let (addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], port), async {
            receiver.await.ok();
        });

    info!("Startet server on {}", addr);
    tokio::task::spawn(server).await.unwrap();

    Ok(())
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DownloadResponse {
    pub hash: String,
    pub content: Vec<u8>,
    pub content_type: String,
    pub file_name: String,
}

async fn download(hash: String, state: Arc<AppState>) -> Result<impl warp::Reply, warp::Rejection> {
    match state.file_store.read().unwrap().get_file(&hash) {
        Some(file_entry) => {
            let response = warp::http::Response::builder()
                .header("Content-Type", &file_entry.content_type)
                .header(
                    "Content-Disposition",
                    format!(":attachment; filename='{}'", &file_entry.file_name),
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
                .body(vec!())
                .unwrap();

            return Ok(warp::reply::with_status(
                response,
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
    }
}

// #[derive(Deserialize)]
// struct UploadRequest {
//     content: String,
// }
#[derive(Serialize)]
struct UploadResponse {
    hash: String,
    content: String,
}
#[derive(Serialize, Deserialize)]
struct UploadRequestQuery {
    distribute: bool,
}
// async fn upload(
//     upload_request: UploadRequest,
//     _query: UploadRequestQuery,
//     state: Arc<AppState>,
// ) -> Result<impl warp::Reply, warp::Rejection> {
//     if state.file_store.read().unwrap().capacity_left() <= 0 {
//         let empty_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
//         let reply = warp::reply::json(&empty_map);
//         return Ok(warp::reply::with_status(
//             reply,
//             warp::http::StatusCode::CONFLICT,
//         ));
//     }

//     let content = upload_request.content.clone();
//     let hash = state.add_new_file(&content, true);

//     let response = UploadResponse { hash, content };
//     let reply = warp::reply::json(&response);
//     Ok(warp::reply::with_status(reply, warp::http::StatusCode::OK))
// }

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

async fn upload_multipart_fun(
    mut data: warp::filters::multipart::FormData,
    state: Arc<AppState>,
) -> Result<impl warp::Reply, warp::Rejection> {
    while let Some(Ok(field)) = data.next().await {
        let mut part: warp::multipart::Part = field;
        let mut buf = part.data().await.unwrap().unwrap();
        let binary_vec = buf.to_bytes().to_vec();

        let content_type = part
            .content_type()
            .or(Some("application/octet-stream"))
            .unwrap();

        let filename = part.filename().or(Some("unknown")).unwrap();
        state.add_new_file(binary_vec.as_slice(), content_type, filename, true);
    }
    return Ok(warp::reply::with_status(
        warp::reply::json(&String::from("lol")),
        warp::http::StatusCode::OK,
    ));
}
