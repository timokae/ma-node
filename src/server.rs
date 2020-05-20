use actix_web::{web, App, HttpResponse, HttpServer, Responder};

pub async fn start_server() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/again", web::get().to(index2))
    })
    .bind("0.0.0.0:8080")?
    .shutdown_timeout(2)
    .run()
    .await
    .unwrap();

    Ok(())
}

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

async fn index2() -> impl Responder {
    HttpResponse::Ok().body("Hello world again!")
}
