use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;
use warp::Filter;

use crate::app_state::AppState;
use crate::config_store::ConfigStoreFunc;
use crate::file_store::FileStoreFunc;

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

    let routes = download_hash.or(upload_file);
    let (_, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], port), async {
            receiver.await.ok();
        });
    tokio::task::spawn(server).await;

    Ok(())
}

// fn with_state(
//     app_state: Arc<AppState>,
// ) -> impl Filter<Extract = (Arc<AppState>,), Error = std::convert::Infallible> + Clone {
//     warp::any().map(move || app_state.clone())
// }

async fn download(hash: String, state: Arc<AppState>) -> Result<impl warp::Reply, warp::Rejection> {
    match state.file_store.read().unwrap().get_file(&hash) {
        Some(content) => {
            let response = DownloadResponse {
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

async fn upload(
    upload_request: UploadRequest,
    state: Arc<AppState>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let content = upload_request.content.clone();
    let hash = state.config_store.write().unwrap().hash_content(&content);
    state
        .file_store
        .write()
        .unwrap()
        .insert_file(&hash, &content);

    let response = UploadResponse { hash, content };
    Ok(warp::reply::json(&response))
}

#[derive(Deserialize, Clone)]
pub struct DownloadRequest {
    hash: String,
}
#[derive(Serialize, Clone)]
pub struct DownloadResponse {
    content: String,
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

// async fn index() -> HttpResponse {
//     let html = r#"<html>
//         <head><title>Upload Test</title></head>
//         <body>
//             <form target="/" method="post" enctype="multipart/form-data">
//                 <input type="file" multiple name="file"/>
//                 <input type="submit" value="Submit"></button>
//             </form>
//         </body>
//     </html>"#;

//     HttpResponse::Ok().body(html)
// }

// fn download(
//     state: Arc<AppState>,
// ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//     let hash = String::from("123");
//     match state.file_store.read().unwrap().get_file(&hash) {
//         Some(content) => {
//             let response = DownloadResponse {
//                 content: String::from(content),
//             };
//             return warp::reply::json(&response);
//         }
//         None => {
//             error!("Could not find file with hash {}", hash);
//             return warp::reject::not_found();
//         }
//     }
// }

// #[derive(Deserialize)]
// struct UploadRequest {
//     content: String,
// }
// #[derive(Serialize)]
// struct UploadResponse {
//     hash: String,
//     content: String,
// }
// async fn upload(
//     app_state: web::Data<Arc<AppState>>,
//     body: web::Json<UploadRequest>,
// ) -> Result<HttpResponse, Error> {
//     let content = body.content.clone();
//     let hash = app_state
//         .config_store
//         .write()
//         .unwrap()
//         .hash_content(&content);
//     app_state
//         .file_store
//         .write()
//         .unwrap()
//         .insert_file(&hash, &content);

//     let response = UploadResponse { hash, content };
//     Ok(HttpResponse::Ok().json(response))
// }

// async fn download(
//     web::Query(info): web::Query<DownloadRequest>,
// ) -> Result<actix_files::NamedFile, Error> {
//     let path_str = format!("files/{}", info.hash);
//     let path = Path::new(path_str.as_str());
//     if path.exists() {
//         return Ok(actix_files::NamedFile::open(path)?);
//     } else {
//         Err(actix_web::error::ErrorNotFound("File not found").into())
//     }
// }

// async fn upload(
//     app_state: web::Data<Arc<Addr<AppState>>>,
//     mut payload: Multipart,
// ) -> Result<HttpResponse, Error> {
//     while let Ok(Some(mut field)) = payload.try_next().await {
//         let content_type = field.content_disposition().unwrap();
//         let filename = content_type.get_filename().unwrap();
//         let filepath = format!("./files/{}", &filename);

//         let mut f = web::block(|| std::fs::File::create(filepath))
//             .await
//             .unwrap();

//         while let Some(chunk) = field.next().await {
//             let data = chunk.unwrap();
//             f = web::block(move || f.write_all(&data).map(|_| f)).await?;
//         }
//     }

//     let response = JsonResponse {
//         status: String::from("ok"),
//         message: Some(String::from("KEKW")),
//     };

//     let _ = app_state.send(FilesChanged()).await;

//     Ok(HttpResponse::Ok().json(response))
// }
