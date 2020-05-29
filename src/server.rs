use crate::app_state::{AppState, FilesChanged};
use actix::prelude::*;
use actix_multipart::Multipart;
use actix_web::{web, App, Error, HttpResponse, HttpServer};
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

#[derive(Serialize)]
struct JsonResponse {
    status: String,
}

#[allow(dead_code)]
pub async fn start_server(app_state: Arc<Addr<AppState>>) -> std::io::Result<()> {
    let app_data = web::Data::new(app_state);
    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .route("/", web::get().to(index))
            .route("/upload", web::post().to(save_file))
            .route("/{download:.*}", web::get().to(download))
    })
    .bind("0.0.0.0:8080")?
    .shutdown_timeout(2)
    .run()
    .await
    .unwrap();

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
async fn download(
    web::Query(info): web::Query<DownloadRequest>,
) -> Result<actix_files::NamedFile, Error> {
    let path_str = format!("files/{}", info.hash);
    let path = Path::new(path_str.as_str());
    if path.exists() {
        return Ok(actix_files::NamedFile::open(path)?);
    } else {
        Err(actix_web::error::ErrorNotFound("File not found").into())
    }
}

async fn save_file(
    app_state: web::Data<Arc<Addr<AppState>>>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap();
        let filepath = format!("./files/{}", &filename);

        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();

        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            f = web::block(move || f.write_all(&data).map(|_| f)).await?;
        }
    }

    let response = JsonResponse {
        status: String::from("ok"),
    };

    let _ = app_state.send(FilesChanged()).await;

    Ok(HttpResponse::Ok().json(response))
}
