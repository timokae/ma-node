use crate::app_state::{AppState, GetFile, NewFile};
use actix::prelude::*;
use actix_web::{web, App, Error, HttpResponse, HttpServer};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
// use std::io::Write;
// use std::path::Path;
// use actix_multipart::Multipart;
// use futures::{StreamExt, TryStreamExt};

#[derive(Serialize)]
struct JsonResponse {
    status: String,
    message: String,
}

#[allow(dead_code)]
pub async fn start_server(app_state: Arc<Addr<AppState>>, port: u16) -> std::io::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let app_data = web::Data::new(app_state);
    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .route("/", web::get().to(index))
            // .route("/upload", web::post().to(upload))
            .route("/upload", web::post().to(upload))
            .route("/download/{hash}", web::get().to(download))
    })
    .bind(addr)?
    .run()
    .await
    .unwrap();

    info!("Shutting down server.");

    Ok(())
}

async fn index() -> HttpResponse {
    let html = r#"<html>
        <head><title>Upload Test</title></head>
        <body>
            <form target="/" method="post" enctype="multipart/form-data">
                <input type="file" multiple name="file"/>
                <input type="submit" value="Submit"></button>
            </form>
        </body>
    </html>"#;

    HttpResponse::Ok().body(html)
}

#[derive(Deserialize)]
struct DownloadRequest {
    hash: String,
}
#[derive(Serialize)]
struct DownloadResponse {
    content: String,
}
async fn download(
    app_state: web::Data<Arc<Addr<AppState>>>,
    info: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let content_fut = app_state
        .send(GetFile {
            hash: info.to_string(),
        })
        .await;

    if let Ok(content_opt) = content_fut {
        match content_opt {
            Some(content) => {
                let response = DownloadResponse { content };
                return Ok(HttpResponse::Ok().json(response));
            }
            None => error!("Could not find file with hash {}", info.to_string()),
        }
    }
    Err(actix_web::error::ErrorNotFound("File not found").into())
}

#[derive(Deserialize)]
struct UploadRequest {
    content: String,
}
async fn upload(
    app_state: web::Data<Arc<Addr<AppState>>>,
    body: web::Json<UploadRequest>,
) -> Result<HttpResponse, Error> {
    let content = body.content.clone();

    let _ = app_state.send(NewFile { content }).await;

    let response = JsonResponse {
        status: String::from("ok"),
        message: String::from("KEKW"),
    };

    Ok(HttpResponse::Ok().json(response))
}

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
