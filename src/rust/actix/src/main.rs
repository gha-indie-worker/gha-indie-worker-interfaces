use actix_web::{
    http::header,
    web::{self, Bytes},
    App, HttpResponse, HttpServer,
};
use std::env;

const MAX_BODY: usize = 1024 * 1024;

async fn health() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .body(r#"{"status":"ok"}"#)
}

async fn plaintext() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/plain; charset=utf-8"))
        .body("hello, world!")
}

async fn json() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .body(r#"{"message":"hello, world!"}"#)
}

async fn echo(body: Bytes) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/octet-stream"))
        .body(body)
}

async fn not_found() -> HttpResponse {
    HttpResponse::NotFound()
        .insert_header((header::CONTENT_TYPE, "text/plain; charset=utf-8"))
        .body("Not Found")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .ok()
        .filter(|port| *port > 0)
        .expect("PORT must be an integer from 1 to 65535");
    eprintln!("ready actix http://{host}:{port}");
    HttpServer::new(|| {
        App::new()
            .app_data(web::PayloadConfig::new(MAX_BODY))
            .route("/health", web::get().to(health))
            .route("/plaintext", web::get().to(plaintext))
            .route("/json", web::get().to(json))
            .route("/echo", web::post().to(echo))
            .default_service(web::to(not_found))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
